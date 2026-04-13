//! [`Entry`] API for [`Children`], modeled after
//! `std::collections::hash_map::Entry`.

use super::Children;
use super::bits::{bit_clear, bit_get, position};

/// A view into a single slot of a [`Children`], whether occupied or vacant.
///
/// Constructed via [`Children::entry`].
///
/// # Examples
///
/// ```
/// use rumors::children::{Children, Entry};
///
/// let mut c: Children<u32> = Children::new();
/// match c.entry(0) {
///     Entry::Occupied(_) => unreachable!(),
///     Entry::Vacant(v) => { v.insert(42); }
/// }
/// assert_eq!(c.get(0), Some(&42));
/// ```
pub enum Entry<'a, T> {
    /// A slot that already contains a value.
    Occupied(OccupiedEntry<'a, T>),
    /// A slot that is empty.
    Vacant(VacantEntry<'a, T>),
}

/// A view into an occupied slot of a [`Children`]. Returned as a variant of
/// [`Entry`].
pub struct OccupiedEntry<'a, T> {
    children: &'a mut Children<T>,
    idx: u8,
    pos: usize,
}

/// A view into a vacant slot of a [`Children`]. Returned as a variant of
/// [`Entry`].
pub struct VacantEntry<'a, T> {
    children: &'a mut Children<T>,
    idx: u8,
    pos: usize,
}

impl<'a, T> Entry<'a, T> {
    pub(super) fn new(children: &'a mut Children<T>, idx: u8) -> Self {
        let pos = position(&children.which, idx);
        if bit_get(&children.which, idx) {
            Entry::Occupied(OccupiedEntry { children, idx, pos })
        } else {
            Entry::Vacant(VacantEntry { children, idx, pos })
        }
    }

    /// Ensure a value is in the slot by inserting `default` if vacant. Returns
    /// a mutable reference to the value either way.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let mut c: Children<i32> = Children::new();
    /// *c.entry(7).or_insert(0) += 1;
    /// *c.entry(7).or_insert(0) += 1;
    /// assert_eq!(c.get(7), Some(&2));
    /// ```
    pub fn or_insert(self, default: T) -> &'a mut T {
        match self {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => e.insert(default),
        }
    }

    /// Ensure a value is in the slot, computing the default lazily via `f`
    /// only if vacant.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let mut c: Children<String> = Children::new();
    /// let s = c.entry(0).or_insert_with(|| "hello".to_string());
    /// assert_eq!(s, "hello");
    /// ```
    pub fn or_insert_with<F: FnOnce() -> T>(self, f: F) -> &'a mut T {
        match self {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => e.insert(f()),
        }
    }

    /// Ensure a value is in the slot, inserting `T::default()` if vacant.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let mut c: Children<i32> = Children::new();
    /// *c.entry(0).or_default() += 5;
    /// assert_eq!(c.get(0), Some(&5));
    /// ```
    pub fn or_default(self) -> &'a mut T
    where
        T: Default,
    {
        self.or_insert_with(T::default)
    }

    /// Apply `f` to the value if the slot is occupied, then return the entry
    /// for further chaining. Vacant entries pass through unchanged.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let mut c = Children::new();
    /// c.insert(0, 1);
    /// c.entry(0).and_modify(|v| *v += 10);
    /// c.entry(1).and_modify(|v| *v += 10);
    /// assert_eq!(c.get(0), Some(&11));
    /// assert_eq!(c.get(1), None);
    /// ```
    pub fn and_modify<F: FnOnce(&mut T)>(mut self, f: F) -> Self {
        if let Entry::Occupied(ref mut e) = self {
            f(e.get_mut());
        }
        self
    }

    /// The index this entry refers to, occupied or not.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::Children;
    ///
    /// let mut c: Children<()> = Children::new();
    /// assert_eq!(c.entry(42).key(), 42);
    /// ```
    pub fn key(&self) -> u8 {
        match self {
            Entry::Occupied(e) => e.idx,
            Entry::Vacant(e) => e.idx,
        }
    }
}

impl<'a, T> OccupiedEntry<'a, T> {
    /// The index of this occupied entry.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::{Children, Entry};
    ///
    /// let mut c = Children::new();
    /// c.insert(7, "x");
    /// if let Entry::Occupied(e) = c.entry(7) {
    ///     assert_eq!(e.key(), 7);
    /// }
    /// ```
    pub fn key(&self) -> u8 {
        self.idx
    }

    /// Borrow the value.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::{Children, Entry};
    ///
    /// let mut c = Children::new();
    /// c.insert(0, 99);
    /// if let Entry::Occupied(e) = c.entry(0) {
    ///     assert_eq!(e.get(), &99);
    /// }
    /// ```
    pub fn get(&self) -> &T {
        &self.children.what[self.pos]
    }

    /// Mutably borrow the value, with the lifetime of `self`. To extend the
    /// borrow to the lifetime of the underlying [`Children`], use
    /// [`OccupiedEntry::into_mut`].
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::{Children, Entry};
    ///
    /// let mut c = Children::new();
    /// c.insert(0, 1);
    /// if let Entry::Occupied(mut e) = c.entry(0) {
    ///     *e.get_mut() += 10;
    /// }
    /// assert_eq!(c.get(0), Some(&11));
    /// ```
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.children.what[self.pos]
    }

    /// Consume the entry and return a mutable reference to the value with the
    /// lifetime of the underlying [`Children`] borrow.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::{Children, Entry};
    ///
    /// let mut c = Children::new();
    /// c.insert(0, 1);
    /// if let Entry::Occupied(e) = c.entry(0) {
    ///     let v: &mut i32 = e.into_mut();
    ///     *v = 99;
    /// }
    /// assert_eq!(c.get(0), Some(&99));
    /// ```
    pub fn into_mut(self) -> &'a mut T {
        &mut self.children.what[self.pos]
    }

    /// Replace the value, returning the previous one.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::{Children, Entry};
    ///
    /// let mut c = Children::new();
    /// c.insert(0, "old");
    /// if let Entry::Occupied(mut e) = c.entry(0) {
    ///     assert_eq!(e.insert("new"), "old");
    /// }
    /// assert_eq!(c.get(0), Some(&"new"));
    /// ```
    pub fn insert(&mut self, value: T) -> T {
        std::mem::replace(self.get_mut(), value)
    }

    /// Remove the value from the [`Children`] and return it. May trigger an
    /// automatic shrink of the inner `Vec`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::{Children, Entry};
    ///
    /// let mut c = Children::new();
    /// c.insert(0, "x");
    /// if let Entry::Occupied(e) = c.entry(0) {
    ///     assert_eq!(e.remove(), "x");
    /// }
    /// assert!(!c.contains(0));
    /// ```
    pub fn remove(self) -> T {
        bit_clear(&mut self.children.which, self.idx);
        let v = self.children.what.remove(self.pos);
        self.children.maybe_shrink();
        v
    }
}

impl<'a, T> VacantEntry<'a, T> {
    /// The index this vacant entry would occupy.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::{Children, Entry};
    ///
    /// let mut c: Children<()> = Children::new();
    /// if let Entry::Vacant(e) = c.entry(99) {
    ///     assert_eq!(e.key(), 99);
    /// }
    /// ```
    pub fn key(&self) -> u8 {
        self.idx
    }

    /// Insert `value` into the [`Children`] and return a mutable reference to
    /// it with the lifetime of the underlying borrow.
    ///
    /// # Examples
    ///
    /// ```
    /// use rumors::children::{Children, Entry};
    ///
    /// let mut c: Children<i32> = Children::new();
    /// if let Entry::Vacant(e) = c.entry(0) {
    ///     *e.insert(7) += 1;
    /// }
    /// assert_eq!(c.get(0), Some(&8));
    /// ```
    pub fn insert(self, value: T) -> &'a mut T {
        let pos = self.pos;
        self.children.insert_at_position(self.idx, pos, value);
        &mut self.children.what[pos]
    }
}
