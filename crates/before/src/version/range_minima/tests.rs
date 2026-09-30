//! Behavioral checks for nested minima, deferred anchors, and client state.
//!
//! Exact cases distinguish comparisons on either side of a deferred minimum.
//! Properties vary widths, nesting, payload lifetimes, and follower transitions.

use core::cmp::Ordering;

use num_bigint::{BigInt, BigUint, Sign};
use proptest::prelude::*;
use suanpan::Accumulator;

use super::{Close, RangeMinima};
use crate::accumulator::BigIntAccumulator as _;

/// Expected results when probing the minimum, one above it, and one below it.
const AROUND_MINIMUM: [Ordering; 3] = [Ordering::Equal, Ordering::Greater, Ordering::Less];

/// Offset for a value `distance` below the running height.
fn below(distance: u64) -> BigInt {
    -BigInt::from(distance)
}

/// Offset for a value an arbitrary-precision distance below the running height.
fn below_magnitude(distance: &BigUint) -> BigInt {
    BigInt::from_biguint(Sign::Minus, distance.clone())
}

/// Probe a claimed minimum and its immediate neighbors without emitting values.
fn around_minimum(minima: &mut RangeMinima<()>, minimum_offset: &BigInt) -> [Ordering; 3] {
    [
        minima.compare_above(minimum_offset),
        minima.compare_above(&(minimum_offset + 1)),
        minima.compare_above(&(minimum_offset - 1)),
    ]
}

/// Construct an accumulator for a known signed difference in a fixture.
fn accumulated(value: &BigInt) -> Accumulator {
    let mut result = Accumulator::new();
    result.add_bigint(value);
    result
}

/// Observable payload construction and retirement during one emission.
#[derive(Default)]
struct PayloadEvents {
    /// Number of times the lazy payload factory ran.
    created: usize,
    /// Outer minima whose suspended payloads were retired, in callback order.
    retired: Vec<BigInt>,
}

/// Close one range and compare its result with an absolute-minimum model.
fn close_against_model(minima: &mut RangeMinima<BigInt>, model: &mut Vec<BigInt>) {
    let inner = model.pop().expect("the model has an open range");
    match (minima.close(), model.last()) {
        (Close::Retired, None) => {}
        (Close::Equal, Some(outer)) => assert_eq!(&inner, outer),
        (Close::Lower(payload), Some(outer)) => {
            assert!(outer < &inner);
            assert_eq!(&payload, outer, "the parent's exact payload resumes");
        }
        _ => panic!("close outcome disagrees with the absolute minima"),
    }
}

/// A deferred distance of 1000 resolves during an emission at 25 without
/// changing the true minimum, zero. Neighboring probes establish its value.
#[test]
fn emitting_25_resolves_a_deferred_1000_without_moving_the_minimum() {
    let mut minima = RangeMinima::new();
    minima.open(2);
    minima.emit_here(); // Both outer ranges have minimum zero.
    minima.open(1);
    minima.fold_height(&BigInt::from(1000));
    minima.emit_here(); // The new range has minimum 1000.
    minima.close(); // Anchor 1000, minimum zero, deferred distance 1000.
    assert!(minima.deferred_live());

    minima.emit_offset(&below(975)); // Value 25 lies above the true minimum.
    assert!(
        !minima.deferred_live(),
        "comparable widths resolve the distance"
    );
    assert_eq!(around_minimum(&mut minima, &below(1000)), AROUND_MINIMUM);
}

/// With a deferred distance of 50, lowering a minimum of `2^36` by `2^34`
/// subtracts that distance before propagation, leaving the outer minimum zero.
#[test]
fn a_deferred_50_does_not_enlarge_the_propagated_2_pow_34_drop() {
    let middle_minimum = 1u64 << 36;
    let decrease = 1u64 << 34;
    let mut minima = RangeMinima::new();
    minima.open(2);
    minima.emit_here(); // Both outer ranges have minimum zero.
    minima.fold_height(&BigInt::from(middle_minimum));
    minima.open(1);
    minima.emit_here();
    minima.fold_height(&BigInt::from(50));
    minima.open(1);
    minima.emit_here();
    minima.close(); // The middle minimum is 50 below the anchor.
    assert!(minima.deferred_live());

    minima.emit_offset(&below(50 + decrease));
    assert!(
        !minima.deferred_live(),
        "the undercut consumes the distance"
    );
    assert_eq!(
        minima.compare_above(&below(50 + decrease)),
        Ordering::Equal,
        "the middle minimum is exactly the emitted value"
    );

    minima.close(); // Propagation must have preserved the outer minimum.
    assert_eq!(
        around_minimum(&mut minima, &below(middle_minimum + 50)),
        AROUND_MINIMUM
    );
}

/// A height 50 below an anchor at `2^36` stays above the true minimum zero.
/// The word-sized deferred distance survives that comparison unchanged.
#[test]
fn a_word_deferred_distance_survives_a_drop_of_fifty() {
    let distance = 1u64 << 36;
    let mut minima = RangeMinima::new();
    minima.open(2);
    minima.emit_here();
    minima.open(1);
    minima.fold_height(&BigInt::from(distance));
    minima.emit_here();
    minima.close(); // Anchor distance, minimum zero.
    assert!(minima.deferred_live());

    minima.fold_height(&BigInt::from(-50));
    minima.emit_here();
    assert!(
        minima.deferred_live(),
        "leading digits decide without resolution"
    );
    assert_eq!(
        around_minimum(&mut minima, &below(distance - 50)),
        AROUND_MINIMUM
    );
}

/// A height 50 below an anchor at `2^200` stays above the true minimum zero.
/// The wide deferred distance survives that comparison unchanged.
#[test]
fn a_wide_deferred_distance_survives_a_drop_of_fifty() {
    let distance = BigUint::from(1u8) << 200usize;
    let mut minima = RangeMinima::new();
    minima.open(2);
    minima.emit_here();
    minima.open(1);
    minima.fold_height(&BigInt::from(distance.clone()));
    minima.emit_here();
    minima.close();
    assert!(minima.deferred_live());

    minima.fold_height(&BigInt::from(-50));
    minima.emit_here();
    assert!(
        minima.deferred_live(),
        "leading digits decide without resolution"
    );
    let height = distance - BigUint::from(50u8);
    assert_eq!(
        around_minimum(&mut minima, &below_magnitude(&height)),
        AROUND_MINIMUM
    );
}

proptest! {
    /// A wide negative height gap decides a small offset emission directly.
    /// The resulting follower decreases by the full `2^bits + offset`, across
    /// varied widths and initial follower values.
    #[test]
    fn a_wide_undercut_moves_live_followers_by_the_exact_drop(
        bits in 128usize..=300,
        offset in 1u64..=u64::from(u32::MAX),
        initial in 0u64..=1_000_000,
    ) {
        let height_drop = BigUint::from(1u8) << bits;
        let mut minima = RangeMinima::new();
        minima.open(1);
        minima.emit_here();
        minima.follower_set(0, accumulated(&BigInt::from(initial)));
        minima.fold_height(&-BigInt::from(height_drop.clone()));
        minima.emit_offset(&below(offset));

        let moved = minima.follower_take(0).into_bigint();
        let expected_drop = height_drop + BigUint::from(offset);
        prop_assert_eq!(moved.sign(), Sign::Minus);
        prop_assert_eq!(moved.magnitude(), &(expected_drop - BigUint::from(initial)));
    }

    /// Values between an anchor and the true minimum leave that minimum
    /// unchanged and retain enough state for a later undercut to lower every
    /// enclosing range. Widths cover word-sized and arbitrary-precision values.
    #[test]
    fn values_between_anchor_and_minimum_preserve_the_minimum(
        bits in 34usize..=260,
        below_anchor in 1u64..=(1u64 << 33),
    ) {
        let distance = BigUint::from(1u8) << bits;
        let mut minima = RangeMinima::new();
        minima.open(2);
        minima.emit_here();
        minima.open(1);
        minima.fold_height(&BigInt::from(distance.clone()));
        minima.emit_here();
        minima.close(); // Anchor distance, true minimum zero.
        prop_assert!(minima.deferred_live());

        minima.fold_height(&-BigInt::from(below_anchor));
        minima.emit_here(); // Strictly between the true minimum and anchor.
        let height = distance - BigUint::from(below_anchor);
        prop_assert_eq!(
            around_minimum(&mut minima, &below_magnitude(&height)),
            AROUND_MINIMUM
        );

        minima.fold_height(&-BigInt::from(height + BigUint::from(1u8)));
        minima.emit_here(); // A true undercut at -1 lowers both outer ranges.
        minima.close();
        minima.fold_height(&BigInt::from(100)); // Height 99, minimum -1.
        prop_assert_eq!(around_minimum(&mut minima, &below(100)), AROUND_MINIMUM);
    }

    /// A batch of equal armed ranges closes one at a time before exposing the
    /// lower outer minimum. Batch sizes include a single range with no zero run.
    #[test]
    fn batch_armed_closes_consume_exactly_one_range_record(count in 1usize..40) {
        let mut minima = RangeMinima::new();
        minima.open(1);
        minima.emit_here();
        minima.fold_height(&BigInt::from(7));
        minima.open(count as u64);
        minima.emit_here(); // One positive boundary and count - 1 zero boundaries.
        for _ in 1..count {
            prop_assert!(matches!(minima.close(), Close::Equal));
        }
        prop_assert!(matches!(minima.close(), Close::Lower(())));
        prop_assert!(!minima.has_pending());
        prop_assert!(minima.armed(), "the outer range survives");
        prop_assert_eq!(around_minimum(&mut minima, &below(7)), AROUND_MINIMUM);
    }

    /// Mixed arming, undercuts, and closes match absolute range minima. Payload
    /// factories run only for distinct minima, retire crossed boundaries in
    /// nesting order, and return the exact parent payload on a positive close.
    #[test]
    fn nested_operations_preserve_minima_and_payload_lifetimes(
        steps in prop::collection::vec((any::<i16>(), 0usize..260, 0u64..4, 0usize..5), 1..60),
    ) {
        let mut minima = RangeMinima::<BigInt>::new();
        let mut model = Vec::<BigInt>::new();
        let mut height = BigInt::from(0);
        for (coefficient, bits, open_count, close_count) in steps {
            let value = BigInt::from(coefficient) << bits;
            minima.fold_height(&(&value - &height));
            height = value.clone();
            // An empty tracker needs a new range before it can observe a value.
            let open_count = if model.is_empty() { open_count.max(1) } else { open_count };
            let previous = model.last().cloned();
            let mut expected_retired = Vec::new();
            if open_count > 0 && previous.as_ref().is_some_and(|minimum| &value < minimum) {
                expected_retired.push(previous.clone().unwrap());
            }
            for adjacent in model.windows(2).rev() {
                if adjacent[0] < adjacent[1] && adjacent[0] >= value {
                    expected_retired.push(adjacent[0].clone());
                }
            }

            let mut events = PayloadEvents::default();
            if open_count > 0 {
                minima.open(open_count);
                minima.arm_at_height(
                    &mut events,
                    |events| {
                        events.created += 1;
                        previous.clone().expect("first arming never constructs a payload")
                    },
                    |payload, events| events.retired.push(payload),
                );
                let distinct = previous.as_ref().is_some_and(|minimum| minimum != &value);
                prop_assert_eq!(events.created, usize::from(distinct));
            } else {
                let undercuts = value < *model.last().unwrap();
                prop_assert_eq!(minima.undercuts_here(), undercuts);
                if undercuts {
                    minima.undercut(&mut events, |payload, events| events.retired.push(payload));
                }
            }
            prop_assert_eq!(events.retired, expected_retired);

            for minimum in &mut model {
                *minimum = minimum.clone().min(value.clone());
            }
            model.extend(core::iter::repeat_n(value, open_count as usize));
            for _ in 0..close_count.min(model.len()) {
                close_against_model(&mut minima, &mut model);
            }
            prop_assert_eq!(minima.armed(), !model.is_empty());
            prop_assert!(!minima.has_pending());
        }
        while !model.is_empty() {
            close_against_model(&mut minima, &mut model);
        }
    }

    /// Followers attached before and after deferred closes keep one shared
    /// anchor. Height conversion cancels the deferred distance; resolution,
    /// each arming form, and ordinary emissions all produce the exact `m - X`.
    #[test]
    fn followers_share_the_anchor_across_deferred_transitions(
        bits in 0usize..260,
        client_values in any::<[i16; 2]>(),
        height_change in any::<i16>(),
        close_middle in any::<bool>(),
        transition in 0u8..5,
        target_multiple in -1i8..=3,
    ) {
        let boundary = BigInt::from(1u8) << bits;
        let anchor = &boundary * 2;
        let mut minima = RangeMinima::new();
        minima.open(1);
        minima.emit_here(); // Outer minimum zero.
        minima.fold_height(&boundary);
        minima.open(1);
        minima.emit_here(); // Middle minimum boundary.
        minima.fold_height(&boundary);
        minima.open(1);
        minima.emit_here(); // Inner minimum 2 * boundary.
        minima.follower_set(0, accumulated(&(&anchor - client_values[0])));
        minima.close();
        minima.follower_set(1, accumulated(&(&anchor - client_values[1])));
        let old_minimum = if close_middle {
            minima.close();
            BigInt::from(0)
        } else {
            boundary.clone()
        };
        prop_assert!(minima.deferred_live());
        minima.fold_height(&BigInt::from(height_change));
        let height = &anchor + height_change;

        let mut detached = minima.follower_take(0);
        prop_assert_eq!(detached.to_bigint(), &anchor - client_values[0]);
        minima.bridge_add_gap(&mut detached);
        prop_assert_eq!(detached.into_bigint(), &height - client_values[0]);
        prop_assert!(minima.deferred_live(), "height conversion cancels the distance");
        minima.follower_set(0, accumulated(&(&anchor - client_values[0])));

        let target = &boundary * target_multiple;
        let expected_minimum = match transition {
            0 => {
                minima.resolve_deferred();
                old_minimum
            }
            1 => {
                minima.fold_height(&(&target - &height));
                minima.open(2);
                minima.emit_here();
                target
            }
            2 => {
                minima.open(2);
                minima.arm_relative(accumulated(&(&target - &anchor)));
                target
            }
            3 => {
                minima.open(2);
                minima.emit_below_accum(accumulated(&(&height - &target)));
                target
            }
            4 => {
                minima.emit_offset(&(&target - &height));
                old_minimum.min(target)
            }
            _ => unreachable!(),
        };
        minima.resolve_deferred();
        for (slot, client_value) in client_values.into_iter().enumerate() {
            let follower = minima.follower_take(slot);
            prop_assert_eq!(
                follower.into_bigint(),
                &expected_minimum - client_value
            );
        }
    }
}
