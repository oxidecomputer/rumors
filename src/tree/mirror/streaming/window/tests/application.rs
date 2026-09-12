//! Check the estimates against actual trees and the queues that use them.
//!
//! These fixtures keep production hashing. They vary shared history, additions,
//! redactions, and budgets, then check the counted prefixes, issued work, and
//! installed capacities. Deep clustered fixtures test progress elsewhere; they
//! are not required to meet estimates derived for uniform hashes.

use std::collections::{BTreeMap, BTreeSet};

use proptest::prelude::*;

use super::super::{
    KEY_DEPTH, WindowConfig, children_quantile, disputed, occupied, stage_population,
};
use crate::message::Message;
use crate::testing::run_to_quiescence;
use crate::tree::mirror::streaming::channel::{QueueKind, with_observation};
use crate::tree::mirror::streaming::materialized::progress::{Kind, with_trace};
use crate::tree::mirror::streaming::{Local, materialized::Handshaking, mirror};
use crate::tree::typed::Path;
use crate::tree::{Action, Tree};

/// Build one shared history and concurrent edits using disjoint parties.
fn replicas(common: u64, additions: [u64; 2], redactions: [usize; 2]) -> [Tree<u64>; 2] {
    let mut party = before::Party::seed();
    let mut tree = Tree::<u64>::new();
    tree.act(&party, (0..common).map(|i| Action::Insert(Message::new(i))));
    let paths: Vec<_> = tree
        .iter()
        .map(|(version, _)| Path::for_leaf(version))
        .collect();
    let other = party.fork();
    let mut trees = [tree.clone(), tree];
    for (side, (tree, party)) in trees.iter_mut().zip([&party, &other]).enumerate() {
        tree.act(
            party,
            (0..additions[side]).map(|i| Action::Insert(Message::new(i))),
        );
        tree.act(
            party,
            paths
                .iter()
                .skip(side)
                .step_by(2)
                .take(redactions[side])
                .copied()
                .map(Action::Forget),
        );
    }
    trees
}

/// Check the occupied, disputed, and child counts by grouping actual leaf addresses.
fn check_tree_shape(trees: &[Tree<u64>; 2]) {
    let paths = trees.each_ref().map(|tree| {
        tree.iter()
            .map(|(version, _)| <[u8; 32]>::from(Path::for_leaf(version)))
            .collect::<BTreeSet<_>>()
    });
    let n = trees[0].len().max(trees[1].len()) as u128;
    let pair = trees[0].len() as u128 * trees[1].len() as u128;
    // Tag the three disjoint leaf families. Two families under a prefix
    // mean both replicas have it and their contents differ; shared alone
    // means a match. This counts tree contents, not model probabilities.
    let tagged: Vec<_> = paths[0]
        .difference(&paths[1])
        .map(|path| (path, 1))
        .chain(paths[1].difference(&paths[0]).map(|path| (path, 2)))
        .chain(paths[0].intersection(&paths[1]).map(|path| (path, 4)))
        .collect();
    for depth in 0..=KEY_DEPTH {
        let mut families = BTreeMap::<&[u8], u8>::new();
        for (path, family) in &tagged {
            *families.entry(&path[..depth]).or_default() |= family;
        }
        let disagreements = families
            .values()
            .filter(|mask| mask.count_ones() >= 2)
            .count();
        assert!(
            disagreements as u128 <= disputed(n, pair, depth),
            "disputes at depth {depth}"
        );
        for leaves in &paths {
            let mut parents = BTreeMap::<&[u8], BTreeSet<u8>>::new();
            for path in leaves {
                let children = parents.entry(&path[..depth]).or_default();
                if depth < KEY_DEPTH {
                    children.insert(path[depth]);
                }
            }
            assert!(
                parents.len() as u128 <= occupied(n, depth),
                "prefixes at depth {depth}"
            );
            assert!(
                parents
                    .values()
                    .all(|children| children.len() as u128 <= children_quantile(n, depth)),
                "fan at depth {depth}"
            );
        }
    }
}

/// Reconcile a fixture and check the model's scope count and height-to-capacity mapping.
fn check_session(trees: [Tree<u64>; 2], budget: usize) {
    check_tree_shape(&trees);
    let (a, b) = (trees[0].len() as u64, trees[1].len() as u64);
    let config = WindowConfig::Budget(budget);
    let window = config.resolve(
        a,
        b,
        trees[0].max_version_bytes() as u64,
        trees[1].max_version_bytes() as u64,
        Local::node_bytes,
    );
    let mut expected = trees[0].clone();
    expected.join(trees[1].clone());
    let [left, right] = trees;
    let ((result, trace), queues) = with_observation(|| {
        with_trace(|| {
            run_to_quiescence(mirror(
                Handshaking::start(Local, left.root.into()).window(config),
                Handshaking::start(Local, right.root.into()).window(config),
            ))
        })
    });
    let (left, right) = result
        .expect("a configured window stays live")
        .expect("valid reconciliation");
    assert_eq!(Tree::<u64>::from_root(left.into()), expected);
    assert_eq!(Tree::<u64>::from_root(right.into()), expected);

    let mut counts = BTreeMap::<usize, usize>::new();
    for event in trace.events() {
        if matches!(event.kind, Kind::InitialQuery | Kind::DependentWork) {
            *counts.entry(event.scope.len() + 1).or_default() += 1;
        }
    }
    for (depth, actual) in counts {
        let estimated =
            stage_population(u128::from(a.max(b)), u128::from(a) * u128::from(b), depth);
        assert!(
            actual as u128 <= estimated,
            "({a}, {b}) depth {depth}: {actual} queries exceed the population estimate {estimated}"
        );
    }
    for (role, stats) in queues.roles() {
        match role.kind {
            QueueKind::ResponderChildQueries
            | QueueKind::InternalChildQueries
            | QueueKind::InternalParentResolutions
            | QueueKind::InternalChildResolutions
            | QueueKind::LeafRequests
            | QueueKind::LeafParentResolutions
            | QueueKind::LeafChildResolutions => {
                assert_eq!(
                    stats.effective_capacity,
                    window.capacity(role.height),
                    "queue {role:?} must use its own height's width"
                );
            }
            _ => {}
        }
    }
}

/// Empty joins, nearly matching replicas, and broad divergence use the same sizing rules.
#[test]
fn window_sizing_covers_shared_and_asymmetric_sessions() {
    for (common, additions) in [
        (0, [0, 256]),
        (0, [256, 0]),
        (1, [0, 256]),
        (256, [0, 1]),
        (256, [256, 256]),
        (0, [256, 256]),
    ] {
        for budget in [0, 256 * 1024, 2 * 1024 * 1024] {
            check_session(replicas(common, additions, [0, 0]), budget);
        }
    }
}

proptest! {
    /// Varied additions and redactions stay within the shape estimate and receive the intended widths.
    #[test]
    fn window_sizing_matches_session_work(
        common in 0u64..128, additions in proptest::array::uniform2(0u64..128),
        redactions in proptest::array::uniform2(0usize..64), budget in 0usize..4 * 1024 * 1024,
    ) {
        check_session(replicas(common, additions, redactions), budget);
    }
}
