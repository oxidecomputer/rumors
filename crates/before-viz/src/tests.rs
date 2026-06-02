//! Host-target tests for the engine and the op-log algebra. The browser-facing wasm
//! wrapper is a thin JSON shim over this core.

use super::*;
use proptest::prelude::*;

/// The empty log is the seed clock: full id, empty history.
#[test]
fn seed_is_the_only_node() {
    let e = Engine::new();
    let nodes = e.descriptors();
    assert_eq!(nodes.len(), 1);
    assert_eq!(
        (
            nodes[0].party.as_str(),
            nodes[0].event.as_str(),
            nodes[0].stamp.as_str()
        ),
        ("1", "0", "(1, 0)")
    );
    assert_eq!(e.live_indices(), vec![0]);
}

/// A tick advances the clock's own component.
#[test]
fn tick_advances_the_event_component() {
    let mut e = Engine::new();
    e.load(vec![Op::Tick { x: 0 }]).unwrap();
    assert_eq!(e.descriptors()[1].stamp, "(1, 1)");
}

/// A fork emits two clocks owning disjoint, complementary halves of the id space.
#[test]
fn fork_splits_into_two_disjoint_halves() {
    let mut e = Engine::new();
    e.load(vec![Op::Fork { x: 0 }]).unwrap();
    let nodes = e.descriptors();
    assert_eq!(nodes.len(), 3);
    let halves: std::collections::BTreeSet<&str> = [1usize, 2]
        .iter()
        .map(|&i| nodes[i].party.as_str())
        .collect();
    assert_eq!(halves, ["(0, 1)", "(1, 0)"].into_iter().collect());
    assert!(e.is_disjoint(1, 2));
}

/// Joining the two halves of a fork reconstitutes the whole id.
#[test]
fn join_reunites_disjoint_halves() {
    let mut e = Engine::new();
    e.load(vec![Op::Fork { x: 0 }, Op::Join { a: 1, b: 2 }])
        .unwrap();
    assert_eq!(e.descriptors()[3].stamp, "(1, 0)");
}

/// Joining clocks whose ids overlap is rejected, leaving prior state intact.
#[test]
fn join_rejects_overlapping_ids() {
    let mut e = Engine::new();
    let err = e.load(vec![Op::Join { a: 0, b: 0 }]).unwrap_err();
    assert_eq!(err, EngineError::JoinOverlap { a: 0, b: 0 });
    assert_eq!(e.node_count(), 1); // unchanged
}

/// Send transfers the sender's history into the receiver without advancing its own
/// component or changing its id.
#[test]
fn send_transfers_history_without_ticking() {
    let mut e = Engine::new();
    e.load(vec![
        Op::Fork { x: 0 },
        Op::Tick { x: 1 },
        Op::Send { from: 3, to: 2 },
    ])
    .unwrap();
    let nodes = e.descriptors();
    assert_eq!(nodes.len(), 5);
    assert_eq!(nodes[4].party, nodes[2].party);
    assert_eq!(nodes[4].event, nodes[3].event);
}

/// The op-log round-trips through its URL fragment.
#[test]
fn fragment_round_trips() {
    let log = vec![
        Op::Fork { x: 0 },
        Op::Tick { x: 1 },
        Op::Send { from: 3, to: 2 },
        Op::Join { a: 4, b: 3 },
    ];
    let mut e = Engine::new();
    e.load(log).unwrap();
    let frag = e.fragment();
    let mut e2 = Engine::new();
    e2.load_fragment(&frag).unwrap();
    assert_eq!(e2.descriptors(), e.descriptors());
}

/// Regression for the non-disjoint-live bug: joining a *live* clock with a *historical*
/// one must rewind the historical clock's future (both join operands are anchors), not
/// leave it alive alongside the new lineage.
#[test]
fn joining_a_historical_clock_rewinds_its_future() {
    let mut e = Engine::new();
    e.apply(Op::Fork { x: 0 }).unwrap(); // live: 1, 2
    e.apply(Op::Tick { x: 1 }).unwrap(); // 1 superseded by 3; live: 2, 3
    e.apply(Op::Join { a: 2, b: 1 }).unwrap(); // join live 2 with historical 1
    let live = e.live_indices();
    for i in 0..live.len() {
        for j in (i + 1)..live.len() {
            assert!(
                e.is_disjoint(live[i], live[j]),
                "live {} and {} overlap",
                live[i],
                live[j]
            );
        }
    }
}

/// Interpret a random command against the engine: act on any node (exercising the
/// historical-rewind path), guarding joins by disjointness as the UI does. A rejected
/// op (e.g. an operand that the rewind would orphan) leaves state unchanged, mirroring
/// the UI declining the gesture; we test the invariant over the *successful* states.
fn step(e: &mut Engine, k: u8, ra: usize, rb: usize) {
    let n = e.node_count();
    let a = ra % n;
    match k {
        0 => drop(e.apply(Op::Tick { x: a })),
        1 => drop(e.apply(Op::Fork { x: a })),
        2 => {
            let b = rb % n;
            if a != b && e.is_disjoint(a, b) {
                drop(e.apply(Op::Join { a, b }));
            }
        }
        _ => drop(e.apply(Op::Send {
            from: a,
            to: rb % n,
        })),
    }
}

proptest! {
    /// The core invariant: after any sequence of operations — including acting on
    /// historical nodes, which rewinds — the live clocks are pairwise id-disjoint (the
    /// frontier always partitions the id space).
    #[test]
    fn live_clocks_stay_pairwise_disjoint(
        cmds in prop::collection::vec((0u8..4, any::<usize>(), any::<usize>()), 0..60)
    ) {
        let mut e = Engine::new();
        for (k, ra, rb) in cmds {
            step(&mut e, k, ra, rb);
            let live = e.live_indices();
            for i in 0..live.len() {
                for j in (i + 1)..live.len() {
                    prop_assert!(e.is_disjoint(live[i], live[j]), "live {} and {} overlap", live[i], live[j]);
                }
            }
        }
    }

    /// Applying the same command sequence is deterministic: same fragment, same nodes.
    #[test]
    fn apply_sequences_are_deterministic(
        cmds in prop::collection::vec((0u8..4, any::<usize>(), any::<usize>()), 0..50)
    ) {
        let run = || {
            let mut e = Engine::new();
            for &(k, ra, rb) in &cmds {
                step(&mut e, k, ra, rb);
            }
            (e.fragment(), e.descriptors())
        };
        prop_assert_eq!(run(), run());
    }
}
