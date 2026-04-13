//! Iterators over [`Children`].

use std::iter::FusedIterator;

use super::Children;
use super::bits::{BITS_PER_WORD, WORDS};

/// Iterator over set keys of a [`Children`], in ascending order.
///
/// Constructed via [`Children::keys`]. Implements [`ExactSizeIterator`],
/// [`FusedIterator`], and [`DoubleEndedIterator`].
///
/// # Examples
///
/// ```
/// use rumors::children::Children;
///
/// let mut c: Children<()> = Children::new();
/// c.insert(0, ());
/// c.insert(255, ());
///
/// let forward: Vec<u8> = c.keys().collect();
/// assert_eq!(forward, vec![0, 255]);
///
/// let reverse: Vec<u8> = c.keys().rev().collect();
/// assert_eq!(reverse, vec![255, 0]);
/// ```
#[derive(Clone)]
pub struct Keys {
    which: [u64; WORDS],
}

impl Keys {
    pub(super) fn new(which: [u64; WORDS]) -> Self {
        Self { which }
    }
}

impl Iterator for Keys {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        for word in 0..WORDS {
            let w = self.which[word];
            if w != 0 {
                let bit = w.trailing_zeros() as u8;
                // Clear the lowest set bit.
                self.which[word] = w & (w - 1);
                return Some(word as u8 * BITS_PER_WORD + bit);
            }
        }
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n: u32 = self.which.iter().map(|w| w.count_ones()).sum();
        (n as usize, Some(n as usize))
    }
}

impl DoubleEndedIterator for Keys {
    fn next_back(&mut self) -> Option<u8> {
        for word in (0..WORDS).rev() {
            let w = self.which[word];
            if w != 0 {
                // Highest set bit position within the word.
                let bit = (BITS_PER_WORD - 1) - w.leading_zeros() as u8;
                self.which[word] = w & !(1u64 << bit);
                return Some(word as u8 * BITS_PER_WORD + bit);
            }
        }
        None
    }
}

impl ExactSizeIterator for Keys {}
impl FusedIterator for Keys {}

/// Iterator yielding `(index, &value)` pairs in ascending index order.
///
/// Constructed via [`Children::iter`] or `(&Children).into_iter()`.
/// Implements [`ExactSizeIterator`], [`FusedIterator`], and
/// [`DoubleEndedIterator`].
///
/// # Examples
///
/// ```
/// use rumors::children::Children;
///
/// let mut c = Children::new();
/// c.insert(2, "two");
/// c.insert(1, "one");
///
/// let pairs: Vec<(u8, &&str)> = c.iter().collect();
/// assert_eq!(pairs, vec![(1, &"one"), (2, &"two")]);
///
/// let reversed: Vec<(u8, &&str)> = c.iter().rev().collect();
/// assert_eq!(reversed, vec![(2, &"two"), (1, &"one")]);
/// ```
pub struct Iter<'a, T> {
    keys: Keys,
    what: std::slice::Iter<'a, T>,
}

impl<'a, T> Iter<'a, T> {
    pub(super) fn new(which: [u64; WORDS], what: std::slice::Iter<'a, T>) -> Self {
        Self {
            keys: Keys::new(which),
            what,
        }
    }
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (u8, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        Some((self.keys.next()?, self.what.next()?))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.what.size_hint()
    }
}

impl<T> DoubleEndedIterator for Iter<'_, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        Some((self.keys.next_back()?, self.what.next_back()?))
    }
}

impl<T> ExactSizeIterator for Iter<'_, T> {}
impl<T> FusedIterator for Iter<'_, T> {}

/// Iterator yielding `(index, &mut value)` pairs in ascending index order.
///
/// Constructed via [`Children::iter_mut`] or `(&mut Children).into_iter()`.
/// Implements [`ExactSizeIterator`], [`FusedIterator`], and
/// [`DoubleEndedIterator`].
///
/// # Examples
///
/// ```
/// use rumors::children::Children;
///
/// let mut c = Children::new();
/// c.insert(0, 1);
/// c.insert(1, 2);
/// for (_, v) in c.iter_mut().rev() {
///     *v *= 10;
/// }
/// assert_eq!(c.get(0), Some(&10));
/// assert_eq!(c.get(1), Some(&20));
/// ```
pub struct IterMut<'a, T> {
    keys: Keys,
    what: std::slice::IterMut<'a, T>,
}

impl<'a, T> IterMut<'a, T> {
    pub(super) fn new(which: [u64; WORDS], what: std::slice::IterMut<'a, T>) -> Self {
        Self {
            keys: Keys::new(which),
            what,
        }
    }
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = (u8, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        Some((self.keys.next()?, self.what.next()?))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.what.size_hint()
    }
}

impl<T> DoubleEndedIterator for IterMut<'_, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        Some((self.keys.next_back()?, self.what.next_back()?))
    }
}

impl<T> ExactSizeIterator for IterMut<'_, T> {}
impl<T> FusedIterator for IterMut<'_, T> {}

/// Owning iterator yielding `(index, value)` pairs in ascending index order.
///
/// Constructed via [`IntoIterator::into_iter`] on an owned `Children`.
/// Implements [`ExactSizeIterator`], [`FusedIterator`], and
/// [`DoubleEndedIterator`].
///
/// # Examples
///
/// ```
/// use rumors::children::Children;
///
/// let mut c = Children::new();
/// c.insert(0, "a".to_string());
/// c.insert(1, "b".to_string());
///
/// let v: Vec<(u8, String)> = c.into_iter().rev().collect();
/// assert_eq!(v, vec![(1, "b".into()), (0, "a".into())]);
/// ```
pub struct IntoIter<T> {
    keys: Keys,
    what: std::vec::IntoIter<T>,
}

impl<T> IntoIter<T> {
    fn new(which: [u64; WORDS], what: std::vec::IntoIter<T>) -> Self {
        Self {
            keys: Keys::new(which),
            what,
        }
    }
}

impl<T> Iterator for IntoIter<T> {
    type Item = (u8, T);

    fn next(&mut self) -> Option<Self::Item> {
        Some((self.keys.next()?, self.what.next()?))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.what.size_hint()
    }
}

impl<T> DoubleEndedIterator for IntoIter<T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        Some((self.keys.next_back()?, self.what.next_back()?))
    }
}

impl<T> ExactSizeIterator for IntoIter<T> {}
impl<T> FusedIterator for IntoIter<T> {}

impl<T> IntoIterator for Children<T> {
    type Item = (u8, T);
    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter::new(self.which, self.what.into_iter())
    }
}

impl<'a, T> IntoIterator for &'a Children<T> {
    type Item = (u8, &'a T);
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut Children<T> {
    type Item = (u8, &'a mut T);
    type IntoIter = IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

/// Draining iterator yielding `(index, value)` pairs in ascending index
/// order. Constructed via [`Children::drain`].
///
/// When the iterator is dropped (or fully consumed), the source [`Children`]
/// is left empty with its inner `Vec` capacity preserved. Implements
/// [`ExactSizeIterator`], [`FusedIterator`], and [`DoubleEndedIterator`].
///
/// # Examples
///
/// ```
/// use rumors::children::Children;
///
/// let mut c = Children::new();
/// c.insert(0, "a");
/// c.insert(1, "b");
///
/// let drained: Vec<(u8, &str)> = c.drain().rev().collect();
/// assert_eq!(drained, vec![(1, "b"), (0, "a")]);
/// assert!(c.is_empty());
/// ```
pub struct Drain<'a, T> {
    keys: Keys,
    what: std::vec::Drain<'a, T>,
}

impl<'a, T> Drain<'a, T> {
    pub(super) fn new(which: [u64; WORDS], what: std::vec::Drain<'a, T>) -> Self {
        Self {
            keys: Keys::new(which),
            what,
        }
    }
}

impl<T> Iterator for Drain<'_, T> {
    type Item = (u8, T);

    fn next(&mut self) -> Option<Self::Item> {
        Some((self.keys.next()?, self.what.next()?))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.what.size_hint()
    }
}

impl<T> DoubleEndedIterator for Drain<'_, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        Some((self.keys.next_back()?, self.what.next_back()?))
    }
}

impl<T> ExactSizeIterator for Drain<'_, T> {}
impl<T> FusedIterator for Drain<'_, T> {}
