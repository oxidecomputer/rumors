//! Packing digits into limbs, against the digits' big-integer value.

use num_bigint::BigUint;
use proptest::prelude::*;

use super::{DigitPacker, Limbs, Storage};

/// Whether the limbs are held inline.
fn is_inline(limbs: &Limbs) -> bool {
    !matches!(limbs.0, Storage::Spilled(_))
}

proptest! {
    /// Packing any digit sequence, including high zero digits, yields the
    /// minimal limbs of its value, inline exactly when the digits fill at
    /// most two limbs.
    #[test]
    fn packed_digits_are_the_minimal_limbs(
        digits in proptest::collection::vec(
            prop_oneof![Just(0u32), Just(u32::MAX), any::<u32>()],
            0..=12,
        ),
    ) {
        let mut packer = DigitPacker::new(digits.len());
        for &digit in &digits {
            packer.push(digit);
        }
        let limbs = packer.finish();
        prop_assert_eq!(&*limbs, &BigUint::new(digits.clone()).to_u64_digits()[..]);
        prop_assert_eq!(is_inline(&limbs), digits.len() <= 4);
    }

    /// A `u128` magnitude is held inline as its minimal limbs.
    #[test]
    fn u128_magnitudes_are_inline(magnitude: u128) {
        let limbs = Limbs::from_u128(magnitude);
        prop_assert_eq!(&*limbs, &BigUint::from(magnitude).to_u64_digits()[..]);
        prop_assert!(is_inline(&limbs));
    }
}
