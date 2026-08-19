//! Spec-shaped oracle for the gossip-set semantics, plus a `readout`
//! lens that projects a [`Snapshot<T>`] back into its currently-live
//! map from message identity (a version's canonical bytes) to value.
//!
//! The oracle holds only `BTreeMap`s and `BTreeSet`s (no rumor set, no
//! merging), so a bug in the live merge primitives cannot silently corrupt
//! the reference state. It records each insert by
//! the schedule's [`EventIdx`] so the oracle and the live executor
//! agree on identity without ever consulting the live [`Version`]s.

use std::collections::{BTreeMap, BTreeSet};

use rumors::{Snapshot, Version};

use super::schedule::EventIdx;

pub struct Oracle<T> {
    values: BTreeMap<EventIdx, T>,
    redacted: BTreeSet<EventIdx>,
}

impl<T> Default for Oracle<T> {
    fn default() -> Self {
        Self {
            values: BTreeMap::new(),
            redacted: BTreeSet::new(),
        }
    }
}

impl<T: Clone + Ord> Oracle<T> {
    pub fn insert(&mut self, id: EventIdx, value: T) {
        self.values.insert(id, value);
    }

    pub fn redact(&mut self, id: EventIdx) {
        self.redacted.insert(id);
    }

    /// Multiset of currently-live message values across the network.
    pub fn expected_live(&self) -> BTreeMap<T, usize> {
        let mut out = BTreeMap::new();
        for (id, value) in &self.values {
            if !self.redacted.contains(id) {
                *out.entry(value.clone()).or_insert(0) += 1;
            }
        }
        out
    }

    /// Every insert the oracle has seen, redacted or not, as
    /// `EventIdx → value`. Used to build the canonical identity → value
    /// map that cross-peer suites compare against.
    pub fn all_inserts(&self) -> &BTreeMap<EventIdx, T> {
        &self.values
    }

    pub fn is_redacted(&self, id: EventIdx) -> bool {
        self.redacted.contains(&id)
    }
}

/// A message's identity as an orderable map key: its [`Version`]'s
/// canonical bytes.
///
/// Canonical and injective, so equality of byte keys is equality of
/// versions; the lexicographic order is an arbitrary total order
/// ([`Version`] itself is only partially ordered).
pub fn version_key(version: &Version) -> Vec<u8> {
    version.as_bytes().to_vec()
}

/// Project a [`Snapshot<T>`] into its currently-live identity → value
/// map, keyed by each message's [`version_key`].
///
/// A direct read via [`Snapshot::iter`]: it enumerates exactly the live
/// leaves, so redacted messages — whose leaves the redaction *removed*,
/// leaving no marker — are simply absent. Taking the [`Snapshot`] rather
/// than a live handle also keeps this oracle independent of observer state.
pub fn readout<T>(snapshot: &Snapshot<T>) -> BTreeMap<Vec<u8>, T>
where
    T: Clone + Send + Sync + 'static,
{
    snapshot
        .iter()
        .map(|(v, m)| (version_key(v), (**m).clone()))
        .collect()
}

/// Multiset (value → count) of a snapshot's currently-live messages.
pub fn readout_multiset<T>(snapshot: &Snapshot<T>) -> BTreeMap<T, usize>
where
    T: Clone + Ord + Send + Sync + 'static,
{
    let mut out = BTreeMap::new();
    for v in readout(snapshot).into_values() {
        *out.entry(v).or_insert(0) += 1;
    }
    out
}
