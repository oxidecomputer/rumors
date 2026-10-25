//! Structural facts about stored forms that cost claims rest on.
//!
//! Each test states a fact about the representation an operation leaves
//! behind: that a query leaves a known zero, that reading an operand compacts
//! it, and that two shapes the fuel claims compare have the stored widths
//! those claims assume. The values themselves are checked elsewhere; these
//! check the forms.

use core::cmp::Ordering;

use num_bigint::BigInt as IBig;
use proptest::prelude::*;

use super::representation::{apply_stream_step, run_forming_stream};
use super::{cancelled_top, fresh, Accumulator};
use crate::accumulator::operand::Update;

proptest! {
    /// A sign query of zero spelled as a top cancelling across two digits
    /// answers zero and leaves a known zero, at any height of the top.
    ///
    /// The query's fold collapses what it reads, so the cancelled top does
    /// not survive it for a later query to walk again.
    #[test]
    fn a_sign_query_of_a_cancelled_top_leaves_a_known_zero(position in 1u64..4_096) {
        let acc = cancelled_top(position);
        prop_assert!(!acc.is_known_zero(), "the cancelled top stays stored until read");
        prop_assert_eq!(acc.sign(), Ordering::Equal);
        prop_assert!(acc.is_known_zero(), "the query's collapse leaves a known zero");
    }

    /// A cleared accumulator reads zero at scale zero, whatever stream formed
    /// the value it held.
    #[test]
    fn a_cleared_accumulator_reads_zero_at_scale_zero(ops in run_forming_stream()) {
        let mut acc = fresh(true);
        let mut oracle = IBig::from(0);
        for (step, op) in ops.iter().enumerate() {
            apply_stream_step(&mut acc, &mut oracle, op, step)?;
        }
        acc.clear();
        let (sign, limbs, shift) = acc.scaled_signed_magnitude();
        prop_assert_eq!(sign, Ordering::Equal);
        prop_assert!(limbs.as_ref().is_empty(), "a cleared value has no limbs");
        prop_assert_eq!(shift, 0, "a cleared value reads at scale zero");
    }

    /// Two values of `n` limbs whose top digits share position `2n − 1` store
    /// the same number of digits, however many interior digits are zero.
    ///
    /// The fuel claim that an owned addition reads its right operand on a tie
    /// compares exactly these shapes: one with every digit `5`, and one with
    /// only its lowest and highest digits nonzero.
    #[test]
    fn values_whose_tops_share_a_position_tie_in_stored_digits(count in 2u64..64) {
        let mut dense = Accumulator::new();
        dense.add_shifted_limbs(
            0,
            core::iter::repeat_n(0x5_0000_0005, usize::try_from(count).unwrap()),
        );
        let mut sparse = Accumulator::new();
        sparse.add_shifted_limbs(0, [5]);
        sparse.add_shifted_limbs(64 * count - 32, [5]);
        let digits = usize::try_from(2 * count).unwrap();
        prop_assert_eq!(dense.stored_digit_count(), digits);
        prop_assert_eq!(sparse.stored_digit_count(), digits);
    }

    /// A single top digit placed `lead` digits above `n` full digits stores
    /// exactly `lead` more digits than they do.
    ///
    /// The fuel claim on comparisons from a two-digit lead builds its wide
    /// operand this way, so its leads are the ones it names.
    #[test]
    fn a_single_top_digit_leads_a_full_run_by_its_position(
        half in 2u64..40,
        lead in 1u64..=3,
        top in prop_oneof![Just(2u64), Just(5)],
    ) {
        let digits = 2 * half;
        let mut narrow = Accumulator::new();
        narrow.add_shifted_limbs(
            0,
            core::iter::repeat_n(u64::MAX, usize::try_from(half).unwrap()),
        );
        let mut wide = Accumulator::new();
        wide.add_shifted_limbs(32 * (digits + lead - 1), [top]);
        prop_assert_eq!(
            wide.stored_digit_count(),
            narrow.stored_digit_count() + usize::try_from(lead).unwrap()
        );
    }
}

/// Reading `2^32 + 1`, stored as two digits of 1, as an accumulator operand
/// compacts it to one digit.
///
/// The operand's comparison with zero cannot decide on its top digit, so it
/// folds the two digits into one, which holds `2^32 + 1` below the digit
/// limit; the operand keeps that form for every later read.
#[test]
fn reading_a_two_digit_operand_compacts_it_to_one_digit() {
    let mut operand = fresh(true);
    operand
        .value
        .get_mut()
        .digits
        .apply_limbs(core::iter::once((1 << 32) | 1), Update::Add, 0);
    assert_eq!(
        operand.stored_digit_count(),
        2,
        "the operand starts as two digits"
    );
    let mut receiver = Accumulator::new();
    receiver.add_shifted(32_000, &operand);
    assert_eq!(
        operand.stored_digit_count(),
        1,
        "the zero comparison compacts the operand"
    );
}
