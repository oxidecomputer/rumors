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
//! `P` has already received the targeted message via an `on_message`
//! callback —
//! which is a function of the gossip schedule. A partitioned schedule
//! may legitimately suppress some redacts (because the targeted peer
//! hasn't observed the key yet), so the two schedules can converge
//! to genuinely different states. The meaningful property is
//! *self-consistency*, not equality with a hypothetical
//! un-partitioned twin.

mod common;

use proptest::prelude::*;

use crate::common::oracle::readout_multiset;
use crate::common::peer::quiesce;
use crate::common::schedule::{arb_schedule, execute_with};
use crate::common::window::arb_window_assignment;

const N_PEERS: std::ops::RangeInclusive<usize> = 3..=8;
const MAX_EVENTS: usize = 50;

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
    /// have issued.
    #[test]
    fn partition_then_heal_is_self_consistent(
        schedule in arb_schedule(any::<u64>(), N_PEERS, MAX_EVENTS)
            .prop_filter("non-empty", |s| !s.events.is_empty()),
        split_seed in any::<usize>(),
        partition_event_seed in any::<usize>(),
        windows in arb_window_assignment(),
    ) {
        let n = schedule.n_peers;
        let split_at = (split_seed % (n - 1)) + 1;
        let partition_event_count = partition_event_seed % (schedule.events.len() + 1);

        // Allow gossip when either (a) we're past the partitioned
        // prefix (the heal phase), or (b) both peers are on the same
        // side of the split. Otherwise, drop the event.
        let mut result = execute_with(&schedule, &windows, |a, b, event_idx| {
            event_idx >= partition_event_count || (a < split_at) == (b < split_at)
        });
        quiesce(&mut result.peers);

        let expected = result.oracle.expected_live();
        for (i, peer) in result.peers.iter().enumerate() {
            prop_assert_eq!(
                readout_multiset(&peer.local.snapshot()), expected.clone(),
                "partitioned peer {} diverged from partitioned oracle", i,
            );
        }
    }
}
