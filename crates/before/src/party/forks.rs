//! Lazy balanced partitioning for [`Party::forks`].
//!
//! Splitting a region into `n` shares forms a nearly complete binary tree. Let
//! `d = floor(log₂ n)` and `r = n - 2ᵈ`. Every path of length `d` names one
//! base share; exactly `r` of those shares split once more. Recursively placing
//! the larger half on the left makes base path `q` split precisely when the
//! low-`d`-bit reversal of `q` is less than `r`.
//!
//! [`Split`] stores that compact plan and the source bytes. It builds a share
//! by descending its final path without restarting from the root, rather than
//! retaining or constructing the intermediate split regions. [`Forks`] then
//! carves each returned share from the borrowed party. The borrowed party
//! therefore always owns the residual and every untaken share; dropping the
//! iterator requires no cleanup.

use num_bigint::BigUint;

use super::Party;
use crate::codec;
use crate::idbits::IdReader;
use crate::Ticks;

/// A compact plan that yields one balanced share at a time in preorder.
struct Split {
    /// Original party bytes shared with the borrowed party where possible.
    source: codec::Bits,
    /// Depth of every base path.
    depth: u64,
    /// Number of base paths that split once more, selected by bit reversal.
    extra: BigUint,
    /// Current base path, interpreted as `depth` binary directions.
    index: BigUint,
    /// Whether the next share is the right child of a split base path.
    second: bool,
    /// Exact number of shares left to produce.
    remaining: Ticks,
}

impl Split {
    /// Plan a partition of `source` into `k >= 1` shares.
    fn new(source: codec::Bits, k: Ticks) -> Self {
        debug_assert!(
            k > Ticks::ZERO,
            "a balanced split yields at least one share"
        );
        let depth = k.0.bits() - 1;
        let mut extra = k.0.clone();
        extra.set_bit(depth, false);
        Split {
            source,
            depth,
            extra,
            index: BigUint::ZERO,
            second: false,
            remaining: k,
        }
    }

    /// Whether the current base path has two leaf children.
    ///
    /// At depth `d`, recursive ceil/floor splitting distributes the `r` extra
    /// leaves in bit-reversal order. Comparing the reversed path to `r` one bit
    /// at a time avoids materializing either reversed integer.
    fn current_splits(&self) -> bool {
        for bit in 0..self.depth {
            let path = self.index.bit(bit);
            let extra = self.extra.bit(self.depth - 1 - bit);
            if path != extra {
                return !path;
            }
        }
        false
    }

    /// Advance past the current share.
    fn advance(&mut self, splits: bool) {
        self.remaining.0 -= 1u8;
        if splits && !self.second {
            self.second = true;
        } else {
            self.second = false;
            self.index += 1u8;
        }
    }

    /// Advance without constructing a share.
    fn skip_one(&mut self) {
        debug_assert!(self.remaining > Ticks::ZERO);
        let splits = self.current_splits();
        self.advance(splits);
    }
}

impl Iterator for Split {
    type Item = Party;

    fn next(&mut self) -> Option<Party> {
        if self.remaining == Ticks::ZERO {
            return None;
        }
        let splits = self.current_splits();
        let path = (0..self.depth)
            .rev()
            .map(|bit| self.index.bit(bit))
            .chain(splits.then_some(self.second));
        let bits = IdReader::root(self.source.live()).split_path(path);
        self.advance(splits);
        Some(Party::from_bits(bits))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        // Rust denominates iterator hints in `usize`. Past that range,
        // `usize::MAX` is still a valid lower bound and no finite upper bound
        // can be represented.
        match usize::try_from(&self.remaining.0) {
            Ok(n) => (n, Some(n)),
            Err(_) => (usize::MAX, None),
        }
    }
}

/// A lazy iterator of balanced [`Party`] shares, returned by [`Party::forks`].
///
/// Yields exactly `k` disjoint shares produced one at a time. The party it
/// borrows keeps the residual and every share not yet returned.
///
/// [`Iterator::size_hint`] is exact while the remaining count fits `usize`;
/// beyond `usize::MAX`, it returns `(usize::MAX, None)`.
///
/// # Complexity
///
#[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/party_forks.html")))]
#[cfg_attr(
    not(doc),
    doc = "`O(n)` in total input bytes; a full drain costs `O(|self| + k (|self| + log k))`"
)]
///
/// Construction costs `O(log k)` for the count representation. Each `next`
/// costs `O(|p| + log k)` and dropping the iterator costs `O(1)`, with `|p|`
/// the borrowed party's encoded size.
#[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscape-assets.html")))]
pub struct Forks<'a> {
    /// The borrowed party, containing the residual and every untaken share.
    rest: &'a mut Party,
    /// The compact plan for shares after the residual.
    split: Split,
}

impl<'a> Forks<'a> {
    /// Borrow `party` and plan `k` children after one residual share.
    pub(crate) fn new(party: &'a mut Party, k: Ticks) -> Self {
        // The first of `k + 1` shares belongs to the borrowed party. Skipping
        // it in the plan leaves the party itself unchanged: until a child is
        // returned, it still owns the entire unsplit region.
        let mut count = k;
        count.0 += 1u32;
        let mut split = Split::new(party.0.clone(), count);
        split.skip_one();
        Forks { rest: party, split }
    }
}

impl Iterator for Forks<'_> {
    type Item = Party;

    fn next(&mut self) -> Option<Party> {
        let share = self.split.next()?;
        let remainder = self.rest.view().diff(share.view());
        assert!(
            !remainder.is_empty(),
            "the reserved residual keeps a fork iterator's party nonempty"
        );
        *self.rest = Party::from_bits(remainder);
        Some(share)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.split.size_hint()
    }
}

#[cfg(test)]
mod tests;

/// Splits a [`Party`] into exactly `N` balanced shares, consuming it.
///
/// The static counterpart of [`forks`](Party::forks): where `forks` borrows the
/// party and leaves it holding a residual share, this consumes it entirely into
/// `N` shares whose id tree has minimal depth `⌈log₂ N⌉`. The shares
/// [`join_all`](Party::join_all) back to the original region.
///
/// # The `N >= 1` bound
///
/// A [`Party`] owns a nonempty region and cannot vanish into zero shares, so
/// `N` must be at least 1. The bound is enforced at compile time, not by a
/// runtime panic: the zero-length split is rejected when the conversion is
/// built, while the same spelling at any nonzero arity compiles and runs.
///
/// ```
/// use before::Party;
/// let _shares: [Party; 1] = Party::seed().into();
/// ```
///
/// ```compile_fail,E0080
/// use before::Party;
/// let _shares: [Party; 0] = Party::seed().into();
/// ```
///
/// # Complexity
///
#[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/party_forks.html")))]
#[cfg_attr(
    not(doc),
    doc = "`O(n)` in total input bytes; a full drain costs `O(|self| + k (|self| + log k))`"
)]
///
/// # Example
///
/// ```
/// use before::Party;
/// let [a, b, c]: [Party; 3] = Party::seed().into();
/// assert!(a.is_disjoint(&b) && b.is_disjoint(&c) && a.is_disjoint(&c));
/// ```
impl<const N: usize> From<Party> for [Party; N] {
    fn from(party: Party) -> [Party; N] {
        // Fires at monomorphization, making `N == 0` a build error. The paired
        // doctests above pin it: the `compile_fail` twin must be rejected while
        // its identical-but-for-arity sibling compiles.
        const { assert!(N >= 1, "a `Party` cannot split into zero shares") }
        let mut split = Split::new(party.0, N.into());
        // `from_fn` calls indices `0..N` in order, and `Split` yields in
        // preorder, so share `i` lands at index `i` — the same order `forks`
        // hands them out.
        let shares = core::array::from_fn(|_| {
            split
                .next()
                .expect("a split into N shares yields exactly N leaves")
        });
        debug_assert!(split.remaining == Ticks::ZERO, "the split yielded N shares");
        shares
    }
}
