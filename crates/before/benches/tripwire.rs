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
//! The target honors the same knobs as `benches/board.rs`:
//! `BOARD_BENCH_SCALE` sizes the probe (`record` maps to the board's ×4)
//! and `BOARD_BENCH_DENOMS` writes the denominator sidecar the judge
//! divides by (here the probe's size parameter: its work is `n²` in it).

use before::meter::board;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

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

/// The probe's size parameter under `BOARD_BENCH_SCALE` (unset means 1,
/// `record` means `board::RECORD_SCALE`).
fn size_from_env() -> usize {
    let scale = match std::env::var("BOARD_BENCH_SCALE") {
        Err(std::env::VarError::NotPresent) => 1.0,
        Ok(raw) if raw == "record" => board::RECORD_SCALE,
        Ok(raw) => raw.parse().unwrap_or_else(|_| {
            panic!("BOARD_BENCH_SCALE must be a positive number or `record`, got {raw:?}")
        }),
        Err(err) => panic!("BOARD_BENCH_SCALE is not valid UTF-8: {err}"),
    };
    ((TRIPWIRE_BASE as f64) * scale).round() as usize
}

/// Write the one-cell denominator sidecar to `BOARD_BENCH_DENOMS`, if set.
fn write_denoms(n: usize) {
    let path = match std::env::var("BOARD_BENCH_DENOMS") {
        Err(std::env::VarError::NotPresent) => return,
        Ok(path) => path,
        Err(err) => panic!("BOARD_BENCH_DENOMS is not valid UTF-8: {err}"),
    };
    let json = format!("{{\n  \"{GROUP}/{FUNCTION}\": {n}\n}}\n");
    std::fs::write(&path, json)
        .unwrap_or_else(|err| panic!("writing the denominator sidecar {path:?} failed: {err}"));
}

/// Time the quadratic probe at the scaled size.
fn bench_tripwire(c: &mut Criterion) {
    let n = size_from_env();
    write_denoms(n);
    let mut group = c.benchmark_group(GROUP);
    group.bench_function(FUNCTION, |b| b.iter(|| unmetered_quadratic(black_box(n))));
    group.finish();
}

criterion_group!(benches, bench_tripwire);
criterion_main!(benches);
