//! Cliff-freedom and rank-freeze checks for version folds.

use super::*;

/// Validate the `k = n = scale` boundary comb's version stream and
/// record both counters over the validation body alone.
///
/// Enforces the touch-meter liveness floor before returning: every
/// delta code writes at least one accumulator digit, so a validator
/// whose height state runs on anything but the metered accumulator
/// (under which the flatness ratio holds vacuously at zero touches)
/// fails loudly here instead. The metered accumulator measures about
/// 1.6 touches per delta on this comb, so the one-touch floor is
/// comfortable.
fn comb_run(scale: usize) -> Run {
    let encoded = Shape::CliffComb.build2(scale, scale);
    let v = encoded.version();
    let enc = v;
    touch_meter::reset();
    meter::version::validate(&enc).expect("the comb stream is canonical");
    let run = Run {
        // 2n + 1 leaves: 2n delta codes follow the first leaf.
        deltas: 2 * scale as u64,
        bytes: enc.as_bytes().len() as u64,
        touches: touch_meter::touches(),
    };
    assert!(
        run.touches >= run.deltas,
        "skyline_comb scale {scale}: {} digit touches under the {}-delta floor: \
         the validator's height state is not running on the metered accumulator",
        run.touches,
        run.deltas,
    );
    run
}

/// The validator's per-delta accumulator touches stay flat across a
/// `k = n` doubling of the boundary comb: the
/// nonnegativity check is cliff-free, achieved rather than promised.
///
/// Each run also carries the one-touch-per-delta liveness floor (in
/// [`comb_run`]), so flatness is asserted over a meter proven live.
#[test]
fn skyline_validate_cliff_cost_is_flat_per_unit() {
    let small = comb_run(512);
    let large = comb_run(1_024);
    assert_flat(
        "touches",
        "delta",
        (small.touches, small.deltas),
        (large.touches, large.deltas),
    );
}

/// Compare the `k = n = scale` boundary comb's version stream against
/// the empty version's and record both counters over the sweep body
/// alone.
///
/// Enforces the same touch-meter liveness floor as [`comb_run`]:
/// every comb delta lands in the running difference, so a sweep whose
/// difference state is not the metered accumulator fails loudly here
/// instead of passing the flatness ratio vacuously at zero touches.
fn comb_cmp_run(scale: usize) -> Run {
    let encoded = Shape::CliffComb.build2(scale, scale);
    let v = encoded.version();
    let a = v;
    let b = before::Version::new();
    touch_meter::reset();
    let verdict = meter::version::causal_cmp(&a, &b);
    assert_eq!(
        verdict,
        Some(std::cmp::Ordering::Greater),
        "the comb strictly dominates the empty version"
    );
    let run = Run {
        // 2n + 1 leaves: 2n delta codes follow the first leaf.
        deltas: 2 * scale as u64,
        bytes: (a.as_bytes().len() + b.as_bytes().len()) as u64,
        touches: touch_meter::touches(),
    };
    assert!(
        run.touches >= run.deltas,
        "skyline_comb_cmp scale {scale}: {} digit touches under the {}-delta floor: \
         the sweep's difference state is not running on the metered accumulator",
        run.touches,
        run.deltas,
    );
    run
}

/// The sweep's per-delta accumulator touches stay flat across a `k = n`
/// doubling of the boundary comb compared against the empty version.
///
/// The running difference crosses the `2^k` carry boundary at every
/// delta and each crossing stays amortized O(1) — the comparison-side
/// cliff-freedom witness.
///
/// Each run also carries the one-touch-per-delta liveness floor (in
/// [`comb_cmp_run`]), so flatness is asserted over a meter proven
/// live.
#[test]
fn skyline_cmp_cliff_cost_is_flat_per_unit() {
    let small = comb_cmp_run(512);
    let large = comb_cmp_run(1_024);
    assert_flat(
        "cmp_touches",
        "delta",
        (small.touches, small.deltas),
        (large.touches, large.deltas),
    );
}

/// Join the `k = n = scale` boundary comb's version stream with a
/// one-tick stream and record both counters over the emission body
/// alone.
///
/// Enforces the same touch-meter liveness floor as [`comb_cmp_run`]:
/// the emitter's running difference lands every comb delta, so an
/// emission whose difference state is not the metered accumulator
/// fails loudly here instead of passing the flatness ratio vacuously
/// at zero touches.
fn comb_join_run(scale: usize) -> Run {
    let encoded = Shape::CliffComb.build2(scale, scale);
    let v = encoded.version();
    let a = v;
    let mut one = before::Version::new();
    one.tick(&before::Party::seed());
    let b = one;
    let expected = &a | &b;
    touch_meter::reset();
    let out = meter::version::join(&a, &b);
    let run = Run {
        // 2n + 1 leaves: 2n delta codes follow the first leaf.
        deltas: 2 * scale as u64,
        bytes: (a.as_bytes().len() + b.as_bytes().len()) as u64,
        touches: touch_meter::touches(),
    };
    assert_eq!(
        out, expected,
        "the emitted join must match the encoded join"
    );
    assert!(
        run.touches >= run.deltas,
        "skyline_comb_join scale {scale}: {} digit touches under the {}-delta floor: \
         the emitter's difference state is not running on the metered accumulator",
        run.touches,
        run.deltas,
    );
    run
}

/// The join emitter's per-delta accumulator touches stay flat across a
/// `k = n` doubling of the boundary comb joined with a one-tick stream.
///
/// The running difference crosses the `2^k` carry boundary at every
/// delta and each crossing stays amortized O(1) — the emission-side
/// cliff-freedom witness, the merge counterpart of the comparison
/// pin above (join, meet, `recv`, `sync`, and the fold operators all
/// ride this emitter). Each run also carries the one-touch-per-delta
/// liveness floor (in [`comb_join_run`]), so flatness is asserted
/// over a meter proven live.
#[test]
fn skyline_join_cliff_cost_is_flat_per_unit() {
    let small = comb_join_run(512);
    let large = comb_join_run(1_024);
    assert_flat(
        "join_touches",
        "delta",
        (small.touches, small.deltas),
        (large.touches, large.deltas),
    );
}

/// Tooth width (bits) one notch under the rank freeze threshold's
/// 256-bit digit bound: the band's flat side.
const FREEZE_BAND_UNDER_BITS: usize = 192;

/// Tooth width (bits) one notch over the rank freeze threshold's
/// 256-bit digit bound: every fold evicts the live component.
const FREEZE_BAND_OVER_BITS: usize = 300;

/// Cliff magnitude (bits) of the small freeze-band run; the frozen
/// component's width, which the over-threshold regime re-reads per
/// tooth.
const FREEZE_BAND_SMALL_K: usize = 9_600;

/// Tooth count of the small freeze-band run.
const FREEZE_BAND_SMALL_N: usize = 128;

/// One rank run over the wide-tooth comb `W(k, w, n)`'s Version
/// stream: the per-unit denominators (deltas, Version bytes) and
/// both counters over the rank body alone.
///
/// Enforces the touch-meter liveness floor before returning — every
/// delta code writes at least one accumulator digit, so a rank whose
/// height state runs on anything but the metered accumulator fails
/// loudly instead of passing a per-unit bound vacuously at zero
/// touches — and pins the result against the encoded rank, so the
/// measured body is proven to compute the right answer.
fn rank_wide_tooth_run(k: usize, w: usize, n: usize) -> Run {
    let encoded = Shape::WideToothComb.build3(k, w, n);
    let v = encoded.version();
    let enc = v.clone();
    touch_meter::reset();
    let r = meter::version::rank(&enc);
    let run = Run {
        // Each tooth's two leaves follow the first leaf as deltas.
        deltas: 2 * n as u64,
        bytes: enc.as_bytes().len() as u64,
        touches: touch_meter::touches(),
    };
    assert_eq!(r, v.rank(), "the kernel must match the encoded rank");
    assert!(
        run.touches >= run.deltas,
        "skyline_rank_wide_tooth w={w}: {} digit touches under the {}-delta floor: \
         the rank height state is not running on the metered accumulator",
        run.touches,
        run.deltas,
    );
    run
}

/// Absolute over-threshold ceilings: the measured record ×1.25
/// (deterministic counters; the record and every re-pin's movement
/// live in the pin commits).
///
/// The ceilings price the freeze discipline's flat over-threshold
/// work — each fold's eviction paid at the drift's own funded width
/// — where a frozen-width-per-tooth accounting reads quadratic and
/// exceeds them.
const FREEZE_BAND_OVER_TOUCH_CEILINGS: (u64, u64) = (6_458, 12_938);

/// The rank kernel's freeze band on the wide-tooth comb, both sides
/// flat: bounded oscillation never freezes at any tooth width.
///
/// A fold's cost rides the live component, paid by the tooth's own
/// code, so per-byte cost stays flat (×1.25) across a doubling of
/// `k` and `n` one notch under the freeze allowance's 256-bit digit
/// bound (192-bit teeth) and one notch over it (300-bit teeth)
/// alike, with the over side's absolute ceilings pinned as the
/// tightened record that retired the frozen-width-per-tooth
/// quadratic baseline.
#[test]
fn skyline_rank_wide_tooth_freeze_band() {
    let under_small = rank_wide_tooth_run(
        FREEZE_BAND_SMALL_K,
        FREEZE_BAND_UNDER_BITS,
        FREEZE_BAND_SMALL_N,
    );
    let under_large = rank_wide_tooth_run(
        2 * FREEZE_BAND_SMALL_K,
        FREEZE_BAND_UNDER_BITS,
        2 * FREEZE_BAND_SMALL_N,
    );
    assert_flat(
        "rank_under_threshold_touches",
        "byte",
        (under_small.touches, under_small.bytes),
        (under_large.touches, under_large.bytes),
    );
    let over_small = rank_wide_tooth_run(
        FREEZE_BAND_SMALL_K,
        FREEZE_BAND_OVER_BITS,
        FREEZE_BAND_SMALL_N,
    );
    let over_large = rank_wide_tooth_run(
        2 * FREEZE_BAND_SMALL_K,
        FREEZE_BAND_OVER_BITS,
        2 * FREEZE_BAND_SMALL_N,
    );
    for (run, touch_ceiling, scale) in [
        (&over_small, FREEZE_BAND_OVER_TOUCH_CEILINGS.0, "small"),
        (&over_large, FREEZE_BAND_OVER_TOUCH_CEILINGS.1, "large"),
    ] {
        eprintln!(
            "MEASURED skyline_rank_over_threshold_{scale}: bytes={} touches={}",
            run.bytes, run.touches,
        );
        assert!(
            run.touches <= touch_ceiling,
            "skyline_rank_over_threshold_{scale}: {} touches exceed the pinned \
             ceiling {touch_ceiling}",
            run.touches,
        );
    }
    assert_flat(
        "rank_over_threshold_touches",
        "byte",
        (over_small.touches, over_small.bytes),
        (over_large.touches, over_large.bytes),
    );
}

/// One rank run over the jump comb `J(k, n)`'s version stream: the
/// per-unit denominators and both counters over the rank body alone.
///
/// Carries the same liveness floor and encoded-rank agreement as
/// [`rank_wide_tooth_run`].
fn rank_jump_run(k: usize, n: usize) -> Run {
    let encoded = Shape::JumpComb.build2(k, n);
    let v = encoded.version();
    let enc = v.clone();
    touch_meter::reset();
    let r = meter::version::rank(&enc);
    let run = Run {
        // Each tooth's two leaves follow the first leaf as deltas.
        deltas: 2 * n as u64,
        bytes: enc.as_bytes().len() as u64,
        touches: touch_meter::touches(),
    };
    assert_eq!(r, v.rank(), "the kernel must match the encoded rank");
    assert!(
        run.touches >= run.deltas,
        "skyline_rank_jump k={k}: {} digit touches under the {}-delta floor: \
         the rank height state is not running on the metered accumulator",
        run.touches,
        run.deltas,
    );
    run
}

/// Absolute jump-comb ceilings: the measured record ×1.25 (three
/// identical runs; the record and every re-pin's movement live in
/// the pin commits).
///
/// The ceilings price one eviction of the `k`-bit jump plus flat
/// 3-bit-delta work — the un-evicted alternative reads the jump's
/// width again on every following delta, `Θ(n·k)` against these
/// ceilings' flat funding, an order-of-magnitude overshoot.
const RANK_JUMP_TOUCH_CEILINGS: (u64, u64) = (6_423, 12_840);

/// The rank kernel's freeze eviction on the jump comb is funded and
/// flat.
///
/// The mid-stream `k`-bit jump lands in the live component, the
/// first cheap delta behind it fires the one freeze — priced by the
/// drift the jump's own code paid for, never by the frozen width —
/// and every later 3-bit delta rides an emptied live component, so
/// per-byte cost stays flat (×1.25) across a doubling of `k` and `n`
/// under absolute ceilings a stale-drift regression (the jump
/// re-read per delta) exceeds ~15-fold.
#[test]
fn skyline_rank_jump_eviction_is_flat_per_unit() {
    let small = rank_jump_run(FREEZE_BAND_SMALL_K, FREEZE_BAND_SMALL_N);
    let large = rank_jump_run(2 * FREEZE_BAND_SMALL_K, 2 * FREEZE_BAND_SMALL_N);
    for (run, touch_ceiling, scale) in [
        (&small, RANK_JUMP_TOUCH_CEILINGS.0, "small"),
        (&large, RANK_JUMP_TOUCH_CEILINGS.1, "large"),
    ] {
        eprintln!(
            "MEASURED skyline_rank_jump_{scale}: bytes={} touches={}",
            run.bytes, run.touches,
        );
        assert!(
            run.touches <= touch_ceiling,
            "skyline_rank_jump_{scale}: {} touches exceed the pinned ceiling \
             {touch_ceiling}: the jump's drift is not being evicted once",
            run.touches,
        );
    }
    assert_flat(
        "rank_jump_touches",
        "byte",
        (small.touches, small.bytes),
        (large.touches, large.bytes),
    );
}
