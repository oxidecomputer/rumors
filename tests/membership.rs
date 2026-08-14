//! Membership churn under the oracle-checked sequential engine.
//!
//! Mid-schedule bootstraps and retirements interleave with inserts,
//! redactions, and gossip, every run checked against the spec-shaped
//! oracle and the global party invariants.
//!
//! The engine runs one session at a time over clean wires, so every
//! retirement commits and content is conserved exactly: unlike the
//! chaos engine's loss accounting, the oracle equality here is
//! unconditional.

mod common;

use std::collections::BTreeMap;

use before::Party;
use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::TestRunner;
use rumors::Key;

use crate::common::oracle::{readout, readout_multiset};
use crate::common::schedule::events::Event;
use crate::common::schedule::{arb_membership_schedule, execute_membership_and_quiesce};
use crate::common::window::arb_window_assignment;

const N_PEERS: std::ops::RangeInclusive<usize> = 2..=6;
const MAX_EVENTS: usize = 40;

proptest! {
    /// Under membership churn, the surviving fleet still converges to
    /// exactly the oracle's projection, honoring every redaction:
    ///
    /// 1. every live peer's readout multiset equals the oracle's
    ///    `expected_live()` — content crosses retirements and
    ///    bootstraps without loss or invention;
    /// 2. no redacted event's key is live anywhere — deletion honoring
    ///    survives absorption and newcomer copies;
    /// 3. every live peer agrees on the full `Key → value` map;
    /// 4. the live parties fold-join back to exactly `Party::seed()` —
    ///    retirement moves id-regions without duplication or leak
    ///    (clean wires: no hand-off can be lost).
    #[test]
    fn membership_churn_converges_to_oracle(
        schedule in arb_membership_schedule(any::<u64>(), N_PEERS, MAX_EVENTS),
        windows in arb_window_assignment(),
    ) {
        let result = execute_membership_and_quiesce(&schedule, &windows);
        let expected = result.oracle.expected_live();
        let canonical: BTreeMap<Key, u64> = result
            .resolved_keys
            .iter()
            .filter(|(id, _)| !result.oracle.is_redacted(**id))
            .map(|(id, k)| (*k, result.oracle.all_inserts()[id]))
            .collect();

        let mut parties: Vec<Party> = Vec::new();
        for (i, peer) in result.live() {
            let actual = readout(&peer.local.snapshot());
            prop_assert_eq!(
                readout_multiset(&peer.local.snapshot()), expected.clone(),
                "live peer {} diverged from the oracle multiset", i,
            );
            for (id, key) in &result.resolved_keys {
                if result.oracle.is_redacted(*id) {
                    prop_assert!(
                        !actual.contains_key(key),
                        "redacted key {:?} (event {}) is live at peer {}",
                        key, id, i,
                    );
                }
            }
            prop_assert_eq!(
                &actual, &canonical,
                "live peer {} readout key→value map does not match canonical", i,
            );
            parties.push(
                peer.local
                    .dangerously_alias_party()
                    .expect("a live peer holds its party"),
            );
        }

        let mut parties = parties.into_iter();
        let mut whole = parties.next().expect("at least one peer survives");
        for party in parties {
            whole.join(party).expect("live parties are pairwise disjoint");
        }
        prop_assert_eq!(
            whole,
            Party::seed(),
            "the live parties must reconstitute the seed's whole id-space",
        );
    }
}

/// The membership alphabet is live in the generated population: sampled
/// under proptest's deterministic runner, the strategy emits schedules
/// containing both mid-schedule bootstraps and retirements.
///
/// Either could silently vanish — a weight edit, or a validity rule
/// that drops every choice — leaving the suite green while testing no
/// membership at all.
#[test]
fn membership_population_contains_churn() {
    let mut runner = TestRunner::deterministic();
    let strategy = arb_membership_schedule(any::<u64>(), N_PEERS, MAX_EVENTS);
    let mut bootstraps = 0usize;
    let mut retires = 0usize;
    for _ in 0..64 {
        let schedule = strategy
            .new_tree(&mut runner)
            .expect("membership strategy always generates")
            .current();
        for event in &schedule.events {
            match event {
                Event::Bootstrap { .. } => bootstraps += 1,
                Event::Retire { .. } => retires += 1,
                _ => {}
            }
        }
    }
    assert!(
        bootstraps > 0,
        "no sampled schedule bootstraps a newcomer: the membership \
         dimension has silently left the population"
    );
    assert!(
        retires > 0,
        "no sampled schedule retires a peer: the membership dimension \
         has silently left the population"
    );
}
