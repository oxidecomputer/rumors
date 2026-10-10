//! Tests for the integer stack.

use num_bigint::{BigInt, BigUint, Sign};
use proptest::prelude::*;

use super::{IntegerStack, Magnitude, PACKED_BITS};

/// A signed integer whose magnitude spans 0 to 300 bits, with every bit
/// pattern reachable at each width, so both sides of [`PACKED_BITS`] and
/// every piece and limb boundary occur.
fn arb_integer() -> impl Strategy<Value = BigInt> {
    (
        any::<bool>(),
        0u64..=300,
        prop::collection::vec(any::<u32>(), 10),
    )
        .prop_map(|(negative, bits, digits)| {
            let mut magnitude = BigUint::from_slice(&digits);
            magnitude &= (BigUint::from(1u8) << bits) - 1u8;
            BigInt::from_biguint(if negative { Sign::Minus } else { Sign::Plus }, magnitude)
        })
}

/// The integer a popped entry denotes.
fn value_of(negative: bool, magnitude: &Magnitude) -> BigInt {
    let digits: Vec<u32> = magnitude
        .limbs()
        .iter()
        .flat_map(|limb| [*limb as u32, (*limb >> 32) as u32])
        .collect();
    let sign = if negative { Sign::Minus } else { Sign::Plus };
    BigInt::from_biguint(sign, BigUint::from_slice(&digits))
}

/// Every pushed integer pops back exactly, in LIFO order, under any
/// interleaving of pushes and pops.
///
/// This holds whichever side of [`PACKED_BITS`] each integer lies on; popped
/// limbs carry no high zero limb, and only magnitudes wider than
/// [`PACKED_BITS`] come back in a separate allocation.
#[test]
fn integers_round_trip_in_stack_order() {
    proptest!(|(ops in prop::collection::vec((any::<bool>(), arb_integer()), 1..80))| {
        let mut stack = IntegerStack::new();
        let mut model: Vec<BigInt> = Vec::new();
        let check = |stack: &mut IntegerStack, expected: BigInt| -> Result<(), TestCaseError> {
            let (negative, magnitude) = stack.pop();
            prop_assert!(magnitude.limbs().last() != Some(&0));
            prop_assert_eq!(
                matches!(magnitude, Magnitude::Separate(_)),
                expected.bits() > PACKED_BITS
            );
            prop_assert_eq!(value_of(negative, &magnitude), expected);
            Ok(())
        };
        for (push, value) in ops {
            if push || model.is_empty() {
                stack.push(value.sign() == Sign::Minus, value.iter_u64_digits());
                model.push(value);
            } else {
                check(&mut stack, model.pop().expect("model tracks the stack"))?;
            }
        }
        while let Some(expected) = model.pop() {
            check(&mut stack, expected)?;
        }
    });
}
