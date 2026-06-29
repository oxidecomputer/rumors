use before::Party;
use proptest::collection::vec;
use proptest::prelude::*;

use crate::tree::traverse::{Action, act};
use crate::tree::typed::height::Root;
use crate::tree::typed::{Node, Path};
use crate::{Version, message::Message};

/// The `index`-th party in a canonical left-leaning fork chain descending from
/// a single [`Party::seed`].
///
/// Distinct indices yield mutually *disjoint* parties, so versions ticked on
/// different indices are causally concurrent — the test analogue of "different
/// peers with independent histories". Because the chain is fully determined by
/// the index, independent proptest strategies can each mint the same disjoint
/// parties without sharing any state, which is what lets two separately
/// generated trees (e.g. `arb_tree_root(0, …)` and `arb_tree_root(1, …)`) end
/// up with incomparable root versions.
pub fn nth_party(index: usize) -> Party {
    let mut keep = Party::seed();
    let mut child = keep.fork();
    for _ in 0..index {
        child = keep.fork();
    }
    child
}

/// Largest number of ticks an [`arb_version`] draw places on a single party.
const MAX_VERSION_TICKS: u64 = 4;

/// Number of distinct disjoint parties an [`arb_version`] draw may tick. Drawing
/// ticks from more than one party lets generated versions be mutually
/// *concurrent*, not just points on a single totally-ordered chain.
const VERSION_PARTIES: usize = 3;

/// Generate an arbitrary [`Version`] by ticking a randomly-chosen disjoint
/// party (see [`nth_party`]) a small random number of times.
///
/// Because different draws may pick different parties, pairs of generated
/// versions can be concurrent, which exercises the multi-way branch join in
/// `Node::branch`.
pub fn arb_version() -> BoxedStrategy<Version> {
    (0..VERSION_PARTIES, 0..=MAX_VERSION_TICKS)
        .prop_map(|(party, ticks)| {
            let p = nth_party(party);
            let mut v = Version::new();
            for _ in 0..ticks {
                v.tick(&p);
            }
            v
        })
        .boxed()
}

/// Build a typed root tree by inserting random leaves via `act`.
///
/// The `party` index controls which disjoint party the inserts are attributed
/// to (see [`nth_party`]), making it possible to generate two trees with
/// independent, causally-concurrent version histories.
pub fn arb_root_node(
    party: usize,
    leaves: impl Into<proptest::collection::SizeRange>,
) -> BoxedStrategy<Option<Node<(), Root>>> {
    vec(any::<()>(), leaves)
        .prop_map(move |draws| {
            // Tick this tree's party once per leaf, so the leaves carry a
            // strictly-increasing chain of versions on a single party. Each
            // leaf is placed at its content-addressed path, exactly as a real
            // insert does (see [`Path::for_leaf`] and `Tree::act`): a tree with
            // a leaf anywhere else can never arise in production, so gossiping
            // one would test an impossible state.
            let p = nth_party(party);
            let mut version = Version::new();
            let actions: Vec<_> = draws
                .into_iter()
                .map(|()| {
                    version.tick(&p);
                    let message = Message::new(());
                    let path = Path::for_leaf(&version, message.bytes());
                    (path, version.clone(), Action::Insert(message))
                })
                .collect();
            act(None, actions, |_| ())
        })
        .boxed()
}

/// Build a [`crate::tree::Root`] by lifting [`arb_root_node`].
///
/// A populated node becomes a populated root, and the empty case still gets a
/// non-default root version (an arbitrary frontier on the same party) so empty
/// branches are exercised too.
pub fn arb_tree_root(
    party: usize,
    leaves: impl Into<proptest::collection::SizeRange>,
) -> BoxedStrategy<crate::tree::Root<()>> {
    (arb_root_node(party, leaves), 0u64..8)
        .prop_map(move |(node, extra_ticks)| {
            // The wrapper version must be a causal upper bound on every version
            // inside the contained tree; the mirror protocol reads it as
            // authoritative for "what we have seen". Fold the root node's own
            // version in so a generated `Root` always satisfies that invariant,
            // regardless of `extra`.
            let inner = node
                .as_ref()
                .map(Node::ceiling)
                .cloned()
                .unwrap_or_default();
            // An arbitrary extra frontier on this tree's own party, so even an
            // empty tree exercises a non-default root version.
            let p = nth_party(party);
            let mut extra = Version::new();
            for _ in 0..extra_ticks {
                extra.tick(&p);
            }
            crate::tree::Root {
                ceiling: extra | inner,
                root: node,
            }
        })
        .boxed()
}

/// Generate a pair of divergent trees that share causal history.
///
/// A common base (inserts on party 0) is forked into two sides, each of which
/// then makes its own concurrent inserts (parties 1 and 2) and redacts an
/// arbitrary subset of the shared keys.
///
/// This exercises every cell a merge must handle: keys only one side has, keys
/// both share (matched subtrees), and keys one side has *deleted*
/// while the other still holds them (which the merge must drop by version
/// dominance, the entire deletion mechanism). With zero shared inserts the two
/// sides are fully disjoint, so this one generator also covers that case.
pub fn arb_divergent_pair() -> BoxedStrategy<(crate::tree::Root<()>, crate::tree::Root<()>)> {
    use crate::tree::{Action, Tree};

    (
        0usize..6,                // shared inserts (the common base)
        0usize..5,                // a-only inserts
        0usize..5,                // b-only inserts
        vec(any::<bool>(), 0..6), // which shared keys side a redacts
        vec(any::<bool>(), 0..6), // which shared keys side b redacts
    )
        .prop_map(|(n_shared, n_a, n_b, a_redact, b_redact)| {
            let p_s = nth_party(0);
            let p_a = nth_party(1);
            let p_b = nth_party(2);

            // Common base; at this point the tree holds exactly the shared
            // inserts, so its live keys are the shared keys each side may
            // redact.
            let mut base = Tree::new();
            base.act(
                &p_s,
                (0..n_shared).map(|_| Action::Insert(Message::new(()))),
            );
            let shared_keys: Vec<_> = base.iter().map(|(k, _, _)| k).collect();

            let side = |party: &Party, n: usize, redact: &[bool]| {
                let mut t = base.clone();
                t.act(party, (0..n).map(|_| Action::Insert(Message::new(()))));
                let forgets: Vec<_> = shared_keys
                    .iter()
                    .zip(redact)
                    .filter_map(|(k, &r)| r.then_some(Action::Forget(*k)))
                    .collect();
                t.act(party, forgets);
                t.root
            };

            (side(&p_a, n_a, &a_redact), side(&p_b, n_b, &b_redact))
        })
        .boxed()
}

#[cfg(test)]
mod test {
    use super::nth_party;

    /// Distinct indices yield mutually *disjoint* parties.
    ///
    /// This is the invariant every strategy here relies on: trees built on
    /// different indices must have causally-concurrent (joinable) histories,
    /// never one containing the other. `nth_party` walks a left-leaning fork
    /// chain, so its string form looks nested — `(0, 1)`, `((0, 1), 0)`, … —
    /// but each owns a disjoint dyadic sub-interval, which `Party::is_disjoint`
    /// confirms.
    #[test]
    fn distinct_indices_are_pairwise_disjoint() {
        const N: usize = 16;
        for i in 0..N {
            for j in 0..N {
                if i != j {
                    let (a, b) = (nth_party(i), nth_party(j));
                    assert!(
                        a.is_disjoint(&b),
                        "nth_party({i}) = {a} and nth_party({j}) = {b} are not disjoint",
                    );
                }
            }
        }
    }
}
