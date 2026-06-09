//! A single zipper level: a flat, strictly-ascending association of
//! [`Prefix<H>`] to [`Node<T, H>`].
//!
//! The mirror protocol builds a fresh level (often two) on each side every
//! round and discards it once the zipper descends. Those levels are small
//! (≤256 entries, usually ~16) and short-lived, so a sorted `Vec` beats a
//! `BTreeMap`: one contiguous allocation instead of a heap node per entry,
//! and cache-friendly iteration in the prefix order the wire and the descent
//! both want. [`Level`] keeps that `Vec` strictly ascending by prefix at all
//! times, so iteration, `collapse`, and the wire encoding need no re-sort.

use crate::tree::typed::{Node, Prefix, height::Height};

/// A zipper level: `(prefix, node)` pairs kept in a flat `Vec`, strictly
/// ascending by prefix, with no duplicate prefixes.
pub struct Level<T, H: Height> {
    /// Invariant: strictly ascending by prefix, no duplicates.
    entries: Vec<(Prefix<H>, Node<T, H>)>,
}

impl<T, H: Height> Default for Level<T, H> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

impl<T, H: Height> Clone for Level<T, H> {
    fn clone(&self) -> Self {
        Self {
            entries: self.entries.clone(),
        }
    }
}

impl<T, H: Height> Level<T, H> {
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Wrap a `Vec` that is already strictly ascending by prefix — e.g. a
    /// canonical wire `providing` frame — as a `Level` without re-sorting. Debug
    /// builds assert the invariant. Pairs with [`Level::extend`] to absorb a
    /// sorted batch in one O(n+m) merge rather than m binary-search inserts.
    pub fn from_sorted(entries: Vec<(Prefix<H>, Node<T, H>)>) -> Self {
        debug_assert!(
            entries.windows(2).all(|w| w[0].0 < w[1].0),
            "Level::from_sorted given non-ascending or duplicated entries",
        );
        Self { entries }
    }

    /// Iterate the level in ascending prefix order, mirroring
    /// `BTreeMap::iter`'s `(&key, &value)` shape.
    pub fn iter(&self) -> impl Iterator<Item = (&Prefix<H>, &Node<T, H>)> + '_ {
        self.entries.iter().map(|(prefix, node)| (prefix, node))
    }

    /// Append `(prefix, node)`, which must be strictly greater than the level's
    /// current last prefix. The mirror descent produces every level in ascending
    /// prefix order, so this O(1) append is the common build path; an
    /// out-of-order push trips a debug assertion (in release it would silently
    /// break the invariant the binary searches rely on).
    pub fn push(&mut self, prefix: Prefix<H>, node: Node<T, H>) {
        debug_assert!(
            self.entries.last().is_none_or(|(last, _)| *last < prefix),
            "Level::push given a prefix not greater than the current last",
        );
        self.entries.push((prefix, node));
    }

    /// Remove and return the node at `prefix`, if present.
    pub fn remove(&mut self, prefix: &Prefix<H>) -> Option<Node<T, H>> {
        match self.entries.binary_search_by(|(p, _)| p.cmp(prefix)) {
            Ok(i) => Some(self.entries.remove(i).1),
            Err(_) => None,
        }
    }

    /// Merge `other` into `self`, preserving the ascending invariant. Both sides
    /// are already sorted, so this is a single linear merge rather than a
    /// binary-search insert per element; on a duplicate prefix `other`'s node
    /// wins (matching `BTreeMap::extend`).
    pub fn extend(&mut self, other: Self) {
        if other.is_empty() {
            return;
        }
        if self.is_empty() {
            self.entries = other.entries;
            return;
        }
        // Grow `merged` by `push` rather than pre-sizing to the exact combined
        // length: these merge buffers are allocated and freed every round, and
        // `Vec`'s power-of-two growth recycles through the allocator's size
        // classes far better than an exact, round-varying `with_capacity` (the
        // same effect that made pre-sizing the drained frontier a regression).
        let mut merged = Vec::new();
        let mut ours = std::mem::take(&mut self.entries).into_iter().peekable();
        let mut theirs = other.entries.into_iter().peekable();
        loop {
            match (ours.peek(), theirs.peek()) {
                (Some((a, _)), Some((b, _))) => match a.cmp(b) {
                    std::cmp::Ordering::Less => merged.push(ours.next().unwrap()),
                    std::cmp::Ordering::Greater => merged.push(theirs.next().unwrap()),
                    std::cmp::Ordering::Equal => {
                        ours.next();
                        merged.push(theirs.next().unwrap());
                    }
                },
                (Some(_), None) => merged.push(ours.next().unwrap()),
                (None, Some(_)) => merged.push(theirs.next().unwrap()),
                (None, None) => break,
            }
        }
        self.entries = merged;
    }
}

impl<T, H: Height> FromIterator<(Prefix<H>, Node<T, H>)> for Level<T, H> {
    /// Collect `(prefix, node)` pairs into a level. Callers feed pairs already
    /// in ascending prefix order (a node's children, sorted by radix), so this
    /// sorts only to defend the invariant and `debug_assert`s the input was
    /// canonical (strictly ascending, no duplicates).
    fn from_iter<I: IntoIterator<Item = (Prefix<H>, Node<T, H>)>>(iter: I) -> Self {
        let mut entries: Vec<(Prefix<H>, Node<T, H>)> = iter.into_iter().collect();
        entries.sort_by(|(a, _), (b, _)| a.cmp(b));
        debug_assert!(
            entries.windows(2).all(|w| w[0].0 != w[1].0),
            "Level built from an iterator with duplicate prefixes",
        );
        Self { entries }
    }
}

impl<T, H: Height> IntoIterator for Level<T, H> {
    type Item = (Prefix<H>, Node<T, H>);
    type IntoIter = std::vec::IntoIter<(Prefix<H>, Node<T, H>)>;

    /// Consume the level in ascending prefix order.
    fn into_iter(self) -> Self::IntoIter {
        self.entries.into_iter()
    }
}
