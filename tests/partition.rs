//! Partition tolerance.
//!
//! A network split during which gossip is restricted to one side or
//! the other, followed by a heal phase, must produce a
//! self-consistent state: every peer agrees, and the state matches an
//! oracle constructed from the events that actually applied during
//! the partitioned execution.
//!
//! We deliberately do *not* compare against an unrestricted run of
//! the same schedule. Doing so would assume order-independence of
//! redactions, but a redact event can only happen at peer `P` once
//! `P` has already observed the targeted message, which depends on the gossip
//! schedule. A partitioned schedule
//! may legitimately suppress some redacts (because the targeted peer
//! hasn't observed the key yet), so the two schedules can converge
//! to genuinely different states. The meaningful property is
//! *self-consistency*, not equality with a hypothetical
//! un-partitioned twin.

use rumors_testkit::common;

use std::collections::BTreeMap;

use proptest::prelude::*;

use crate::common::oracle::readout_multiset;
use crate::common::peer::quiesce;
use crate::common::schedule::events::Event;
use crate::common::schedule::executor::ExecutionResult;
use crate::common::schedule::{Schedule, arb_schedule, execute_with};
use crate::common::window::{WindowAssignment, arb_window_assignment};

/// Fleet sizes exercised by generated partition schedules.
const N_PEERS: std::ops::RangeInclusive<usize> = 3..=8;
/// Maximum events in one generated partition schedule.
const MAX_EVENTS: usize = 50;

/// Execute a partitioned prefix, then heal the fleet to quiescence.
fn execute_partition_and_heal(
    schedule: &Schedule<u64>,
    split_at: usize,
    partition_event_count: usize,
    windows: &WindowAssignment,
) -> ExecutionResult<u64> {
    assert!((1..schedule.n_peers).contains(&split_at));
    assert!(partition_event_count <= schedule.events.len());

    let mut result = execute_with(schedule, windows, |a, b, event_idx| {
        event_idx >= partition_event_count || (a < split_at) == (b < split_at)
    });
    quiesce(&mut result.peers);
    result
}

proptest! {
    /// A schedule executed under a partition followed by a full-mesh
    /// heal converges to a state in which every peer's readout
    /// multiset equals the partitioned execution's oracle.
    ///
    /// For the first `partition_event_count` events, gossip is
    /// allowed only within each side of the split at `split_at`;
    /// after that, any gossip event is allowed and a final quiesce
    /// drives the network to convergence. `execute_with` already
    /// honors the "only redact a message you've observed" invariant,
    /// so redacts whose targets never crossed the partition are
    /// silently skipped — matching what real application code could
    /// have issued. An empty schedule is valid and converges immediately.
    #[test]
    fn partition_then_heal_is_self_consistent(
        (schedule, split_at, partition_event_count) in
            arb_schedule(any::<u64>(), N_PEERS, MAX_EVENTS)
                .prop_flat_map(|schedule| {
                    let peers = schedule.n_peers;
                    let events = schedule.events.len();
                    (Just(schedule), 1..peers, 0..=events)
                }),
        windows in arb_window_assignment(),
    ) {
        let result = execute_partition_and_heal(
            &schedule,
            split_at,
            partition_event_count,
            &windows,
        );

        let expected = result.oracle.expected_live();
        for (i, peer) in result.peers.iter().enumerate() {
            prop_assert_eq!(
                readout_multiset(&peer.local.snapshot()), expected.clone(),
                "partitioned peer {} diverged from partitioned oracle", i,
            );
        }
    }
}

/// With an empty partitioned prefix, equal payloads inserted by different
/// peers remain two distinct messages as the fleet reaches quiescence.
#[test]
fn partition_boundary_preserves_equal_valued_inserts() {
    let schedule = Schedule {
        n_peers: 3,
        fork_parents: vec![0, 0, 0],
        events: vec![
            Event::Insert { peer: 0, value: 0 },
            Event::Insert { peer: 1, value: 0 },
        ],
    };
    let result = execute_partition_and_heal(&schedule, 1, 0, &WindowAssignment::floor());
    let expected = BTreeMap::from([(0, 2)]);

    assert_eq!(result.oracle.expected_live(), expected);
    for peer in &result.peers {
        assert_eq!(readout_multiset(&peer.local.snapshot()), expected);
    }
}
