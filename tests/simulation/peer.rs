//! Peer<T>: a `Local<T>` plus its observation log, with helpers for the
//! schedule executor — `gossip_step` (bidirectional `Local::process`)
//! and `quiesce` (repeated full-mesh gossip to a fixed point).

use borsh::{BorshDeserialize, BorshSerialize};
use rumors::{Key, Local, Version};

use crate::oracle::PartyId;

/// One simulated peer.
pub struct Peer<T> {
    /// Party identifier, kept around for human-readable assertion
    /// messages and Debug-printing in proptest counterexamples.
    #[allow(dead_code)]
    pub party: PartyId,
    pub local: Local<T>,
    /// All observations this peer has accumulated, across `message`,
    /// `redact`, and `process` calls. Ordered as the callbacks fired
    /// (which is arbitrary within a batch per the public API contract,
    /// but consistent across runs because the underlying types are
    /// `imbl::OrdMap`-backed and deterministic).
    pub observations: Vec<(Key, Version, T)>,
}

impl<T: Clone + BorshSerialize + BorshDeserialize> Peer<T> {
    pub fn new(party: impl Into<PartyId>) -> Self {
        let party = party.into();
        let local = Local::for_party(&party);
        Self {
            party,
            local,
            observations: Vec::new(),
        }
    }

    /// Insert a single value, returning the `Key` minted for it.
    pub fn insert_one(&mut self, value: T) -> Key {
        let mut produced: Option<Key> = None;
        let obs = &mut self.observations;
        self.local.message([value], |k, v, m| {
            obs.push((k, v.clone(), T::clone(m)));
            produced = Some(k);
        });
        produced.expect("Local::message must fire on_message for every inserted value")
    }

    pub fn redact_one(&mut self, key: Key) {
        self.local.redact([key]);
    }
}

/// Bidirectional gossip between two raw `Local`s — discards
/// observation callbacks. Used by the algebraic (Group B) tests that
/// only care about final state, not observation history.
pub fn gossip_step_local<T>(a: &mut Local<T>, b: &mut Local<T>)
where
    T: Clone + BorshSerialize + BorshDeserialize,
{
    let a_snapshot = a.clone();
    let b_snapshot = b.clone();
    a.process(b_snapshot, |_, _, _| {});
    b.process(a_snapshot, |_, _, _| {});
}

/// Bidirectional gossip between two peers: each side merges the other's
/// state into its own. After this returns, `a.local == b.local`.
///
/// Implemented in terms of `Local::process` (the public, pure,
/// deterministic merge primitive) — see plan §"Approach".
pub fn gossip_step<T>(a: &mut Peer<T>, b: &mut Peer<T>)
where
    T: Clone + BorshSerialize + BorshDeserialize,
{
    let a_snapshot = a.local.clone();
    let b_snapshot = b.local.clone();

    let obs_a = &mut a.observations;
    a.local.process(b_snapshot, |k, v, m| {
        obs_a.push((k, v.clone(), T::clone(m)));
    });

    let obs_b = &mut b.observations;
    b.local.process(a_snapshot, |k, v, m| {
        obs_b.push((k, v.clone(), T::clone(m)));
    });
}

/// Drive every pair toward convergence by repeatedly running
/// `gossip_step` over all pairs in a fixed order until no peer's
/// `Local` changes for a full round. A bounded outer loop guards
/// against pathological non-termination (which would itself be a bug
/// the test should catch).
pub fn quiesce<T>(peers: &mut [Peer<T>])
where
    T: Clone + Eq + BorshSerialize + BorshDeserialize,
{
    let n = peers.len();
    if n < 2 {
        return;
    }

    let max_rounds = MAX_QUIESCE_ROUNDS_PER_PEER * n.max(1);
    for _ in 0..max_rounds {
        let snapshot: Vec<Local<T>> = peers.iter().map(|p| p.local.clone()).collect();

        for i in 0..n {
            for j in (i + 1)..n {
                let (left, right) = peers.split_at_mut(j);
                gossip_step(&mut left[i], &mut right[0]);
            }
        }

        let changed = peers
            .iter()
            .zip(snapshot.iter())
            .any(|(p, s)| &p.local != s);
        if !changed {
            return;
        }
    }

    panic!(
        "quiesce did not converge within {} rounds for {} peers — \
         either a CRDT bug or the schedule diverges",
        max_rounds, n
    );
}

/// Headroom factor on the convergence loop: each round runs all
/// `n_peers * (n_peers - 1) / 2` pairs, and a single piece of
/// information needs at most O(diameter) rounds to reach every peer.
/// 16 rounds per peer is dramatically more than enough; we use it
/// only to bound test pathologies.
const MAX_QUIESCE_ROUNDS_PER_PEER: usize = 16;
