//! The bench judge's tripwire: an unmetered quadratic the judge must read
//! red.
//!
//! One bench, `tripwire_unmetered_quadratic/quadratic`: a quadratic pass of
//! plain machine-word arithmetic — no allocation, no recursion, no
//! big-integer ops, no stream reads — so every counter column the
//! amplification board judges records zero and the board's ceilings and
//! floors all pass. Only a clock sees it. Running this target at two scales
//! and pointing `tools/benchjudge` at the saved baselines (`just
//! bench-judge-tripwire`) must produce a RED verdict on the fitted time
//! exponent (~2.0 against the 1.3 ceiling): the live demonstration that the
//! judge's leg catches what no deterministic meter can. The same shape is
//! pinned deterministically in the judge's `--self-test`, so the criterion
//! cannot soften silently between live demonstrations.
//!
//! The target honors the same knobs as `benches/board.rs`, shared and
//! documented in `common::sidecar`: `BOARD_BENCH_SCALE` sizes the probe
//! (`record` maps to the board's ×4) and `BOARD_BENCH_DENOMS` writes the
//! stamped denominator sidecar the judge divides by (here the probe's
//! size parameter: its work is `n²` in it).

use criterion::{black_box, criterion_group, criterion_main, Criterion};

mod common;
use common::sidecar;

/// The probe's size parameter at scale 1; the scale knob multiplies it.
///
/// Sized so the scale-1 body is well above the judge's sub-floor cutoff
/// (hundreds of microseconds against the judge's 10 µs floor) while the ×4
/// demonstration stays at a few milliseconds per iteration.
const TRIPWIRE_BASE: usize = 512;

/// The criterion group ID: the judge's cell IDs are `group/function`.
const GROUP: &str = "tripwire_unmetered_quadratic";

/// The criterion function ID within the group.
const FUNCTION: &str = "quadratic";

/// One quadratic pass of plain machine-word arithmetic keyed to `n`: no
/// allocation, no recursion, no metered reads.
fn unmetered_quadratic(n: usize) -> u64 {
    let mut acc = 0u64;
    for i in 0..n {
        for j in 0..n {
            acc = acc
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add((i ^ j) as u64);
        }
    }
    acc
}

/// Time the quadratic probe at the scaled size.
fn bench_tripwire(c: &mut Criterion) {
    let scale = sidecar::scale_from_env();
    let n = ((TRIPWIRE_BASE as f64) * scale).round() as usize;
    let id = format!("{GROUP}/{FUNCTION}");
    sidecar::write_denoms(scale, [(id.as_str(), n)]);
    let mut group = c.benchmark_group(GROUP);
    group.bench_function(FUNCTION, |b| b.iter(|| unmetered_quadratic(black_box(n))));
    group.finish();
}

criterion_group!(benches, bench_tripwire);
criterion_main!(benches);
