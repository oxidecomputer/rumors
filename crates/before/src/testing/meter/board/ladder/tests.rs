//! Checks that one ladder sweep reaches each of the board's consumers at the
//! right sampling: the acceptance matrices, the ranking pin, and the
//! capture.
//!
//! The ladders here are synthetic: two families on one operation, with heap
//! readings chosen so that each sweep is recognizable in every consumer's
//! output. No test sweeps the board; the ladders are built directly, so the
//! suite adds no measurement work.
//!
//! Every reading derives from the board's heap ceiling and fit resolution, so
//! the fixture stays green, and its rankings hold, whatever values those
//! constants take.

use super::super::compare::{capture, BuildStamp};
use super::super::currency::{ByCurrency, Currency};
use super::super::floors::na;
use super::super::measure::{Model, Sample};
use super::super::shard::Measured;
use super::super::{
    HEAP_INTERCEPT_BYTES, HEAP_TREND_RESOLUTION_BYTES, MAX_HEAP_BYTES_PER_INPUT_BYTE,
};
use super::{Ladder, Verdict, LADDER_SAMPLINGS};

/// The one synthetic operation.
const OP: &str = "probe_op";

/// The smaller input size of the small-input sweep's pair, in bytes.
const SMALL_DENOM: usize = 10;

/// The smaller input size of the past-power sweep's pair, in bytes.
const PAST_POWER_DENOM: usize = 1_200;

/// The smaller input size of the growth ladder base's pair, in bytes.
const BASE_DENOM: usize = 1_000;

/// The smaller input size of the growth ladder top's pair, in bytes.
const TOP_DENOM: usize = 4_000;

/// Returns the heap ceiling at `denom` input bytes, in bytes.
fn heap_ceiling(denom: usize) -> f64 {
    HEAP_INTERCEPT_BYTES as f64 + MAX_HEAP_BYTES_PER_INPUT_BYTE * denom as f64
}

/// Returns the largest heap reading that is green at `denom` input bytes and
/// flat in the growth fit: within the heap ceiling, and no larger than the
/// fit's resolution, at which the fit clamps every reading.
fn green_cap(denom: usize) -> u64 {
    (heap_ceiling(denom) as u64).min(HEAP_TREND_RESOLUTION_BYTES as u64)
}

/// A sample of `denom` input bytes reading `heap` heap bytes, with no
/// liveness obligation.
fn sample(denom: usize, heap: u64) -> Sample {
    Sample {
        denom_bytes: denom,
        exp_denom_bytes: denom,
        floors: ByCurrency {
            heap: na("no liveness obligation in this probe"),
            fuel: na("no liveness obligation in this probe"),
        },
        models: ByCurrency {
            heap: None,
            fuel: None,
        },
        readings: ByCurrency {
            heap: Some(heap),
            fuel: None,
        },
    }
}

/// Builds one sweep of the synthetic board: the families `alpha` and `beta`,
/// each measured at `denom` bytes and its double, reading the given heap bytes
/// at both sizes.
fn sweep(denom: usize, alpha: u64, beta: u64) -> Vec<Measured> {
    vec![
        (OP, "alpha", sample(denom, alpha), sample(2 * denom, alpha)),
        (OP, "beta", sample(denom, beta), sample(2 * denom, beta)),
    ]
}

/// Returns `alpha`'s heap reading in each of [`ladder`]'s sweeps, in
/// [`LADDER_SAMPLINGS`] order.
fn alpha_readings() -> [u64; 4] {
    [
        green_cap(SMALL_DENOM) / 2,
        green_cap(PAST_POWER_DENOM) / 4,
        green_cap(BASE_DENOM),
        green_cap(TOP_DENOM) / 2,
    ]
}

/// Builds a green ladder whose worst heap family differs between the growth
/// scales.
///
/// At the base, `alpha` reads the green cap ([`green_cap`]) and `beta` half of
/// it; at the top, the roles swap. Both families share each sweep's input
/// sizes, so `alpha` is worst at the base and `beta` at the top. On the small
/// inputs both read half the cap, and on the past-power inputs a quarter of
/// it, ties. No reading exceeds the fit's resolution, so every fitted
/// exponent is flat, and every reading is within the heap ceiling at its
/// size.
fn ladder() -> Ladder {
    let [small, past_power, base, top] = alpha_readings();
    Ladder {
        measured_on: uncalibrated_build(),
        small: sweep(SMALL_DENOM, small, small),
        past_power: sweep(PAST_POWER_DENOM, past_power, past_power),
        base: sweep(BASE_DENOM, base, base / 2),
        top: sweep(TOP_DENOM, top, green_cap(TOP_DENOM)),
    }
}

/// A release build for an architecture no per-cell table covers, so the
/// board-wide ceilings alone judge the synthetic cells.
fn uncalibrated_build() -> BuildStamp {
    BuildStamp {
        debug_assertions: false,
        arch: "uncalibrated".to_owned(),
        os: "test".to_owned(),
    }
}

/// The ranking pin [`ladder`] satisfies.
const PINS: &[(&str, &str, [&str; 1])] =
    &[("default", OP, ["alpha"]), ("acceptance", OP, ["beta"])];

/// Judges `ladder` against `pins`, returning the verdict and the rendered text.
fn judge(ladder: &Ladder, pins: &[(&str, &str, [&str; 1])]) -> (Verdict, String) {
    let mut out = Vec::new();
    let verdict = ladder
        .judge_with(&[Currency::Heap], pins, &mut out)
        .expect("writing to a Vec succeeds");
    (
        verdict,
        String::from_utf8(out).expect("the board renders UTF-8"),
    )
}

/// A green ladder that matches its pin passes, and the pin reads the base
/// sweep at the default scale and the top sweep at the acceptance scale: the
/// same pin with its two scales' rows exchanged drifts at both.
///
/// A pin check handed the small or past-power sweep fails the first half,
/// because both families tie there. One handed base and top swapped comes up clean against
/// the exchanged pin, and fails the second half.
#[test]
fn ranking_pin_reads_each_growth_sweep_at_its_own_scale() {
    let (verdict, text) = judge(&ladder(), PINS);
    assert!(verdict.passed(), "the matching pin passes:\n{text}");
    assert!(
        text.contains("worst-case pin: clean (2 pinned rows verified at 2 sampling scales)"),
        "the clean pin reports itself:\n{text}"
    );

    let swapped = [("default", OP, ["beta"]), ("acceptance", OP, ["alpha"])];
    let (verdict, text) = judge(&ladder(), &swapped);
    assert_eq!(verdict.summary.red, 0, "the readings are still green");
    assert!(
        !verdict.pin_clean && !verdict.passed(),
        "the swapped pin drifts:\n{text}"
    );
    for (label, pinned, live) in [
        ("default", "beta", "alpha"),
        ("acceptance", "alpha", "beta"),
    ] {
        assert!(
            text.contains(&format!(
                "{OP} x heap at the {label} scale: pinned worst {pinned}, live worst {live}"
            )),
            "the {label} scale drifts from {pinned} to {live}:\n{text}"
        );
    }
}

/// A cell red only on the small-input sweep fails the verdict, and the ranking
/// pin is still checked and reported beside it.
///
/// The small sweep binds acceptance even though the growth fit never reads it,
/// and a red cell never stops the pin from running: a change that breaks
/// both reports both.
#[test]
fn small_input_red_fails_the_verdict_and_the_pin_still_runs() {
    let mut red = ladder();
    let over_ceiling = heap_ceiling(SMALL_DENOM) as u64 + 1;
    red.small = sweep(SMALL_DENOM, over_ceiling, green_cap(SMALL_DENOM) / 2);
    let (verdict, text) = judge(&red, PINS);
    assert_eq!(verdict.summary.red, 1, "alpha is red once:\n{text}");
    assert_eq!(verdict.summary.green, 1, "beta stays green:\n{text}");
    assert!(verdict.pin_clean, "the pin is unmoved:\n{text}");
    assert!(!verdict.passed(), "a red cell fails the verdict");
    assert!(
        text.contains("worst-case pin: clean"),
        "the pin runs after a red acceptance:\n{text}"
    );
}

/// A cell red only on the past-power sweep fails the verdict.
///
/// The past-power samples sit where a doubling buffer holds its peak, and
/// their readings bind every ceiling though no growth fit reads them.
#[test]
fn past_power_red_fails_the_verdict() {
    let mut red = ladder();
    let over_ceiling = heap_ceiling(PAST_POWER_DENOM) as u64 + 1;
    red.past_power = sweep(
        PAST_POWER_DENOM,
        over_ceiling,
        green_cap(PAST_POWER_DENOM) / 4,
    );
    let (verdict, text) = judge(&red, PINS);
    assert_eq!(verdict.summary.red, 1, "alpha is red once:\n{text}");
    assert!(verdict.pin_clean, "the pin is unmoved:\n{text}");
    assert!(!verdict.passed(), "a red cell fails the verdict");
}

/// Each acceptance matrix renders its own sweep: the base, top, small-input,
/// and past-power matrices each show that sweep's per-byte heap reading for
/// `alpha`.
///
/// `alpha`'s reading per input byte differs between the four sweeps (the
/// test checks that premise first), so a matrix rendered from another sweep
/// shows another value.
#[test]
fn acceptance_matrices_render_their_own_sweeps() {
    let per_byte = |reading: u64, denom: usize| format!("{:.1}/B", reading as f64 / denom as f64);
    let [small, past_power, base, top] = alpha_readings();
    let expected = [
        ("=== ladder base:", per_byte(base, BASE_DENOM)),
        ("=== ladder top:", per_byte(top, TOP_DENOM)),
        ("=== small inputs:", per_byte(small, SMALL_DENOM)),
        (
            "=== past a power of two:",
            per_byte(past_power, PAST_POWER_DENOM),
        ),
    ];
    let distinct: std::collections::BTreeSet<&String> =
        expected.iter().map(|(_, reading)| reading).collect();
    assert_eq!(
        distinct.len(),
        expected.len(),
        "the fixture's per-byte readings must tell the four sweeps apart: {expected:?}"
    );

    let (_, text) = judge(&ladder(), PINS);
    let section = |header: &str| -> &str {
        let start = text
            .find(header)
            .unwrap_or_else(|| panic!("the judgment renders {header:?}:\n{text}"));
        let rest = &text[start + header.len()..];
        &rest[..rest.find("=== ").unwrap_or(rest.len())]
    };
    let alpha_heap = |section: &str| -> String {
        let row = section
            .lines()
            .find(|line| line.contains(OP) && line.contains("alpha"))
            .unwrap_or_else(|| panic!("the matrix has an alpha row:\n{section}"));
        let heap = row
            .split("heap[")
            .nth(1)
            .and_then(|rest| rest.split(']').next())
            .unwrap_or_else(|| panic!("the row has a heap column: {row}"));
        heap.split_whitespace()
            .last()
            .unwrap_or_else(|| panic!("the heap column has a reading: {heap}"))
            .to_owned()
    };
    for (header, reading) in &expected {
        assert_eq!(
            &alpha_heap(section(header)),
            reading,
            "the matrix under {header:?} renders its own sweep"
        );
    }
}

/// The capture writes each sweep under its own sampling's label, in ladder
/// order: the small pair, the past-power pair, the base pair, then the top
/// pair.
///
/// Each sweep's `alpha` heap reading is distinct, so a capture that wrote one
/// sweep under another's label shows the wrong reading on that line.
#[test]
fn capture_writes_each_sweep_under_its_own_sampling() {
    let readings = alpha_readings();
    let distinct: std::collections::BTreeSet<u64> = readings.into_iter().collect();
    assert_eq!(
        distinct.len(),
        readings.len(),
        "the fixture's readings must tell the four sweeps apart: {readings:?}"
    );

    let mut out = Vec::new();
    capture(&ladder(), "tree-a", &mut out).expect("writing to a Vec succeeds");
    let text = String::from_utf8(out).expect("a capture is UTF-8");
    let alpha_heap: Vec<(String, String, String)> = text
        .lines()
        .filter_map(|line| {
            let fields: Vec<&str> = line.split('\t').collect();
            let [op, family, "heap", scale, sample, _axis, _units, reading] = fields[..] else {
                return None;
            };
            (op == OP && family == "alpha")
                .then(|| (scale.to_owned(), sample.to_owned(), reading.to_owned()))
        })
        .collect();
    let expected: Vec<(String, String, String)> = LADDER_SAMPLINGS
        .iter()
        .zip(readings)
        .flat_map(|(sampling, reading)| {
            ["1", "2"].map(|sample| (sampling.to_string(), sample.to_owned(), reading.to_string()))
        })
        .collect();
    assert_eq!(
        alpha_heap, expected,
        "the capture's alpha heap row:\n{text}"
    );
}

/// A verdict that requires a currency the run did not measure is refused
/// before anything is judged.
///
/// A run whose build left a counter out, or whose runner supplied no meter,
/// reads `None` in that currency everywhere, and the judgment skips it; were
/// the verdict not refused, such a run would pass while judging nothing in
/// that currency.
#[test]
fn verdicts_refuse_runs_missing_a_required_currency() {
    let mut out = Vec::new();
    let refused = ladder()
        .accept(&[Currency::Heap, Currency::Fuel], &mut out)
        .expect_err("the fixture measures no fuel");
    assert_eq!(refused.kind(), std::io::ErrorKind::InvalidInput);
    assert!(out.is_empty(), "nothing is rendered for a refused run");
    ladder()
        .judge_with(&[Currency::Fuel], PINS, &mut out)
        .expect_err("judge refuses as accept does");
}

/// `ladder` with every sample's heap measured against a model that restates
/// the defaults: the sample's own growth and proportional axes, and no
/// ceiling of its own.
fn restating_models(mut ladder: Ladder) -> Ladder {
    let cells = ladder
        .small
        .iter_mut()
        .chain(&mut ladder.past_power)
        .chain(&mut ladder.base)
        .chain(&mut ladder.top);
    for (_, _, first, second) in cells {
        for sample in [first, second] {
            sample.models.heap = Some(Model {
                trend_units: sample.exp_denom_bytes as u64,
                constant_units: sample.denom_bytes as u64,
                ceiling: None,
            });
        }
    }
    ladder
}

/// An operation whose heap model restates the defaults at every sample of its
/// ladder is refused: the rendered row would disclose an override that
/// changes nothing.
#[test]
#[should_panic(expected = "probe_op's heap model must change a unit axis or the ceiling")]
fn a_model_restating_the_defaults_everywhere_is_refused() {
    restating_models(ladder()).assert_models_override();
}

/// A model that departs from the defaults at one sample of one sweep, and
/// restates them everywhere else, passes the guard.
///
/// A model whose units are a function of the operands coincides with the
/// defaults on families whose operands stay small, so the guard judges an
/// operation's model across its whole ladder, never per sample.
#[test]
fn a_model_departing_from_the_defaults_at_one_sample_passes() {
    let mut ladder = restating_models(ladder());
    let model = ladder.top[1]
        .3
        .models
        .heap
        .as_mut()
        .expect("every sample is modeled");
    model.constant_units += 1;
    ladder.assert_models_override();
}
