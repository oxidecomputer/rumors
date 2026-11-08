//! Batch edits preserve sharing and version identity across tree depths.

use super::*;

proptest! {
    /// Empty batches and missing-key redactions retain the warmed root at
    /// every possible divergence depth, including a fully compressed spine.
    #[test]
    fn no_op_retains_root_at_any_depth(depth in 0usize..32, width in 1usize..12) {
        let versions: Vec<_> = (1..=width).map(|i| version_for("P", i as u64)).collect();
        let paths: Vec<_> = (0..width).map(|i| {
            let mut bytes = [0; 32];
            bytes[depth] = i as u8;
            Path::from(bytes)
        }).collect();
        // Versions the tree never holds, placed where each redaction descends
        // to a different depth before finding nothing.
        let missing: Vec<_> = (0..32).map(|split| {
            let mut bytes = [0; 32];
            bytes[split] = 255;
            (version_for("Q", split as u64 + 1), Path::from(bytes))
        }).collect();
        let fixture = versions.into_iter().zip(paths).chain(missing.iter().cloned());
        Path::with_leaf_paths(fixture, || {
            let mut tree = Tree::<Bytes>::default();
            tree.act(&party_of("P"), (0..width).map(|_| insert_action(Bytes::new())));
            tree.warm_memos();
            let before = tree.clone();
            assert!(!tree.act(&party_of("P"), []).changed);
            assert!(tree.root_is(&before));
            let missing = missing.iter().map(|(version, _)| Action::Forget(version.clone()));
            assert!(!tree.act(&party_of("P"), missing).changed);
            // Identical allocation means the cached hash and bounds survived;
            // value equality alone would miss a needless rebuild.
            assert!(tree.root_is(&before));
            assert_eq!(tree, before);
        });
    }

    /// A real edit in one root child preserves a warmed sibling even when
    /// missing-key redactions descend through that sibling's compressed path,
    /// and only the held leaf counts as removed.
    #[test]
    fn mixed_edits_retain_unchanged_subtree(depth in 1usize..32, width in 2usize..12) {
        let versions: Vec<_> = (1..=width + 2).map(|i| version_for("P", i as u64)).collect();
        let mut paths: Vec<_> = (0..width).map(|i| {
            let mut bytes = [0; 32];
            bytes[depth] = i as u8;
            Path::from(bytes)
        }).collect();
        let target = versions[width].clone();
        paths.push(Path::from([1; 32]));
        paths.push(Path::from([2; 32]));
        // A version the tree never holds, whose path descends through the
        // first root child's compressed span.
        let missing = version_for("Q", 1);
        let mut missing_path = [0; 32];
        missing_path[depth] = 255;
        let fixture = versions
            .into_iter()
            .zip(paths)
            .chain([(missing.clone(), Path::from(missing_path))]);
        Path::with_leaf_paths(fixture, || {
            let mut tree = Tree::<Bytes>::default();
            tree.act(&party_of("P"), (0..width + 2).map(|_| insert_action(Bytes::new())));
            tree.warm_memos();
            let before = tree.clone();
            let sibling = before.root.node.as_ref().unwrap().clone().into_children().remove(0).unwrap();
            let acted = tree.act(&party_of("P"), [
                Action::Forget(missing.clone()),
                Action::Forget(target.clone()),
                Action::Forget(missing.clone()),
            ]);
            assert!(acted.changed);
            // Only the held target was removed; the absent key, forgotten
            // twice, adds nothing.
            assert_eq!(acted.removed, 1);
            let after = tree.root.node.as_ref().unwrap().clone().into_children().remove(0).unwrap();
            assert!(after.ptr_eq(&sibling));
            assert_eq!(tree.len(), width + 1);
            assert_eq!(before.len(), width + 2);
        });
    }

    /// Arbitrary interleaved edits agree with a map oracle for the live
    /// versions, the committed ceiling, and the count of removed leaves, at
    /// every divergence depth.
    #[test]
    fn deep_batches_match_map(
        depth in 0usize..32,
        initial in 0usize..8,
        actions in proptest::collection::vec((any::<bool>(), 0usize..48), 0..32),
    ) {
        let path = |index: usize| {
            let mut bytes = [0; 32];
            bytes[depth] = index as u8;
            Path::from(bytes)
        };
        // Map every version a batch may stamp or forget: forgets name any of
        // the first 48 paths, held or not.
        let versions: Vec<_> = (1..=(initial + actions.len()).max(48))
            .map(|i| (version_for("P", i as u64), path(i - 1))).collect();
        Path::with_leaf_paths(versions, || {
            let mut tree = Tree::<Bytes>::default();
            tree.act(&party_of("P"), (0..initial).map(|_| insert_action(Bytes::new())));
            let before = tree.clone();
            let mut expected: std::collections::BTreeMap<_, _> =
                (0..initial).map(|i| (path(i), i + 1)).collect();
            let original = expected.clone();
            let mut touched = std::collections::BTreeMap::new();
            let batch: Vec<_> = actions.iter().enumerate().map(|(i, &(insert, index))| {
                let stamp = initial + i + 1;
                if insert {
                    let key = path(stamp - 1);
                    expected.insert(key, stamp);
                    touched.insert(key, stamp);
                    insert_action(Bytes::new())
                } else {
                    let key = path(index);
                    expected.remove(&key);
                    touched.insert(key, stamp);
                    Action::Forget(version_for("P", index as u64 + 1))
                }
            }).collect();
            // Only a key present before or after the batch contributes its
            // last action. Fresh insert/forget pairs are never published.
            let ceiling = touched.iter()
                .filter(|(key, _)| original.contains_key(key) || expected.contains_key(key))
                .map(|(_, stamp)| *stamp).max().unwrap_or(initial).max(initial);
            let acted = tree.act(&party_of("P"), batch);
            let changed = acted.changed;
            // A removal is a key held before the batch and not after it.
            let removed = original.keys().filter(|key| !expected.contains_key(*key)).count();
            assert_eq!(acted.removed, removed);
            let expected_versions: Vec<_> = expected.values()
                .map(|&stamp| version_for("P", stamp as u64)).collect();
            let actual: Vec<_> = tree.iter().map(|(version, _)| version.clone()).collect();
            assert_eq!(actual, expected_versions);
            assert_eq!(tree.latest(), &version_for("P", ceiling as u64));
            assert_eq!(changed, expected != original);
            if !changed { assert!(tree.root_is(&before)); }
            for version in &actual { assert!(tree.get(version).is_some()); }
        });
    }

    /// Stable grouping retains insertion/forget order on a fresh key at any
    /// depth, without replacing a pre-existing root when the pair cancels.
    #[test]
    fn cancelled_insert_retains_root(depth in 0usize..32, repeats in 1usize..8) {
        let initial = version_for("P", 1);
        let inserted = version_for("P", 2);
        let held = Path::from([0; 32]);
        let mut fresh = [0; 32];
        fresh[depth] = 1;
        let fresh = Path::from(fresh);
        Path::with_leaf_paths([(initial, held), (inserted.clone(), fresh)], || {
            let mut tree = Tree::<Bytes>::default();
            tree.act(&party_of("P"), [insert_action(Bytes::new())]);
            tree.warm_memos();
            let before = tree.clone();
            let actions = std::iter::once(insert_action(Bytes::from_static(b"cancelled")))
                .chain(std::iter::repeat_n(Action::Forget(inserted.clone()), repeats));
            assert!(!tree.act(&party_of("P"), actions).changed);
            assert!(tree.root_is(&before));
        });
    }
}

/// Even outside the ascending-action contract, an inserted leaf keeps its
/// own version, rather than the join of concurrent actions at its key.
#[test]
fn concurrent_fixture_stores_insert_version() {
    let a = version_for("A", 1);
    let b = version_for("B", 1);
    let c = version_for("C", 1);
    let path = Path::for_fixture_leaf(&c);
    let mut tree = Tree::<Bytes>::default();
    tree.react([(path, a, traverse::Action::Insert(msg(Bytes::new())))]);
    tree.react([
        (path, b, traverse::Action::Forget),
        (path, c.clone(), traverse::Action::Insert(msg(Bytes::new()))),
    ]);
    assert_eq!(tree.iter().next().unwrap().0, &c);
    assert!(tree.get(&c).is_some());
}

/// Descending actions are outside `react`'s contract: a forget followed by
/// an earlier insert need not stay deleted, but the leaf's version is its own.
#[test]
fn descending_fixture_stores_insert_version() {
    let earlier = version_for("P", 1);
    let later = version_for("P", 2);
    let path = Path::for_fixture_leaf(&earlier);
    let mut tree = Tree::<Bytes>::default();
    tree.react([
        (path, later, traverse::Action::Forget),
        (
            path,
            earlier.clone(),
            traverse::Action::Insert(msg(Bytes::new())),
        ),
    ]);
    assert_eq!(tree.iter().next().unwrap().0, &earlier);
    assert!(tree.get(&earlier).is_some());
}
