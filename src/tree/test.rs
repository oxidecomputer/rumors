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
/// Tests speak in terms of raw `Bytes`, but the tree's API now takes
/// `Message<T>`, so every insert goes through this one-liner.
fn msg(b: Bytes) -> Message<Bytes> {
    Message::new(b)
}

/// Convert an action into the form the tree accepts: `Action::Insert` wraps
/// its value in a `Message<Bytes>`; `Action::Delete` passes through.
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

/// Pre-hash a human-readable party label into the `Bytes` form that `Tree`
/// stores internally. `Tree::for_party` hashes its input once, so any
/// test that wants to address the tree by its original label must apply the
/// same hash before comparing version vectors, computing leaf paths, or
/// constructing reactions.
fn hashed_party(party: impl AsRef<[u8]>) -> Bytes {
    Bytes::copy_from_slice(&Hash::of(party.as_ref()).as_bytes()[..])
}

/// Build a [`Version`] keyed by the pre-hashed form of a human-readable
/// party label. This is the `Version<Bytes>` that `Tree::react` accepts
/// after the tree started pre-hashing its own party.
fn version_for(party: impl AsRef<[u8]>, scalar: u64) -> Version {
    Version::from((hashed_party(party), scalar))
}

/// Compute the leaf-path `Id` that `Tree::act` would assign for an insert of
/// `value` at scalar version `scalar` under the given party label. The tree
/// hashes over the *serialized* message bytes, not the raw inner value, so
/// we wrap `value` in a `Message` and feed the cached serialization to
/// `Path::for_leaf`. The party is also pre-hashed so the resulting path
/// matches the one the tree derives internally.
fn leaf_path(party: impl AsRef<[u8]>, scalar: u64, value: &Bytes) -> Key {
    Path::for_leaf(&hashed_party(party), scalar, msg(value.clone()).bytes()).into()
}

/// Build a versioned insert triple of the shape `Tree::react` expects:
/// `(version, leaf_path, message)`. The leaf path matches what `act` would
/// have computed for the given party label and scalar version. Wrapping the
/// boilerplate keeps the test bodies focused on the property under test.
fn insert_at(
    version: Version,
    party: impl AsRef<[u8]>,
    scalar: u64,
    value: Bytes,
) -> (Version, Key, Message<Bytes>) {
    (version, leaf_path(party, scalar, &value), msg(value))
}

/// Build a versioned delete triple of the shape `Tree::react` expects:
/// `(version, key, None)`. Pairs with [`insert_at`] when a test needs to
/// mix inserts and deletes, though it is useful on its own for tests that
/// care only about deletion bookkeeping.
fn delete_at(version: Version, id: Key) -> (Version, Key, Option<Message<Bytes>>) {
    (version, id, None)
}

/// Perform one full bidirectional synchronization step between two trees
/// using `unknown`: both sides snapshot their version vectors up front,
/// each asks the other for everything unknown relative to that snapshot,
/// and each replays the received leaves via `react`. Because the snapshots
/// are taken before any reaction, the two directions are independent and
/// can be applied in either order. Absent deletions, this is the entire
/// protocol needed for two parties to converge.
fn sync_via_unknown(a: &mut Tree<Bytes>, b: &mut Tree<Bytes>) {
    let from_a = a.unknown(b.version());
    let from_b = b.unknown(a.version());
    a.react(from_b, |_, _, _| {});
    b.react(from_a, |_, _, _| {});
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

/// Compute the root hash of the fully-expanded (uVn-path-compressed) 256-ary
/// trie over the given set of values. For every unique hash path, a leaf
/// sentinel of all-0xff bytes sits at depth 32; at each level above, a 256-way
/// branch hashes its child slots (0x00-filled where absent, recursive hash
/// where present). This is the ground truth that the compressed tree's root
/// hash must match.
fn reference_hash(values: &[(Bytes, u64, Bytes)]) -> Hash {
    const LEAF_SENTINEL: [u8; 32] = [0xff; 32];
    const ZERO: [u8; 32] = [0x00; 32];

    // The empty tree has the hash of an empty node
    if values.is_empty() {
        return Hash::default();
    }

    let hash_branch = |children: &HashMap<u8, Hash>| -> Hash {
        let mut hasher = Hasher::new();
        for i in u8::MIN..=u8::MAX {
            match children.get(&i) {
                Some(h) => hasher.update(h.as_bytes()),
                None => hasher.update(&ZERO),
            };
        }
        hasher.finalize()
    };

    // Level 32 (the value level): every distinct path maps to the sentinel.
    // The tree hashes over the serialized `Message` bytes, not the raw
    // inner value, so we do the same here.
    let paths: BTreeSet<Key> = values
        .iter()
        .map(|(party, version, value)| {
            Path::for_leaf(party, *version, msg(value.clone()).bytes()).into()
        })
        .collect();

    if paths.is_empty() {
        return hash_branch(&HashMap::new());
    }

    let mut current: HashMap<Vec<u8>, Hash> = paths
        .into_iter()
        .map(|p| {
            (
                <[u8; 32]>::from(typed::Path::from(p)).to_vec(),
                Hash(LEAF_SENTINEL),
            )
        })
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

/// An empty tree's root hash must match the reference (256 zero slots).
#[test]
fn empty_tree_hash_matches_reference() {
    let tree: Tree<Bytes> = Tree::for_party("P".to_string());
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
    let mut tree: Tree<Bytes> = Tree::for_party("P".to_string());
    tree.act([insert_action(value.clone())], |_, _, _| {});
    let tree_hash = tree.hash();
    let reference = reference_hash(&[(hashed_party("P"), 1, value)]);
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
        let mut tree = Tree::for_party("P".to_string());
        tree.act(values.iter().cloned().map(insert_action), |_, _, _| {});
        let reference_input: Vec<_> = values
            .into_iter()
            .enumerate()
            .map(|(i, v)| (hashed_party("P"), (i + 1) as u64, v))
            .collect();
        let reference = reference_hash(&reference_input);
        prop_assert_eq!(&tree.hash(), reference.as_bytes());
    }

    /// `act` is associative under partitioning: splitting a sequence of
    /// actions across multiple `act` calls produces a structurally-equal
    /// tree to a single `act` over their concatenation. With per-insert
    /// versioning each insert's claimed version depends only on the number
    /// of preceding inserts in the running sequence, so the partition is
    /// observable only as a batching optimization, not a semantic change.
    #[test]
    fn act_partitioning_preserves_tree(
        inserts in distinct_bytes(8),
        deletes in proptest::collection::vec(any::<Key>(), 0..4),
        interleave in any::<u64>(),
        breaks in proptest::collection::vec(any::<bool>(), 0..16),
    ) {
        let party = "P".to_string();

        // Deterministically interleave inserts and forgets, matching the
        // mixing pattern used by `act_observer_mirrors_actions`.
        let mut actions: Vec<Action<Bytes>> = Vec::new();
        let mut ins = inserts.into_iter();
        let mut del = deletes.into_iter();
        let mut rng = interleave;
        loop {
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
            let prefer_insert = rng & 1 == 0;
            let has_ins = ins.clone().next().is_some();
            let has_del = del.clone().next().is_some();
            match (prefer_insert, has_ins, has_del) {
                (true, true, _) | (false, true, false) => {
                    actions.push(insert_action(ins.next().unwrap()));
                }
                (false, _, true) | (true, false, true) => {
                    actions.push(Action::Forget(del.next().unwrap()));
                }
                _ => break,
            }
        }
        let n = actions.len();

        let mut all_in_one = Tree::for_party(party.clone());
        all_in_one.act(actions.clone(), |_, _, _| {});

        let mut partitioned = Tree::for_party(party.clone());
        let mut chunk: Vec<Action<Bytes>> = Vec::new();
        for (i, a) in actions.into_iter().enumerate() {
            chunk.push(a);
            let at_boundary =
                breaks.get(i).copied().unwrap_or(false) || i + 1 == n;
            if at_boundary {
                partitioned.act(std::mem::take(&mut chunk), |_, _, _| {});
            }
        }

        prop_assert_eq!(all_in_one, partitioned);
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

        let mut all_in_one = Tree::for_party(party.clone());
        all_in_one.react(
            bytes
                .iter()
                .cloned()
                .map(|b| insert_at(version.clone(), &party, 1, b)),
            |_, _, _| {}
        );

        let mut partitioned = Tree::for_party(party.clone());
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
                partitioned.react(batch, |_, _, _| {});
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
        let mut t_act = Tree::for_party("P".to_string());
        for b in &bytes {
            t_act.act([insert_action(b.clone())], |_, _, _| {});
        }

        let party = "P".to_string();
        let versions: Vec<Version> = (1..=bytes.len())
            .map(|i| version_for(&party, i as u64))
            .collect();

        let mut t_react = Tree::for_party(party.clone());
        t_react.react(
            versions
                .into_iter()
                .zip(bytes.iter().cloned())
                .enumerate()
                .map(|(i, (v, b))| insert_at(v, &party, (i + 1) as u64, b)),
            |_, _, _| {}
        );

        prop_assert_eq!(t_act.hash(), t_react.hash());
        prop_assert_eq!(t_act.version(), t_react.version());
    }

    /// Inserting a value and then deleting its leaf path via two separate `act`
    /// calls must leave the tree empty (zero root hash). Each `act` batch
    /// advances the party's scalar version by one — inserts and forgets
    /// both claim a fresh version, so that the mirror protocol can
    /// distinguish "I forgot this" from "I never knew about it."
    #[test]
    fn insert_then_delete_is_empty(value in any::<Vec<u8>>()) {
        let party = "P".to_string();
        let value = Bytes::from(value);
        let path = leaf_path(&party, 1, &value);

        let mut tree = Tree::for_party(party.clone());
        tree.act([insert_action(value)], |_, _, _| {});
        tree.act([Action::Forget(path)], |_, _, _| {});

        prop_assert_eq!(tree.hash(), [0u8; 32]);
        prop_assert_eq!(tree.version().for_party(&hashed_party(&party)), 2);
    }

    /// Inserting a value and deleting its leaf path within the same `act`
    /// batch must leave the tree empty with the version bumped twice (once
    /// per action). The "last action on a given path wins" rule makes the
    /// delete prevail.
    #[test]
    fn insert_and_delete_same_batch_is_empty(value in any::<Vec<u8>>()) {
        let party = "P".to_string();
        let value = Bytes::from(value);
        let path = leaf_path(&party, 1, &value);

        let mut tree = Tree::for_party(party.clone());
        tree.act([insert_action(value), Action::Forget(path)], |_, _, _| {});

        prop_assert_eq!(tree.hash(), [0u8; 32]);
        prop_assert_eq!(tree.version().for_party(&hashed_party(&party)), 2);
    }

    /// Deleting a path that is not present in the tree must not change the
    /// root hash. The version vector still advances because `act` always
    /// bumps, but the leaf multiset is identical, so the hash is unchanged.
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

        let mut t_before = Tree::for_party(party.clone());
        t_before.act(bytes.into_iter().map(insert_action), |_, _, _| {});
        let mut t_after = t_before.clone();
        t_after.act([Action::Forget(nuke)], |_, _, _| {});

        prop_assert_eq!(t_before.hash(), t_after.hash());
    }

    /// A fresh tree returns no values for any requested paths: no leaves are
    /// present, so every lookup misses.
    #[test]
    fn get_on_empty_tree_is_empty(
        paths in proptest::collection::vec(any::<Key>(), 0..8),
    ) {
        let tree: Tree<Bytes> = Tree::for_party("P".to_string());
        prop_assert!(tree.get(paths).is_empty());
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
        let mut tree = Tree::for_party(party.clone());
        tree.act(bytes.iter().cloned().map(insert_action), |_, _, _| {});

        let paths: Vec<Key> = bytes
            .iter()
            .enumerate()
            .map(|(i, b)| leaf_path(&party, (i + 1) as u64, b))
            .collect();

        let mut got: Vec<Bytes> =
            tree.get(paths).into_iter()
                .map(|(_, _, m)| m)
                .map(Message::clone_into_inner)
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
        let mut tree = Tree::for_party(party.clone());
        tree.act(bytes.iter().cloned().map(insert_action), |_, _, _| {});

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
            .get(all_paths)
            .into_iter()
            .map(|(_, _, m)| m)
            .map(Message::clone_into_inner)
            .collect();
        got.sort();
        let mut expected: Vec<Bytes> = bytes;
        expected.sort();
        prop_assert_eq!(got, expected);
    }

    /// Every `act` call advances the owning party's scalar version by exactly
    /// the number of [`Action::Insert`]s in the batch: each insert claims a
    /// fresh version so that content-identical messages produce distinct keys.
    /// Forgets do not advance the version.
    #[test]
    fn act_bumps_self_party_by_number_of_inserts(
        prior_inserts in 0usize..4,
        batch_size in 1usize..8,
    ) {
        let party = "P".to_string();
        let mut tree = Tree::for_party(party.clone());
        for i in 0..prior_inserts {
            tree.act([insert_action(Bytes::from(
                format!("prior-{i}").into_bytes(),
            ))], |_, _, _| {});
        }
        let before = tree.version().for_party(&hashed_party(&party));
        let party_before = tree.party().clone();

        let actions: Vec<Action<Bytes>> = (0..batch_size)
            .map(|i| {
                insert_action(Bytes::from(format!("batch-{i}").into_bytes()))
            })
            .collect();
        tree.act(actions, |_, _, _| {});

        prop_assert_eq!(
            tree.version().for_party(&hashed_party(&party)),
            before + batch_size as u64,
        );
        prop_assert_eq!(tree.party(), &party_before);
    }

    /// An empty `act` batch leaves the version vector completely unchanged.
    /// There are no actions to observe, so there is nothing to mark as seen.
    #[test]
    fn empty_act_is_a_version_noop(prior_batches in 0usize..4) {
        let party = "P".to_string();
        let mut tree = Tree::for_party(party.clone());
        for i in 0..prior_batches {
            tree.act([insert_action(Bytes::from(
                format!("prior-{i}").into_bytes(),
            ))], |_, _, _| {});
        }
        let before = tree.version().clone();
        tree.act(std::iter::empty::<Action<Bytes>>(), |_, _, _| {});
        prop_assert_eq!(tree.version(), before);
    }

    /// After `react(versions)`, the tree's version vector is exactly the
    /// join (pointwise max) of its prior version with every incoming
    /// version. In particular, it never decreases any component: a tree
    /// that has observed an action is forever causally downstream of it.
    #[test]
    fn react_joins_incoming_versions(
        prior_batches in 0usize..3,
        incoming in proptest::collection::vec(
            (prop::sample::select(vec!["A".to_string(), "B".to_string(), "C".to_string()]),
             1u64..5u64),
            0..8,
        ),
    ) {
        let party = "P".to_string();
        let mut tree = Tree::for_party(party.clone());
        for i in 0..prior_batches {
            tree.act([insert_action(Bytes::from(
                format!("prior-{i}").into_bytes(),
            ))], |_, _, _| {});
        }
        let before = tree.version().clone();
        let party_before = tree.party().clone();

        let versions: Vec<Version> = incoming
            .iter()
            .map(|(p, s)| version_for(p, *s))
            .collect();

        let mut expected = before.clone();
        for v in &versions {
            expected |= v.clone();
        }

        // Use deletes on random unrelated paths so the actions never
        // disturb pre-existing leaves; we are testing version bookkeeping,
        // not tree mutation.
        tree.react(
            versions
                .into_iter()
                .enumerate()
                .map(|(i, v)| delete_at(v, typed::Path::from([i as u8; 32]).into())),
            |_, _, _| {}
        );

        prop_assert_eq!(tree.version(), expected);
        prop_assert!(tree.version() >= before);
        prop_assert_eq!(tree.party(), &party_before);
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

        let mut t_ab = Tree::for_party(party.clone());
        t_ab.react(
            bytes_a.iter().cloned().map(|b| insert_at(v_a.clone(), &party, 1, b)), |_, _, _| {}
        );
        t_ab.react(
            bytes_b.iter().cloned().map(|b| insert_at(v_b.clone(), &party, 2, b)), |_, _, _| {}
        );

        let mut t_ba = Tree::for_party(party.clone());
        t_ba.react(
            bytes_b.iter().cloned().map(|b| insert_at(v_b.clone(), &party, 2, b)), |_, _, _| {}
        );
        t_ba.react(
            bytes_a.iter().cloned().map(|b| insert_at(v_a.clone(), &party, 1, b)), |_, _, _| {}
        );

        prop_assert_eq!(t_ab, t_ba);
    }

    /// `react` is idempotent: applying the same batch twice is identical to
    /// applying it once. This is the CRDT property that lets us re-deliver
    /// messages safely in the face of retries or out-of-order transport.
    #[test]
    fn react_idempotent(bytes in distinct_bytes(16)) {
        let party = "P".to_string();
        let v = version_for(&party, 1);

        let mut t_once = Tree::for_party(party.clone());
        t_once.react(
            bytes.iter().cloned().map(|b| insert_at(v.clone(), &party, 1, b)), |_, _, _| {}
        );

        let mut t_twice = Tree::for_party(party.clone());
        t_twice.react(
            bytes.iter().cloned().map(|b| insert_at(v.clone(), &party, 1, b)), |_, _, _| {}
        );
        t_twice.react(
            bytes.iter().cloned().map(|b| insert_at(v.clone(), &party, 1, b)), |_, _, _| {}
        );

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

        let mut t_base = Tree::for_party(party.clone());
        t_base.react(base.iter().cloned().map(|b| {
            let (v, scalar) = meta_by_value.get(&b).unwrap();
            insert_at(v.clone(), &party, *scalar, b)
        }), |_, _, _| {});

        let mut t_shuf = Tree::for_party(party.clone());
        t_shuf.react(shuffled.iter().cloned().map(|b| {
            let (v, scalar) = meta_by_value.get(&b).unwrap();
            insert_at(v.clone(), &party, *scalar, b)
        }), |_, _, _| {});

        prop_assert_eq!(t_base, t_shuf);
    }

    /// Strong eventual consistency: if two parties each apply their own
    /// actions locally and then cross-react to each other's recorded event
    /// history, their trees converge to the same leaf multiset (and thus
    /// the same root hash and version vector). Different parties keep
    /// distinct `party` fields, so we can't use `Tree`'s full structural
    /// equality, but the observable invariants — `hash()` and `version()`
    /// — must agree.
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
        let mut tree_a: Tree<Bytes> = Tree::for_party(a_id.clone());
        let mut a_events: Vec<(Version, Key, Message<Bytes>)> = Vec::new();
        for (i, value) in a_inserts.iter().enumerate() {
            let scalar = (i + 1) as u64;
            let mut recorded = tree_a.version().clone();
            recorded.event(&hashed_party(&a_id));
            tree_a.act([insert_action(value.clone())], |_, _, _| {});
            a_events.push(insert_at(recorded, &a_id, scalar, value.clone()));
        }

        let mut tree_b: Tree<Bytes> = Tree::for_party(b_id.clone());
        let mut b_events: Vec<(Version, Key, Message<Bytes>)> = Vec::new();
        for (i, value) in b_inserts.iter().enumerate() {
            let scalar = (i + 1) as u64;
            let mut recorded = tree_b.version().clone();
            recorded.event(&hashed_party(&b_id));
            tree_b.act([insert_action(value.clone())], |_, _, _| {});
            b_events.push(insert_at(recorded, &b_id, scalar, value.clone()));
        }

        tree_a.react(b_events.iter().map(|(v, k, m)| (v.clone(), *k, m.clone())), |_, _, _| {});
        tree_b.react(a_events.iter().map(|(v, k, m)| (v.clone(), *k, m.clone())), |_, _, _| {});

        prop_assert_eq!(tree_a.version(), tree_b.version());
        prop_assert_eq!(tree_a.hash(), tree_b.hash());
    }

    /// A tree remembers the party it was built for: `for_party(p).party()`
    /// is `&p`, and no sequence of `act`/`react` changes that.
    #[test]
    fn party_is_remembered_across_mutation(
        acts in distinct_bytes(6),
        reacts in proptest::collection::vec(1u64..5u64, 0..6),
    ) {
        let party = "P".to_string();
        let hashed = hashed_party(&party);
        let mut tree = Tree::for_party(party.clone());
        prop_assert_eq!(tree.party(), &hashed);

        tree.act(acts.into_iter().map(insert_action), |_, _, _| {});
        prop_assert_eq!(tree.party(), &hashed);

        let versions: Vec<Version> = reacts
            .iter()
            .map(|s| version_for(&party, *s))
            .collect();
        tree.react(
            versions
                .iter()
                .enumerate()
                .map(|(i, v)| delete_at(v.clone(), typed::Path::from([i as u8; 32]).into())),
            |_, _, _| {}
        );
        prop_assert_eq!(tree.party(), &hashed);
    }

    /// `Clone` yields a tree that is structurally indistinguishable: equal
    /// under `Eq`, same party, same version, same hash. Cloning is a pure
    /// copy, not a semantic operation.
    #[test]
    fn clone_preserves_all_observables(acts in distinct_bytes(8)) {
        let party = "P".to_string();
        let mut tree = Tree::for_party(party.clone());
        tree.act(acts.into_iter().map(insert_action), |_, _, _| {});
        let cloned = tree.clone();

        prop_assert_eq!(cloned.party(), tree.party());
        prop_assert_eq!(cloned.version(), tree.version());
        prop_assert_eq!(cloned.hash(), tree.hash());
        prop_assert_eq!(cloned, tree);
    }

    /// Structural equality implies hash equality. `Eq` compares root nodes
    /// directly, so if two trees are `Eq` their root hashes — a pure
    /// function of the root node — must agree. Two independently-built
    /// trees that applied the same batch of actions are expected to be
    /// structurally equal.
    #[test]
    fn eq_implies_same_hash(acts in distinct_bytes(8)) {
        let party = "P".to_string();
        let mut t1 = Tree::for_party(party.clone());
        t1.act(acts.iter().cloned().map(insert_action), |_, _, _| {});
        let mut t2 = Tree::for_party(party.clone());
        t2.act(acts.into_iter().map(insert_action), |_, _, _| {});

        prop_assert_eq!(&t1, &t2);
        prop_assert_eq!(t1.hash(), t2.hash());
    }

    /// Inserting the same value under different parties produces different
    /// leaf paths, and therefore different root hashes. Party identity
    /// participates in the path derivation precisely so two parties can
    /// concurrently write the same value without colliding.
    #[test]
    fn same_value_different_parties_differ(value in any::<Vec<u8>>()) {
        let value = Bytes::from(value);
        let mut t_a = Tree::for_party("A".to_string());
        let mut t_b = Tree::for_party("B".to_string());
        t_a.act([insert_action(value.clone())], |_, _, _| {});
        t_b.act([insert_action(value)], |_, _, _| {});

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
        let party = "P".to_string();
        let mut tree = Tree::for_party(party.clone());
        for batch in batches {
            tree.act(batch.into_iter().map(insert_action), |_, _, _| {});
        }
        prop_assert!(tree.unknown(tree.version().clone()).is_empty());
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
        let mut tree = Tree::for_party(party.clone());
        tree.act(bytes.iter().cloned().map(insert_action), |_, _, _| {});

        let got = tree.unknown(Version::default());

        let got_paths: BTreeSet<Key> = got.iter().map(|(_, p, _)| *p).collect();
        let expected_paths: BTreeSet<Key> = bytes
            .iter()
            .enumerate()
            .map(|(i, b)| leaf_path(&party, (i + 1) as u64, b))
            .collect();
        prop_assert_eq!(got_paths, expected_paths);

        let mut got_scalars: Vec<u64> = got
            .iter()
            .map(|(v, _, _)| v.for_party(&hashed_party(&party)))
            .collect();
        got_scalars.sort();
        let expected_scalars: Vec<u64> = (1..=bytes.len() as u64).collect();
        prop_assert_eq!(got_scalars, expected_scalars);

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
        let mut tree_a = Tree::for_party("A".to_string());
        let mut tree_b = Tree::for_party("B".to_string());
        tree_a.act(a_inserts.into_iter().map(insert_action), |_, _, _| {});
        tree_b.act(b_inserts.into_iter().map(insert_action), |_, _, _| {});

        sync_via_unknown(&mut tree_a, &mut tree_b);

        prop_assert_eq!(tree_a.hash(), tree_b.hash());
        prop_assert_eq!(tree_a.version(), tree_b.version());
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
        let mut tree_a = Tree::for_party("A".to_string());
        let mut tree_b = Tree::for_party("B".to_string());
        tree_a.act(a_inserts.into_iter().map(insert_action), |_, _, _| {});
        tree_b.act(b_inserts.into_iter().map(insert_action), |_, _, _| {});

        sync_via_unknown(&mut tree_a, &mut tree_b);

        let hash_a = tree_a.hash();
        let hash_b = tree_b.hash();
        let version_a = tree_a.version().clone();
        let version_b = tree_b.version().clone();

        // The deltas in both directions must now be empty.
        prop_assert!(tree_a.unknown(version_b.clone()).is_empty());
        prop_assert!(tree_b.unknown(version_a.clone()).is_empty());

        // And a second sync must leave both trees bit-identical.
        sync_via_unknown(&mut tree_a, &mut tree_b);
        prop_assert_eq!(tree_a.hash(), hash_a);
        prop_assert_eq!(tree_b.hash(), hash_b);
        prop_assert_eq!(tree_a.version(), version_a);
        prop_assert_eq!(tree_b.version(), version_b);
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
        let b_id = "B".to_string();
        let mut tree_a = Tree::for_party(a_id.clone());
        let mut tree_b = Tree::for_party(b_id.clone());
        tree_a.act(a_inserts.iter().cloned().map(insert_action), |_, _, _| {});
        tree_b.act(b_inserts.iter().cloned().map(insert_action), |_, _, _| {});

        tree_b.react(tree_a.unknown(tree_b.version().clone()), |_, _, _| {});

        prop_assert!(tree_b.version() >= tree_a.version());

        let a_paths: Vec<Key> = a_inserts
            .iter()
            .enumerate()
            .map(|(i, b)| leaf_path(&a_id, (i + 1) as u64, b))
            .collect();
        let mut got: Vec<Bytes> = tree_b
            .get(a_paths)
            .into_iter()
            .map(|(_, _, m)| m)
            .map(Message::clone_into_inner)
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
        let mut tree_a = Tree::for_party("A".to_string());
        let mut tree_b = Tree::for_party("B".to_string());

        for op in ops {
            match op {
                SyncOp::ActA(values) => {
                    tree_a.act(values.into_iter().map(insert_action), |_, _, _| {});
                }
                SyncOp::ActB(values) => {
                    tree_b.act(values.into_iter().map(insert_action), |_, _, _| {});
                }
                SyncOp::Sync => {
                    sync_via_unknown(&mut tree_a, &mut tree_b);
                    prop_assert_eq!(tree_a.hash(), tree_b.hash());
                    prop_assert_eq!(tree_a.version(), tree_b.version());
                }
            }
        }

        sync_via_unknown(&mut tree_a, &mut tree_b);
        prop_assert_eq!(tree_a.hash(), tree_b.hash());
        prop_assert_eq!(tree_a.version(), tree_b.version());
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
        let mut tree = Tree::for_party(party.clone());
        tree.act([insert_action(value.clone())], |_, _, _| {});
        tree.act([insert_action(value.clone())], |_, _, _| {});

        let path_v1 = leaf_path(&party, 1, &value);
        let path_v2 = leaf_path(&party, 2, &value);

        prop_assert_ne!(path_v1, path_v2);
        let got = tree.get([path_v1, path_v2]);
        prop_assert_eq!(got.len(), 2);
        prop_assert!(got.iter().all(|b| b.2.message() == &value));
    }

    /// `act`'s observer closure fires exactly once per supplied action, in
    /// the order the actions were presented, and each emitted `Reaction`
    /// structurally mirrors its originating `Action`: an insert yields an
    /// `Insert(path, value)` whose path is the leaf path `act` assigned at
    /// the scalar produced by advancing the party's version vector once per
    /// insert so far, and whose value is byte-identical to the original; a
    /// forget yields a `(path, None)` reaction with the same id passed
    /// through verbatim and without advancing the scalar.
    #[test]
    fn act_observer_mirrors_actions(
        prior_batches in 0usize..3,
        inserts in distinct_bytes(6),
        deletes in proptest::collection::vec(any::<Key>(), 0..4),
        interleave in any::<u64>(),
    ) {
        let party = "P".to_string();
        let mut tree = Tree::for_party(party.clone());
        for i in 0..prior_batches {
            tree.act(
                [insert_action(Bytes::from(format!("prior-{i}").into_bytes()))],
                |_, _, _| {},
            );
        }
        // Scalar the next insert in the batch will claim; advances by one
        // for every insert observed and stays put for every forget.
        let mut running_scalar = tree.version().for_party(&hashed_party(&party));

        // Deterministically interleave inserts and deletes so the proptest
        // exercises many orderings without giving up reproducibility.
        let mut actions: Vec<Action<Bytes>> = Vec::new();
        let mut ins = inserts.iter().cloned();
        let mut del = deletes.iter().copied();
        let mut rng = interleave;
        loop {
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
            let prefer_insert = rng & 1 == 0;
            match (prefer_insert, ins.clone().next().is_some(), del.clone().next().is_some()) {
                (true, true, _) | (false, true, false) => {
                    actions.push(insert_action(ins.next().unwrap()));
                }
                (false, _, true) | (true, false, true) => {
                    actions.push(Action::Forget(del.next().unwrap()));
                }
                _ => break,
            }
        }

        let expected_actions = actions.clone();
        let mut captured: Vec<(Key, Option<Message<Bytes>>)> = Vec::new();
        tree.act(actions, |_, k, m| captured.push((k, m.cloned())));

        prop_assert_eq!(captured.len(), expected_actions.len());
        for ((path, message), action) in captured.iter().zip(expected_actions.iter()) {
            // Every action — Insert or Forget — bumps the local
            // party's scalar version once, so the next leaf-path
            // computation reflects the post-bump value.
            running_scalar += 1;
            match (message, action) {
                (Some(v), Action::Insert(value)) => {
                    prop_assert_eq!(v, value);
                    prop_assert_eq!(
                        *path,
                        leaf_path(&party, running_scalar, value.message()),
                    );
                }
                (None, Action::Forget(id)) => {
                    prop_assert_eq!(path, id);
                }
                _ => prop_assert!(false, "reaction/action kind mismatch"),
            }
        }
    }

    /// An empty `act` batch never invokes the observer closure: with no
    /// actions there is nothing to report, so the callback is untouched
    /// regardless of how many prior batches the tree has seen.
    #[test]
    fn act_observer_silent_on_empty_batch(prior_batches in 0usize..4) {
        let party = "P".to_string();
        let mut tree = Tree::for_party(party.clone());
        for i in 0..prior_batches {
            tree.act(
                [insert_action(Bytes::from(format!("prior-{i}").into_bytes()))],
                |_, _, _| {},
            );
        }
        let mut fired = 0usize;
        tree.act(std::iter::empty::<Action<Bytes>>(), |_, _, _| fired += 1);
        prop_assert_eq!(fired, 0);
    }

    /// The reactions surfaced through `act`'s observer are exactly the
    /// payload a peer needs to reproduce the batch: replaying them via
    /// `react` on a fresh tree for the same party — each reaction paired
    /// with the version `act` assigned it — yields a structurally equal
    /// tree. This is the contract that makes the observer usable as the
    /// wire-format outbox for a synchronization protocol.
    #[test]
    fn act_observer_reactions_replay_to_equal_tree(
        inserts in distinct_bytes(8),
        deletes in proptest::collection::vec(any::<Key>(), 0..4),
    ) {
        let party = "P".to_string();
        let mut original: Tree<Bytes> = Tree::for_party(party.clone());
        let actions: Vec<Action<Bytes>> = inserts
            .iter()
            .cloned()
            .map(insert_action)
            .chain(deletes.iter().copied().map(Action::Forget))
            .collect();

        // Capture the version `act` assigned to each reaction; with per-insert
        // versioning each insert gets a distinct vector and forgets share the
        // running vector at their position.
        let mut captured: Vec<(Version, Key, Option<Message<Bytes>>)> = Vec::new();
        original.act(actions, |v, k, m| captured.push((v.clone(), k, m.cloned())));

        let mut replay = Tree::for_party(party.clone());
        replay.react(
            captured
                .iter()
                .map(|(v, k, m)| (v.clone(), *k, m.clone())),
                |_, _, _| {}
        );

        prop_assert_eq!(original, replay);
    }
}
