//! Reduction-tree depth bounds for unevenly sized deferred contributions.

use num_bigint::{BigInt, BigUint, Sign};
use proptest::prelude::*;
use suanpan::Accumulator;

use super::super::width::ScaledWidth;
use super::Aggregate;
use crate::accumulator;

/// Build the cumulative work weights used to split reduction ranges.
fn weight_prefix(weights: &[u64]) -> Vec<u64> {
    let mut prefix = Vec::with_capacity(weights.len() + 1);
    prefix.push(0);
    for &weight in weights {
        prefix.push(prefix.last().expect("the zero prefix exists") + weight.max(1));
    }
    prefix
}

/// Find the deepest leaf by expanding the production split rule on an explicit stack.
fn split_depth(weights: &[u64]) -> usize {
    let prefix = weight_prefix(weights);
    let mut deepest = 0;
    let mut stack = vec![(0, weights.len(), 0)];
    while let Some((lo, hi, depth)) = stack.pop() {
        if hi - lo == 1 {
            deepest = deepest.max(depth);
        } else {
            let mid = Aggregate::split(&prefix, lo, hi);
            stack.push((mid, hi, depth + 1));
            stack.push((lo, mid, depth + 1));
        }
    }
    deepest
}

/// Exponentially increasing weights produce a chain even though uniform weights balance.
///
/// Repeating each power of two also produces a chain: the weight need only halve
/// every second level. These cases protect both the chosen depth measure and its factor two.
#[test]
fn exponential_and_uniform_weights_have_expected_depths() {
    let n = 16;
    let exponential: Vec<u64> = (1..=n).map(|exponent| 1 << exponent).collect();
    assert_eq!(split_depth(&exponential), n - 1);

    // Three unit leaves followed by each power twice. A midpoint can first
    // isolate a straddling leaf before the next split halves the remaining weight.
    let mut doubled = vec![1, 1, 1];
    for exponent in 1..=6 {
        doubled.extend([1 << exponent; 2]);
    }
    assert_eq!(split_depth(&doubled), doubled.len() - 1);
    assert_eq!(split_depth(&vec![8; n]), 4);
}

/// Recursively expand a bounded test tree, asserting nonempty children at every split.
fn recursive_depth(prefix: &[u64], lo: usize, hi: usize, depth: usize) -> usize {
    if hi - lo == 1 {
        return 0;
    }
    let mid = Aggregate::split(prefix, lo, hi);
    assert!(lo < mid && mid < hi, "both halves must be nonempty");
    1 + crate::recurse::descend!(depth + 1, recursive_depth(prefix, lo, mid, depth + 1)).max(
        crate::recurse::descend!(depth + 1, recursive_depth(prefix, mid, hi, depth + 1)),
    )
}

proptest! {
    /// The reduction charges each height against exactly the widths following it.
    /// A direct prefix sum supplies an independent reference for arbitrary signed
    /// heights and scaled widths, including zero entries and cancellations.
    #[test]
    fn deferred_products_match_the_ordered_pair_sum(
        entries in proptest::collection::vec((
            proptest::collection::vec(any::<u8>(), 0..64),
            any::<bool>(),
            proptest::collection::vec(any::<u8>(), 0..64),
            0u32..40,
        ), 1..40),
    ) {
        let mut preceding_heights = BigInt::ZERO;
        let mut expected = BigInt::ZERO;
        let mut leaves = Vec::new();
        for (height_bytes, negative, width_bytes, scale_digits) in entries {
            let height = BigInt::from_biguint(
                if negative { Sign::Minus } else { Sign::Plus },
                BigUint::from_bytes_le(&height_bytes),
            );
            let width = BigUint::from_bytes_le(&width_bytes);
            let shift = u64::from(scale_digits) * 32;
            expected += &preceding_heights * BigInt::from(&width << shift as usize);
            preceding_heights += &height;

            let mut width_accumulator = Accumulator::new();
            accumulator::fold(&mut width_accumulator, &width, shift, false);
            leaves.push(Aggregate::new(&height, &ScaledWidth::read(&width_accumulator)));
        }
        let mut actual = Accumulator::new();
        Aggregate::reduce(leaves, &mut actual);
        prop_assert_eq!(accumulator::signed_value(&actual), expected);
    }

    /// Every split is nonempty and depth is bounded by twice the log of total weight.
    ///
    /// Log-uniform weights span 48 bits, including heavily uneven trees. A recursive
    /// traversal cross-checks the iterative traversal and checks each split locally.
    #[test]
    fn arbitrary_weights_keep_splits_nonempty_and_depth_bounded(
        weights in proptest::collection::vec(
            (0u32..48).prop_flat_map(|shift| (1u64 << shift)..=((1u64 << (shift + 1)) - 1)),
            1..64,
        ),
    ) {
        let prefix = weight_prefix(&weights);
        let total = *prefix.last().expect("the zero prefix exists");
        let depth = recursive_depth(&prefix, 0, weights.len(), 0);
        prop_assert_eq!(depth, split_depth(&weights));
        prop_assert!(depth as u32 <= 2 * total.ilog2() + 2,
            "depth {} exceeds the bound for total weight {}", depth, total);
    }
}
