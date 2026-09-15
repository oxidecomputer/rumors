use std::collections::BTreeMap;
use std::ops::Range;

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
/// the index, independent proptest strategies can each derive the same disjoint
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
) -> BoxedStrategy<Option<Node<Root>>> {
    vec(any::<()>(), leaves)
        .prop_map(move |draws| {
            // Tick this tree's party once per leaf, so the leaves carry a
            // strictly-increasing chain of versions on a single party. Each
            // leaf is placed at its version-derived path, exactly as a real
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
                    let path = Path::for_leaf(&version);
                    (path, version.clone(), Action::Insert(message))
                })
                .collect();
            act(None, actions, &mut |_| ())
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
) -> BoxedStrategy<crate::tree::Root> {
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

/// Replicas with a shared history, independent inserts, and shared-key redactions.
///
/// All parties descend from one seed. Each side retains the common history
/// even when it redacts every shared leaf, so joins must distinguish a missing
/// message from one the side has already seen and deleted.
/// `shared` and `per_side` bound the insertion counts before redaction.
pub fn arb_divergent_roots<const N: usize>(
    shared: Range<usize>,
    per_side: Range<usize>,
) -> BoxedStrategy<[crate::tree::Root; N]> {
    use crate::tree::{Action, Tree};

    let side = (per_side, vec(any::<bool>(), 0..shared.end));
    (shared, vec(side, N))
        .prop_map(|(shared, sides)| {
            let mut parties = Party::seed();
            let mut base = Tree::<()>::new();
            base.act(
                &parties.fork(),
                (0..shared).map(|_| Action::Insert(Message::new(()))),
            );
            let keys: Vec<_> = base.iter().map(|(v, _)| Path::for_leaf(v)).collect();
            let mut sides = sides.into_iter();
            std::array::from_fn(|_| {
                let (inserts, redact) = sides.next().expect("one plan per replica");
                let party = parties.fork();
                let mut tree = base.clone();
                tree.act(
                    &party,
                    (0..inserts).map(|_| Action::Insert(Message::new(()))),
                );
                tree.act(
                    &party,
                    keys.iter()
                        .zip(redact)
                        .filter_map(|(key, forget)| forget.then_some(Action::Forget(*key))),
                );
                tree.root
            })
        })
        .boxed()
}

/// A small pair with shared history, concurrent inserts, and redactions.
///
/// Counts include zero, covering empty trees, identical trees, and one-sided
/// additions or deletions as well as disagreement on both sides.
pub fn arb_divergent_pair() -> BoxedStrategy<(crate::tree::Root, crate::tree::Root)> {
    arb_divergent_roots(0..6, 0..5)
        .prop_map(|[a, b]| (a, b))
        .boxed()
}

/// A broader pair, giving wire tests more root children and shared hash prefixes.
///
/// More leaves increase the chance that a reply mixes whole-subtree supplies
/// with disputed children. This samples breadth; tests requiring a particular
/// deep shape use a constructed fixture instead of searching on every draw.
pub fn arb_wide_divergent_pair() -> BoxedStrategy<(crate::tree::Root, crate::tree::Root)> {
    arb_divergent_roots(0..12, 0..40)
        .prop_map(|[a, b]| (a, b))
        .boxed()
}

/// Place unit-valued replicas' leaves below a shared prefix of `depth` bytes.
///
/// One distinct radix per version preserves unique addresses and shared leaves
/// across replicas. The versions and causal ceilings stay unchanged, so the
/// same history can exercise deletion filtering at any tree height without
/// searching for a cryptographic hash collision. At most 256 distinct versions
/// may be present, and `depth` must be less than the path length.
pub fn roots_at_depth<const N: usize>(
    roots: [crate::tree::Root; N],
    depth: usize,
) -> [crate::tree::Root; N] {
    let trees = roots.map(crate::tree::Tree::<()>::from_root);
    let mut paths: BTreeMap<_, _> = trees
        .iter()
        .flat_map(|tree| {
            tree.iter()
                .map(|(v, _)| (v.as_bytes().to_vec(), Path::from([0; 32])))
        })
        .collect();
    for (radix, path) in paths.values_mut().enumerate() {
        let mut bytes = [0; 32];
        bytes[depth] = u8::try_from(radix).expect("one radix per distinct version");
        *path = bytes.into();
    }
    trees.map(|tree| {
        let leaves = tree
            .iter()
            .map(|(version, _)| {
                (
                    paths[version.as_bytes()],
                    version.clone(),
                    Action::Insert(Message::new(())),
                )
            })
            .collect();
        root_with_ceiling(act(None, leaves, &mut |_| ()), tree.root.ceiling)
    })
}

/// A redacting pair placed below a drawn-length shared prefix.
///
/// Every version still names exactly one leaf. Only placement changes, to
/// exercise deep traversal that version hashing would almost never produce.
pub fn arb_deep_divergent_pair() -> BoxedStrategy<(crate::tree::Root, crate::tree::Root)> {
    (arb_divergent_pair(), 0usize..32)
        .prop_map(|((a, b), depth)| {
            let [a, b] = roots_at_depth([a, b], depth);
            (a, b)
        })
        .boxed()
}

/// A version-addressed pair whose first root child requires deeper reconciliation.
///
/// Both sides hold divergent content under that child, with branching on at
/// least one side. Later root children include whole-subtree supplies queued
/// behind the dispute, exercising progress when descent and supplies share a
/// reply stream.
///
/// The search varies each side's starting version until the hashed paths have
/// this geometry. It is deterministic and checks its prediction against the
/// constructed trees.
pub fn early_first_child_dispute_pair() -> (crate::tree::Root, crate::tree::Root) {
    use crate::tree::{Action, Tree};

    /// Left-side leaves: enough for wide roots with shared hash prefixes.
    const LEFT_LEAVES: usize = 32;
    /// Right-side leaves: few enough that most left children are supplies.
    const RIGHT_LEAVES: usize = 8;

    /// Window stride between attempts: larger than either window, so
    /// successive attempts draw fully disjoint leaf populations.
    const STRIDE: usize = 64;

    /// Maximum candidate windows to search; exhaustion panics.
    /// Precomputation cost is proportional to this bound.
    const ATTEMPTS: usize = 2048;

    // Paths depend only on versions. Starting from a later ceiling shifts all
    // the leaf addresses, as if earlier content had been redacted. Precompute
    // each party's first radix bytes and inspect windows of that sequence;
    // build trees only for the first window satisfying the geometry.
    let firsts = |party: &Party, ticks: usize| -> Vec<u8> {
        let mut version = Version::new();
        (0..ticks)
            .map(|_| {
                version.tick(party);
                let path: [u8; 32] = Path::for_leaf(&version).into();
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
                let mut tree = Tree::<()>::new();
                tree.root.ceiling = base;
                tree.act(party, (0..live).map(|_| Action::Insert(Message::new(()))));
                tree
            };
            let left = build(&p_a, burnt(&p_a, at), LEFT_LEAVES);
            let right = build(&p_b, burnt(&p_b, at), RIGHT_LEAVES);
            // Both sides' geometry was judged from the simulation, so both
            // sides must agree with the honestly built trees.
            for (tree, firsts) in [(&left, left_firsts), (&right, right_firsts)] {
                let mut built: Vec<u8> = tree
                    .iter()
                    .map(|(v, _)| <[u8; 32]>::from(Path::for_leaf(v))[0])
                    .collect();
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

/// Extra ticks in malformed fixtures, exceeding their tests' later honest ticks.
const ESCAPE_MARGIN: usize = 64;

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
/// version-derived path and its version.
pub fn uncontained_supply_pair() -> (crate::tree::Root, crate::tree::Root, Path, Version) {
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
    let receiver_path = Path::for_leaf(&receiver_version);
    let receiver = root_with_ceiling(
        act(
            None,
            vec![(
                receiver_path,
                receiver_version.clone(),
                Action::Insert(receiver_message),
            )],
            &mut |_| (),
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
    let path = Path::for_leaf(&escaped);
    let poisoned = root_with_ceiling(
        act(
            None,
            vec![(path, escaped.clone(), Action::Insert(message))],
            &mut |_| (),
        ),
        declared,
    );
    (receiver, poisoned, path, escaped)
}

/// A path all-zero except its final byte: siblings under a single leaf-parent
/// (`S<Z>`) prefix.
///
/// Real leaves are version-addressed, so two distinct messages share a
/// 31-byte prefix only under a hash-prefix collision; these hand-picked
/// paths let a test construct that shape deliberately.
pub fn leaf_sibling_path(last: u8) -> Path {
    let mut bytes = [0u8; 32];
    bytes[31] = last;
    Path::from(bytes)
}

/// Wrap an optional root node in a [`tree::Root`](crate::tree::Root) with the
/// given ceiling.
fn root_with_ceiling(node: Option<Node<Root>>, ceiling: Version) -> crate::tree::Root {
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
/// escape. Returns the root plus the escaped leaf's version-derived path
/// and its version.
pub fn poisoned_root(
    party: &Party,
    base: &Version,
    message: Message,
) -> (crate::tree::Root, Path, Version) {
    let mut escaped = base.clone();
    for _ in 0..ESCAPE_MARGIN {
        escaped.tick(party);
    }
    let path = Path::for_leaf(&escaped);
    let root = root_with_ceiling(
        act(
            None,
            vec![(path, escaped.clone(), Action::Insert(message))],
            &mut |_| (),
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
pub fn leaf_parent_dispute_pair() -> (crate::tree::Root, crate::tree::Root, crate::tree::Root) {
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
        &mut |_| (),
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
        &mut |_| (),
    );

    let mut b_version = Version::new();
    b_version.tick(&nth_party(2));
    let b_extra = (
        leaf_sibling_path(0x02),
        b_version.clone(),
        Action::Insert(Message::new(())),
    );
    let b_node = act(base, vec![b_extra.clone()], &mut |_| ());

    let union = act(a_node.clone(), vec![b_extra], &mut |_| ());

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
pub fn leaf_parent_redaction_pair() -> (crate::tree::Root, crate::tree::Root, crate::tree::Root) {
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
        &mut |_| (),
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
        act(a_node.clone(), vec![b_insert.clone()], &mut |_| ()),
        vec![(
            leaf_sibling_path(0x00),
            forget_version.clone(),
            Action::Forget,
        )],
        &mut |_| (),
    );

    let survivor = act(None, vec![b_insert], &mut |_| ());

    let b_ceiling = a_version.clone() | forget_version;
    let expected = root_with_ceiling(survivor, a_version.clone() | b_ceiling.clone());
    (
        root_with_ceiling(a_node, a_version),
        root_with_ceiling(b_node, b_ceiling),
        expected,
    )
}

/// A pair where `a` holds two concurrent leaves under one leaf-parent
/// (`S<Z>`) prefix and `b` has forgotten one of them and never held the
/// other, plus the tree both sides must converge to.
///
/// `b` holds nothing under the parent, so the parent is never disputed:
/// the streaming filter judges `a`'s leaves one at a time, at leaf height,
/// against `b`'s ceiling, which dominates the forgotten leaf's version and
/// is concurrent with the other's. The survivor is the concurrent leaf
/// alone.
pub fn forgotten_sibling_pair() -> (crate::tree::Root, crate::tree::Root, crate::tree::Root) {
    let mut forgotten_version = Version::new();
    forgotten_version.tick(&nth_party(0));
    let mut survivor_version = Version::new();
    survivor_version.tick(&nth_party(1));
    let survivor = (
        leaf_sibling_path(0x01),
        survivor_version.clone(),
        Action::Insert(Message::new(())),
    );
    let a_node = act(
        None,
        vec![
            (
                leaf_sibling_path(0x00),
                forgotten_version.clone(),
                Action::Insert(Message::new(())),
            ),
            survivor.clone(),
        ],
        &mut |_| (),
    );

    // b remembers the forgotten leaf only through its ceiling: a forget
    // tick on its own party, joined with the leaf's version.
    let mut forget_version = Version::new();
    forget_version.tick(&nth_party(2));
    let b_ceiling = forgotten_version.clone() | forget_version;

    let a_ceiling = forgotten_version | survivor_version;
    let expected = root_with_ceiling(
        act(None, vec![survivor], &mut |_| ()),
        a_ceiling.clone() | b_ceiling.clone(),
    );
    (
        root_with_ceiling(a_node, a_ceiling),
        root_with_ceiling(None, b_ceiling),
        expected,
    )
}

/// Generate a pair where `a` holds concurrent sibling leaves under one
/// leaf-parent prefix and `b` has forgotten a drawn subset of them and
/// never held the rest, plus the count of forgotten leaves.
///
/// Each leaf sits on its own party. The general form of
/// [`forgotten_sibling_pair`]: every subset, the empty one (nothing to
/// shed) and the full one (the whole parent sheds without a leaf-height
/// verdict) included.
pub fn arb_forgotten_siblings() -> BoxedStrategy<(crate::tree::Root, crate::tree::Root, usize)> {
    vec(any::<bool>(), 1..=8)
        .prop_map(|forgotten| {
            let mut leaves = Vec::new();
            let mut a_ceiling = Version::new();
            let mut b_ceiling = Version::new();
            for (index, &forget) in forgotten.iter().enumerate() {
                let mut version = Version::new();
                version.tick(&nth_party(index));
                a_ceiling |= version.clone();
                if forget {
                    b_ceiling |= version.clone();
                }
                leaves.push((
                    leaf_sibling_path(index as u8),
                    version,
                    Action::Insert(Message::new(())),
                ));
            }
            let mut forget_version = Version::new();
            forget_version.tick(&nth_party(forgotten.len()));
            let a = root_with_ceiling(act(None, leaves, &mut |_| ()), a_ceiling);
            let b = root_with_ceiling(None, b_ceiling | forget_version);
            (a, b, forgotten.iter().filter(|&&forget| forget).count())
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
