//! Integrate deferred heights over all regions following their deferral.
//!
//! An entry records a height and the width *preceding* it. Its height applies
//! to every later entry's width. A final zero-height entry supplies the width
//! after the last deferral. The answer is therefore `sum(P_i * w_j, i < j)`.
//! No pair has `j = 0`, so the first entry's width enters no product, and
//! none is collected: that entry records a zero width.
//!
//! A reduction over adjacent entry groups computes this sum without revisiting
//! each suffix: an internal node adds the sum of its left heights times the
//! sum of its right widths. Every ordered pair crosses exactly one split.
//! Signed heights can cancel before a product; sparse widths remain compact
//! when adjacent intervals combine.

use num_bigint::BigInt;
use suanpan::Accumulator;

use super::width::{ScaledWidth, SparseWidth};

/// Deferred heights, ordered by the boundary where each began to apply.
pub struct DeferredIntegral {
    /// Width since the latest deferral; zero until the first deferral.
    width: Accumulator,
    /// Heights already deferred, each paired with the width preceding it.
    entries: Vec<Entry>,
}

/// A deferred height and the interval between this deferral and its predecessor.
struct Entry {
    /// Applies to subsequent intervals; its preceding interval is already settled.
    height: BigInt,
    /// Width behind this entry, retained separately from its power-of-two
    /// scale; zero for the first entry.
    width: ScaledWidth,
}

/// Collect relative widths during the sweep, then settle all future contributions together.
impl DeferredIntegral {
    /// Begin without deferred heights or accumulated interval widths.
    pub fn new() -> Self {
        Self {
            width: Accumulator::new(),
            entries: Vec::new(),
        }
    }

    /// Whether there are no future contributions to settle.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Extend the interval preceding the next deferral by a completed segment.
    ///
    /// Before the first deferral this collects nothing, since that interval
    /// would be the first entry's width, which no product reads.
    pub fn add_width(&mut self, width: &ScaledWidth) {
        if !self.is_empty() {
            width.add_to(&mut self.width);
        }
    }

    /// Defer a nonzero height from the current boundary onward.
    pub fn push(&mut self, height: BigInt) {
        let width = ScaledWidth::read(&self.width);
        debug_assert!(
            self.is_empty() || !width.is_zero(),
            "a later deferral follows at least one interval"
        );
        self.entries.push(Entry { height, width });

        // Keep the buffer, as `Integrator::freeze` keeps the segment width's:
        // a fresh accumulator would grow from digit 0 to the next interval's
        // scale at every deferral. Clearing keeps the buffer, at amortized
        // O(1).
        self.width.clear();
    }

    /// Add every deferred contribution after the final segment width has arrived.
    pub fn finish(&mut self, total: &mut Accumulator) {
        debug_assert!(
            !self.is_empty(),
            "an empty deferred integral needs no reduction"
        );
        let entries = core::mem::take(&mut self.entries);
        let final_width = ScaledWidth::read(&self.width);
        let mut leaves = Vec::with_capacity(entries.len() + 1);
        for entry in entries {
            leaves.push(Aggregate::new(&entry.height, &entry.width));
        }
        // Every deferred height applies to this final width; its zero height
        // adds no contribution of its own.
        leaves.push(Aggregate {
            heights: Accumulator::new(),
            widths: SparseWidth::from_scaled(&final_width),
        });
        Aggregate::reduce(leaves, total);
    }
}

/// The summed heights and widths of a contiguous range of entries.
struct Aggregate {
    /// Signed sums allow large opposing heights to cancel before multiplication.
    heights: Accumulator,
    /// Balanced sparse digits compact contiguous interval widths during merging.
    widths: SparseWidth,
}

/// Preserve boundary order while combining a pair of completed reduction groups.
impl Aggregate {
    /// Convert one entry into the representations used throughout the reduction.
    fn new(height: &BigInt, width: &ScaledWidth) -> Self {
        let mut heights = Accumulator::new();
        heights += height;
        Self {
            heights,
            widths: SparseWidth::from_scaled(width),
        }
    }

    /// Bits read when this group participates in a merge; even empty groups
    /// cost one.
    fn weight(&self) -> u64 {
        (self.heights.stored_bits() + self.widths.stored_bits()).max(1)
    }

    /// Merge a newer right group into this older left group.
    fn merge(&mut self, right: Self, total: &mut Accumulator) {
        // Pairs entirely within either half have already contributed. This
        // product accounts exactly for the pairs crossing this split.
        let heights = BigInt::from(&self.heights);
        if heights != BigInt::ZERO {
            right.widths.add_product(total, &heights);
        }
        self.heights += &right.heights;
        self.widths.add(right.widths);
    }
    /// Reduce boundary-ordered groups, balancing the work by integer data size.
    ///
    /// Balancing by entry count would let one wide height participate in large
    /// products at many levels. Data-weighted splits isolate expensive entries
    /// near the root. The combined weight at each level stays bounded by the
    /// input data; along a descending path it halves at least every two levels.
    /// The backend multiplication, `num-bigint`'s and the `M(n)` of the
    /// integrator's bound, costs polynomially more than linear (Toom-3,
    /// `Θ(n^log₃5)`, at large sizes). The products' costs therefore fall
    /// geometrically down the tree and sum to the order of the root's product.
    /// The argument needs that polynomial gap: an `M` growing only as
    /// `n log n` would leave a factor of the tree's depth.
    fn reduce(leaves: Vec<Self>, total: &mut Accumulator) {
        let mut prefix = Vec::with_capacity(leaves.len() + 1);
        let mut running = 0u64;
        prefix.push(0);
        for leaf in &leaves {
            running += leaf.weight();
            prefix.push(running);
        }

        let leaf_count = leaves.len();
        let mut leaves = leaves.into_iter();
        let mut next_leaf = 0;
        let mut control = vec![Step::Open {
            lo: 0,
            hi: leaf_count,
        }];
        let mut reduced = Vec::new();
        while let Some(step) = control.pop() {
            match step {
                Step::Open { lo, hi } if hi - lo == 1 => {
                    debug_assert_eq!(next_leaf, lo, "left-first traversal visits leaves in order");
                    next_leaf += 1;
                    reduced.push(leaves.next().expect("one aggregate per leaf range"));
                }
                Step::Open { lo, hi } => {
                    let mid = Self::split(&prefix, lo, hi);
                    // LIFO order visits left, then right, then merges their
                    // completed aggregates. The older half must stay left.
                    control.push(Step::Merge);
                    control.push(Step::Open { lo: mid, hi });
                    control.push(Step::Open { lo, hi: mid });
                }
                Step::Merge => {
                    let right = reduced.pop().expect("the right half completed");
                    let mut left = reduced.pop().expect("the left half completed");
                    left.merge(right, total);
                    reduced.push(left);
                }
            }
        }
    }

    /// Split `[lo, hi)` at its weight midpoint, keeping both halves nonempty.
    ///
    /// The left half ends just after the leaf containing the midpoint. If
    /// that leaf makes the left half heavier than half the range, the next
    /// split either halves the weight or isolates that heavy leaf. Thus a
    /// non-leaf path halves its weight at least every two levels. Tree depth
    /// is bounded by `2 * log2(total weight) + 2`, not by log entry count:
    /// exponentially growing leaf weights can produce a chain of splits.
    fn split(prefix: &[u64], lo: usize, hi: usize) -> usize {
        let target = (prefix[lo] + prefix[hi]).div_ceil(2);
        (lo + 1 + prefix[lo + 1..hi].partition_point(|&p| p < target)).min(hi - 1)
    }
}

/// Explicit postorder traversal, avoiding recursion on the number of deferrals.
enum Step {
    /// Expand a nonempty contiguous range of entries.
    Open { lo: usize, hi: usize },
    /// Merge the two completed child aggregates at the top of the result stack.
    Merge,
}

#[cfg(test)]
mod tests;
