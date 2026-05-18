//! Group E — partition tolerance.
//!
//! A network split (during which gossip is restricted to one side or
//! the other) followed by a heal phase must produce a self-consistent
//! state: every peer agrees, and the state matches the oracle of the
//! events actually applied during the partitioned execution.
//!
//! Note: we deliberately do *not* compare against an unrestricted run
//! of the same schedule. Doing so would assume order-independence of
//! redactions, but a `redact(K)` event can only happen at peer P if P
//! has already received `K` via an `on_message` callback — which is a
//! function of the gossip schedule. A partitioned schedule may
//! legitimately suppress some redacts (because the targeted peer
//! hasn't observed the key by then), so the two schedules can
//! converge to genuinely different states. The meaningful
//! partition-tolerance property is *self-consistency*, not equality
//! with a hypothetical un-partitioned twin.

use proptest::prelude::*;
use rumors::Key;

use crate::oracle::{Oracle, PartyId, readout_multiset};
use crate::peer::{Peer, gossip_step, quiesce};
use crate::schedule::{Event, Schedule, arb_schedule};

struct PartitionedResult {
    peers: Vec<Peer<u64>>,
    oracle: Oracle<u64>,
}

/// Re-implements `execute` but during the partition phase rewrites
/// any cross-partition `Gossip` events into no-ops. Local actions are
/// unaffected by the partition. A `Redact` of a `Key` the targeted
/// peer has not yet observed is skipped entirely — that event could
/// not have been issued in real usage.
fn execute_with_partition(
    schedule: &Schedule,
    split_at: usize,
    partition_event_count: usize,
) -> PartitionedResult {
    let party_ids: Vec<PartyId> = (0..schedule.n_peers).map(|i| format!("p{i}")).collect();
    let mut peers: Vec<Peer<u64>> = party_ids.iter().map(Peer::<u64>::new).collect();
    let mut oracle = Oracle::<u64>::new();
    let mut resolved_keys: std::collections::BTreeMap<usize, Key> =
        std::collections::BTreeMap::new();

    let in_partition = |p: usize| p < split_at;
    let allow_gossip = |a: usize, b: usize, event_idx: usize| -> bool {
        if event_idx >= partition_event_count {
            return true;
        }
        in_partition(a) == in_partition(b)
    };

    for (i, event) in schedule.events.iter().enumerate() {
        match event {
            Event::Insert { peer, value } => {
                let k = peers[*peer].insert_one(*value);
                resolved_keys.insert(i, k);
                oracle.insert(i, party_ids[*peer].clone(), *value);
            }
            Event::Redact {
                peer,
                target_event_idx,
            } => {
                // Schedule guarantees this target was observed by the
                // peer in the *unrestricted* execution; under a
                // partition, the gossip carrying the observation may
                // not have happened yet. Skip if so — the
                // partitioned execution then legitimately doesn't
                // record this redact in its oracle.
                if let Some(key) = resolved_keys.get(target_event_idx) {
                    if peers[*peer].observations.iter().any(|(k, _, _)| k == key) {
                        peers[*peer].redact_one(*key);
                        oracle.redact(*target_event_idx);
                    }
                }
            }
            Event::Gossip { a, b } => {
                if !allow_gossip(*a, *b, i) {
                    continue;
                }
                let (lo, hi) = if a < b { (*a, *b) } else { (*b, *a) };
                let (left, right) = peers.split_at_mut(hi);
                gossip_step(&mut left[lo], &mut right[0]);
            }
        }
    }

    quiesce(&mut peers);
    PartitionedResult { peers, oracle }
}

proptest! {
    /// E1: a schedule executed under a partition (gossip restricted to
    /// one side or the other for the first `partition_event_count`
    /// events) followed by a full-mesh heal converges to a
    /// self-consistent state: every peer's readout multiset equals
    /// the partitioned execution's oracle's `expected_live()`.
    #[test]
    fn e1_partition_then_heal_self_consistent(
        schedule in arb_schedule(3..=8, 50),
        split_seed in any::<usize>(),
        partition_event_seed in any::<usize>(),
    ) {
        let n = schedule.n_peers;
        if n < 3 || schedule.events.is_empty() {
            return Ok(());
        }
        let split_at = (split_seed % (n - 1)) + 1;
        let partition_event_count = partition_event_seed % (schedule.events.len() + 1);

        let result = execute_with_partition(&schedule, split_at, partition_event_count);
        let expected = result.oracle.expected_live();
        for (i, peer) in result.peers.iter().enumerate() {
            prop_assert_eq!(
                readout_multiset(&peer.local), expected.clone(),
                "partitioned peer {} diverged from partitioned oracle", i,
            );
        }
    }
}
