//! A rank family that separates live spans from absolute positions.
//!
//! The settle's densified-image span case. The family is the wide-arming
//! close with its block terminal deepened into a dense tail: the tail
//! funds no window density, no settle width, and no freeze — its consumed
//! interval mass is one contiguous run whose balanced spelling compacts
//! to O(1) digits — but it hoists the absolute digit position of every
//! settle cluster (the trailing window's punctured run, the block's
//! banked mass) by ~t/32 base-2^32 digits while every cluster's span
//! stays put. So across a tail doubling, span-priced work (the walk, the
//! folds, the settle products, the images the settle densifies) grows
//! only with the tail's own linear scan work, and work priced by a
//! cluster's absolute position — the case invisible to the width and
//! touch counters, because a zeroed image byte no digit lands on enters
//! no operand width and touches no accumulator digit — scales with the
//! knob instead.

use super::ticks_from_big;
use before::testing::meter;
use before::testing::meter::registry::Shape;
use num_bigint::BigUint;
use suanpan::touch_meter;

/// Arming width (base-2^32 digits) of every run: wide enough that the
/// trailing window's unit-gap digits sit far inside every settle
/// factor's cluster gap limit, so the punctured run densifies as one
/// cluster.
const HOISTED_WINDOW_WIDTH: usize = 12;

/// Gap count of every run: the trailing window's punctured digit span,
/// the family's fixed span axis.
const HOISTED_WINDOW_GAPS: usize = 40;

/// Tail depth of the band's small run (the large run doubles it).
///
/// A position hoist of ~8× the window span in digits, so span-priced
/// and position-priced densification separate by nearly an order of
/// magnitude before the doubling separates them again.
const HOISTED_WINDOW_SMALL_TAIL: usize = 10_240;

/// One public `Version::rank` run over `HW(w, d, t)` at the band's
/// fixed width and gap knobs, with touch and densification counts.
///
/// Carries `min_ticks`' closed form as the cross-fold semantic leg —
/// tail-independent by construction, so it also proves the tail adds
/// no stored-base mass — and the one-touch-per-operand-byte liveness
/// floor.
fn run(t: usize) -> (u64, u64, u64) {
    let v = Shape::HoistedWindow
        .build3(HOISTED_WINDOW_WIDTH, HOISTED_WINDOW_GAPS, t)
        .version();
    let bytes = v.encode().len() as u64;
    let expected = BigUint::from(HOISTED_WINDOW_GAPS as u64)
        + (BigUint::ONE << (32 * HOISTED_WINDOW_WIDTH))
        + (BigUint::ONE << 288usize)
        + 3u8;
    assert_eq!(
        v.min_ticks(),
        ticks_from_big(&expected),
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

/// Absolute two-scale touch ceilings for rank on the
/// hoisted-window family, measured ×1.25 (the record and every re-pin's
/// movement live in the pin commits).
///
/// Flat per encoded byte across the tail doubling: the tail's unit deltas
/// add proportional accumulator work while the settle clusters do not grow.
const HOISTED_WINDOW_CEILINGS: [u64; 2] = [15_952, 29_752];

/// rank is flat per byte on the hoisted-window family: per-byte touch
/// work stays within ×1.25 across a tail doubling, under
/// absolute two-scale ceilings.
///
/// The tail doubling moves only the settle clusters' absolute digit
/// positions; any accumulator work priced by such a position —
/// a scaled read walking a never-written prefix, a settle product
/// re-based on an absolute index — grows superlinearly here while the
/// input grows only by tail bits.
#[test]
fn rank_hoisted_window_is_flat_per_unit() {
    let (small_bytes, small_touches, _) = run(HOISTED_WINDOW_SMALL_TAIL);
    let (large_bytes, large_touches, _) = run(2 * HOISTED_WINDOW_SMALL_TAIL);
    eprintln!(
        "MEASURED rank_hoisted_window: small={small_touches}/{small_bytes}B \
         large={large_touches}/{large_bytes}B"
    );
    assert!(
        small_touches <= HOISTED_WINDOW_CEILINGS[0] && large_touches <= HOISTED_WINDOW_CEILINGS[1],
        "rank touches exceed the pinned ceilings on the hoisted-window \
         family ({small_touches}/{small_bytes}B -> \
         {large_touches}/{large_bytes}B against {} / {})",
        HOISTED_WINDOW_CEILINGS[0],
        HOISTED_WINDOW_CEILINGS[1],
    );
    assert!(
        u128::from(large_touches) * u128::from(small_bytes) * 4
            <= u128::from(small_touches) * u128::from(large_bytes) * 5,
        "rank touches grew more than x1.25 per byte across the \
         hoisted-window tail doubling ({small_touches}/{small_bytes}B -> \
         {large_touches}/{large_bytes}B): work is scaling with the settle \
         clusters' absolute positions",
    );
}

/// Absolute two-scale densify ceilings for rank on the hoisted-window
/// family: the measured record ×1.25, rounded up (the record and every
/// re-pin's movement live in the pin commits).
///
/// Judged absolute, never per byte: the tail doubling adds no window
/// density and no settle width, so the densified spans — and this
/// column with them — must not grow across it at all. An image sized
/// by a cluster's absolute digit position grows with the tail knob —
/// roughly ×2 across the doubling — while a span-priced column does not
/// move at all.
const HOISTED_WINDOW_DENSIFY_CEILINGS: [u64; 2] = [105, 105];

/// The densify liveness floor at both scales: two span-wide images per
/// charge of the trailing window's punctured cluster.
///
/// The premise is the mechanism's irreducible per-charge work, never a
/// reading: the family's close settles at least one charge against the
/// trailing window, whose punctured run holds the `d` gap digits at
/// unit interior gaps — inside every settle factor's cluster gap limit
/// — so it densifies as one multi-digit cluster of span at least `d`,
/// and every multi-digit cluster's densification zero-fills two
/// span-wide images. A run under this floor means the settle stopped
/// densifying the window this band exists to price, and every densify
/// ceiling above it would be passing vacuously.
const HOISTED_WINDOW_DENSIFY_FLOOR: u64 = 2 * HOISTED_WINDOW_GAPS as u64;

/// rank's densified-image fill is span-priced on the hoisted-window
/// family: the densify column stays within ×1.25 *absolute* across the
/// tail doubling, under two-scale ceilings and over the two-image
/// liveness floor.
///
/// The tail moves cluster positions only, so this column must not move
/// with it. This is the row the width and touch counters cannot express: an
/// image sized by a cluster's absolute digit position zero-fills
/// O(position) bytes per cluster that enter no operand width and touch
/// no accumulator digit, so every other column reads byte-identical
/// while this one scales with the tail knob.
#[test]
fn rank_hoisted_window_densify_span_band() {
    let (small_bytes, _, small_densify) = run(HOISTED_WINDOW_SMALL_TAIL);
    let (large_bytes, _, large_densify) = run(2 * HOISTED_WINDOW_SMALL_TAIL);
    eprintln!(
        "MEASURED rank_hoisted_window_densify: small={small_densify}dg/{small_bytes}B \
         large={large_densify}dg/{large_bytes}B"
    );
    assert!(
        small_densify >= HOISTED_WINDOW_DENSIFY_FLOOR
            && large_densify >= HOISTED_WINDOW_DENSIFY_FLOOR,
        "rank densified {small_densify} -> {large_densify} digits, under the \
         {HOISTED_WINDOW_DENSIFY_FLOOR}-digit two-image floor: the settle is \
         not densifying the trailing window, and the densify ceilings are \
         passing vacuously"
    );
    assert!(
        small_densify <= HOISTED_WINDOW_DENSIFY_CEILINGS[0]
            && large_densify <= HOISTED_WINDOW_DENSIFY_CEILINGS[1],
        "rank's densify column exceeds the pinned ceilings on the \
         hoisted-window family ({small_densify} -> {large_densify} against \
         {} / {})",
        HOISTED_WINDOW_DENSIFY_CEILINGS[0],
        HOISTED_WINDOW_DENSIFY_CEILINGS[1],
    );
    assert!(
        large_densify * 4 <= small_densify * 5,
        "rank's densify column grew more than x1.25 absolute across the \
         hoisted-window tail doubling ({small_densify} -> {large_densify}): \
         the densified images are being sized by the settle clusters' \
         absolute positions, not their spans",
    );
}
