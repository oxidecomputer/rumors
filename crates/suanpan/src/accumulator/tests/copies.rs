//! Clones: the value, the source's value, and the copy's working width.
//!
//! A clone normalizes its source in place and copies only the normalized
//! digits, so it must hold the value in the value's own width, whatever
//! cancellation the source carried. A copy of the stored form would carry
//! that cancellation into every later read of the copy; the width bound here
//! tells the two apart.

use num_bigint::BigInt as IBig;
use proptest::prelude::*;

use super::{assert_value, cancelled_top, from_limbs, Accumulator};

/// Whether `copy` holds `value` within the value's own width plus the one
/// position a normalized form can add.
fn holds_its_own_width(copy: &Accumulator, value: &IBig) -> bool {
    let own_digits = usize::try_from(value.magnitude().bits().div_ceil(32).max(1))
        .expect("tested values have addressable widths");
    copy.stored_digit_count() <= own_digits + 1
}

/// A generated history: an optional cancelled top, then shifted limb updates
/// of either sign.
#[derive(Clone, Debug)]
struct History {
    /// The position of a top that cancels across two digits, if any.
    cancelled: Option<u64>,
    /// Updates as (subtract, shift, limbs).
    updates: Vec<(bool, u64, Vec<u64>)>,
}

/// Generate histories whose updates mix small and wide shifts.
fn arb_history() -> impl Strategy<Value = History> {
    (
        proptest::option::of(1u64..=60),
        proptest::collection::vec(
            (
                any::<bool>(),
                prop_oneof![0u64..=64, 0u64..=2_048],
                proptest::collection::vec(any::<u64>(), 0..=3),
            ),
            0..=12,
        ),
    )
        .prop_map(|(cancelled, updates)| History { cancelled, updates })
}

/// Replay a history into an accumulator and its oracle value.
fn replay(history: &History) -> (Accumulator, IBig) {
    let mut acc = history
        .cancelled
        .map_or_else(Accumulator::new, cancelled_top);
    let mut oracle = IBig::ZERO;
    for (subtract, shift, limbs) in &history.updates {
        let value = IBig::from(from_limbs(limbs)) << usize::try_from(*shift).unwrap();
        if *subtract {
            acc.sub_shifted_limbs(*shift, limbs.iter().copied());
            oracle -= value;
        } else {
            acc.add_shifted_limbs(*shift, limbs.iter().copied());
            oracle += value;
        }
    }
    (acc, oracle)
}

proptest! {
    /// A clone, or a `clone_from` into a receiver of any prior width, holds the
    /// source's value within the value's own width, and leaves the source's
    /// value unchanged and, normalized in place, in its own width too.
    #[test]
    fn clones_hold_the_value_in_its_own_width(
        history in arb_history(),
        prior in arb_history(),
    ) {
        let (source, oracle) = replay(&history);
        let copy = source.clone();
        assert_value(&copy, &oracle);
        prop_assert!(holds_its_own_width(&copy, &oracle), "clone keeps a cancelled width");
        copy.stored().assert_invariants(&[]);
        assert_value(&source, &oracle);
        prop_assert!(holds_its_own_width(&source, &oracle), "clone left the source's cancelled width");

        let (mut receiver, _) = replay(&prior);
        receiver.clone_from(&source);
        assert_value(&receiver, &oracle);
        prop_assert!(holds_its_own_width(&receiver, &oracle), "clone_from keeps a cancelled width");
        receiver.stored().assert_invariants(&[]);
        assert_value(&source, &oracle);
    }
}

/// A copy of the stored form, as a clone that skipped normalization would
/// make, keeps the source's cancelled width and fails the width bound the
/// clone property checks, while a clone of the same source passes it.
#[test]
fn stored_form_copies_fail_the_width_bound() {
    for position in [3u64, 30] {
        for tail in [5i64, -5, 1 << 40] {
            let mut source = cancelled_top(position);
            source += tail;
            let value = IBig::from(tail);
            assert!(
                !holds_its_own_width(&source.stored_copy(), &value),
                "a stored-form copy of {tail} under a top cancelled at {position}"
            );
            assert!(
                holds_its_own_width(&source.clone(), &value),
                "a clone of {tail} under a top cancelled at {position}"
            );
        }
    }
}

/// `clone_from` writes the copy into the receiver's existing buffer when it
/// is large enough.
#[test]
fn clone_from_reuses_the_receivers_buffer() {
    let mut receiver = Accumulator::new();
    receiver.add_shifted_limbs(32 * 100, [1]);
    let buffer = receiver.stored().digits.stored_digits().as_ptr();
    let mut source = cancelled_top(40);
    source.add_shifted_limbs(0, [u64::MAX, 7]);
    receiver.clone_from(&source);
    assert_eq!(receiver.stored().digits.stored_digits().as_ptr(), buffer);
    assert_value(&receiver, &IBig::from(from_limbs(&[u64::MAX, 7])));
}
