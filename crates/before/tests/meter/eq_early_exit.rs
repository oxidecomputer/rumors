//! A cost band proving comparison stops after a decisive prefix.
//!
//! Both operands have the same opening values at each scale, but one operand's
//! irrelevant tail doubles. Their first interval decides inequality before
//! either reader advances. A correct early exit therefore has the same fixed
//! cost at both scales; continuing into the tail exceeds the absolute pins.
//! Floors on both counters ensure that a disconnected meter cannot satisfy the
//! flat-cost assertion vacuously.

use before::testing::meter;
use before::testing::meter::registry::Shape;
use suanpan::touch_meter;

/// Tooth magnitude (bits) of the refuted pair's comb: fixed across
/// both scales.
///
/// Fixing the magnitude keeps the deciding first interval's own
/// codes — the work a valid early exit is allowed to read —
/// scale-independent, so only the tail the exit must NOT read
/// grows.
const EQ_EXIT_TOOTH_BITS: usize = 1_024;

/// One `eq` run over the first-interval-refuted pair: the comb's
/// input bytes (the refuted tail the exit must not read) and the
/// touch and scan counters over the sweep body alone.
struct Run {
    bytes: u64,
    touches: u64,
    #[cfg(feature = "scan-meter")]
    scan_bits: u64,
}

/// Run `eq` on the boundary comb `C(EQ_EXIT_TOOTH_BITS, teeth)`
/// against the empty version and record both counters over the
/// sweep body alone.
///
/// The comb's first preorder leaf sits at `2^EQ_EXIT_TOOTH_BITS − 1`
/// and the empty version's one plateau at 0, so the first elementary
/// interval's sign refutes `a <= b` and equality is decided there;
/// the verdict assert is the value leg. Carries a one-touch liveness
/// floor: the deciding interval folds the first delta into the
/// metered accumulator, so a sweep whose difference state left the
/// metered representation fails loudly here instead of passing the
/// absolute pins vacuously at zero.
fn run(teeth: usize) -> Run {
    let a = Shape::CliffComb.build2(EQ_EXIT_TOOTH_BITS, teeth).version();
    let b = before::Version::new();
    touch_meter::reset();
    #[cfg(feature = "scan-meter")]
    meter::reset_scan_bits();
    let verdict = meter::version::equal(&a, &b);
    let run = Run {
        bytes: (a.as_bytes().len() + b.as_bytes().len()) as u64,
        touches: touch_meter::touches(),
        #[cfg(feature = "scan-meter")]
        scan_bits: meter::scan_bits(),
    };
    assert!(
        !verdict,
        "the comb differs from the empty version at the first elementary interval"
    );
    assert!(
        run.touches >= 1,
        "eq_exit at {} operand bytes: zero digit touches under the one-touch \
         floor: the sweep's difference state is not running on the metered \
         accumulator",
        run.bytes,
    );
    run
}

/// Absolute two-scale touch pins for the refuted pair: per scale,
/// (ceiling, floor) = measured ×1.25 rounded up, ×0.75 rounded down
/// (the file doc's ceiling and improvement-tripwire conventions).
///
/// The reading is identical at both tooth-count scales: the deciding
/// interval performs one wide delta fold, unaffected by the tail.
const EQ_EXIT_TOUCH_PINS: (u64, u64) = (62, 36);

/// Absolute two-scale scan pins paired with
/// [`EQ_EXIT_TOUCH_PINS`], same conventions.
///
/// The record is identical at both scales: the two streams'
/// opening codes — dominated by the comb's
/// `2·EQ_EXIT_TOOTH_BITS + 1`-bit first absolute — read once,
/// tail-independent.
#[cfg(feature = "scan-meter")]
const EQ_EXIT_SCAN_PINS: (u64, u64) = (2_568, 1_540);

/// `eq` on a first-interval-refuted pair reads a constant prefix of
/// either stream: absolute touch and scan readings hold at both
/// tooth-count scales, so the decided sweep's cost is pinned
/// tail-independent.
///
/// Two scales, one fixed tooth magnitude: the deciding interval's
/// work is identical by construction, so the pins can only move if
/// the sweep reads past its decision — the early-exit contract of
/// the `sweep` module doc, enforced as committed numbers.
#[test]
fn eq_exit_cost_is_tail_independent_on_first_interval_refuted_pair() {
    let small = run(512);
    let large = run(1_024);
    #[cfg(feature = "scan-meter")]
    let scan_cols = (small.scan_bits, large.scan_bits);
    #[cfg(not(feature = "scan-meter"))]
    let scan_cols = ("off", "off");
    eprintln!(
        "MEASURED eq_exit: small touches={}/{}B scan={:?} large touches={}/{}B scan={:?}",
        small.touches, small.bytes, scan_cols.0, large.touches, large.bytes, scan_cols.1,
    );
    for (scale, run) in [("small", &small), ("large", &large)] {
        assert!(
            run.touches <= EQ_EXIT_TOUCH_PINS.0,
            "eq_exit_{scale}: {} touches exceed the pinned absolute ceiling {}: \
             the decided sweep is reading the refuted operand's tail",
            run.touches,
            EQ_EXIT_TOUCH_PINS.0,
        );
        assert!(
            run.touches >= EQ_EXIT_TOUCH_PINS.1,
            "eq_exit_{scale}: {} touches under the {} improvement tripwire \
             (measured ×0.75): attribute the drop — a genuine improvement \
             re-pins the band; a dead meter is the bypass this floor catches",
            run.touches,
            EQ_EXIT_TOUCH_PINS.1,
        );
        #[cfg(feature = "scan-meter")]
        {
            assert!(
                run.scan_bits <= EQ_EXIT_SCAN_PINS.0,
                "eq_exit_{scale}: {} scanned bits exceed the pinned absolute \
                 ceiling {}: the decided sweep is reading the refuted operand's \
                 tail",
                run.scan_bits,
                EQ_EXIT_SCAN_PINS.0,
            );
            assert!(
                run.scan_bits >= EQ_EXIT_SCAN_PINS.1,
                "eq_exit_{scale}: {} scanned bits under the {} improvement \
                 tripwire (measured ×0.75): attribute the drop — a genuine \
                 improvement re-pins the band; a dead meter is the bypass this \
                 floor catches",
                run.scan_bits,
                EQ_EXIT_SCAN_PINS.1,
            );
        }
    }
}
