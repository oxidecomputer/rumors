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
    assert_eq!(nodes[0].kind, NodeKind::Clock);
    assert_eq!(nodes[0].party.as_deref(), Some("1"));
    assert_eq!(nodes[0].event, "0");
    assert_eq!(nodes[0].stamp.as_deref(), Some("(1, 0)"));
}

/// A tick advances the clock's own component by one event and supersedes nothing
/// in the engine itself (it just appends a new node).
#[test]
fn tick_advances_the_event_component() {
    let mut engine = Engine::new();
    engine.replay(&[Op::Tick { x: 0 }]).unwrap();
    let nodes = engine.descriptors();
    assert_eq!(nodes.len(), 2);
    assert_eq!(nodes[1].party.as_deref(), Some("1"));
    assert_eq!(nodes[1].stamp.as_deref(), Some("(1, 1)"));
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
        .map(|&i| nodes[i].party.as_deref().unwrap())
        .collect();
    assert_eq!(
        halves,
        ["(0, 1)", "(1, 0)"].into_iter().collect(),
        "fork should yield the two complementary half-ids"
    );
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
    assert_eq!(nodes[3].stamp.as_deref(), Some("(1, 0)"));
}

/// Joining clocks whose ids overlap is rejected (here: the seed with itself),
/// and the engine reports which operands collided.
#[test]
fn join_rejects_overlapping_ids() {
    let mut engine = Engine::new();
    let err = engine.replay(&[Op::Join { a: 0, b: 0 }]).unwrap_err();
    assert_eq!(err, EngineError::JoinOverlap { a: 0, b: 0 });
}

/// `is_disjoint` is false for overlapping ids and for non-clock operands.
#[test]
fn is_disjoint_guards_join_validity() {
    let mut engine = Engine::new();
    engine
        .replay(&[Op::Fork { x: 0 }, Op::Peek { x: 1 }])
        .unwrap();
    assert!(engine.is_disjoint(1, 2)); // the two fork halves
    assert!(!engine.is_disjoint(0, 1)); // seed overlaps its own half
    assert!(!engine.is_disjoint(1, 3)); // node 3 is a message, not a clock
    assert!(!engine.is_disjoint(1, 99)); // out of range
}

/// A peek snapshots history into a message node without advancing the source,
/// which remains available and unchanged.
#[test]
fn peek_snapshots_without_advancing() {
    let mut engine = Engine::new();
    engine
        .replay(&[Op::Tick { x: 0 }, Op::Peek { x: 1 }])
        .unwrap();
    let nodes = engine.descriptors();
    assert_eq!(nodes.len(), 3);
    // The source clock is untouched by the peek.
    assert_eq!(nodes[1].stamp.as_deref(), Some("(1, 1)"));
    // The message carries only history (no id) and equals the source's version.
    assert_eq!(nodes[2].kind, NodeKind::Message);
    assert_eq!(nodes[2].party, None);
    assert_eq!(nodes[2].event, "1");
}

/// Merging a message folds in its history but does NOT tick the receiver's own
/// component: merging a clock's own (empty) peek leaves it unchanged.
#[test]
fn merge_does_not_tick_on_self_peek() {
    let mut engine = Engine::new();
    engine
        .replay(&[Op::Peek { x: 0 }, Op::Merge { t: 0, m: 1 }])
        .unwrap();
    let nodes = engine.descriptors();
    assert_eq!(nodes.len(), 3);
    // Had merge ticked, this would read "(1, 1)".
    assert_eq!(nodes[2].stamp.as_deref(), Some("(1, 0)"));
}

/// Merge brings in another node's history without ticking: a tick's event is
/// absorbed by the receiver, leaving the same height (not one higher).
#[test]
fn merge_absorbs_history_without_ticking() {
    let mut engine = Engine::new();
    engine
        .replay(&[
            Op::Tick { x: 0 },
            Op::Peek { x: 1 },
            Op::Merge { t: 0, m: 2 },
        ])
        .unwrap();
    let nodes = engine.descriptors();
    assert_eq!(nodes.len(), 4);
    // Seed (1,0) merges the message "1"; result is "(1, 1)", not "(1, 2)".
    assert_eq!(nodes[3].stamp.as_deref(), Some("(1, 1)"));
}

/// Replaying the same log twice yields byte-identical descriptors — the
/// determinism the URL-fragment replay model depends on.
#[test]
fn replay_is_deterministic() {
    let log = [
        Op::Fork { x: 0 },
        Op::Tick { x: 1 },
        Op::Peek { x: 3 },
        Op::Merge { t: 2, m: 4 },
        Op::Join { a: 5, b: 3 },
    ];
    let mut a = Engine::new();
    let mut b = Engine::new();
    a.replay(&log).unwrap();
    b.replay(&log).unwrap();
    assert_eq!(a.descriptors(), b.descriptors());
}

/// Build a valid op-log from random choices, interpreting each against a running
/// model so every operand is well-typed. Returns the log and the number of nodes
/// it should produce (seed included). `join` is excluded so logs never error,
/// isolating the determinism property; join correctness is covered above.
fn oplog_from_choices(choices: &[(u8, usize, usize)]) -> (Vec<Op>, usize) {
    let mut ops = Vec::new();
    let mut clocks: Vec<usize> = vec![0];
    let mut messages: Vec<usize> = Vec::new();
    let mut next = 1usize;
    for &(sel, a, b) in choices {
        let x = clocks[a % clocks.len()];
        match sel % 4 {
            0 => {
                ops.push(Op::Tick { x });
                clocks.push(next);
                next += 1;
            }
            1 => {
                ops.push(Op::Fork { x });
                clocks.push(next);
                clocks.push(next + 1);
                next += 2;
            }
            2 => {
                ops.push(Op::Peek { x });
                messages.push(next);
                next += 1;
            }
            _ => {
                if let Some(&m) = messages.get(b % messages.len().max(1)).filter(|_| !messages.is_empty())
                {
                    ops.push(Op::Merge { t: x, m });
                    clocks.push(next);
                    next += 1;
                } else {
                    // No message yet: fall back to a tick so the choice still progresses.
                    ops.push(Op::Tick { x });
                    clocks.push(next);
                    next += 1;
                }
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
