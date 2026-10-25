//! Carry-boundary crossings and freeze schedules the board's families do not
//! reach.
//!
//! They are the boundary comb against the empty version and a one-tick
//! version, the rank freeze threshold's two sides, and the pair queries
//! against mates no board family builds.

use focused_fuel_harness::{assert_flat, measure_per, Probe};

/// The comparison's running difference crosses the `2^k` carry boundary at
/// every delta of the boundary comb `C(k, k)` compared against the empty
/// version, and each crossing stays amortized O(1).
///
/// The board compares each family with its own ticked successor, whose
/// difference never builds the comb's width. Form: flat per unit (F-lin) per
/// delta code across a `k` doubling.
#[test]
fn comparison_cliff_cost_is_flat_per_delta() {
    assert_flat(
        "comb vs empty per delta",
        measure_per(Probe::CliffCompareEmpty, 512, 0),
        measure_per(Probe::CliffCompareEmpty, 1_024, 0),
    );
}

/// The join emitter's running difference crosses the `2^k` carry boundary
/// at every delta of the boundary comb joined with a one-tick version, and
/// each crossing stays amortized O(1).
///
/// This is the emission side of cliff-freedom, which join, meet, `recv`,
/// `sync`, and the folds all ride.
///
/// Form: flat per unit (F-lin) per delta code across a `k` doubling.
#[test]
fn join_cliff_cost_is_flat_per_delta() {
    assert_flat(
        "comb join one tick per delta",
        measure_per(Probe::CliffJoinOneTick, 512, 0),
        measure_per(Probe::CliffJoinOneTick, 1_024, 0),
    );
}

/// The rank fold's cost on the wide-tooth comb rides the live component,
/// paid by each tooth's own code, on both sides of the freeze threshold.
///
/// The threshold is a 256-bit digit bound: at 192-bit teeth, where bounded oscillation never
/// freezes, and at 300-bit teeth, where every fold evicts the live component
/// at the drift's own funded width.
///
/// Form: flat per unit (F-lin) per encoded byte across a doubling of the
/// comb's magnitude and tooth count, at each tooth width.
#[test]
fn rank_wide_tooth_cost_is_flat_on_both_sides_of_the_freeze() {
    for bits in [192, 300] {
        assert_flat(
            &format!("rank of {bits}-bit teeth per byte"),
            measure_per(Probe::RankWideTooth, 1, bits),
            measure_per(Probe::RankWideTooth, 2, bits),
        );
    }
}

/// Distance and lag stay linear as the freeze-position family's freeze
/// count doubles, against its descending unit spine.
///
/// Form: flat per unit (F-lin) per encoded byte across a block-count
/// doubling.
#[test]
fn freeze_position_pair_queries_are_flat_per_byte() {
    assert_flat(
        "freeze-position pair queries per byte",
        measure_per(Probe::DistanceFreezePosition, 1_000, 0),
        measure_per(Probe::DistanceFreezePosition, 2_000, 0),
    );
}

/// Distance and lag stay linear on the deferral re-arm pair, where one
/// operand's cheap codes trigger freezes and deferrals of drift the other
/// operand's wide codes deposited.
///
/// The reduction records each deferred height with the relative widths it
/// covers and settles the entries together.
///
/// Form: flat per unit (F-lin) per encoded byte across a block-count
/// doubling.
#[test]
fn promotion_rearm_pair_queries_are_flat_per_byte() {
    assert_flat(
        "promotion re-arm pair queries per byte",
        measure_per(Probe::DistancePromotionRearm, 1_000, 0),
        measure_per(Probe::DistancePromotionRearm, 2_000, 0),
    );
}
