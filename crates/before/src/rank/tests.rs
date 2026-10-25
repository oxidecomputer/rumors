//! Rank encoding arithmetic at the edges of the `u64` exponent.

use proptest::prelude::*;

use super::{fraction_group_bits, FRACTION_GROUP_BITS};

/// Exponents within this distance of `u64::MAX` are checked exhaustively.
const EXPONENT_WINDOW: u64 = 64;

/// The groups the encoder writes for exponent `exp`.
fn group_count(exp: u64) -> u64 {
    exp.div_ceil(FRACTION_GROUP_BITS)
}

/// The numerator bits group `group` carries, computed in `u128`: expansion
/// bit `j = 8g + k` is numerator bit `exp - j` when `j <= exp`, and padding
/// otherwise. The oracle for [`fraction_group_bits`].
fn widened_group_bits(exp: u64, group: u64) -> Vec<Option<u64>> {
    let width = u128::from(FRACTION_GROUP_BITS);
    (1..=width)
        .map(|k| {
            let j = u128::from(group) * width + k;
            (j <= u128::from(exp)).then(|| (u128::from(exp) - j) as u64)
        })
        .collect()
}

/// Every group of every exponent up to 64, and the first and last groups of
/// every exponent near `u64::MAX`, carry the numerator bits widened
/// arithmetic assigns them.
///
/// The last group of an exponent of `2^64 - 7` or more is where the
/// expansion position `8g + 8` reaches 2^64.
#[test]
fn fraction_groups_carry_their_bits_near_the_u64_limit() {
    for exp in 0..=64 {
        for group in 0..group_count(exp) {
            let bits: Vec<_> = fraction_group_bits(exp, group).collect();
            assert_eq!(
                bits,
                widened_group_bits(exp, group),
                "exp {exp}, group {group}"
            );
        }
    }
    for exp in u64::MAX - EXPONENT_WINDOW..=u64::MAX {
        let last = group_count(exp) - 1;
        for group in [0, last - 1, last] {
            let bits: Vec<_> = fraction_group_bits(exp, group).collect();
            assert_eq!(
                bits,
                widened_group_bits(exp, group),
                "exp {exp}, group {group}"
            );
        }
    }
}

proptest! {
    /// Across every exponent and every group the encoder writes for it,
    /// each group carries the numerator bits widened arithmetic assigns it.
    #[test]
    fn fraction_groups_agree_with_widened_arithmetic(
        (exp, group) in prop_oneof![1..=u64::MAX, u64::MAX - EXPONENT_WINDOW..=u64::MAX]
            .prop_flat_map(|exp| (Just(exp), 0..group_count(exp))),
    ) {
        let bits: Vec<_> = fraction_group_bits(exp, group).collect();
        prop_assert_eq!(bits, widened_group_bits(exp, group));
    }
}
