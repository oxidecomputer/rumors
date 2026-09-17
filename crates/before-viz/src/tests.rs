//! Host-side checks for engine behavior and operation-log encoding.

use super::*;
use proptest::prelude::*;

/// A new engine contains one live seed clock with an empty history.
#[test]
fn seed_is_the_only_node() {
    let e = Engine::new();
    let nodes = e.descriptors();
    assert_eq!(nodes.len(), 1);
    assert_eq!(
        nodes[0].party,
        vec![PartyRegion {
            owned: true,
            depth: 0,
        }]
    );
    assert_eq!(nodes[0].version, vec![VersionPlateau { rise: 0, depth: 0 }]);
    assert_eq!(e.live_indices(), vec![0]);
}

/// Ticking the seed advances its version by one event.
#[test]
fn tick_advances_the_event_component() {
    let mut e = Engine::new();
    e.load(vec![Op::Tick { x: 0 }]).unwrap();
    assert_eq!(
        e.descriptors()[1].version,
        vec![VersionPlateau { rise: 1, depth: 0 }]
    );
}

/// Forking the seed produces two disjoint parties whose regions cover the seed.
#[test]
fn fork_splits_into_two_disjoint_halves() {
    let mut e = Engine::new();
    e.load(vec![Op::Fork { x: 0 }]).unwrap();
    let nodes = e.descriptors();
    assert_eq!(nodes.len(), 3);
    let left = vec![
        PartyRegion {
            owned: true,
            depth: 1,
        },
        PartyRegion {
            owned: false,
            depth: 1,
        },
    ];
    let right = vec![
        PartyRegion {
            owned: false,
            depth: 1,
        },
        PartyRegion {
            owned: true,
            depth: 1,
        },
    ];
    assert!(nodes[1].party == left || nodes[1].party == right);
    assert!(nodes[2].party == left || nodes[2].party == right);
    assert_ne!(nodes[1].party, nodes[2].party);
    assert!(e.is_disjoint(1, 2));
}

/// Joining the two children of a fork recovers the seed party.
#[test]
fn join_reunites_disjoint_halves() {
    let mut e = Engine::new();
    e.load(vec![Op::Fork { x: 0 }, Op::Join { a: 1, b: 2 }])
        .unwrap();
    assert_eq!(
        e.descriptors()[3].party,
        vec![PartyRegion {
            owned: true,
            depth: 0,
        }]
    );
}

/// Joining overlapping parties fails without changing the engine.
#[test]
fn join_rejects_overlapping_ids() {
    let mut e = Engine::new();
    let err = e.load(vec![Op::Join { a: 0, b: 0 }]).unwrap_err();
    assert_eq!(err, EngineError::JoinOverlap { a: 0, b: 0 });
    assert_eq!(e.node_count(), 1);
}

/// Sending copies the sender's history into the receiver without changing its party.
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
    assert_eq!(nodes[4].version, nodes[3].version);
}

/// Encoding and loading a URL fragment preserves every rendered clock.
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

/// Joining a live clock with an ancestor rewinds that ancestor's descendants.
///
/// The remaining live clocks must still own pairwise-disjoint parties.
#[test]
fn joining_a_historical_clock_rewinds_its_future() {
    let mut e = Engine::new();
    e.apply(Op::Fork { x: 0 }).unwrap();
    e.apply(Op::Tick { x: 1 }).unwrap();
    e.apply(Op::Join { a: 2, b: 1 }).unwrap();
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

/// An operation fails without mutation when rewinding one operand would remove another.
///
/// Joining an ancestor to its descendant exercises this check directly.
#[test]
fn op_orphaning_its_own_operand_is_rejected() {
    let mut e = Engine::new();
    e.apply(Op::Fork { x: 0 }).unwrap();
    e.apply(Op::Send { from: 1, to: 2 }).unwrap();
    let before = e.descriptors();
    let err = e.apply(Op::Join { a: 3, b: 1 }).unwrap_err();
    assert_eq!(err, EngineError::Orphaned { operand: 3 });
    assert_eq!(e.descriptors(), before);
}

/// Apply one generated command, using the same disjointness guard as the UI.
fn step(engine: &mut Engine, kind: u8, raw_a: usize, raw_b: usize) {
    let node_count = engine.node_count();
    let a = raw_a % node_count;
    match kind {
        0 => drop(engine.apply(Op::Tick { x: a })),
        1 => drop(engine.apply(Op::Fork { x: a })),
        2 => {
            let b = raw_b % node_count;
            if a != b && engine.is_disjoint(a, b) {
                drop(engine.apply(Op::Join { a, b }));
            }
        }
        _ => drop(engine.apply(Op::Send {
            from: a,
            to: raw_b % node_count,
        })),
    }
}

proptest! {
    /// Every generated operation sequence leaves the live parties pairwise disjoint.
    ///
    /// Commands may target historical nodes, so the property also exercises rewinding.
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

    /// Replaying the same generated commands produces the same fragment and clocks.
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

/// Assert that loading an invalid fragment reports `BadFragment` without mutation.
fn assert_rejects_as_bad_fragment(fragment: &str) {
    let mut e = crate::Engine::new();
    match e.load_fragment(fragment) {
        Err(crate::EngineError::BadFragment(_)) => {}
        other => panic!("fragment must reject as BadFragment, got {other:?}"),
    }
    assert_eq!(e.node_count(), 1, "a rejected load leaves the prior state");
}

/// A URL fragment with an overflowing index varint reports `BadFragment`.
///
/// The fragments are
/// `[tag 1, 0x80 x 10, 0x01]` base64url-encoded and its `0x00`-terminated
/// variant. If the shift were masked, the second fragment would decode as
/// a valid but incorrect index.
#[test]
fn overlong_varint_fragment_rejects_cleanly() {
    for fragment in ["AYCAgICAgICAgIAB", "AYCAgICAgICAgIAA"] {
        assert_rejects_as_bad_fragment(fragment);
    }
}

/// Every operation-log entry point enforces the same inclusive operation limit.
///
/// The test accepts a fragment at the limit, then checks that `apply`, `load`,
/// and fragment loading reject one additional operation without losing the
/// accepted state.
#[test]
fn op_budget_covers_every_entry() {
    let at_cap = oplog::encode(&vec![Op::Tick { x: 0 }; oplog::MAX_OPS]);
    let mut e = crate::Engine::new();
    e.load_fragment(&at_cap).expect("a log at the budget loads");
    assert_eq!(e.op_log().len(), oplog::MAX_OPS);

    match e.apply(Op::Tick { x: oplog::MAX_OPS }) {
        Err(crate::EngineError::TooManyOps(n)) => assert_eq!(n, oplog::MAX_OPS + 1),
        other => panic!("an apply past the op budget must reject as TooManyOps, got {other:?}"),
    }
    assert_eq!(e.op_log().len(), oplog::MAX_OPS);
    let frag = e.fragment();
    let mut e2 = crate::Engine::new();
    e2.load_fragment(&frag)
        .expect("the last accepted state must remain encodable");

    let past_cap = oplog::encode(&vec![Op::Tick { x: 0 }; oplog::MAX_OPS + 1]);
    assert_rejects_as_bad_fragment(&past_cap);
    let mut e3 = crate::Engine::new();
    match e3.load(vec![Op::Tick { x: 0 }; oplog::MAX_OPS + 1]) {
        Err(crate::EngineError::TooManyOps(n)) => assert_eq!(n, oplog::MAX_OPS + 1),
        other => panic!("a load past the op budget must reject as TooManyOps, got {other:?}"),
    }
}

/// The fragment-length precheck is exact: it admits every fragment a
/// within-budget log can encode and rejects longer strings before any
/// base64 work.
///
/// The widest op (a join of two maximal varint operands) encodes to 21
/// bytes, so a budget-full log of them is the longest legitimate
/// fragment — exactly `MAX_FRAGMENT_CHARS` — and one more such op pushes
/// past it.
#[test]
fn fragment_length_precheck_is_exact() {
    let fat = Op::Join {
        a: usize::MAX,
        b: usize::MAX,
    };
    let widest = oplog::encode(&vec![fat; oplog::MAX_OPS]);
    assert_eq!(widest.len(), oplog::MAX_FRAGMENT_CHARS);
    assert_eq!(
        oplog::decode(&widest)
            .expect("the widest within-budget fragment decodes")
            .len(),
        oplog::MAX_OPS
    );

    let over = oplog::encode(&vec![fat; oplog::MAX_OPS + 1]);
    assert!(over.len() > oplog::MAX_FRAGMENT_CHARS);
    assert!(oplog::decode(&over).is_err());
}
