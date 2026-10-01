//! A wide-by-dense rank family that prices promotion settlement.
//!
//! The input parks one `w`-digit height before a run containing `Θ(w)` interval
//! mass. Rank must eventually multiply those two input-funded quantities. The
//! family puts the cancelling descent after the measured aggregation, so that
//! unavoidable product cannot disappear through boundary cancellation.
//!
//! The deterministic counters price the fold's operand, window, and result
//! traffic around that multiplication. The same integrator implements distance
//! and lag, so their wide-by-dense claim shares this witness.

use super::min_ticks_from_big;
use before::testing::meter;
use before::testing::meter::registry::Shape;
use num_bigint::BigUint;
use suanpan::touch_meter;

/// One public `Version::rank` run over `WA(w, w)`.
///
/// Carries `min_ticks`' closed form as the cross-fold semantic leg
/// (proving the generator builds the gap spine and the wide arming
/// this band reasons about) and the one-touch-per-operand-byte
/// liveness floor.
fn run(w: usize) -> (u64, u64, u64) {
    let v = Shape::WideArming.build2(w, w).version();
    let bytes = v.encode().len() as u64;
    let expected =
        BigUint::from(w as u64) + (BigUint::ONE << (32 * w)) + (BigUint::ONE << 288usize) + 3u8;
    assert_eq!(
        v.min_ticks(),
        min_ticks_from_big(&expected),
        "the family's stored-code sum disagrees with min_ticks: the \
         generator does not build the tree this band reasons about"
    );
    touch_meter::reset();
    meter::reset_densified_digits();
    let rank = v.rank();
    std::hint::black_box(rank);
    let touches = touch_meter::touches();
    let densified = meter::densified_digits();
    assert!(
        touches >= bytes,
        "rank at {bytes} operand bytes: {touches} digit touches under \
         the one-per-byte floor: the fold's accumulator work is not \
         metered",
    );
    (bytes, touches, densified)
}

/// Suffix digits (and arming digits) of the band's small run (the
/// large run doubles both).
const WIDE_ARMING_SMALL: usize = 500;

/// Absolute two-scale touch ceilings for rank on the
/// wide-arming family, measured ×1.25 (the record and every
/// re-pin's movement live in the pin commits).
///
/// Flat per encoded byte across the doubling; a schoolbook settle that pays
/// the parked width once per window digit would be quadratic here.
const WIDE_ARMING_CEILINGS: [u64; 2] = [43_427, 86_716];

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

/// rank is flat per byte on the wide-arming family: per-byte touch work
/// stays within ×1.25 across a `WA(w, w)` doubling,
/// under absolute two-scale ceilings.
///
/// `WA(w, w)` scales both factors of the settle's one aggregate
/// product with the input, so a settle that pays their schoolbook
/// product — or one whose product traffic stops being metered —
/// moves this band, in opposite directions: the schoolbook charge
/// reads ~×2 per byte per doubling (both factors scale with the
/// input), and a dark tap reads under the liveness floor in
/// [`run`].
#[test]
fn rank_wide_arming_is_flat_per_unit() {
    let (small_bytes, small_touches, small_densify) = run(WIDE_ARMING_SMALL);
    let (large_bytes, large_touches, large_densify) = run(2 * WIDE_ARMING_SMALL);
    eprintln!(
        "MEASURED rank_wide_arming: small={small_touches}/{small_bytes}B \
         (densify {small_densify}) \
         large={large_touches}/{large_bytes}B \
         (densify {large_densify})"
    );
    for (name, small, large, ceilings) in [
        (
            "touches",
            small_touches,
            large_touches,
            (WIDE_ARMING_CEILINGS[0], WIDE_ARMING_CEILINGS[1]),
        ),
        (
            "densified digits",
            small_densify,
            large_densify,
            WIDE_ARMING_DENSIFY_CEILINGS,
        ),
    ] {
        assert!(
            small <= ceilings.0 && large <= ceilings.1,
            "rank ({name}) exceeds the pinned ceilings on the wide-arming \
             family ({small}/{small_bytes}B -> {large}/{large_bytes}B \
             against {} / {})",
            ceilings.0,
            ceilings.1,
        );
        assert!(
            u128::from(large) * u128::from(small_bytes) * 4
                <= u128::from(small) * u128::from(large_bytes) * 5,
            "rank ({name}) grew more than x1.25 per byte across the \
             wide-arming doubling ({small}/{small_bytes}B -> \
             {large}/{large_bytes}B): the settle is paying a settle \
             product's width times its density again",
        );
    }
}
