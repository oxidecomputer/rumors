//! Lazy balanced partitioning for [`Party::forks`].
//!
//! Forking a region into `n` shares forms a nearly complete binary tree. Let
//! `d = floor(log₂ n)` and `r = n - 2ᵈ`. Every path of length `d` names one
//! base share; exactly `r` of those shares fork once more. Recursively placing
//! the larger half on the left makes base path `q` fork precisely when the
//! low-`d`-bit reversal of `q` is less than `r`.
//!
//! [`Plan`] stores that compact plan and a read-only source snapshot. It
//! composes every fork for one share into a single descent, without constructing
//! the intermediate regions. [`PartyForks`] then removes that share from the
//! borrowed party in one rebuilding pass. The removal remembers bits per open
//! ancestor, not a machine word or tree node, so even an arbitrary-width count
//! cannot turn path depth into disproportionate transient memory. The borrowed
//! party always owns the residual and every untaken share; dropping the
//! iterator requires no cleanup.

use num_bigint::BigUint;

use super::Party;
use crate::party::io::{PartyBranch, PartyNode, PartyReader, PartySnapshot};
use crate::Count;

/// The current share's directions in the party's spatial tree.
///
/// The balanced fork plan describes only choices that fork an owned region.
/// The source party may also contain one-child branches, which locate that
/// region but do not fork it. [`SharePath`] merges the two: it yields every
/// one-child direction, consumes one planned choice at each two-child branch,
/// and uses any choices remaining below an owned region to create deeper
/// levels. The resulting path addresses the exact subtree that
/// [`PartyReader::remove_path`] removes.
#[must_use = "iterators are lazy and do nothing unless consumed"]
struct SharePath<'a> {
    /// Current source node while the path remains inside the original tree.
    source: PartyReader<'a>,
    /// Current base-share index, read most-significant decision first.
    index: &'a BigUint,
    /// Number of base-path decisions.
    depth: u64,
    /// Whether this base share is forked once more.
    forks_again: bool,
    /// The final child decision when [`forks_again`](Self::forks_again) is set.
    second: bool,
    /// Number of planned fork directions already consumed.
    decision: u64,
    /// Whether the path has descended below a source owned region.
    below_owned_region: bool,
}

/// Reads directions from the compact fork plan.
impl SharePath<'_> {
    /// Take the next direction from the balanced plan.
    ///
    /// The `depth` bits of `index` name the base share from root to leaf. A base
    /// share selected for one extra fork appends `second` as its final
    /// decision.
    fn next_decision(&mut self) -> Option<bool> {
        let direction = if self.decision < self.depth {
            self.index.bit(self.depth - 1 - self.decision)
        } else if self.decision == self.depth && self.forks_again {
            self.second
        } else {
            return None;
        };
        self.decision += 1;
        Some(direction)
    }
}

/// Expands ownership decisions into one direction per spatial-tree level.
impl Iterator for SharePath<'_> {
    type Item = bool;

    fn next(&mut self) -> Option<bool> {
        // Once every ownership choice is placed, the remaining source subtree
        // belongs wholly to this share. Unary source edges below this point are
        // part of that subtree, not part of the path selecting it.
        if self.decision == self.depth + u64::from(self.forks_again) {
            return None;
        }
        if self.below_owned_region {
            // The source has no more branches. Each remaining planned choice
            // divides the owned region and therefore adds one level.
            return self.next_decision();
        }
        match self.source.read() {
            PartyNode::Owned => {
                self.below_owned_region = true;
                self.next_decision()
            }
            PartyNode::Branch(PartyBranch::Both) => {
                // A two-child branch forks ownership, so place the next
                // planned choice here. The right child follows the complete
                // left subtree in preorder.
                let direction = self
                    .next_decision()
                    .expect("an unfinished fork path has a decision");
                if direction {
                    self.source.skip();
                }
                Some(direction)
            }
            PartyNode::Branch(PartyBranch::Left) => Some(false),
            PartyNode::Branch(PartyBranch::Right) => Some(true),
        }
    }
}

/// Remaining shares without an arbitrary-width duplicate of the fork count.
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
#[must_use = "iterators are lazy and do nothing unless consumed"]
struct Plan {
    /// Original party topology retained while the borrowed party changes.
    source: PartySnapshot,
    /// Depth of every base path.
    depth: u64,
    /// Number of base paths that fork once more, selected by bit reversal.
    extra: BigUint,
    /// Current base path, interpreted as `depth` binary directions.
    index: BigUint,
    /// Whether the next share is the right child of a fork base path.
    second: bool,
    /// Constant-space state for exhaustion and [`Iterator::size_hint`].
    remaining: Remaining,
}

/// Builds and advances the compact balanced-fork plan.
impl Plan {
    /// Plan a partition of `source` into `k >= 1` shares.
    fn new(source: PartySnapshot, k: Count) -> Self {
        debug_assert!(k > Count::ZERO, "a balanced fork yields at least one share");
        let remaining = Remaining::new(&k.0);
        let depth = k.0.bits() - 1;
        let mut extra = k.0;
        extra.set_bit(depth, false);
        Plan {
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
    /// At depth `d`, recursive ceil/floor forking distributes the `r` extra
    /// leaves in bit-reversal order. Comparing the reversed path to `r` one bit
    /// at a time avoids materializing either reversed integer.
    fn current_forks_again(&self) -> bool {
        for bit in 0..self.depth {
            let path = self.index.bit(bit);
            let extra = self.extra.bit(self.depth - 1 - bit);
            if path != extra {
                return !path;
            }
        }
        false
    }

    /// Expand the current share's fork decisions into a spatial-tree path.
    fn coordinate_path(&self, forks_again: bool) -> SharePath<'_> {
        SharePath {
            source: self.source.reader(),
            index: &self.index,
            depth: self.depth,
            forks_again,
            second: self.second,
            decision: 0,
            below_owned_region: false,
        }
    }

    /// Build the current share without advancing the plan.
    fn current_share(&self, forks_again: bool) -> Party {
        let path = (0..self.depth)
            .rev()
            .map(|bit| self.index.bit(bit))
            .chain(forks_again.then_some(self.second));
        self.source.reader().select_path(path)
    }

    /// Advance past the current share.
    fn advance(&mut self, forks_again: bool) {
        self.remaining.advance();
        if forks_again && !self.second {
            self.second = true;
        } else {
            self.second = false;
            self.index += 1u8;
        }
    }

    /// Advance without constructing a share.
    fn skip_one(&mut self) {
        debug_assert!(!self.is_empty());
        let forks_again = self.current_forks_again();
        self.advance(forks_again);
    }
}

/// A consuming balanced partition built from ordinary binary forks.
///
/// Each pending entry owns a complete region and the number of final shares it
/// must produce. Forking that entry once and assigning `ceil(n/2)` shares to
/// its left half and `floor(n/2)` to its right is the recursive definition of
/// the balanced partition. The stack visits left before right, so the result
/// order matches [`Plan`]. Unlike `Plan`, it never rescans the original party
/// from its root for another output: every intermediate party is consumed by
/// at most one binary fork.
#[must_use = "iterators are lazy and do nothing unless consumed"]
struct Shares {
    /// Regions still to fork, with the next region last.
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
impl Iterator for Plan {
    type Item = Party;

    fn next(&mut self) -> Option<Party> {
        if self.is_empty() {
            return None;
        }
        let forks_again = self.current_forks_again();
        let share = self.current_share(forks_again);
        self.advance(forks_again);
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
/// the borrowed party's size.
#[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscape-assets.html")))]
#[must_use = "`PartyForks` produces no child parties unless consumed"]
pub struct PartyForks<'a> {
    /// The borrowed party, containing the residual and every untaken share.
    rest: &'a mut Party,
    /// The compact plan for shares after the residual.
    plan: Plan,
}

/// Creates a borrowing fork iterator.
impl<'a> PartyForks<'a> {
    /// Borrow `party` and plan `k` children after one residual share.
    pub(crate) fn new(party: &'a mut Party, k: Count) -> Self {
        // The first of `k + 1` shares belongs to the borrowed party. Skipping
        // it in the plan leaves the party itself unchanged: until a child is
        // returned, it still owns the entire region.
        let mut count = k;
        count.0 += 1u32;
        let mut plan = Plan::new(PartySnapshot::new(party), count);
        plan.skip_one();
        PartyForks { rest: party, plan }
    }
}

/// Produces one child while leaving every untaken region with the borrower.
impl Iterator for PartyForks<'_> {
    type Item = Party;

    fn next(&mut self) -> Option<Party> {
        if self.plan.is_empty() {
            return None;
        }
        let forks_again = self.plan.current_forks_again();
        let share = self.plan.current_share(forks_again);
        // The share and residual have similar shape, so the writer uses the
        // share to reserve a suitably sized result without exposing storage
        // details to this algorithm.
        let remainder = self
            .rest
            .reader()
            .remove_path(self.plan.coordinate_path(forks_again), &share);
        *self.rest = remainder;
        self.plan.advance(forks_again);
        Some(share)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.plan.size_hint()
    }
}

#[cfg(test)]
mod tests;

/// Forks a [`Party`] into exactly `N` balanced shares, consuming it.
///
/// The static counterpart of [`forks`](Party::forks): where `forks` borrows the
/// party and leaves it holding a residual share, this consumes it entirely into
/// `N` shares whose party tree has minimal depth `⌈log₂ N⌉`. The shares
/// [`join_all`](Party::join_all) back to the original region.
///
/// # The `N >= 1` bound
///
/// A [`Party`] owns a nonempty region and cannot vanish into zero shares, so
/// `N` must be at least 1. The bound is enforced at compile time, not by a
/// runtime panic: the zero-length fork is rejected when the conversion is
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
        const { assert!(N >= 1, "a `Party` cannot fork into zero shares") }
        let mut partition = party.into_shares(N);
        // `from_fn` calls indices `0..N` in order, and the consuming traversal
        // yields in preorder, so share `i` lands at index `i` — the same order
        // `forks` hands them out.
        let shares = core::array::from_fn(|_| {
            partition
                .next()
                .expect("a fork into N shares yields exactly N leaves")
        });
        assert!(partition.next().is_none(), "the fork yielded N shares");
        shares
    }
}
