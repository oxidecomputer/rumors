//! Balanced n-way fork: [`Party::forks`] and its [`Forks`] iterator, plus the
//! consuming [`From<Party>`](From) for `[Party; N]` static split.
//!
//! Both rest on `Split`, which lazily divides a region into `k` shares of
//! minimal-depth (`⌈log₂ k⌉`) id tree, emitting one share per step. This is the
//! cure for the [`fork`](Party::fork) footgun: forking one party `n` times
//! deepens a single spine into a linear tree, whereas a balanced split keeps
//! every share shallow — and it does so without ever materializing the whole
//! list of shares, so a consumer collects them into its own structure with a
//! single allocation.

use core::mem;

use super::Party;

/// A lazy balanced partition: yields a region's `k` shares one at a time, in
/// preorder, each a leaf of a minimal-depth (`⌈log₂ k⌉`) id tree.
///
/// The work stack holds the not-yet-emitted subregions — the right siblings
/// along the current spine — so it is bounded by the tree depth, `O(log k)`.
/// Each [`next`](Iterator::next) descends to one leaf by
/// [`fork`](Party::fork)ing the front region, pushing its right child for later
/// and recurring left, until a single-share region remains.
struct Split {
    /// Pending subregions in emission order; the top of the stack (the last
    /// element) is produced next, and each entry still owes `count` shares.
    /// Holding the owned regions is what lets a partial read fold them back.
    stack: Vec<(Party, usize)>,
    /// Shares still to emit (`Σ count`); kept as a running total so the
    /// iterator's size is exact in `O(1)`.
    remaining: usize,
}

impl Split {
    /// A partition of `party` into `k` shares. `k >= 1`.
    fn new(party: Party, k: usize) -> Self {
        debug_assert!(k >= 1, "a balanced split yields at least one share");
        Split {
            stack: vec![(party, k)],
            remaining: k,
        }
    }
}

impl Iterator for Split {
    type Item = Party;

    fn next(&mut self) -> Option<Party> {
        let (mut region, mut count) = self.stack.pop()?;
        // Descend the left spine, forking off and stacking each right sibling,
        // until the kept region owes a single share — that region is the leaf.
        // `⌈count/2⌉` shares stay left (preorder: emitted before the right
        // child), `⌊count/2⌋` go right. The recursion of `Split` made iterative,
        // so a huge `count` cannot overflow the call stack.
        while count > 1 {
            let right = region.fork();
            let left_count = count.div_ceil(2);
            self.stack.push((right, count - left_count));
            count = left_count;
        }
        self.remaining -= 1;
        Some(region)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl ExactSizeIterator for Split {}

/// A lazy iterator of balanced [`Party`] shares, returned by [`Party::forks`].
///
/// Yields exactly `n` disjoint shares produced one at a time. The party it
/// borrows keeps the remaining share and is never left empty; any share not
/// taken before the iterator drops is [`join`](Party::join)ed back into that
/// party, so a partial read leaves the original [`Party`] holding everything it
/// did not hand out.
pub struct Forks<'a> {
    /// The borrowed party: keeps the residual share and reabsorbs unconsumed
    /// shares on drop.
    rest: &'a mut Party,
    /// The lazy partition of the `n` shares to hand out; the residual the
    /// borrowed party keeps has already been drawn off the front.
    split: Split,
}

impl<'a> Forks<'a> {
    /// Borrow `party` and reserve `n` balanced shares, leaving the residual in
    /// place. The public entry point is [`Party::forks`].
    pub(crate) fn new(party: &'a mut Party, n: usize) -> Self {
        // `n + 1`, not `n`: a Party is never empty, so `party` must retain a
        // share even once every yielded share has been consumed. The first
        // preorder leaf becomes that residual — reaching it costs O(log n)
        // forks, not the whole partition — and the same preorder governs the
        // `n` shares yielded after it, matching the consuming `From` split.
        let whole = mem::replace(party, Party::anonymous());
        let mut split = Split::new(whole, n + 1);
        *party = split
            .next()
            .expect("a split into n + 1 >= 1 shares yields a residual leaf");
        Forks { rest: party, split }
    }
}

impl Iterator for Forks<'_> {
    type Item = Party;
    fn next(&mut self) -> Option<Party> {
        self.split.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.split.size_hint()
    }
}

impl ExactSizeIterator for Forks<'_> {}

impl Drop for Forks<'_> {
    /// Fold every unconsumed share back into the borrowed party. Whatever the
    /// consumer did not take remains as coarse, un-subdivided subregions on the
    /// stack — pairwise disjoint and disjoint from the residual — so
    /// [`join`](Party::join) cannot overlap, and joining a coarse region is the
    /// same union as joining the leaves it would have split into.
    fn drop(&mut self) {
        let rest = &mut *self.rest;
        for (region, _count) in self.split.stack.drain(..) {
            rest.join(region)
                .expect("balanced-fork shares are pairwise disjoint, so rejoin cannot overlap");
        }
    }
}

/// Splits a [`Party`] into exactly `N` balanced shares, consuming it.
///
/// The static counterpart of [`forks`](Party::forks): where `forks` borrows the
/// party and leaves it holding a residual share, this consumes it entirely into
/// `N` shares whose id tree has minimal depth `⌈log₂ N⌉`. The shares
/// [`join_all`](Party::join_all) back to the original region.
///
/// `N` must be at least 1: a [`Party`] owns a nonempty region and cannot vanish
/// into zero shares, so `<[Party; 0]>::from` fails to compile.
///
/// ```
/// use before::Party;
/// let [a, b, c]: [Party; 3] = Party::seed().into();
/// assert!(a.is_disjoint(&b) && b.is_disjoint(&c) && a.is_disjoint(&c));
/// ```
impl<const N: usize> From<Party> for [Party; N] {
    fn from(party: Party) -> [Party; N] {
        const { assert!(N >= 1, "a `Party` cannot split into zero shares") }
        let mut split = Split::new(party, N);
        // `from_fn` calls indices `0..N` in order, and `Split` yields in
        // preorder, so share `i` lands at index `i` — the same order `forks`
        // hands them out.
        let shares = core::array::from_fn(|_| {
            split
                .next()
                .expect("a split into N shares yields exactly N leaves")
        });
        debug_assert!(
            split.next().is_none(),
            "a split into N shares yields no more than N"
        );
        shares
    }
}
