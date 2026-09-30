//! Direct cost checks for the accumulator streams used by version walks.
//!
//! The digit-touch cost of the cliff-free accumulator on the worst-case
//! families' delta streams, with the sign read after every delta (the read
//! the sweeps depend on), plus the read-heavy stream where the sign folds
//! outnumber the writes. Each scenario runs at a base scale and its
//! doubling under the same per-stream ceiling — pinning the per-unit cost
//! (per delta, per coded byte where the deltas themselves widen, or per
//! sign read where reads dominate) *flat* across the doubling is the
//! linearity claim — plus an explicit cross-scale ratio bound. Touch
//! counts are deterministic, so the ceilings are exact-measured ×1.25 like
//! every other column; the counter exists only under the `touch-meter`
//! feature, which is the whole scenario's gate.

use std::cmp::Ordering;

use num_bigint::BigUint;
use suanpan::{touch_meter, Accumulator};

/// Fold a backend magnitude into the accumulator without materializing it.
fn fold_big(acc: &mut Accumulator, value: &BigUint, subtract: bool) {
    let limbs = value.iter_u64_digits();
    if subtract {
        acc.sub_limbs_shl(limbs, 0);
    } else {
        acc.add_limbs_shl(limbs, 0);
    }
}

/// Slack numerator over the measured value, matching the ×1.25 envelope
/// convention (denominator [`SLACK_DEN`]).
const SLACK_NUM: u64 = 5;

/// Slack denominator: ceilings and flatness bounds are measured ×5/4.
const SLACK_DEN: u64 = 4;

/// One accumulator stream measurement over the stream body (setup
/// excluded).
///
/// Carries the linearity denominator (delta count, coded bytes
/// where deltas widen, or sign reads where reads dominate), the
/// operations the body performed, and the digit touches counted.
struct Run {
    denominator: u64,
    /// Accumulator calls in the stream body (deltas plus sign
    /// reads): the touch counter's liveness floor, one touch per
    /// call.
    ///
    /// Every nonzero delta deposits into at least one digit, and
    /// every sign fold reads at least one — the floor is the
    /// mechanism's minimum possible work, not the typical work.
    ops: u64,
    touches: u64,
}

/// Assert a two-scale stream family's touch counter is alive (at
/// least one touch per operation performed), stays under its pinned
/// per-unit ceiling at both scales, and flat (×1.25) across the
/// doubling.
fn assert_flat(name: &str, small: &Run, large: &Run, ceiling_milli_per_unit: u64) {
    for run in [small, large] {
        eprintln!(
            "MEASURED accum_{name}: denominator={} touches={} milli_per_unit={}",
            run.denominator,
            run.touches,
            run.touches * 1000 / run.denominator,
        );
        assert!(
            run.touches >= run.ops,
            "accum_{name}: {} touches under {} operations: a deposit or \
             sign fold stopped counting, so every ceiling above would \
             hold vacuously",
            run.touches,
            run.ops,
        );
        assert!(
            u128::from(run.touches) * 1000
                <= u128::from(ceiling_milli_per_unit) * u128::from(run.denominator),
            "accum_{name}: {} touches over {} units exceed the pinned \
             {ceiling_milli_per_unit} milli-touches per unit",
            run.touches,
            run.denominator,
        );
    }
    assert!(
        u128::from(large.touches) * u128::from(small.denominator) * u128::from(SLACK_DEN)
            <= u128::from(small.touches) * u128::from(large.denominator) * u128::from(SLACK_NUM),
        "accum_{name}: per-unit touch cost grew more than ×1.25 across the \
         size doubling: {}/{} -> {}/{}",
        small.touches,
        small.denominator,
        large.touches,
        large.denominator,
    );
}

/// The boundary-comb delta stream: setup `2^k − 1`, then `2n` deltas of
/// `±1` oscillating across the `2^k` cliff, sign read after each.
fn comb_run(k: u32, n: usize) -> Run {
    let mut acc = Accumulator::new();
    fold_big(&mut acc, &((BigUint::from(1u8) << k as usize) - 1u8), false);
    touch_meter::reset();
    for _ in 0..n {
        acc.add_small(1);
        assert_eq!(acc.sign(), Ordering::Greater, "at 2^k");
        acc.sub_small(1);
        assert_eq!(acc.sign(), Ordering::Greater, "back at 2^k - 1");
    }
    Run {
        denominator: 2 * n as u64,
        ops: 4 * n as u64,
        touches: touch_meter::touches(),
    }
}

/// The wide-tooth delta stream: setup `2^k`, then `2n` deltas of `±2^w`
/// oscillating across the `2^k` cliff, sign read after each.
fn wide_tooth_run(k: u32, w: u32, n: usize) -> Run {
    let tooth = BigUint::from(1u8) << w as usize;
    let mut acc = Accumulator::new();
    fold_big(&mut acc, &(BigUint::from(1u8) << k as usize), false);
    touch_meter::reset();
    for _ in 0..n {
        fold_big(&mut acc, &tooth, true);
        assert_eq!(acc.sign(), Ordering::Greater, "below the cliff");
        fold_big(&mut acc, &tooth, false);
        assert_eq!(acc.sign(), Ordering::Greater, "back at the cliff");
    }
    Run {
        denominator: 2 * n as u64,
        ops: 4 * n as u64,
        touches: touch_meter::touches(),
    }
}

/// The cancelling-prefix chain stream: setup `2^k`, then `2n` deltas of
/// `∓(2^k − 1)` dropping to 1 and back, sign read after each.
///
/// The deltas themselves are `k` bits wide, so the linearity
/// denominator is the stream's own coded size — `2n` zigzag-gamma codes
/// of `2k + 3` bits each — in bytes, not the delta count.
fn cancelling_run(k: u32, n: usize) -> Run {
    let drop = (BigUint::from(1u8) << k as usize) - 1u8;
    let mut acc = Accumulator::new();
    fold_big(&mut acc, &(BigUint::from(1u8) << k as usize), false);
    touch_meter::reset();
    for _ in 0..n {
        fold_big(&mut acc, &drop, true);
        assert_eq!(acc.sign(), Ordering::Greater, "down at 1");
        fold_big(&mut acc, &drop, false);
        assert_eq!(acc.sign(), Ordering::Greater, "back at the peak");
    }
    Run {
        denominator: (2 * n as u64) * (2 * u64::from(k) + 3) / 8,
        ops: 4 * n as u64,
        touches: touch_meter::touches(),
    }
}

/// The static-prefix read stream: a cancelling prefix built once,
/// then `n` cycles of `add_small(1)` / sign / `sub_small(1)` / sign.
///
/// The prefix is `+2^k` then `−(2^k − 1)`, leaving value 1 spelled
/// across `k/32` wide digits. Setup is excluded from the count; the
/// linearity denominator is the `2n` sign reads.
///
/// Unlike [`cancelling_run`], no wide write precedes the reads: the
/// first sign fold must scan the whole prefix, and only its collapse
/// keeps every later read from re-scanning it. A no-collapse
/// implementation reads the full `k/32`-digit prefix on every sign
/// here, so its per-read cost grows linearly with `k` instead of
/// staying flat.
fn static_prefix_run(k: u32, n: usize) -> Run {
    let drop = (BigUint::from(1u8) << k as usize) - 1u8;
    let mut acc = Accumulator::new();
    fold_big(&mut acc, &(BigUint::from(1u8) << k as usize), false);
    fold_big(&mut acc, &drop, true);
    touch_meter::reset();
    for _ in 0..n {
        acc.add_small(1);
        assert_eq!(acc.sign(), Ordering::Greater, "up at 2");
        acc.sub_small(1);
        assert_eq!(acc.sign(), Ordering::Greater, "back at 1");
    }
    Run {
        denominator: 2 * n as u64,
        ops: 4 * n as u64,
        touches: touch_meter::touches(),
    }
}

/// The boundary-comb stream's per-delta digit-touch cost stays under
/// its pinned ceiling at both scales and flat across the `k`, `n`
/// doubling (the shape where a normalized representation is quadratic).
#[test]
fn accum_comb_touches_flat() {
    let small = comb_run(4_096, 50_000);
    let large = comb_run(8_192, 100_000);
    assert_flat("comb", &small, &large, envelope::COMB_MILLI_PER_DELTA);
}

/// The unpaid-crossing fan's entry/exit stream — the root magnitude
/// paid once, then `±1` path-sum crossings per tooth — stays under the
/// same pinned per-delta ceiling, flat across the doubling.
///
/// The fan prices Dyck-walk accumulation where the boundary comb prices
/// consecutive-leaf deltas; per delta the two streams are the same
/// arithmetic, and the pinned ceiling says so.
#[test]
fn accum_fan_touches_flat() {
    let small = comb_run(4_096, 50_000);
    let large = comb_run(8_192, 100_000);
    assert_flat("fan", &small, &large, envelope::COMB_MILLI_PER_DELTA);
}

/// The wide-tooth stream's per-delta digit-touch cost (tooth width
/// fixed, cliff height and tooth count doubling) stays under its pinned
/// ceiling at both scales and flat across the doubling.
///
/// This is the stream on which any normalized-prefix-plus-window form
/// is quadratic.
#[test]
fn accum_wide_tooth_touches_flat() {
    let small = wide_tooth_run(4_096, 192, 25_000);
    let large = wide_tooth_run(8_192, 192, 50_000);
    assert_flat(
        "wide_tooth",
        &small,
        &large,
        envelope::WIDE_TOOTH_MILLI_PER_DELTA,
    );
}

/// The cancelling-prefix chain's digit-touch cost per coded byte of its
/// own wide deltas stays under its pinned ceiling at both scales and
/// flat across the doubling.
///
/// Every deep sign scan here is funded by the wide delta immediately
/// preceding it, so the stream's cost tracks its own coded size (the
/// collapse itself is priced by `accum_static_prefix_touches_flat`,
/// where no adjacent write funds the scans).
#[test]
fn accum_cancelling_touches_flat() {
    let small = cancelling_run(2_048, 4_096);
    let large = cancelling_run(4_096, 8_192);
    assert_flat(
        "cancelling",
        &small,
        &large,
        envelope::CANCELLING_MILLI_PER_CODED_BYTE,
    );
}

/// The static-prefix read stream's digit-touch cost per sign read
/// stays under its pinned ceiling at both scales and flat across the
/// `k`, `n` doubling.
///
/// The sign fold's collapse pays for the deep scan exactly once, so a
/// cancelling prefix built once and then read many times costs O(1)
/// digit touches per read.
///
/// This is the pin that makes the collapse load-bearing: the other
/// three streams fund every deep scan with an immediately preceding
/// wide write, so they stay flat even with the collapse deleted; only
/// this stream's ceiling breaks (by a factor growing linearly in `k`)
/// when a sign fold leaves the scanned prefix in place.
#[test]
fn accum_static_prefix_touches_flat() {
    let small = static_prefix_run(2_048, 10_000);
    let large = static_prefix_run(4_096, 20_000);
    assert_flat(
        "static_prefix",
        &small,
        &large,
        envelope::STATIC_PREFIX_MILLI_PER_READ,
    );
}

// The pinned per-unit ceilings: measured ×1.25, rounded up
// (aarch64-apple-darwin, dev profile, three identical runs), in
// milli-touches per unit (per delta, per coded byte for the chain, per
// sign read for the static prefix). The measurements of record live in
// the pin commits (`git log -S` the constant); re-pin from the
// MEASURED lines under `--no-capture` with `--all-features`. The comb
// ceiling also serves the fan row: per delta the two streams are the
// same arithmetic.
#[rustfmt::skip]
mod envelope {
    pub const COMB_MILLI_PER_DELTA: u64            = 2_500;
    pub const WIDE_TOOTH_MILLI_PER_DELTA: u64      = 7_501;
    pub const CANCELLING_MILLI_PER_CODED_BYTE: u64 =   314;
    pub const STATIC_PREFIX_MILLI_PER_READ: u64    = 2_509;
}
