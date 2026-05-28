//! The proptest strategy for `Schedule<T>` and its shadow simulator.
//!
//! Every schedule emitted by [`arb_schedule`] is *valid by
//! construction*: a `Redact` event always references an `Insert`
//! whose `Key` the redacting peer has already observed by that point.
//! To enforce this, the generator drives a [`SimState`] in lockstep
//! with the choices it emits — a shadow simulator that mirrors what
//! each `Peer<T>` would observe under the protocol (including the
//! deletion-honoring propagation of redactions during gossip). A
//! `RedactObservation` choice whose peer has nothing observable is
//! simply dropped, so the executor never has to filter "impossible"
//! events at runtime.

use std::collections::BTreeSet;
use std::fmt::Debug;
use std::ops::RangeInclusive;

use proptest::collection::vec;
use proptest::prelude::*;

use super::events::{Event, EventIdx, Schedule};

/// Strategy: every emitted schedule has only causally-valid events.
///
/// `value_strategy` supplies the value type carried by each `Insert`
/// event; pass `any::<u64>()` for the default suite or e.g.
/// `"[a-z]{0,8}".prop_map(String::from)` for a string-valued variant.
pub fn arb_schedule<T, S>(
    value_strategy: S,
    n_peers_range: RangeInclusive<usize>,
    max_events: usize,
) -> impl Strategy<Value = Schedule<T>>
where
    T: Clone + Debug + 'static,
    S: Strategy<Value = T> + Clone + 'static,
{
    arb_schedule_with_shadow(value_strategy, n_peers_range, max_events)
        .prop_map(|(schedule, _shadow)| schedule)
}

/// Final state of the shadow simulator after a schedule has been
/// built, surfaced by [`arb_schedule_with_shadow`] for use by the
/// shadow-validity meta-test.
#[derive(Debug, Clone)]
pub struct ShadowFinal {
    /// Per-peer sequence of `EventIdx`s the shadow predicts the live
    /// `Peer<T>` would have appended to its observation vector.
    pub observed_log: Vec<Vec<EventIdx>>,
    /// Per-peer set of `EventIdx`s the shadow predicts the live peer
    /// still has *live* in its rumor set at the end of the schedule.
    pub live: Vec<BTreeSet<EventIdx>>,
}

/// Variant of [`arb_schedule`] that also yields the shadow simulator's
/// final state. Used by the shadow-validity meta-test to confirm the
/// generator's model agrees with what the real executor produces.
pub fn arb_schedule_with_shadow<T, S>(
    value_strategy: S,
    n_peers_range: RangeInclusive<usize>,
    max_events: usize,
) -> impl Strategy<Value = (Schedule<T>, ShadowFinal)>
where
    T: Clone + Debug + 'static,
    S: Strategy<Value = T> + Clone + 'static,
{
    n_peers_range.prop_flat_map(move |n_peers| {
        vec(arb_choice(value_strategy.clone(), n_peers), 0..=max_events)
            .prop_map(move |choices| build_schedule(n_peers, choices))
    })
}

/// Abstract action the strategy emits. Concrete `Event`s are derived
/// from these in [`build_schedule`] by consulting a per-peer
/// observation log that mirrors the protocol's effects.
#[derive(Debug, Clone)]
enum Choice<T> {
    Insert {
        peer: usize,
        value: T,
    },
    /// Pick the `idx % len`-th key in the redacting peer's current
    /// observation log; if the log is empty, the choice is dropped.
    RedactObservation {
        peer: usize,
        idx: usize,
    },
    Gossip {
        a: usize,
        b: usize,
    },
}

fn arb_choice<T, S>(value_strategy: S, n_peers: usize) -> impl Strategy<Value = Choice<T>>
where
    T: Clone + Debug + 'static,
    S: Strategy<Value = T> + Clone + 'static,
{
    prop_oneof![
        4 => (0..n_peers, value_strategy)
            .prop_map(|(peer, value)| Choice::Insert { peer, value }),
        2 => (0..n_peers, any::<usize>())
            .prop_map(|(peer, idx)| Choice::RedactObservation { peer, idx }),
        3 => (0..n_peers, 0..n_peers)
            .prop_map(|(a, b)| Choice::Gossip { a, b }),
    ]
}

/// Shadow simulator: per-peer state kept in lockstep with what the
/// live simulation would observe under the actual protocol. For peer
/// `p`:
///
/// * `ever_known[p]` is every `EventIdx` whose `Key` `p` has ever
///   held (whether it currently holds it or has since redacted it).
/// * `live[p]` is the subset currently in `p`'s live rumor set.
/// * `observed_log[p]` is the exact sequence of `EventIdx`s that the
///   live `Peer<T>` would have appended to its observation vector by
///   this point — driven by both local inserts and gossip events.
///
/// `RedactObservation` picks an entry from `observed_log` to redact,
/// so the schedule is guaranteed to issue every `Redact` on a `Key`
/// the peer actually holds at that moment.
struct SimState {
    ever_known: Vec<BTreeSet<EventIdx>>,
    live: Vec<BTreeSet<EventIdx>>,
    observed_log: Vec<Vec<EventIdx>>,
}

impl SimState {
    fn new(n_peers: usize) -> Self {
        Self {
            ever_known: vec![BTreeSet::new(); n_peers],
            live: vec![BTreeSet::new(); n_peers],
            observed_log: vec![Vec::new(); n_peers],
        }
    }

    fn record_insert(&mut self, peer: usize, event_idx: EventIdx) {
        self.ever_known[peer].insert(event_idx);
        self.live[peer].insert(event_idx);
        self.observed_log[peer].push(event_idx);
    }

    fn record_redact(&mut self, peer: usize, target_event_idx: EventIdx) {
        // Removing from live (the peer's act of forgetting).
        // `ever_known` and `observed_log` are unchanged: the peer
        // still remembers that it once held this key.
        self.live[peer].remove(&target_event_idx);
    }

    fn gossip(&mut self, a: usize, b: usize) {
        if a == b {
            return;
        }
        // The hash-tree mirror exchanges *every* tree position that
        // differs, which means the receiver learns about both live
        // values (with an `on_message` callback) and redaction
        // tombstones (silently, no callback). Walking the union of
        // each side's `ever_known` lets us model both in one pass.
        //
        // For each key any side has ever held:
        //
        //  - If either side has it redacted (in `ever_known` but
        //    not in `live`), the redaction is contagious: both
        //    sides end up with the key in `ever_known` and out of
        //    `live`. No observation fires.
        //  - Otherwise (no side has redacted yet), the value
        //    propagates to whichever side hasn't seen it; that side
        //    appends the `EventIdx` to its `observed_log`.
        let combined: BTreeSet<EventIdx> = self.ever_known[a]
            .union(&self.ever_known[b])
            .copied()
            .collect();
        for k in combined {
            let a_known = self.ever_known[a].contains(&k);
            let b_known = self.ever_known[b].contains(&k);
            let a_live = self.live[a].contains(&k);
            let b_live = self.live[b].contains(&k);
            let any_redacted = (a_known && !a_live) || (b_known && !b_live);

            if any_redacted {
                if !a_known {
                    self.ever_known[a].insert(k);
                }
                if !b_known {
                    self.ever_known[b].insert(k);
                }
                self.live[a].remove(&k);
                self.live[b].remove(&k);
            } else {
                if !a_known {
                    self.ever_known[a].insert(k);
                    self.live[a].insert(k);
                    self.observed_log[a].push(k);
                }
                if !b_known {
                    self.ever_known[b].insert(k);
                    self.live[b].insert(k);
                    self.observed_log[b].push(k);
                }
            }
        }
    }

    fn lookup_observation(&self, peer: usize, idx: usize) -> Option<EventIdx> {
        let log = &self.observed_log[peer];
        if log.is_empty() {
            None
        } else {
            Some(log[idx % log.len()])
        }
    }
}

fn build_schedule<T>(n_peers: usize, choices: Vec<Choice<T>>) -> (Schedule<T>, ShadowFinal) {
    let mut sim = SimState::new(n_peers);
    let mut events: Vec<Event<T>> = Vec::new();
    for choice in choices {
        let next_event_idx = events.len();
        match choice {
            Choice::Insert { peer, value } => {
                sim.record_insert(peer, next_event_idx);
                events.push(Event::Insert { peer, value });
            }
            Choice::RedactObservation { peer, idx } => {
                if let Some(target_event_idx) = sim.lookup_observation(peer, idx) {
                    sim.record_redact(peer, target_event_idx);
                    events.push(Event::Redact {
                        peer,
                        target_event_idx,
                    });
                }
                // else: the peer has not yet observed any key, so no
                // application code path could have produced this
                // `redact()` call. Drop the choice.
            }
            Choice::Gossip { a, b } => {
                if a == b {
                    continue;
                }
                sim.gossip(a, b);
                events.push(Event::Gossip { a, b });
            }
        }
    }
    let SimState {
        observed_log, live, ..
    } = sim;
    (
        Schedule { n_peers, events },
        ShadowFinal { observed_log, live },
    )
}
