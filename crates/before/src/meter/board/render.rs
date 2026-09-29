//! The measurement discipline and the printed matrix: how one grid cell is
//! measured and judged, and how a whole board's judged cells render.
//!
//! The sweep itself is the `shard` module's — a child measures its slice of the
//! operation × family grid through [`measure_cell`] and the parent renders the
//! merged cells through [`render_results`].

use std::io::{self, Write};

use super::ceilings::{
    HEAP_INTERCEPT_BYTES, HEAP_TREND_RESOLUTION_BYTES, MAX_HEAP_BYTES_PER_INPUT_BYTE,
    MAX_SCALING_EXPONENT, MAX_SCAN_BITS_PER_INPUT_BYTE, MAX_TOUCHES_PER_INPUT_BYTE,
    MIN_EXPONENT_DENOM_GROWTH,
};
use super::currency::{Currency, Liveness};
use super::family::FamilyData;
use super::judge::{evaluate, CellResult, Score};
use super::measure::{measure, HeapMeter};
use super::ops::Op;
use crate::meter::registry::FamilyId;

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
/// work) or a stale declared model.
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
    kinds.join("+")
}

/// Render one result row.
///
/// The byte range is the cell's denominator. The `flr` column shows the larger
/// scale's committed liveness floors
/// per judged column (`-` where not applicable; derivations in the legend above
/// the matrix).
fn row(out: &mut dyn Write, r: &CellResult) -> io::Result<()> {
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
    let custom_constant_units = |currency: Currency| {
        let ordinary = r.s2.denom_bytes;
        r.s2.models
            .get(currency)
            .is_some_and(|model| model.constant_units != ordinary)
    };
    let heap_unit = if custom_constant_units(Currency::Heap) {
        "/u"
    } else {
        "/B"
    };
    let scan_unit = if custom_constant_units(Currency::Scan) {
        "/u"
    } else {
        "/B"
    };
    let touch_unit = if custom_constant_units(Currency::Touch) {
        "/u"
    } else {
        "/B"
    };
    let scan = match (r.scores.scan.exp, r.scores.scan.per_unit) {
        (Some(_), Some(c)) => format!("scan[e{} {c:>10.1}{scan_unit}]", exp_text(&r.scores.scan)),
        _ => "scan[      off      ]".to_string(),
    };
    let touch = match (r.scores.touch.exp, r.scores.touch.per_unit) {
        (Some(_), Some(c)) => format!(
            "touch[e{} {c:>10.1}{touch_unit}]",
            exp_text(&r.scores.touch)
        ),
        _ => "touch[      off      ]".to_string(),
    };
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
            .filter_map(|(currency, second)| {
                let second = second.as_ref()?;
                let first =
                    r.s1.models.get(currency).as_ref().expect(
                        "a cell's resource-model applicability is independent of sample size",
                    );
                let first_default_constant = r.s1.denom_bytes;
                let second_default_constant = r.s2.denom_bytes;
                let custom_trend = first.trend_units != r.s1.exp_denom_bytes
                    || second.trend_units != r.s2.exp_denom_bytes;
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
        "{verdict:<5} {op:<24} {family:<12} {n1:>8}->{n2:<8} B  \
         heap[e{he} {hc:>10.1}{heap_unit}]  {scan}  {touch}  \
         flr[h {fh:>6} s {fs:>6} t {ft:>6}]{expd}{model}{reasons}",
        op = r.op,
        family = r.family,
        n1 = r.s1.denom_bytes,
        n2 = r.s2.denom_bytes,
        he = exp_text(&r.scores.heap),
        hc = r.scores.heap.per_unit.unwrap_or(0.0),
        fh = floor_value(r.s2.floors.heap),
        fs = floor_value(r.s2.floors.scan),
        ft = floor_value(r.s2.floors.touch),
    )
}

/// The one scale guard, shared by every entry that measures.
pub(super) fn assert_scale(scale: f64) {
    assert!(
        scale > 0.0 && scale.is_finite(),
        "amp-board: scale must be a positive finite number"
    );
}

/// Build one family's operand bundles at both sample levels.
pub(super) fn build_pair(kind: FamilyId, scale: f64) -> (FamilyData, FamilyData) {
    (
        FamilyData::build(kind, scale, 0),
        FamilyData::build(kind, scale, 1),
    )
}

/// Measure and judge one grid cell — or nothing, where the family's bundle
/// supplies no operand for the operation's signature.
///
/// The board's one measurement discipline, driven by every shard child (the
/// `shard` module): single-threaded, the cell at the scaled size and its
/// double, the peak-heap counter reset between samples.
pub(super) fn measure_cell(
    heap: &HeapMeter,
    op: &Op,
    (small, large): &(FamilyData, FamilyData),
) -> Option<CellResult> {
    let c1 = (op.prepare)(small)?;
    let c2 =
        (op.prepare)(large).expect("a cell's applicability depends on the family, never the size");
    let s1 = measure(heap, c1, small.content_bytes);
    let s2 = measure(heap, c2, large.content_bytes);
    Some(evaluate(op.name, small.name, s1, s2))
}

/// Render one full sweep's judged cells as the matrix: the legend derived from
/// the results themselves, red rows first, the summary line last.
///
/// `results` must be a whole board's cells in board row order (operation outer,
/// family inner): a shard merge's reconstruction of one sweep.
pub(super) fn render_results(results: &[CellResult], out: &mut dyn Write) -> io::Result<Summary> {
    writeln!(
        out,
        "amplification board: peak additional heap and work vs each cell's model \
         (input bytes by default; total I/O for output-dominated operations; value width \
         for numbers). Per-unit readings show the larger cost density of the two samples."
    )?;
    writeln!(
        out,
        "green iff every meter's exponent <= {MAX_SCALING_EXPONENT}, constants within: \
         heap <= {HEAP_INTERCEPT_BYTES} + {MAX_HEAP_BYTES_PER_INPUT_BYTE} * units bytes, \
         scan <= {MAX_SCAN_BITS_PER_INPUT_BYTE} bits/B, \
         touch <= {MAX_TOUCHES_PER_INPUT_BYTE} touches/B; \
         and every liveness floor met. All ceilings and floors apply at both sample sizes. \
         Growth fits require denominator span >= x{MIN_EXPONENT_DENOM_GROWTH}; heap readings \
         are clamped to {HEAP_TREND_RESOLUTION_BYTES} B only for that fit. Growth below that \
         resolution is unobserved by the fit. Small-input samples check only ceilings and \
         floors. An unjudged exponent renders -.--. These finite checks are regression \
         evidence, not universal resource guarantees."
    )?;
    writeln!(out)?;
    writeln!(out, "liveness declarations on this board:")?;
    let mut legend = std::collections::BTreeSet::new();
    for r in results {
        for (currency, liveness) in r.s2.floors.each() {
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
            .any(|(_, model)| model.is_some())
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
        row(out, r)?;
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
