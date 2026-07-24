//! The full-surface benches: every amplification-board cell as a wall-clock
//! number.
//!
//! One criterion group per board operation row, one bench per input family,
//! driven by the board's own cell table (`meter::board::bench_cells`) — so
//! the IDs mirror the board exactly: the board cell `version_rank × harmonic`
//! is the bench `version_rank/harmonic`, and a red board cell names the bench
//! that times it. The deterministic record stays with the board and the
//! envelope suite (`tests/meter.rs`); these rows exist so a wall-time
//! regression (or win) on any operation × shape is a first-class,
//! criterion-tracked number — and so the bench judge (`tools/benchjudge`,
//! `just bench-judge`) can fit each cell's time exponent across two scales
//! of these benches' saved criterion baselines, the time leg of the board's
//! judgment.
//!
//! # Running
//!
//! Targeted runs go through the justfile recipes (criterion filter
//! passthrough — group and function IDs concatenate, so `version_rank`
//! selects a row and `version_rank/harmonic` one cell):
//!
//! ```text
//! just bench board version_rank            # one operation, full sampling
//! just bench-quick board version_rank      # the reduced-sampling inner loop
//! ```
//!
//! Full sampling (plain `just bench board <filter>`) is criterion's default
//! 100-sample regime over this file's committed measurement windows, and is
//! required for any number quoted as a result of record. Quick mode
//! (`just bench-quick`, criterion `--sample-size 10 --measurement-time 1`)
//! is for agent iteration only.
//!
//! Setup runs per iteration but outside the timed region: each measured body
//! comes from the board's prepare (operands decoded fresh, exactly what the
//! board meters), so destructive operations are rebuilt untimed.
//!
//! # The judge's knobs
//!
//! Two environment variables exist for the bench-judge recipes and default
//! to off:
//!
//! - `BOARD_BENCH_SCALE`: the family scale the cells are built at (a
//!   positive number, or the literal `record` for the board's acceptance
//!   scale `board::RECORD_SCALE`). Unset means the board's seconds-scale
//!   default of 1, so plain bench numbers and board verdicts keep
//!   describing the same operands.
//! - `BOARD_BENCH_DENOMS`: a path; when set, the harness writes a JSON
//!   sidecar mapping each cell ID to its denominator bytes at this scale
//!   (`BenchCell::denominator_bytes`, the board's own denomination), which
//!   is what the judge divides by when it fits exponents.

use std::time::Duration;

use before::meter::board;
use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};

/// The scale environment variable read by [`scale_from_env`].
const SCALE_ENV: &str = "BOARD_BENCH_SCALE";

/// The denominator-sidecar environment variable read by [`write_denoms`].
const DENOMS_ENV: &str = "BOARD_BENCH_DENOMS";

/// Warm-up window per bench.
///
/// Shorter than criterion's 3 s default: the bodies are decode-fresh every
/// iteration, so there is no cache to warm beyond the allocator, and the
/// full surface is ~200 cells — the committed windows are what keeps a
/// full-sampling sweep in minutes.
const WARM_UP: Duration = Duration::from_millis(500);

/// Measurement window per bench at full sampling.
///
/// Criterion still draws its default 100 samples inside it; cells whose
/// bodies outgrow the window keep their 100 samples and simply run longer
/// (criterion notes the overshoot but never truncates the sample count).
const MEASUREMENT: Duration = Duration::from_secs(2);

/// The family scale from `BOARD_BENCH_SCALE`: unset means the board's
/// seconds-scale default of 1, `record` means `board::RECORD_SCALE`.
fn scale_from_env() -> f64 {
    match std::env::var(SCALE_ENV) {
        Err(std::env::VarError::NotPresent) => 1.0,
        Ok(raw) if raw == "record" => board::RECORD_SCALE,
        Ok(raw) => raw.parse().unwrap_or_else(|_| {
            panic!("{SCALE_ENV} must be a positive number or `record`, got {raw:?}")
        }),
        Err(err) => panic!("{SCALE_ENV} is not valid UTF-8: {err}"),
    }
}

/// Write the denominator sidecar to the `BOARD_BENCH_DENOMS` path, if set:
/// one JSON object, cell ID (`op/family`) to denominator bytes, in board
/// row order.
fn write_denoms(cells: &[board::BenchCell]) {
    let path = match std::env::var(DENOMS_ENV) {
        Err(std::env::VarError::NotPresent) => return,
        Ok(path) => path,
        Err(err) => panic!("{DENOMS_ENV} is not valid UTF-8: {err}"),
    };
    let mut json = String::from("{\n");
    for (i, cell) in cells.iter().enumerate() {
        let comma = if i + 1 < cells.len() { "," } else { "" };
        json.push_str(&format!(
            "  \"{}/{}\": {}{comma}\n",
            cell.op,
            cell.family,
            cell.denominator_bytes()
        ));
    }
    json.push_str("}\n");
    std::fs::write(&path, json)
        .unwrap_or_else(|err| panic!("writing the denominator sidecar {path:?} failed: {err}"));
}

/// Time every board cell, grouped by operation row.
fn bench_board(c: &mut Criterion) {
    let cells = board::bench_cells(scale_from_env());
    write_denoms(&cells);
    let mut next = 0;
    while next < cells.len() {
        let op = cells[next].op;
        let mut group = c.benchmark_group(op);
        while next < cells.len() && cells[next].op == op {
            let cell = &cells[next];
            group.bench_function(cell.family, |b| {
                b.iter_batched(
                    || cell.body(),
                    |body| black_box(body()),
                    BatchSize::LargeInput,
                );
            });
            next += 1;
        }
        group.finish();
    }
}

criterion_group!(
    name = benches;
    // The windows land on the `Criterion` object before `configure_from_args`
    // so the quick-mode flags (`--sample-size`, `--measurement-time`) keep
    // their CLI precedence; a group-level setting would silently win over
    // them.
    config = Criterion::default()
        .warm_up_time(WARM_UP)
        .measurement_time(MEASUREMENT);
    targets = bench_board
);
criterion_main!(benches);
