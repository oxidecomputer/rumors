//! Spec-shaped oracle for the gossip-set semantics, plus a readout lens
//! that projects a `Local<T>` back into a multiset of currently-live
//! values.
//!
//! The oracle holds only sets and maps — no `Local`, no `process`, no
//! `+` — so a bug in the CRDT's join cannot silently corrupt the
//! reference state.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use borsh::{BorshDeserialize, BorshSerialize};
use rumors::{Key, Local};

pub type PartyId = String;

/// Index of an `Insert` event in a schedule's flat event vector. The
/// oracle and the simulator agree on this identifier so the oracle can
/// record redactions by `InsertId` while the simulator looks up the
/// actual `Key` for the same event.
pub type InsertId = usize;

pub struct Oracle<T> {
    versions: BTreeMap<PartyId, u64>,
    inserts: BTreeMap<InsertId, (PartyId, u64, T)>,
    redacted: BTreeSet<InsertId>,
}

impl<T: Clone + Ord> Oracle<T> {
    pub fn new() -> Self {
        Self {
            versions: BTreeMap::new(),
            inserts: BTreeMap::new(),
            redacted: BTreeSet::new(),
        }
    }

    pub fn insert(&mut self, id: InsertId, party: PartyId, value: T) {
        let counter = self.versions.entry(party.clone()).or_insert(0);
        *counter += 1;
        let v = *counter;
        self.inserts.insert(id, (party, v, value));
    }

    pub fn redact(&mut self, id: InsertId) {
        self.redacted.insert(id);
    }

    /// Multiset of currently-live message values across the whole network.
    pub fn expected_live(&self) -> BTreeMap<T, usize> {
        let mut out = BTreeMap::new();
        for (id, (_, _, value)) in &self.inserts {
            if !self.redacted.contains(id) {
                *out.entry(value.clone()).or_insert(0) += 1;
            }
        }
        out
    }

    /// Every `(InsertId, value)` pair the oracle has seen, redacted or
    /// not. Used by tests that want to inspect the schedule's full set
    /// of inserts.
    pub fn all_inserts(&self) -> &BTreeMap<InsertId, (PartyId, u64, T)> {
        &self.inserts
    }

    pub fn is_redacted(&self, id: InsertId) -> bool {
        self.redacted.contains(&id)
    }
}

/// Project a `Local<T>` into its currently-live `(Key, T)` map by
/// mirroring it into a fresh empty `Local`. Forgotten entries do not
/// fire `on_message` (their paths carry forget tombstones, not
/// messages), so they are naturally excluded.
///
/// The lens uses `Local::process` purely as an enumeration mechanism
/// against an empty baseline; it never accumulates state that anything
/// else depends on.
pub fn readout<T>(peer: &Local<T>) -> BTreeMap<Key, T>
where
    T: Clone + Ord + BorshSerialize + BorshDeserialize,
{
    let mut out = BTreeMap::new();
    let mut lens = Local::<T>::for_party("__readout__");
    lens.process(peer.clone(), |k, _v, m: &Arc<T>| {
        out.insert(k, T::clone(m));
    });
    out
}

/// Multiset (value → count) of a peer's currently-live messages.
pub fn readout_multiset<T>(peer: &Local<T>) -> BTreeMap<T, usize>
where
    T: Clone + Ord + BorshSerialize + BorshDeserialize,
{
    let mut out = BTreeMap::new();
    for v in readout(peer).into_values() {
        *out.entry(v).or_insert(0) += 1;
    }
    out
}
