//! Flatness checks for minimum-tick comb families.

use super::*;

/// Blocks of the min_ticks comb bands' small runs (the large runs
/// double both comb parameters, doubling the operand's size).
const MIN_TICKS_COMB_SMALL: usize = 1_000;

/// Absolute touch ceilings at two scales for min_ticks on the
/// pure comb, measured ×1.25 (the record and every re-pin's
/// movement live in the pin commits).
///
/// The range-minimum tracker fold reads flat per encoded byte across the
/// doubling; an accounting that circulates the full plateau width
/// per closing node reads superlinear and exceeds these ceilings.
const MIN_TICKS_PURE_COMB_CEILINGS: [u64; 2] = [2_785, 5_558];

/// Absolute touch ceilings at two scales for min_ticks on the
/// reveal comb, measured ×1.25 (the record and every re-pin's
/// movement live in the pin commits).
const MIN_TICKS_REVEAL_COMB_CEILINGS: [u64; 2] = [12_958, 25_907];

/// min_ticks is linear on the pure comb: per-byte touch work stays flat
/// (×1.25) across a joint `(k, b)` doubling, under
/// absolute two-scale ceilings.
///
/// `k` wide plateau leaves ride one wide code and unit deltas over
/// a zero floor, so every closing comb node's minimum is the floor:
/// the fold must subtract the same value `k` times. An accounting
/// that folds the minimum's width per closing node pays the plateau
/// width `k` times from one funding code and reads superlinear;
/// one contribution counts every subtree using the floor and settles it once, so
/// the flatness bound holds with the closed form
/// `min_ticks = k·2^b` exact at both scales.
#[test]
fn skyline_min_ticks_pure_comb_is_flat_per_unit() {
    let k = MIN_TICKS_COMB_SMALL;
    let expected = |k: usize| (BigUint::ONE << k) * BigUint::from(k as u64);
    let small = min_ticks_family_run(Shape::PureComb.build2(k, k), &expected(k));
    let large = min_ticks_family_run(Shape::PureComb.build2(2 * k, 2 * k), &expected(2 * k));
    assert_ceilings(
        "skyline_min_ticks_pure_comb",
        &small,
        &large,
        MIN_TICKS_PURE_COMB_CEILINGS,
    );
    assert_flat(
        "min_ticks_pure_comb_touches",
        "byte",
        (small.touches, small.bytes),
        (large.touches, large.bytes),
    );
}

/// min_ticks is linear on the reveal comb: per-byte touch work stays flat
/// (×1.25) across a joint
/// `(k, b)` doubling, under absolute two-scale ceilings.
///
/// The reveal comb's `k` sibling sites share one `2^b`-wide minimum
/// over a zero floor, so the sweep's minimum tracking crosses the
/// width-`b` boundary between the floor and the site plateau at
/// every site — the close-reveal case. The range-minimum tracker moves that
/// boundary between the difference stack and the latent register by
/// moves alone, so the flatness bound holds with
/// the closed form `min_ticks = k·2^b` exact at both scales (an
/// accounting that re-folds the boundary's width per site reads
/// superlinear here).
#[test]
fn skyline_min_ticks_reveal_comb_is_flat_per_unit() {
    let k = MIN_TICKS_COMB_SMALL;
    let expected = |k: usize| (BigUint::ONE << k) * BigUint::from(k as u64);
    let small = min_ticks_family_run(Shape::RevealComb.build2(k, k), &expected(k));
    let large = min_ticks_family_run(Shape::RevealComb.build2(2 * k, 2 * k), &expected(2 * k));
    assert_ceilings(
        "skyline_min_ticks_reveal_comb",
        &small,
        &large,
        MIN_TICKS_REVEAL_COMB_CEILINGS,
    );
    assert_flat(
        "min_ticks_reveal_comb_touches",
        "byte",
        (small.touches, small.bytes),
        (large.touches, large.bytes),
    );
}
