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
//! Two invariants are checked against the live executor:
//!
//! * `observed_log` — the set of `EventIdx`s the shadow predicts
//!   each peer's `on_message` callback would have fired for must
//!   match the set the live executor actually fired.
//! * `live` — the set of `EventIdx`s the shadow predicts each peer
//!   still holds at the end of the schedule must match the live
//!   peer's readout (translated through `resolved_keys`).
//!
//! Comparison is set-wise: callback order within a batch is
//! unspecified, so a sequence-wise comparison would over-constrain.

use std::collections::{BTreeMap, BTreeSet};

use proptest::prelude::*;
use rumors::Key;

use crate::oracle::readout;
use crate::schedule::{EventIdx, arb_schedule_with_shadow, execute_with};

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
    ) {
        let result = execute_with(&schedule, |_, _, _| true);
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

            let live_held: BTreeSet<EventIdx> = readout(&peer.local)
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
