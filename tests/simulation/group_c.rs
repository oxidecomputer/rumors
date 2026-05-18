//! Group C — multi-peer eventual consistency.

use std::collections::BTreeMap;

use proptest::prelude::*;
use rumors::Key;

use crate::oracle::{readout, readout_multiset};
use crate::peer::gossip_step;
use crate::schedule::{arb_schedule, execute_and_quiesce};

const N_PEERS: std::ops::RangeInclusive<usize> = 2..=8;
const MAX_EVENTS: usize = 50;

proptest! {
    /// C1: after the final quiesce phase, every peer's live content
    /// (per readout) matches every other's. We compare readouts rather
    /// than `Local`s directly because `Local::eq` includes the party
    /// tag, which always differs across peers.
    #[test]
    fn c1_convergence_under_quiescence(
        schedule in arb_schedule(N_PEERS, MAX_EVENTS),
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

    /// C2: after quiesce, every peer's readout multiset equals the
    /// oracle's `expected_live()`. The oracle is pure data and never
    /// invokes `process`, so this is a genuinely independent check.
    #[test]
    fn c2_simulation_matches_oracle(
        schedule in arb_schedule(N_PEERS, MAX_EVENTS),
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

    /// C3: post-quiesce, every peer's readout map (Key → value) is
    /// identical to the canonical map built from the originating
    /// peers' `Key`s and the oracle's per-insert values, filtered by
    /// the oracle's redaction set. This pins down that every peer
    /// converges on exactly the same `Key`s for exactly the same
    /// values — no per-peer key drift.
    #[test]
    fn c3_key_stability_across_peers(
        schedule in arb_schedule(N_PEERS, MAX_EVENTS),
    ) {
        let result = execute_and_quiesce(&schedule);
        let expected: BTreeMap<Key, u64> = result
            .resolved_keys
            .iter()
            .filter(|(id, _)| !result.oracle.is_redacted(**id))
            .map(|(id, k)| (*k, result.oracle.all_inserts()[id].2))
            .collect();

        for (i, peer) in result.peers.iter().enumerate() {
            let actual = readout(&peer.local);
            prop_assert_eq!(
                &actual, &expected,
                "peer {} readout key→value map does not match canonical", i,
            );
        }
    }

    /// C4: at every peer, no `Key` is observed (via `on_message`) more
    /// than once across the entire schedule. Re-gossip with an
    /// already-known message must not re-fire the callback.
    #[test]
    fn c4_observation_uniqueness(
        schedule in arb_schedule(N_PEERS, MAX_EVENTS),
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

    /// C5: once peers have converged, an additional gossip event fires
    /// zero `on_message` callbacks and changes no peer's state.
    #[test]
    fn c5_quiescent_fixed_point(
        schedule in arb_schedule(N_PEERS, MAX_EVENTS),
        i in 0usize..8,
        j in 0usize..8,
    ) {
        let mut result = execute_and_quiesce(&schedule);
        let n = result.peers.len();
        let a = i % n;
        let b = j % n;
        if a == b {
            return Ok(());
        }
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
}
