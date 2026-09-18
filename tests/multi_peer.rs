//! Multi-peer eventual consistency under randomised schedules.
//!
//! Each property runs a generated [`Schedule`] through the executor and a
//! full-mesh quiesce. Assertions over the same result share one property;
//! independent behaviors keep separate properties. The exact-oracle property
//! uses four default case budgets for a broad schedule population, while the
//! observation and fixed-point properties sample independently.
//!
//! [`Schedule`]: crate::common::schedule::Schedule

use rumors_testkit::common;

use std::collections::{BTreeMap, BTreeSet};

use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::TestRunner;

use crate::common::oracle::{readout, readout_multiset, version_key};
use crate::common::peer::gossip_step;
use crate::common::schedule::events::Event;
use crate::common::schedule::executor::ExecutionResult;
use crate::common::schedule::{Schedule, arb_schedule, execute_and_quiesce};
use crate::common::window::{WindowAssignment, arb_window_assignment};

/// Fleet sizes exercised by generated schedules.
const N_PEERS: std::ops::RangeInclusive<usize> = 2..=8;
/// Maximum events in one generated schedule.
const MAX_EVENTS: usize = 50;

/// Generate schedules with fixed-size scalar payloads.
fn schedule_u64() -> impl Strategy<Value = Schedule<u64>> {
    arb_schedule(any::<u64>(), N_PEERS, MAX_EVENTS)
}

/// Generate schedules with variable-size string payloads.
fn schedule_string() -> impl Strategy<Value = Schedule<String>> {
    arb_schedule("[a-z]{0,8}".prop_map(String::from), N_PEERS, MAX_EVENTS)
}

/// Build the exact live identity-to-value map predicted by the schedule oracle.
fn canonical_readout<T>(result: &ExecutionResult<T>) -> BTreeMap<Vec<u8>, T>
where
    T: Clone + Ord + Send + Sync + 'static,
{
    result
        .resolved_versions
        .iter()
        .filter(|(id, _)| !result.oracle.is_redacted(**id))
        .map(|(id, version)| {
            (
                version_key(version),
                result.oracle.all_inserts()[id].clone(),
            )
        })
        .collect()
}

/// Report whether every insert received a distinct version.
fn resolved_versions_are_unique<T>(result: &ExecutionResult<T>) -> bool
where
    T: Send + Sync + 'static,
{
    result
        .resolved_versions
        .values()
        .map(version_key)
        .collect::<BTreeSet<_>>()
        .len()
        == result.resolved_versions.len()
}

proptest! {
    #![proptest_config(ProptestConfig {
        // Give the shared exact-oracle assertions four default case budgets.
        cases: ProptestConfig::default().cases * 4,
        ..ProptestConfig::default()
    })]

    /// Every peer's readout identity → value map equals the canonical
    /// map built from the originating peers' created `Version`s and the
    /// oracle's per-insert values, filtered by the oracle's redaction set.
    ///
    /// The property checks global version uniqueness, exact identity-to-value
    /// agreement, and the live-value multiset separately. This prevents an
    /// identity collision from being hidden when the exact map is built.
    #[test]
    fn versions_stable_across_peers(
        schedule in schedule_u64(),
        windows in arb_window_assignment(),
    ) {
        let result = execute_and_quiesce(&schedule, &windows);
        let expected = canonical_readout(&result);
        let expected_values = result.oracle.expected_live();
        prop_assert!(
            resolved_versions_are_unique(&result),
            "distinct inserts received the same Version",
        );

        for (i, peer) in result.peers.iter().enumerate() {
            let actual = readout(&peer.local.snapshot());
            prop_assert_eq!(
                &actual, &expected,
                "peer {} readout identity→value map does not match canonical", i,
            );
            prop_assert_eq!(
                readout_multiset(&peer.local.snapshot()),
                expected_values.clone(),
                "peer {} live-value multiset does not match the oracle", i,
            );
        }
    }
}

proptest! {
    /// No message is observed more than once at any peer across the
    /// entire schedule: re-gossip with an already-known message must
    /// not re-surface it in the observation log.
    #[test]
    fn each_message_observed_at_most_once_per_peer(
        schedule in schedule_u64(),
        windows in arb_window_assignment(),
    ) {
        let result = execute_and_quiesce(&schedule, &windows);
        for (i, peer) in result.peers.iter().enumerate() {
            let mut counts: BTreeMap<Vec<u8>, usize> = BTreeMap::new();
            for (v, _) in peer.observations.iter() {
                *counts.entry(version_key(v)).or_insert(0) += 1;
            }
            for (k, c) in &counts {
                prop_assert_eq!(
                    *c, 1,
                    "peer {} observed version {:?} {} times (must be at most once)",
                    i, k, c,
                );
            }
        }
    }

    /// Once peers have converged, an additional gossip event yields
    /// zero new observations and changes no peer's state (live
    /// content and causal version alike).
    ///
    /// A coincident pair advances the second peer by one. Distinct draws stay
    /// unchanged, preserving saved cases while making every result distinct.
    #[test]
    fn quiesced_state_is_gossip_fixed_point(
        (schedule, a, b) in schedule_u64().prop_flat_map(|s| {
            let n = s.n_peers;
            (Just(s), 0..n, 0..n)
        }).prop_map(|(s, a, b)| {
            let b = if a == b { (b + 1) % s.n_peers } else { b };
            (s, a, b)
        }),
        windows in arb_window_assignment(),
    ) {
        prop_assert_ne!(a, b);
        let mut result = execute_and_quiesce(&schedule, &windows);
        let fingerprint = |peer: &crate::common::peer::Peer<u64>| {
            let snapshot = peer.local.snapshot();
            (snapshot.hash(), snapshot.latest().clone())
        };
        let before_a = fingerprint(&result.peers[a]);
        let before_b = fingerprint(&result.peers[b]);
        let obs_a_before = result.peers[a].observations.len();
        let obs_b_before = result.peers[b].observations.len();

        let (lo, hi) = if a < b { (a, b) } else { (b, a) };
        let (left, right) = result.peers.split_at_mut(hi);
        gossip_step(&mut left[lo], &mut right[0]);

        prop_assert_eq!(fingerprint(&result.peers[a]), before_a);
        prop_assert_eq!(fingerprint(&result.peers[b]), before_b);
        prop_assert_eq!(result.peers[a].observations.len(), obs_a_before);
        prop_assert_eq!(result.peers[b].observations.len(), obs_b_before);
    }

    /// The floor-everywhere baseline leg of the oracle check: every
    /// window pinned at the serialization floor on every iteration.
    ///
    /// The capacity-one orderings the deadlock-freedom argument
    /// certifies stay deterministically exercised in this engine
    /// regardless of what the swept legs draw.
    #[test]
    fn readout_matches_oracle_after_quiesce_at_floor(
        schedule in schedule_u64(),
    ) {
        let result = execute_and_quiesce(&schedule, &WindowAssignment::floor());
        let expected = canonical_readout(&result);
        let expected_values = result.oracle.expected_live();
        prop_assert!(
            resolved_versions_are_unique(&result),
            "distinct inserts received the same Version at the window floor",
        );
        for (i, peer) in result.peers.iter().enumerate() {
            let actual = readout(&peer.local.snapshot());
            prop_assert_eq!(
                &actual, &expected,
                "peer {} exact readout does not match the oracle at the floor", i,
            );
            prop_assert_eq!(
                readout_multiset(&peer.local.snapshot()),
                expected_values.clone(),
                "peer {} live-value multiset does not match the oracle at the floor", i,
            );
        }
    }

    /// The exact oracle property for `String`, exercising the wire round-trip
    /// for variable-size values and catching payload-path defects that a
    /// fixed-size scalar may not expose.
    #[test]
    fn readout_matches_oracle_after_quiesce_string(
        schedule in schedule_string(),
        windows in arb_window_assignment(),
    ) {
        let result = execute_and_quiesce(&schedule, &windows);
        let expected = canonical_readout(&result);
        let expected_values = result.oracle.expected_live();
        prop_assert!(
            resolved_versions_are_unique(&result),
            "distinct String inserts received the same Version",
        );
        for (i, peer) in result.peers.iter().enumerate() {
            let actual = readout(&peer.local.snapshot());
            prop_assert_eq!(
                &actual, &expected,
                "peer {} String readout does not match the exact oracle", i,
            );
            prop_assert_eq!(
                readout_multiset(&peer.local.snapshot()),
                expected_values.clone(),
                "peer {} String value multiset does not match the oracle", i,
            );
        }
    }
}

/// Among 64 deterministic samples, `arb_schedule` emits a `Redact` event, so
/// the generator's redaction branch is live in the population.
#[test]
fn schedule_population_contains_redactions() {
    let mut runner = TestRunner::deterministic();
    let strategy = schedule_u64();
    let mut redactions = 0usize;
    for _ in 0..64 {
        let schedule = strategy
            .new_tree(&mut runner)
            .expect("schedule strategy always generates")
            .current();
        redactions += schedule
            .events
            .iter()
            .filter(|event| matches!(event, Event::Redact { .. }))
            .count();
    }
    assert!(
        redactions > 0,
        "no sampled schedule redacts a message: the redaction dimension \
         has silently left the population"
    );
}

/// Check convergence after isolated peer groups reconnect.
#[path = "multi_peer/partition.rs"]
mod partition;
