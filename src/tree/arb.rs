//! Generators and deterministic fixtures for tree tests.

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

use before::Party;
use proptest::array::uniform32;
use proptest::collection::{SizeRange, vec};
use proptest::prelude::*;
use proptest::sample::subsequence;

use crate::network::NetworkKeys;
use crate::tree::traverse::{Action, act};
use crate::tree::typed::height::Root;
use crate::tree::typed::{Node, Path};
use crate::{Version, message::Message};

/// The `index`-th party in a deterministic fork chain from one seed.
///
/// Distinct indices yield disjoint parties. Deterministic derivation lets
/// independent strategies reproduce the same parties without shared state.
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

/// Generate an ordered set of distinct child radixes with a size in `count`.
///
/// Choosing a subsequence of the complete radix space makes every requested
/// size, including a full 256-child fan, reachable without rejecting draws.
pub fn arb_radixes(count: impl Into<SizeRange>) -> impl Strategy<Value = BTreeSet<u8>> {
    subsequence((0..=u8::MAX).collect::<Vec<_>>(), count)
        .prop_map(|radixes| radixes.into_iter().collect())
}

/// Generate a nonempty set of full-width leaf paths with at most `max` members.
///
/// The small alphabet produces long shared prefixes; the full byte alphabet
/// usually produces broad, shallow branches. Folding a nonempty vector keeps
/// the result nonempty even when duplicate paths coincide.
pub fn arb_leaf_paths(max: usize) -> impl Strategy<Value = BTreeSet<[u8; 32]>> {
    prop_oneof![
        vec(uniform32(0u8..3), 1..=max),
        vec(uniform32(any::<u8>()), 1..=max),
    ]
    .prop_map(|paths| paths.into_iter().collect())
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
                    let path = Path::for_fixture_leaf(&version);
                    (path, version.clone(), Action::Insert(message))
                })
                .collect();
            act(None, actions, &mut |_, _| ())
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
                node,
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
            let mut base = Tree::<()>::default();
            base.act(
                &parties.fork(),
                (0..shared).map(|_| Action::Insert(Message::new(()))),
            );
            let keys: Vec<_> = base.iter().map(|(v, _)| v.clone()).collect();
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
                        .filter(|(_, forget)| *forget)
                        .map(|(key, _)| Action::Forget(key.clone())),
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
///
/// The roots serve in-memory reconciliation only. They place leaves at
/// addresses other than their versions' hashes, and a wire session's decoder
/// recomputes each supplied leaf's path from its version (`Path::for_leaf`)
/// and rejects such a leaf as `LeafOutsideScope`: a session between these
/// roots violates the protocol by construction.
pub fn roots_at_depth<const N: usize>(
    roots: [crate::tree::Root; N],
    depth: usize,
) -> [crate::tree::Root; N] {
    let trees = roots.map(|root| crate::tree::Tree::<()>::from_root(NetworkKeys::fixture(), root));
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
        root_with_ceiling(act(None, leaves, &mut |_, _| ()), tree.root.ceiling)
    })
}

/// A redacting pair placed below a drawn-length shared prefix.
///
/// Every version still names exactly one leaf. Only placement changes, to
/// exercise deep traversal that version hashing would almost never produce.
/// The pair serves in-memory reconciliation only, for the reason
/// [`roots_at_depth`] gives: a wire session rejects its placed leaves.
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
/// Each side's starting version is chosen so the hashed paths have this
/// geometry: [`EARLY_DISPUTE_HINT`] names the candidate, and the built trees
/// are checked against it.
pub fn early_first_child_dispute_pair() -> (crate::tree::Root, crate::tree::Root) {
    use crate::tree::{Action, Tree};

    let path_of = |version: &Version| <[u8; 32]>::from(Path::for_fixture_leaf(version));
    let attempt = EARLY_DISPUTE_HINT;
    assert!(
        early_dispute_matches(attempt, &path_of),
        "EARLY_DISPUTE_HINT is stale; `early_dispute_hint_matches_current_paths` finds a fresh one"
    );
    let at = attempt * EARLY_DISPUTE_STRIDE;
    let [left_firsts, right_firsts] = early_dispute_radices(attempt, &path_of);

    // Starting from a later ceiling shifts every leaf address as if earlier
    // content had been redacted. Build only the selected candidate.
    let burnt = |party: &Party, ticks: usize| {
        let mut version = Version::new();
        version.ticks(party, ticks);
        version
    };

    let p_a = nth_party(1);
    let p_b = nth_party(2);
    let build = |party: &Party, base: Version, live: usize| {
        let mut tree = Tree::<()>::default();
        tree.root.ceiling = base;
        tree.act(party, (0..live).map(|_| Action::Insert(Message::new(()))));
        tree
    };
    let left = build(&p_a, burnt(&p_a, at), EARLY_DISPUTE_LEFT_LEAVES);
    let right = build(&p_b, burnt(&p_b, at), EARLY_DISPUTE_RIGHT_LEAVES);

    // The simulation selected the candidate, so verify both built trees
    // reproduce it exactly.
    for (tree, mut simulated) in [(&left, left_firsts), (&right, right_firsts)] {
        let mut built: Vec<u8> = tree
            .iter()
            .map(|(version, _)| path_of(version)[0])
            .collect();
        built.sort_unstable();
        simulated.sort_unstable();
        assert_eq!(
            built, simulated,
            "the path simulation must agree with the tree builder",
        );
    }
    (left.root, right.root)
}

/// Leaves in the fixture's wider tree.
const EARLY_DISPUTE_LEFT_LEAVES: usize = 32;

/// Leaves in the fixture's narrower tree.
const EARLY_DISPUTE_RIGHT_LEAVES: usize = 8;

/// Tick distance between candidate windows, larger than either window.
const EARLY_DISPUTE_STRIDE: usize = 64;

/// Candidate chosen by searching for the geometry under the current leaf-path
/// derivation; `early_dispute_hint_matches_current_paths` checks it and
/// finds a replacement when it goes stale.
const EARLY_DISPUTE_HINT: usize = 60;

/// Derive the first radix of `leaves` successive paths after `skip` ticks.
fn first_radices(
    party: &Party,
    skip: usize,
    leaves: usize,
    path_of: &impl Fn(&Version) -> [u8; 32],
) -> Vec<u8> {
    let mut version = Version::new();
    version.ticks(party, skip);
    (0..leaves)
        .map(|_| {
            version.tick(party);
            path_of(&version)[0]
        })
        .collect()
}

/// Simulate the root radices for one candidate window.
fn early_dispute_radices(attempt: usize, path_of: &impl Fn(&Version) -> [u8; 32]) -> [Vec<u8>; 2] {
    let skip = attempt * EARLY_DISPUTE_STRIDE;
    [
        first_radices(&nth_party(1), skip, EARLY_DISPUTE_LEFT_LEAVES, path_of),
        first_radices(&nth_party(2), skip, EARLY_DISPUTE_RIGHT_LEAVES, path_of),
    ]
}

/// Whether two root-radix populations have the fixture's required geometry.
fn is_early_dispute([left, right]: [&[u8]; 2]) -> bool {
    let Some(&first) = left.iter().chain(right).min() else {
        return false;
    };
    let left_under = left.iter().filter(|&&radix| radix == first).count();
    let right_under = right.iter().filter(|&&radix| radix == first).count();
    let mut provisions: Vec<u8> = left
        .iter()
        .filter(|radix| !right.contains(radix))
        .chain(right.iter().filter(|radix| !left.contains(radix)))
        .copied()
        .filter(|&radix| radix > first)
        .collect();
    provisions.sort_unstable();
    provisions.dedup();

    // The first child is disputed and branches on at least one side. Six
    // later one-sided children keep whole-subtree supplies queued behind it.
    left_under.min(right_under) >= 1 && left_under.max(right_under) >= 2 && provisions.len() >= 6
}

/// Whether candidate `attempt` has the early-dispute geometry.
fn early_dispute_matches(attempt: usize, path_of: &impl Fn(&Version) -> [u8; 32]) -> bool {
    let [left, right] = early_dispute_radices(attempt, path_of);
    is_early_dispute([&left, &right])
}

/// Extra ticks that keep malformed fixture versions beyond later test traffic.
const ESCAPE_MARGIN: usize = 64;

/// A `(receiver, poisoned)` pair for version-containment tripwires: the
/// poisoned tree holds one leaf whose version escapes its declared ceiling.
///
/// A conforming tree cannot take this shape because its ceiling joins every
/// version it applies.
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

    // The receiver's valid content: one leaf on its own party,
    // ceiling covering it, exactly as `Tree::act` would leave it.
    let receiver_message = Message::new(());
    let receiver_path = Path::for_fixture_leaf(&receiver_version);
    let receiver = root_with_ceiling(
        act(
            None,
            vec![(
                receiver_path,
                receiver_version.clone(),
                Action::Insert(receiver_message),
            )],
            &mut |_, _| (),
        ),
        receiver_version.clone(),
    );

    // The escaped version: strictly above everything either side declared,
    // by a margin the test's later ticks never close.
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
    let path = Path::for_fixture_leaf(&escaped);
    let poisoned = root_with_ceiling(
        act(
            None,
            vec![(path, escaped.clone(), Action::Insert(message))],
            &mut |_, _| (),
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
    crate::tree::Root { ceiling, node }
}

/// A poisoned root for the local join boundary: one leaf whose version escapes
/// `base` by a 64-tick margin on `party`, declared at the empty ceiling.
///
/// Joining it into a store whose ceiling is at or above `base` plants the
/// leaf (the escaped version defeats the join's deletion filter) while
/// leaving the store's own declared ceiling untouched — the shape only a
/// nonconforming implementation can then transmit. The margin bounds the
/// ticks a test may perform afterward without containing the
/// escape. The leaf sits at its path under `key`, which must be the key of
/// the store it is joined into. Returns the root plus the escaped leaf's
/// version-derived path and its version.
pub fn poisoned_root(
    key: &crate::tree::typed::PathKey,
    party: &Party,
    base: &Version,
    message: Message,
) -> (crate::tree::Root, Path, Version) {
    let mut escaped = base.clone();
    for _ in 0..ESCAPE_MARGIN {
        escaped.tick(party);
    }
    let path = Path::for_leaf(key, &escaped);
    let root = root_with_ceiling(
        act(
            None,
            vec![(path, escaped.clone(), Action::Insert(message))],
            &mut |_, _| (),
        ),
        Version::new(),
    );
    (root, path, escaped)
}

/// Versions on `party`'s chain whose leaf paths open with each of
/// `prefixes`, one per prefix, in prefix order, at the tick counts in `hint`.
///
/// Leaf paths are keyed hashes of versions, so a fixture that needs leaves under
/// chosen prefixes takes versions a search of the chain found. Skipped
/// versions read as content the side sent and redacted, which is invisible
/// to a peer on another party. `None` means the hint is stale: some tick
/// count no longer lands under its prefix.
fn hinted_versions(party: &Party, prefixes: &[[u8; 2]], hint: &[usize]) -> Option<Vec<Version>> {
    if hint.len() != prefixes.len() {
        return None;
    }
    prefixes
        .iter()
        .zip(hint)
        .map(|(prefix, &ticks)| {
            let mut version = Version::new();
            version.ticks(party, ticks);
            let path = <[u8; 32]>::from(Path::for_fixture_leaf(&version));
            (path[..2] == prefix[..]).then_some(version)
        })
        .collect()
}

/// The prefixes the full-fan fixture's initiator needs: `[0, 1]` and
/// `[1, 0]`.
const FULL_FAN_INITIATOR_PREFIXES: [[u8; 2]; 2] = [[0, 1], [1, 0]];

/// Tick counts on party 2 reaching [`FULL_FAN_INITIATOR_PREFIXES`], found by
/// searching the chain under the current leaf-path derivation.
///
/// `full_fan_hints_match_current_paths` checks them and finds replacements
/// when they go stale.
const FULL_FAN_INITIATOR_HINT: [usize; 2] = [9_609, 4_740];

/// The prefixes the full-fan fixture's responder needs: `[0, 1]`, `[0, 2]`,
/// then every `[1, r]`.
fn full_fan_responder_prefixes() -> Vec<[u8; 2]> {
    let mut prefixes = vec![[0, 1], [0, 2]];
    prefixes.extend((0..=u8::MAX).map(|radix| [1, radix]));
    prefixes
}

/// Tick counts on party 1 reaching [`full_fan_responder_prefixes`], in
/// order, found and checked as [`FULL_FAN_INITIATOR_HINT`] is.
const FULL_FAN_RESPONDER_HINT: [usize; 258] = [
    59_036, 97_616, 57_860, 9_325, 81_292, 127_271, 639, 113_482, 50_757, 38_052, 30_149, 17_063,
    42_078, 7_189, 151_151, 234_927, 20_704, 69_426, 130_699, 60_998, 287_954, 37_950, 76_339,
    23_796, 135_434, 178_012, 107_632, 46_786, 38_680, 92_923, 70_011, 8_197, 73_877, 73_770,
    49_434, 25_554, 145_057, 39_222, 53_441, 46_421, 82_318, 39_453, 122_060, 40_044, 19_353,
    26_828, 99_748, 134_260, 24_285, 161_536, 38_302, 65_605, 30_438, 6_924, 42_625, 39_018,
    231_376, 2_995, 328_491, 114_720, 71_413, 79_460, 47_257, 10_582, 14_813, 84_495, 15_616,
    39_933, 54_156, 9_904, 47_110, 76_152, 71_623, 75_238, 15_250, 32_677, 40_450, 97_990, 627,
    22_262, 44_194, 189_998, 12_745, 48_845, 19_573, 62_953, 178_604, 43_552, 40_290, 13_741,
    1_400, 81_161, 40_265, 85_837, 12_348, 40_521, 167_457, 44_888, 52_141, 2_631, 12_094, 19_308,
    5_788, 3_307, 78_800, 391_830, 26_719, 33_005, 73_921, 79, 46_867, 11_140, 68_889, 167_427,
    95_168, 79_024, 498_285, 107_655, 20_977, 171_706, 141_311, 40_623, 73_342, 44_122, 112_131,
    118_611, 54_853, 39_740, 20_779, 57_159, 40_638, 186_764, 108_430, 4_307, 267_919, 41_217,
    26_562, 1_544, 158_423, 71_404, 110_616, 342, 38_019, 173_321, 50_941, 5_171, 153_158, 62_533,
    97_541, 56_442, 32_995, 2_778, 26_600, 87_877, 50_750, 8_090, 209_069, 14_038, 110_650, 20_170,
    76_966, 33_450, 233_502, 82_794, 43_525, 114_402, 22_026, 7_704, 86_967, 53_172, 317_843,
    152_406, 2_539, 2_961, 19_249, 72_464, 75_231, 308, 8_817, 8_850, 8_584, 41_575, 172_119,
    137_510, 65_650, 125_368, 18_445, 24_089, 46_491, 64_008, 208_790, 7_124, 51_417, 215_698,
    62_045, 4_703, 25_942, 219_324, 50_729, 103_257, 104_533, 3_319, 52_243, 82_285, 38_206,
    62_609, 20_940, 185_426, 66_730, 151_740, 54_201, 192_225, 19_115, 58_982, 31_341, 942, 3_462,
    229_353, 6_736, 17_136, 7_419, 37_007, 35_493, 110_283, 87_079, 67_310, 285_397, 56_407,
    99_089, 13_420, 39_944, 64_002, 16_853, 49_104, 55_878, 8_081, 39_240, 17_040, 25_072, 5_568,
    139_209, 60_898, 57_980, 165_128, 183_140, 88_400, 9_737, 16_708, 1_486, 76_243, 36_117,
    65_000, 19_400, 55_237, 45_560, 634, 45_321, 10_899,
];

/// A tree holding one leaf at each version, at the version's own path.
fn tree_of(versions: Vec<Version>) -> crate::tree::Root {
    let ceiling = versions
        .iter()
        .fold(Version::new(), |ceiling, version| ceiling | version.clone());
    let actions = versions
        .into_iter()
        .map(|version| {
            (
                Path::for_fixture_leaf(&version),
                version,
                Action::Insert(Message::new(())),
            )
        })
        .collect();
    root_with_ceiling(act(None, actions, &mut |_, _| ()), ceiling)
}

/// Two trees, initiator first, whose initiator answers one disputed root
/// child with a reply carrying two level-3 questions and the next with a
/// reply carrying a full fan of them.
///
/// Both sides hold root children 0 and 1, on concurrent parties, so every
/// prefix both hold differs. Under 0 the responder lists `[0, 1]`, which the
/// initiator also holds, and `[0, 2]`, which it lacks: two questions. Under 1
/// the responder lists all 256 depth-2 prefixes, of which the initiator
/// holds only `[1, 0]`: 256 questions. The initiator holds two leaves
/// against the responder's 258, so it wins the smaller-set election. Paths
/// are version hashes, at versions hinted from a search of each party's
/// chain, so the pair crosses a real wire.
///
/// The initiator's second reply asks a full fan of questions, the most one
/// reply can ask, behind the two its first reply asked.
pub fn full_fan_behind_two_pair() -> (crate::tree::Root, crate::tree::Root) {
    const STALE: &str =
        "a full-fan hint is stale; `full_fan_hints_match_current_paths` finds a fresh one";
    let initiator = hinted_versions(
        &nth_party(2),
        &FULL_FAN_INITIATOR_PREFIXES,
        &FULL_FAN_INITIATOR_HINT,
    )
    .expect(STALE);
    let responder = hinted_versions(
        &nth_party(1),
        &full_fan_responder_prefixes(),
        &FULL_FAN_RESPONDER_HINT,
    )
    .expect(STALE);
    let (initiator, responder) = (tree_of(initiator), tree_of(responder));
    (initiator, responder)
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
        &mut |_, _| (),
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
        &mut |_, _| (),
    );

    let mut b_version = Version::new();
    b_version.tick(&nth_party(2));
    let b_extra = (
        leaf_sibling_path(0x02),
        b_version.clone(),
        Action::Insert(Message::new(())),
    );
    let b_node = act(base, vec![b_extra.clone()], &mut |_, _| ());

    let union = act(a_node.clone(), vec![b_extra], &mut |_, _| ());

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
        &mut |_, _| (),
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
        act(a_node.clone(), vec![b_insert.clone()], &mut |_, _| ()),
        vec![(
            leaf_sibling_path(0x00),
            forget_version.clone(),
            Action::Forget,
        )],
        &mut |_, _| (),
    );

    let survivor = act(None, vec![b_insert], &mut |_, _| ());

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
        &mut |_, _| (),
    );

    // b remembers the forgotten leaf only through its ceiling: a forget
    // tick on its own party, joined with the leaf's version.
    let mut forget_version = Version::new();
    forget_version.tick(&nth_party(2));
    let b_ceiling = forgotten_version.clone() | forget_version;

    let a_ceiling = forgotten_version | survivor_version;
    let expected = root_with_ceiling(
        act(None, vec![survivor], &mut |_, _| ()),
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
            let a = root_with_ceiling(act(None, leaves, &mut |_, _| ()), a_ceiling);
            let b = root_with_ceiling(None, b_ceiling | forget_version);
            (a, b, forgotten.iter().filter(|&&forget| forget).count())
        })
        .boxed()
}

#[cfg(test)]
mod tests;
