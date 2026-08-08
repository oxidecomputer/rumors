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
            v.ticks(&p, ticks);
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
            extra.ticks(&p, extra_ticks);
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

/// [`arb_divergent_pair`] at a budget wide enough to reach the streaming
/// wire deadlock's trigger geometry.
///
/// Wide roots whose opening reply mixes disputed children with outright
/// provisions, with disputes that descend several levels — the shape the
/// small budget rarely produces.
///
/// The small-budget generator stays the default for properties where case
/// count matters more than per-case breadth; wire-liveness properties run
/// both.
///
/// This strategy closes the proxy tier's generator gap on *budget* only,
/// deliberately not on *bias*: content addressing makes each child's radix a
/// function of leaf
/// hashes, so steering generation toward the early-radix-order deep-dispute
/// shape would mean a per-case search inside the strategy. The geometry pin
/// is instead the deterministic [`early_first_child_dispute_pair`] fixture,
/// which performs that search once; this strategy provides breadth around
/// it.
pub fn arb_wide_divergent_pair() -> BoxedStrategy<(crate::tree::Root<()>, crate::tree::Root<()>)> {
    use crate::tree::{Action, Tree};

    (
        0usize..12,                // shared inserts (the common base)
        0usize..40,                // a-only inserts
        0usize..40,                // b-only inserts
        vec(any::<bool>(), 0..12), // which shared keys side a redacts
        vec(any::<bool>(), 0..12), // which shared keys side b redacts
    )
        .prop_map(|(n_shared, n_a, n_b, a_redact, b_redact)| {
            let p_s = nth_party(0);
            let p_a = nth_party(1);
            let p_b = nth_party(2);

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

/// A divergent pair whose sides share a spine of drawn depth before their
/// novelty splits: the constructed analogue of a hash-prefix collision.
///
/// Content-addressed generators cannot produce this shape — blake3 scatters
/// their keys at the root fan, so a merge's divergent descent below the
/// root is reachable only through chosen paths like these. Both sides
/// extend one shared leaf whose all-zero path pins the spine; each side's
/// novelty diverges at the drawn byte position and rides its own disjoint
/// party, so it is concurrent and survives deletion-pruning. Novelty
/// widths draw zero too, so subset, identical, and ceiling-only merges —
/// a changed flag's `false` arm — are sampled at depth alongside the
/// gains.
pub fn arb_deep_divergent_pair() -> BoxedStrategy<(crate::tree::Root<()>, crate::tree::Root<()>)> {
    (0usize..32, 0u8..5, 0u8..5)
        .prop_map(|(depth, a_width, b_width)| {
            let path_at = |branch: u8| {
                let mut bytes = [0u8; 32];
                bytes[depth] = branch;
                Path::from(bytes)
            };

            let mut shared_version = Version::new();
            shared_version.tick(&nth_party(0));
            let base = act(
                None,
                vec![(
                    path_at(0),
                    shared_version.clone(),
                    Action::Insert(Message::new(())),
                )],
                |_| (),
            );

            // One side: `width` sibling leaves diverging at `depth`, all on
            // the side's own party. The branch ranges are disjoint across
            // sides so novelty never collides into a same-path dispute.
            let side = |party_index: usize, first_branch: u8, width: u8| {
                let mut version = Version::new();
                version.tick(&nth_party(party_index));
                let leaves: Vec<_> = (first_branch..first_branch + width)
                    .map(|branch| {
                        (
                            path_at(branch),
                            version.clone(),
                            Action::Insert(Message::new(())),
                        )
                    })
                    .collect();
                let node = if leaves.is_empty() {
                    base.clone()
                } else {
                    act(base.clone(), leaves, |_| ())
                };
                root_with_ceiling(node, shared_version.clone() | version)
            };

            (side(1, 1, a_width), side(2, 5, b_width))
        })
        .boxed()
}

/// A deterministic pair with the streaming deadlock's trigger geometry.
///
/// The radix-*first* root child is disputed (both sides hold divergent,
/// branching content under it), while at least six higher-radix root
/// children exist on one side only, queueing whole-subtree provisions
/// behind the dispute on the same reply stream.
///
/// This is the streaming wire deadlock's counterexample skeleton, made
/// permanent at the tier that should have owned it. Content
/// addressing means the shape cannot be dictated, so it is *searched*: insert
/// counts vary per attempt, each attempt's honestly-built pair is checked
/// against the geometry, and the first satisfying pair wins. Hashing is
/// deterministic, so the search — and therefore the fixture — is too.
pub fn early_first_child_dispute_pair() -> (crate::tree::Root<()>, crate::tree::Root<()>) {
    use crate::tree::{Action, Tree};

    /// Leaves per side: enough on the left for wide roots with collisions,
    /// few enough on the right that most left children are provisions.
    const LEFT_LEAVES: usize = 32;
    const RIGHT_LEAVES: usize = 8;

    /// Window stride between attempts: larger than either window, so
    /// successive attempts draw fully disjoint leaf populations.
    const STRIDE: usize = 64;

    /// Attempt budget; the assert below turns exhaustion into a loud failure.
    ///
    /// The precompute below is proportional to this bound, so it directly
    /// prices the fixture. Hashing is deterministic and the winning window
    /// is attempt 1581, so 2048 is exact headroom, not a guess; if hashing
    /// or the leaf encoding ever changes, the search either finds another
    /// window within the budget or fails loudly here.
    const ATTEMPTS: usize = 2048;

    // Paths are functions of (version, payload) and payloads are unit, so a
    // candidate pair is fully determined by where each side's version chain
    // *starts*: `Tree::act` ticks from the root ceiling, so seeding a built
    // tree's ceiling with a pre-ticked version shifts every leaf's version —
    // and therefore its whole path — while keeping version encodings small
    // and the state legitimate (indistinguishable from a tree whose earlier
    // content was redacted). The search therefore precomputes each party's
    // whole first-byte sequence in one pass and examines disjoint windows of
    // it; only the one winning attempt pays for real tree construction, and
    // the equality assert below keeps the simulation honest against the
    // builder.
    let firsts = |party: &Party, ticks: usize| -> Vec<u8> {
        let mut version = Version::new();
        (0..ticks)
            .map(|_| {
                version.tick(party);
                let path: [u8; 32] = Path::for_leaf(&version, Message::new(()).as_slice()).into();
                path[0]
            })
            .collect()
    };
    let burnt = |party: &Party, ticks: usize| {
        let mut version = Version::new();
        version.ticks(party, ticks);
        version
    };

    let p_a = nth_party(1);
    let p_b = nth_party(2);
    let f_a = firsts(&p_a, ATTEMPTS * STRIDE + LEFT_LEAVES);
    let f_b = firsts(&p_b, ATTEMPTS * STRIDE + RIGHT_LEAVES);

    for attempt in 0..ATTEMPTS {
        let at = attempt * STRIDE;
        let left_firsts = &f_a[at..at + LEFT_LEAVES];
        let right_firsts = &f_b[at..at + RIGHT_LEAVES];
        let Some(&first) = left_firsts.iter().chain(right_firsts.iter()).min() else {
            continue;
        };

        // The radix-first root child must be present on both sides (a
        // dispute) with branching content on at least one (two or more
        // leaves, so the dispute descends instead of resolving by an inline
        // supply), and at least six higher-radix children must exist on one
        // side only — whole-subtree provisions queued behind the dispute.
        let left_under = left_firsts.iter().filter(|&&b| b == first).count();
        let right_under = right_firsts.iter().filter(|&&b| b == first).count();
        let provisions = {
            let mut one_sided: Vec<u8> = left_firsts
                .iter()
                .filter(|b| !right_firsts.contains(b))
                .chain(right_firsts.iter().filter(|b| !left_firsts.contains(b)))
                .copied()
                .filter(|b| *b > first)
                .collect();
            one_sided.sort_unstable();
            one_sided.dedup();
            one_sided.len()
        };
        if left_under.min(right_under) >= 1 && left_under.max(right_under) >= 2 && provisions >= 6 {
            let build = |party: &Party, base: Version, live: usize| {
                let mut tree = Tree::new();
                tree.root.ceiling = base;
                tree.act(party, (0..live).map(|_| Action::Insert(Message::new(()))));
                tree
            };
            let left = build(&p_a, burnt(&p_a, at), LEFT_LEAVES);
            let right = build(&p_b, burnt(&p_b, at), RIGHT_LEAVES);
            // Both sides' geometry was judged from the simulation, so both
            // sides must agree with the honestly built trees.
            for (tree, firsts) in [(&left, left_firsts), (&right, right_firsts)] {
                let mut built: Vec<u8> = tree.iter().map(|(k, _, _)| k.as_bytes()[0]).collect();
                let mut simulated = firsts.to_vec();
                built.sort_unstable();
                simulated.sort_unstable();
                assert_eq!(
                    built, simulated,
                    "the path simulation must agree with the tree builder",
                );
            }
            return (left.root, right.root);
        }
    }
    unreachable!("the deterministic geometry search must terminate");
}

/// A `(receiver, poisoned)` pair for version-containment tripwires: the
/// poisoned tree holds one leaf whose version escapes its declared ceiling.
///
/// An honest tree cannot take this shape — its ceiling joins every version
/// it applies — so transmitting it marks a nonconforming implementation.
/// The escaped version is built to dominate the join of both declared
/// ceilings by a 64-tick margin on *both* parties, so nothing derived from
/// the declared versions within a test's horizon — the session ceiling the
/// receiver adopts, or the receiver's own later redact ticks — ever
/// contains it. Returns the two roots plus the escaped leaf's
/// content-addressed path and its version.
pub fn uncontained_supply_pair() -> (crate::tree::Root<()>, crate::tree::Root<()>, Path, Version) {
    /// How far the escaped version outruns both declared ceilings, per
    /// party: an upper bound on the honest ticks a test performs after
    /// the pair is built.
    const ESCAPE_MARGIN: usize = 64;

    // The party pair: disjoint parties whose single-tick versions order
    // the *sender's* above the receiver's in canonical bytes, so the
    // poisoned sender wins the initiator election (live counts tie at one
    // leaf each, and greater version bytes initiate). As the initiator,
    // the sender ships the escaped leaf up front and still owes protocol
    // when the receiver aborts on ingesting it, which lets the wire-level
    // tripwires pin that the sender's session dies with its counterparty.
    // Version bytes are a function of the wire coding, so the ordered
    // pair is searched, never hardcoded.
    let single_tick = |n: usize| {
        let party = nth_party(n);
        let mut version = Version::new();
        version.tick(&party);
        (party, version)
    };
    let (receiver_party, receiver_version, sender_party, declared) = (0..8)
        .flat_map(|r| (0..8).map(move |s| (r, s)))
        .filter(|(r, s)| r != s)
        .map(|(r, s)| {
            let (receiver_party, receiver_version) = single_tick(r);
            let (sender_party, declared) = single_tick(s);
            (receiver_party, receiver_version, sender_party, declared)
        })
        .find(|(_, receiver_version, _, declared)| {
            declared.as_bytes() > receiver_version.as_bytes()
        })
        .expect("some ordered pair of single-tick versions must order by canonical bytes");

    // The receiving side's honest content: one leaf on its own party,
    // ceiling covering it, exactly as `Tree::act` would leave it.
    let receiver_message = Message::new(());
    let receiver_path = Path::for_leaf(&receiver_version, receiver_message.bytes());
    let receiver = root_with_ceiling(
        act(
            None,
            vec![(
                receiver_path,
                receiver_version.clone(),
                Action::Insert(receiver_message),
            )],
            |_| (),
        ),
        receiver_version.clone(),
    );

    // The escaped version: strictly above everything either side declared,
    // by a margin the test's own honest ticks never close.
    let mut escaped = receiver_version | &declared;
    for _ in 0..ESCAPE_MARGIN {
        escaped.tick(&receiver_party);
        escaped.tick(&sender_party);
    }
    assert!(
        !crate::tree::mirror::contained(&escaped, &declared),
        "the escaped version must not be contained in the declared version",
    );

    let message = Message::new(());
    let path = Path::for_leaf(&escaped, message.bytes());
    let poisoned = root_with_ceiling(
        act(
            None,
            vec![(path, escaped.clone(), Action::Insert(message))],
            |_| (),
        ),
        declared,
    );
    (receiver, poisoned, path, escaped)
}

/// A path all-zero except its final byte: siblings under a single leaf-parent
/// (`S<Z>`) prefix.
///
/// Real leaves are content-addressed, so two distinct messages share a
/// 31-byte prefix only under a hash-prefix collision; these hand-picked
/// paths let a test construct that shape deliberately.
fn leaf_sibling_path(last: u8) -> Path {
    let mut bytes = [0u8; 32];
    bytes[31] = last;
    Path::from(bytes)
}

/// Wrap an optional root node in a [`tree::Root`](crate::tree::Root) with the
/// given ceiling.
fn root_with_ceiling<T>(node: Option<Node<T, Root>>, ceiling: Version) -> crate::tree::Root<T> {
    crate::tree::Root {
        ceiling,
        root: node,
    }
}

/// A poisoned root for the local join seam: one leaf whose version escapes
/// `base` by a 64-tick margin on `party`, declared at the empty ceiling.
///
/// Joining it into a store whose ceiling is at or above `base` plants the
/// leaf (the escaped version defeats the join's deletion filter) while
/// leaving the store's own declared ceiling untouched — the shape only a
/// nonconforming implementation can then transmit. The margin bounds the
/// honest ticks a test may perform afterward without containing the
/// escape. Returns the root plus the escaped leaf's content-addressed path
/// and its version.
pub fn poisoned_root<T: Send + Sync>(
    party: &Party,
    base: &Version,
    message: Message<T>,
) -> (crate::tree::Root<T>, Path, Version) {
    /// How far the escaped version outruns `base`: an upper bound on the
    /// honest ticks a test performs after the root is planted.
    const ESCAPE_MARGIN: usize = 64;

    let mut escaped = base.clone();
    for _ in 0..ESCAPE_MARGIN {
        escaped.tick(party);
    }
    let path = Path::for_leaf(&escaped, message.bytes());
    let root = root_with_ceiling(
        act(
            None,
            vec![(path, escaped.clone(), Action::Insert(message))],
            |_| (),
        ),
        Version::new(),
    );
    (root, path, escaped)
}

/// A pair of trees sharing one leaf and each holding one more, all under the
/// same leaf-parent (`S<Z>`) prefix, plus the union both sides must converge
/// to.
///
/// The paths differ only in their final byte, so every level from the root
/// down to `S<Z>` holds exactly one child on each side and disputes at every
/// height: the difference survives to the closing rounds, where each side
/// must provide its own extra and absorb the other's.
pub fn leaf_parent_dispute_pair() -> (
    crate::tree::Root<()>,
    crate::tree::Root<()>,
    crate::tree::Root<()>,
) {
    // The shared leaf: one tick on party 0, literally the same node in both
    // trees (each side is built on top of `base`).
    let mut shared_version = Version::new();
    shared_version.tick(&nth_party(0));
    let base = act(
        None,
        vec![(
            leaf_sibling_path(0x00),
            shared_version.clone(),
            Action::Insert(Message::new(())),
        )],
        |_| (),
    );

    // Each side's extra rides its own disjoint party, so both extras are
    // causally concurrent with everything else and survive deletion-pruning.
    let mut a_version = Version::new();
    a_version.tick(&nth_party(1));
    let a_node = act(
        base.clone(),
        vec![(
            leaf_sibling_path(0x01),
            a_version.clone(),
            Action::Insert(Message::new(())),
        )],
        |_| (),
    );

    let mut b_version = Version::new();
    b_version.tick(&nth_party(2));
    let b_extra = (
        leaf_sibling_path(0x02),
        b_version.clone(),
        Action::Insert(Message::new(())),
    );
    let b_node = act(base, vec![b_extra.clone()], |_| ());

    let union = act(a_node.clone(), vec![b_extra], |_| ());

    let a_ceiling = shared_version.clone() | a_version;
    let b_ceiling = shared_version | b_version;
    let expected = root_with_ceiling(union, a_ceiling.clone() | b_ceiling.clone());
    (
        root_with_ceiling(a_node, a_ceiling),
        root_with_ceiling(b_node, b_ceiling),
        expected,
    )
}

/// A pair of trees where `b` has redacted the one leaf `a` still holds, and
/// concurrently inserted a sibling under the same leaf-parent (`S<Z>`)
/// prefix, plus the tree both sides must converge to.
///
/// The redacted leaf's version is causally at or before `b`'s ceiling while
/// `b` lacks the leaf, so reconciliation must delete it from `a` too — with
/// no tombstone to say so, only the version bounds. The surviving tree is
/// `b`'s: the concurrent insert alone.
pub fn leaf_parent_redaction_pair() -> (
    crate::tree::Root<()>,
    crate::tree::Root<()>,
    crate::tree::Root<()>,
) {
    // a's only leaf, on party 0.
    let mut a_version = Version::new();
    a_version.tick(&nth_party(0));
    let a_node = act(
        None,
        vec![(
            leaf_sibling_path(0x00),
            a_version.clone(),
            Action::Insert(Message::new(())),
        )],
        |_| (),
    );

    // b: built on a's history, inserts a concurrent sibling, then forgets
    // a's leaf. The forget leaves no tombstone; b remembers only through its
    // ceiling, which dominates the forgotten leaf's version.
    let mut b_version = Version::new();
    b_version.tick(&nth_party(1));
    let b_insert = (
        leaf_sibling_path(0x01),
        b_version.clone(),
        Action::Insert(Message::new(())),
    );
    let mut forget_version = b_version.clone();
    forget_version.tick(&nth_party(1));
    let b_node = act(
        act(a_node.clone(), vec![b_insert.clone()], |_| ()),
        vec![(
            leaf_sibling_path(0x00),
            forget_version.clone(),
            Action::Forget,
        )],
        |_| (),
    );

    let survivor = act(None, vec![b_insert], |_| ());

    let b_ceiling = a_version.clone() | forget_version;
    let expected = root_with_ceiling(survivor, a_version.clone() | b_ceiling.clone());
    (
        root_with_ceiling(a_node, a_version),
        root_with_ceiling(b_node, b_ceiling),
        expected,
    )
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
