//! Tests for the minimum record stack.

use num_bigint::BigInt;
use proptest::prelude::*;

use super::super::integers::tests::{sign_of, value_of};
use super::{Popped, Records};

/// The signed offset a popped record carries.
fn offset_of(popped: &Popped) -> BigInt {
    value_of(popped.sign, &popped.magnitude)
}

/// Offsets of any sign and width, close counts, and first flags come back
/// exactly, in LIFO order, after any interleaving of pushes, increments, and
/// pops.
///
/// Each record's close count is its own: increments reach only the current
/// minimum.
#[test]
fn records_round_trip_every_width_in_stack_order() {
    proptest!(|(ops in prop::collection::vec(
        (any::<bool>(), -1i64..=1, 0u32..320, 0u8..4, any::<bool>()),
        1..60,
    ))| {
        let mut records = Records::new();
        let mut model: Vec<(BigInt, u64, bool)> = Vec::new();
        for (push, sign, bits, closes, first) in ops {
            if push || model.is_empty() {
                let offset = BigInt::from(sign) * ((BigInt::from(1) << bits) - 1);
                records.push(sign_of(&offset), &offset.to_u64_digits().1, first);
                model.push((offset, 0, first));
            }
            for _ in 0..closes {
                records.increment();
                model.last_mut().expect("a record was pushed").1 += 1;
            }
            if !push {
                let popped = records.pop();
                let (offset, closes, first) = model.pop().expect("model tracks the stack");
                prop_assert_eq!(offset_of(&popped), offset);
                prop_assert_eq!(popped.closes, closes);
                prop_assert_eq!(popped.first, first);
            }
        }
        while let Some((offset, closes, first)) = model.pop() {
            let popped = records.pop();
            prop_assert_eq!(offset_of(&popped), offset);
            prop_assert_eq!(popped.closes, closes);
            prop_assert_eq!(popped.first, first);
        }
    });
}
