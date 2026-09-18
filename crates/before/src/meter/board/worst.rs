//! Ranks the board's input families by measured cost for each operation.
//!
//! The map folds the board's normalized heap, scan, and touch readings. It does
//! not claim that the measured roster contains every possible worst case;
//! complexity arguments and focused tests establish that. Stack segments are
//! excluded because they use an absolute ceiling rather than a density.
//!
//! [`WORST_RANKINGS`] pins the family with the largest reading in each cell.
//! Exact ties retain every family in name order. The renderer also flags a
//! runner-up within [`NEAR_TIE_RATIO`] so a small constant difference is not
//! mistaken for a distinct complexity class.

use std::collections::BTreeSet;
use std::io::{self, Write};

use super::ceilings::{HEAP_FLAT_ALLOWANCE_BYTES, LADDER_TOP_SCALE};
use super::currency::Currency;
use super::judge::CellResult;

/// The measurement ladder's two sampling scales, at each of which the
/// worst-case map is rendered and pinned: the board's seconds-scale base and
/// the ladder top ([`LADDER_TOP_SCALE`], which owns the ×4 calibration
/// argument).
///
/// The map's claim is scale-qualified because a ranking is: a shape's
/// normalized constant carries its intercept at the default scale, and the
/// acceptance scale is where the known onset effects (segment growth,
/// doubling-chain steps) have fired.
pub const WORST_MAP_SCALES: [(&str, f64); 2] = [("default", 1.0), ("acceptance", LADDER_TOP_SCALE)];

/// A runner-up within this ratio of the worst reading is flagged `~near-tie` in
/// the rendered table.
///
/// The band is the constant-factor headroom the board's calibrated ceilings
/// grant a single reading (a ratified ceiling is the worst reading ×1.25): two
/// families inside it are one reading apart, not two classes, so their rank
/// order is a fact about the chosen scale's constants, not about the shapes —
/// the flag stops a reader from over-reading rank 1 vs rank 2. The flag never
/// enters the pin: the pin records the exact deterministic argmax, and a flip
/// inside the band is still news worth a look.
pub const NEAR_TIE_RATIO: f64 = 1.25;

/// The normalized measurements ranked by the map, in display order.
const MAP_CURRENCIES: [Currency; 3] = [Currency::Heap, Currency::Scan, Currency::Touch];

/// One family's normalized reading in a map row.
pub(super) struct Entry {
    /// The family name.
    pub(super) family: &'static str,
    /// The board's normalized constant of record for the cell.
    pub(super) value: f64,
    /// Whether the cell is judged under a declared model for this
    /// currency (rendered `*`: intended and modeled).
    pub(super) modeled: bool,
}

/// The highest reading and runner-up for one operation and measurement.
pub(super) struct CurrencyWorst {
    /// The currency this column ranks.
    pub(super) currency: Currency,
    /// True when the counter is not compiled into this run (the
    /// feature-gated columns without `touch-meter`/`scan-meter`).
    pub(super) off: bool,
    /// Every family at the maximum reading, sorted by name; empty when no
    /// committed shape drives the currency on this row.
    pub(super) worst: Vec<Entry>,
    /// The best family strictly below the maximum (name-order first on an
    /// exact tie); `None` when every other shape reads zero.
    pub(super) runner_up: Option<Entry>,
}

/// One operation's ranked measurements.
pub(super) struct OpWorst {
    /// The board row's operation name.
    pub(super) op: &'static str,
    /// One argmax per [`MAP_CURRENCIES`] column, in that order.
    pub(super) per_currency: Vec<CurrencyWorst>,
}

/// Find the largest nonzero reading and the largest reading below it.
///
/// Exact ties are sorted by family name. An all-zero row has no winner.
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

/// Whether this cell overrides `currency`'s default model.
fn modeled(r: &CellResult, currency: Currency) -> bool {
    r.s2.models.get(currency).is_some()
}

/// Fold one sweep's cell results into the worst-case map, in board row order.
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

/// A reading rendered at a precision that keeps small constants legible without
/// decorating large ones.
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
        unit = "/B",
    )
}

/// Render one operation × currency row of the map.
pub(super) fn row(out: &mut dyn Write, op: &str, c: &CurrencyWorst) -> io::Result<()> {
    let lead = format!("{op:<28} {cur:<5}", cur = c.currency.label());
    if c.off {
        return writeln!(
            out,
            "{lead}  worst off  (counter not compiled into this run: touch-meter/scan-meter)"
        );
    }
    if c.worst.is_empty() {
        return writeln!(
            out,
            "{lead}  worst -  (no committed shape drives this currency on this row)"
        );
    }
    let unit = "/B";
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

/// Fold one whole sweep's judged cells and render the worst-case map table to
/// `out`, one row per operation × mapped currency, in board row order.
///
/// `results` must be a whole board's cells in board row order: a shard merge's
/// reconstruction of one sweep. `label` names the scale in the header (the
/// sampling scales are [`WORST_MAP_SCALES`]; a smoke run may pass its own).
/// The fold is a pure consumer of the board's own judged cells: no reading,
/// family, or ceiling is recomputed here.
pub(super) fn render_map(
    label: &str,
    scale: f64,
    results: &[CellResult],
    out: &mut dyn Write,
) -> io::Result<()> {
    let map = fold(results);
    writeln!(
        out,
        "worst-case map at the {label} scale (x{scale}): the worst instrumented shape per \
         operation x currency"
    )?;
    writeln!(
        out,
        "  worst instrumented shape: the maximum over the committed family roster - the claim \
         that this is the true worst case is carried by the rustdoc complexity sections and \
         the asymptotics liveness pins, not by this table."
    )?;
    writeln!(
        out,
        "  reading: the board's normalized constant at the cell's larger sample: heap bytes net \
         of the {HEAP_FLAT_ALLOWANCE_BYTES} B flat allowance, scan bits, and touches per constant \
         unit. A unit is ordinarily one denominator byte (encoded input, or total I/O where \
         required output can dominate); modeled rows use their stated work units. Readings rank \
         cost density, so a row may mix units exactly where the board does."
    )?;
    writeln!(
        out,
        "  margin: worst/runner-up, a ratio: unit-free across a row's denominators and legible \
         across the constants' magnitudes. ~near-tie flags margins under x{NEAR_TIE_RATIO}: the \
         constant-factor band the board's calibrated ceilings treat as one reading, so rank 1 \
         vs rank 2 inside it is one reading apart, not two classes."
    )?;
    writeln!(
        out,
        "  *: the reading sits under a cell-specific resource model (the board's model[...] rows): \
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

/// Expected winners as `(scale, operation, [heap, scan, touch])`.
///
/// Tied family names are comma-separated in name order; `-` means every family
/// read zero. [`check_worst_map`](super::shard::check_worst_map) compares this
/// table with fresh release-profile measurements at [`WORST_MAP_SCALES`].
pub(super) const WORST_RANKINGS: &[(&str, &str, [&str; 3])] = &[
    ("default", "version_decode", ["hugeleaf", "freeze-pos", "staircase"]),
    ("default", "version_encode", ["promo-rearm", "-", "-"]),
    ("default", "version_cmp", ["hugeleaf", "promo-rearm", "staircase"]),
    ("default", "version_eq", ["-", "-", "-"]),
    ("default", "version_concurrent", ["hugeleaf", "promo-rearm", "staircase"]),
    ("default", "version_join", ["plateau-puncture", "bigroot", "staircase"]),
    ("default", "version_join_assign", ["plateau-puncture", "bigroot", "staircase"]),
    ("default", "version_meet", ["hugeleaf", "weight-comb", "staircase"]),
    ("default", "version_meet_assign", ["hugeleaf", "weight-comb", "staircase"]),
    ("default", "version_span", ["jump-pair", "jump-pair", "concurrent-pair"]),
    ("default", "span_encode", ["promo-rearm", "-", "-"]),
    ("default", "span_decode", ["hugeleaf", "weight-comb", "staircase"]),
    ("default", "version_tick", ["wide-arming", "memo-oscillating", "mirror-narrow"]),
    ("default", "version_ticks", ["wide-arming", "memo-oscillating", "reveal-hifloor"]),
    ("default", "version_tick_adv_party", ["id-pair", "id-pair", "-"]),
    ("default", "version_rank", ["wide-arming", "freeze-pos", "harmonic"]),
    ("default", "rank_pair_ops", ["hugeleaf", "-", "-"]),
    ("default", "rank_sum", ["plateau-puncture", "-", "freeze-pos"]),
    ("default", "rank_encode", ["hugeleaf", "-", "-"]),
    ("default", "rank_decode", ["plateau-puncture", "-", "-"]),
    ("default", "version_distance", ["wide-arming", "promo-rearm", "harmonic"]),
    ("default", "version_lag", ["wide-arming", "promo-rearm", "harmonic"]),
    ("default", "ranked_cmp", ["wide-arming", "promo-rearm", "harmonic"]),
    ("default", "ranked_encode", ["wide-arming", "freeze-pos", "harmonic"]),
    ("default", "ranked_encode_rank", ["wide-arming", "freeze-pos", "harmonic"]),
    ("default", "ranked_decode", ["wide-arming", "memo-oscillating", "staircase"]),
    ("default", "version_min_ticks", ["ascend-cliff", "freeze-pos", "staircase"]),
    ("default", "version_join_all", ["-", "stagger", "stagger"]),
    ("default", "version_meet_all", ["-", "weave", "stagger"]),
    ("default", "version_span_all", ["stagger", "stagger", "stagger"]),
    ("default", "own_version_to_version", ["hugeleaf", "comb-scatter", "lone-freeze"]),
    ("default", "own_version_cmp", ["hugeleaf", "promo-rearm", "lone-freeze"]),
    ("default", "own_version_pair_cmp", ["hugeleaf", "jump-pair", "dense"]),
    ("default", "version_hash", ["-", "-", "-"]),
    ("default", "causally_contains", ["hugeleaf", "dense-suffix", "ascend-plateau"]),
    ("default", "span_place", ["hugeleaf", "promo-rearm", "staircase"]),
    ("default", "span_dominance", ["hugeleaf", "promo-rearm", "staircase"]),
    ("default", "span_precedence", ["hugeleaf", "promo-rearm", "staircase"]),
    ("default", "span_contains", ["hugeleaf", "promo-rearm", "staircase"]),
    ("default", "query_contains", ["hugeleaf", "promo-rearm", "staircase"]),
    ("default", "query_coverage", ["hugeleaf", "hugeleaf", "staircase"]),
    ("default", "query_contains_many", ["scatter", "scatter", "scatter"]),
    ("default", "query_coverage_many", ["scatter", "benign", "weave"]),
    ("default", "query_conjoin_many", ["scatter", "scatter", "scatter"]),
    ("default", "query_clone_many", ["scatter", "-", "-"]),
    ("default", "party_decode", ["id-pair", "id-pair", "-"]),
    ("default", "party_encode", ["-", "-", "-"]),
    ("default", "party_fork", ["id-pair", "mirror-narrow,nested-full", "-"]),
    ("default", "party_forks", ["id-pair", "id-pair", "-"]),
    ("default", "party_forks_full", ["scatter", "weave", "-"]),
    (
        "default",
        "party_split_array",
        ["id-pair", "mirror-narrow,nested-full", "-"],
    ),
    ("default", "party_join", ["id-pair", "benign", "-"]),
    ("default", "party_join_all", ["-", "stagger", "-"]),
    ("default", "party_covers", ["-", "id-pair", "-"]),
    ("default", "party_disjoint", ["-", "id-pair", "-"]),
    ("default", "party_without", ["id-pair", "id-pair", "-"]),
    ("default", "party_hash", ["-", "-", "-"]),
    ("default", "clock_decode", ["id-pair", "promo-rearm", "lone-freeze"]),
    ("default", "clock_encode", ["id-pair", "-", "-"]),
    ("default", "clock_tick", ["id-pair", "memo-oscillating", "mirror-narrow"]),
    ("default", "clock_fork", ["id-pair", "mirror-narrow,nested-full", "-"]),
    (
        "default",
        "clock_forks",
        ["dominated-undercut", "id-pair", "-"],
    ),
    ("default", "clock_forks_full", ["scatter", "weave", "-"]),
    (
        "default",
        "clock_split_array",
        ["id-pair", "mirror-narrow,nested-full", "-"],
    ),
    ("default", "clock_join", ["plateau-puncture", "bigroot", "lone-freeze"]),
    ("default", "clock_sync", ["plateau-puncture", "bigroot", "lone-freeze"]),
    ("default", "clock_sync_all", ["stagger", "stagger", "stagger"]),
    ("default", "clock_recv", ["id-pair", "hugeleaf", "lone-freeze"]),
    ("default", "clock_own_version_to_version", ["id-pair", "comb-scatter", "staircase"]),
    ("default", "clock_hash", ["-", "-", "-"]),
    ("default", "version_decode_truncated", ["wide-arming", "ascend-cliff,ascend-plateau,benign,bigroot,cliff,comb-scatter,concurrent-pair,dense,dense-suffix,descending-raises,dominated-undercut,freeze-parade,freeze-pos,harmonic,hugeleaf,jump-pair,lone-freeze,memo-chain,memo-churn,memo-comb,memo-fanout,memo-oscillating,mirror-narrow,mirror-wide,nested-full,nested-wide,plateau-puncture,promo-rearm,pure-comb,reveal-comb,reveal-hifloor,staircase,tooth-tail,weight-comb,wide-arming", "staircase"]),
    ("default", "version_decode_trailing", ["hugeleaf", "promo-rearm", "staircase"]),
    ("default", "version_decode_noncanon", ["hugeleaf", "promo-rearm", "staircase"]),
    ("default", "span_decode_truncated", ["wide-arming", "jump-pair", "staircase"]),
    ("default", "span_decode_trailing", ["hugeleaf", "weight-comb", "staircase"]),
    ("default", "span_decode_crossed", ["hugeleaf", "hugeleaf", "ascend-plateau"]),
    ("default", "party_decode_truncated", ["id-pair", "ascend-cliff,ascend-plateau,benign,comb-scatter,descending-raises,dominated-undercut,id-pair,memo-chain,memo-churn,memo-comb,memo-fanout,memo-oscillating,mirror-narrow,mirror-wide,nested-full,nested-wide,pure-comb,reveal-comb,reveal-hifloor,staircase", "-"]),
    ("default", "party_decode_trailing", ["id-pair", "id-pair", "-"]),
    ("default", "party_decode_noncanon", ["id-pair", "id-pair", "-"]),
    ("default", "clock_decode_truncated", ["id-pair", "promo-rearm", "lone-freeze"]),
    ("default", "clock_decode_trailing", ["id-pair", "promo-rearm", "lone-freeze"]),
    ("default", "party_join_overlap", ["id-pair", "mirror-narrow", "-"]),
    ("default", "clock_join_overlap", ["id-pair", "id-pair", "-"]),
    ("default", "clock_sync_overlap", ["id-pair", "id-pair", "-"]),
    ("default", "party_without_none", ["id-pair", "id-pair", "-"]),
    ("acceptance", "version_decode", ["hugeleaf", "memo-oscillating", "staircase"]),
    ("acceptance", "version_encode", ["memo-oscillating", "-", "-"]),
    ("acceptance", "version_cmp", ["hugeleaf", "memo-oscillating", "staircase"]),
    ("acceptance", "version_eq", ["-", "-", "-"]),
    ("acceptance", "version_concurrent", ["hugeleaf", "memo-oscillating", "staircase"]),
    ("acceptance", "version_join", ["bigroot", "bigroot", "staircase"]),
    ("acceptance", "version_join_assign", ["bigroot", "bigroot", "staircase"]),
    ("acceptance", "version_meet", ["hugeleaf", "weight-comb", "staircase"]),
    ("acceptance", "version_meet_assign", ["hugeleaf", "weight-comb", "staircase"]),
    ("acceptance", "version_span", ["jump-pair", "jump-pair", "concurrent-pair"]),
    ("acceptance", "span_encode", ["memo-oscillating", "-", "-"]),
    ("acceptance", "span_decode", ["hugeleaf", "weight-comb", "staircase"]),
    ("acceptance", "version_tick", ["memo-comb", "memo-oscillating", "mirror-narrow"]),
    ("acceptance", "version_ticks", ["memo-comb", "memo-oscillating", "reveal-hifloor"]),
    ("acceptance", "version_tick_adv_party", ["id-pair", "id-pair", "-"]),
    ("acceptance", "version_rank", ["wide-arming", "memo-oscillating", "harmonic"]),
    ("acceptance", "rank_pair_ops", ["hugeleaf", "-", "-"]),
    ("acceptance", "rank_sum", ["bigroot", "-", "freeze-pos"]),
    ("acceptance", "rank_encode", ["hugeleaf", "-", "-"]),
    ("acceptance", "rank_decode", ["bigroot", "-", "-"]),
    ("acceptance", "version_distance", ["wide-arming", "memo-oscillating", "harmonic"]),
    ("acceptance", "version_lag", ["wide-arming", "memo-oscillating", "harmonic"]),
    ("acceptance", "ranked_cmp", ["wide-arming", "memo-oscillating", "harmonic"]),
    ("acceptance", "ranked_encode", ["wide-arming", "memo-oscillating", "harmonic"]),
    ("acceptance", "ranked_encode_rank", ["wide-arming", "memo-oscillating", "harmonic"]),
    ("acceptance", "ranked_decode", ["wide-arming", "memo-oscillating", "staircase"]),
    ("acceptance", "version_min_ticks", ["ascend-cliff", "memo-oscillating", "staircase"]),
    ("acceptance", "version_join_all", ["weave", "stagger", "stagger"]),
    ("acceptance", "version_meet_all", ["weave", "weave", "stagger"]),
    ("acceptance", "version_span_all", ["weave", "stagger", "stagger"]),
    ("acceptance", "own_version_to_version", ["hugeleaf", "comb-scatter", "lone-freeze"]),
    ("acceptance", "own_version_cmp", ["hugeleaf", "memo-oscillating", "lone-freeze"]),
    ("acceptance", "own_version_pair_cmp", ["hugeleaf", "memo-oscillating", "dense"]),
    ("acceptance", "version_hash", ["-", "-", "-"]),
    ("acceptance", "causally_contains", ["hugeleaf", "dense-suffix", "ascend-plateau"]),
    ("acceptance", "span_place", ["hugeleaf", "memo-oscillating", "staircase"]),
    ("acceptance", "span_dominance", ["hugeleaf", "memo-oscillating", "staircase"]),
    ("acceptance", "span_precedence", ["hugeleaf", "memo-oscillating", "staircase"]),
    ("acceptance", "span_contains", ["hugeleaf", "memo-oscillating", "staircase"]),
    ("acceptance", "query_contains", ["hugeleaf", "memo-oscillating", "staircase"]),
    ("acceptance", "query_coverage", ["hugeleaf", "hugeleaf", "staircase"]),
    ("acceptance", "query_contains_many", ["scatter", "scatter", "weave"]),
    ("acceptance", "query_coverage_many", ["scatter", "benign", "weave"]),
    ("acceptance", "query_conjoin_many", ["scatter", "benign", "scatter"]),
    ("acceptance", "query_clone_many", ["scatter", "-", "-"]),
    ("acceptance", "party_decode", ["id-pair", "id-pair", "-"]),
    ("acceptance", "party_encode", ["id-pair", "-", "-"]),
    ("acceptance", "party_fork", ["id-pair", "mirror-narrow,nested-full", "-"]),
    ("acceptance", "party_forks", ["id-pair", "id-pair", "-"]),
    ("acceptance", "party_forks_full", ["scatter", "weave", "-"]),
    (
        "acceptance",
        "party_split_array",
        ["id-pair", "mirror-narrow,nested-full", "-"],
    ),
    ("acceptance", "party_join", ["id-pair", "benign", "-"]),
    ("acceptance", "party_join_all", ["weave", "stagger", "-"]),
    ("acceptance", "party_covers", ["-", "id-pair", "-"]),
    ("acceptance", "party_disjoint", ["-", "id-pair", "-"]),
    (
        "acceptance",
        "party_without",
        ["ascend-cliff,ascend-plateau", "id-pair", "-"],
    ),
    ("acceptance", "party_hash", ["-", "-", "-"]),
    ("acceptance", "clock_decode", ["id-pair", "memo-oscillating", "lone-freeze"]),
    ("acceptance", "clock_encode", ["id-pair", "-", "-"]),
    ("acceptance", "clock_tick", ["id-pair", "memo-oscillating", "mirror-narrow"]),
    ("acceptance", "clock_fork", ["id-pair", "mirror-narrow,nested-full", "-"]),
    (
        "acceptance",
        "clock_forks",
        ["descending-raises", "id-pair", "-"],
    ),
    ("acceptance", "clock_forks_full", ["scatter", "weave", "-"]),
    (
        "acceptance",
        "clock_split_array",
        ["id-pair", "mirror-narrow,nested-full", "-"],
    ),
    ("acceptance", "clock_join", ["bigroot", "bigroot", "lone-freeze"]),
    ("acceptance", "clock_sync", ["bigroot", "bigroot", "lone-freeze"]),
    ("acceptance", "clock_sync_all", ["benign", "stagger", "stagger"]),
    ("acceptance", "clock_recv", ["id-pair", "hugeleaf", "lone-freeze"]),
    ("acceptance", "clock_own_version_to_version", ["id-pair", "comb-scatter", "staircase"]),
    ("acceptance", "clock_hash", ["-", "-", "-"]),
    ("acceptance", "version_decode_truncated", ["wide-arming", "ascend-cliff,ascend-plateau,benign,bigroot,cliff,comb-scatter,concurrent-pair,dense,dense-suffix,descending-raises,dominated-undercut,freeze-parade,freeze-pos,harmonic,hugeleaf,jump-pair,lone-freeze,memo-chain,memo-churn,memo-comb,memo-fanout,memo-oscillating,mirror-narrow,mirror-wide,nested-full,nested-wide,plateau-puncture,promo-rearm,pure-comb,reveal-comb,reveal-hifloor,staircase,tooth-tail,weight-comb,wide-arming", "staircase"]),
    ("acceptance", "version_decode_trailing", ["hugeleaf", "memo-oscillating", "staircase"]),
    ("acceptance", "version_decode_noncanon", ["hugeleaf", "promo-rearm", "staircase"]),
    ("acceptance", "span_decode_truncated", ["wide-arming", "jump-pair", "staircase"]),
    ("acceptance", "span_decode_trailing", ["hugeleaf", "weight-comb", "staircase"]),
    ("acceptance", "span_decode_crossed", ["hugeleaf", "hugeleaf", "ascend-plateau"]),
    ("acceptance", "party_decode_truncated", ["id-pair", "ascend-cliff,ascend-plateau,benign,comb-scatter,descending-raises,dominated-undercut,id-pair,memo-chain,memo-churn,memo-comb,memo-fanout,memo-oscillating,mirror-narrow,mirror-wide,nested-full,nested-wide,pure-comb,reveal-comb,reveal-hifloor,staircase", "-"]),
    ("acceptance", "party_decode_trailing", ["id-pair", "id-pair", "-"]),
    ("acceptance", "party_decode_noncanon", ["id-pair", "id-pair", "-"]),
    ("acceptance", "clock_decode_truncated", ["id-pair", "memo-oscillating", "lone-freeze"]),
    ("acceptance", "clock_decode_trailing", ["id-pair", "memo-oscillating", "lone-freeze"]),
    ("acceptance", "party_join_overlap", ["id-pair", "mirror-narrow", "-"]),
    ("acceptance", "clock_join_overlap", ["id-pair", "id-pair", "-"]),
    ("acceptance", "clock_sync_overlap", ["id-pair", "id-pair", "-"]),
    ("acceptance", "party_without_none", ["id-pair", "id-pair", "-"]),
];

/// Entry-compare the live worst-case fold against the committed ranking pin
/// (the `WORST_RANKINGS` table beside the fold), writing one drift line per
/// disagreement to `out`.
///
/// `sweeps` yields one whole board's judged cells per sampling scale — a shard
/// merge under [`check_worst_map`](super::shard::check_worst_map).
///
/// Returns `Ok(true)` when the pin matches exactly. Detects both directions of
/// rot: a live row missing from the pin and a pinned row the board no longer
/// produces.
///
/// # Panics
///
/// Panics if a mapped counter is not compiled into this run (the pin is stated
/// over all three mapped currencies, so the check requires the `touch-meter` and
/// `scan-meter` features), or if the pin table itself is malformed (duplicate
/// or unknown scale/operation keys).
pub(super) fn check_with(
    sweeps: &mut dyn FnMut(f64) -> io::Result<Vec<CellResult>>,
    out: &mut dyn Write,
) -> io::Result<bool> {
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
        let results = sweeps(scale)?;
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
                     needs the touch-meter and scan-meter features",
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
            "worst-case pin: clean ({} pinned rows verified at {} sampling scales)",
            WORST_RANKINGS.len(),
            WORST_MAP_SCALES.len()
        )?;
    }
    Ok(clean)
}
