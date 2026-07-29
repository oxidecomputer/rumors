//! The worst-case map: the argmax family per operation × currency, folded
//! from the same judged cells the rendered matrix walks, with a committed
//! ranking pin.
//!
//! "Which committed shape is worst for operation X" is a mechanically
//! re-derivable fact, never a curated list: the map is a pure fold over
//! [`sweep`]'s cell results — the board's own
//! readings, normalized by each cell's own denominator of record — and the
//! rankings are drift-detected by [`WORST_RANKINGS`], a tamper-evident pin
//! whose diff a reviewer sees ([`check_worst_map`]).
//!
//! # Honest scope
//!
//! The map names the **worst instrumented shape**: the maximum over the
//! committed family roster. The claim that this is the true worst case is
//! carried by the complexity claims and their tripwires, not by this
//! table.
//!
//! # The reading and its denominator
//!
//! Each cell contributes the board's normalized constant of record at the
//! cell's larger sample (`Score::per_unit`, exactly the number the matrix
//! prints): heap bytes net of the flat allowance per denominator byte,
//! limb ops per denominator byte (text rows: per radix-work unit `R`),
//! scan bits and touches per denominator byte. The denominator is the
//! cell's own denominator of record — packed input, or total I/O where the
//! board re-denominates (the `cell` module's Denomination rules) — so the
//! map ranks cost *density*, and a row may mix denominators exactly where
//! the board does.
//!
//! Segments is deliberately absent from the map: it is an absolute,
//! ceiling-only count by policy (the target is walks that never grow the
//! stack), not a per-byte density a normalized argmax can rank.
//!
//! # Ties and near-ties
//!
//! Every judged quantity is a deterministic counter, so an exact tie at
//! the top is a stable fact: the fold records **all** tied families,
//! sorted by name, and the pin carries the whole set — a tie can never
//! make the pin flappy. Near-ties are a *reading* hazard, not a pin
//! hazard: a runner-up within [`NEAR_TIE_RATIO`] is flagged in the
//! rendered table so rank 1 vs rank 2 is not over-read, but the pin still
//! records the exact argmax.

use std::collections::BTreeSet;
use std::io::{self, Write};

use super::ceilings::{ACCEPTANCE_SCALE, HEAP_FLAT_ALLOWANCE_BYTES};
use super::currency::Currency;
use super::judge::CellResult;
use super::measure::HeapMeter;
use super::render::sweep;

/// The two scales of record the worst-case map is rendered and pinned at:
/// the board's seconds-scale default and the acceptance scale
/// ([`ACCEPTANCE_SCALE`], which owns the ×4 calibration argument).
///
/// The map's claim is scale-qualified because a ranking is: a shape's
/// normalized constant carries its intercept at the default scale, and
/// the acceptance scale is where the known onset effects (segment growth,
/// doubling-chain steps) have fired.
pub const WORST_MAP_SCALES: [(&str, f64); 2] = [("default", 1.0), ("acceptance", ACCEPTANCE_SCALE)];

/// A runner-up within this ratio of the worst reading is flagged
/// `~near-tie` in the rendered table.
///
/// The band is the constant-factor headroom the board's family-stated
/// ceilings grant a single reading (a ratified ceiling is the worst
/// reading ×1.25, as at
/// [`MIRROR_WIDE_RENDER_LIMB_OPS_PER_RADIX_UNIT`](super::ceilings::MIRROR_WIDE_RENDER_LIMB_OPS_PER_RADIX_UNIT)):
/// two families inside it are one reading apart, not two classes, so
/// their rank order is a fact about the chosen scale's constants, not
/// about the shapes — the flag stops a reader from over-reading rank 1
/// vs rank 2. The flag never enters the pin: the pin records the exact
/// deterministic argmax, and a flip inside the band is still news worth
/// a look.
pub const NEAR_TIE_RATIO: f64 = 1.25;

/// The map's currency axis: the four normalized counter columns, in
/// render and pin order (segments is excluded; the module doc says why).
const MAP_CURRENCIES: [Currency; 4] = [
    Currency::Heap,
    Currency::Limb,
    Currency::Scan,
    Currency::Touch,
];

/// One family's candidacy for a row's argmax: its normalized reading and
/// whether that reading sits under a declared per-cell model.
pub(super) struct Entry {
    /// The family name.
    pub(super) family: &'static str,
    /// The board's normalized constant of record for the cell.
    pub(super) value: f64,
    /// Whether the cell is judged under a declared model for this
    /// currency (rendered `*`: intended and modeled).
    pub(super) modeled: bool,
}

/// One operation × currency argmax: the worst set, and the best family
/// strictly below it.
pub(super) struct CurrencyWorst {
    /// The currency this column ranks.
    pub(super) currency: Currency,
    /// True when the counter is not compiled into this run (the
    /// feature-gated columns without `limb-meter`/`scan-meter`).
    pub(super) off: bool,
    /// Whether the readings are per radix-work unit `R` (the text rows'
    /// limb constant) rather than per denominator byte.
    pub(super) per_r: bool,
    /// Every family at the maximum reading, sorted by name; empty when no
    /// committed shape drives the currency on this row.
    pub(super) worst: Vec<Entry>,
    /// The best family strictly below the maximum (name-order first on an
    /// exact tie); `None` when every other shape reads zero.
    pub(super) runner_up: Option<Entry>,
}

/// One operation row of the map: the argmax per mapped currency.
pub(super) struct OpWorst {
    /// The board row's operation name.
    pub(super) op: &'static str,
    /// One argmax per [`MAP_CURRENCIES`] column, in that order.
    pub(super) per_currency: Vec<CurrencyWorst>,
}

/// The argmax kernel: the worst set and the runner-up from one row's
/// candidates.
///
/// Zero readings never place (a shape that does none of this work is not
/// a worst case); an empty result means the currency is dead on the row.
/// Exact ties — stable facts, since every reading is a deterministic
/// counter over a fixed denominator — are all recorded, sorted by family
/// name; the runner-up is the best entry strictly below the maximum,
/// name-order first on a tie.
pub(super) fn rank(mut candidates: Vec<Entry>) -> (Vec<Entry>, Option<Entry>) {
    candidates.retain(|e| e.value > 0.0);
    let Some(max) = candidates.iter().map(|e| e.value).max_by(f64::total_cmp) else {
        return (Vec::new(), None);
    };
    let (mut worst, rest): (Vec<Entry>, Vec<Entry>) =
        candidates.into_iter().partition(|e| e.value == max);
    worst.sort_by_key(|e| e.family);
    let runner_value = rest.iter().map(|e| e.value).max_by(f64::total_cmp);
    let runner_up = runner_value.and_then(|v| {
        let mut at = rest
            .into_iter()
            .filter(|e| e.value == v)
            .collect::<Vec<_>>();
        at.sort_by_key(|e| e.family);
        at.into_iter().next()
    });
    (worst, runner_up)
}

/// Whether a cell's reading in `currency` is judged under a declared
/// per-cell model (the `ceilings` module's declared-models section).
///
/// The models by currency: the capacity-chain band and family-stated
/// heap ceilings on heap, the family-stated limb models on limb, and the
/// fold rows' `O(D log k)` model on limb, scan, and touch.
fn modeled(r: &CellResult, currency: Currency) -> bool {
    match currency {
        Currency::Heap => r.s2.declared_heap.is_some() || r.s2.heap_model.is_some(),
        Currency::Limb => r.s2.declared_limb.is_some() || r.s2.fold_arity.is_some(),
        Currency::Scan | Currency::Touch => r.s2.fold_arity.is_some(),
        Currency::Segments => false,
    }
}

/// Fold one sweep's cell results into the worst-case map, in board row
/// order.
pub(super) fn fold(results: &[CellResult]) -> Vec<OpWorst> {
    let mut map: Vec<OpWorst> = Vec::new();
    let mut start = 0;
    while start < results.len() {
        let op = results[start].op;
        let mut end = start;
        while end < results.len() && results[end].op == op {
            end += 1;
        }
        let row = &results[start..end];
        // A row's denomination rule is the operation's, so the text-row
        // marker cannot vary across a row's families.
        assert!(
            row.iter().all(|r| r.s2.text_row == row[0].s2.text_row),
            "worst-case map: {op}: text denomination differs across families"
        );
        let per_currency = MAP_CURRENCIES
            .iter()
            .map(|&currency| {
                let off = row
                    .iter()
                    .any(|r| r.scores.get(currency).per_unit.is_none());
                let candidates = row
                    .iter()
                    .filter_map(|r| {
                        r.scores.get(currency).per_unit.map(|value| Entry {
                            family: r.family,
                            value,
                            modeled: modeled(r, currency),
                        })
                    })
                    .collect();
                let (worst, runner_up) = rank(candidates);
                CurrencyWorst {
                    currency,
                    off,
                    per_r: currency == Currency::Limb && row[0].s2.text_row,
                    worst,
                    runner_up,
                }
            })
            .collect();
        map.push(OpWorst { op, per_currency });
        start = end;
    }
    map
}

/// A reading rendered at a precision that keeps small constants legible
/// without decorating large ones.
fn fmt_value(v: f64) -> String {
    if v >= 100.0 {
        format!("{v:.1}")
    } else if v >= 1.0 {
        format!("{v:.2}")
    } else {
        format!("{v:.4}")
    }
}

/// One worst set rendered as `family[*],family[*] value/unit`.
fn fmt_worst(c: &CurrencyWorst) -> String {
    let names = c
        .worst
        .iter()
        .map(|e| format!("{}{}", e.family, if e.modeled { "*" } else { "" }))
        .collect::<Vec<_>>()
        .join(",");
    let value = c.worst.first().expect("fmt_worst needs a non-empty set");
    format!(
        "{names} {value}{unit}",
        value = fmt_value(value.value),
        unit = if c.per_r { "/R" } else { "/B" },
    )
}

/// Render one operation × currency row of the map.
pub(super) fn row(out: &mut dyn Write, op: &str, c: &CurrencyWorst) -> io::Result<()> {
    let lead = format!("{op:<28} {cur:<5}", cur = c.currency.label());
    if c.off {
        return writeln!(
            out,
            "{lead}  worst off  (counter not compiled into this run: limb-meter/scan-meter)"
        );
    }
    if c.worst.is_empty() {
        return writeln!(
            out,
            "{lead}  worst -  (no committed shape drives this currency on this row)"
        );
    }
    let unit = if c.per_r { "/R" } else { "/B" };
    let tail = match &c.runner_up {
        None => "  runner-up -  (every other shape reads 0)".to_string(),
        Some(ru) => {
            let margin = c.worst[0].value / ru.value;
            format!(
                "  runner-up {name}{star} {value}{unit}  x{margin}{flag}",
                name = ru.family,
                star = if ru.modeled { "*" } else { "" },
                value = fmt_value(ru.value),
                margin = if margin >= 100.0 {
                    format!("{margin:.0}")
                } else {
                    format!("{margin:.2}")
                },
                flag = if margin < NEAR_TIE_RATIO {
                    "  ~near-tie"
                } else {
                    ""
                },
            )
        }
    };
    writeln!(out, "{lead}  worst {worst:<42}{tail}", worst = fmt_worst(c))
}

/// Run one board sweep at `scale` and render the worst-case map table to
/// `out`, one row per operation × mapped currency, in board row order.
///
/// `label` names the scale in the header (the scales of record are
/// [`WORST_MAP_SCALES`]; a smoke run may pass its own). The fold is a
/// pure consumer of the board's own judged cells: no reading, family, or
/// ceiling is recomputed here.
///
/// # Panics
///
/// Panics if `scale` is not strictly positive.
pub fn worst_map(label: &str, scale: f64, heap: &HeapMeter, out: &mut dyn Write) -> io::Result<()> {
    let results = sweep(scale, heap);
    let map = fold(&results);
    writeln!(
        out,
        "worst-case map at the {label} scale (x{scale}): the worst instrumented shape per \
         operation x currency"
    )?;
    writeln!(
        out,
        "  worst instrumented shape: the maximum over the committed family roster - the claim \
         that this is the true worst case is carried by the complexity claims and their \
         tripwires, not by this table."
    )?;
    writeln!(
        out,
        "  reading: the board's normalized constant of record at the cell's larger sample: heap \
         bytes net of the {HEAP_FLAT_ALLOWANCE_BYTES} B flat allowance per denominator byte, \
         limb ops per denominator byte (text rows: per radix-work unit R), scan bits and touches \
         per denominator byte; the denominator is the cell's own denominator of record (packed \
         input, or total I/O where the board re-denominates), so readings rank cost density and \
         a row may mix denominators exactly where the board does."
    )?;
    writeln!(
        out,
        "  margin: worst/runner-up, a ratio: unit-free across a row's denominators and legible \
         across the constants' magnitudes. ~near-tie flags margins under x{NEAR_TIE_RATIO}: the \
         constant-factor band the board's family-stated ceilings treat as one reading, so rank 1 \
         vs rank 2 inside it is one reading apart, not two classes."
    )?;
    writeln!(
        out,
        "  *: the reading sits under a declared per-cell model (the board's decl[...] rows): \
         intended and modeled. segments is absent by policy: an absolute ceiling-only count, \
         not a per-byte density a normalized argmax can rank."
    )?;
    writeln!(out)?;
    for op in &map {
        for c in &op.per_currency {
            row(out, op.op, c)?;
        }
    }
    writeln!(out)?;
    writeln!(
        out,
        "worst-case map: {} operations x {} currencies at the {label} scale",
        map.len(),
        MAP_CURRENCIES.len()
    )
}

/// The committed argmax rankings: `(scale, operation, [heap, limb, scan,
/// touch])`.
///
/// Each column is the worst family set, comma-joined in family-name
/// order, `-` where no committed shape drives the currency; one entry
/// per operation per scale of record, in board row order.
///
/// The tamper-evident ranking pin: [`check_worst_map`] entry-compares the
/// live fold against this table, so "which committed shape is worst for
/// operation X" is a drift-detected fact. A ranking flip is news: either
/// a family legitimately overtook (re-pin deliberately, with a movement
/// annotation dated at the changed entry) or a code change made some
/// shape relatively worse (investigate first). Exact ties are stable
/// deterministic facts and the whole tied set is pinned, so a tie cannot
/// flap this table.
///
/// Pinned from the release-profile fold (the board's profile of record)
/// at both [`WORST_MAP_SCALES`], 2026-07-29.
pub(super) const WORST_RANKINGS: &[(&str, &str, [&str; 4])] = &[
    ("default", "version_decode", ["hugeleaf", "dense", "staircase", "staircase"]),
    ("default", "version_encode", ["promo-rearm", "-", "-", "-"]),
    ("default", "version_cmp", ["hugeleaf", "staircase", "staircase", "staircase"]),
    ("default", "version_eq", ["-", "-", "-", "-"]),
    ("default", "version_concurrent", ["hugeleaf", "staircase", "staircase", "staircase"]),
    ("default", "version_join", ["hugeleaf", "dense", "mirror-wide", "staircase"]),
    ("default", "version_join_assign", ["hugeleaf", "dense", "mirror-wide", "staircase"]),
    ("default", "version_meet", ["hugeleaf", "dense", "weight-comb", "staircase"]),
    ("default", "version_meet_assign", ["hugeleaf", "dense", "weight-comb", "staircase"]),
    ("default", "version_tick", ["ascend-cliff", "mirror-narrow", "hugeleaf", "mirror-narrow"]),
    ("default", "version_ticks", ["ascend-cliff", "mirror-narrow", "reveal-comb", "mirror-narrow"]),
    ("default", "version_tick_adv_party", ["id-pair", "id-pair", "id-pair", "comb-scatter"]),
    ("default", "version_rank", ["hugeleaf", "staircase", "hugeleaf", "harmonic"]),
    ("default", "rank_pair_ops", ["freeze-parade", "concurrent-pair", "-", "-"]),
    ("default", "rank_sum", ["weight-comb", "hugeleaf", "-", "freeze-pos"]),
    ("default", "version_distance", ["promo-rearm", "staircase", "hugeleaf", "staircase"]),
    ("default", "version_lag", ["promo-rearm", "staircase", "hugeleaf", "staircase"]),
    ("default", "version_min_ticks", ["ascend-cliff", "staircase", "staircase", "staircase"]),
    ("default", "version_join_all", ["-", "stagger", "benign", "stagger"]),
    ("default", "version_meet_all", ["-", "stagger", "stagger", "stagger"]),
    ("default", "own_version_to_version", ["hugeleaf", "dense", "promo-rearm", "benign"]),
    ("default", "own_version_cmp", ["hugeleaf", "dense", "promo-rearm", "benign"]),
    ("default", "own_version_pair_cmp", ["hugeleaf", "dense", "jump-pair", "dense"]),
    ("default", "version_display", ["mirror-narrow", "mirror-wide", "jump-pair", "-"]),
    ("default", "version_from_str", ["mirror-narrow", "staircase", "jump-pair", "staircase"]),
    ("default", "version_hash", ["-", "-", "-", "-"]),
    ("default", "causally_contains", ["hugeleaf", "staircase", "staircase", "staircase"]),
    ("default", "party_decode", ["id-pair", "-", "id-pair", "-"]),
    ("default", "party_encode", ["-", "-", "-", "-"]),
    ("default", "party_fork", ["id-pair", "-", "mirror-narrow,nested-full", "-"]),
    ("default", "party_join", ["id-pair", "-", "benign", "-"]),
    ("default", "party_join_all", ["-", "-", "weave", "-"]),
    ("default", "party_covers", ["-", "-", "id-pair", "-"]),
    ("default", "party_disjoint", ["-", "-", "id-pair", "-"]),
    ("default", "party_without", ["id-pair", "-", "id-pair", "-"]),
    ("default", "party_display", ["pure-comb", "-", "mirror-narrow,nested-full", "-"]),
    ("default", "party_from_str", ["id-pair", "-", "mirror-narrow,nested-full", "-"]),
    ("default", "party_hash", ["-", "-", "-", "-"]),
    ("default", "clock_decode", ["id-pair", "dense", "id-pair", "staircase"]),
    ("default", "clock_encode", ["id-pair", "-", "-", "-"]),
    ("default", "clock_tick", ["ascend-cliff", "mirror-narrow", "hugeleaf", "mirror-narrow"]),
    ("default", "clock_fork", ["promo-rearm", "-", "mirror-narrow,nested-full", "-"]),
    ("default", "clock_join", ["hugeleaf", "dense", "id-pair", "concurrent-pair"]),
    ("default", "clock_sync", ["hugeleaf", "dense", "mirror-narrow", "concurrent-pair"]),
    ("default", "clock_recv", ["id-pair", "id-pair", "hugeleaf", "staircase"]),
    ("default", "clock_own_version_to_version", ["hugeleaf", "dense", "promo-rearm", "benign"]),
    ("default", "clock_display", ["tooth-tail", "mirror-wide", "jump-pair", "-"]),
    ("default", "clock_from_str", ["dense,harmonic", "concurrent-pair", "jump-pair", "concurrent-pair"]),
    ("default", "clock_hash", ["-", "-", "-", "-"]),
    ("default", "version_decode_truncated", ["weight-comb", "harmonic", "ascend-cliff,ascend-plateau,benign,bigroot,cliff,comb-scatter,concurrent-pair,dense,freeze-parade,freeze-pos,harmonic,hugeleaf,jump-pair,mirror-narrow,mirror-wide,nested-full,nested-wide,promo-rearm,pure-comb,reveal-comb,reveal-hifloor,staircase,tooth-tail,weight-comb", "staircase"]),
    ("default", "version_decode_trailing", ["hugeleaf", "dense", "promo-rearm", "staircase"]),
    ("default", "version_decode_noncanon", ["hugeleaf", "harmonic", "freeze-parade,reveal-hifloor", "staircase"]),
    ("default", "version_parse_trailing", ["mirror-narrow", "staircase", "jump-pair", "staircase"]),
    ("default", "version_parse_noncanon", ["tooth-tail", "staircase", "jump-pair", "staircase"]),
    ("default", "party_decode_truncated", ["id-pair", "-", "ascend-cliff,ascend-plateau,benign,comb-scatter,id-pair,mirror-narrow,mirror-wide,nested-full,nested-wide,pure-comb,reveal-comb,reveal-hifloor,staircase", "-"]),
    ("default", "party_decode_trailing", ["id-pair", "-", "id-pair", "-"]),
    ("default", "party_decode_noncanon", ["id-pair", "-", "benign,comb-scatter,staircase", "-"]),
    ("default", "party_parse_trailing", ["id-pair", "-", "-", "-"]),
    ("default", "party_parse_noncanon", ["id-pair", "-", "-", "-"]),
    ("default", "clock_decode_truncated", ["id-pair", "harmonic", "id-pair", "staircase"]),
    ("default", "clock_decode_trailing", ["id-pair", "dense", "id-pair", "staircase"]),
    ("default", "clock_parse_trailing", ["dense,harmonic", "concurrent-pair", "jump-pair", "hugeleaf"]),
    ("default", "party_join_overlap", ["id-pair", "-", "mirror-narrow", "-"]),
    ("default", "clock_join_overlap", ["id-pair", "-", "id-pair", "-"]),
    ("default", "clock_sync_overlap", ["id-pair", "-", "id-pair", "-"]),
    ("default", "party_join_all_overlap", ["nested-full", "-", "benign", "-"]),
    ("default", "party_without_none", ["id-pair", "-", "id-pair", "-"]),
    ("acceptance", "version_decode", ["hugeleaf", "dense", "freeze-pos", "staircase"]),
    ("acceptance", "version_encode", ["promo-rearm", "-", "-", "-"]),
    ("acceptance", "version_cmp", ["hugeleaf", "staircase", "freeze-pos", "staircase"]),
    ("acceptance", "version_eq", ["-", "-", "-", "-"]),
    ("acceptance", "version_concurrent", ["hugeleaf", "staircase", "freeze-pos", "staircase"]),
    ("acceptance", "version_join", ["hugeleaf", "dense", "mirror-wide", "staircase"]),
    ("acceptance", "version_join_assign", ["hugeleaf", "dense", "mirror-wide", "staircase"]),
    ("acceptance", "version_meet", ["hugeleaf", "dense", "weight-comb", "staircase"]),
    ("acceptance", "version_meet_assign", ["hugeleaf", "dense", "weight-comb", "staircase"]),
    ("acceptance", "version_tick", ["ascend-cliff", "mirror-narrow", "hugeleaf", "mirror-narrow"]),
    ("acceptance", "version_ticks", ["ascend-cliff", "comb-scatter", "comb-scatter", "mirror-narrow"]),
    ("acceptance", "version_tick_adv_party", ["id-pair", "id-pair", "id-pair", "comb-scatter"]),
    ("acceptance", "version_rank", ["hugeleaf", "staircase", "hugeleaf", "staircase"]),
    ("acceptance", "rank_pair_ops", ["hugeleaf", "concurrent-pair", "-", "-"]),
    ("acceptance", "rank_sum", ["bigroot", "hugeleaf", "-", "freeze-pos"]),
    ("acceptance", "version_distance", ["pure-comb", "staircase", "hugeleaf", "staircase"]),
    ("acceptance", "version_lag", ["pure-comb", "staircase", "hugeleaf", "staircase"]),
    ("acceptance", "version_min_ticks", ["ascend-cliff", "staircase", "freeze-pos", "staircase"]),
    ("acceptance", "version_join_all", ["weave", "stagger", "benign", "stagger"]),
    ("acceptance", "version_meet_all", ["weave", "stagger", "stagger", "stagger"]),
    ("acceptance", "own_version_to_version", ["hugeleaf", "dense", "promo-rearm", "benign"]),
    ("acceptance", "own_version_cmp", ["hugeleaf", "dense", "freeze-pos", "benign"]),
    ("acceptance", "own_version_pair_cmp", ["hugeleaf", "dense", "jump-pair", "dense"]),
    ("acceptance", "version_display", ["mirror-narrow", "mirror-wide", "jump-pair", "-"]),
    ("acceptance", "version_from_str", ["mirror-narrow", "staircase", "jump-pair", "staircase"]),
    ("acceptance", "version_hash", ["-", "-", "-", "-"]),
    ("acceptance", "causally_contains", ["hugeleaf", "staircase", "freeze-pos", "staircase"]),
    ("acceptance", "party_decode", ["id-pair", "-", "id-pair", "-"]),
    ("acceptance", "party_encode", ["id-pair", "-", "-", "-"]),
    ("acceptance", "party_fork", ["id-pair", "-", "mirror-narrow,nested-full", "-"]),
    ("acceptance", "party_join", ["id-pair", "-", "benign", "-"]),
    ("acceptance", "party_join_all", ["weave", "-", "weave", "-"]),
    ("acceptance", "party_covers", ["-", "-", "id-pair", "-"]),
    ("acceptance", "party_disjoint", ["-", "-", "id-pair", "-"]),
    ("acceptance", "party_without", ["id-pair", "-", "id-pair", "-"]),
    ("acceptance", "party_display", ["pure-comb", "-", "mirror-narrow,nested-full", "-"]),
    ("acceptance", "party_from_str", ["comb-scatter", "-", "mirror-narrow,nested-full", "-"]),
    ("acceptance", "party_hash", ["-", "-", "-", "-"]),
    ("acceptance", "clock_decode", ["id-pair", "dense", "id-pair", "staircase"]),
    ("acceptance", "clock_encode", ["id-pair", "-", "-", "-"]),
    ("acceptance", "clock_tick", ["ascend-cliff", "mirror-narrow", "hugeleaf", "mirror-narrow"]),
    ("acceptance", "clock_fork", ["id-pair", "-", "mirror-narrow,nested-full", "-"]),
    ("acceptance", "clock_join", ["hugeleaf", "dense", "id-pair", "concurrent-pair"]),
    ("acceptance", "clock_sync", ["hugeleaf", "dense", "mirror-narrow", "concurrent-pair"]),
    ("acceptance", "clock_recv", ["id-pair", "id-pair", "hugeleaf", "staircase"]),
    ("acceptance", "clock_own_version_to_version", ["hugeleaf", "dense", "promo-rearm", "benign"]),
    ("acceptance", "clock_display", ["tooth-tail", "mirror-wide", "jump-pair", "-"]),
    ("acceptance", "clock_from_str", ["dense,harmonic", "concurrent-pair", "jump-pair", "concurrent-pair"]),
    ("acceptance", "clock_hash", ["-", "-", "-", "-"]),
    ("acceptance", "version_decode_truncated", ["bigroot", "harmonic", "ascend-cliff,ascend-plateau,benign,bigroot,cliff,comb-scatter,concurrent-pair,dense,freeze-parade,freeze-pos,harmonic,hugeleaf,jump-pair,mirror-narrow,mirror-wide,nested-full,nested-wide,promo-rearm,pure-comb,reveal-comb,reveal-hifloor,staircase,tooth-tail,weight-comb", "staircase"]),
    ("acceptance", "version_decode_trailing", ["hugeleaf", "dense", "promo-rearm", "staircase"]),
    ("acceptance", "version_decode_noncanon", ["hugeleaf", "harmonic", "reveal-hifloor", "staircase"]),
    ("acceptance", "version_parse_trailing", ["mirror-narrow", "staircase", "jump-pair", "staircase"]),
    ("acceptance", "version_parse_noncanon", ["tooth-tail", "staircase", "jump-pair", "staircase"]),
    ("acceptance", "party_decode_truncated", ["id-pair", "-", "ascend-cliff,ascend-plateau,benign,comb-scatter,id-pair,mirror-narrow,mirror-wide,nested-full,nested-wide,pure-comb,reveal-comb,reveal-hifloor,staircase", "-"]),
    ("acceptance", "party_decode_trailing", ["id-pair", "-", "id-pair", "-"]),
    ("acceptance", "party_decode_noncanon", ["id-pair", "-", "comb-scatter,staircase", "-"]),
    ("acceptance", "party_parse_trailing", ["comb-scatter", "-", "-", "-"]),
    ("acceptance", "party_parse_noncanon", ["comb-scatter", "-", "-", "-"]),
    ("acceptance", "clock_decode_truncated", ["id-pair", "harmonic", "id-pair", "staircase"]),
    ("acceptance", "clock_decode_trailing", ["id-pair", "dense", "id-pair", "staircase"]),
    ("acceptance", "clock_parse_trailing", ["dense,harmonic", "concurrent-pair", "jump-pair", "hugeleaf"]),
    ("acceptance", "party_join_overlap", ["id-pair", "-", "mirror-narrow", "-"]),
    ("acceptance", "clock_join_overlap", ["id-pair", "-", "id-pair", "-"]),
    ("acceptance", "clock_sync_overlap", ["id-pair", "-", "id-pair", "-"]),
    ("acceptance", "party_join_all_overlap", ["nested-full", "-", "mirror-narrow", "-"]),
    ("acceptance", "party_without_none", ["id-pair", "-", "id-pair", "-"]),
];

/// Run the board at both scales of record and entry-compare the live
/// worst-case fold against the committed ranking pin (the
/// `WORST_RANKINGS` table beside the fold), writing one drift line per
/// disagreement to `out`.
///
/// Returns `Ok(true)` when the pin matches exactly. Detects both
/// directions of rot: a live row missing from the pin and a pinned row
/// the board no longer produces.
///
/// # Panics
///
/// Panics if a mapped counter is not compiled into this run (the pin is
/// stated over all four currencies, so the check requires the
/// `limb-meter` and `scan-meter` features), or if the pin table itself is
/// malformed (duplicate or unknown scale/operation keys).
pub fn check_worst_map(heap: &HeapMeter, out: &mut dyn Write) -> io::Result<bool> {
    let mut seen = BTreeSet::new();
    for (scale, op, _) in WORST_RANKINGS {
        assert!(
            WORST_MAP_SCALES.iter().any(|(label, _)| label == scale),
            "worst-case pin: unknown scale label {scale:?} on {op}"
        );
        assert!(
            seen.insert((*scale, *op)),
            "worst-case pin: duplicate entry for {op} at the {scale} scale"
        );
    }
    let mut clean = true;
    for (label, scale) in WORST_MAP_SCALES {
        let results = sweep(scale, heap);
        let map = fold(&results);
        let mut live_ops = BTreeSet::new();
        for op in &map {
            live_ops.insert(op.op);
            let pinned = WORST_RANKINGS
                .iter()
                .find(|(s, o, _)| *s == label && *o == op.op);
            for (i, c) in op.per_currency.iter().enumerate() {
                assert!(
                    !c.off,
                    "worst-case pin: the {} counter is not compiled into this run: the check \
                     needs the limb-meter and scan-meter features",
                    c.currency.label()
                );
                let live = if c.worst.is_empty() {
                    "-".to_string()
                } else {
                    c.worst
                        .iter()
                        .map(|e| e.family)
                        .collect::<Vec<_>>()
                        .join(",")
                };
                let old = pinned.map(|(_, _, columns)| columns[i]);
                if old != Some(live.as_str()) {
                    clean = false;
                    writeln!(
                        out,
                        "worst-case pin drift: {op} x {cur} at the {label} scale: pinned worst \
                         {old}, live worst {live}: a ranking flip is news: either a family \
                         legitimately overtook (re-pin deliberately with a movement annotation) \
                         or a code change made some shape relatively worse (investigate first)",
                        op = op.op,
                        cur = c.currency.label(),
                        old = old.unwrap_or("(no entry)"),
                    )?;
                }
            }
        }
        for (s, o, _) in WORST_RANKINGS {
            if *s == label && !live_ops.contains(o) {
                clean = false;
                writeln!(
                    out,
                    "worst-case pin drift: the pin names {o} at the {label} scale but the board \
                     produces no such operation row: drop or rename the stale entry"
                )?;
            }
        }
    }
    if clean {
        writeln!(
            out,
            "worst-case pin: clean ({} pinned rows verified at {} scales)",
            WORST_RANKINGS.len(),
            WORST_MAP_SCALES.len()
        )?;
    }
    Ok(clean)
}
