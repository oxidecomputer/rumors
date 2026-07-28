//! The full-surface benches: every amplification-board cell as a wall-clock
//! number.
//!
//! One criterion group per board operation row, one bench per input family,
//! driven by the board's own cell table (`meter::board::bench_cells`) — so
//! the IDs mirror the board exactly: the board cell `version_rank × harmonic`
//! is the bench `version_rank/harmonic`, and a red board cell names the bench
//! that times it. The cell set is a `board::BenchMode` slice of the board's
//! shape × operation product, chosen by `BOARD_BENCH_MODE`
//! (`common::sidecar::mode_from_env`): the default pinned subset for the
//! judge recipes' cadence, the full product for final verdicts — both
//! derived from the board's own axis declarations, never a second list. The deterministic record stays with the board and the
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
//!
//! # The wide-display pair
//!
//! Two judge-only cells follow the board rows in the same sidecar:
//! `version_display_wide/hugeleaf` and `display_schoolbook/hugeleaf`. They
//! are not board cells — the board's display rows already meter rendering
//! at board sizes, but the binary→decimal conversion runs inside the bignum
//! dependency, below the limb shim, where no deterministic counter can see
//! its complexity class. The judged time exponent is the class witness, and
//! at board widths conversion does not yet dominate the render; this pair
//! is the same `Display` path at conversion-dominated widths
//! ([`WIDE_DISPLAY_BASE_MAGNITUDE_BITS`], the scale knob supplying the
//! second width point), where divide-and-conquer conversion (fitted
//! e ≈ 1.5 against `n_io`) and a schoolbook conversion (e ≈ 2.0) separate
//! decisively. Both cells are denominated like the board's text rows:
//! `n_io` = packed input bytes + rendered text bytes, output read back
//! from an actual render.
//!
//! Both cells declare the text ceiling class in the sidecar
//! (`common::sidecar::Ceiling::Text`, membership pinned by
//! `sidecar::TEXT_CEILING_CELLS` and `tests/bench_judge_roster.rs`), so the
//! judge fits them against its text-conversion ceiling — a property of the
//! cells, declared here where they are defined, never by the roster. The
//! honest cell times the crate's own `Display`; the schoolbook cell times
//! [`schoolbook_decimal`], a per-chunk repeated-division renderer kept in
//! bench code only, spelling the identical value (asserted at setup) — the
//! known-quadratic class the text ceiling must read RED, enforced every run
//! as a roster red expectation (`tools/benchjudge-expected.json`).

use std::time::Duration;

use before::meter::board;
use before::{meter, Version};
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

/// The wide-display pair's magnitude width at scale 1, in bits.
///
/// Wide enough that binary→decimal conversion dominates the render, so the
/// fitted exponent reads the conversion's class rather than the walk's
/// linear overhead; the scale knob (×4 at `record`) supplies the second
/// width point, so the judge fits across a 128k→512k-bit span.
const WIDE_DISPLAY_BASE_MAGNITUDE_BITS: usize = 128_000;

/// The honest wide-display cell's criterion group ID.
const WIDE_DISPLAY_GROUP: &str = "version_display_wide";

/// The schoolbook tripwire's criterion group ID.
const SCHOOLBOOK_GROUP: &str = "display_schoolbook";

/// Both wide-display cells' criterion function ID: the operand family.
const WIDE_DISPLAY_FAMILY: &str = "hugeleaf";

/// Time every board cell, grouped by operation row, then the wide-display
/// pair.
fn bench_board(c: &mut Criterion) {
    let scale = sidecar::scale_from_env();
    let cells = board::bench_cells(scale, sidecar::mode_from_env());
    let wide = WideDisplay::build(scale);
    // Cell IDs are `op/family` and denominators are the board's own
    // per-cell bytes, in board row order; the wide-display pair rides the
    // same sidecar after the board rows and declares the text ceiling —
    // every board cell is judged at the general one.
    let mut denoms: Vec<(String, usize, sidecar::Ceiling)> = cells
        .iter()
        .map(|cell| {
            (
                format!("{}/{}", cell.op, cell.family),
                cell.denominator_bytes(),
                sidecar::Ceiling::General,
            )
        })
        .collect();
    denoms.push((
        format!("{WIDE_DISPLAY_GROUP}/{WIDE_DISPLAY_FAMILY}"),
        wide.n_io,
        sidecar::Ceiling::Text,
    ));
    denoms.push((
        format!("{SCHOOLBOOK_GROUP}/{WIDE_DISPLAY_FAMILY}"),
        wide.n_io,
        sidecar::Ceiling::Text,
    ));
    sidecar::write_denoms(scale, denoms.iter().map(|(id, n, c)| (id.as_str(), *n, *c)));
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
    wide.bench(c);
}

/// The wide-display pair's prepared operands at one scale.
struct WideDisplay {
    /// The hugeleaf version whose render the honest cell times.
    version: Version,
    /// The same magnitude as a little-endian limb image, the schoolbook
    /// renderer's working form.
    limbs: Vec<u64>,
    /// The shared denominator: packed input bytes + rendered text bytes,
    /// the board text rows' `n_io` convention with the output side read
    /// back from an actual render.
    n_io: usize,
}

impl WideDisplay {
    /// Build both cells' operands at `scale` and settle their denominator.
    ///
    /// The two cells spell the identical value across the identical widths,
    /// asserted here, so the judge's verdicts differ only by the conversion
    /// under test; the render's length is held to the magnitude's honest
    /// decimal band, so the text side of `n_io` cannot be padded.
    fn build(scale: f64) -> WideDisplay {
        let bits = ((WIDE_DISPLAY_BASE_MAGNITUDE_BITS as f64) * scale).round() as usize;
        // The generator emits its construction language; `Packed::version`
        // transcodes it into the stored coding whose packed bytes are the
        // denominator's input side.
        let version = meter::hugeleaf(bits).version();
        let text = version.to_string();
        // A b-bit magnitude spells ~0.301·b decimal digits (log10 2); the
        // band [b/4, b/2] brackets that with room for the leaf syntax.
        assert!(
            text.len() > bits / 4 && text.len() < bits / 2,
            "a {bits}-bit magnitude renders ~0.301 digits per bit; got {} bytes",
            text.len()
        );
        let limbs = all_ones_limbs(bits);
        assert_eq!(
            schoolbook_decimal(limbs.clone()),
            text,
            "the schoolbook renderer spells the hugeleaf value exactly"
        );
        let n_io = version.encode().len() + text.len();
        WideDisplay {
            version,
            limbs,
            n_io,
        }
    }

    /// Register both cells: the crate's `Display`, then the schoolbook.
    fn bench(self, c: &mut Criterion) {
        let mut group = c.benchmark_group(WIDE_DISPLAY_GROUP);
        let version = self.version;
        group.bench_function(WIDE_DISPLAY_FAMILY, |b| {
            b.iter(|| black_box(version.to_string()))
        });
        group.finish();
        let mut group = c.benchmark_group(SCHOOLBOOK_GROUP);
        let limbs = self.limbs;
        group.bench_function(WIDE_DISPLAY_FAMILY, |b| {
            // The division loop consumes its limb image, so each iteration
            // gets a fresh copy in untimed setup.
            b.iter_batched(
                || limbs.clone(),
                |image| black_box(schoolbook_decimal(image)),
                BatchSize::LargeInput,
            );
        });
        group.finish();
    }
}

/// The magnitude `2^bits − 1` as little-endian 64-bit limbs.
fn all_ones_limbs(bits: usize) -> Vec<u64> {
    let mut limbs = vec![u64::MAX; bits / 64];
    let rem = bits % 64;
    if rem > 0 {
        limbs.push((1u64 << rem) - 1);
    }
    limbs
}

/// The largest power of ten below 2^64: the schoolbook renderer's per-pass
/// divisor, so one full-width division pass yields 19 digits.
const SCHOOLBOOK_CHUNK: u64 = 10_000_000_000_000_000_000;

/// Decimal digits per schoolbook division pass: `log10(SCHOOLBOOK_CHUNK)`.
const SCHOOLBOOK_CHUNK_DIGITS: usize = 19;

/// Render a little-endian limb image to decimal by schoolbook repeated
/// division: one full-width pass per 19 digits, quadratic in the width.
///
/// This is the conversion class the judge's text ceiling exists to catch,
/// resurrected in bench code only — the crate's own `Display` delegates to
/// the backend's divide-and-conquer conversion.
fn schoolbook_decimal(mut limbs: Vec<u64>) -> String {
    while limbs.last() == Some(&0) {
        limbs.pop();
    }
    if limbs.is_empty() {
        return "0".to_string();
    }
    // Digits accumulate least-significant first, one chunk per pass; the
    // final (most significant) chunk drops its leading zeros.
    let mut out: Vec<u8> = Vec::new();
    while !limbs.is_empty() {
        let mut rem: u64 = 0;
        for limb in limbs.iter_mut().rev() {
            let cur = ((rem as u128) << 64) | (*limb as u128);
            *limb = (cur / SCHOOLBOOK_CHUNK as u128) as u64;
            rem = (cur % SCHOOLBOOK_CHUNK as u128) as u64;
        }
        while limbs.last() == Some(&0) {
            limbs.pop();
        }
        if limbs.is_empty() {
            while rem > 0 {
                out.push(b'0' + (rem % 10) as u8);
                rem /= 10;
            }
        } else {
            for _ in 0..SCHOOLBOOK_CHUNK_DIGITS {
                out.push(b'0' + (rem % 10) as u8);
                rem /= 10;
            }
        }
    }
    out.reverse();
    String::from_utf8(out).expect("decimal digits are ASCII")
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
