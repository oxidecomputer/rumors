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
//! The judge's environment knobs and the sidecar's stamped format are
//! shared with the tripwire target and documented once, in
//! `common::sidecar`: `BOARD_BENCH_SCALE` picks the family scale the cells
//! are built at (unset means the board's seconds-scale default of 1, so
//! plain bench numbers and board verdicts keep describing the same
//! operands), and `BOARD_BENCH_DENOMS` names the sidecar path — here the
//! denominators are `BenchCell::denominator_bytes`, the board's own
//! denomination, which is what the judge divides by when it fits
//! exponents.

use std::time::Duration;

use before::meter::board;
use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};

mod common;
use common::sidecar;

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

/// Time every board cell, grouped by operation row.
fn bench_board(c: &mut Criterion) {
    let scale = sidecar::scale_from_env();
    let cells = board::bench_cells(scale);
    // Cell IDs are `op/family` and denominators are the board's own
    // per-cell bytes, in board row order.
    let denoms: Vec<(String, usize)> = cells
        .iter()
        .map(|cell| {
            (
                format!("{}/{}", cell.op, cell.family),
                cell.denominator_bytes(),
            )
        })
        .collect();
    sidecar::write_denoms(scale, denoms.iter().map(|(id, n)| (id.as_str(), *n)));
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
