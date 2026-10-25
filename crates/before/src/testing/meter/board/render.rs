//! The measurement discipline and the printed matrix: how one grid cell is
//! measured and judged, and how a whole board's judged cells render.
//!
//! The sweep itself is the `shard` module's — a child measures its slice of the
//! operation × family grid through [`measure_cell`] and the parent renders the
//! merged cells through [`render_results`].

use std::io::{self, Write};

use super::ceilings::{
    HEAP_INTERCEPT_BYTES, MAX_HEAP_BYTES_PER_INPUT_BYTE, MAX_SCALING_EXPONENT,
    MAX_SCAN_BITS_PER_INPUT_BYTE, MAX_TOUCHES_PER_INPUT_BYTE, MIN_EXPONENT_DENOM_GROWTH,
};
use super::cell_ceilings::Calibration;
use super::currency::{ByCurrency, Currency, Liveness};
use super::family::FamilyData;
use super::judge::{currency_intercept, trend_resolution, CellResult, Score};
use super::measure::{measure, Meters, Sample};
use super::ops::Op;
use super::sampling::Sampling;
use crate::testing::meter::registry::FamilyId;

/// The board's bottom line: how many cells scored green and red.
#[derive(Debug, Clone, Copy)]
pub struct Summary {
    /// Cells within every ceiling and exponent bound.
    pub green: usize,
    /// Cells over at least one bound, i.e. amplification findings.
    pub red: usize,
}

/// Render one liveness declaration's floor value: the committed minimum, or
/// `-` for a not-applicable column.
fn floor_value(liveness: Liveness) -> String {
    match liveness {
        Liveness::Floor { min, .. } => min.to_string(),
        Liveness::NotApplicable { .. } => "-".to_string(),
    }
}

/// A red cell's mechanism tag: the judgment kinds present on its red list, in a
/// fixed order.
///
/// An `exponent` red is a scaling-class finding; a `constant` red is a
/// proportionality finding at
/// exponent ~1; a `floor` red is a liveness vacuity (a meter not watching the
/// work) or a stale declared model; an `uncalibrated` red is a cell its
/// per-cell table omits, awaiting a recalibration.
fn mechanism(red: &[&'static str]) -> String {
    let mut kinds = Vec::new();
    if red.iter().any(|label| label.contains("exponent")) {
        kinds.push("exponent");
    }
    if red.iter().any(|label| label.ends_with("constant")) {
        kinds.push("constant");
    }
    if red.iter().any(|label| label.contains("floor")) {
        kinds.push("floor");
    }
    if red.iter().any(|label| label.ends_with("uncalibrated")) {
        kinds.push("uncalibrated");
    }
    kinds.join("+")
}

/// Which currencies the board measured: those with a reading in some cell.
///
/// A currency no cell measured (a feature-gated counter not compiled in, or a
/// meter the runner does not supply) gets no column.
fn measured(results: &[CellResult]) -> ByCurrency<bool> {
    ByCurrency::from_fn(|currency| {
        results
            .iter()
            .any(|r| r.scores.get(currency).per_unit.is_some())
    })
}

/// One currency's ceiling, as the matrix's preamble states it: its
/// board-wide ceiling, and the per-cell table that tightens it, if any.
///
/// A currency whose table does not hold for the run's architecture is judged
/// without one, and the text names the missing table.
pub(super) fn ceiling_text(currency: Currency, calibration: &Calibration) -> String {
    let label = currency.label();
    let board = match currency {
        Currency::Heap => Some(format!(
            "heap <= {HEAP_INTERCEPT_BYTES} + {MAX_HEAP_BYTES_PER_INPUT_BYTE} * units bytes"
        )),
        Currency::Scan => Some(format!("scan <= {MAX_SCAN_BITS_PER_INPUT_BYTE} bits/B")),
        Currency::Touch => Some(format!("touch <= {MAX_TOUCHES_PER_INPUT_BYTE} touches/B")),
        Currency::Fuel => None,
    };
    let own = calibration.table(currency).map(|table| {
        format!(
            "{label} <= {intercept} + the cell's own ceiling * units (calibrated from {archs})",
            intercept = currency_intercept(currency),
            archs = table.archs.join(" and ")
        )
    });
    let missing = calibration.missing(currency).map(|archs| {
        format!(
            "no per-cell {label} table for {arch}; the table is calibrated from {archs}",
            arch = calibration.arch,
            archs = archs.join(" and ")
        )
    });
    match (board, own, missing) {
        (Some(board), Some(own), _) => format!("{board} and {own}"),
        (Some(board), None, Some(missing)) => format!("{board} ({missing})"),
        (Some(board), None, None) => board,
        (None, Some(own), _) => own,
        (None, None, missing) => format!(
            "{label} unbounded: {}",
            missing.unwrap_or_else(|| "no per-cell table".to_owned())
        ),
    }
}

/// Render one result row, with a column for each `measured` currency.
///
/// The byte range is the cell's denominator. The `flr` column shows the larger
/// scale's committed liveness floors
/// per judged column (`-` where not applicable; derivations in the legend above
/// the matrix).
fn row(out: &mut dyn Write, r: &CellResult, measured: &ByCurrency<bool>) -> io::Result<()> {
    let verdict = if r.red.is_empty() { "GREEN" } else { "RED" };
    // An exponent the guards leave unjudged renders -.-- : printing the
    // fitted digits would invite reading noise as a measurement.
    let exp_text = |s: &Score| -> String {
        match s.exp {
            Some(e) if s.exp_judged => format!("{e:5.2}"),
            Some(_) => " -.--".to_string(),
            None => "     ".to_string(),
        }
    };
    let columns: Vec<String> = r
        .scores
        .each()
        .into_iter()
        .filter(|(currency, _)| *measured.get(*currency))
        .map(|(currency, score)| {
            let label = currency.label();
            let ordinary = r.s2.denom_bytes as u64;
            let custom_constant_units =
                r.s2.models
                    .get(currency)
                    .is_some_and(|model| model.constant_units != ordinary);
            let unit = if custom_constant_units { "/u" } else { "/B" };
            match (score.exp, score.per_unit) {
                (Some(_), Some(c)) => format!("{label}[e{} {c:>10.1}{unit}]", exp_text(score)),
                _ => format!("{label}[      off      ]"),
            }
        })
        .collect();
    let floors: Vec<String> =
        r.s2.floors
            .each()
            .into_iter()
            .filter(|(currency, _)| *measured.get(*currency))
            .map(|(currency, liveness)| {
                let initial = &currency.label()[..1];
                format!("{initial} {:>6}", floor_value(*liveness))
            })
            .collect();
    // A red cell's mechanism tag: which judgment kinds put it on the red list,
    // mirroring the tags a red-buffer triage entry commits.
    let reasons = if r.red.is_empty() {
        String::new()
    } else {
        format!("  mech[{}]  <- {}", mechanism(&r.red), r.red.join(", "))
    };
    // A cell whose exponents are fitted against a different denominator than
    // its constants discloses the pair on its own row.
    let expd = if r.s2.exp_denom_bytes == r.s2.denom_bytes {
        String::new()
    } else {
        format!(
            "  expd[content {e1}->{e2} B]",
            e1 = r.s1.exp_denom_bytes,
            e2 = r.s2.exp_denom_bytes,
        )
    };
    // Every non-default resource model discloses its concrete units on the row
    // it judges. This makes a generous work law visible instead of turning it
    // into a hidden exemption.
    let modeled =
        r.s2.models
            .each()
            .into_iter()
            .filter(|(currency, _)| *measured.get(*currency))
            .filter_map(|(currency, second)| {
                let second = second.as_ref()?;
                let first =
                    r.s1.models.get(currency).as_ref().expect(
                        "a cell's resource-model applicability is independent of sample size",
                    );
                let first_default_constant = r.s1.denom_bytes as u64;
                let second_default_constant = r.s2.denom_bytes as u64;
                let custom_trend = first.trend_units != r.s1.exp_denom_bytes as u64
                    || second.trend_units != r.s2.exp_denom_bytes as u64;
                let custom_constant = first.constant_units != first_default_constant
                    || second.constant_units != second_default_constant;
                let mut parts = Vec::new();
                if custom_trend
                    && custom_constant
                    && first.trend_units == first.constant_units
                    && second.trend_units == second.constant_units
                {
                    parts.push(format!(
                        "units {}->{}",
                        first.trend_units, second.trend_units
                    ));
                } else {
                    if custom_trend {
                        parts.push(format!(
                            "trend {}->{}",
                            first.trend_units, second.trend_units
                        ));
                    }
                    if custom_constant {
                        parts.push(format!(
                            "constant {}->{}",
                            first.constant_units, second.constant_units
                        ));
                    }
                }
                if let Some(ceiling) = second.ceiling {
                    parts.push(format!("ceiling {ceiling:.1}"));
                }
                Some(format!("{} {}", currency.label(), parts.join(", ")))
            })
            .collect::<Vec<_>>();
    let model = if modeled.is_empty() {
        String::new()
    } else {
        format!("  model[{}]", modeled.join("; "))
    };
    writeln!(
        out,
        "{verdict:<5} {op:<24} {family:<12} {n1:>8}->{n2:<8} B  {columns}  \
         flr[{floors}]{expd}{model}{reasons}",
        op = r.op,
        family = r.family,
        n1 = r.s1.denom_bytes,
        n2 = r.s2.denom_bytes,
        columns = columns.join("  "),
        floors = floors.join(" "),
    )
}

/// Build one family's operand bundles at both sample levels.
pub(super) fn build_pair(kind: FamilyId, sampling: Sampling) -> (FamilyData, FamilyData) {
    (
        FamilyData::build(kind, sampling, 0),
        FamilyData::build(kind, sampling, 1),
    )
}

/// Measure one grid cell's two samples — or nothing, where the family's
/// bundle supplies no operand for the operation's signature.
///
/// The board's one measurement discipline, driven by every shard child (the
/// `shard` module): single-threaded, the cell at the scaled size and its
/// double, the peak-heap counter reset between samples. Judgment stays with
/// the parent that merges the children's samples.
pub(super) fn measure_cell(
    meters: &Meters,
    op: &Op,
    (small, large): &(FamilyData, FamilyData),
) -> Option<(Sample, Sample)> {
    let c1 = (op.prepare)(small)?;
    let c2 =
        (op.prepare)(large).expect("a cell's applicability depends on the family, never the size");
    let s1 = measure(meters, c1, small.content_bytes);
    let s2 = measure(meters, c2, large.content_bytes);
    Some((s1, s2))
}

/// Render one full sweep's judged cells as the matrix: the legend derived from
/// the results themselves, red rows first, the summary line last.
///
/// `results` must be a whole board's cells in board row order (operation outer,
/// family inner): a shard merge's reconstruction of one sweep, judged under
/// `calibration`.
pub(super) fn render_results(
    calibration: &Calibration,
    results: &[CellResult],
    out: &mut dyn Write,
) -> io::Result<Summary> {
    let measured = measured(results);
    let ceilings: Vec<String> = measured
        .each()
        .into_iter()
        .filter(|(_, measured)| **measured)
        .map(|(currency, _)| ceiling_text(currency, calibration))
        .collect();
    let clamps: Vec<String> = measured
        .each()
        .into_iter()
        .filter(|(_, measured)| **measured)
        .filter_map(|(currency, _)| {
            let resolution = trend_resolution(currency)?;
            let unit = match currency {
                Currency::Heap => "B",
                Currency::Scan | Currency::Touch | Currency::Fuel => currency.label(),
            };
            Some(format!(
                "{} readings are clamped to {resolution} {unit}",
                currency.label()
            ))
        })
        .collect();
    writeln!(
        out,
        "amplification board: peak additional heap and work vs each cell's model \
         (input bytes by default; total I/O for output-dominated operations; value width \
         for numbers). Per-unit readings show the larger cost density of the two samples."
    )?;
    writeln!(
        out,
        "green iff every meter's exponent <= {MAX_SCALING_EXPONENT}, constants within: \
         {ceilings}; \
         and every liveness floor met. All ceilings and floors apply at both sample sizes. \
         Growth fits require denominator span >= x{MIN_EXPONENT_DENOM_GROWTH}; {clamps} \
         only for that fit. Growth below that resolution is unobserved by the fit. \
         Small-input and past-power samples check only ceilings and floors. An unjudged \
         exponent renders \
         -.--. These finite checks are regression evidence, not universal resource \
         guarantees.",
        ceilings = ceilings.join(", "),
        clamps = clamps.join(" and "),
    )?;
    writeln!(out)?;
    writeln!(out, "liveness declarations on this board:")?;
    let mut legend = std::collections::BTreeSet::new();
    for r in results {
        for (currency, liveness) in r.s2.floors.each() {
            if !*measured.get(currency) {
                continue;
            }
            legend.insert(match liveness {
                Liveness::Floor { why, .. } => format!("  {} floor: {why}", currency.label()),
                Liveness::NotApplicable { reason } => {
                    format!("  {} n/a: {reason}", currency.label())
                }
            });
        }
    }
    for line in &legend {
        writeln!(out, "{line}")?;
    }
    if results.iter().any(|result| {
        result
            .s2
            .models
            .each()
            .iter()
            .any(|(currency, model)| *measured.get(*currency) && model.is_some())
    }) {
        writeln!(
            out,
            "  resource models (model[...] rows): `trend` units govern the exponent fit; \
             `constant` units govern the proportional reading; one `units` range means both \
             axes agree. An explicit ceiling replaces only that currency's global \
             proportional ceiling. Every model retains the global exponent bound and is \
             printed on each row that uses it"
        )?;
    }
    writeln!(out)?;

    let red: Vec<&CellResult> = results.iter().filter(|r| !r.red.is_empty()).collect();
    let green: Vec<&CellResult> = results.iter().filter(|r| r.red.is_empty()).collect();
    for r in red.iter().chain(green.iter()) {
        row(out, r, &measured)?;
    }

    writeln!(out)?;
    writeln!(
        out,
        "amp-board: {} green / {} red ({} cells)",
        green.len(),
        red.len(),
        results.len()
    )?;
    Ok(Summary {
        green: green.len(),
        red: red.len(),
    })
}
