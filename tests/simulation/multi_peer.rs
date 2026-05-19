//! Multi-peer eventual consistency under randomised schedules.
//!
//! Each test runs a proptest-generated [`Schedule`] through the
//! executor and then a full-mesh quiesce, asserting that every peer
//! agrees with every other and with the oracle's projection of the
//! same schedule.
//!
//! [`Schedule`]: crate::schedule::Schedule

use std::collections::BTreeMap;

use proptest::prelude::*;
use rumors::Key;

use crate::oracle::{readout, readout_multiset};
use crate::peer::gossip_step;
use crate::schedule::{Schedule, arb_schedule, execute_and_quiesce};

const N_PEERS: std::ops::RangeInclusive<usize> = 2..=8;
const MAX_EVENTS: usize = 50;

fn schedule_u64() -> impl Strategy<Value = Schedule<u64>> {
    arb_schedule(any::<u64>(), N_PEERS, MAX_EVENTS)
}

fn schedule_string() -> impl Strategy<Value = Schedule<String>> {
    arb_schedule("[a-z]{0,8}".prop_map(String::from), N_PEERS, MAX_EVENTS)
}

proptest! {
    /// After the final quiesce phase, every peer's live content (per
    /// `readout`) matches every other's. Compared via `readout`
    /// rather than `Local::eq` because the latter includes the party
    /// tag, which always differs across peers.
    #[test]
    fn all_peers_converge_after_quiesce(
        schedule in schedule_u64(),
    ) {
        let result = execute_and_quiesce(&schedule);
        let first = readout(&result.peers[0].local);
        for (i, peer) in result.peers.iter().enumerate().skip(1) {
            prop_assert_eq!(
                readout(&peer.local), first.clone(),
                "peer {} diverged from peer 0", i,
            );
        }
    }

    /// After quiesce, every peer's readout multiset equals the
    /// oracle's `expected_live()`. The oracle is pure data and never
    /// invokes `process`, so this is a genuinely independent check.
    #[test]
    fn readout_matches_oracle_after_quiesce(
        schedule in schedule_u64(),
    ) {
        let result = execute_and_quiesce(&schedule);
        let expected = result.oracle.expected_live();
        for (i, peer) in result.peers.iter().enumerate() {
            let actual = readout_multiset(&peer.local);
            prop_assert_eq!(
                &actual, &expected,
                "peer {} readout does not match oracle", i,
            );
        }
    }

    /// Every peer's readout `Key → value` map equals the canonical
    /// map built from the originating peers' `Key`s and the oracle's
    /// per-insert values, filtered by the oracle's redaction set.
    /// Pins down that every peer converges on exactly the same
    /// `Key`s for exactly the same values — no per-peer key drift.
    #[test]
    fn keys_stable_across_peers(
        schedule in schedule_u64(),
    ) {
        let result = execute_and_quiesce(&schedule);
        let expected: BTreeMap<Key, u64> = result
            .resolved_keys
            .iter()
            .filter(|(id, _)| !result.oracle.is_redacted(**id))
            .map(|(id, k)| (*k, result.oracle.all_inserts()[id]))
            .collect();

        for (i, peer) in result.peers.iter().enumerate() {
            let actual = readout(&peer.local);
            prop_assert_eq!(
                &actual, &expected,
                "peer {} readout key→value map does not match canonical", i,
            );
        }
    }

    /// No `Key` is observed (via `on_message`) more than once at any
    /// peer across the entire schedule. Re-gossip with an
    /// already-known message must not re-fire the callback.
    #[test]
    fn each_key_observed_at_most_once_per_peer(
        schedule in schedule_u64(),
    ) {
        let result = execute_and_quiesce(&schedule);
        for (i, peer) in result.peers.iter().enumerate() {
            let mut counts: BTreeMap<Key, usize> = BTreeMap::new();
            for (k, _, _) in &peer.observations {
                *counts.entry(*k).or_insert(0) += 1;
            }
            for (k, c) in &counts {
                prop_assert_eq!(
                    *c, 1,
                    "peer {} observed key {:?} {} times (must be at most once)",
                    i, k, c,
                );
            }
        }
    }

    /// Once peers have converged, an additional gossip event fires
    /// zero `on_message` callbacks and changes no peer's state.
    /// Picks two distinct peer indices via `prop_flat_map` on the
    /// schedule so the shrinker sees them as first-class inputs
    /// rather than modulo'd seeds.
    #[test]
    fn quiesced_state_is_gossip_fixed_point(
        (schedule, a, b) in schedule_u64().prop_flat_map(|s| {
            let n = s.n_peers;
            (Just(s), 0..n, 0..n)
        }).prop_filter("distinct peers", |(_, a, b)| a != b),
    ) {
        let mut result = execute_and_quiesce(&schedule);
        let before_a = result.peers[a].local.clone();
        let before_b = result.peers[b].local.clone();
        let obs_a_before = result.peers[a].observations.len();
        let obs_b_before = result.peers[b].observations.len();

        let (lo, hi) = if a < b { (a, b) } else { (b, a) };
        let (left, right) = result.peers.split_at_mut(hi);
        gossip_step(&mut left[lo], &mut right[0]);

        prop_assert_eq!(&result.peers[a].local, &before_a);
        prop_assert_eq!(&result.peers[b].local, &before_b);
        prop_assert_eq!(result.peers[a].observations.len(), obs_a_before);
        prop_assert_eq!(result.peers[b].observations.len(), obs_b_before);
    }

    /// String-T variant of `readout_matches_oracle_after_quiesce`,
    /// exercising the borsh round-trip for a non-primitive value
    /// type. Catches any serialization-path bug invisible to
    /// fixed-size scalars.
    #[test]
    fn readout_matches_oracle_after_quiesce_string(
        schedule in schedule_string(),
    ) {
        let result = execute_and_quiesce(&schedule);
        let expected = result.oracle.expected_live();
        for (i, peer) in result.peers.iter().enumerate() {
            let actual = readout_multiset(&peer.local);
            prop_assert_eq!(
                &actual, &expected,
                "peer {} (string-T) readout does not match oracle", i,
            );
        }
    }
}
