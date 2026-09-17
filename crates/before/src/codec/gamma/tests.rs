//! Tests for the signed zigzag layer of the gamma codec.

use num_bigint::{BigInt, BigUint, Sign};
use proptest::prelude::*;

use crate::codec::BitsBuf;

use super::{decode_signed, encode, encode_negative, encode_positive, encode_signed, zigzag};

/// Magnitudes spanning inline words and arbitrary-width digit sequences.
fn arb_magnitude() -> impl Strategy<Value = BigUint> {
    prop_oneof![
        any::<u64>().prop_map(BigUint::from),
        prop::collection::vec(any::<u32>(), 0..8).prop_map(BigUint::new),
        (0u32..=255, -1i8..=1).prop_map(|(shift, offset)| {
            let power = BigUint::from(1u8) << shift;
            match offset {
                -1 if shift > 0 => power - 1u8,
                1 => power + 1u8,
                _ => power,
            }
        }),
    ]
}

proptest! {
    /// Both fused signed-emission entry points are bit-identical to zigzag
    /// followed by ordinary gamma encoding at every tested width.
    #[test]
    fn signed_emission_matches_the_composition(
        magnitude in arb_magnitude(),
        negative in any::<bool>(),
    ) {
        let sign = if negative && magnitude != BigUint::ZERO {
            Sign::Minus
        } else {
            Sign::Plus
        };
        let value = BigInt::from_biguint(sign, magnitude);
        let mut fused = BitsBuf::new();
        encode_signed(&value, &mut fused);
        let mut from_magnitude = BitsBuf::new();
        if sign == Sign::Minus {
            encode_negative(value.magnitude(), &mut from_magnitude);
        } else {
            encode_positive(value.magnitude(), &mut from_magnitude);
        }
        let mut composed = BitsBuf::new();
        encode(&zigzag(value), &mut composed);
        prop_assert_eq!(&fused, &composed);
        prop_assert_eq!(from_magnitude, composed);
    }

    /// Zigzag decoding recovers the normalized signed integer supplied to the
    /// encoder, including zero and magnitudes wider than a machine word.
    #[test]
    fn signed_zigzag_round_trips(
        magnitude in arb_magnitude(),
        negative in any::<bool>(),
    ) {
        let sign = if negative && magnitude != BigUint::ZERO {
            Sign::Minus
        } else {
            Sign::Plus
        };
        let expected = BigInt::from_biguint(sign, magnitude);
        prop_assert_eq!(decode_signed(zigzag(expected.clone())), expected);
    }
}
