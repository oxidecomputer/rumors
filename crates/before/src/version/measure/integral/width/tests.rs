//! Sparse width arithmetic checked against ordinary arbitrary-precision integers.

use num_bigint::{BigInt, BigUint, Sign};
use proptest::prelude::*;
use suanpan::Accumulator;

use super::{ScaledWidth, SparseWidth};
use crate::accumulator::BigIntAccumulator as _;

/// These five digit positions split only where their zero gaps exceed the limit.
///
/// Testing positions independently of products protects the allocation bound:
/// a mathematically correct product could still densify an unnecessarily wide gap.
#[test]
fn five_positions_split_at_the_selected_gap_limits() {
    let width = SparseWidth {
        digits: vec![(0, 1), (3, -2), (4, 5), (8, 1), (20, -7)],
    };
    // There are 2, 0, 3, and 11 empty positions between successive digits.
    let split = |limit| -> Vec<Vec<u64>> {
        width
            .clusters(limit)
            .map(|cluster| cluster.iter().map(|&(index, _)| index).collect())
            .collect()
    };
    assert_eq!(split(1), vec![vec![0], vec![3, 4], vec![8], vec![20]]);
    assert_eq!(split(3), vec![vec![0, 3, 4, 8], vec![20]]);
    assert_eq!(split(11), vec![vec![0, 3, 4, 8, 20]]);
}

/// Evaluate sparse digits directly, without the production clustering or carry merge.
fn dense_value(width: &SparseWidth) -> BigInt {
    width
        .digits
        .iter()
        .map(|&(index, digit)| {
            BigInt::from(digit) << usize::try_from(32 * index).expect("test indices fit usize")
        })
        .sum()
}

proptest! {
    /// Clustered multiplication equals one whole-width signed integer product.
    ///
    /// Arbitrary signs and gaps exercise one-digit products, both dense sign parts,
    /// cancellation, and gaps on either side of the height-width split threshold.
    #[test]
    fn clustered_products_match_dense_integer_multiplication(
        factor_bytes in proptest::collection::vec(any::<u8>(), 1..200),
        entries in proptest::collection::vec(
            (
                0u64..80,
                prop_oneof![-(1i64 << 31)..0, 1..(1i64 << 31)],
            ),
            1..60,
        ),
        negative in any::<bool>(),
    ) {
        let mut digits = Vec::with_capacity(entries.len());
        let mut index = 0;
        for (gap, digit) in entries {
            index += gap;
            digits.push((index, digit));
            index += 1;
        }
        let width = SparseWidth { digits };
        let factor = BigInt::from_biguint(
            if negative { Sign::Minus } else { Sign::Plus },
            BigUint::from_bytes_le(&factor_bytes),
        );
        let expected = &factor * dense_value(&width);
        let mut actual = Accumulator::new();
        width.add_product(&mut actual, &factor);
        prop_assert_eq!(actual.to_bigint(), expected);
    }

    /// Converting and adding scaled widths preserves their sum and sparse invariants.
    ///
    /// Independent scales exercise empty prefixes, overlapping digits, carries,
    /// and disjoint spans; ordinary integer addition supplies the reference value.
    #[test]
    fn scaled_width_addition_preserves_values_and_balanced_digits(
        left_bytes in proptest::collection::vec(any::<u8>(), 0..160),
        right_bytes in proptest::collection::vec(any::<u8>(), 0..160),
        left_digits in 0u64..80,
        right_digits in 0u64..80,
    ) {
        let left = ScaledWidth { magnitude: BigUint::from_bytes_le(&left_bytes), shift: 32 * left_digits };
        let right = ScaledWidth { magnitude: BigUint::from_bytes_le(&right_bytes), shift: 32 * right_digits };
        let expected = (&left.magnitude << left.shift as usize) + (&right.magnitude << right.shift as usize);
        let mut sum = SparseWidth::from_scaled(&left);
        sum.add(SparseWidth::from_scaled(&right));

        prop_assert_eq!(dense_value(&sum), BigInt::from(expected));
        prop_assert!(sum.digits.iter().all(|&(_, digit)| {
            digit != 0 && (-(1i64 << 31)..(1i64 << 31)).contains(&digit)
        }), "every stored digit must be nonzero and balanced");
        prop_assert!(sum.digits.windows(2).all(|pair| pair[0].0 < pair[1].0));
    }
}
