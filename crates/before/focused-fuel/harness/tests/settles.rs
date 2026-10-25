//! The rank settle's multi-arming and pair legs, and the measure folds on
//! versions whose answer is a wide counter, at couplings no board family
//! drives.

use focused_fuel_harness::{
    assert_flat, assert_flat_within, assert_no_product, measure_per, Probe,
};

/// Distance and lag on the plateau puncture paired with an arming train
/// stay flat per byte: one co-sweep drives both settle cases, the plateau
/// side parking one wide drift and the train side settling its armings.
///
/// Form: flat per unit (F-lin) per encoded byte across a doubling of both
/// operands.
#[test]
fn plateau_train_pair_is_flat_per_byte() {
    assert_flat(
        "plateau-train pair per byte",
        measure_per(Probe::PairPlateauTrain, 1, 0),
        measure_per(Probe::PairPlateauTrain, 2, 0),
    );
}

/// Rank on the same-sign arming train stays flat per byte across two
/// arming-count doublings.
///
/// Every level of the settle's reduction holds the full window density under
/// full-width parked sums, and a doubling adds one level to a logarithmic
/// stack while the bytes double.
///
/// The board's arming-train family takes the alternating schedule. Form:
/// flat per unit (F-lin) per encoded byte across each doubling.
#[test]
fn same_sign_trains_are_flat_per_byte() {
    let runs = [4, 8, 16].map(|blocks| measure_per(Probe::SameSignTrainRank, blocks, 0));
    assert_flat("same-sign train 4 to 8 blocks", runs[0], runs[1]);
    assert_flat("same-sign train 8 to 16 blocks", runs[1], runs[2]);
}

/// Neither measure fold couples the tail length to the answer's width on the
/// wide-base tiny-tail family: the wide component is paid once, not per
/// leaf.
///
/// Form: no product term (F-mix) over the grid of tail lengths 128 and 256
/// and answer widths `10^2000` and `10^4000`, for `min_ticks` and for `rank`.
#[test]
fn measure_folds_are_additive_on_wide_base_tiny_tail() {
    for probe in [Probe::WideBaseTinyTailMinTicks, Probe::WideBaseTinyTailRank] {
        let grid = [(128, 2_000), (256, 2_000), (128, 4_000), (256, 4_000)]
            .map(|(tail, width)| measure_per(probe, tail, width).0);
        assert_no_product(&format!("{probe:?}"), grid);
    }
}

/// Both measure folds stay flat per byte on the wide ladder, whose input
/// itself carries the product of party count and width: near-equal wide
/// subtree minima are each discriminated within the input bits that spelled
/// them.
///
/// Every rung's minimum is discriminated within the bits that spelled it, so
/// the fuel per byte should not move at all across the grid; the band is
/// ×1.10 rather than ×1.25 because nothing here, unlike a doubling of a
/// settle's levels, adds a legitimate logarithmic term. Form: flat per unit
/// (F-lin) per encoded byte, within ×1.10 of the smallest cell, at every cell
/// of the grid of 64 and 128 parties and widths `10^1500` and `10^3000`, for
/// `min_ticks` and for `rank`.
#[test]
fn measure_folds_are_flat_per_byte_on_wide_ladder() {
    for probe in [Probe::WideLadderMinTicks, Probe::WideLadderRank] {
        let base = measure_per(probe, 64, 1_500);
        for (parties, width) in [(128, 1_500), (64, 3_000), (128, 3_000)] {
            assert_flat_within(
                &format!("{probe:?} at {parties} parties and width {width} per byte"),
                base,
                measure_per(probe, parties, width),
                (11, 10),
            );
        }
    }
}
