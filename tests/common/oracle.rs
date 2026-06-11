//! Spec-shaped oracle for the gossip-set semantics, plus a `readout`
//! lens that projects a [`Snapshot<T>`] back into its currently-live
//! `(Key, T)` map.
//!
//! The oracle holds only `BTreeMap`s and `BTreeSet`s (no rumor set, no
//! merging), so a bug in the live merge primitives cannot silently corrupt
//! the reference state. It records each insert by
//! the schedule's [`EventIdx`] so the oracle and the live executor
//! agree on identity without ever consulting the live `Key`s.

use std::collections::{BTreeMap, BTreeSet};

use rumors::{Key, Snapshot};

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
    /// `EventIdx → value`. Used by [`multi_peer::keys_stable_across_peers`]
    /// to build the canonical `Key → value` map.
    ///
    /// [`multi_peer::keys_stable_across_peers`]: crate::multi_peer
    pub fn all_inserts(&self) -> &BTreeMap<EventIdx, T> {
        &self.values
    }

    pub fn is_redacted(&self, id: EventIdx) -> bool {
        self.redacted.contains(&id)
    }
}

/// Project a [`Snapshot<T>`] into its currently-live `(Key, T)` map.
///
/// A direct read via [`Snapshot::iter`]: it enumerates exactly the live
/// leaves, so redacted messages — whose leaves the redaction *removed*,
/// leaving no marker — are simply absent. Taking the [`Snapshot`] (rather
/// than a handle type) keeps one lens for `Rumors` and `sync::Rumors`
/// alike.
pub fn readout<T>(snapshot: &Snapshot<T>) -> BTreeMap<Key, T>
where
    T: Clone + Send + Sync + 'static,
{
    snapshot
        .iter()
        .map(|(k, _v, m)| (k, (**m).clone()))
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
