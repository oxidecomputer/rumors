//! Meta-test: the shadow simulator inside `schedule::arb` agrees
//! with the live executor.
//!
//! The schedule generator is *valid by construction* — every `Redact`
//! references an `Insert` the redacting peer has already observed.
//! That guarantee rests on a shadow simulator (`SimState`) that the
//! generator drives in lockstep with the choices it emits. If that
//! shadow disagrees with the real protocol, the generator silently
//! emits wrong events and every multi-peer property in the suite
//! loses its grounding.
//!
//! The invariants checked against the live executor, for the
//! membership-free alphabet and the membership alphabet alike:
//!
//! * `observed_log` — the set of `EventIdx`s the shadow predicts
//!   each peer's `on_message` callback would have fired for must
//!   match the set the live executor actually fired (for a retired
//!   peer, its complete lifetime log).
//! * `live` — the set of `EventIdx`s the shadow predicts each peer
//!   still holds at the end of the schedule must match the live
//!   peer's readout (translated through `resolved_keys`).
//! * `alive` — under membership events, exactly the slots the shadow
//!   predicts alive must have survived.
//!
//! Comparison is set-wise: callback order within a batch is
//! unspecified, so a sequence-wise comparison would over-constrain.

mod common;

use std::collections::{BTreeMap, BTreeSet};

use proptest::prelude::*;
use rumors::Key;

use crate::common::oracle::readout;
use crate::common::schedule::{
    EventIdx, arb_membership_schedule_with_shadow, arb_schedule_with_shadow, execute_membership,
    execute_with,
};
use crate::common::window::arb_window_assignment;

const N_PEERS: std::ops::RangeInclusive<usize> = 2..=8;
const MAX_EVENTS: usize = 50;

proptest! {
    /// For every peer, the shadow simulator's `observed_log` and
    /// `live` sets (as `BTreeSet<EventIdx>`) match the live
    /// executor's observations and current readout, translated
    /// through `resolved_keys` back to event indices.
    #[test]
    fn shadow_predicts_live_state(
        (schedule, shadow) in arb_schedule_with_shadow(any::<u64>(), N_PEERS, MAX_EVENTS),
        windows in arb_window_assignment(),
    ) {
        let result = execute_with(&schedule, &windows, |_, _, _| true);
        let key_to_event_idx: BTreeMap<Key, EventIdx> =
            result.resolved_keys.iter().map(|(eid, k)| (*k, *eid)).collect();

        for (p, peer) in result.peers.iter().enumerate() {
            let live_observed: BTreeSet<EventIdx> = peer
                .observations
                .iter()
                .map(|(k, _, _)| key_to_event_idx[k])
                .collect();
            let predicted_observed: BTreeSet<EventIdx> =
                shadow.observed_log[p].iter().copied().collect();
            prop_assert_eq!(
                live_observed, predicted_observed,
                "peer {} observation set disagrees with shadow", p,
            );

            let live_held: BTreeSet<EventIdx> = readout(&peer.local.snapshot())
                .into_keys()
                .map(|k| key_to_event_idx[&k])
                .collect();
            prop_assert_eq!(
                live_held, shadow.live[p].clone(),
                "peer {} live set disagrees with shadow", p,
            );
        }
    }

    /// The membership-alphabet twin of `shadow_predicts_live_state`.
    ///
    /// Under mid-schedule bootstraps and retirements, the shadow's
    /// `alive` map matches which slots survived, every live peer's
    /// observation set and readout match the shadow's prediction, and
    /// every retired peer's complete lifetime observation log matches
    /// what the shadow predicted it observed before absorption. If the
    /// shadow's membership model drifted from the real bootstrap copy or
    /// retirement absorption, the generator's validity guarantee (and
    /// every membership property built on it) would silently rot.
    #[test]
    fn shadow_predicts_membership_state(
        (schedule, shadow) in arb_membership_schedule_with_shadow(any::<u64>(), N_PEERS, MAX_EVENTS),
        windows in arb_window_assignment(),
    ) {
        let result = execute_membership(&schedule, &windows);
        let key_to_event_idx: BTreeMap<Key, EventIdx> =
            result.resolved_keys.iter().map(|(eid, k)| (*k, *eid)).collect();

        prop_assert_eq!(
            result.slots.len(), shadow.alive.len(),
            "fleet size disagrees with shadow",
        );
        for (p, slot) in result.slots.iter().enumerate() {
            prop_assert_eq!(
                slot.is_some(), shadow.alive[p],
                "peer {} aliveness disagrees with shadow", p,
            );
            let observations: Vec<Key> = match slot {
                Some(peer) => peer.observations.iter().map(|(k, _, _)| *k).collect(),
                None => result.retired_observations[&p]
                    .iter()
                    .map(|(k, _, _)| *k)
                    .collect(),
            };
            let live_observed: BTreeSet<EventIdx> = observations
                .iter()
                .map(|k| key_to_event_idx[k])
                .collect();
            let predicted_observed: BTreeSet<EventIdx> =
                shadow.observed_log[p].iter().copied().collect();
            prop_assert_eq!(
                live_observed, predicted_observed,
                "peer {} observation set disagrees with shadow", p,
            );

            if let Some(peer) = slot {
                let live_held: BTreeSet<EventIdx> = readout(&peer.local.snapshot())
                    .into_keys()
                    .map(|k| key_to_event_idx[&k])
                    .collect();
                prop_assert_eq!(
                    live_held, shadow.live[p].clone(),
                    "peer {} live set disagrees with shadow", p,
                );
            }
        }
    }
}
