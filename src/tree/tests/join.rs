//! Joins preserve the live set, causal ceiling, and shared storage at any depth.

use super::*;

proptest! {
    /// Interleaved shared, new, and deleted leaves merge to the set oracle at
    /// every divergence depth and fan width, in either direction.
    #[test]
    fn divergent_fans_match_set_oracle(
        depth in 0usize..32,
        choices in proptest::collection::vec((any::<bool>(), any::<bool>(), any::<bool>()), 1..=256),
    ) {
        // For shared leaves, the two flags say which sides still hold them.
        // For new leaves, the first flag chooses the sole author. This gives
        // each path one version while mixing all merge cases in radix order.
        let path = |index: usize| {
            let mut bytes = [0; 32];
            bytes[depth] = u8::try_from(index).unwrap();
            Path::from(bytes)
        };
        let mut placements = Vec::new();
        let mut expected = std::collections::BTreeMap::new();
        let mut base_version = Version::new();
        let shared_party = party_of("A");
        let mut shared = Vec::new();
        for (index, &(common, ours, theirs)) in choices.iter().enumerate() {
            if common {
                base_version.tick(&shared_party);
                let version = base_version.clone();
                placements.push((version.clone(), path(index)));
                shared.push((path(index), version.clone(), traverse::Action::Insert(Message::new(index))));
                if ours && theirs {
                    expected.insert(path(index), (version, index));
                }
            }
        }
        let mut edits = [Vec::new(), Vec::new()];
        for (side, party) in [party_of("B"), party_of("C")].iter().enumerate() {
            let mut version = base_version.clone();
            for (index, &(common, ours, theirs)) in choices.iter().enumerate() {
                if common && ![ours, theirs][side] {
                    version.tick(party);
                    edits[side].push((path(index), version.clone(), traverse::Action::Forget));
                } else if !common && ours == (side == 0) {
                    version.tick(party);
                    placements.push((version.clone(), path(index)));
                    expected.insert(path(index), (version.clone(), index));
                    edits[side].push((path(index), version.clone(), traverse::Action::Insert(Message::new(index))));
                }
            }
        }
        Path::with_leaf_paths(placements, || {
            let mut base = Tree::<usize>::new();
            base.react(shared);
            let mut ours = base.clone();
            let mut theirs = base;
            let [our_edits, their_edits] = edits;
            ours.react(our_edits);
            theirs.react(their_edits);
            ours.warm_memos();
            theirs.warm_memos();
            let ceiling = ours.latest() | theirs.latest();
            let original = ours.clone();
            let counter = theirs.clone();
            let hash_before = ours.hash();
            let changed = ours.join(theirs);
            let actual: std::collections::BTreeMap<_, _> = ours.iter()
                .map(|(version, value)| (Path::for_leaf(version), (version.clone(), *value)))
                .collect();
            assert_eq!(actual, expected);
            assert_eq!(ours.latest(), &ceiling);
            assert_eq!(changed, ours.hash() != hash_before);
            let mut reverse = counter;
            reverse.join(original);
            assert_eq!(ours, reverse);
            let before = ours.clone();
            assert!(!ours.join(reverse));
            assert!(ours.root_is(&before));
        });
    }

    /// Joining a change beside a shared subtree keeps that subtree's allocation
    /// and its warmed memos, including a compressed path of any length.
    #[test]
    fn join_retains_equal_sibling(depth in 1usize..32, width in 2usize..64) {
        let mut version = Version::new();
        let party = party_of("P");
        let paths: Vec<_> = (0..width + 2).map(|index| {
            version.tick(&party);
            let mut bytes = [0; 32];
            if index < width {
                bytes[depth] = u8::try_from(index).unwrap();
            } else {
                bytes[0] = u8::try_from(index - width + 1).unwrap();
            }
            (version.clone(), Path::from(bytes))
        }).collect();
        Path::with_leaf_paths(paths, || {
            let mut ours = Tree::<Bytes>::new();
            ours.act(&party, (0..width + 1).map(|_| insert_action(Bytes::new())));
            ours.warm_memos();
            let sibling = ours.root.root.as_ref().unwrap().clone().into_children().remove(0).unwrap();
            let mut theirs = ours.clone();
            theirs.act(&party, [insert_action(Bytes::new())]);
            assert!(ours.join(theirs));
            let kept = ours.root.root.unwrap().into_children().remove(0).unwrap();
            assert!(kept.ptr_eq(&sibling));
        });
    }
}
