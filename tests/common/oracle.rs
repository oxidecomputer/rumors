//! Spec-shaped oracle for the gossip-set semantics, plus a `readout`
//! lens that projects a `Local<T>` back into its currently-live
//! `(Key, T)` map.
//!
//! The oracle holds only `BTreeMap`s and `BTreeSet`s — no `Local`, no
//! `process`, no `+` — so a bug in the live merge primitives cannot
//! silently corrupt the reference state. It records each insert by
//! the schedule's [`EventIdx`] so the oracle and the live executor
//! agree on identity without ever consulting the live `Key`s.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use borsh::{BorshDeserialize, BorshSerialize};
use rumors::Key;
use rumors::sync::Local;

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

/// Project a `Local<T>` into its currently-live `(Key, T)` map by
/// mirroring it into a fresh empty `Local`. Forgotten entries do not
/// fire `on_message` (their paths carry forget tombstones, not
/// messages), so they are naturally excluded.
///
/// `Local::process` is used here purely as an enumeration mechanism
/// against an empty baseline; the throwaway lens never feeds back into
/// anything.
pub fn readout<T, Id>(peer: &Local<T, Id>) -> BTreeMap<Key, T>
where
    T: Clone + Ord + BorshSerialize + BorshDeserialize + Send + Sync + 'static,
{
    // The sync API's callback bound is `Send + 'static`; route the
    // observation map through an `Arc<Mutex<_>>` clone and unwrap the
    // sole remaining reference once `process` returns.
    let out: Arc<Mutex<BTreeMap<Key, T>>> = Arc::new(Mutex::new(BTreeMap::new()));
    let out_in = Arc::clone(&out);
    // Non-ASCII magic bytes: cannot collide with any test party id,
    // which are all human-readable ASCII strings.
    let mut lens = Local::<T, _>::for_party(b"\x00READOUT\x00", 0).unwrap();
    lens.process(peer.fork(), move |k, _v, m: &Arc<T>| {
        out_in.lock().unwrap().insert(k, T::clone(m));
    });
    Arc::try_unwrap(out)
        .ok()
        .expect("callback closure dropped after `process` returns")
        .into_inner()
        .expect("mutex not poisoned")
}

/// Multiset (value → count) of a peer's currently-live messages.
pub fn readout_multiset<T, Id>(peer: &Local<T, Id>) -> BTreeMap<T, usize>
where
    T: Clone + Ord + BorshSerialize + BorshDeserialize + Send + Sync + 'static,
{
    let mut out = BTreeMap::new();
    for v in readout(peer).into_values() {
        *out.entry(v).or_insert(0) += 1;
    }
    out
}
