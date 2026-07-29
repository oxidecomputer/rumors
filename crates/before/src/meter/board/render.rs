//! The board driver: sweep the whole shape × operation product, judge
//! every cell, and render the matrix.

use std::io::{self, Write};

use super::ceilings::{
    CAPACITY_MODEL_CEILING, CAPACITY_MODEL_FLOOR, FOLD_SCAN_BITS_PER_INPUT_BYTE_PER_LEVEL,
    HEAP_FLAT_ALLOWANCE_BYTES, MAX_GROWN_STACK_SEGMENTS, MAX_HEAP_BYTES_PER_INPUT_BYTE,
    MAX_LIMB_OPS_PER_INPUT_BYTE, MAX_SCALING_EXPONENT, MAX_SCAN_BITS_PER_INPUT_BYTE,
    MAX_TEXT_LIMB_OPS_PER_RADIX_UNIT, MAX_TOUCHES_PER_INPUT_BYTE, MIN_EXPONENT_DENOM_GROWTH,
};
use super::currency::Liveness;
use super::family::{FamilyData, FAMILIES};
use super::judge::{assert_deterministic, evaluate, CellResult, Score};
use super::measure::{measure, HeapMeter};
use super::ops::ops;

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

/// A red cell's mechanism tag: the judgment kinds present on its red
/// list, in a fixed order.
///
/// An `exponent` red is a scaling-class finding; a `constant` red (flat
/// or declared-model ceilings, the segments count) is a proportionality
/// finding at exponent ~1; a `floor` red is a liveness vacuity (a meter
/// not watching the work) or a stale declared model.
fn mechanism(red: &[&'static str]) -> String {
    let mut kinds = Vec::new();
    if red.iter().any(|label| label.contains("exponent")) {
        kinds.push("exponent");
    }
    if red.iter().any(|label| {
        label.contains("constant") || label.contains("count") || label.contains("ceiling")
    }) {
        kinds.push("constant");
    }
    if red.iter().any(|label| label.contains("floor")) {
        kinds.push("floor");
    }
    kinds.join("+")
}

/// Render one result row.
///
/// The byte range is the cell's denominator (packed input, or `n_io` on the
/// I/O-denominated cells); a text row's limb constant reads `/R` (its
/// exponent, like every exponent, is against the denominator bytes),
/// everything else `/B`. The `flr` column shows the larger scale's committed
/// liveness floors per judged column (`-` where not applicable; derivations
/// in the legend above the matrix).
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
    let limb = match (r.scores.limb.exp, r.scores.limb.per_unit) {
        (Some(_), Some(c)) => {
            let unit = if r.s2.text_row { "/R" } else { "/B" };
            format!("limb[e{} {c:>10.1}{unit}]", exp_text(&r.scores.limb))
        }
        _ => "limb[      off      ]".to_string(),
    };
    let scan = match (r.scores.scan.exp, r.scores.scan.per_unit) {
        (Some(_), Some(c)) => format!("scan[e{} {c:>10.1}/B]", exp_text(&r.scores.scan)),
        _ => "scan[      off      ]".to_string(),
    };
    let touch = match (r.scores.touch.exp, r.scores.touch.per_unit) {
        (Some(_), Some(c)) => format!("touch[e{} {c:>10.1}/B]", exp_text(&r.scores.touch)),
        _ => "touch[      off      ]".to_string(),
    };
    // A red cell's mechanism tag: which judgment kinds put it on the red
    // list (the class-binding seal in `testing::complexity_claims` keys
    // on the exponent kind).
    let reasons = if r.red.is_empty() {
        String::new()
    } else {
        format!("  mech[{}]  <- {}", mechanism(&r.red), r.red.join(", "))
    };
    // A cell whose exponents are fitted against a different denominator
    // than its constants discloses the pair on its own row.
    let expd = if r.s2.exp_denom_bytes == r.s2.denom_bytes {
        String::new()
    } else {
        format!(
            "  expd[content {e1}->{e2} B]",
            e1 = r.s1.exp_denom_bytes,
            e2 = r.s2.exp_denom_bytes,
        )
    };
    // A declared per-cell model is disclosed on the row it judges; the
    // legend above the matrix carries the derivations.
    let decl = match (r.s1.heap_model, r.s2.heap_model, r.s2.fold_arity) {
        _ if r.s2.declared_heap.is_some() => {
            let d = r.s2.declared_heap.expect("just matched");
            format!("  decl[heap {d:.0} B/B family-stated]")
        }
        _ if r.s2.declared_limb.is_some() => {
            let (e, k) = r.s2.declared_limb.expect("just matched");
            format!("  decl[limb e {e:.2} {k:.2}/R family-stated]")
        }
        (Some(m1), Some(m2), _) => {
            format!("  decl[heap cap-chain {m1:.0}->{m2:.0} B]")
        }
        (_, _, Some(k2)) => {
            let k1 = r.s1.fold_arity.expect("fold cells declare both scales");
            if r.s2.fold_search_bits > 0 {
                format!(
                    "  decl[fold k {k1}->{k2} search {s1}->{s2} bits]",
                    s1 = r.s1.fold_search_bits,
                    s2 = r.s2.fold_search_bits,
                )
            } else {
                format!("  decl[fold k {k1}->{k2}]")
            }
        }
        _ => String::new(),
    };
    writeln!(
        out,
        "{verdict:<5} {op:<24} {family:<12} {n1:>8}->{n2:<8} B  \
         heap[e{he} {hc:>10.1}/B]  seg[e{se} {sc:>4}]  {limb}  {scan}  {touch}  \
         flr[h {fh:>6} l {fl:>6} s {fs:>6} t {ft:>6}]{expd}{decl}{reasons}",
        op = r.op,
        family = r.family,
        n1 = r.s1.denom_bytes,
        n2 = r.s2.denom_bytes,
        he = exp_text(&r.scores.heap),
        hc = r.scores.heap.per_unit.unwrap_or(0.0),
        se = exp_text(&r.scores.segments),
        sc = r.s2.readings.segments.unwrap_or(0),
        fh = floor_value(r.s2.floors.heap),
        fl = floor_value(r.s2.floors.limb),
        fs = floor_value(r.s2.floors.scan),
        ft = floor_value(r.s2.floors.touch),
    )
}

/// Sweep the whole shape × operation product at `scale` and judge every
/// cell: the one evaluation pass behind both the rendered matrix
/// ([`run`]) and the worst-case fold ([`worst_map`](super::worst::worst_map)).
///
/// Results arrive in board row order (operation outer, family inner),
/// each cell measured at the scaled size and its double, under the
/// runner's in-process determinism self-verification.
///
/// # Panics
///
/// Panics if `scale` is not strictly positive.
pub(super) fn sweep(scale: f64, heap: &HeapMeter) -> Vec<CellResult> {
    assert!(
        scale > 0.0 && scale.is_finite(),
        "amp-board: scale must be a positive finite number"
    );

    let families: Vec<(FamilyData, FamilyData)> = FAMILIES
        .iter()
        .map(|&kind| {
            (
                FamilyData::build(kind, scale, 0),
                FamilyData::build(kind, scale, 1),
            )
        })
        .collect();

    let mut results = Vec::new();
    for op in ops() {
        for (small, large) in &families {
            let Some(c1) = (op.prepare)(small) else {
                continue;
            };
            let c2 = (op.prepare)(large)
                .expect("a cell's applicability depends on the family, never the size");
            let s1 = measure(heap, op.name, c1, small.content_bytes);
            let s2 = measure(heap, op.name, c2, large.content_bytes);
            // The runner self-verifies: every cell is measured twice in
            // process and every counter reading and denominator must
            // agree exactly — the board's judged quantities are
            // deterministic domain counters, so any disagreement is a
            // nondeterminism bug in a meter or a body, stopped here
            // rather than laundered into a verdict.
            for (level, first) in [(small, &s1), (large, &s2)] {
                let again = (op.prepare)(level)
                    .expect("a cell's applicability depends on the family, never the size");
                let second = measure(heap, op.name, again, level.content_bytes);
                assert_deterministic(op.name, small.name, first, &second);
            }
            results.push(evaluate(op.name, small.name, s1, s2));
        }
    }
    results
}

/// Run the whole board and render the matrix to `out`.
///
/// `scale` multiplies every family's base size (1.0 is the seconds-scale
/// default; the smoke test passes a small fraction). Cells run at the scaled
/// size and its double. Red rows print first.
///
/// # Panics
///
/// Panics if `scale` is not strictly positive.
pub fn run(scale: f64, heap: &HeapMeter, out: &mut dyn Write) -> io::Result<Summary> {
    let results = sweep(scale, heap);

    writeln!(
        out,
        "amplification board: transient cost vs denominator bytes (packed input; total I/O on \
         the text and cross cells), each cell at two scales"
    )?;
    writeln!(
        out,
        "green iff every meter's exponent <= {MAX_SCALING_EXPONENT}, constants within: \
         heap <= {MAX_HEAP_BYTES_PER_INPUT_BYTE} B/B over {HEAP_FLAT_ALLOWANCE_BYTES} B flat, \
         segments <= {MAX_GROWN_STACK_SEGMENTS}, \
         limb <= {MAX_LIMB_OPS_PER_INPUT_BYTE} ops/B \
         (text rows: <= {MAX_TEXT_LIMB_OPS_PER_RADIX_UNIT} ops/R), \
         scan <= {MAX_SCAN_BITS_PER_INPUT_BYTE} bits/B, \
         touch <= {MAX_TOUCHES_PER_INPUT_BYTE} touches/B; \
         and every committed liveness floor met (flr[...]: a counter below its floor is red: \
         the meter is not watching that work; segments is ceiling-only by policy, its honest \
         floor is zero). exponent legs are fitted only where the denominator pair scales \
         (>= x{MIN_EXPONENT_DENOM_GROWTH} between probes) and, on heap, where a reading \
         clears the flat allowance the constant leg already forgives; an unjudged exponent \
         renders -.-- and the cell rides its constants and floors. every judged quantity is \
         a deterministic counter: the time-exponent leg lives in the bench judge \
         (just bench-judge)"
    )?;
    writeln!(out)?;
    writeln!(out, "liveness declarations on this board:")?;
    let mut legend = std::collections::BTreeSet::new();
    for r in &results {
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
    if results.iter().any(|r| r.s2.fold_arity.is_some()) {
        writeln!(
            out,
            "  declared fold model (decl[fold ...] rows): the balanced reduction's O(D log k) \
             class - exponent ceilings on limb/scan/touch at the model's predicted exponent \
             plus the linear cells' slack, scan constant at \
             {FOLD_SCAN_BITS_PER_INPUT_BYTE_PER_LEVEL} bits/B per log2(2k) reduction level"
        )?;
    }
    if results.iter().any(|r| r.s2.heap_model.is_some()) {
        writeln!(
            out,
            "  declared capacity model (decl[heap ...] rows): peak = 3(n+m)2^(k-1) B, \
             k = ceil(log2(output/(n+m))) - the output builder's doubling chain anchored at \
             the operand-size reserve; readings banded within x{CAPACITY_MODEL_FLOOR} to \
             x{CAPACITY_MODEL_CEILING} of the model at both scales"
        )?;
    }
    if results.iter().any(|r| r.s2.declared_heap.is_some()) {
        writeln!(
            out,
            "  family-stated heap ceilings (decl[heap ... B/B family-stated] rows): the heap \
             constant is judged at the stated flat ceiling in place of the global \
             {MAX_HEAP_BYTES_PER_INPUT_BYTE} B/B; the exponent leg stays at the global bound \
             (each declaration's derivation lives at its constant)"
        )?;
    }
    if results.iter().any(|r| r.s2.declared_limb.is_some()) {
        writeln!(
            out,
            "  family-stated limb models (decl[limb e ... .../R family-stated] rows): the limb \
             exponent and per-radix-unit constant are judged at the stated ceilings in place \
             of the global exponent bound and the text ceiling — the documented superlinear \
             render class, intended and modeled (each declaration's derivation lives at its \
             constants)"
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
