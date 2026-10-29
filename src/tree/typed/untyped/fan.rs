//! Ordered children of an untyped branch.
//!
//! A [`Fan`] maps byte radixes to node handles. Entries are unique and sorted,
//! allowing binary-search lookups and ordered traversal without sorting again.
//! Small fans fit inline; wider ones use one contiguous allocation.
//!
//! Stored branches have at least two children because path compression
//! absorbs singletons. Temporary fans may be empty or contain one child.
//! Cloning copies the entries and shares their child nodes; copy-on-write
//! belongs to the enclosing node, not this container.

use std::fmt;

use smallvec::SmallVec;

use super::Node;

#[cfg(test)]
mod tests;

/// Entries a [`Fan`] holds inline before spilling to the heap.
///
/// Two entries cover the smallest stored branch and temporary singletons.
/// A larger inline buffer would enlarge every node, including leaves, to
/// avoid allocations only for wider branches.
const FAN_INLINE: usize = 2;

/// The most heap slots a stored fan of `len` children holds: none while its
/// entries fit inline, otherwise the next power of two of its length.
///
/// Growth by [`Fan::push`] doubles the allocation, so a fan built by pushes
/// already meets this bound. Cloning collects the entries into a fresh
/// allocation sized the same way, so the copy-on-write copy of a stored node
/// meets it too. A fan reserved larger than it filled, deduplicated by
/// collection, or shortened by [`Fan::remove`] can hold more; [`Fan::fit`]
/// restores the bound, and every branch constructor fits its fan before
/// storing it. A stored node's heap memory is therefore bounded by its child
/// count, however its fan was assembled.
pub const fn heap_slots(len: usize) -> usize {
    if len <= FAN_INLINE {
        0
    } else {
        len.next_power_of_two()
    }
}

/// Child handles keyed by unique radixes in ascending order.
///
/// Constructors and mutations preserve this order. Hashing, wire encoding,
/// and merge walks can consume it directly.
#[derive(Default, Clone)]
pub struct Fan {
    /// Invariant: strictly ascending by radix, no duplicates.
    entries: SmallVec<[(u8, Node); FAN_INLINE]>,
}

/// Display the fan as an ordered radix-to-node map.
impl fmt::Debug for Fan {
    /// Format entries in ascending radix order.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

/// Construct and update sorted child entries.
impl Fan {
    /// The empty fan.
    pub fn new() -> Self {
        Self::default()
    }

    /// An empty fan with room for `capacity` children, at most 256.
    pub fn with_capacity(capacity: usize) -> Self {
        assert!(capacity <= 256, "a fan has at most 256 radices");
        Self {
            entries: SmallVec::with_capacity(capacity),
        }
    }

    /// A temporary fan holding just `child` at `radix`, without allocating.
    ///
    /// Traversal uses this shape when expanding a compressed path. The node
    /// constructor folds it back into the child's prefix when reassembling.
    pub fn unit(radix: u8, child: Node) -> Self {
        let mut entries = SmallVec::new();
        entries.push((radix, child));
        Self { entries }
    }

    /// Release heap slots beyond [`heap_slots`] of the current length.
    ///
    /// A fan beyond the bound shrinks to exactly its length, moving back
    /// inline when its entries fit there. A fan within the bound, such as one
    /// grown by pushes, keeps its allocation untouched.
    pub fn fit(&mut self) {
        if self.heap_capacity() > heap_slots(self.len()) {
            self.entries.shrink_to_fit();
        }
    }

    /// The entry slots this fan's heap allocation holds; zero while inline.
    pub(crate) fn heap_capacity(&self) -> usize {
        if self.entries.spilled() {
            self.entries.capacity()
        } else {
            0
        }
    }

    /// The number of children present (0..=256).
    pub fn len(&self) -> usize {
        self.entries.len()
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

    /// Remove and return the child at `radix`, if any.
    pub fn remove(&mut self, radix: u8) -> Option<Node> {
        self.search(radix).ok().map(|at| self.entries.remove(at).1)
    }

    /// The first entry at or above `radix`, found by binary search.
    ///
    /// Owned walks use this to resume without collecting pending children.
    pub fn successor(&self, radix: u8) -> Option<(u8, &Node)> {
        let at = self
            .entries
            .partition_point(|(present, _)| *present < radix);
        self.entries.get(at).map(|(radix, child)| (*radix, child))
    }

    /// Append a child whose radix is greater than every existing radix.
    ///
    /// Bulk builds use this to avoid searching for an insertion point. The
    /// caller must supply ascending, unique radixes; debug builds check this.
    pub fn push(&mut self, radix: u8, child: Node) {
        debug_assert!(
            self.entries.last().is_none_or(|(last, _)| *last < radix),
            "Fan::push given a radix not greater than the current last",
        );
        self.entries.push((radix, child));
    }

    /// Borrow entries in ascending radix order, with reverse traversal available.
    ///
    /// The remaining length is exact; hash construction sizes its buffer from it.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = (u8, &Node)> + ExactSizeIterator {
        self.entries.iter().map(|(radix, child)| (*radix, child))
    }

    /// Iterate the children alone, in ascending radix order.
    pub fn values(&self) -> impl DoubleEndedIterator<Item = &Node> + ExactSizeIterator {
        self.entries.iter().map(|(_, child)| child)
    }
}

/// Collect entries, keeping the last child supplied for each radix.
///
/// Strictly ascending input needs no sort. Otherwise a stable sort keeps the
/// last child supplied at each duplicate radix.
impl FromIterator<(u8, Node)> for Fan {
    /// Sort and deduplicate only when input is not already strictly ascending.
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

/// The consuming walk over a fan, ascending by radix.
pub type IntoIter = smallvec::IntoIter<[(u8, Node); FAN_INLINE]>;

/// Transfer the fan's entries to the caller in radix order.
impl IntoIterator for Fan {
    /// A radix and its owned child handle.
    type Item = (u8, Node);
    /// The backing vector's consuming iterator.
    type IntoIter = IntoIter;

    /// Consume the fan in ascending radix order.
    fn into_iter(self) -> Self::IntoIter {
        self.entries.into_iter()
    }
}
