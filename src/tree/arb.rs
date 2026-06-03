use before::Party;
use proptest::collection::vec;
use proptest::prelude::*;

use crate::tree::traverse::{Action, act};
use crate::tree::typed::height::Root;
use crate::tree::typed::{Node, Path};
use crate::{message::Message, version::Version};

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
/// party (see [`nth_party`]) a small random number of times. Because different
/// draws may pick different parties, pairs of generated versions can be
/// concurrent, which exercises the multi-way branch join in `Node::branch`.
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
    vec(any::<[u8; 32]>(), leaves)
        .prop_map(move |paths| {
            // Tick this tree's party once per leaf, so the leaves carry a
            // strictly-increasing chain of versions on a single party.
            let p = nth_party(party);
            let mut version = Version::new();
            let actions: Vec<_> = paths
                .into_iter()
                .map(|bytes| {
                    let path = Path::from(bytes);
                    version.tick(&p);
                    (path, version.clone(), Action::Insert(Message::new(())))
                })
                .collect();
            pollster::block_on(act(None, actions, crate::tree::ignore))
        })
        .boxed()
}

/// Build a [`crate::tree::Root`] by lifting [`arb_root_node`]: a populated node
/// becomes a populated root, and the empty case still gets a non-default root
/// version (an arbitrary frontier on the same party) so empty branches are
/// exercised too.
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
                .map(Node::version)
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
                version: extra | inner,
                root: node,
            }
        })
        .boxed()
}

#[cfg(test)]
mod test {
    use super::nth_party;

    /// Distinct indices yield mutually *disjoint* parties. This is the
    /// load-bearing invariant for every strategy here: trees built on different
    /// indices must have causally-concurrent (joinable) histories, never one
    /// containing the other. `nth_party` walks a left-leaning fork chain, so its
    /// string form looks nested — `(0, 1)`, `((0, 1), 0)`, … — but each owns a
    /// disjoint dyadic sub-interval, which `Party::is_disjoint` confirms.
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
