//! Host-target tests for the replay engine. The browser-facing wasm wrapper is a
//! thin JSON shim over this core, so exercising the core covers the semantics.

use super::*;
use proptest::prelude::*;

/// The empty log yields exactly the seed clock: full id, empty history.
#[test]
fn seed_is_the_only_node_of_an_empty_log() {
    let mut engine = Engine::new();
    engine.replay(&[]).unwrap();
    let nodes = engine.descriptors();
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].party, "1");
    assert_eq!(nodes[0].event, "0");
    assert_eq!(nodes[0].stamp, "(1, 0)");
}

/// A tick advances the clock's own component by one event.
#[test]
fn tick_advances_the_event_component() {
    let mut engine = Engine::new();
    engine.replay(&[Op::Tick { x: 0 }]).unwrap();
    let nodes = engine.descriptors();
    assert_eq!(nodes.len(), 2);
    assert_eq!(nodes[1].stamp, "(1, 1)");
}

/// A fork emits two clocks that share the parent's history and own disjoint,
/// complementary halves of the id space.
#[test]
fn fork_splits_into_two_disjoint_halves() {
    let mut engine = Engine::new();
    engine.replay(&[Op::Fork { x: 0 }]).unwrap();
    let nodes = engine.descriptors();
    assert_eq!(nodes.len(), 3);
    let halves: std::collections::BTreeSet<&str> = [1usize, 2]
        .iter()
        .map(|&i| nodes[i].party.as_str())
        .collect();
    assert_eq!(halves, ["(0, 1)", "(1, 0)"].into_iter().collect());
    assert_eq!(nodes[1].event, "0");
    assert_eq!(nodes[2].event, "0");
    assert!(engine.is_disjoint(1, 2));
}

/// Joining the two halves of a fork reconstitutes the whole id with merged history.
#[test]
fn join_reunites_disjoint_halves() {
    let mut engine = Engine::new();
    engine
        .replay(&[Op::Fork { x: 0 }, Op::Join { a: 1, b: 2 }])
        .unwrap();
    let nodes = engine.descriptors();
    assert_eq!(nodes.len(), 4);
    assert_eq!(nodes[3].stamp, "(1, 0)");
}

/// Joining clocks whose ids overlap is rejected (here: the seed with itself).
#[test]
fn join_rejects_overlapping_ids() {
    let mut engine = Engine::new();
    let err = engine.replay(&[Op::Join { a: 0, b: 0 }]).unwrap_err();
    assert_eq!(err, EngineError::JoinOverlap { a: 0, b: 0 });
}

/// `is_disjoint` is false for overlapping ids and out-of-range indices.
#[test]
fn is_disjoint_guards_join_validity() {
    let mut engine = Engine::new();
    engine.replay(&[Op::Fork { x: 0 }]).unwrap();
    assert!(engine.is_disjoint(1, 2)); // the two fork halves
    assert!(!engine.is_disjoint(0, 1)); // seed overlaps its own half
    assert!(!engine.is_disjoint(1, 99)); // out of range
}

/// Send transfers the sender's history into the receiver without advancing the
/// receiver's own component or changing its id: the result keeps the receiver's
/// party and takes on the sender's (here previously-empty receiver) version.
#[test]
fn send_transfers_history_without_ticking() {
    // 0 seed; fork → 1,2; tick 1 → 3; send 3 → 2 → 4.
    let mut engine = Engine::new();
    engine
        .replay(&[
            Op::Fork { x: 0 },
            Op::Tick { x: 1 },
            Op::Send { from: 3, to: 2 },
        ])
        .unwrap();
    let nodes = engine.descriptors();
    assert_eq!(nodes.len(), 5);
    // The result keeps the receiver's id (node 2) and absorbs the sender's history
    // (node 3) without a tick — so its event equals the sender's, its party node 2's.
    assert_eq!(nodes[4].party, nodes[2].party);
    assert_eq!(nodes[4].event, nodes[3].event);
}

/// Replaying the same log twice yields byte-identical descriptors — the
/// determinism the URL-fragment replay model depends on.
#[test]
fn replay_is_deterministic() {
    let log = [
        Op::Fork { x: 0 },
        Op::Tick { x: 1 },
        Op::Send { from: 3, to: 2 },
        Op::Join { a: 4, b: 3 },
    ];
    let mut a = Engine::new();
    let mut b = Engine::new();
    a.replay(&log).unwrap();
    b.replay(&log).unwrap();
    assert_eq!(a.descriptors(), b.descriptors());
}

/// Build a valid op-log from random choices, interpreting each against a running
/// model so every operand is in range. `join` is excluded so logs never error,
/// isolating the determinism property; join correctness is covered above. Every op
/// produces exactly one new clock except fork (two).
fn oplog_from_choices(choices: &[(u8, usize, usize)]) -> (Vec<Op>, usize) {
    let mut ops = Vec::new();
    let mut next = 1usize; // 0 is the seed
    for &(sel, a, b) in choices {
        let x = a % next;
        match sel % 3 {
            0 => {
                ops.push(Op::Tick { x });
                next += 1;
            }
            1 => {
                ops.push(Op::Fork { x });
                next += 2;
            }
            _ => {
                ops.push(Op::Send {
                    from: x,
                    to: b % next,
                });
                next += 1;
            }
        }
    }
    (ops, next)
}

proptest! {
    /// For any valid log, replay succeeds, produces the predicted node count, and
    /// is deterministic across two engines.
    #[test]
    fn random_valid_logs_replay_deterministically(
        choices in prop::collection::vec((any::<u8>(), any::<usize>(), any::<usize>()), 0..40)
    ) {
        let (ops, expected_nodes) = oplog_from_choices(&choices);
        let mut a = Engine::new();
        let mut b = Engine::new();
        a.replay(&ops).unwrap();
        b.replay(&ops).unwrap();
        prop_assert_eq!(a.descriptors().len(), expected_nodes);
        prop_assert_eq!(a.descriptors(), b.descriptors());
    }
}
