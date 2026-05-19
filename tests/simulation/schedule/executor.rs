//! Run a [`Schedule<T>`] against a fresh fleet of peers and a
//! spec-shaped oracle.
//!
//! The single primitive is [`execute_with`], which accepts a gossip
//! filter; [`execute`] and [`execute_and_quiesce`] are thin
//! convenience wrappers that allow every gossip event.

use std::collections::BTreeMap;

use borsh::{BorshDeserialize, BorshSerialize};
use rumors::Key;

use crate::oracle::Oracle;
use crate::peer::{Peer, gossip_step, quiesce};
use crate::schedule::events::{Event, EventIdx, Schedule};

pub struct ExecutionResult<T> {
    pub peers: Vec<Peer<T>>,
    pub oracle: Oracle<T>,
    /// For each `Insert` event, the `Key` minted at the originating peer.
    pub resolved_keys: BTreeMap<EventIdx, Key>,
}

/// Run the schedule against a fresh `Vec<Peer<T>>` and a fresh
/// `Oracle<T>`, allowing every gossip event.
pub fn execute<T>(schedule: &Schedule<T>) -> ExecutionResult<T>
where
    T: Clone + Ord + BorshSerialize + BorshDeserialize,
{
    execute_with(schedule, |_, _, _| true)
}

/// Run the schedule and then drive every peer to a full-mesh fixed
/// point. After this returns, every `peers[i].local` should equal
/// every other and match the oracle's projection.
pub fn execute_and_quiesce<T>(schedule: &Schedule<T>) -> ExecutionResult<T>
where
    T: Clone + Eq + Ord + BorshSerialize + BorshDeserialize,
{
    let mut result = execute(schedule);
    quiesce(&mut result.peers);
    result
}

/// Run the schedule with a caller-supplied gossip filter.
///
/// `allow_gossip(a, b, event_idx)` returns whether the gossip event
/// at `event_idx` between peers `a` and `b` should actually fire; a
/// `false` return turns it into a no-op for the purposes of this
/// execution.
///
/// When gossip is suppressed, the schedule's *valid-by-construction*
/// guarantee for `Redact` events no longer holds: a `Redact` whose
/// target the peer has not yet observed in this run is silently
/// skipped (and the oracle does not record it), which models real
/// usage — application code can only `redact()` a `Key` it has been
/// handed.
pub fn execute_with<T, F>(schedule: &Schedule<T>, allow_gossip: F) -> ExecutionResult<T>
where
    T: Clone + Ord + BorshSerialize + BorshDeserialize,
    F: Fn(usize, usize, EventIdx) -> bool,
{
    let mut peers: Vec<Peer<T>> = (0..schedule.n_peers)
        .map(|i| Peer::<T>::new(format!("p{i}")))
        .collect();
    let mut oracle = Oracle::<T>::default();
    let mut resolved_keys: BTreeMap<EventIdx, Key> = BTreeMap::new();

    for (i, event) in schedule.events.iter().enumerate() {
        match event {
            Event::Insert { peer, value } => {
                let key = peers[*peer].insert_one(value.clone());
                resolved_keys.insert(i, key);
                oracle.insert(i, value.clone());
            }
            Event::Redact {
                peer,
                target_event_idx,
            } => {
                let key = resolved_keys[target_event_idx];
                let observed_locally = peers[*peer].observations.iter().any(|(k, _, _)| *k == key);
                if observed_locally {
                    peers[*peer].redact_one(key);
                    oracle.redact(*target_event_idx);
                }
                // else: under a gossip filter, this peer may not yet
                // have observed the key. Real application code
                // couldn't issue this redact, so skip it.
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

    ExecutionResult {
        peers,
        oracle,
        resolved_keys,
    }
}
