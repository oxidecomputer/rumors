//! Concurrent gossip must preserve sends and honor redactions.
//!
//! A session reconciles snapshots taken after its preamble exchange, then
//! publishes its result alongside any changes made while it ran. The focused
//! sweep parks either of two sessions at every polling round; generated
//! schedules mix overlapping sessions with local sends and redactions.

mod common;

use std::collections::BTreeMap;

use proptest::prelude::*;
use rumors::{Peer, Rumors};

use crate::common::oracle::{readout, readout_multiset};
use crate::common::overlap::{self, arb_overlap_schedule, execute_overlap_and_quiesce};
use crate::common::wire::{bootstrap_fork, wire_gossip};

/// Build three replicas at the minimum window, then redact `target` only at B.
fn redacted_trio(n: u64, target: u64) -> (Rumors<u64>, Rumors<u64>, Rumors<u64>) {
    let a = Peer::seed().sync_window_floor().into_rumors();
    a.send_all(0..n).unwrap();
    let b = bootstrap_fork(&a);
    let c = bootstrap_fork(&a);
    // Resolve the target in this network; versions from another seeded
    // fixture need not identify its messages, even if their values match.
    let version = b
        .snapshot()
        .iter()
        .find(|(_, value)| **value == target)
        .unwrap()
        .0
        .clone();
    b.redact(&version);
    (a, b, c)
}

/// Drive the three peers to agreement and return the common readout.
///
/// Panics if they fail to agree within a bounded number of full-mesh
/// rounds: overlapped sessions must still converge.
fn converge(a: &Rumors<u64>, b: &Rumors<u64>, c: &Rumors<u64>) -> BTreeMap<Vec<u8>, u64> {
    /// Maximum full-mesh passes allowed to settle the already completed sessions.
    const ROUNDS: usize = 8;
    for _ in 0..ROUNDS {
        wire_gossip(a, b);
        wire_gossip(a, c);
        wire_gossip(b, c);
        let (ra, rb, rc) = (
            readout(&a.snapshot()),
            readout(&b.snapshot()),
            readout(&c.snapshot()),
        );
        if ra == rb && rb == rc {
            return ra;
        }
    }
    panic!("three peers failed to agree within {ROUNDS} full-mesh rounds");
}

/// Either ordering of overlapping sessions preserves every message except the redaction.
///
/// B redacts a message shared with A and C. Park A's session with either peer
/// while its session with the other runs to completion. Check A immediately
/// after both sessions, then check the settled fleet, at every parking round.
#[test]
fn overlapped_install_never_loses_innocent_messages() {
    /// Enough distinct messages to span multiple radix-fan chunks.
    const MESSAGES: u64 = 25;

    for target in 0..MESSAGES {
        for redacting_first in [false, true] {
            // The redacting session does more work than the converged one.
            // Calibrate each direction and target, including the completing
            // round, so the sweep covers parking before, during, and after it.
            let rounds = {
                let (a, b, c) = redacted_trio(MESSAGES, target);
                let first = if redacting_first { &b } else { &c };
                overlap::open(&a, first).finish()
            };
            for park in 0..=rounds {
                let (a, b, c) = redacted_trio(MESSAGES, target);
                // Derive the result from the untouched replica, independently
                // of either redaction or reconciliation. Fixture values are unique.
                let mut expected = readout(&a.snapshot());
                expected.retain(|_, value| *value != target);
                let (first, second) = if redacting_first { (&b, &c) } else { (&c, &b) };
                let mut session = overlap::open(&a, first);
                session.step(park);
                wire_gossip(&a, second);
                session.finish();

                // Later gossip could repair a temporary resurrection at A.
                // A has completed both exchanges, so it already owes the full
                // result; C may still need to learn the redaction afterward.
                assert_eq!(
                    readout(&a.snapshot()),
                    expected,
                    "shared replica: target={target}, redacting_first={redacting_first}, park={park}"
                );
                assert_eq!(
                    converge(&a, &b, &c),
                    expected,
                    "settled fleet: target={target}, redacting_first={redacting_first}, park={park}"
                );
            }
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 48,
        ..ProptestConfig::default()
    })]

    /// Generated overlaps converge to exactly the sends not redacted by the schedule.
    ///
    /// Schedules include two sessions sharing a replica in either ordering,
    /// alongside arbitrary sends, redactions, and other session interleavings.
    /// Compare both values and versions so equal payloads cannot hide a loss.
    #[test]
    fn overlapping_schedules_converge_to_the_oracle(
        schedule in arb_overlap_schedule(any::<u64>(), 2..=4, 24),
    ) {
        let (peers, oracle) = execute_overlap_and_quiesce(&schedule);
        let expected = oracle.expected_live();
        let readouts: Vec<_> = peers
            .iter()
            .map(|p| p.local.snapshot())
            .collect();
        for (i, snapshot) in readouts.iter().enumerate() {
            prop_assert_eq!(
                readout_multiset(snapshot),
                expected.clone(),
                "peer {} diverged from the oracle after quiescence",
                i,
            );
        }
        // Equal payloads are distinct messages when their versions differ.
        for pair in readouts.windows(2) {
            prop_assert_eq!(readout(&pair[0]), readout(&pair[1]));
        }
    }
}
