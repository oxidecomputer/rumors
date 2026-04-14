use std::collections::BTreeSet;
use std::ops::RangeInclusive;

use bytes::Bytes;
use proptest::prelude::*;

use super::*;
use crate::version::Version;

/// Party type for tests. `u8` keeps the version-vector search space small.
type P = u8;

/// Reverse a forward-order path into the pop-order that both `Node::insert`
/// consumes and `Node::path` stores.
fn rev(path: &[u8]) -> Vec<u8> {
    let mut v = path.to_vec();
    v.reverse();
    v
}

/// Build a `Version` by recording one event per (party, count) pair.
fn ver(events: &[(P, u64)]) -> Version<P> {
    let mut v = Version::<P>::default();
    for &(p, n) in events {
        for _ in 0..n {
            v.event(p);
        }
    }
    v
}

/// A flat description of every leaf reachable from `node`: its full path from
/// the root, the bytes at that leaf, and the leaf's version.
fn leaves<Q: Ord + Clone>(node: &Node<Q>) -> Vec<(Vec<u8>, Bytes, Version<Q>)> {
    fn walk<Q: Ord + Clone>(
        node: &Node<Q>,
        prefix: &mut Vec<u8>,
        out: &mut Vec<(Vec<u8>, Bytes, Version<Q>)>,
    ) {
        let saved = prefix.len();
        // `node.path` is stored in reverse descent order; reverse it back
        // onto the running forward-order prefix.
        prefix.extend(node.path.iter().rev());
        match &node.children {
            Children::Leaf(b) => {
                out.push((prefix.clone(), b.clone(), node.version.clone()));
            }
            Children::Branch(m) => {
                for (byte, child) in m {
                    prefix.push(*byte);
                    walk(child, prefix, out);
                    prefix.pop();
                }
            }
        }
        prefix.truncate(saved);
    }
    let mut out = Vec::new();
    walk(node, &mut Vec::new(), &mut out);
    out
}

/// Recursively count nodes that violate the "branches have 2+ live children
/// unless tombstones justify staying a branch" invariant. Pure-insert trees
/// have no tombstones, so every branch must have at least 2 live children.
fn undersized_branches_without_tombstones<Q: Ord>(node: &Node<Q>) -> usize {
    let mut bad = 0;
    fn walk<Q: Ord>(node: &Node<Q>, bad: &mut usize) {
        if let Children::Branch(m) = &node.children {
            if m.len() < 2 && node.deleted.is_empty() {
                *bad += 1;
            }
            for child in m.values() {
                walk(child, bad);
            }
        }
    }
    walk(node, &mut bad);
    bad
}

/// Join all leaf versions in the subtree rooted at `node`.
fn leaves_version_join<Q: Ord + Clone>(node: &Node<Q>) -> Version<Q> {
    Version::new(leaves(node).into_iter().map(|(_, _, v)| v))
}

/// A root freshly built with the empty state absorbs the first insert: its
/// path becomes the full inserted path, its children become a single leaf,
/// and its version becomes the inserted version.
#[test]
fn insert_into_empty_node_absorbs_as_leaf() {
    let mut n = Node::<P>::new();
    let v = ver(&[(1, 1)]);
    let applied = n.insert(v.clone(), rev(&[10, 20, 30]), Bytes::from_static(b"x"));
    assert!(applied);
    assert_eq!(n.path, rev(&[10, 20, 30]));
    assert_eq!(n.version, v);
    assert!(n.deleted.is_empty());
    assert!(matches!(&n.children, Children::Leaf(b) if b.as_ref() == b"x"));
}

/// Reinserting the same path with a concurrent version replaces the leaf
/// bytes (debug_assert ensures they match) and joins the versions at the
/// leaf.
#[test]
fn insert_replaces_leaf_and_joins_version() {
    let mut n = Node::<P>::new();
    let v1 = ver(&[(1, 1)]);
    let v2 = ver(&[(2, 1)]);
    assert!(n.insert(v1.clone(), rev(&[0, 1, 2]), Bytes::from_static(b"v")));
    assert!(n.insert(v2.clone(), rev(&[0, 1, 2]), Bytes::from_static(b"v")));
    assert_eq!(n.version, v1 | v2);
    assert!(matches!(&n.children, Children::Leaf(b) if b.as_ref() == b"v"));
}

/// When a second insert diverges at the last byte of an existing leaf's
/// path, the node splits into a branch whose compressed path is the common
/// prefix and whose two leaf children carry empty paths.
#[test]
fn insert_splits_at_last_byte() {
    let mut n = Node::<P>::new();
    let va = ver(&[(1, 1)]);
    let vb = ver(&[(2, 1)]);
    n.insert(va.clone(), rev(&[1, 2, 3]), Bytes::from_static(b"a"));
    n.insert(vb.clone(), rev(&[1, 2, 4]), Bytes::from_static(b"b"));

    assert_eq!(n.path, rev(&[1, 2]));
    assert_eq!(n.version, va.clone() | vb.clone());
    let Children::Branch(m) = &n.children else {
        panic!("expected branch");
    };
    assert_eq!(m.len(), 2);
    let c3 = m.get(&3).expect("child at 3");
    let c4 = m.get(&4).expect("child at 4");
    assert!(c3.path.is_empty());
    assert!(c4.path.is_empty());
    assert!(matches!(&c3.children, Children::Leaf(b) if b.as_ref() == b"a"));
    assert!(matches!(&c4.children, Children::Leaf(b) if b.as_ref() == b"b"));
    assert_eq!(c3.version, va);
    assert_eq!(c4.version, vb);
}

/// Splitting in the middle of a compressed path leaves each child with a
/// non-empty suffix of the original path.
#[test]
fn insert_splits_mid_path_preserves_suffixes() {
    let mut n = Node::<P>::new();
    n.insert(
        ver(&[(1, 1)]),
        rev(&[1, 2, 3, 4, 5]),
        Bytes::from_static(b"a"),
    );
    n.insert(
        ver(&[(2, 1)]),
        rev(&[1, 2, 7, 8, 9]),
        Bytes::from_static(b"b"),
    );

    assert_eq!(n.path, rev(&[1, 2]));
    let Children::Branch(m) = &n.children else {
        panic!("expected branch");
    };
    let c3 = m.get(&3).expect("child at 3");
    let c7 = m.get(&7).expect("child at 7");
    assert_eq!(c3.path, rev(&[4, 5]));
    assert_eq!(c7.path, rev(&[8, 9]));
}

/// A tombstone that strictly dominates the incoming version drops the insert
/// and leaves the node untouched.
#[test]
fn tombstone_dominated_insert_dropped() {
    let v_del = ver(&[(1, 3)]);
    let v_ins = ver(&[(1, 1)]);
    let mut n = Node::<P> {
        path: rev(&[1, 2]),
        version: v_del.clone(),
        deleted: vec![(RangeInclusive::new(0u8, 255u8), v_del.clone())],
        children: Children::Branch(BTreeMap::new()),
    };
    let applied = n.insert(v_ins, rev(&[1, 2, 3]), Bytes::from_static(b"x"));
    assert!(!applied);
    assert!(matches!(&n.children, Children::Branch(m) if m.is_empty()));
    assert_eq!(n.deleted.len(), 1);
    assert_eq!(n.version, v_del);
}

/// A tombstone concurrent with the incoming version does not block the
/// insert, and the tombstone itself is preserved verbatim so gossip peers
/// that missed the delete can still learn about it.
#[test]
fn tombstone_concurrent_insert_survives_tombstone_intact() {
    let v_del = ver(&[(1, 1)]);
    let v_ins = ver(&[(2, 1)]);
    let tombstone = (RangeInclusive::new(0u8, 255u8), v_del.clone());
    let mut n = Node::<P> {
        path: rev(&[1, 2]),
        version: v_del.clone(),
        deleted: vec![tombstone.clone()],
        children: Children::Branch(BTreeMap::new()),
    };
    let applied = n.insert(v_ins.clone(), rev(&[1, 2, 7]), Bytes::from_static(b"x"));
    assert!(applied);
    assert_eq!(n.deleted, vec![tombstone]);
    assert_eq!(n.version, v_del | v_ins);
    let Children::Branch(m) = &n.children else {
        panic!("expected branch");
    };
    assert!(m.contains_key(&7));
}

/// A tombstone strictly dominated by the incoming version does not block the
/// insert, and the tombstone is preserved intact.
#[test]
fn tombstone_newer_insert_survives_tombstone_intact() {
    let v_del = ver(&[(1, 1)]);
    let mut v_ins = v_del.clone();
    v_ins.event(1);
    let tombstone = (RangeInclusive::new(0u8, 255u8), v_del.clone());
    let mut n = Node::<P> {
        path: rev(&[1, 2]),
        version: v_del.clone(),
        deleted: vec![tombstone.clone()],
        children: Children::Branch(BTreeMap::new()),
    };
    assert!(n.insert(v_ins.clone(), rev(&[1, 2, 7]), Bytes::from_static(b"x")));
    assert_eq!(n.deleted, vec![tombstone]);
    assert_eq!(n.version, v_del | v_ins);
}

/// A tombstone whose range excludes the insert byte never blocks the insert,
/// regardless of its version.
#[test]
fn tombstone_range_excluding_byte_does_not_block() {
    let v_del = ver(&[(1, 5)]);
    let v_ins = ver(&[(1, 1)]);
    let tombstone = (RangeInclusive::new(100u8, 200u8), v_del.clone());
    let mut n = Node::<P> {
        path: rev(&[1, 2]),
        version: v_del.clone(),
        deleted: vec![tombstone.clone()],
        children: Children::Branch(BTreeMap::new()),
    };
    assert!(n.insert(v_ins.clone(), rev(&[1, 2, 50]), Bytes::from_static(b"x")));
    assert_eq!(n.deleted, vec![tombstone]);
    assert_eq!(n.version, v_del | v_ins);
}

fn arb_version() -> impl Strategy<Value = Version<P>> {
    prop::collection::vec((any::<P>(), 0u64..=3), 0..4).prop_map(|events| ver(&events))
}

/// Fixed-length paths keep every leaf at the same depth. Four bytes is
/// enough to exercise splits and descents without blowing up the search
/// space.
const PATH_LEN: usize = 4;

fn arb_insert() -> impl Strategy<Value = (Version<P>, [u8; PATH_LEN], u8)> {
    (arb_version(), any::<[u8; PATH_LEN]>(), any::<u8>())
}

fn arb_inserts() -> impl Strategy<Value = Vec<(Version<P>, [u8; PATH_LEN], u8)>> {
    prop::collection::vec(arb_insert(), 0..12)
}

/// Apply a sequence of inserts to a fresh node. Because the hash-derived
/// path is a function of the value in the real API, test inputs with the
/// same path but different `tag` bytes represent a (hypothetical) collision;
/// the insert contract says same-path values are structurally equal, so we
/// derive the value bytes from the path alone, reusing `tag` only as extra
/// entropy on the path itself.
type InsertLog = Vec<(Vec<u8>, Version<P>)>;

fn apply_inserts(inserts: &[(Version<P>, [u8; PATH_LEN], u8)]) -> (Node<P>, InsertLog) {
    let mut n = Node::<P>::new();
    let mut applied_log: InsertLog = Vec::new();
    for (v, path, _tag) in inserts {
        let value = Bytes::copy_from_slice(path);
        let ok = n.insert(v.clone(), rev(path), value);
        assert!(
            ok,
            "insert with no tombstones present must never be dropped",
        );
        applied_log.push((path.to_vec(), v.clone()));
    }
    (n, applied_log)
}

proptest! {
    /// Every leaf sits at exactly `PATH_LEN` bytes from the root, regardless
    /// of how many splits and descents the insert sequence triggers.
    #[test]
    fn leaves_are_at_fixed_depth(seq in arb_inserts()) {
        let (n, _) = apply_inserts(&seq);
        for (p, _, _) in leaves(&n) {
            prop_assert_eq!(p.len(), PATH_LEN);
        }
    }

    /// With pure inserts (no deletions), every branch node has at least two
    /// live children: one-child branches would have been path-compressed
    /// into their parent. The empty tree (no inserts) is exempt: its root
    /// is a zero-child branch until the first value arrives.
    #[test]
    fn pure_inserts_produce_no_singleton_branches(seq in arb_inserts()) {
        prop_assume!(!seq.is_empty());
        let (n, _) = apply_inserts(&seq);
        prop_assert_eq!(undersized_branches_without_tombstones(&n), 0);
    }

    /// A leaf's recorded path matches the path at which it was inserted;
    /// a leaf's bytes match the value inserted at that path; duplicate
    /// inserts at the same path collapse to one leaf carrying the joined
    /// version.
    #[test]
    fn inserted_leaves_round_trip(seq in arb_inserts()) {
        let (n, log) = apply_inserts(&seq);

        let mut expected: std::collections::BTreeMap<Vec<u8>, Version<P>> =
            std::collections::BTreeMap::new();
        for (p, v) in log {
            expected
                .entry(p)
                .and_modify(|acc| *acc |= v.clone())
                .or_insert(v);
        }

        let ls: std::collections::BTreeMap<Vec<u8>, (Bytes, Version<P>)> = leaves(&n)
            .into_iter()
            .map(|(p, b, v)| (p, (b, v)))
            .collect();

        prop_assert_eq!(ls.len(), expected.len());
        for (p, v) in expected {
            let (b, lv) = ls.get(&p).expect("leaf present for inserted path");
            prop_assert_eq!(b.as_ref(), p.as_slice());
            prop_assert_eq!(lv, &v);
        }
    }

    /// Every node's version is exactly the join of its live descendants'
    /// versions. "Join" here is the version-vector upper bound taken over
    /// every leaf reachable from the node.
    #[test]
    fn subtree_version_equals_leaf_join(seq in arb_inserts()) {
        let (n, _) = apply_inserts(&seq);
        fn check(node: &Node<P>) -> Result<(), TestCaseError> {
            let expected = leaves_version_join(node);
            prop_assert_eq!(&node.version, &expected);
            if let Children::Branch(m) = &node.children {
                for child in m.values() {
                    check(child)?;
                }
            }
            Ok(())
        }
        check(&n)?;
    }

    /// The set of leaf paths held by the tree equals the set of distinct
    /// inserted paths, regardless of insert order or duplication.
    #[test]
    fn leaf_path_set_matches_distinct_insert_paths(seq in arb_inserts()) {
        let (n, log) = apply_inserts(&seq);
        let got: BTreeSet<Vec<u8>> = leaves(&n).into_iter().map(|(p, _, _)| p).collect();
        let want: BTreeSet<Vec<u8>> = log.into_iter().map(|(p, _)| p).collect();
        prop_assert_eq!(got, want);
    }
}

type InsertSeq = Vec<(Version<P>, [u8; PATH_LEN], u8)>;

/// A sequence of inserts paired with a random permutation of its own
/// indices. The pair feeds commutativity/permutation tests: inserting
/// `seq` into one node and the permuted sequence into another must yield
/// identical leaf state.
fn arb_inserts_and_permutation() -> impl Strategy<Value = (InsertSeq, InsertSeq)> {
    arb_inserts().prop_flat_map(|seq| {
        let n = seq.len();
        prop::collection::vec(any::<u32>(), n).prop_map(move |keys| {
            // Sort indices by the generated keys to obtain a uniformly
            // random permutation of 0..n.
            let mut indices: Vec<usize> = (0..n).collect();
            indices.sort_by_key(|&i| keys[i]);
            let permuted: Vec<_> = indices.into_iter().map(|i| seq[i].clone()).collect();
            (seq.clone(), permuted)
        })
    })
}

proptest! {
    /// Insertion order is irrelevant to the final state: any permutation of
    /// the same insert sequence yields the same tree, down to the structural
    /// representation.
    #[test]
    fn permutation_invariance((seq_a, seq_b) in arb_inserts_and_permutation()) {
        let (a, _) = apply_inserts(&seq_a);
        let (b, _) = apply_inserts(&seq_b);
        prop_assert_eq!(a, b);
    }

    /// Inserting into a clone never mutates the original: `Arc::make_mut`
    /// clones every descent-path node that was shared with the original, so
    /// structural sharing keeps the original byte-for-byte intact regardless
    /// of what the clone does afterwards.
    #[test]
    fn clone_and_insert_leaves_original_intact(
        (base, extra) in (arb_inserts(), arb_inserts()),
    ) {
        let (original, _) = apply_inserts(&base);
        let snapshot = original.clone();
        let mut clone = original.clone();
        for (v, path, _) in &extra {
            let _ = clone.insert(v.clone(), rev(path), Bytes::copy_from_slice(path));
        }
        prop_assert_eq!(original, snapshot);
    }

    /// An insert strictly dominated by a tombstone is a pure no-op: it
    /// returns `false` and leaves the node byte-for-byte unchanged. No
    /// partial mutation (child creation, version bump, tombstone edit) may
    /// occur before the drop decision.
    #[test]
    fn dominated_inserts_are_noops(
        seq in prop::collection::vec((0u64..10, any::<[u8; PATH_LEN]>()), 0..10),
    ) {
        // Tombstone version strictly dominates any v_ins with count < 10
        // on the same party and zero elsewhere.
        let v_t = ver(&[(1, 10)]);
        let seed = Node::<P> {
            path: Vec::new(),
            version: Version::default(),
            deleted: vec![(RangeInclusive::new(0u8, 255u8), v_t.clone())],
            children: Children::Branch(BTreeMap::new()),
        };
        let mut n = seed.clone();
        for (count, path) in seq {
            let v_ins = ver(&[(1, count)]);
            let applied = n.insert(v_ins, rev(&path), Bytes::copy_from_slice(&path));
            prop_assert!(!applied);
            prop_assert_eq!(&n, &seed);
        }
    }

    /// Inserts never retract versions: after applying further inserts, every
    /// leaf path that already existed still exists and carries a version
    /// greater than or equal to its prior value under the vector-clock
    /// partial order.
    #[test]
    fn inserts_do_not_decrease_leaf_versions(
        (base, extra) in (arb_inserts(), arb_inserts()),
    ) {
        let (mut n, _) = apply_inserts(&base);
        let before: std::collections::BTreeMap<Vec<u8>, Version<P>> = leaves(&n)
            .into_iter()
            .map(|(p, _, v)| (p, v))
            .collect();
        for (v, path, _) in &extra {
            let _ = n.insert(v.clone(), rev(path), Bytes::copy_from_slice(path));
        }
        let after: std::collections::BTreeMap<Vec<u8>, Version<P>> = leaves(&n)
            .into_iter()
            .map(|(p, _, v)| (p, v))
            .collect();
        for (p, v_old) in before {
            let v_new = after.get(&p).expect(
                "pre-existing leaf path must survive further inserts (insert never deletes)",
            );
            prop_assert!(
                matches!(v_old.partial_cmp(v_new), Some(Ordering::Less | Ordering::Equal)),
                "version decreased at path {:?}: {:?} -> {:?}",
                p,
                v_old,
                v_new,
            );
        }
    }
}

/// When a split rearranges a node's compressed path, its tombstones travel
/// with the old contents (which become the old-child under the separator
/// edge), not with the new intermediate. The intermediate is fresh
/// structure representing a split point; it has no deletion history of its
/// own.
#[test]
fn split_carries_tombstones_to_old_child() {
    let v_node = ver(&[(1, 1)]);
    let v_ins = ver(&[(2, 1)]);
    let v_ts = ver(&[(3, 1)]);
    let tombstone = (RangeInclusive::new(10u8, 20u8), v_ts);

    // Seed a branch with compressed path [1, 2], a lone live child at byte
    // 5, and a tombstone. A single-child branch is permitted here because
    // the tombstone justifies staying a branch (it carries history we must
    // preserve).
    let child_leaf = Node::<P> {
        path: rev(&[100]),
        version: v_node.clone(),
        deleted: Vec::new(),
        children: Children::Leaf(Bytes::from_static(b"original")),
    };
    let mut map = BTreeMap::new();
    map.insert(5u8, Arc::new(child_leaf));
    let mut n = Node::<P> {
        path: rev(&[1, 2]),
        version: v_node.clone(),
        deleted: vec![tombstone.clone()],
        children: Children::Branch(map),
    };

    // Insert at forward path [1, 9, 99] diverges at index 1 of self.path
    // (byte 2 vs byte 9), forcing a split: common prefix [1], old child
    // moves under byte 2 with empty residual path, new leaf at byte 9.
    assert!(n.insert(v_ins.clone(), rev(&[1, 9, 99]), Bytes::from_static(b"new")));

    assert_eq!(n.path, rev(&[1]));
    assert!(
        n.deleted.is_empty(),
        "the new intermediate must not inherit tombstones",
    );
    assert_eq!(n.version, v_node.clone() | v_ins.clone());

    let Children::Branch(m) = &n.children else {
        panic!("expected branch after split");
    };
    assert_eq!(m.len(), 2);

    let old_child = m.get(&2).expect("old contents moved under byte 2");
    assert!(old_child.path.is_empty());
    assert_eq!(
        old_child.deleted,
        vec![tombstone],
        "tombstones stay with the old child",
    );
    assert_eq!(old_child.version, v_node);
    let Children::Branch(old_map) = &old_child.children else {
        panic!("old child should still be a branch");
    };
    assert!(old_map.contains_key(&5));

    let new_leaf = m.get(&9).expect("new leaf at byte 9");
    assert_eq!(new_leaf.path, rev(&[99]));
    assert!(new_leaf.deleted.is_empty());
    assert_eq!(new_leaf.version, v_ins);
    assert!(matches!(&new_leaf.children, Children::Leaf(b) if b.as_ref() == b"new"));
}

/// Multiple disjoint tombstone ranges each arbitrate inserts in their own
/// range independently. An insert in the gap between ranges is unaffected;
/// an insert dominated by one tombstone is dropped regardless of its
/// relation to other tombstones; an insert concurrent with a tombstone
/// survives regardless of other tombstones.
#[test]
fn multiple_tombstone_ranges_arbitrate_independently() {
    let v_t1 = ver(&[(1, 3)]);
    let v_t2 = ver(&[(2, 3)]);
    let ts1 = (RangeInclusive::new(10u8, 20u8), v_t1.clone());
    let ts2 = (RangeInclusive::new(100u8, 200u8), v_t2.clone());
    let v_cmp1 = ver(&[(1, 1)]); // < v_t1, concurrent with v_t2
    let v_cmp2 = ver(&[(2, 1)]); // < v_t2, concurrent with v_t1

    let seed = Node::<P> {
        path: Vec::new(),
        version: v_t1.clone() | v_t2.clone(),
        deleted: vec![ts1.clone(), ts2.clone()],
        children: Children::Branch(BTreeMap::new()),
    };

    // Byte 15 falls in ts1's range; v_cmp1 < v_t1: dropped.
    let mut n = seed.clone();
    assert!(!n.insert(v_cmp1.clone(), rev(&[15]), Bytes::from_static(b"a")));
    assert_eq!(n, seed);

    // Byte 15 with v_cmp2 (concurrent with v_t1): survives; tombstones
    // remain intact.
    assert!(n.insert(v_cmp2.clone(), rev(&[15]), Bytes::from_static(b"b")));
    assert_eq!(n.deleted, seed.deleted);
    let Children::Branch(m) = &n.children else {
        panic!("expected branch")
    };
    assert!(m.contains_key(&15));

    // Byte 150 falls in ts2's range; v_cmp2 < v_t2: dropped, even though
    // byte 15 is already live.
    let before = n.clone();
    assert!(!n.insert(v_cmp2.clone(), rev(&[150]), Bytes::from_static(b"c")));
    assert_eq!(n, before);

    // Byte 150 with v_cmp1 (concurrent with v_t2): survives.
    assert!(n.insert(v_cmp1.clone(), rev(&[150]), Bytes::from_static(b"d")));
    assert_eq!(n.deleted, seed.deleted);
    let Children::Branch(m) = &n.children else {
        panic!("expected branch")
    };
    assert!(m.contains_key(&150));

    // Byte 50 sits in the gap between ranges: every insert here goes
    // through regardless of version, and tombstones remain intact.
    assert!(n.insert(v_cmp1.clone(), rev(&[50]), Bytes::from_static(b"e")));
    assert_eq!(n.deleted, seed.deleted);
    let Children::Branch(m) = &n.children else {
        panic!("expected branch")
    };
    assert!(m.contains_key(&50));
}
