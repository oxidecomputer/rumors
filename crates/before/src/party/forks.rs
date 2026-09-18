//! Lazy balanced partitioning for [`Party::forks`].
//!
//! Splitting a region into `n` shares forms a nearly complete binary tree. Let
//! `d = floor(log₂ n)` and `r = n - 2ᵈ`. Every path of length `d` names one
//! base share; exactly `r` of those shares split once more. Recursively placing
//! the larger half on the left makes base path `q` split precisely when the
//! low-`d`-bit reversal of `q` is less than `r`.
//!
//! [`Split`] stores that compact plan and the source bytes. It composes every
//! split for one share into a single descent, without constructing the
//! intermediate regions. [`Forks`] then removes that share from the borrowed
//! party in one rebuilding pass. The removal remembers bits per open ancestor,
//! not a machine word or tree node, so even an arbitrary-width count cannot
//! turn path depth into disproportionate transient memory. The borrowed party
//! always owns the residual and every untaken share; dropping the iterator
//! requires no cleanup.

use num_bigint::BigUint;

use super::Party;
use crate::codec;
use crate::idbits::IdReader;
use crate::Ticks;

/// The current share's directions in the party's spatial tree.
///
/// The balanced split plan describes only choices that divide an owned region.
/// The source encoding may also contain unary nodes, which locate that region
/// in the spatial tree but do not divide it. [`CoordinatePath`] merges the two:
/// it yields every unary source direction, consumes one planned choice at each
/// source branch, and uses any choices remaining below a source terminal to
/// create deeper levels. The resulting path addresses the exact subtree that
/// [`IdReader::remove_path`] must remove.
struct CoordinatePath<'a> {
    /// Source bits whose unary nodes contribute spatial directions.
    bits: codec::BitsView<'a>,
    /// Current source node while the path remains inside the original tree.
    pos: u64,
    /// Current base-share index, read most-significant decision first.
    index: &'a BigUint,
    /// Number of base-path decisions.
    depth: u64,
    /// Whether this base share is divided once more.
    splits: bool,
    /// The final child decision when [`splits`](Self::splits) is set.
    second: bool,
    /// Number of ownership-dividing decisions already consumed.
    decision: u64,
    /// Whether the path has descended below a source terminal.
    below_terminal: bool,
}

/// Reads ownership decisions from the compact split plan.
impl CoordinatePath<'_> {
    /// Take the next ownership-dividing decision from the balanced plan.
    ///
    /// The `depth` bits of `index` name the base share from root to leaf. A base
    /// share selected for one extra division appends `second` as its final
    /// decision.
    fn next_decision(&mut self) -> Option<bool> {
        let direction = if self.decision < self.depth {
            self.index.bit(self.depth - 1 - self.decision)
        } else if self.decision == self.depth && self.splits {
            self.second
        } else {
            return None;
        };
        self.decision += 1;
        Some(direction)
    }
}

/// Expands ownership decisions into one direction per spatial-tree level.
impl Iterator for CoordinatePath<'_> {
    type Item = bool;

    fn next(&mut self) -> Option<bool> {
        // Once every ownership choice is placed, the remaining source subtree
        // belongs wholly to this share. Unary source edges below this point are
        // part of that subtree, not part of the path selecting it.
        if self.decision == self.depth + u64::from(self.splits) {
            return None;
        }
        if self.below_terminal {
            // The source has no more topology. Each remaining planned choice
            // conceptually divides the terminal and therefore adds one level.
            return self.next_decision();
        }
        crate::codec::scan::record_bits(2);
        let (left, right) = (self.bits.bit(self.pos), self.bits.bit(self.pos + 1));
        if !left && !right {
            self.below_terminal = true;
            return self.next_decision();
        }
        let child = self.pos + 2;
        if left && right {
            // A true branch divides ownership, so place the next planned
            // choice here. Preorder requires skipping left to locate right.
            let direction = self
                .next_decision()
                .expect("an unfinished split path has a decision");
            self.pos = if direction {
                let mut left = IdReader::at(self.bits, child);
                left.skip();
                left.pos()
            } else {
                child
            };
            return Some(direction);
        }
        // A unary node only locates the owned region. Preserve its direction
        // without consuming a choice from the balanced plan.
        let direction = right;
        self.pos = child;
        Some(direction)
    }
}

/// Remaining shares without an arbitrary-width duplicate of the split count.
enum Remaining {
    /// The exact count.
    Exact(usize),
    /// Steps before the exact count becomes `usize::MAX`.
    Near(usize),
    /// Too far above `usize::MAX` to track in one machine word.
    Distant,
}

/// Maintains a useful sound `usize` size hint in constant space.
impl Remaining {
    /// Classify an arbitrary-width initial count.
    fn new(count: &BigUint) -> Self {
        if let Ok(exact) = usize::try_from(count) {
            return Remaining::Exact(exact);
        }
        let Ok(count) = u128::try_from(count) else {
            return Remaining::Distant;
        };
        let excess = count - usize::MAX as u128;
        usize::try_from(excess).map_or(Remaining::Distant, Remaining::Near)
    }

    /// Account for one produced share.
    fn advance(&mut self) {
        *self = match *self {
            Remaining::Exact(remaining) => Remaining::Exact(remaining - 1),
            Remaining::Near(1) => Remaining::Exact(usize::MAX),
            Remaining::Near(excess) => Remaining::Near(excess - 1),
            Remaining::Distant => Remaining::Distant,
        };
    }

    /// Whether an exactly tracked plan is exhausted.
    fn is_empty(&self) -> bool {
        matches!(self, Remaining::Exact(0))
    }

    /// The strongest sound iterator hint represented by this state.
    fn size_hint(&self) -> (usize, Option<usize>) {
        match *self {
            Remaining::Exact(remaining) => (remaining, Some(remaining)),
            Remaining::Near(_) => (usize::MAX, None),
            Remaining::Distant => (0, None),
        }
    }
}

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
    /// Constant-space state for exhaustion and [`Iterator::size_hint`].
    remaining: Remaining,
}

/// Builds and advances the compact balanced-split plan.
impl Split {
    /// Plan a partition of `source` into `k >= 1` shares.
    fn new(source: codec::Bits, k: Ticks) -> Self {
        debug_assert!(
            k > Ticks::ZERO,
            "a balanced split yields at least one share"
        );
        let remaining = Remaining::new(&k.0);
        let depth = k.0.bits() - 1;
        let mut extra = k.0;
        extra.set_bit(depth, false);
        Split {
            source,
            depth,
            extra,
            index: BigUint::ZERO,
            second: false,
            remaining,
        }
    }

    /// Whether every planned share has been produced.
    fn is_empty(&self) -> bool {
        self.remaining.is_empty()
            // Distant counts deliberately remain inexact. The base-path index
            // still identifies their exact end.
            || (matches!(self.remaining, Remaining::Distant) && self.index.bit(self.depth))
    }

    /// Whether at least `usize::MAX` base paths remain in a distant plan.
    ///
    /// With `B = 2^depth` base paths and current index `q`, the condition is
    /// `q <= B - usize::MAX`. The boundary's bits are all ones above the low
    /// machine word and the value one within it, so the comparison needs no
    /// arbitrary-width temporary.
    fn has_saturated_lower_bound(&self) -> bool {
        debug_assert!(self.depth >= u64::from(usize::BITS));
        let word_bits = u64::from(usize::BITS);
        let high_bits_are_max = (word_bits..self.depth).all(|bit| self.index.bit(bit));
        !high_bits_are_max || !(1..word_bits).any(|bit| self.index.bit(bit))
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

    /// Expand the current share's split decisions into a spatial-tree path.
    fn coordinate_path(&self, splits: bool) -> CoordinatePath<'_> {
        CoordinatePath {
            bits: self.source.live(),
            pos: 0,
            index: &self.index,
            depth: self.depth,
            splits,
            second: self.second,
            decision: 0,
            below_terminal: false,
        }
    }

    /// Build the current share without advancing the plan.
    fn current_share(&self, splits: bool) -> Party {
        let path = (0..self.depth)
            .rev()
            .map(|bit| self.index.bit(bit))
            .chain(splits.then_some(self.second));
        Party::from_bits(IdReader::root(self.source.live()).split_path(path))
    }

    /// Advance past the current share.
    fn advance(&mut self, splits: bool) {
        self.remaining.advance();
        if splits && !self.second {
            self.second = true;
        } else {
            self.second = false;
            self.index += 1u8;
        }
    }

    /// Advance without constructing a share.
    fn skip_one(&mut self) {
        debug_assert!(!self.is_empty());
        let splits = self.current_splits();
        self.advance(splits);
    }
}

/// A consuming balanced partition built from ordinary binary splits.
///
/// Each pending entry owns a complete region and the number of final shares it
/// must produce. Splitting that entry once and assigning `ceil(n/2)` shares to
/// its left half and `floor(n/2)` to its right is the recursive definition of
/// the balanced partition. The stack visits left before right, so the result
/// order matches [`Split`]. Unlike `Split`, it never rescans the original party
/// from its root for another output: every intermediate party is consumed by
/// at most one binary split.
struct Shares {
    /// Regions still to divide, with the next region last.
    pending: Vec<(Party, usize)>,
}

/// Produces all shares of an owned party without maintaining a residual.
impl Iterator for Shares {
    type Item = Party;

    fn next(&mut self) -> Option<Party> {
        while let Some((mut party, count)) = self.pending.pop() {
            if count == 1 {
                return Some(party);
            }

            let right = party.fork();
            // Push right first so the LIFO walk completes the left partition
            // before it enters the right partition.
            self.pending.push((right, count / 2));
            self.pending.push((party, count.div_ceil(2)));
        }
        None
    }
}

impl Party {
    /// Consume this party into `count` balanced shares.
    pub(crate) fn into_shares(self, count: usize) -> impl Iterator<Item = Party> {
        assert!(count > 0, "a party yields at least one share");
        Shares {
            pending: vec![(self, count)],
        }
    }
}

/// Produces the plan's balanced shares in preorder.
impl Iterator for Split {
    type Item = Party;

    fn next(&mut self) -> Option<Party> {
        if self.is_empty() {
            return None;
        }
        let splits = self.current_splits();
        let share = self.current_share(splits);
        self.advance(splits);
        Some(share)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.is_empty() {
            (0, Some(0))
        } else if matches!(self.remaining, Remaining::Distant) && self.has_saturated_lower_bound() {
            (usize::MAX, None)
        } else {
            self.remaining.size_hint()
        }
    }
}

/// A lazy iterator of balanced [`Party`] shares, returned by [`Party::forks`].
///
/// Yields exactly `k` disjoint shares produced one at a time. The party it
/// borrows keeps the residual and every share not yet returned.
///
/// [`Iterator::size_hint`] is exact for initial counts fitting `usize`. Wider
/// counts report a sound lower bound and no upper bound.
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

/// Creates a borrowing fork iterator.
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

/// Produces one child while leaving every untaken region with the borrower.
impl Iterator for Forks<'_> {
    type Item = Party;

    fn next(&mut self) -> Option<Party> {
        if self.split.is_empty() {
            return None;
        }
        let splits = self.split.current_splits();
        let share = self.split.current_share(splits);
        // The share and residual reach the same split depth, so the share's
        // encoded size is a useful capacity hint when a wide count created
        // most of that path.
        let remainder = self
            .rest
            .view()
            .remove_path(self.split.coordinate_path(splits), share.0.len());
        assert!(
            !remainder.is_empty(),
            "the reserved residual keeps a fork iterator's party nonempty"
        );
        *self.rest = Party::from_bits(remainder);
        self.split.advance(splits);
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
/// Consumes a party into a statically sized balanced partition.
impl<const N: usize> From<Party> for [Party; N] {
    fn from(party: Party) -> [Party; N] {
        // Fires at monomorphization, making `N == 0` a build error. The paired
        // doctests above pin it: the `compile_fail` twin must be rejected while
        // its identical-but-for-arity sibling compiles.
        const { assert!(N >= 1, "a `Party` cannot split into zero shares") }
        let mut partition = party.into_shares(N);
        // `from_fn` calls indices `0..N` in order, and the consuming traversal
        // yields in preorder, so share `i` lands at index `i` — the same order
        // `forks` hands them out.
        let shares = core::array::from_fn(|_| {
            partition
                .next()
                .expect("a split into N shares yields exactly N leaves")
        });
        assert!(partition.next().is_none(), "the split yielded N shares");
        shares
    }
}
