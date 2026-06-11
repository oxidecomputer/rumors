use std::collections::{BTreeSet, HashMap};

use bytes::Bytes;
use proptest::prelude::*;

use super::typed::{Hash, Path, hash::Hasher};
use super::*;
use crate::message::Message;

impl Arbitrary for Key {
    type Parameters = ();
    type Strategy = BoxedStrategy<Key>;

    fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
        any::<[u8; 32]>().prop_map(Key).boxed()
    }
}

/// Wrap a `Bytes` value as a `Message<Bytes>` with its cached serialization.
/// Tests speak in terms of raw `Bytes`, but the tree's API takes
/// `Message<T>`, so every insert goes through this one-liner.
fn msg(b: Bytes) -> Message<Bytes> {
    Message::new(b)
}

/// Wrap a value as the insert action the tree accepts, with its cached
/// serialization.
fn insert_action(b: Bytes) -> Action<Bytes> {
    Action::Insert(msg(b))
}

/// Generate a vector of distinct `Bytes`, deduplicated so every element maps
/// to a unique leaf path when inserted under the same party and version. Many
/// of the hash-invariance properties below are only meaningful when no two
/// inserts collide by path; collision semantics are exercised separately.
fn distinct_bytes(max: usize) -> impl Strategy<Value = Vec<Bytes>> {
    proptest::collection::hash_set(any::<Vec<u8>>(), 0..=max)
        .prop_map(|s| s.into_iter().map(Bytes::from).collect())
}

/// Generate a vector of distinct `Bytes` along with a permutation of itself,
/// so tests can assert that the tree is invariant under the order in which
/// actions are supplied.
fn distinct_bytes_and_permutation(max: usize) -> impl Strategy<Value = (Vec<Bytes>, Vec<Bytes>)> {
    distinct_bytes(max)
        .prop_flat_map(|base| {
            let n = base.len();
            (Just(base), proptest::collection::vec(any::<u64>(), n))
        })
        .prop_map(|(base, keys)| {
            let mut pairs: Vec<_> = base.clone().into_iter().zip(keys).collect();
            pairs.sort_by_key(|(_, k)| *k);
            let shuffled = pairs.into_iter().map(|(b, _)| b).collect();
            (base, shuffled)
        })
}

/// Map a human-readable party label to a small disjoint-party index. The
/// distinct labels the tests use ("A"/"B"/"C"/"P", or proptest-generated
/// strings) map to distinct indices, so [`party_of`] yields mutually
/// disjoint parties.
fn idx(label: impl AsRef<[u8]>) -> usize {
    label.as_ref().first().map_or(0, |b| {
        (b.to_ascii_lowercase().wrapping_sub(b'a') as usize) % 16
    })
}

/// The disjoint [`Party`] for a label (see [`crate::tree::arb::nth_party`]).
/// Distinct labels give disjoint parties, hence causally-concurrent histories.
fn party_of(label: impl AsRef<[u8]>) -> before::Party {
    crate::tree::arb::nth_party(idx(label))
}

/// Build the [`Version`] a party reaches after `ticks` events: tick its
/// disjoint party `ticks` times from the empty version.
fn version_for(party: impl AsRef<[u8]>, ticks: u64) -> Version {
    let p = party_of(party);
    let mut v = Version::new();
    let mut batch = v.batch();
    for _ in 0..ticks {
        batch.tick(&p);
    }
    drop(batch);
    v
}

/// Compute the leaf-path `Key` that `Tree::act` assigns for an insert of
/// `value` at the version a party reaches after `scalar` events. The path is
/// derived from the version's canonical bytes (see [`Path::for_leaf`]), and the
/// tree hashes over the *serialized* message bytes, so we feed the cached
/// serialization through. This matches what the tree derives internally for the
/// same post-tick version.
fn leaf_path(party: impl AsRef<[u8]>, scalar: u64, value: &Bytes) -> Key {
    Path::for_leaf(&version_for(party, scalar), msg(value.clone()).bytes()).into()
}

/// Build a versioned insert triple of the shape `Tree::react` expects:
/// `(leaf_path, version, message)`. The leaf path matches what `act` would
/// have computed for the given party label and scalar version. Wrapping the
/// boilerplate keeps the test bodies focused on the property under test.
fn insert_at(
    version: Version,
    party: impl AsRef<[u8]>,
    scalar: u64,
    value: Bytes,
) -> (Key, Version, Message<Bytes>) {
    (leaf_path(party, scalar, &value), version, msg(value))
}

/// Perform one full bidirectional synchronization step between two trees
/// using `unknown`: both sides snapshot their version vectors up front,
/// each asks the other for everything unknown relative to that snapshot,
/// and each replays the received leaves via `react`. Because the snapshots
/// are taken before any reaction, the two directions are independent and
/// can be applied in either order. Absent deletions, this is the entire
/// protocol needed for two parties to converge.
fn sync_via_unknown(a: &mut Tree<Bytes>, b: &mut Tree<Bytes>) {
    let from_a = a.unknown(b.latest());
    let from_b = b.unknown(a.latest());
    a.react(from_b);
    b.react(from_a);
}

/// One step in an interleaved two-party simulation: either party applies a
/// local batch of inserts, or the two parties perform a full bidirectional
/// sync via `unknown`. Generated as a uniform mix so the random
/// interleaving exercises every sequencing of local mutation and remote
/// exchange.
#[derive(Debug, Clone)]
enum SyncOp {
    ActA(Vec<Bytes>),
    ActB(Vec<Bytes>),
    Sync,
}

fn sync_ops_strategy(max_ops: usize, max_batch: usize) -> impl Strategy<Value = Vec<SyncOp>> {
    proptest::collection::vec(
        prop_oneof![
            distinct_bytes(max_batch).prop_map(SyncOp::ActA),
            distinct_bytes(max_batch).prop_map(SyncOp::ActB),
            Just(SyncOp::Sync),
        ],
        0..=max_ops,
    )
}

/// Compute the root hash of the fully-expanded (un-path-compressed) 256-ary
/// trie over the given set of values, recomputed independently of the
/// implementation as a ground truth. A leaf hashes to `blake3(LEAF_TAG)`; at
/// each level above, a branch hashes `blake3(BRANCH_TAG ‖ r₀ ‖ h₀ ‖ …)` over
/// its *present* children in ascending radix order (absent slots are omitted,
/// not zero-filled). The empty tree is the branch with no children. The
/// compressed tree's root hash must match this regardless of how it compresses.
fn reference_hash(values: &[(Version, Bytes)]) -> Hash {
    const LEAF_TAG: u8 = 0;
    const BRANCH_TAG: u8 = 1;

    let leaf_hash = || -> Hash {
        let mut hasher = Hasher::new();
        hasher.update(&[LEAF_TAG]);
        hasher.finalize()
    };

    let hash_branch = |children: &HashMap<u8, Hash>| -> Hash {
        let mut entries: Vec<(u8, Hash)> = children.iter().map(|(k, v)| (*k, *v)).collect();
        entries.sort_by_key(|(radix, _)| *radix);
        let mut hasher = Hasher::new();
        hasher.update(&[BRANCH_TAG]);
        for (radix, h) in entries {
            hasher.update(&[radix]);
            hasher.update(h.as_bytes());
        }
        hasher.finalize()
    };

    // Level 32 (the value level): every distinct path maps to a leaf. The tree
    // hashes over the serialized `Message` bytes, not the raw inner value, so
    // we do the same here.
    let paths: BTreeSet<Key> = values
        .iter()
        .map(|(version, value)| Path::for_leaf(version, msg(value.clone()).bytes()).into())
        .collect();

    // The empty tree is a branch with no children.
    if paths.is_empty() {
        return hash_branch(&HashMap::new());
    }

    let mut current: HashMap<Vec<u8>, Hash> = paths
        .into_iter()
        .map(|p| (<[u8; 32]>::from(typed::Path::from(p)).to_vec(), leaf_hash()))
        .collect();

    // Fold upward one level at a time: group entries by the prefix they share
    // at the next-shallower depth, then hash each group as a 256-way branch.
    for level in (0..32).rev() {
        let mut groups: HashMap<Vec<u8>, HashMap<u8, Hash>> = HashMap::new();
        for (prefix, hash) in current {
            let new_prefix = prefix[..level].to_vec();
            let byte = prefix[level];
            groups.entry(new_prefix).or_default().insert(byte, hash);
        }
        current = groups
            .into_iter()
            .map(|(prefix, children)| (prefix, hash_branch(&children)))
            .collect();
    }

    *current
        .get(&Vec::<u8>::new())
        .expect("exactly one root entry")
}

/// An empty tree's root hash must match the reference: the branch with no
/// children, `blake3(BRANCH_TAG)`.
#[test]
fn empty_tree_hash_matches_reference() {
    let tree: Tree<Bytes> = Tree::new();
    let tree_hash = tree.hash();
    let reference = reference_hash(&[]);
    assert_eq!(&tree_hash, reference.as_bytes());
}

/// A single inserted value must hash identically to the uncompressed trie
/// containing just that value. This exercises Leaf::hash path compression
/// with a maximal 31-byte leaf prefix.
#[test]
fn single_value_hash_matches_reference() {
    let value = Bytes::from(&b"hello"[..]);
    let mut tree: Tree<Bytes> = Tree::new();
    tree.act(&party_of("P"), [insert_action(value.clone())]);
    let tree_hash = tree.hash();
    let reference = reference_hash(&[(version_for("P", 1), value)]);
    assert_eq!(&tree_hash, reference.as_bytes());
}

proptest! {
    /// The compressed tree's root hash must equal the hash computed over the
    /// fully-expanded uncompressed trie, for any sequence of inserted values.
    /// This is the ground-truth invariant for path compression: the hash
    /// depends on the set of leaves, not on how the tree chooses to compress
    /// them. Each insert in the batch claims a fresh scalar version, so the
    /// reference input must mirror that per-insert numbering.
    #[test]
    fn compressed_hash_matches_reference(
        values in proptest::collection::vec(any::<Vec<u8>>(), 0..16)
            .prop_map(|v| v.into_iter().map(Bytes::from).collect::<Vec<_>>()),
    ) {
        let mut tree = Tree::new();
        tree.act(&party_of("P"), values.iter().cloned().map(insert_action));
        let reference_input: Vec<_> = values
            .into_iter()
            .enumerate()
            .map(|(i, v)| (version_for("P", (i + 1) as u64), v))
            .collect();
        let reference = reference_hash(&reference_input);
        prop_assert_eq!(&tree.hash(), reference.as_bytes());
    }

    /// A list of versioned actions applied through `react` must produce the
    /// same tree hash regardless of how the list is partitioned across react
    /// calls. This is the batching-transparency claim in `react`'s doc: the
    /// "single traversal" optimization is only a speedup, not a semantic
    /// change.
    #[test]
    fn react_batch_partitioning_preserves_hash(
        bytes in distinct_bytes(16),
        breaks in proptest::collection::vec(any::<bool>(), 0..16),
    ) {
        let party = "P".to_string();
        let version = version_for(&party, 1);

        let mut all_in_one = Tree::new();
        all_in_one.react(
            bytes
                .iter()
                .cloned()
                .map(|b| insert_at(version.clone(), &party, 1, b)));

        let mut partitioned = Tree::new();
        let mut chunk: Vec<Bytes> = Vec::new();
        for (i, b) in bytes.iter().cloned().enumerate() {
            chunk.push(b);
            let at_boundary =
                breaks.get(i).copied().unwrap_or(false) || i + 1 == bytes.len();
            if at_boundary {
                let batch: Vec<_> = std::mem::take(&mut chunk)
                    .into_iter()
                    .map(|b| insert_at(version.clone(), &party, 1, b))
                    .collect();
                partitioned.react(batch);
            }
        }

        prop_assert_eq!(all_in_one.hash(), partitioned.hash());
    }

    /// Two action sequences that end with the same set of leaves must produce
    /// the same root hash. Concretely, a sequence of individual `act` calls
    /// (each bumping the scalar version) must agree with a single `react`
    /// call that re-presents those same inserts at the versions `act`
    /// implicitly assigned them.
    #[test]
    fn act_sequence_equals_react_with_explicit_versions(
        bytes in distinct_bytes(16),
    ) {
        let mut t_act = Tree::new();
        for b in &bytes {
            t_act.act(&party_of("P"), [insert_action(b.clone())]);
        }

        let party = "P".to_string();
        let versions: Vec<Version> = (1..=bytes.len())
            .map(|i| version_for(&party, i as u64))
            .collect();

        let mut t_react = Tree::new();
        t_react.react(
            versions
                .into_iter()
                .zip(bytes.iter().cloned())
                .enumerate()
                .map(|(i, (v, b))| insert_at(v, &party, (i + 1) as u64, b)));

        prop_assert_eq!(t_act.hash(), t_react.hash());
        prop_assert_eq!(t_act.latest(), t_react.latest());
    }

    /// The size and version accessors agree with an independent walk of the
    /// tree. Inserting `n` distinct values must make `len` report `n`, `iter`
    /// yield `n` leaves, and `is_empty` track `n == 0`. `iter` is moreover an
    /// honest `ExactSizeIterator`: its reported length starts at `n` and falls
    /// by exactly one per yielded leaf, hitting zero precisely at the end.
    /// Finally `earliest`/`latest` bracket every live leaf version (`<=` in the
    /// causal order), and `earliest` is absent exactly when the tree is empty.
    #[test]
    fn size_and_version_accessors_are_consistent(bytes in distinct_bytes(16)) {
        let mut tree: Tree<Bytes> = Tree::new();
        if !bytes.is_empty() {
            tree.act(
                &party_of("P"),
                bytes.iter().cloned().map(insert_action));
        }
        let n = bytes.len();

        prop_assert_eq!(tree.len(), n);
        prop_assert_eq!(tree.is_empty(), n == 0);
        prop_assert_eq!(tree.iter().count(), n);

        // `iter()` reports an exact, monotonically-shrinking remaining count.
        let mut it = tree.iter();
        prop_assert_eq!(it.len(), n);
        let mut seen = 0usize;
        while it.len() > 0 {
            let before = it.len();
            prop_assert!(it.next().is_some());
            prop_assert_eq!(it.len(), before - 1);
            seen += 1;
        }
        prop_assert!(it.next().is_none());
        prop_assert_eq!(seen, n);

        // `earliest` is present iff non-empty, and bounds every leaf version.
        prop_assert_eq!(tree.earliest().is_none(), tree.is_empty());
        if let Some(earliest) = tree.earliest() {
            let latest = tree.latest();
            for (_, v, _) in tree.iter() {
                prop_assert!(earliest <= v);
                prop_assert!(v <= latest);
            }
        }
    }

    /// The leaf iterator is a consistent `DoubleEndedIterator`: the forward
    /// walk is in strictly ascending key order, reverse iteration yields exactly
    /// that sequence reversed, and consuming alternately from both ends visits
    /// every leaf exactly once — the ends meet in the middle with no overlap and
    /// no gap, so `front ++ reverse(back)` reconstructs the forward order.
    #[test]
    fn iter_is_double_ended(bytes in distinct_bytes(16)) {
        let mut tree: Tree<Bytes> = Tree::new();
        if !bytes.is_empty() {
            tree.act(
                &party_of("P"),
                bytes.iter().cloned().map(insert_action));
        }

        // Forward order is strictly ascending by key.
        let fwd: Vec<[u8; 32]> = tree.iter().map(|(k, _, _)| k.0).collect();
        prop_assert!(fwd.windows(2).all(|w| w[0] < w[1]));

        // Reverse iteration is the forward sequence, reversed.
        let bwd: Vec<[u8; 32]> = tree.iter().rev().map(|(k, _, _)| k.0).collect();
        let mut fwd_rev = fwd.clone();
        fwd_rev.reverse();
        prop_assert_eq!(bwd, fwd_rev);

        // Pulling alternately from each end visits every leaf once; reuniting
        // the two halves (back reversed) must rebuild the forward order.
        let mut it = tree.iter();
        let (mut front, mut back) = (Vec::new(), Vec::new());
        let mut take_front = true;
        while let Some((k, _, _)) = if take_front { it.next() } else { it.next_back() } {
            if take_front { front.push(k.0) } else { back.push(k.0) }
            take_front = !take_front;
        }
        back.reverse();
        front.extend(back);
        prop_assert_eq!(front, fwd);
    }

    /// Inserting a value and then deleting its leaf path via two separate
    /// `act` calls must leave the tree empty (the empty-tree hash), with the
    /// version two ticks along: inserts and effectual forgets each claim a
    /// fresh version, so the mirror protocol can distinguish "I forgot this"
    /// from "I never knew about it."
    #[test]
    fn insert_then_delete_is_empty(value in any::<Vec<u8>>()) {
        let party = "P".to_string();
        let value = Bytes::from(value);
        let path = leaf_path(&party, 1, &value);

        let mut tree = Tree::new();
        tree.act(&party_of("P"), [insert_action(value)]);
        tree.act(&party_of("P"), [Action::Forget(path)]);

        prop_assert_eq!(tree.hash(), *reference_hash(&[]).as_bytes());
        prop_assert_eq!(tree.latest(), version_for(&party, 2));
    }

    /// Inserting a value and deleting its leaf path within the same `act`
    /// batch must leave the tree empty (the empty-tree hash) with the version
    /// untouched. The "last action on a given path wins" rule makes the delete
    /// prevail.
    #[test]
    fn insert_and_delete_same_batch_is_empty(value in any::<Vec<u8>>()) {
        let party = "P".to_string();
        let value = Bytes::from(value);
        let path = leaf_path(&party, 1, &value);

        let mut tree = Tree::new();
        tree.act(&party_of("P"), [insert_action(value), Action::Forget(path)]);

        prop_assert_eq!(tree.hash(), *reference_hash(&[]).as_bytes());
        prop_assert_eq!(tree.latest(), Version::new());
    }

    /// Deleting a path that is not present in the tree changes neither the
    /// root hash nor the version: the leaf multiset is identical, and the
    /// tree's version absorbs a tick only from actions that have an effect.
    #[test]
    fn delete_absent_path_preserves_hash(
        bytes in distinct_bytes(8),
        nuke in any::<Key>(),
    ) {
        let party = "P".to_string();
        let present: BTreeSet<Key> = bytes
            .iter()
            .map(|b| leaf_path(&party, 1, b))
            .collect();
        prop_assume!(!present.contains(&nuke));

        let mut t_before = Tree::new();
        t_before.act(&party_of("P"), bytes.into_iter().map(insert_action));
        let mut t_after = t_before.clone();
        t_after.act(&party_of("P"), [Action::Forget(nuke)]);

        prop_assert_eq!(t_before.hash(), t_after.hash());
        prop_assert_eq!(t_before.latest(), t_after.latest());
    }

    /// A fresh tree returns no values for any requested paths: no leaves are
    /// present, so every lookup misses.
    #[test]
    fn get_on_empty_tree_is_empty(
        paths in proptest::collection::vec(any::<Key>(), 0..8),
    ) {
        let tree: Tree<Bytes> = Tree::new();
        prop_assert!(tree.get_all(paths).is_empty());
    }

    /// After inserting a set of distinct values via `act`, looking up the
    /// corresponding leaf paths must return the same multiset of values.
    /// Each insert in the batch claims its own scalar version, so we derive
    /// each path with its position-based version. Returned order is arbitrary
    /// per `get`'s contract, so we compare as sorted multisets.
    #[test]
    fn get_after_insert_returns_same_multiset(
        bytes in distinct_bytes(16),
    ) {
        let party = "P".to_string();
        let mut tree = Tree::new();
        tree.act(&party_of("P"), bytes.iter().cloned().map(insert_action));

        let paths: Vec<Key> = bytes
            .iter()
            .enumerate()
            .map(|(i, b)| leaf_path(&party, (i + 1) as u64, b))
            .collect();

        let mut got: Vec<Bytes> =
            tree.get_all(paths).into_iter()
                .map(|(_, _, m)| m.as_ref().clone())
                .collect();
        got.sort();
        let mut expected: Vec<Bytes> = bytes;
        expected.sort();
        prop_assert_eq!(got, expected);
    }

    /// Requesting a mix of present and absent paths returns exactly the
    /// values for the present ones. Absent paths contribute nothing. As with
    /// the all-present case, each insert claims its own version, so we
    /// derive each present path with its position-based version.
    #[test]
    fn get_filters_absent_paths(
        bytes in distinct_bytes(8),
        extra in proptest::collection::vec(any::<Key>(), 0..8),
    ) {
        let party = "P".to_string();
        let mut tree = Tree::new();
        tree.act(&party_of("P"), bytes.iter().cloned().map(insert_action));

        let present_paths: BTreeSet<Key> = bytes
            .iter()
            .enumerate()
            .map(|(i, b)| leaf_path(&party, (i + 1) as u64, b))
            .collect();
        // Exclude any "extra" paths that happen to collide with a real leaf.
        let absent: Vec<Key> = extra
            .into_iter()
            .filter(|p| !present_paths.contains(p))
            .collect();

        let all_paths: Vec<Key> =
            present_paths.iter().copied().chain(absent).collect();

        let mut got: Vec<Bytes> = tree
            .get_all(all_paths)
            .into_iter()
            .map(|(_, _, m)| m.as_ref().clone())
            .collect();
        got.sort();
        let mut expected: Vec<Bytes> = bytes;
        expected.sort();
        prop_assert_eq!(got, expected);
    }

    /// Every insert in an `act` batch advances the owning party's version by
    /// one, so a run of batches totalling `n` inserts leaves the tree's
    /// version exactly `n` ticks along. Each insert claims a fresh version
    /// so that content-identical messages produce distinct keys. (Effectual
    /// forgets advance the version too, pinned by
    /// `insert_then_delete_is_empty`; ineffectual ones do not, pinned by
    /// `delete_absent_path_preserves_hash`.)
    #[test]
    fn act_bumps_self_party_by_number_of_inserts(
        prior_inserts in 0usize..4,
        batch_size in 1usize..8,
    ) {
        let party = "P".to_string();
        let mut tree = Tree::new();
        for i in 0..prior_inserts {
            tree.act(&party_of(&party), [insert_action(Bytes::from(
                format!("prior-{i}").into_bytes(),
            ))]);
        }

        let actions: Vec<Action<Bytes>> = (0..batch_size)
            .map(|i| {
                insert_action(Bytes::from(format!("batch-{i}").into_bytes()))
            })
            .collect();
        tree.act(&party_of(&party), actions);

        // Each prior insert and each batch insert ticks the party once, so the
        // tree's version is exactly that many ticks of the owning party.
        prop_assert_eq!(
            tree.latest(),
            version_for(&party, (prior_inserts + batch_size) as u64),
        );
    }

    /// An empty `act` batch leaves the version vector completely unchanged.
    #[test]
    fn empty_act_is_a_version_noop(prior_batches in 0usize..4) {
        let mut tree = Tree::new();
        for i in 0..prior_batches {
            tree.act(&party_of("P"), [insert_action(Bytes::from(
                format!("prior-{i}").into_bytes(),
            ))]);
        }
        let before = tree.latest().clone();
        tree.act(&party_of("P"), std::iter::empty::<Action<Bytes>>());
        prop_assert_eq!(tree.latest(), before);
    }

    /// Two disjoint batches of versioned inserts applied via `react` must
    /// commute: the order in which the batches are applied does not change
    /// the resulting tree. "Disjoint" here is ensured by giving the two
    /// batches different scalar versions, which produces different leaf
    /// paths regardless of any overlap in values.
    #[test]
    fn react_commutative(
        bytes_a in distinct_bytes(8),
        bytes_b in distinct_bytes(8),
    ) {
        let party = "P".to_string();
        let v_a = version_for(&party, 1);
        let v_b = version_for(&party, 2);

        let mut t_ab = Tree::new();
        t_ab.react(
            bytes_a.iter().cloned().map(|b| insert_at(v_a.clone(), &party, 1, b)));
        t_ab.react(
            bytes_b.iter().cloned().map(|b| insert_at(v_b.clone(), &party, 2, b)));

        let mut t_ba = Tree::new();
        t_ba.react(
            bytes_b.iter().cloned().map(|b| insert_at(v_b.clone(), &party, 2, b)));
        t_ba.react(
            bytes_a.iter().cloned().map(|b| insert_at(v_a.clone(), &party, 1, b)));

        prop_assert_eq!(t_ab, t_ba);
    }

    /// `react` is idempotent: applying the same batch twice is identical to
    /// applying it once. This is the CRDT property that lets us re-deliver
    /// messages safely in the face of retries or out-of-order transport.
    #[test]
    fn react_idempotent(bytes in distinct_bytes(16)) {
        let party = "P".to_string();
        let v = version_for(&party, 1);

        let mut t_once = Tree::new();
        t_once.react(
            bytes.iter().cloned().map(|b| insert_at(v.clone(), &party, 1, b)));

        let mut t_twice = Tree::new();
        t_twice.react(
            bytes.iter().cloned().map(|b| insert_at(v.clone(), &party, 1, b)));
        t_twice.react(
            bytes.iter().cloned().map(|b| insert_at(v.clone(), &party, 1, b)));

        prop_assert_eq!(t_once, t_twice);
    }

    /// Replaying a history of versioned actions in any order produces the
    /// same tree, as long as the actions do not conflict on a path. Giving
    /// every action a unique scalar version makes every leaf path unique,
    /// so no last-writer-wins tie-breaking can mask a reordering bug.
    #[test]
    fn react_replay_order_invariant(
        (base, shuffled) in distinct_bytes_and_permutation(12),
    ) {
        let party = "P".to_string();
        // One distinct version per element so paths are always distinct.
        let versions: Vec<Version> = (1..=base.len())
            .map(|i| version_for(&party, i as u64))
            .collect();

        // Mapping from each value to the (version, scalar) it was "produced"
        // at, so that any permutation of (value, version) pairs addresses
        // the same leaves.
        let meta_by_value: HashMap<Bytes, (Version, u64)> = base
            .iter()
            .cloned()
            .zip(versions.iter().cloned().enumerate().map(|(i, v)| (v, (i + 1) as u64)))
            .collect();

        let mut t_base = Tree::new();
        t_base.react(base.iter().cloned().map(|b| {
            let (v, scalar) = meta_by_value.get(&b).unwrap();
            insert_at(v.clone(), &party, *scalar, b)
        }));

        let mut t_shuf = Tree::new();
        t_shuf.react(shuffled.iter().cloned().map(|b| {
            let (v, scalar) = meta_by_value.get(&b).unwrap();
            insert_at(v.clone(), &party, *scalar, b)
        }));

        prop_assert_eq!(t_base, t_shuf);
    }

    /// Strong eventual consistency: if two parties each apply their own
    /// actions locally and then cross-react to each other's recorded event
    /// history, their trees converge to the same leaf multiset, so the
    /// observable invariants (`hash()` and `latest()`) must agree.
    #[test]
    fn two_party_sec_cross_replay(
        a_inserts in distinct_bytes(4),
        b_inserts in distinct_bytes(4),
    ) {
        let a_id = "A".to_string();
        let b_id = "B".to_string();

        // Each party `act`s locally and simultaneously records the
        // `(version, key, message)` triple another party would need to
        // replay the event. This is the information a real synchronization
        // protocol would put on the wire.
        let mut tree_a: Tree<Bytes> = Tree::new();
        let mut a_events: Vec<(Key, Version, Message<Bytes>)> = Vec::new();
        for (i, value) in a_inserts.iter().enumerate() {
            let scalar = (i + 1) as u64;
            let mut recorded = tree_a.latest().clone();
            recorded.batch().tick(&party_of(&a_id));
            tree_a.act(&party_of("A"), [insert_action(value.clone())]);
            a_events.push(insert_at(recorded, &a_id, scalar, value.clone()));
        }

        let mut tree_b: Tree<Bytes> = Tree::new();
        let mut b_events: Vec<(Key, Version, Message<Bytes>)> = Vec::new();
        for (i, value) in b_inserts.iter().enumerate() {
            let scalar = (i + 1) as u64;
            let mut recorded = tree_b.latest().clone();
            recorded.batch().tick(&party_of(&b_id));
            tree_b.act(&party_of("B"), [insert_action(value.clone())]);
            b_events.push(insert_at(recorded, &b_id, scalar, value.clone()));
        }

        tree_a.react(b_events.iter().map(|(k, v, m)| (*k, v.clone(), m.clone())));
        tree_b.react(a_events.iter().map(|(k, v, m)| (*k, v.clone(), m.clone())));

        prop_assert_eq!(tree_a.latest(), tree_b.latest());
        prop_assert_eq!(tree_a.hash(), tree_b.hash());
    }

    /// `Clone` yields a tree that is structurally indistinguishable: equal
    /// under `Eq`, same version, same hash. Cloning is a pure copy, not a
    /// semantic operation.
    #[test]
    fn clone_preserves_all_observables(acts in distinct_bytes(8)) {
        let mut tree = Tree::new();
        tree.act(&party_of("P"), acts.into_iter().map(insert_action));
        let cloned = tree.clone();

        prop_assert_eq!(cloned.latest(), tree.latest());
        prop_assert_eq!(cloned.hash(), tree.hash());
        prop_assert_eq!(cloned, tree);
    }

    /// Structural equality implies hash equality. `Eq` compares root nodes
    /// directly, so if two trees are `Eq` their root hashes — a pure
    /// function of the root node — must agree. Two independently-built
    /// trees that applied the same batch of actions are expected to be
    /// equal, so the implication is exercised on its non-vacuous branch.
    #[test]
    fn eq_implies_same_hash(acts in distinct_bytes(8)) {
        let mut t1 = Tree::new();
        t1.act(&party_of("P"), acts.iter().cloned().map(insert_action));
        let mut t2 = Tree::new();
        t2.act(&party_of("P"), acts.into_iter().map(insert_action));

        prop_assert_eq!(&t1, &t2);
        prop_assert_eq!(t1.hash(), t2.hash());
    }

    /// Inserting the same value under different parties produces different
    /// leaf paths, and therefore different root hashes. The path derives from
    /// the leaf's version (never the party itself; see `Path::for_leaf`), and
    /// disjoint parties tick structurally distinct versions, so two parties
    /// can concurrently write the same value without colliding.
    #[test]
    fn same_value_different_parties_differ(value in any::<Vec<u8>>()) {
        let value = Bytes::from(value);
        let mut t_a = Tree::new();
        let mut t_b = Tree::new();
        t_a.act(&party_of("A"), [insert_action(value.clone())]);
        t_b.act(&party_of("B"), [insert_action(value)]);

        prop_assert_ne!(t_a.hash(), t_b.hash());
    }

    /// `unknown` relative to a tree's own version is always empty: every
    /// leaf's version is a subvector of the tree's version by construction
    /// (the tree's version vector is the join of every leaf's version plus
    /// every version observed via `react`), so the owner never holds a
    /// leaf that dominates its own version vector.
    #[test]
    fn unknown_relative_to_self_is_empty(
        batches in proptest::collection::vec(distinct_bytes(6), 0..4),
    ) {
        let mut tree = Tree::new();
        for batch in batches {
            tree.act(&party_of("P"), batch.into_iter().map(insert_action));
        }
        prop_assert!(tree.unknown(tree.latest()).is_empty());
    }

    /// `unknown` relative to the default (empty) version enumerates every
    /// leaf in the tree, each labeled with the exact version vector and
    /// value it was inserted at. This is the "full state transfer" case:
    /// a peer with no prior knowledge receives the entire leaf set. Each
    /// insert in the batch claims its own scalar version, so the returned
    /// versions span `1..=N` rather than all sharing a single value.
    #[test]
    fn unknown_relative_to_empty_is_everything(bytes in distinct_bytes(16)) {
        let party = "P".to_string();
        let mut tree = Tree::new();
        tree.act(&party_of("P"), bytes.iter().cloned().map(insert_action));

        let got = tree.unknown(&Version::default());

        let got_paths: BTreeSet<Key> = got.iter().map(|(p, _, _)| *p).collect();
        let expected_paths: BTreeSet<Key> = bytes
            .iter()
            .enumerate()
            .map(|(i, b)| leaf_path(&party, (i + 1) as u64, b))
            .collect();
        prop_assert_eq!(got_paths, expected_paths);

        let mut got_versions: Vec<Version> = got.iter().map(|(_, v, _)| v.clone()).collect();
        let mut expected_versions: Vec<Version> =
            (1..=bytes.len() as u64).map(|i| version_for(&party, i)).collect();
        // `Version` is only partially ordered, so compare the two multisets via
        // their canonical bytes: an arbitrary but total, deterministic key.
        got_versions.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
        expected_versions.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
        prop_assert_eq!(got_versions, expected_versions);

        let mut got_values: Vec<Bytes> =
            got.into_iter().map(|(_, _, b)| b.clone_into_inner()).collect();
        got_values.sort();
        let mut expected_values = bytes;
        expected_values.sort();
        prop_assert_eq!(got_values, expected_values);
    }

    /// Two parties that each apply local inserts and then perform one full
    /// bidirectional sync via `unknown` must converge: the same leaf
    /// multiset (equal root hash) and the same observed version vector.
    /// This is the minimal form of the synchronization invariant, without
    /// any interleaving or prior history.
    #[test]
    fn sync_converges_after_independent_acts(
        a_inserts in distinct_bytes(6),
        b_inserts in distinct_bytes(6),
    ) {
        let mut tree_a = Tree::new();
        let mut tree_b = Tree::new();
        tree_a.act(&party_of("A"), a_inserts.into_iter().map(insert_action));
        tree_b.act(&party_of("B"), b_inserts.into_iter().map(insert_action));

        sync_via_unknown(&mut tree_a, &mut tree_b);

        prop_assert_eq!(tree_a.hash(), tree_b.hash());
        prop_assert_eq!(tree_a.latest(), tree_b.latest());
    }

    /// A second sync immediately after a first is a complete no-op: the
    /// two parties already agree, so each side's `unknown` relative to the
    /// other's version is empty and neither tree changes. This rules out
    /// any "every sync shuffles state" bug and witnesses that `unknown`
    /// precisely characterizes the causal delta.
    #[test]
    fn sync_is_idempotent(
        a_inserts in distinct_bytes(6),
        b_inserts in distinct_bytes(6),
    ) {
        let mut tree_a = Tree::new();
        let mut tree_b = Tree::new();
        tree_a.act(&party_of("A"), a_inserts.into_iter().map(insert_action));
        tree_b.act(&party_of("B"), b_inserts.into_iter().map(insert_action));

        sync_via_unknown(&mut tree_a, &mut tree_b);

        let hash_a = tree_a.hash();
        let hash_b = tree_b.hash();
        let version_a = tree_a.latest().clone();
        let version_b = tree_b.latest().clone();

        // The deltas in both directions must now be empty.
        prop_assert!(tree_a.unknown(&version_b).is_empty());
        prop_assert!(tree_b.unknown(&version_a).is_empty());

        // And a second sync must leave both trees bit-identical.
        sync_via_unknown(&mut tree_a, &mut tree_b);
        prop_assert_eq!(tree_a.hash(), hash_a);
        prop_assert_eq!(tree_b.hash(), hash_b);
        prop_assert_eq!(tree_a.latest(), version_a);
        prop_assert_eq!(tree_b.latest(), version_b);
    }

    /// A one-way delivery of A's `unknown(V_B)` to B — only half of a
    /// full sync — makes B a causal superset of A: B's version dominates
    /// A's, and every leaf A holds (at the paths `act` would have written)
    /// is retrievable from B. This isolates the "receiver gains" half of
    /// the bidirectional invariant.
    #[test]
    fn one_way_sync_makes_receiver_superset(
        a_inserts in distinct_bytes(6),
        b_inserts in distinct_bytes(6),
    ) {
        let a_id = "A".to_string();
        let mut tree_a = Tree::new();
        let mut tree_b = Tree::new();
        tree_a.act(&party_of("A"), a_inserts.iter().cloned().map(insert_action));
        tree_b.act(&party_of("B"), b_inserts.iter().cloned().map(insert_action));

        tree_b.react(tree_a.unknown(tree_b.latest()));

        prop_assert!(tree_b.latest() >= tree_a.latest());

        let a_paths: Vec<Key> = a_inserts
            .iter()
            .enumerate()
            .map(|(i, b)| leaf_path(&a_id, (i + 1) as u64, b))
            .collect();
        let mut got: Vec<Bytes> = tree_b
            .get_all(a_paths)
            .into_iter()
            .map(|(_, _, m)| m.as_ref().clone())
            .collect();
        got.sort();
        let mut expected: Vec<Bytes> = a_inserts;
        expected.sort();
        prop_assert_eq!(got, expected);
    }

    /// Arbitrary interleavings of local `act` batches and bidirectional
    /// `unknown`-driven syncs converge at every sync step — not just at
    /// the end. After each sync, the two parties must agree on both hash
    /// and version vector; a final sync after the last op guarantees
    /// convergence regardless of whether the trace happened to end with
    /// a sync.
    #[test]
    fn interleaved_acts_and_syncs_converge_at_every_sync(
        ops in sync_ops_strategy(20, 4),
    ) {
        let mut tree_a = Tree::new();
        let mut tree_b = Tree::new();

        for op in ops {
            match op {
                SyncOp::ActA(values) => {
                    tree_a.act(&party_of("A"), values.into_iter().map(insert_action));
                }
                SyncOp::ActB(values) => {
                    tree_b.act(&party_of("B"), values.into_iter().map(insert_action));
                }
                SyncOp::Sync => {
                    sync_via_unknown(&mut tree_a, &mut tree_b);
                    prop_assert_eq!(tree_a.hash(), tree_b.hash());
                    prop_assert_eq!(tree_a.latest(), tree_b.latest());
                }
            }
        }

        sync_via_unknown(&mut tree_a, &mut tree_b);
        prop_assert_eq!(tree_a.hash(), tree_b.hash());
        prop_assert_eq!(tree_a.latest(), tree_b.latest());
    }

    /// Inserting the same value twice under the same party via two `act`
    /// calls produces two distinct leaves: the scalar version participates
    /// in the path, so the second insert does not overwrite the first.
    /// Both leaves hold the same value, and both are retrievable by their
    /// respective paths.
    #[test]
    fn same_value_different_versions_produce_two_leaves(value in any::<Vec<u8>>()) {
        let party = "P".to_string();
        let value = Bytes::from(value);
        let mut tree = Tree::new();
        tree.act(&party_of("P"), [insert_action(value.clone())]);
        tree.act(&party_of("P"), [insert_action(value.clone())]);

        let path_v1 = leaf_path(&party, 1, &value);
        let path_v2 = leaf_path(&party, 2, &value);

        prop_assert_ne!(path_v1, path_v2);
        let got = tree.get_all([path_v1, path_v2]);
        prop_assert_eq!(got.len(), 2);
        prop_assert!(got.iter().all(|b| b.2.as_ref() == &value));
    }

    /// `iter` enumerates exactly the live leaves: the same `(key, value)` set
    /// as `unknown` relative to the empty version (the established
    /// full-state-transfer enumeration). Interleaved redactions exercise the
    /// no-tombstone path — a redacted leaf is removed outright, so it must be
    /// absent from `iter`, not merely skipped. This pins the borrowing lazy
    /// walk against the trusted owning traversal.
    #[test]
    fn iter_matches_unknown_from_empty(
        batches in proptest::collection::vec(distinct_bytes(6), 0..4),
        redactions in proptest::collection::vec(any::<bool>(), 0..8),
    ) {
        let mut tree = Tree::new();
        for batch in batches {
            tree.act(&party_of("P"), batch.into_iter().map(insert_action));
        }

        // Redact a sampling of currently-live keys, removing their leaves.
        let live: Vec<Key> = tree.iter().map(|(k, _, _)| k).collect();
        for (i, redact) in redactions.iter().enumerate() {
            if *redact && !live.is_empty() {
                let key = live[i % live.len()];
                tree.act(&party_of("P"), [Action::Forget(key)]);
            }
        }

        let mut from_iter: Vec<(Key, Bytes)> =
            tree.iter().map(|(k, _, m)| (k, (**m).clone())).collect();
        let mut from_unknown: Vec<(Key, Bytes)> = tree
            .unknown(&Version::default())
            .into_iter()
            .map(|(k, _, m)| (k, m.message().clone()))
            .collect();
        from_iter.sort();
        from_unknown.sort();
        prop_assert_eq!(from_iter, from_unknown);
    }
}

#[test]
fn delete_nonexistent_key() {
    let mut tree: Tree<()> = Tree::new();
    tree.act(&party_of("P"), [Action::Forget(Key([0; 32]))]);
    assert_eq!(tree, Tree::new());
}

/// Project a borrowed leaf triple to an owned one, for collecting and
/// comparing walk outputs.
fn owned<T>((key, version, value): (Key, &Version, &Arc<T>)) -> (Key, Version, Arc<T>) {
    (key, version.clone(), value.clone())
}

proptest! {
    /// The walk machinery's correctness core, differentially: for arbitrary
    /// divergent trees and an arbitrary causal bound pair (every `Bound`
    /// kind, over versions sampled from both trees' leaves and ceilings plus
    /// genesis — so dominated, dominating, equal, and concurrent bounds all
    /// occur), `Tree::range` yields exactly the leaves that
    /// `before::causally`'s membership predicate admits from the unfiltered
    /// walk — the prune/promote shortcuts are pure optimization — in
    /// ascending key order; and the frozen spine walk (`Tree::freeze`)
    /// yields the identical sequence. Two independent implementations of
    /// the same partial-order semantics checking each other.
    #[test]
    fn range_and_freeze_match_the_naive_filter(
        (a, b) in crate::tree::arb::arb_divergent_pair(),
        start_sel in any::<prop::sample::Index>(),
        end_sel in any::<prop::sample::Index>(),
        start_kind in 0u8..3,
        end_kind in 0u8..3,
    ) {
        use std::ops::Bound;

        let tree = Tree { root: a };
        let other = Tree { root: b };

        // Bound candidates spanning the partial order's relationships to the
        // walked tree: its own leaf versions and ceiling (dominated/equal),
        // the divergent sibling's (concurrent), and genesis (bottom).
        let mut candidates = vec![
            Version::new(),
            tree.latest().clone(),
            other.latest().clone(),
        ];
        candidates.extend(tree.iter().map(|(_, v, _)| v.clone()));
        candidates.extend(other.iter().map(|(_, v, _)| v.clone()));

        let pick = |sel: &prop::sample::Index, kind: u8| match kind {
            0 => Bound::Unbounded,
            1 => Bound::Included(candidates[sel.index(candidates.len())].clone()),
            _ => Bound::Excluded(candidates[sel.index(candidates.len())].clone()),
        };
        let start = pick(&start_sel, start_kind);
        let end = pick(&end_sel, end_kind);

        // Ground truth: compose the equivalent `causally` range and filter
        // the unfiltered walk by its membership predicate.
        let admits = |version: &Version| {
            let mut range = crate::causally::all();
            match &start {
                Bound::Included(s) => range = range.not_before(s),
                Bound::Excluded(s) => range = range.since(s),
                Bound::Unbounded => {}
            }
            match &end {
                Bound::Included(e) => range = range.known_at(e),
                Bound::Excluded(e) => range = range.before(e),
                Bound::Unbounded => {}
            }
            range.contains(version)
        };
        let naive: Vec<_> = tree
            .iter()
            .filter(|(_, version, _)| admits(version))
            .map(owned)
            .collect();

        let ranged: Vec<_> = tree.range((start.clone(), end.clone())).map(owned).collect();
        prop_assert_eq!(&ranged, &naive, "range must equal the naive filter");
        prop_assert!(
            ranged.windows(2).all(|pair| pair[0].0 < pair[1].0),
            "range yields ascending keys",
        );

        let mut frozen = tree.freeze((start, end));
        let mut thawed = Vec::new();
        while let Some((key, leaf)) = frozen.next() {
            thawed.push((key, leaf.version().clone(), leaf.value().clone()));
        }
        prop_assert_eq!(&thawed, &naive, "the frozen walk must equal the naive filter");
    }

    /// The unfiltered walk is exact and reversible, and the point lookup
    /// resolves it: `iter`'s size hint equals the tree's length, the
    /// backward walk is the forward walk reversed, `get` finds every
    /// iterated key with the same version and value, and a perturbed key
    /// that names no leaf misses.
    #[test]
    fn iteration_and_point_lookup_agree(
        root in crate::tree::arb::arb_tree_root(0, 0..24),
        flip in any::<prop::sample::Index>(),
    ) {
        let tree = Tree { root };

        let forward: Vec<_> = tree.iter().map(owned).collect();
        prop_assert_eq!(tree.iter().len(), forward.len());
        prop_assert_eq!(tree.len(), forward.len());

        let mut backward: Vec<_> = tree.iter().rev().map(owned).collect();
        backward.reverse();
        prop_assert_eq!(&backward, &forward, "backward is forward reversed");

        for (key, version, value) in &forward {
            prop_assert_eq!(
                tree.get(key),
                Some((version, value)),
                "get resolves every iterated key",
            );
        }

        if !forward.is_empty() {
            let (key, ..) = &forward[flip.index(forward.len())];
            let mut bytes = *key.as_bytes();
            bytes[31] ^= 1;
            let perturbed = Key::from(bytes);
            // The flipped path could, in principle, name another live leaf;
            // only assert the miss when it does not.
            if !forward.iter().any(|(k, ..)| *k == perturbed) {
                prop_assert_eq!(tree.get(&perturbed), None, "a foreign key misses");
            }
        }
    }
}
