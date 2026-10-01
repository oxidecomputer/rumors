//! Count-scaling checks for `Version::ticks`.

use super::*;

/// The flatness pin: `O(|v| + |p| + log n)` as a committed two-point
/// check, not prose.
///
/// On each tick-designated family the whole cost
/// movement from `ticks(512)` to `ticks(4096)` — three doublings — must
/// sit inside the boundary codes' gamma-width delta band: the two codes
/// that carry the count widen by 2 bits per doubling each, and no other
/// column may move beyond a word of slack. An implementation iterating
/// any fraction of the count moves every column by ~8x here and cannot
/// hide in a constant; a dead meter reads zero movement AND a zero
/// point, which the amplification board's growth checks already reject.
#[test]
fn ticks_flatness_holds_the_log_band() {
    let cases: Vec<(&str, Version, Party)> = vec![
        (
            "dense",
            version_of(&Shape::Dense.build1(DENSE_DEPTH)),
            Party::seed(),
        ),
        (
            "nested-wide",
            version_of(&Shape::Bigroot.build2(TICK_CROSS_SCALE, TICK_CROSS_SCALE)),
            party_of(&Shape::NestedFullId.build1(TICK_CROSS_SCALE)),
        ),
        (
            "mirror-wide",
            version_of(&Shape::WideTail.build2(TICK_CROSS_SCALE, TICK_CROSS_SCALE)),
            party_of(&Shape::NestedLeftFullId.build1(TICK_CROSS_SCALE)),
        ),
    ];
    for (name, v, p) in &cases {
        let lo = ticks_counters(v, p, TICKS_POINT_LO);
        let hi = ticks_counters(v, p, TICKS_POINT_HI);
        let moved = [
            ("scan", lo.0, hi.0, TICKS_FLATNESS_SCAN_BAND),
            ("touch", lo.1, hi.1, TICKS_FLATNESS_TOUCH_BAND),
        ];
        for (col, at_lo, at_hi, band) in moved {
            let delta = at_hi.abs_diff(at_lo);
            eprintln!("MEASURED ticks_flatness {name}/{col}: lo={at_lo} hi={at_hi} delta={delta}");
            assert!(
                delta <= band,
                "{name}/{col}: ticks({}) -> ticks({}) moved {delta}                  (from {at_lo} to {at_hi}), outside the gamma-width band {band}",
                TICKS_POINT_LO,
                TICKS_POINT_HI,
            );
        }
    }
}

/// One `ticks(n)` run's scanned bits and accumulator touches on fresh
/// counters — the flatness pin's probe.
fn ticks_counters(v: &Version, p: &Party, n: u64) -> (u64, u64) {
    let mut v = v.clone();
    meter::reset_scan_bits();
    suanpan::touch_meter::reset();
    v.ticks(p, n);
    (meter::scan_bits(), suanpan::touch_meter::touches())
}

/// One `ticks(n)` run at an arbitrary-width count, on the operand's
/// post-fill tree, with the exact `min_ticks` movement as the value
/// leg.
///
/// One public tick is applied *outside* the metered body first: on the
/// once-ticked tree the fill is the identity (fill is idempotent, and
/// a grow opens no fillable structure), so the metered `ticks(n)` is
/// the pure grow branch, where registering `n` events grows the
/// minimum tick count by exactly `n` — the fill branch instead
/// collapses owned structure and moves `min_ticks` by a
/// shape-dependent amount, which the committed small-count
/// differentials pin byte-for-byte against iterated ticks.
fn ticks_counters_wide(v: &Version, p: &Party, n: &before::Count) -> (u64, u64) {
    let mut v = v.clone();
    v.tick(p);
    let before_ticks = v.min_ticks();
    meter::reset_scan_bits();
    suanpan::touch_meter::reset();
    v.ticks(p, n.clone());
    let counters = (meter::scan_bits(), suanpan::touch_meter::touches());
    assert_eq!(
        v.min_ticks(),
        before_ticks + n.clone(),
        "a grow-branch ticks(n) must grow the minimum tick count by exactly n"
    );
    counters
}

/// The wide-count pin's count width in bits (the second wide point
/// doubles it): far past every machine integer.
///
/// The count's own arithmetic — the splice's site addition, the
/// changed branch's decrement, the count-carrying gamma codes — runs
/// at genuinely wide operands here.
///
/// Both wide points sit *above* the crosses' site-value width
/// ([`TICK_CROSS_SCALE`] bits), because the emitted stream's count
/// dependence is piecewise-linear with a knee exactly there: below it
/// the output carries the count in one gamma code (span `2·bits(n)`),
/// above it the min-lift re-coding around the grown site carries the
/// count's excess over the site again (span `4·bits(n) − 2·site`, the
/// law the wide points' scan spans track). Judging two points in one
/// regime keeps the ratio band tight; a probe straddling the knee
/// legitimately reads up to ×3 without any superlinearity.
const TICKS_WIDE_COUNT_BITS: usize = 8_192;

/// The count-attributable growth bound: doubling the count's width may at most
/// double the measured scan and accumulator cost, with ×1.25 slack plus a word
/// of boundary slack per column.
///
/// `ticks(n)` claims `O(|v| + |p| + log n)`: the whole `n`-dependence
/// visible to these counters is the count-carrying stream work, so scan cost
/// above the word-count baseline must scale linearly in `bits(n)`.
///
/// Dense and nested-wide sit below the site-width knee; mirror-wide
/// sits in the slope-4 regime above it (both wide points exceed its
/// [`TICK_CROSS_SCALE`]-bit site width by construction), and both
/// regimes are width-linear, so the ratio band holds across all
/// three. The touch span carries no count dependence because count arithmetic
/// runs outside the accumulator. This bound does not measure that standalone
/// `BigUint` arithmetic.
const TICKS_WIDE_GROWTH_NUM: u64 = 5;

/// See [`TICKS_WIDE_GROWTH_NUM`]: the ratio denominator.
const TICKS_WIDE_GROWTH_DEN: u64 = 2;

/// The wide-count flatness pin: `ticks(n)` keeps its scan and accumulator work
/// width-linear in counts far past every machine integer.
///
/// Three points per family — the word-count baseline `n₀ = 512`, an
/// 8,192-bit count, and its width doubling — judged as two spans: the
/// count-attributable movement (each counter above its baseline) may
/// at most double, ×1.25, when the width doubles. The scan span
/// additionally carries a liveness floor of `2 · bits(n)` (the
/// widened count-carrying code must be written), so a dead meter or a
/// count that never reaches the splice cannot pass vacuously; the
/// value leg inside the probe holds the `min_ticks` movement exactly
/// equal to `n` at every point, so the wide registration is proven to
/// have happened before any cost is judged.
#[test]
fn ticks_wide_count_flatness_holds_the_width_band() {
    let n1 = power_of_two(TICKS_WIDE_COUNT_BITS);
    let n2 = power_of_two(2 * TICKS_WIDE_COUNT_BITS);
    let cases: Vec<(&str, Version, Party)> = vec![
        (
            "dense",
            version_of(&Shape::Dense.build1(DENSE_DEPTH)),
            Party::seed(),
        ),
        (
            "nested-wide",
            version_of(&Shape::Bigroot.build2(TICK_CROSS_SCALE, TICK_CROSS_SCALE)),
            party_of(&Shape::NestedFullId.build1(TICK_CROSS_SCALE)),
        ),
        (
            "mirror-wide",
            version_of(&Shape::WideTail.build2(TICK_CROSS_SCALE, TICK_CROSS_SCALE)),
            party_of(&Shape::NestedLeftFullId.build1(TICK_CROSS_SCALE)),
        ),
    ];
    for (name, v, p) in &cases {
        let base = ticks_counters_wide(v, p, &before::Count::from(TICKS_POINT_LO));
        let at1 = ticks_counters_wide(v, p, &n1);
        let at2 = ticks_counters_wide(v, p, &n2);
        let spans = [
            ("scan", base.0, at1.0, at2.0),
            ("touch", base.1, at1.1, at2.1),
        ];
        for (col, c0, c1, c2) in spans {
            let d1 = c1.saturating_sub(c0);
            let d2 = c2.saturating_sub(c0);
            eprintln!(
                "MEASURED ticks_wide_count {name}/{col}: base={c0} w={c1} 2w={c2} \
                 spans {d1} -> {d2}"
            );
            assert!(
                d2 * TICKS_WIDE_GROWTH_DEN
                    <= d1 * TICKS_WIDE_GROWTH_NUM + 64 * TICKS_WIDE_GROWTH_DEN,
                "{name}/{col}: doubling the count's width from {TICKS_WIDE_COUNT_BITS} bits \
                 grew the count-attributable cost {d1} -> {d2}, past the width-linear band"
            );
        }
        // The scan span's liveness floor: one count-carrying gamma code
        // widened from word scale to `bits(n)` writes at least
        // `2·(bits(n) − 64)` fresh bits through the metered builder.
        assert!(
            at1.0.saturating_sub(base.0) >= 2 * (TICKS_WIDE_COUNT_BITS as u64 - 64),
            "{name}: a {TICKS_WIDE_COUNT_BITS}-bit count moved the scan column by only \
             {} bits — the widened count-carrying code is not being written through \
             the metered builder",
            at1.0.saturating_sub(base.0),
        );
    }
}

/// See [`TICKS_POINT_LO`].
const TICKS_POINT_HI: u64 = 4_096;
/// The scan movement band: up to two count-carrying codes x 2 bits per
/// doubling x 3 doublings.
///
/// One code carries the count on the committed families; the second
/// code's budget covers operand shapes where the successor repair
/// carries it too.
const TICKS_FLATNESS_SCAN_BAND: u64 = 12;
/// The touch movement band. Count arithmetic never reaches the accumulator,
/// so a small allowance covers unrelated boundary effects.
const TICKS_FLATNESS_TOUCH_BAND: u64 = 8;
