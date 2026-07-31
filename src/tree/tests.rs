use std::collections::{BTreeSet, HashMap};

use bytes::Bytes;
use proptest::prelude::*;

use super::typed::{Hash, Path, hash::Hasher, untyped};
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
/// to a unique leaf path when inserted under the same party and version.
///
/// Many of the hash-invariance properties below are only meaningful when no two
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

/// Map a human-readable party label to a small disjoint-party index.
///
/// The distinct labels the tests use ("A"/"B"/"C"/"P", or proptest-generated
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

/// Build the [`Version`] a party reaches after `ticks` events: advance
/// its disjoint party by the whole count from the empty version.
fn version_for(party: impl AsRef<[u8]>, ticks: u64) -> Version {
    let p = party_of(party);
    let mut v = Version::new();
    v.ticks(&p, ticks);
    v
}

/// Compute the leaf-path `Key` that `Tree::act` assigns for an insert of
/// `value` at the version a party reaches after `scalar` events.
///
/// The path is derived from the version's canonical bytes (see
/// [`Path::for_leaf`]), and the tree hashes over the *serialized* message
/// bytes, so we feed the cached serialization through. This matches what the
/// tree derives internally for the same post-tick version.
fn leaf_path(party: impl AsRef<[u8]>, scalar: u64, value: &Bytes) -> Key {
    Path::for_leaf(&version_for(party, scalar), msg(value.clone()).bytes()).into()
}

/// Build a versioned insert triple of the shape `Tree::react` expects:
/// `(leaf_path, version, message)`.
///
/// The leaf path matches what `act` would have computed for the given party
/// label and scalar version. Wrapping the boilerplate keeps the test bodies
/// focused on the property under test.
fn insert_at(
    version: Version,
    party: impl AsRef<[u8]>,
    scalar: u64,
    value: Bytes,
) -> (Key, Version, Message<Bytes>) {
    (leaf_path(party, scalar, &value), version, msg(value))
}

/// Compute the root hash of the canonical maximally-compressed trie over the
/// given set of values, recomputed independently of the implementation as a
/// ground truth.
///
/// The canonical shape is derived directly from the sorted leaf-path set: a
/// lone path below `depth` is a leaf committing its remaining suffix, and
/// otherwise the run's shared span up to its first divergence byte is the
/// branch's compressed prefix, with one child recursing per divergence
/// radix — so every branch has >= 2 children and maximal prefixes by
/// construction. Preimages are assembled with literal tag bytes,
/// `LEAF_TAG ‖ len ‖ suffix` and
/// `BRANCH_TAG ‖ len ‖ prefix ‖ count(u16 BE) ‖ (radix ‖ hash)*`, each hash
/// truncated to its leading
/// [`MERKLE_HASH_LEN`](crate::tree::typed::hash::MERKLE_HASH_LEN) bytes. The
/// empty tree is the prefixless branch with no children. The tree's root
/// hash must match this however its content arrived.
fn reference_hash(values: &[(Version, Bytes)]) -> Hash {
    const LEAF_TAG: u8 = 0;
    const BRANCH_TAG: u8 = 1;

    fn hash_at(depth: usize, paths: &[[u8; 32]]) -> Hash {
        if let [path] = paths {
            let mut hasher = Hasher::new();
            hasher.update(&[LEAF_TAG, (32 - depth) as u8]);
            hasher.update(&path[depth..]);
            return hasher.finalize().truncate();
        }

        // Two or more distinct sorted paths diverge at the first byte where
        // the least and greatest differ; the span from `depth` up to that
        // byte is the branch's compressed prefix.
        let first = paths.first().expect("a run is non-empty");
        let last = paths.last().expect("a run is non-empty");
        let branch_at = (depth..32)
            .find(|&at| first[at] != last[at])
            .expect("distinct 32-byte paths diverge before the bottom");

        let mut records: Vec<(u8, Hash)> = Vec::new();
        let mut rest = paths;
        while let Some(radix) = rest.first().map(|path| path[branch_at]) {
            let split = rest
                .iter()
                .position(|path| path[branch_at] != radix)
                .unwrap_or(rest.len());
            let (group, tail) = rest.split_at(split);
            records.push((radix, hash_at(branch_at + 1, group)));
            rest = tail;
        }

        let mut hasher = Hasher::new();
        hasher.update(&[BRANCH_TAG, (branch_at - depth) as u8]);
        hasher.update(&first[depth..branch_at]);
        let count = u16::try_from(records.len()).expect("fan-out is at most 256");
        hasher.update(&count.to_be_bytes());
        for (radix, hash) in records {
            hasher.update(&[radix]);
            hasher.update(hash.as_bytes());
        }
        hasher.finalize().truncate()
    }

    // Level 32 (the value level): every distinct path maps to a leaf. The tree
    // hashes over the serialized `Message` bytes, not the raw inner value, so
    // we do the same here.
    let paths: BTreeSet<Key> = values
        .iter()
        .map(|(version, value)| Path::for_leaf(version, msg(value.clone()).bytes()).into())
        .collect();

    if paths.is_empty() {
        // The empty tree: a prefixless branch with no children.
        let mut hasher = Hasher::new();
        hasher.update(&[BRANCH_TAG, 0, 0, 0]);
        return hasher.finalize().truncate();
    }

    let paths: Vec<[u8; 32]> = paths
        .into_iter()
        .map(|p| <[u8; 32]>::from(typed::Path::from(p)))
        .collect();
    hash_at(0, &paths)
}

/// An empty tree's root hash must match the reference: the prefixless branch
/// with no children, `blake3(BRANCH_TAG ‖ 0 ‖ 0u16)`.
#[test]
fn empty_tree_hash_matches_reference() {
    let tree: Tree<Bytes> = Tree::new();
    let tree_hash = tree.hash();
    let reference = reference_hash(&[]);
    assert_eq!(&tree_hash, reference.as_bytes());
}

/// A single inserted value must hash identically to the reference leaf whose
/// suffix is its entire 32-byte path — the maximal compressed span, pinned
/// so the length-tagged suffix commitment covers its extreme.
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
    /// The tree's root hash must equal the reference hash derived
    /// independently from the leaf-path set alone, for any sequence of
    /// inserted values.
    ///
    /// This is the ground-truth invariant for hashing: the hash is a pure
    /// function of the set of leaves, reachable only through the canonical
    /// compressed shape the reference re-derives from scratch. Each insert
    /// in the batch claims a fresh scalar version, so the reference input
    /// must mirror that per-insert numbering.
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

    /// Canonicity: the tree's shape is a pure function of its live leaf set.
    ///
    /// The same final leaf set — reached by different insertion orders,
    /// different react-batch partitionings, a join of two disjoint halves,
    /// and a detour through extra leaves that are then redacted — must
    /// produce byte-identical serialized root nodes (the serialization *is*
    /// the structure: maximal prefixes, >= 2-child branches) and equal root
    /// hashes, all matching the canonical bulk construction over the sorted
    /// leaf set. The single-preimage hash rule commits the compressed shape
    /// directly, so cross-peer hash agreement rests on exactly this
    /// invariant.
    ///
    /// Every leaf rides its own disjoint party, so all versions are pairwise
    /// concurrent: no reordering across react batches or join sides can
    /// trip deletion honoring (a leaf whose version the tree already
    /// dominates would be pruned as forgotten — real CRDT semantics, but
    /// not the property under test here).
    #[test]
    fn tree_shape_is_canonical_in_the_leaf_set(
        (kept, shuffled) in distinct_bytes_and_permutation(10),
        extras in distinct_bytes(4),
        split in any::<prop::sample::Index>(),
    ) {
        // One tick of the leaf's own disjoint party; kept leaf indices come
        // from base order, extras continue the numbering beyond them.
        let event = |index: usize, b: &Bytes| -> (Key, Version, Message<Bytes>) {
            let mut version = Version::new();
            version.tick(&crate::tree::arb::nth_party(index));
            let message = msg(b.clone());
            let key = Path::for_leaf(&version, message.bytes()).into();
            (key, version, message)
        };
        let index_of: HashMap<Bytes, usize> = kept
            .iter()
            .cloned()
            .enumerate()
            .map(|(i, b)| (b, i))
            .collect();
        let versioned = |b: &Bytes| event(index_of[b], b);
        let cut = split.index(kept.len() + 1);

        // Route A: one react batch, base order.
        let mut direct = Tree::new();
        direct.react(kept.iter().map(versioned));

        // Route B: shuffled order, split into two batches, with the extra
        // leaves inserted in between and redacted again afterwards.
        let extra_events: Vec<(Key, Version, Message<Bytes>)> = extras
            .iter()
            .enumerate()
            .map(|(i, b)| event(kept.len() + i, b))
            .collect();
        let mut detoured = Tree::new();
        detoured.react(shuffled[..cut].iter().map(versioned));
        detoured.react(extra_events.iter().map(|(k, v, m)| (*k, v.clone(), m.clone())));
        detoured.react(shuffled[cut..].iter().map(versioned));
        detoured.act(
            &party_of("P"),
            extra_events.iter().rev().map(|(k, _, _)| Action::Forget(*k)),
        );

        // Route C: two disjoint halves, merged in memory.
        let mut joined = Tree::new();
        joined.react(kept[..cut].iter().map(versioned));
        let mut right = Tree::new();
        right.react(kept[cut..].iter().map(versioned));
        joined.join(right);

        let serialize = |tree: &Tree<Bytes>| -> Option<Vec<u8>> {
            tree.root
                .root
                .as_ref()
                .map(|node| borsh::to_vec(node).expect("node serializes"))
        };
        let expected = serialize(&direct);
        prop_assert_eq!(&serialize(&detoured), &expected);
        prop_assert_eq!(&serialize(&joined), &expected);
        prop_assert_eq!(detoured.hash(), direct.hash());
        prop_assert_eq!(joined.hash(), direct.hash());

        // The canonical bulk construction over the sorted live leaf set.
        let mut entries: Vec<([u8; 32], Option<untyped::Node<Bytes>>)> = direct
            .iter()
            .map(|(key, version, value)| {
                (
                    key.0,
                    Some(untyped::Node::leaf(
                        version.clone(),
                        Message::new((**value).clone()),
                    )),
                )
            })
            .collect();
        let canonical =
            (!entries.is_empty()).then(|| untyped::Node::from_sorted_leaves(0, &mut entries));
        let canonical_bytes = canonical.as_ref().map(|node| {
            let mut buf = Vec::new();
            node.serialize_to(&mut buf).expect("node serializes");
            buf
        });
        prop_assert_eq!(&canonical_bytes, &expected);
        if let Some(node) = canonical {
            prop_assert_eq!(*node.hash().as_bytes(), direct.hash());
        }
    }

    /// A list of versioned actions applied through `react` must produce the
    /// same tree hash regardless of how the list is partitioned across react
    /// calls.
    ///
    /// This is the batching-transparency claim in `react`'s doc: the
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
    /// the same root hash.
    ///
    /// Concretely, a sequence of individual `act` calls
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
    /// tree.
    ///
    /// Inserting `n` distinct values must make `len` report `n`, `iter`
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

    /// The leaf iterator is a consistent `DoubleEndedIterator`.
    ///
    /// The forward
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
    /// version two ticks along.
    ///
    /// Inserts and effectual forgets each claim a
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

    /// Every insert in an `act` batch advances the owning party's version by
    /// one, so a run of batches totalling `n` inserts leaves the tree's
    /// version exactly `n` ticks along.
    ///
    /// Each insert claims a fresh version
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
    /// the resulting tree.
    ///
    /// "Disjoint" here is ensured by giving the two
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
    /// same tree, as long as the actions do not conflict on a path.
    ///
    /// Giving every action a unique scalar version makes every leaf path
    /// unique, so no last-writer-wins tie-breaking can mask a reordering bug.
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
    /// history, their trees converge to the same leaf multiset.
    ///
    /// The observable invariants (`hash()` and `latest()`) must agree.
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
            recorded.tick(&party_of(&a_id));
            tree_a.act(&party_of("A"), [insert_action(value.clone())]);
            a_events.push(insert_at(recorded, &a_id, scalar, value.clone()));
        }

        let mut tree_b: Tree<Bytes> = Tree::new();
        let mut b_events: Vec<(Key, Version, Message<Bytes>)> = Vec::new();
        for (i, value) in b_inserts.iter().enumerate() {
            let scalar = (i + 1) as u64;
            let mut recorded = tree_b.latest().clone();
            recorded.tick(&party_of(&b_id));
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

    /// Structural equality implies hash equality.
    ///
    /// `Eq` compares root nodes
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
    /// leaf paths, and therefore different root hashes.
    ///
    /// The path derives from
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

    /// Inserting the same value twice under the same party via two `act`
    /// calls produces two distinct leaves: the scalar version participates
    /// in the path, so the second insert does not overwrite the first.
    ///
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
        let got = [tree.get(&path_v1).unwrap(), tree.get(&path_v2).unwrap()];
        prop_assert!(got.iter().all(|b| b.1[..] == *value));
    }
}

/// Forgetting a key the tree never held is a complete no-op: no leaf, and
/// — because the action was zero-effect — no version bump either, so the
/// tree stays equal to a fresh one.
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
    /// The walk machinery's correctness core, differentially.
    ///
    /// For arbitrary
    /// divergent trees and an arbitrary causal bound pair (every `Bound`
    /// kind, over versions sampled from both trees' leaves and ceilings plus
    /// genesis — so dominated, dominating, equal, concurrent, and crossed
    /// bounds all occur), `Tree::range` yields exactly the leaves that the
    /// documented per-bound membership predicate admits from the unfiltered
    /// walk — the prune/promote shortcuts are pure optimization — in
    /// ascending key order; and the frozen spine walk (`Tree::freeze`)
    /// yields the identical sequence. Two independent implementations of
    /// the same partial-order semantics checking each other. (The predicate
    /// is stated inline over the raw `Bound` pair: `Tree::range`'s
    /// `RangeBounds` surface accepts crossed pairs, which `before::causally`
    /// validates away at composition.)
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

        // Ground truth: the documented per-bound predicate — contained in
        // the end bound and not contained in the start bound — applied to
        // the unfiltered walk. Stated directly over the raw `Bound` pair
        // rather than a composed `causally` range: composition validates
        // its bounds, and this generator deliberately also drives crossed
        // pairs through `Tree::range`'s raw `RangeBounds` surface.
        let admits = |version: &Version| {
            use std::cmp::Ordering;
            let past_start = match &start {
                Bound::Unbounded => true,
                // Not in the start's causal past (greater than or
                // concurrent to it): not subtracted.
                Bound::Excluded(s) => {
                    matches!(version.partial_cmp(s), None | Some(Ordering::Greater))
                }
                // As above, but the bound itself also survives.
                Bound::Included(s) => matches!(
                    version.partial_cmp(s),
                    None | Some(Ordering::Equal | Ordering::Greater)
                ),
            };
            let within_end = match &end {
                Bound::Unbounded => true,
                Bound::Included(e) => version <= e,
                Bound::Excluded(e) => version < e,
            };
            past_start && within_end
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

        let mut frozen = tree.range_owned((start, end));
        let mut thawed = Vec::new();
        while let Some((key, leaf)) = frozen.next() {
            thawed.push((key, leaf.version().clone(), leaf.value().clone()));
        }
        prop_assert_eq!(&thawed, &naive, "the frozen walk must equal the naive filter");
    }

    /// The unfiltered walk is exact and reversible, and the point lookup
    /// resolves it.
    ///
    /// `iter`'s size hint equals the tree's length, the
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

// ───────────────────────── version-size aggregate ─────────────────────────

/// The maximum canonical encoding over every version bound a tree holds
/// — leaf versions and every branch's ceiling and floor — recomputed by
/// direct walk: the oracle `Tree::max_version_bytes` must match.
fn naive_max_version_bytes(tree: &Tree<Bytes>) -> usize {
    tree.max_bound_bytes()
}

proptest! {
    /// The version-size aggregate is exact through inserts and forgets.
    ///
    /// After every action, `max_version_bytes` equals the recomputed max
    /// over every bound the tree holds — leaf versions and every
    /// branch's ceiling and floor (zero once emptied).
    ///
    /// Versions grow as a party's history accumulates, so late inserts
    /// carry strictly larger encodings than early ones; forgetting the
    /// message that carries the maximum must therefore resize the
    /// aggregate *down* — the behavior a monotone high-water scalar would
    /// get wrong. Rotating parties makes sibling versions concurrent, so
    /// branch ceilings genuinely join and can outgrow every leaf below
    /// them — the interior contribution the aggregate must cover.
    #[test]
    fn version_size_aggregate_is_exact(
        values in distinct_bytes(12),
        forgets in proptest::collection::vec(any::<prop::sample::Index>(), 0..18),
    ) {
        let mut tree: Tree<Bytes> = Tree::new();
        prop_assert_eq!(tree.max_version_bytes(), 0, "the empty tree bounds nothing");

        for (i, value) in values.iter().enumerate() {
            // Rotating parties makes sibling versions concurrent, not just
            // points on one chain, so branch maxima genuinely compare.
            tree.act(&party_of([b'a' + (i % 5) as u8]), [insert_action(value.clone())]);
            prop_assert_eq!(tree.max_version_bytes(), naive_max_version_bytes(&tree));
        }

        for forget in forgets {
            let keys: Vec<Key> = tree.iter().map(|(key, ..)| key).collect();
            let Some(&key) = keys.get(forget.index(keys.len().max(1))) else {
                break;
            };
            // Target the argmax half the time so the resize-down direction
            // is exercised on every run, not left to index luck.
            let key = tree
                .iter()
                .max_by_key(|(_, version, _)| version.as_bytes().len())
                .map(|(argmax, ..)| if forget.index(2) == 0 { argmax } else { key })
                .unwrap_or(key);
            tree.act(&party_of("P"), [Action::Forget(key)]);
            prop_assert_eq!(tree.max_version_bytes(), naive_max_version_bytes(&tree));
        }
    }
}

proptest! {
    /// The version-size aggregate survives the merge path, deletion
    /// included.
    ///
    /// Joining two independently grown trees (disjoint parties, concurrent
    /// histories) yields exactly the recomputed max over the union's
    /// bounds. Forgetting on one side first exercises the
    /// deletion-honoring arm: the filter drops the other side's
    /// causally-covered leaves through the merge, and the rebuilt spines
    /// must resize the aggregate to exactly what survives.
    #[test]
    fn version_size_aggregate_survives_join(
        left_values in distinct_bytes(8),
        right_values in distinct_bytes(8),
        forgets in proptest::collection::vec(any::<prop::sample::Index>(), 0..6),
    ) {
        let mut left: Tree<Bytes> = Tree::new();
        for value in &left_values {
            left.act(&party_of("A"), [insert_action(value.clone())]);
        }
        let mut right: Tree<Bytes> = Tree::new();
        for value in &right_values {
            right.act(&party_of("B"), [insert_action(value.clone())]);
        }

        // A fork of `right` that `left` first absorbs wholesale: the
        // forgets below then land on messages `left`'s vector already
        // covers, so the join must *drop* them from the incoming side —
        // the deletion-honoring arm, aimed at the argmax half the time
        // so the resize-down direction is exercised through the merge.
        let absorbed = right.clone();
        left.join(absorbed);
        prop_assert_eq!(left.max_version_bytes(), naive_max_version_bytes(&left));

        for forget in forgets {
            let Some(key) = left
                .iter()
                .max_by_key(|(_, version, _)| version.as_bytes().len())
                .map(|(argmax, ..)| argmax)
                .filter(|_| forget.index(2) == 0)
                .or_else(|| {
                    let keys: Vec<Key> = left.iter().map(|(key, ..)| key).collect();
                    keys.get(forget.index(keys.len().max(1))).copied()
                })
            else {
                break;
            };
            left.act(&party_of("A"), [Action::Forget(key)]);
        }

        left.join(right);
        prop_assert_eq!(left.max_version_bytes(), naive_max_version_bytes(&left));
    }
}

proptest! {
    /// The changed flag [`Tree::act`] returns tracks the root hash exactly
    /// on honestly built trees: `false` iff the root hash is byte-identical
    /// across the call.
    ///
    /// The batches mix inserts, forgets of live keys, and forgets of keys
    /// nothing holds — including the all-no-op and empty batches, which
    /// must read `false`.
    ///
    /// The `false ⇒ hash-equal` direction is the flag's contract — a
    /// watcher skipped on `false` must miss nothing. The converse direction
    /// pins that honest commits never pay a spurious wakeup: every action
    /// ticks strictly above the tree's ceiling, which bounds every leaf, so
    /// the causally-prior skip (the flag's one conservative case, see
    /// `act_changed_flag_is_conservative_only_in_a_poisoned_store`) is
    /// unreachable here.
    #[test]
    fn act_changed_flag_tracks_the_root_hash(
        base_values in distinct_bytes(6),
        batch_values in distinct_bytes(4),
        forget_live in proptest::collection::vec(any::<prop::sample::Index>(), 0..4),
        forget_missing in proptest::collection::vec(any::<Key>(), 0..3),
    ) {
        let mut tree: Tree<Bytes> = Tree::new();
        tree.act(
            &party_of("A"),
            base_values.iter().cloned().map(insert_action),
        );
        let live: Vec<Key> = tree.iter().map(|(k, ..)| k).collect();

        let mut actions: Vec<Action<Bytes>> =
            batch_values.iter().cloned().map(insert_action).collect();
        for index in forget_live {
            if !live.is_empty() {
                actions.push(Action::Forget(live[index.index(live.len())]));
            }
        }
        // A drawn key matching a live one is astronomically unlikely but
        // harmless: the forget would then be effectual and both sides of
        // the equality move together.
        actions.extend(forget_missing.into_iter().map(Action::Forget));

        let before = tree.hash();
        let changed = tree.act(&party_of("A"), actions);
        prop_assert_eq!(changed, tree.hash() != before);
    }

    /// The changed flag [`Tree::join`] returns tracks the root hash
    /// exactly: `false` iff the merge left this tree's root hash
    /// byte-identical.
    ///
    /// The divergent pairs cover one-sided novelty, shared subtrees, and
    /// deletion honoring in both directions.
    ///
    /// The `false ⇒ hash-equal` direction is the flag's contract — a
    /// watcher skipped on `false` must miss nothing. The converse pins
    /// that a merge which nets nothing — the counterparty's novelty all
    /// dropped by deletion honoring, or no novelty at all — never pays a
    /// spurious wakeup, even while the causal ceiling advances.
    #[test]
    fn join_changed_flag_tracks_the_root_hash(
        (a, b) in crate::tree::arb::arb_divergent_pair(),
    ) {
        let mut tree = Tree { root: a };
        let before = tree.hash();
        let changed = tree.join(Tree { root: b });
        prop_assert_eq!(changed, tree.hash() != before);
    }

    /// The changed flag stays exact through deep divergent descent.
    ///
    /// The pairs' paths share a drawn-length spine (the constructed
    /// analogue of a hash-prefix collision), driving the merge's divergent
    /// arm at every level down to the split, where content-addressed pairs
    /// scatter at the root fan and never descend. Zero novelty widths are
    /// drawn too, so subset, identical, and ceiling-only merges — the
    /// flag's `false` arm — are sampled at depth alongside the gains.
    #[test]
    fn join_changed_flag_tracks_the_root_hash_at_depth(
        (a, b) in crate::tree::arb::arb_deep_divergent_pair(),
    ) {
        let mut tree = Tree { root: a };
        let before = tree.hash();
        let changed = tree.join(Tree { root: b });
        prop_assert_eq!(changed, tree.hash() != before);
    }
}

/// The changed-flag biconditional at full depth, on the three shapes that
/// decide it.
///
/// Gain in both directions and deletion honoring at depth report changed;
/// the subtle no-op — merging a strict subset with concurrent versions —
/// runs the divergent descent over the whole 31-byte shared prefix and
/// must still report unchanged.
#[test]
fn deep_divergent_join_changed_flag_is_exact() {
    // Gain in both directions.
    let (a, b, expected) = crate::tree::arb::leaf_parent_dispute_pair();
    for (receiver, counter) in [(a.clone(), b.clone()), (b, a.clone())] {
        let mut tree = Tree { root: receiver };
        let before = tree.hash();
        let changed = tree.join(Tree { root: counter });
        assert_eq!(changed, tree.hash() != before, "deep gain is biconditional");
        assert!(changed, "a deep gain must report changed");
    }

    // The deep no-op: the receiver already holds everything the
    // counterparty has, so the full-depth divergent descent nets nothing.
    let mut tree = Tree { root: expected };
    let before = tree.hash();
    let changed = tree.join(Tree { root: a });
    assert_eq!(
        changed,
        tree.hash() != before,
        "a deep subset merge is biconditional"
    );
    assert!(!changed, "a subset merge must report unchanged");

    // Deletion honoring at depth.
    let (a, b, _survivor) = crate::tree::arb::leaf_parent_redaction_pair();
    let mut tree = Tree { root: a };
    let before = tree.hash();
    let changed = tree.join(Tree { root: b });
    assert_eq!(
        changed,
        tree.hash() != before,
        "a deep redaction drop is biconditional"
    );
    assert!(changed, "deletion honoring must report changed");
}

/// A ceiling-only join reports unchanged.
///
/// Absorbing a counterparty whose every message we already hold or honor
/// as deleted advances our causal ceiling but leaves the content — and the
/// root hash — untouched, and the changed flag stays `false`, so watchers
/// of the *set* are not woken for a merge that taught the set nothing.
#[test]
fn ceiling_only_join_reports_unchanged() {
    let mut tree: Tree<Bytes> = Tree::new();
    tree.act(&party_of("A"), [insert_action(Bytes::from_static(b"kept"))]);

    // The counterparty: a tree that sent one message on its own disjoint
    // party and then redacted it, leaving no content but an advanced
    // ceiling. Its frontier is news to us; its (empty) content is not.
    let mut other: Tree<Bytes> = Tree::new();
    other.act(&party_of("B"), [insert_action(Bytes::from_static(b"gone"))]);
    let key = other
        .iter()
        .map(|(k, ..)| k)
        .next()
        .expect("one live message");
    other.act(&party_of("B"), [Action::Forget(key)]);
    assert!(
        other.is_empty(),
        "the counterparty redacted its only message"
    );

    let before = tree.hash();
    let ceiling_before = tree.latest().clone();
    let changed = tree.join(other);
    assert!(
        !changed,
        "a merge that teaches the set nothing reports unchanged",
    );
    assert_eq!(tree.hash(), before, "the root hash is byte-identical");
    assert_ne!(
        tree.latest(),
        &ceiling_before,
        "the causal ceiling did advance: the flag answers for content, not the frontier",
    );
}

/// The changed flag's one conservative case, constructed: `Tree::act`
/// reporting `true` while the root hash is byte-identical.
///
/// In a store poisoned by an escaped version (a leaf *above* the tree's
/// ceiling — the shape only nonconforming gossip can plant, and which
/// session ingestion rejects as `UncontainedSupply`), a forget of the
/// escaped key ticks from the ceiling, lands causally *prior* to the leaf
/// it targets, and is silently skipped — yet the traversal's observer
/// fires, so the flag reads `true` with the hash untouched. The cost is
/// one spurious watch wakeup, never a missed one: the flag's `false` stays
/// exact even here. In an honestly built tree the ceiling bounds every
/// leaf, so the skip is unreachable and the flag is exact both ways
/// (pinned by `act_changed_flag_tracks_the_root_hash`).
#[test]
fn act_changed_flag_is_conservative_only_in_a_poisoned_store() {
    let (receiver, poisoned, path, _escaped) = super::arb::uncontained_supply_pair();
    let receiver_party = super::arb::nth_party(0);
    let key = Key::from(path);

    let mut tree = Tree { root: receiver };
    assert!(
        tree.join(Tree { root: poisoned }),
        "planting the escaped leaf is a real change",
    );

    let before = tree.hash();
    let changed = tree.act(&receiver_party, [Action::Forget(key)]);
    assert!(
        changed,
        "the skipped forget reports changed: the conservative direction",
    );
    assert_eq!(
        tree.hash(),
        before,
        "the skipped forget left the root hash byte-identical",
    );
    assert!(
        tree.get(&key).is_some(),
        "the escaped leaf survives the skipped forget",
    );
}

/// An escaped version already resident in a store defeats redaction and
/// survives every merge: the mechanism that session ingestion enforcement
/// exists to keep out.
///
/// With a leaf whose version dominates the tree's ceiling (the shape only
/// a nonconforming implementation can transmit, and which ingestion now
/// rejects as `UncontainedSupply`), a forget of its key ticks from the
/// ceiling and is dropped by the causally-prior skip in `traverse::act`,
/// and the deletion filter never classifies the leaf deleted, so a merge
/// re-plants it in a replica that lacks it. This pins why enforcement
/// must happen at ingestion: once resident, the record is immortal.
#[test]
fn escaped_version_defeats_redaction_in_a_poisoned_store() {
    let (receiver, poisoned, path, escaped) = super::arb::uncontained_supply_pair();
    let receiver_party = super::arb::nth_party(0);
    let key = Key::from(path);

    // Plant the escaped leaf by in-memory join: `Tree::join` is a local
    // merge, not wire ingestion, so no session tripwire guards it.
    let mut tree = Tree { root: receiver };
    tree.join(Tree { root: poisoned });
    assert!(tree.get(&key).is_some(), "the join plants the escaped leaf");
    assert!(
        !mirror::contained(&escaped, tree.latest()),
        "the merged ceiling never covers the escaped version",
    );

    // Redaction is silently skipped: the forget's version ticks from the
    // ceiling, which the escaped version strictly dominates.
    tree.act(&receiver_party, [Action::Forget(key)]);
    assert!(
        tree.get(&key).is_some(),
        "redacting the escaped leaf is silently skipped",
    );

    // And the leaf re-propagates: a fresh replica that never held it
    // receives it on merge, because no ceiling ever classifies it as
    // already-seen-and-deleted.
    let mut fresh: Tree<()> = Tree::new();
    fresh.join(tree);
    assert!(
        fresh.get(&key).is_some(),
        "the escaped leaf re-plants into a fresh replica",
    );
}

/// The pair-hull traffic mix at the tree's bounds-memo door, in
/// `before`'s span-ladder rung counters.
///
/// Which span-ladder rung a memo's pair hull takes decides which kernel
/// regime the tree pays — a comparable pair is handed back at one
/// comparison sweep, only a concurrent pair reaches the emitting walk —
/// and the mix is a property of the workload's versions, not of the
/// kernel. Two structural facts anchor the readings: an interior memo's
/// `lo.span(&hi)` operands are ordered by construction (the meet of the
/// floors never exceeds the join of the ceilings), so interior-regime
/// traffic is never concurrent; and a fringe memo's `span_all` leaf
/// combines read sibling leaf versions, whose relation tracks the
/// writers'. The counters are process-global and meaningful one
/// scenario per process (nextest's model).
#[cfg(feature = "meter")]
mod span_door_traffic {
    use before::meter;
    use bytes::Bytes;

    use super::{Tree, insert_action, party_of};

    /// One writer's batch of distinct payloads, tagged so no two
    /// workloads' leaves collide by content.
    fn batch(writer: &str, round: usize, count: usize) -> impl Iterator<Item = Bytes> + use<> {
        let writer = writer.to_owned();
        (0..count).map(move |i| Bytes::from(format!("{writer}-{round}-{i}")))
    }

    /// The four rung cells of one measured workload run.
    fn cells(work: impl FnOnce()) -> (u64, u64, u64, u64) {
        meter::reset_span_traffic();
        work();
        let read = meter::span_traffic();
        (read.equal, read.empty, read.comparable, read.concurrent)
    }

    /// A single-writer tree never presents a concurrent pair at the
    /// bounds-memo door: one party's versions form a chain, every
    /// bound folded from a chain stays on it, and the door's traffic
    /// is entirely fast-path (comparable or coincident), zero
    /// emissions.
    #[test]
    fn single_writer_bounds_never_emit() {
        let (equal, empty, comparable, concurrent) = cells(|| {
            let mut tree: Tree<Bytes> = Tree::new();
            for round in 0..8 {
                tree.act(
                    &party_of("A"),
                    batch("a", round, 64).map(insert_action),
                );
                tree.warm_caches();
            }
        });
        eprintln!(
            "MEASURED span_door_single_writer: equal={equal} empty={empty} \
             comparable={comparable} concurrent={concurrent}"
        );
        assert!(
            comparable > 0,
            "a warmed single-writer tree folds bounds through the comparable rung"
        );
        assert_eq!(
            concurrent, 0,
            "one party's versions form a chain: no memo pair is ever concurrent"
        );
    }

    /// Divergent writers split the door's traffic: fringe combines of
    /// concurrent leaf versions reach the emitting walk, while the
    /// interior regime (ordered by construction) and same-writer
    /// sibling runs stay on the fast paths — both rungs read live on
    /// one merged four-writer tree, incremental rounds included.
    #[test]
    fn merged_writers_split_the_door() {
        let (equal, empty, comparable, concurrent) = cells(|| {
            let mut merged: Tree<Bytes> = Tree::new();
            for label in ["A", "B", "C", "D"] {
                let mut tree: Tree<Bytes> = Tree::new();
                for round in 0..4 {
                    tree.act(
                        &party_of(label),
                        batch(label, round, 32).map(insert_action),
                    );
                }
                tree.warm_caches();
                merged.join(tree);
            }
            merged.warm_caches();
            // Incremental rounds on the merged tree: acts invalidate
            // ancestor memos, so re-warming re-folds them against the
            // merged population.
            for round in 100..104 {
                merged.act(
                    &party_of("A"),
                    batch("a", round, 32).map(insert_action),
                );
                merged.warm_caches();
            }
        });
        eprintln!(
            "MEASURED span_door_merged_writers: equal={equal} empty={empty} \
             comparable={comparable} concurrent={concurrent}"
        );
        assert!(
            comparable > 0,
            "interior memos are ordered by construction: the comparable rung stays live"
        );
        assert!(
            concurrent > 0,
            "divergent writers' sibling leaves are concurrent: the emitting rung stays live"
        );
    }
}
