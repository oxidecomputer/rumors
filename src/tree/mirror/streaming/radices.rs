//! Sets of radices: which of a node's child positions a side holds.
//!
//! A child sits under its parent at a *radix*, the byte that extends the
//! parent's prefix to the child's, so a node has at most a fan of 256
//! children, each at its own radix. A set of 256 bits therefore represents
//! one side's children of a node exactly, and the children either of two
//! sides holds are one union of two sets. A session prices an answer at the
//! size of that union before it explodes the node
//! ([`ErasedNode::child_radices`](super::ErasedNode::child_radices)).

#[cfg(test)]
mod tests;

/// The bits in one word of a [`Radices`].
const WORD_BITS: usize = u64::BITS as usize;

/// A set of radices, one bit for each possible child of a node.
///
/// The [`Default`] set is empty.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Radices([u64; 4]);

/// Four words of bits hold one bit for every radix a byte can name.
const _: () = assert!(4 * WORD_BITS == u8::MAX as usize + 1);

impl Radices {
    /// The word holding `radix`'s bit, and the bit within it.
    fn position(radix: u8) -> (usize, u64) {
        let radix = usize::from(radix);
        (radix / WORD_BITS, 1 << (radix % WORD_BITS))
    }

    /// Add `radix` to the set.
    pub fn insert(&mut self, radix: u8) {
        let (word, bit) = Self::position(radix);
        self.0[word] |= bit;
    }

    /// The number of radices in the set: at most a fan.
    pub fn len(self) -> usize {
        self.0.iter().map(|word| word.count_ones() as usize).sum()
    }

    /// The radices in this set or in `other`.
    pub fn union(self, other: Self) -> Self {
        let mut words = self.0;
        for (word, theirs) in words.iter_mut().zip(other.0) {
            *word |= theirs;
        }
        Self(words)
    }
}

/// Collect radices into a set; a radix collected twice is held once.
impl FromIterator<u8> for Radices {
    fn from_iter<I: IntoIterator<Item = u8>>(radices: I) -> Self {
        let mut set = Self::default();
        for radix in radices {
            set.insert(radix);
        }
        set
    }
}
