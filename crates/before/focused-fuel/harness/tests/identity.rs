//! The identity fast paths: operands that are clones of one another,
//! byte-equal in distinct buffers, or empty.
//!
//! The at-rest form's shared buffer makes clone identity observable, and the
//! identity-law fast paths dispatch on it, each citing its law in
//! `before::testing::laws` at the code site. These claims hold the fast paths
//! in place: a clone or empty operand answers in fuel independent of the
//! operands' size, and a byte-equal operand in a distinct buffer answers by a
//! byte comparison rather than the walk a comparison must take.

use focused_fuel_harness::{assert_equal, assert_less, measure, Probe};

/// Dense spine depths, a fourfold step apart, so a walk's fuel would grow
/// sixteenfold across them.
const DEPTHS: [u64; 3] = [1_000, 4_000, 16_000];

/// Clone operands answer every identity-law fast path without a walk.
///
/// The fast paths are comparison (`order_reflexive`), join and meet
/// idempotence (`merge_idempotent`, `meet_idempotent`), the coincident hull
/// (`span_with_self_is_coincident`), the n-ary folds' collapse of adjacent
/// clones, and the metrics (`distance_to_self_is_zero`,
/// `lag_to_self_is_zero`).
///
/// Form: equal fuel (F-eq) over the operands' depth, for each operation.
#[test]
fn clone_operands_answer_without_a_walk() {
    for probe in [
        Probe::CloneCompare,
        Probe::CloneJoin,
        Probe::CloneMeet,
        Probe::CloneSpan,
        Probe::CloneJoinAll,
        Probe::CloneMeetAll,
        Probe::CloneSpanAll,
        Probe::CloneDistance,
        Probe::CloneLag,
    ] {
        assert_equal(probe, &DEPTHS);
    }
}

/// Comparing byte-equal operands in distinct buffers takes the full sweep:
/// the clone fast path must not fire across buffers.
///
/// Form: strict saving (F-lt) of the clone comparison under the byte-equal
/// one at every depth, and of each depth's byte-equal comparison under the
/// next, so the sweep's cost grows with the operands.
#[test]
fn distinct_buffers_take_the_comparison_sweep() {
    let mut previous = None;
    for depth in DEPTHS {
        let clone = measure(Probe::CloneCompare, depth, 0);
        let redecoded = measure(Probe::RedecodedCompare, depth, 0);
        assert_less(
            &format!("clone vs byte-equal compare at depth {depth}"),
            clone,
            redecoded,
        );
        if let Some((smaller, fuel)) = previous {
            assert_less(
                &format!("byte-equal compare at depth {smaller} vs {depth}"),
                fuel,
                redecoded,
            );
        }
        previous = Some((depth, redecoded));
    }
}

/// Byte-equal operands in distinct buffers answer join, meet, span,
/// distance, and lag by comparing their bytes, not by walking their streams
/// or folding through the accumulator.
///
/// The comparison of the same operands must take the full sweep (the test
/// above), and each of these operations, without its byte-equality rung,
/// would take at least that sweep. Form: strict saving (F-lt) of each
/// operation under the comparison of the same operands, at every depth.
#[test]
fn byte_equal_operands_answer_by_the_byte_compare() {
    for depth in DEPTHS {
        let sweep = measure(Probe::RedecodedCompare, depth, 0);
        for probe in [
            Probe::RedecodedJoin,
            Probe::RedecodedMeet,
            Probe::RedecodedSpan,
            Probe::RedecodedDistance,
            Probe::RedecodedLag,
        ] {
            assert_less(
                &format!("{probe:?} vs the comparison sweep at depth {depth}"),
                measure(probe, depth, 0),
                sweep,
            );
        }
    }
}

/// An empty operand answers every lattice identity and absorption rung
/// without a walk, in both orders, at every entry point.
///
/// The entry points are the operators, the `|=` and `&=` assignments
/// (`acc |= v` on an empty `acc` is the first join every fold accumulator
/// makes), `span`, and two-element folds.
///
/// Without these rungs the general walks produce identical values, so no
/// value law can tell whether they exist; only their cost can. Form: equal
/// fuel (F-eq) over the nonempty operand's depth, for each entry point.
#[test]
fn empty_operands_answer_without_a_walk() {
    for probe in [
        Probe::EmptyJoinLeft,
        Probe::EmptyJoinRight,
        Probe::EmptyMeetLeft,
        Probe::EmptyMeetRight,
        Probe::EmptySpanLeft,
        Probe::EmptySpanRight,
        Probe::EmptyJoinAssignLeft,
        Probe::EmptyJoinAssignRight,
        Probe::EmptyMeetAssignLeft,
        Probe::EmptyMeetAssignRight,
        Probe::EmptyJoinAllRight,
        Probe::EmptyJoinAllLeft,
        Probe::EmptyMeetAllRight,
        Probe::EmptyMeetAllLeft,
    ] {
        assert_equal(probe, &DEPTHS);
    }
}
