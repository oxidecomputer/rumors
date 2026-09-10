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
        Path::with_leaf_paths(versions.into_iter().zip(paths), || {
            let mut tree = Tree::<Bytes>::new();
            tree.act(&party_of("P"), (0..width).map(|_| insert_action(Bytes::new())));
            tree.warm_memos();
            let before = tree.clone();
            assert!(!tree.act(&party_of("P"), []));
            assert!(tree.root_is(&before));
            let missing = (0..32).map(|split| {
                let mut bytes = [0; 32];
                bytes[split] = 255;
                Action::Forget(Path::from(bytes))
            });
            assert!(!tree.act(&party_of("P"), missing));
            // Identical allocation means the cached hash and bounds survived;
            // value equality alone would miss a needless rebuild.
            assert!(tree.root_is(&before));
            assert_eq!(tree, before);
        });
    }

    /// A real edit in one root child preserves a warmed sibling even when
    /// missing-key redactions descend through that sibling's compressed path.
    #[test]
    fn mixed_edits_retain_unchanged_subtree(depth in 1usize..32, width in 2usize..12) {
        let versions: Vec<_> = (1..=width + 2).map(|i| version_for("P", i as u64)).collect();
        let mut paths: Vec<_> = (0..width).map(|i| {
            let mut bytes = [0; 32];
            bytes[depth] = i as u8;
            Path::from(bytes)
        }).collect();
        let target = Path::from([1; 32]);
        paths.push(target);
        paths.push(Path::from([2; 32]));
        Path::with_leaf_paths(versions.into_iter().zip(paths), || {
            let mut tree = Tree::<Bytes>::new();
            tree.act(&party_of("P"), (0..width + 2).map(|_| insert_action(Bytes::new())));
            tree.warm_memos();
            let before = tree.clone();
            let sibling = before.root.root.as_ref().unwrap().clone().into_children().remove(0).unwrap();
            let mut missing = [0; 32];
            missing[depth] = 255;
            assert!(tree.act(&party_of("P"), [
                Action::Forget(Path::from(missing)),
                Action::Forget(target),
                Action::Forget(Path::from(missing)),
            ]));
            let after = tree.root.root.as_ref().unwrap().clone().into_children().remove(0).unwrap();
            assert!(after.ptr_eq(&sibling));
            assert_eq!(tree.len(), width + 1);
            assert_eq!(before.len(), width + 2);
        });
    }

    /// Arbitrary interleaved edits agree with a map oracle for both live
    /// versions and the committed ceiling, at every divergence depth.
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
        let versions: Vec<_> = (1..=initial + actions.len())
            .map(|i| (version_for("P", i as u64), path(i - 1))).collect();
        Path::with_leaf_paths(versions, || {
            let mut tree = Tree::<Bytes>::new();
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
                    Action::Forget(key)
                }
            }).collect();
            // Only a key present before or after the batch contributes its
            // last action. Fresh insert/forget pairs are never published.
            let ceiling = touched.iter()
                .filter(|(key, _)| original.contains_key(key) || expected.contains_key(key))
                .map(|(_, stamp)| *stamp).max().unwrap_or(initial).max(initial);
            let changed = tree.act(&party_of("P"), batch);
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
        Path::with_leaf_paths([(initial, held), (inserted, fresh)], || {
            let mut tree = Tree::<Bytes>::new();
            tree.act(&party_of("P"), [insert_action(Bytes::new())]);
            tree.warm_memos();
            let before = tree.clone();
            let actions = std::iter::once(insert_action(Bytes::from_static(b"cancelled")))
                .chain(std::iter::repeat_n(Action::Forget(fresh), repeats));
            assert!(!tree.act(&party_of("P"), actions));
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
    let path = Path::for_leaf(&c);
    let mut tree = Tree::<Bytes>::new();
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
    let path = Path::for_leaf(&earlier);
    let mut tree = Tree::<Bytes>::new();
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
