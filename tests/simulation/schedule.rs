//! Proptest-generated multi-peer schedules and the executor that runs
//! them against both the live simulation and the spec-shaped oracle.
//!
//! Every schedule emitted by [`arb_schedule`] is *valid by construction*:
//! `Redact` events always reference an `Insert` event that the redacting
//! peer has already observed by that point in the schedule. The
//! generator runs a small abstract simulator that mirrors the live
//! gossip protocol's deletion-honoring inference; a `RedactObservation`
//! choice that has no observed key to bind to is simply dropped during
//! the build, so the final event sequence contains no impossible
//! events the executor would have to filter.

use std::collections::{BTreeMap, BTreeSet};

use proptest::collection::vec;
use proptest::prelude::*;
use rumors::Key;

use crate::oracle::{InsertId, Oracle, PartyId};
use crate::peer::{Peer, gossip_step, quiesce};

#[derive(Debug, Clone)]
pub enum Event {
    Insert {
        peer: usize,
        value: u64,
    },
    /// Redact the `Key` minted by the `Insert` event at this index in
    /// the schedule's emitted event sequence. The strategy guarantees
    /// the redacting peer has observed that `Key` by the time this
    /// event runs.
    Redact {
        peer: usize,
        target_event_idx: usize,
    },
    Gossip {
        a: usize,
        b: usize,
    },
}

#[derive(Debug, Clone)]
pub struct Schedule {
    pub n_peers: usize,
    pub events: Vec<Event>,
}

/// Strategy: every emitted schedule has only causally-valid events.
pub fn arb_schedule(
    n_peers_range: std::ops::RangeInclusive<usize>,
    max_events: usize,
) -> impl Strategy<Value = Schedule> {
    n_peers_range.prop_flat_map(move |n_peers| {
        vec(arb_choice(n_peers), 0..=max_events)
            .prop_map(move |choices| build_schedule(n_peers, choices))
    })
}

/// Abstract action the strategy emits. Concrete `Event`s are derived
/// from these in [`build_schedule`] by consulting a per-peer
/// observation log that mirrors the protocol's effects.
#[derive(Debug, Clone)]
enum Choice {
    Insert {
        peer: usize,
        value: u64,
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

fn arb_choice(n_peers: usize) -> impl Strategy<Value = Choice> {
    prop_oneof![
        4 => (0..n_peers, any::<u64>())
            .prop_map(|(peer, value)| Choice::Insert { peer, value }),
        2 => (0..n_peers, any::<usize>())
            .prop_map(|(peer, idx)| Choice::RedactObservation { peer, idx }),
        3 => (0..n_peers, 0..n_peers)
            .prop_map(|(a, b)| Choice::Gossip { a, b }),
    ]
}

/// Per-peer state tracked during schedule generation, kept in lockstep
/// with what the live simulation would observe — including the
/// version-vector deletion-honoring inference. A peer's
/// `observed_log[p]` is exactly the sequence of insert event indices
/// that the live `Peer<T>` would have accumulated in its observation
/// vector by this point.
struct SimState {
    ever_known: Vec<BTreeSet<usize>>,
    live: Vec<BTreeSet<usize>>,
    observed_log: Vec<Vec<usize>>,
}

impl SimState {
    fn new(n_peers: usize) -> Self {
        Self {
            ever_known: vec![BTreeSet::new(); n_peers],
            live: vec![BTreeSet::new(); n_peers],
            observed_log: vec![Vec::new(); n_peers],
        }
    }

    fn record_insert(&mut self, peer: usize, event_idx: usize) {
        self.ever_known[peer].insert(event_idx);
        self.live[peer].insert(event_idx);
        self.observed_log[peer].push(event_idx);
    }

    fn record_redact(&mut self, peer: usize, target_event_idx: usize) {
        // Removing from live (peer's act of forgetting). `ever_known`
        // and `observed_log` are unchanged — the peer still remembers
        // that it once held this key.
        self.live[peer].remove(&target_event_idx);
    }

    fn gossip(&mut self, a: usize, b: usize) {
        if a == b {
            return;
        }
        // Transmissions: each side sends keys it holds live that the
        // other side has never observed. Receiving this Insert
        // populates the receiver's observation log.
        let a_to_b: Vec<usize> = self.live[a]
            .difference(&self.ever_known[b])
            .copied()
            .collect();
        let b_to_a: Vec<usize> = self.live[b]
            .difference(&self.ever_known[a])
            .copied()
            .collect();
        for k in &a_to_b {
            self.ever_known[b].insert(*k);
            self.live[b].insert(*k);
            self.observed_log[b].push(*k);
        }
        for k in &b_to_a {
            self.ever_known[a].insert(*k);
            self.live[a].insert(*k);
            self.observed_log[a].push(*k);
        }

        // Deletion-honoring: for any key in the intersection of both
        // peers' `ever_known`, if one side has it live and the other
        // has it redacted (in ever_known but not live), the redaction
        // propagates and the live side drops. No `on_message`
        // callback fires for this — observation logs are unchanged.
        let intersect: Vec<usize> = self.ever_known[a]
            .intersection(&self.ever_known[b])
            .copied()
            .collect();
        for k in intersect {
            let a_live = self.live[a].contains(&k);
            let b_live = self.live[b].contains(&k);
            if a_live && !b_live {
                self.live[a].remove(&k);
            } else if b_live && !a_live {
                self.live[b].remove(&k);
            }
        }
    }

    fn lookup_observation(&self, peer: usize, idx: usize) -> Option<usize> {
        let log = &self.observed_log[peer];
        if log.is_empty() {
            None
        } else {
            Some(log[idx % log.len()])
        }
    }
}

fn build_schedule(n_peers: usize, choices: Vec<Choice>) -> Schedule {
    let mut sim = SimState::new(n_peers);
    let mut events: Vec<Event> = Vec::new();
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
                // else: peer has not yet observed any key, so no
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
    Schedule { n_peers, events }
}

pub struct ExecutionResult {
    pub peers: Vec<Peer<u64>>,
    pub oracle: Oracle<u64>,
    /// Party identifiers in peer-index order. Kept for proptest
    /// counterexample legibility.
    #[allow(dead_code)]
    pub party_ids: Vec<PartyId>,
    /// For each `Insert` event, the `Key` minted at the originating peer.
    pub resolved_keys: BTreeMap<InsertId, Key>,
}

/// Run the schedule against a fresh `Vec<Peer<u64>>` and a fresh
/// `Oracle<u64>`. Schedules from [`arb_schedule`] are valid by
/// construction, so the executor performs no event-level filtering.
pub fn execute(schedule: &Schedule) -> ExecutionResult {
    let party_ids: Vec<PartyId> = (0..schedule.n_peers).map(|i| format!("p{i}")).collect();
    let mut peers: Vec<Peer<u64>> = party_ids.iter().map(Peer::<u64>::new).collect();
    let mut oracle = Oracle::<u64>::new();
    let mut resolved_keys: BTreeMap<InsertId, Key> = BTreeMap::new();

    for (i, event) in schedule.events.iter().enumerate() {
        match event {
            Event::Insert { peer, value } => {
                let key = peers[*peer].insert_one(*value);
                resolved_keys.insert(i, key);
                oracle.insert(i, party_ids[*peer].clone(), *value);
            }
            Event::Redact {
                peer,
                target_event_idx,
            } => {
                let key = resolved_keys[target_event_idx];
                peers[*peer].redact_one(key);
                oracle.redact(*target_event_idx);
            }
            Event::Gossip { a, b } => {
                let (lo, hi) = if a < b { (*a, *b) } else { (*b, *a) };
                let (left, right) = peers.split_at_mut(hi);
                gossip_step(&mut left[lo], &mut right[0]);
            }
        }
    }

    ExecutionResult {
        peers,
        oracle,
        party_ids,
        resolved_keys,
    }
}

/// Run the schedule and then drive every peer to a full-mesh fixed
/// point. After this returns, every `peers[i].local` should equal
/// every other and equal the oracle's projection (per invariant C2).
pub fn execute_and_quiesce(schedule: &Schedule) -> ExecutionResult {
    let mut result = execute(schedule);
    quiesce(&mut result.peers);
    result
}
