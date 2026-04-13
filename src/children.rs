//! A 256-wide child container for HAMT nodes, indexed by a single byte.
//!
//! [`Children<T>`] stores up to 256 values keyed by `u8` keys, optimized
//! for sparse occupancy. The internal layout is the canonical compressed
//! HAMT representation:
//!
//! - A 256-bit bitmap (`which`, four `u64` words) records which slots are
//!   populated.
//! - A `Vec<T>` (`what`) stores the values in ascending index order, packed
//!   without gaps.
//!
//! The position of the value at index `idx` in `what` is the popcount of bits
//! set in `which` strictly below `idx`. This gives:
//!
//! - O(1) lookup via popcount + indexed slice access.
//! - O(n) insert/remove via [`Vec::insert`] / [`Vec::remove`] shift.
//! - Zero per-slot overhead for absent children.
//!
//! Capacity of the inner `Vec` is bounded above by 256 (since there are only
//! 256 possible keys), with manual power-of-two growth (4, 8, 16, 32, 64,
//! 128, 256). Removal triggers an automatic shrink when the inner `Vec` is
//! more than four times its length, with a hysteresis hint that prevents
//! repeated no-op `shrink_to` calls when the allocator refuses to release
//! memory.

mod bits;
mod entry;
mod iter;

#[cfg(any(test, feature = "proptest"))]
mod proptest;

#[cfg(test)]
mod tests;

use std::hash::{Hash, Hasher};
use std::ops::{Bound, Index, RangeBounds};

use bits::{WORDS, bit_clear, bit_get, bit_set, mask_range, popcount, position};

pub use entry::{Entry, OccupiedEntry, VacantEntry};
pub use iter::{Drain, IntoIter, Iter, IterMut, Keys};

/// Maximum number of children, equal to the number of distinct `u8` keys.
const MAX_CHILDREN: usize = 256;

/// Minimum capacity for the inner `Vec`, used as the initial growth target
/// and as a floor for automatic shrinking.
const MIN_CAPACITY: usize = 4;

/// Up to 256 children of a HAMT node, indexed by a single byte.
///
/// See the [module-level documentation](self) for the storage layout and
/// complexity guarantees.
///
/// # Examples
///
/// ```
/// use rumors::children::Children;
///
/// let mut c: Children<&'static str> = Children::new();
/// c.insert(0x10, "alpha");
/// c.insert(0xff, "omega");
/// assert_eq!(c.len(), 2);
/// assert_eq!(c.get(0x10), Some(&"alpha"));
/// assert_eq!(c.get(0x42), None);
///
/// let keys: Vec<u8> = c.keys().collect();
/// assert_eq!(keys, vec![0x10, 0xff]);
/// ```
#[derive(Clone)]
pub struct Children<T> {
    which: [u64; WORDS],
    what: Vec<T>,
    /// The `what.capacity()` value at the last `maybe_shrink` attempt.
    /// `Vec::shrink_to` is advisory — the allocator may refuse to shrink — so
    /// we record the capacity we last asked at and skip re-attempting until
    /// it changes (which can only happen if an insert grows the `Vec`). Kept
    /// as `usize` because struct alignment to 8 means a smaller integer type
    /// wouldn't actually shrink the struct, and `Vec::capacity` returns
    /// `usize` anyway (notably, for ZST `T` the capacity is `usize::MAX`).
    last_shrink_capacity: usize,
}

impl<T> Children<T> {
    /// Construct an empty `Children` with no slots populated and no
    /// pre-allocated capacity.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let c: Children<u32> = Children::new();
    /// assert!(c.is_empty());
    /// assert_eq!(c.len(), 0);
    /// ```
    #[must_use]
    pub const fn new() -> Self {
        Self {
            which: [0; WORDS],
            what: Vec::new(),
            last_shrink_capacity: 0,
        }
    }

    /// Construct an empty `Children` with the inner `Vec` pre-allocated to
    /// at least `capacity` slots, capped at the maximum of 256.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let c: Children<u32> = Children::with_capacity(64);
    /// assert!(c.is_empty());
    /// ```
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            which: [0; WORDS],
            what: Vec::with_capacity(capacity.min(MAX_CHILDREN)),
            last_shrink_capacity: 0,
        }
    }

    /// The number of children present.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let mut c = Children::new();
    /// c.insert(0, "a");
    /// c.insert(1, "b");
    /// assert_eq!(c.len(), 2);
    /// ```
    #[must_use]
    pub fn len(&self) -> usize {
        self.what.len()
    }

    /// Whether no children are present.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let mut c: Children<i32> = Children::new();
    /// assert!(c.is_empty());
    /// c.insert(7, 0);
    /// assert!(!c.is_empty());
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.what.is_empty()
    }

    /// The current capacity of the inner `Vec`, bounded above by 256.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let c: Children<u32> = Children::with_capacity(32);
    /// assert!(c.capacity() >= 32);
    /// assert!(c.capacity() <= 256);
    /// ```
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.what.capacity()
    }

    /// Whether a child exists at `idx`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let mut c = Children::new();
    /// c.insert(42, "answer");
    /// assert!(c.contains(42));
    /// assert!(!c.contains(43));
    /// ```
    #[must_use]
    pub fn contains(&self, idx: u8) -> bool {
        bit_get(&self.which, idx)
    }

    /// Alias for [`Children::contains`], matching `BTreeMap`'s naming.
    #[must_use]
    pub fn contains_key(&self, idx: u8) -> bool {
        self.contains(idx)
    }

    /// Borrow the child at `idx`, if present.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let mut c = Children::new();
    /// c.insert(7, 100);
    /// assert_eq!(c.get(7), Some(&100));
    /// assert_eq!(c.get(8), None);
    /// ```
    #[must_use]
    pub fn get(&self, idx: u8) -> Option<&T> {
        if self.contains(idx) {
            Some(&self.what[position(&self.which, idx)])
        } else {
            None
        }
    }

    /// Borrow the child at `idx`, returning `(idx, &value)`. Mirrors
    /// [`std::collections::BTreeMap::get_key_value`].
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let mut c = Children::new();
    /// c.insert(7, 100);
    /// assert_eq!(c.get_key_value(7), Some((7, &100)));
    /// assert_eq!(c.get_key_value(8), None);
    /// ```
    #[must_use]
    pub fn get_key_value(&self, idx: u8) -> Option<(u8, &T)> {
        self.get(idx).map(|v| (idx, v))
    }

    /// Mutably borrow the child at `idx`, if present.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let mut c = Children::new();
    /// c.insert(7, 100);
    /// if let Some(v) = c.get_mut(7) {
    ///     *v += 1;
    /// }
    /// assert_eq!(c.get(7), Some(&101));
    /// ```
    pub fn get_mut(&mut self, idx: u8) -> Option<&mut T> {
        if self.contains(idx) {
            let pos = position(&self.which, idx);
            Some(&mut self.what[pos])
        } else {
            None
        }
    }

    /// Insert `value` at `idx`, returning the previous value if the slot was
    /// already occupied.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let mut c = Children::new();
    /// assert_eq!(c.insert(5, "first"), None);
    /// assert_eq!(c.insert(5, "second"), Some("first"));
    /// assert_eq!(c.get(5), Some(&"second"));
    /// ```
    pub fn insert(&mut self, idx: u8, value: T) -> Option<T> {
        let pos = position(&self.which, idx);
        let prev = if bit_get(&self.which, idx) {
            Some(std::mem::replace(&mut self.what[pos], value))
        } else {
            self.insert_at_position(idx, pos, value);
            None
        };
        self.assert_invariant();
        prev
    }

    /// Remove and return the child at `idx`, or `None` if absent.
    ///
    /// May trigger an automatic shrink of the inner `Vec` if its capacity is
    /// now more than four times its length and the allocator hasn't already
    /// refused to shrink at this capacity.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let mut c = Children::new();
    /// c.insert(5, "x");
    /// assert_eq!(c.remove(5), Some("x"));
    /// assert_eq!(c.remove(5), None);
    /// ```
    pub fn remove(&mut self, idx: u8) -> Option<T> {
        if !bit_get(&self.which, idx) {
            return None;
        }
        let pos = position(&self.which, idx);
        bit_clear(&mut self.which, idx);
        let v = self.what.remove(pos);
        self.maybe_shrink();
        self.assert_invariant();
        Some(v)
    }

    /// Remove the child at `idx` and return `(idx, value)`. Mirrors
    /// [`std::collections::BTreeMap::remove_entry`].
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let mut c = Children::new();
    /// c.insert(7, "x");
    /// assert_eq!(c.remove_entry(7), Some((7, "x")));
    /// assert_eq!(c.remove_entry(7), None);
    /// ```
    pub fn remove_entry(&mut self, idx: u8) -> Option<(u8, T)> {
        self.remove(idx).map(|v| (idx, v))
    }

    /// Remove and return the lowest-indexed entry, if any.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let mut c = Children::new();
    /// c.insert(99, "ninety-nine");
    /// c.insert(7, "seven");
    /// assert_eq!(c.pop_first(), Some((7, "seven")));
    /// assert_eq!(c.pop_first(), Some((99, "ninety-nine")));
    /// assert_eq!(c.pop_first(), None);
    /// ```
    pub fn pop_first(&mut self) -> Option<(u8, T)> {
        let idx = self.keys().next()?;
        bit_clear(&mut self.which, idx);
        let v = self.what.remove(0);
        self.maybe_shrink();
        self.assert_invariant();
        Some((idx, v))
    }

    /// Remove and return the highest-indexed entry, if any. O(1) since the
    /// underlying `Vec::pop` doesn't shift elements.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let mut c = Children::new();
    /// c.insert(7, "seven");
    /// c.insert(99, "ninety-nine");
    /// assert_eq!(c.pop_last(), Some((99, "ninety-nine")));
    /// assert_eq!(c.pop_last(), Some((7, "seven")));
    /// assert_eq!(c.pop_last(), None);
    /// ```
    pub fn pop_last(&mut self) -> Option<(u8, T)> {
        let idx = self.keys().next_back()?;
        bit_clear(&mut self.which, idx);
        let v = self
            .what
            .pop()
            .expect("popcount invariant: bit set but what empty");
        self.maybe_shrink();
        self.assert_invariant();
        Some((idx, v))
    }

    /// Remove all children, leaving `self` empty. Capacity of the inner
    /// `Vec` is preserved (use [`Children::shrink_to_fit`] to release it).
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let mut c = Children::new();
    /// c.insert(0, "a");
    /// c.insert(1, "b");
    /// c.clear();
    /// assert!(c.is_empty());
    /// assert_eq!(c.get(0), None);
    /// ```
    pub fn clear(&mut self) {
        self.which = [0; WORDS];
        self.what.clear();
        self.assert_invariant();
    }

    /// Drain all children, returning an iterator that yields `(index, value)`
    /// pairs in ascending index order. After the iterator is dropped, `self`
    /// is empty; capacity of the inner `Vec` is preserved.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let mut c = Children::new();
    /// c.insert(0, "a");
    /// c.insert(1, "b");
    /// let drained: Vec<(u8, &str)> = c.drain().collect();
    /// assert_eq!(drained, vec![(0, "a"), (1, "b")]);
    /// assert!(c.is_empty());
    /// ```
    pub fn drain(&mut self) -> Drain<'_, T> {
        let which = std::mem::take(&mut self.which);
        Drain::new(which, self.what.drain(..))
    }

    /// Retain only the children for which `f` returns `true`. The closure
    /// receives `(index, &mut value)` for each entry in ascending order.
    ///
    /// May trigger an automatic shrink of the inner `Vec`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let mut c = Children::new();
    /// for i in 0u8..10 {
    ///     c.insert(i, i as i32);
    /// }
    /// c.retain(|_, v| *v % 2 == 0);
    /// let kept: Vec<u8> = c.keys().collect();
    /// assert_eq!(kept, vec![0, 2, 4, 6, 8]);
    /// ```
    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(u8, &mut T) -> bool,
    {
        let mut keys = Keys::new(self.which);
        let mut new_which = [0u64; WORDS];
        self.what.retain_mut(|v| {
            let idx = keys
                .next()
                .expect("popcount invariant: keys and what desync");
            let keep = f(idx, v);
            if keep {
                bit_set(&mut new_which, idx);
            }
            keep
        });
        self.which = new_which;
        self.maybe_shrink();
        self.assert_invariant();
    }

    /// Borrow the lowest-indexed child as `(index, &value)`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let mut c = Children::new();
    /// c.insert(99, "ninety-nine");
    /// c.insert(7, "seven");
    /// assert_eq!(c.first(), Some((7, &"seven")));
    /// ```
    #[must_use]
    pub fn first(&self) -> Option<(u8, &T)> {
        let idx = self.keys().next()?;
        Some((idx, &self.what[0]))
    }

    /// Mutably borrow the lowest-indexed child as `(index, &mut value)`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let mut c = Children::new();
    /// c.insert(7, 1);
    /// c.insert(99, 2);
    /// if let Some((_, v)) = c.first_mut() {
    ///     *v = 100;
    /// }
    /// assert_eq!(c.get(7), Some(&100));
    /// ```
    pub fn first_mut(&mut self) -> Option<(u8, &mut T)> {
        let idx = self.keys().next()?;
        Some((idx, &mut self.what[0]))
    }

    /// Return the [`OccupiedEntry`] for the lowest-indexed child, if any.
    /// Useful for "modify or remove" patterns at the endpoints.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let mut c = Children::new();
    /// c.insert(7, 1);
    /// c.insert(99, 2);
    /// if let Some(e) = c.first_entry() {
    ///     assert_eq!(e.key(), 7);
    ///     assert_eq!(e.remove(), 1);
    /// }
    /// assert!(!c.contains(7));
    /// ```
    pub fn first_entry(&mut self) -> Option<OccupiedEntry<'_, T>> {
        let idx = self.keys().next()?;
        match self.entry(idx) {
            Entry::Occupied(e) => Some(e),
            Entry::Vacant(_) => unreachable!("first index from keys() must be occupied"),
        }
    }

    /// Borrow the highest-indexed child as `(index, &value)`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let mut c = Children::new();
    /// c.insert(7, "seven");
    /// c.insert(99, "ninety-nine");
    /// assert_eq!(c.last(), Some((99, &"ninety-nine")));
    /// ```
    #[must_use]
    pub fn last(&self) -> Option<(u8, &T)> {
        let idx = self.keys().next_back()?;
        let last_pos = self.what.len() - 1;
        Some((idx, &self.what[last_pos]))
    }

    /// Mutably borrow the highest-indexed child as `(index, &mut value)`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let mut c = Children::new();
    /// c.insert(7, 1);
    /// c.insert(99, 2);
    /// if let Some((_, v)) = c.last_mut() {
    ///     *v = 200;
    /// }
    /// assert_eq!(c.get(99), Some(&200));
    /// ```
    pub fn last_mut(&mut self) -> Option<(u8, &mut T)> {
        let idx = self.keys().next_back()?;
        let last_pos = self.what.len() - 1;
        Some((idx, &mut self.what[last_pos]))
    }

    /// Return the [`OccupiedEntry`] for the highest-indexed child, if any.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let mut c = Children::new();
    /// c.insert(7, 1);
    /// c.insert(99, 2);
    /// if let Some(mut e) = c.last_entry() {
    ///     *e.get_mut() = 200;
    /// }
    /// assert_eq!(c.get(99), Some(&200));
    /// ```
    pub fn last_entry(&mut self) -> Option<OccupiedEntry<'_, T>> {
        let idx = self.keys().next_back()?;
        match self.entry(idx) {
            Entry::Occupied(e) => Some(e),
            Entry::Vacant(_) => unreachable!("last index from keys() must be occupied"),
        }
    }

    /// Move every entry from `other` into `self`, leaving `other` empty.
    /// On a key collision, the value from `other` overwrites the value in
    /// `self` (matching [`std::collections::BTreeMap::append`]).
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let mut a: Children<i32> = [(0, 1), (1, 10)].into_iter().collect();
    /// let mut b: Children<i32> = [(1, 100), (2, 1000)].into_iter().collect();
    /// a.append(&mut b);
    /// assert_eq!(a.get(0), Some(&1));
    /// assert_eq!(a.get(1), Some(&100));
    /// assert_eq!(a.get(2), Some(&1000));
    /// assert!(b.is_empty());
    /// ```
    pub fn append(&mut self, other: &mut Self) {
        self.extend(other.drain());
    }

    /// Iterate the keys of populated slots in ascending order.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let mut c: Children<()> = Children::new();
    /// c.insert(200, ());
    /// c.insert(7, ());
    /// let keys: Vec<u8> = c.keys().collect();
    /// assert_eq!(keys, vec![7, 200]);
    /// ```
    #[must_use]
    pub fn keys(&self) -> Keys {
        Keys::new(self.which)
    }

    /// Iterate `(index, &value)` pairs in ascending index order.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let mut c = Children::new();
    /// c.insert(2, "two");
    /// c.insert(1, "one");
    /// let pairs: Vec<(u8, &&str)> = c.iter().collect();
    /// assert_eq!(pairs, vec![(1, &"one"), (2, &"two")]);
    /// ```
    #[must_use]
    pub fn iter(&self) -> Iter<'_, T> {
        Iter::new(self.which, self.what.iter())
    }

    /// Iterate `(index, &mut value)` pairs in ascending index order.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let mut c = Children::new();
    /// c.insert(1, 10);
    /// c.insert(2, 20);
    /// for (_, v) in c.iter_mut() {
    ///     *v *= 2;
    /// }
    /// assert_eq!(c.get(1), Some(&20));
    /// assert_eq!(c.get(2), Some(&40));
    /// ```
    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut::new(self.which, self.what.iter_mut())
    }

    /// Iterate `(index, &value)` pairs whose index falls within the given
    /// range, in ascending order.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let mut c = Children::new();
    /// for i in [0u8, 5, 10, 15, 20] {
    ///     c.insert(i, i);
    /// }
    /// let r: Vec<u8> = c.range(5..15).map(|(k, _)| k).collect();
    /// assert_eq!(r, vec![5, 10]);
    /// let r: Vec<u8> = c.range(5..=15).map(|(k, _)| k).collect();
    /// assert_eq!(r, vec![5, 10, 15]);
    /// let r: Vec<u8> = c.range(..10).map(|(k, _)| k).collect();
    /// assert_eq!(r, vec![0, 5]);
    /// ```
    pub fn range<R: RangeBounds<u8>>(&self, range: R) -> Iter<'_, T> {
        let (start, end) = range_bounds_to_u16(range);
        let (start_pos, end_pos) = self.range_positions(start, end);
        let masked = mask_range(self.which, start, end);
        Iter::new(masked, self.what[start_pos..end_pos].iter())
    }

    /// Iterate `(index, &mut value)` pairs whose index falls within the given
    /// range, in ascending order.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let mut c = Children::new();
    /// for i in 0u8..10 {
    ///     c.insert(i, i as i32);
    /// }
    /// for (_, v) in c.range_mut(3..7) {
    ///     *v *= 10;
    /// }
    /// assert_eq!(c.get(2), Some(&2));
    /// assert_eq!(c.get(3), Some(&30));
    /// assert_eq!(c.get(6), Some(&60));
    /// assert_eq!(c.get(7), Some(&7));
    /// ```
    pub fn range_mut<R: RangeBounds<u8>>(&mut self, range: R) -> IterMut<'_, T> {
        let (start, end) = range_bounds_to_u16(range);
        let (start_pos, end_pos) = self.range_positions(start, end);
        let masked = mask_range(self.which, start, end);
        IterMut::new(masked, self.what[start_pos..end_pos].iter_mut())
    }

    /// Iterate the values in ascending index order, ignoring keys.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let mut c = Children::new();
    /// c.insert(2, 20);
    /// c.insert(1, 10);
    /// let v: Vec<i32> = c.values().copied().collect();
    /// assert_eq!(v, vec![10, 20]);
    /// ```
    pub fn values(&self) -> std::slice::Iter<'_, T> {
        self.what.iter()
    }

    /// Mutably iterate the values in ascending index order, ignoring keys.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let mut c = Children::new();
    /// c.insert(0, 1);
    /// c.insert(1, 2);
    /// for v in c.values_mut() {
    ///     *v *= 100;
    /// }
    /// assert_eq!(c.get(0), Some(&100));
    /// assert_eq!(c.get(1), Some(&200));
    /// ```
    pub fn values_mut(&mut self) -> std::slice::IterMut<'_, T> {
        self.what.iter_mut()
    }

    /// Consume `self` and yield owned values in ascending index order.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let mut c = Children::new();
    /// c.insert(0, "a".to_string());
    /// c.insert(1, "b".to_string());
    /// let v: Vec<String> = c.into_values().collect();
    /// assert_eq!(v, vec!["a".to_string(), "b".to_string()]);
    /// ```
    pub fn into_values(self) -> std::vec::IntoIter<T> {
        self.what.into_iter()
    }

    /// Get the [`Entry`] for `idx`, supporting in-place insertion or update
    /// patterns. Modeled after `std::collections::hash_map::Entry`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let mut c: Children<i32> = Children::new();
    /// *c.entry(0).or_insert(1) += 10;
    /// *c.entry(0).or_insert(0) += 100;
    /// assert_eq!(c.get(0), Some(&111));
    /// ```
    pub fn entry(&mut self, idx: u8) -> Entry<'_, T> {
        Entry::new(self, idx)
    }

    /// Split `self` at index `at`, returning a new `Children` containing all
    /// entries with index `>= at`. `self` retains entries with index `< at`.
    /// Both halves may be empty.
    ///
    /// May trigger an automatic shrink of the inner `Vec` on `self`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let mut left: Children<i32> = (0u8..10).map(|i| (i, i as i32)).collect();
    /// let right = left.split_off(5);
    /// let lk: Vec<u8> = left.keys().collect();
    /// let rk: Vec<u8> = right.keys().collect();
    /// assert_eq!(lk, vec![0, 1, 2, 3, 4]);
    /// assert_eq!(rk, vec![5, 6, 7, 8, 9]);
    /// ```
    #[must_use = "split_off returns the right half; discarding it drops those entries"]
    pub fn split_off(&mut self, at: u8) -> Self {
        let split_pos = position(&self.which, at);
        let right_what = self.what.split_off(split_pos);
        let right_which = mask_range(self.which, at as u16, 256);
        self.which = mask_range(self.which, 0, at as u16);
        self.maybe_shrink();
        self.assert_invariant();
        let right = Self {
            which: right_which,
            what: right_what,
            last_shrink_capacity: 0,
        };
        right.assert_invariant();
        right
    }

    /// Shrink the inner `Vec`'s capacity as close to `len()` as the allocator
    /// permits. The hysteresis hint is updated so subsequent auto-shrinks
    /// won't redundantly re-attempt at the same capacity.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let mut c: Children<u32> = Children::with_capacity(128);
    /// c.insert(0, 1);
    /// c.shrink_to_fit();
    /// assert!(c.capacity() < 128);
    /// ```
    pub fn shrink_to_fit(&mut self) {
        self.what.shrink_to_fit();
        self.last_shrink_capacity = self.what.capacity();
    }

    /// Consume `self` and `other`, producing a `Children` containing every
    /// index present in either operand. For keys present in both,
    /// `combiner` is called with `(idx, value_from_self, value_from_other)`
    /// to produce the merged value.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let a: Children<i32> = [(0, 1), (1, 10)].into_iter().collect();
    /// let b: Children<i32> = [(1, 100), (2, 1000)].into_iter().collect();
    /// let merged = a.union(b, |_, l, r| l + r);
    /// assert_eq!(merged.get(0), Some(&1));
    /// assert_eq!(merged.get(1), Some(&110));
    /// assert_eq!(merged.get(2), Some(&1000));
    /// ```
    #[must_use]
    pub fn union<F>(self, other: Self, mut combiner: F) -> Self
    where
        F: FnMut(u8, T, T) -> T,
    {
        let total_capacity = (self.len() + other.len()).min(MAX_CHILDREN);
        let mut result = Self::with_capacity(total_capacity);
        let mut a = self.into_iter().peekable();
        let mut b = other.into_iter().peekable();
        loop {
            match (a.peek(), b.peek()) {
                (None, None) => break,
                (Some(_), None) => {
                    let (k, v) = a.next().unwrap();
                    result.push_back_unchecked(k, v);
                }
                (None, Some(_)) => {
                    let (k, v) = b.next().unwrap();
                    result.push_back_unchecked(k, v);
                }
                (Some(&(ka, _)), Some(&(kb, _))) => match ka.cmp(&kb) {
                    std::cmp::Ordering::Less => {
                        let (k, v) = a.next().unwrap();
                        result.push_back_unchecked(k, v);
                    }
                    std::cmp::Ordering::Greater => {
                        let (k, v) = b.next().unwrap();
                        result.push_back_unchecked(k, v);
                    }
                    std::cmp::Ordering::Equal => {
                        let (_, va) = a.next().unwrap();
                        let (k, vb) = b.next().unwrap();
                        result.push_back_unchecked(k, combiner(k, va, vb));
                    }
                },
            }
        }
        result.assert_invariant();
        result
    }

    /// Consume `self` and `other`, producing a `Children` containing only
    /// keys present in both operands. `combiner` produces the merged value
    /// for each shared index.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let a: Children<i32> = [(0, 1), (1, 10), (2, 100)].into_iter().collect();
    /// let b: Children<i32> = [(1, 1000), (2, 2000), (3, 3000)].into_iter().collect();
    /// let shared = a.intersection(b, |_, l, r| l + r);
    /// assert_eq!(shared.get(1), Some(&1010));
    /// assert_eq!(shared.get(2), Some(&2100));
    /// assert!(!shared.contains(0));
    /// assert!(!shared.contains(3));
    /// ```
    #[must_use]
    pub fn intersection<F>(self, other: Self, mut combiner: F) -> Self
    where
        F: FnMut(u8, T, T) -> T,
    {
        let mut result = Self::new();
        let mut a = self.into_iter().peekable();
        let mut b = other.into_iter().peekable();
        loop {
            match (a.peek(), b.peek()) {
                (None, _) | (_, None) => break,
                (Some(&(ka, _)), Some(&(kb, _))) => match ka.cmp(&kb) {
                    std::cmp::Ordering::Less => {
                        a.next();
                    }
                    std::cmp::Ordering::Greater => {
                        b.next();
                    }
                    std::cmp::Ordering::Equal => {
                        let (_, va) = a.next().unwrap();
                        let (k, vb) = b.next().unwrap();
                        result.push_back_unchecked(k, combiner(k, va, vb));
                    }
                },
            }
        }
        result.assert_invariant();
        result
    }

    /// Consume `self`, producing a `Children` containing only keys in
    /// `self` that are not in `other`. No combiner is needed since shared
    /// keys are excluded.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let a: Children<i32> = [(0, 1), (1, 10), (2, 100)].into_iter().collect();
    /// let b: Children<i32> = [(1, 1000), (3, 3000)].into_iter().collect();
    /// let only_a = a.difference(&b);
    /// let keys: Vec<u8> = only_a.keys().collect();
    /// assert_eq!(keys, vec![0, 2]);
    /// ```
    #[must_use]
    pub fn difference(self, other: &Self) -> Self {
        let mut result = Self::new();
        for (k, v) in self {
            if !other.contains(k) {
                result.push_back_unchecked(k, v);
            }
        }
        result.assert_invariant();
        result
    }

    /// Consume `self` and `other`, producing a `Children` containing keys
    /// present in exactly one operand. No combiner is needed since shared
    /// keys are excluded.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let a: Children<i32> = [(0, 1), (1, 10), (2, 100)].into_iter().collect();
    /// let b: Children<i32> = [(1, 1000), (3, 3000)].into_iter().collect();
    /// let xor = a.symmetric_difference(b);
    /// let keys: Vec<u8> = xor.keys().collect();
    /// assert_eq!(keys, vec![0, 2, 3]);
    /// ```
    #[must_use]
    pub fn symmetric_difference(self, other: Self) -> Self {
        let mut result = Self::new();
        let mut a = self.into_iter().peekable();
        let mut b = other.into_iter().peekable();
        loop {
            match (a.peek(), b.peek()) {
                (None, None) => break,
                (Some(_), None) => {
                    let (k, v) = a.next().unwrap();
                    result.push_back_unchecked(k, v);
                }
                (None, Some(_)) => {
                    let (k, v) = b.next().unwrap();
                    result.push_back_unchecked(k, v);
                }
                (Some(&(ka, _)), Some(&(kb, _))) => match ka.cmp(&kb) {
                    std::cmp::Ordering::Less => {
                        let (k, v) = a.next().unwrap();
                        result.push_back_unchecked(k, v);
                    }
                    std::cmp::Ordering::Greater => {
                        let (k, v) = b.next().unwrap();
                        result.push_back_unchecked(k, v);
                    }
                    std::cmp::Ordering::Equal => {
                        a.next();
                        b.next();
                    }
                },
            }
        }
        result.assert_invariant();
        result
    }

    // ------------------------------------------------------------------
    // Internal helpers shared with submodules.
    // ------------------------------------------------------------------

    /// Insert `value` at `(idx, pos)` assuming `idx` is currently absent and
    /// `pos == position(which, idx)`. Grows the inner `Vec` if needed.
    pub(super) fn insert_at_position(&mut self, idx: u8, pos: usize, value: T) {
        self.grow_to_fit_one_more();
        bit_set(&mut self.which, idx);
        self.what.insert(pos, value);
    }

    /// Try to shrink the inner `Vec` if it's significantly over-allocated and
    /// we haven't already attempted at this capacity. The hysteresis prevents
    /// repeated calls when the allocator refuses to shrink.
    pub(super) fn maybe_shrink(&mut self) {
        let cap = self.what.capacity();
        let len = self.what.len();
        if len.saturating_mul(4) > cap {
            return;
        }
        if cap == self.last_shrink_capacity {
            return;
        }
        let target = len.saturating_mul(2).max(MIN_CAPACITY);
        self.what.shrink_to(target);
        self.last_shrink_capacity = self.what.capacity();
    }

    /// Append `(idx, value)` directly, assuming `idx` is greater than every
    /// currently-set index (so it goes at the end of `what`). Used by set
    /// operations that build the result in ascending order.
    fn push_back_unchecked(&mut self, idx: u8, value: T) {
        debug_assert!(
            !bit_get(&self.which, idx),
            "push_back_unchecked called on occupied index"
        );
        debug_assert!(
            self.keys().next_back().is_none_or(|prev| prev < idx),
            "push_back_unchecked called out of order"
        );
        self.grow_to_fit_one_more();
        bit_set(&mut self.which, idx);
        self.what.push(value);
    }

    /// Grow the inner `Vec` by one power-of-two step (capped at 256) if it's
    /// currently full. Uses [`Vec::reserve_exact`] to avoid `Vec`'s default
    /// over-allocation strategy.
    fn grow_to_fit_one_more(&mut self) {
        let len = self.what.len();
        let cap = self.what.capacity();
        if len < cap {
            return;
        }
        let target = next_capacity(cap);
        if target > cap {
            self.what.reserve_exact(target - cap);
        }
    }

    /// Compute the start and end positions in `what` for the given half-open
    /// `[start, end)` index range, where bounds are in `0..=256`.
    fn range_positions(&self, start: u16, end: u16) -> (usize, usize) {
        if start >= end {
            return (0, 0);
        }
        let start_pos = if start == 256 {
            self.what.len()
        } else {
            position(&self.which, start as u8)
        };
        let end_pos = if end == 256 {
            self.what.len()
        } else {
            position(&self.which, end as u8)
        };
        (start_pos, end_pos)
    }

    /// Debug-only check that the popcount of `which` matches `what.len()`.
    /// Capacity is intentionally not asserted: for zero-sized `T`, `Vec`'s
    /// capacity is `usize::MAX` and our manual growth logic doesn't apply.
    #[cfg(debug_assertions)]
    #[track_caller]
    fn assert_invariant(&self) {
        debug_assert_eq!(popcount(&self.which), self.what.len(), "popcount mismatch");
    }

    #[cfg(not(debug_assertions))]
    #[inline(always)]
    fn assert_invariant(&self) {}
}

impl<T> Default for Children<T> {
    /// Returns an empty `Children`. Equivalent to [`Children::new`].
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let c: Children<u8> = Children::default();
    /// assert!(c.is_empty());
    /// ```
    fn default() -> Self {
        Self::new()
    }
}

/// Two `Children` are equal if their bitmaps and value vectors agree. The
/// internal shrink hint is excluded — it's a transient hysteresis state, not
/// part of the value.
impl<T: PartialEq> PartialEq for Children<T> {
    fn eq(&self, other: &Self) -> bool {
        self.which == other.which && self.what == other.what
    }
}

impl<T: Eq> Eq for Children<T> {}

/// Wrapper that formats a child index as `0xNN` (lowercase, zero-padded to
/// two hex digits) in `Debug` output.
struct HexIdx(u8);

impl std::fmt::Debug for HexIdx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "0x{:02x}", self.0)
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Children<T> {
    /// Format `Children` as a map from index to value, in ascending index
    /// order. Keys are shown in lowercase hex, zero-padded to two digits.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let mut c = Children::new();
    /// c.insert(1, "a");
    /// c.insert(0xff, "b");
    /// assert_eq!(format!("{:?}", c), r#"{0x01: "a", 0xff: "b"}"#);
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map()
            .entries(self.iter().map(|(k, v)| (HexIdx(k), v)))
            .finish()
    }
}

impl<T> FromIterator<(u8, T)> for Children<T> {
    /// Build a `Children` from an iterator of `(index, value)` pairs. If the
    /// iterator yields the same index more than once, the last write wins.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let c: Children<i32> = [(2, 20), (1, 10), (3, 30)].into_iter().collect();
    /// assert_eq!(c.len(), 3);
    /// assert_eq!(c.get(1), Some(&10));
    /// ```
    fn from_iter<I: IntoIterator<Item = (u8, T)>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let (lower, upper) = iter.size_hint();
        // For ExactSizeIterator, upper == Some(lower) == exact remaining;
        // a single `with_capacity` saves all subsequent reallocations.
        // For non-exact iterators, fall back to the lower bound.
        let cap = upper.unwrap_or(lower).min(MAX_CHILDREN);
        let mut c = Self::with_capacity(cap);
        for (k, v) in iter {
            c.insert(k, v);
        }
        c
    }
}

impl<T> Extend<(u8, T)> for Children<T> {
    /// Insert each `(index, value)` from `iter` into `self`. If the iterator
    /// yields the same index more than once, the last write wins.
    ///
    /// Pre-reserves capacity using the iterator's lower size hint, capped at
    /// the maximum of 256.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let mut c = Children::new();
    /// c.insert(0, 0);
    /// c.extend([(1, 10), (2, 20)]);
    /// assert_eq!(c.len(), 3);
    /// ```
    fn extend<I: IntoIterator<Item = (u8, T)>>(&mut self, iter: I) {
        let iter = iter.into_iter();
        let (lower, upper) = iter.size_hint();
        // Use the upper bound if available (ExactSizeIterator) so the reserve
        // covers the entire iterator; otherwise fall back to the lower bound.
        let need = upper.unwrap_or(lower);
        let cap_room = MAX_CHILDREN.saturating_sub(self.what.len());
        self.what.reserve_exact(need.min(cap_room));
        for (k, v) in iter {
            self.insert(k, v);
        }
    }
}

impl<T, const N: usize> From<[(u8, T); N]> for Children<T> {
    /// Build a `Children` from an array of `(index, value)` pairs.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let c = Children::from([(0, "a"), (1, "b"), (2, "c")]);
    /// assert_eq!(c.len(), 3);
    /// assert_eq!(c.get(1), Some(&"b"));
    /// ```
    fn from(arr: [(u8, T); N]) -> Self {
        arr.into_iter().collect()
    }
}

/// `Hash` mirrors [`PartialEq`]: it hashes `which` and `what`, ignoring the
/// transient `last_shrink_capacity` hint so that equal `Children` hash equal.
impl<T: Hash> Hash for Children<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.which.hash(state);
        self.what.hash(state);
    }
}

impl<T> Index<u8> for Children<T> {
    type Output = T;

    /// Borrow the value at `idx`. **Panics** if `idx` is absent — prefer
    /// [`Children::get`] if you want a checked lookup.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let mut c = Children::new();
    /// c.insert(7, "seven");
    /// assert_eq!(c[7], "seven");
    /// ```
    ///
    /// ```should_panic
    /// use rumors::children::Children;
    ///
    /// let c: Children<&str> = Children::new();
    /// let _ = c[0];  // panics: no entry
    /// ```
    fn index(&self, idx: u8) -> &T {
        self.get(idx)
            .expect("Children: no entry at the requested index")
    }
}

/// Compute the next power-of-two capacity (4, 8, 16, ..., 256) above the
/// given current capacity.
fn next_capacity(cap: usize) -> usize {
    if cap >= MAX_CHILDREN {
        return MAX_CHILDREN;
    }
    if cap == 0 {
        return MIN_CAPACITY;
    }
    (cap * 2).min(MAX_CHILDREN)
}

/// Convert any `RangeBounds<u8>` into a half-open `[start, end)` pair where
/// both endpoints are in `0..=256`.
fn range_bounds_to_u16<R: RangeBounds<u8>>(range: R) -> (u16, u16) {
    let start: u16 = match range.start_bound() {
        Bound::Included(&n) => n as u16,
        Bound::Excluded(&n) => n as u16 + 1,
        Bound::Unbounded => 0,
    };
    let end: u16 = match range.end_bound() {
        Bound::Included(&n) => n as u16 + 1,
        Bound::Excluded(&n) => n as u16,
        Bound::Unbounded => 256,
    };
    (start, end)
}
