//! A display list ordered by causality.
//!
//! Rumors delivers messages in arbitrary order but tags each with a causal
//! [`Version`](rumors::Version), a *partial* order: versions of concurrent
//! events are incomparable. [`CausalList`] maintains a **linear extension**
//! of that partial order — a sequence in which a message never appears
//! before one it causally depends on — and inserts each newcomer at the
//! right position as it arrives, which may be mid-list when gossip delivers
//! a causally old message late.
//!
//! Concurrent entries carry no ordering obligation, so they sit wherever
//! arrival happened to place them: two peers may show concurrent messages in
//! different relative orders, but neither ever violates causality. This is
//! deliberate; the demo's contract is causal consistency, not byte-identical
//! transcripts.
//!
//! The one thing this module must never do is `sort_by` on versions: a
//! partial order has no total comparator, and Rust's sort is allowed to do
//! anything (including panic) when the comparator is inconsistent. Ordered
//! *insertion* against a partial order is the correct tool.
//!
//! The list is generic over the key and version types so its invariants can
//! be property-tested with a synthetic vector clock (real `Version`s can
//! only be minted by a live `Known`).

use std::collections::HashSet;
use std::hash::Hash;

/// One placed entry: a key into the application's message store and the
/// causal version it was observed at.
#[derive(Clone, Debug)]
pub struct Slot<K, V> {
    /// The message's stable identity (a [`rumors::Key`] in production).
    pub key: K,
    /// The causal version the message was observed at.
    pub version: V,
}

/// A sequence of message keys maintained as a linear extension of the causal
/// partial order on their versions.
#[derive(Clone, Debug)]
pub struct CausalList<K, V> {
    slots: Vec<Slot<K, V>>,
    present: HashSet<K>,
}

/// Manual impl: the derive would needlessly bound `K: Default + V: Default`.
impl<K, V> Default for CausalList<K, V> {
    fn default() -> Self {
        CausalList {
            slots: Vec::new(),
            present: HashSet::new(),
        }
    }
}

impl<K: Copy + Eq + Hash, V: PartialOrd> CausalList<K, V> {
    /// An empty list. (Production code reaches this through `Default`; the
    /// tests construct lists directly.)
    #[cfg(test)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Place `key` at its causal position and return the index it landed at,
    /// or `None` if the key is already present (nothing changes).
    ///
    /// An index less than the pre-insert length means the entry landed
    /// mid-list: gossip delivered a message causally older than something
    /// already displayed.
    ///
    /// Placement: immediately after the *last* entry causally before the
    /// newcomer. (Safe in a linear extension: if anything at or before that
    /// position were causally *after* the newcomer, transitivity would force
    /// it to appear after that causally-prior entry — contradiction.) When
    /// no entry precedes it causally, the newcomer goes immediately before
    /// the *first* entry causally after it, past any concurrent prefix; when
    /// neither exists, every entry is concurrent with it and it appends.
    pub fn insert(&mut self, key: K, version: V) -> Option<usize> {
        if !self.present.insert(key) {
            return None;
        }
        let index = match self.slots.iter().rposition(|s| s.version < version) {
            Some(last_before) => last_before + 1,
            None => self
                .slots
                .iter()
                .position(|s| s.version > version)
                .unwrap_or(self.slots.len()),
        };
        self.slots.insert(index, Slot { key, version });
        Some(index)
    }

    /// Remove `key`, returning the index it occupied, or `None` if absent.
    pub fn remove(&mut self, key: &K) -> Option<usize> {
        if !self.present.remove(key) {
            return None;
        }
        let index = self
            .slots
            .iter()
            .position(|s| s.key == *key)
            .expect("present set and slots agree");
        self.slots.remove(index);
        Some(index)
    }

    /// Whether `key` is present.
    #[cfg(test)]
    pub fn contains(&self, key: &K) -> bool {
        self.present.contains(key)
    }

    /// The entries in display order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &Slot<K, V>> {
        self.slots.iter()
    }

    /// The number of entries.
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// Whether the list is empty.
    #[allow(dead_code)] // `len` without `is_empty` trips clippy; keep the pair
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }
}

#[cfg(test)]
mod tests;
