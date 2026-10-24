//! A wide-by-dense rank family that prices deferred-height settlement.
//!
//! The input parks one `w`-digit height before a run containing `Θ(w)` interval
//! mass. Rank must eventually multiply those two input-funded quantities. The
//! family puts the cancelling descent after the measured aggregation, so that
//! unavoidable product cannot disappear through boundary cancellation.
//!
//! The densify counter prices the images the settle zero-fills around that
//! multiplication. The same integrator implements distance and lag, so their
//! wide-by-dense claim shares this witness.

use super::min_ticks_from_big;
use before::testing::meter;
use before::testing::meter::registry::Shape;
use num_bigint::BigUint;

/// One public `Version::rank` run over `WA(w, w)`.
///
/// Carries `min_ticks`' closed form as the cross-fold semantic leg
/// (proving the generator builds the gap spine and the wide arming
/// this band reasons about).
fn run(w: usize) -> (u64, u64) {
    let v = Shape::WideArming.build2(w, w).version();
    let bytes = v.encode().len() as u64;
    let expected = BigUint::from(w as u64)
        + (BigUint::ONE << (32 * w))
        + (BigUint::ONE << meter::FREEZE_DRIFT_BITS)
        + 3u8;
    assert_eq!(
        v.min_ticks(),
        min_ticks_from_big(&expected),
        "the family's stored-code sum disagrees with min_ticks: the \
         generator does not build the tree this band reasons about"
    );
    meter::reset_densified_digits();
    let rank = v.rank();
    std::hint::black_box(rank);
    (bytes, meter::densified_digits())
}

/// Suffix digits (and arming digits) of the band's small run (the
/// large run doubles both).
const WIDE_ARMING_SMALL: usize = 500;

/// Absolute two-scale densify ceilings for rank on the wide-arming
/// family: the measured record ×1.25, rounded up (the record and every
/// re-pin's movement live in the pin commits).
///
/// Flat per encoded byte across the doubling: the settle's densified
/// spans are the dense trailing window's, which scale with the knob
/// exactly as the input does. An image sized by a cluster's absolute
/// digit position instead reads the never-written scale prefix into
/// every image and leaves flatness while inflating the absolute record;
/// the position axis itself is isolated (and killed) by the
/// hoisted-window band, where spans stay fixed as positions grow.
const WIDE_ARMING_DENSIFY_CEILINGS: (u64, u64) = (2_580, 5_160);

/// rank's densified-image fill is flat per byte on the wide-arming family:
/// the densify column stays within ×1.25 per byte across a `WA(w, w)`
/// doubling, under absolute two-scale ceilings.
///
/// `WA(w, w)` scales both factors of the settle's one aggregate product
/// with the input; the fold's time on this family is judged in fuel by the
/// board's `version_rank` cell.
#[test]
fn rank_wide_arming_densify_is_flat_per_unit() {
    let (small_bytes, small) = run(WIDE_ARMING_SMALL);
    let (large_bytes, large) = run(2 * WIDE_ARMING_SMALL);
    eprintln!(
        "MEASURED rank_wide_arming: densify small={small}/{small_bytes}B \
         large={large}/{large_bytes}B"
    );
    let ceilings = WIDE_ARMING_DENSIFY_CEILINGS;
    assert!(
        small <= ceilings.0 && large <= ceilings.1,
        "rank's densify column exceeds the pinned ceilings on the wide-arming \
         family ({small}/{small_bytes}B -> {large}/{large_bytes}B against {} / {})",
        ceilings.0,
        ceilings.1,
    );
    assert!(
        u128::from(large) * u128::from(small_bytes) * 4
            <= u128::from(small) * u128::from(large_bytes) * 5,
        "rank's densify column grew more than x1.25 per byte across the \
         wide-arming doubling ({small}/{small_bytes}B -> {large}/{large_bytes}B)",
    );
}
