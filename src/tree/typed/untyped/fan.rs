//! The radix fan: a branch's children as a flat, strictly-ascending
//! association from child index to child node.
//!
//! A fan holds at most 256 entries (its keys are the byte alphabet) and a
//! materialized branch holds at least 2 (path compression collapses
//! singletons), with the population concentrated near the small end: past
//! the first few levels of a uniform-hash tree almost every branch carries
//! only a handful of children. At that size a sorted inline vector beats a
//! tree-shaped map on every operation the crate performs — one contiguous
//! (often inline) allocation instead of a heap node per entry, and
//! cache-friendly iteration in the ascending radix order the hash preimage,
//! the wire encoding, and the merge walks all consume directly.
//!
//! [`Fan`] is deliberately *not* a persistent map. Structural sharing lives
//! one level up, on the node handles the fan stores (each entry is one
//! `Arc` reference): cloning a fan is one refcount bump per child, and
//! every mutation site first takes exclusive ownership of the fan (via
//! `Arc::make_mut` or `mem::take` on the enclosing node), so copy-on-write
//! inside the container would buy nothing that the walk following every
//! clone does not already pay for.

use std::mem;

use smallvec::SmallVec;

use super::Node;

#[cfg(test)]
mod tests;

/// Entries a [`Fan`] holds inline before spilling to the heap.
///
/// An entry is 16 bytes (`(u8, Node)`, padded to the handle's
/// alignment), so the fan occupies `8 + max(16 × FAN_INLINE, 16)` bytes
/// inline — 40 at 2. Two entries cover the modal materialized branch (path
/// compression guarantees at least two children, and interior branches
/// rarely carry more) and the transient singleton produced by exploding a
/// path-compressed node, so the tree's hottest reassembly paths never
/// touch the allocator for the fan itself; anything wider spills once to a
/// size-classed heap block the branch then owns. A larger inline capacity
/// would tax every node allocation — the fan is embedded in the largest
/// variant of every node's children enum — for shapes the tree seldom
/// materializes.
const FAN_INLINE: usize = 2;

/// The children of a branch: `(radix, child)` pairs kept strictly
/// ascending by radix, with no duplicate radixes.
///
/// The ordering invariant is private to this module; every constructor and
/// mutator preserves it, so consumers read ascending radix order
/// structurally — the hash preimage and the wire encoding need no re-sort
/// and no caller discipline.
pub struct Fan {
    /// Invariant: strictly ascending by radix, no duplicates.
    entries: SmallVec<[(u8, Node); FAN_INLINE]>,
}

impl Default for Fan {
    fn default() -> Self {
        Self {
            entries: SmallVec::new(),
        }
    }
}

impl Clone for Fan {
    fn clone(&self) -> Self {
        Self {
            entries: self.entries.clone(),
        }
    }
}

impl std::fmt::Debug for Fan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

impl Fan {
    /// The empty fan.
    pub fn new() -> Self {
        Self::default()
    }

    /// The fan holding exactly `child` at `radix`.
    ///
    /// The single-entry shape is transient — it exists between exploding a
    /// path-compressed node and the `branch` constructor collapsing it back
    /// — and fits inline, so building it never allocates.
    pub fn unit(radix: u8, child: Node) -> Self {
        let mut entries = SmallVec::new();
        entries.push((radix, child));
        Self { entries }
    }

    /// The number of children present (0..=256).
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether no child is present.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The index of `radix`, or the insertion point that keeps the fan
    /// ascending.
    fn search(&self, radix: u8) -> Result<usize, usize> {
        self.entries
            .binary_search_by_key(&radix, |(radix, _)| *radix)
    }

    /// The child at `radix`, if any.
    pub fn get(&self, radix: u8) -> Option<&Node> {
        self.search(radix).ok().map(|at| &self.entries[at].1)
    }

    /// Insert `child` at `radix`, returning any child it displaced.
    pub fn insert(&mut self, radix: u8, child: Node) -> Option<Node> {
        match self.search(radix) {
            Ok(at) => Some(mem::replace(&mut self.entries[at].1, child)),
            Err(at) => {
                self.entries.insert(at, (radix, child));
                None
            }
        }
    }

    /// Remove and return the child at `radix`, if any.
    pub fn remove(&mut self, radix: u8) -> Option<Node> {
        self.search(radix).ok().map(|at| self.entries.remove(at).1)
    }

    /// The least entry at or above `radix`, if any.
    ///
    /// The resume point of a suspended ascending walk: one binary search,
    /// so the entries between the cursor and the answer are never
    /// enumerated and no sibling handle is materialized.
    pub fn successor(&self, radix: u8) -> Option<(u8, &Node)> {
        let at = self
            .entries
            .partition_point(|(present, _)| *present < radix);
        self.entries.get(at).map(|(radix, child)| (*radix, child))
    }

    /// Append `(radix, child)`, which must be strictly greater than the
    /// fan's current last radix.
    ///
    /// Sorted bulk builds produce every entry in ascending radix order, so
    /// this O(1) append is their build path; an out-of-order push trips a
    /// debug assertion (in release it would silently break the invariant
    /// the binary searches rely on).
    pub fn push(&mut self, radix: u8, child: Node) {
        debug_assert!(
            self.entries.last().is_none_or(|(last, _)| *last < radix),
            "Fan::push given a radix not greater than the current last",
        );
        self.entries.push((radix, child));
    }

    /// Iterate the fan in ascending radix order.
    ///
    /// Double-ended, and the length reported by `size_hint` is exact at
    /// every step: preimage assembly sizes its buffer from it.
    pub fn iter(&self) -> Iter<'_> {
        Iter {
            inner: self.entries.iter(),
        }
    }

    /// Iterate the children alone, in ascending radix order.
    pub fn values(&self) -> impl DoubleEndedIterator<Item = &Node> + ExactSizeIterator {
        self.entries.iter().map(|(_, child)| child)
    }
}

/// Collect `(radix, child)` pairs into a fan.
///
/// Later pairs displace earlier ones at the same radix, matching repeated
/// [`insert`](Fan::insert). Every reassembly in the crate feeds pairs
/// already strictly ascending and duplicate-free, which this recognizes in
/// one pass; anything else pays one stable sort.
impl FromIterator<(u8, Node)> for Fan {
    fn from_iter<I: IntoIterator<Item = (u8, Node)>>(iter: I) -> Self {
        let mut entries: SmallVec<[(u8, Node); FAN_INLINE]> = iter.into_iter().collect();
        if !entries.windows(2).all(|pair| pair[0].0 < pair[1].0) {
            entries.sort_by_key(|(radix, _)| *radix);
            let mut deduped: SmallVec<[(u8, Node); FAN_INLINE]> =
                SmallVec::with_capacity(entries.len());
            for (radix, child) in entries {
                match deduped.last_mut() {
                    Some((last, slot)) if *last == radix => *slot = child,
                    _ => deduped.push((radix, child)),
                }
            }
            entries = deduped;
        }
        Self { entries }
    }
}

/// The borrowing walk over a fan, ascending by radix; see [`Fan::iter`].
pub struct Iter<'a> {
    inner: std::slice::Iter<'a, (u8, Node)>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = (u8, &'a Node);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(radix, child)| (*radix, child))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl DoubleEndedIterator for Iter<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back().map(|(radix, child)| (*radix, child))
    }
}

impl ExactSizeIterator for Iter<'_> {}

/// The consuming walk over a fan, ascending by radix.
pub struct IntoIter {
    inner: smallvec::IntoIter<[(u8, Node); FAN_INLINE]>,
}

impl Iterator for IntoIter {
    type Item = (u8, Node);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl DoubleEndedIterator for IntoIter {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back()
    }
}

impl ExactSizeIterator for IntoIter {}

impl IntoIterator for Fan {
    type Item = (u8, Node);
    type IntoIter = IntoIter;

    /// Consume the fan in ascending radix order.
    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            inner: self.entries.into_iter(),
        }
    }
}
