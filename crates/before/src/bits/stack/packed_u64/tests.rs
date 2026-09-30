use proptest::prelude::*;

use super::PackedU64Stack;

/// Packed values round-trip in LIFO order at representative widths, including
/// zero and the full machine word.
#[test]
fn packed_values_round_trip() {
    let mut stack = PackedU64Stack::new();
    let values = [0u64, 1, 7, 64, 3, 1, 100_000, 2, u64::MAX];
    for &value in &values {
        stack.push(value);
    }
    for &value in values.iter().rev() {
        assert_eq!(stack.pop(), value);
    }
}

proptest! {
    /// The packed stack agrees with `Vec<u64>` under arbitrary pushes and pops
    /// with values distributed across every possible bit width.
    #[test]
    fn packed_stack_matches_a_vec_across_all_widths(
        ops in proptest::collection::vec(
            (any::<bool>(), 0u32..64, any::<u64>()),
            1..200,
        ),
    ) {
        let mut stack = PackedU64Stack::new();
        let mut model: Vec<u64> = Vec::new();
        for (push, shift, raw) in ops {
            if push || model.is_empty() {
                // Give the generated value exactly `shift + 1` bits. `shift =
                // 63` exercises the split full-word path; zero remains part of
                // the one-bit class.
                let value = if shift == 0 {
                    raw & 1
                } else {
                    (1u64 << shift) | (raw & ((1u64 << shift) - 1))
                };
                stack.push(value);
                model.push(value);
            } else {
                prop_assert_eq!(stack.pop(), model.pop().expect("guarded nonempty"));
            }
        }
        for value in model.into_iter().rev() {
            prop_assert_eq!(stack.pop(), value);
        }
    }
}
