//! Ranks the board's input families by measured cost for each operation.
//!
//! The map folds the board's normalized heap, scan, and touch readings. It does
//! not claim that the measured roster contains every possible worst case;
//! complexity arguments and focused tests establish that.
//!
//! [`WORST_RANKINGS`] pins the family with the largest reading in each cell,
//! except in the cells it declares [`TARGET_DEPENDENT`], where it checks only
//! that some family drives the currency. Exact ties retain every family in
//! name order. The renderer also flags a runner-up within [`NEAR_TIE_RATIO`]
//! so a small constant difference is not mistaken for a distinct complexity
//! class.

use std::collections::BTreeSet;
use std::io::{self, Write};

use super::ceilings::LADDER_TOP_SCALE;
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
/// enters the pin: the pin records the exact deterministic argmax of every cell
/// it does not declare target-dependent, and a flip inside the band is still
/// news worth a look.
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
        "  reading: the larger raw cost density across the cell's two samples: heap bytes, \
         scan bits, and touches per constant unit. A unit is ordinarily one denominator byte \
         (input, or total I/O where \
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
        "  *: the reading uses a cell-specific resource model (the board's model[...] rows)."
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

/// Every party-bearing family, for exact ties at one scan per input bit.
const PARTY_STREAM_FAMILIES: &str = concat!(
    "ascend-cliff,ascend-plateau,benign,collapse-hole,comb-scatter,copy-hole,",
    "descending-raises,dominated-undercut,id-pair,memo-chain,memo-churn,",
    "memo-comb,memo-fanout,memo-oscillating,mirror-narrow,mirror-wide,",
    "nested-full,nested-wide,pure-comb,raise-hole,reveal-comb,reveal-hifloor,",
    "site-hole,staircase",
);

/// Every version-bearing family, for exact ties at one scan per input bit.
const VERSION_STREAM_FAMILIES: &str = concat!(
    "alt-spine,arming-train,ascend-cliff,ascend-plateau,benign,bigroot,",
    "cancelling-chain,cliff,cliff-fan,collapse-hole,comb-scatter,concurrent-pair,",
    "copy-hole,dense,dense-suffix,descending-raises,dominated-undercut,",
    "freeze-parade,freeze-pos,harmonic,hoisted-window,hugeleaf,jump-comb,",
    "jump-pair,latent-ladder,lone-freeze,mask-drift,masked-hole,memo-chain,",
    "memo-churn,memo-comb,memo-fanout,memo-oscillating,mirror-narrow,",
    "mirror-wide,nested-full,nested-wide,plateau-puncture,promo-rearm,",
    "propagate-seam,pure-comb,raise-hole,reveal-comb,reveal-hifloor,site-hole,",
    "staircase,tooth-tail,weight-comb,wide-arming,wide-tooth-comb",
);

/// The pinned worst set of a cell whose ranking depends on the build target.
///
/// Such a cell's readings are deterministic for each build target but differ
/// between targets, so no single family is its argmax on every target; the
/// [board's module docs](super#target-dependence) say when a reading depends
/// on the target. The pin checks such a cell for liveness only: any
/// live worst set matches, and a row on which every family reads zero is
/// drift. The cell's ceilings judge its readings on every target as usual.
///
/// Declare a cell target-dependent only when a target-conditional code path
/// that the measured operation reaches changes *which family reads worst*
/// between the 64-bit hosts the pin is taken and checked on, and name that
/// path in a `// Target-conditional path:` comment on the line above the
/// entry; a test fails any marked entry without one. A reading that merely
/// differs by target does not qualify, nor does a flip between near-equal
/// families with no such path behind it, however small its margin: that flip
/// is news (see [`NEAR_TIE_RATIO`]). The margin plays no part in the rule, so
/// a qualifying cell may itself be a near-tie on some target.
pub(super) const TARGET_DEPENDENT: &str = "(target-dependent)";

/// Expected winners as `(scale, operation, [heap, scan, touch])`.
///
/// Tied family names are comma-separated in name order; `-` means every family
/// read zero; [`TARGET_DEPENDENT`] means the winner depends on the build
/// target. [`check_worst_map`](super::shard::check_worst_map) compares this
/// table with fresh release-profile measurements at [`WORST_MAP_SCALES`].
#[rustfmt::skip]
pub(super) const WORST_RANKINGS: &[(&str, &str, [&str; 3])] = &[
    ("default", "version_decode", ["hugeleaf", "promo-rearm", "staircase"]),
    ("default", "version_encode", [VERSION_STREAM_FAMILIES, "-", "-"]),
    ("default", "version_display", [VERSION_STREAM_FAMILIES, "-", "-"]),
    ("default", "version_parse", ["mirror-narrow,nested-full", "promo-rearm", "staircase"]),
    ("default", "version_cmp", ["hugeleaf", "promo-rearm", "staircase"]),
    ("default", "version_eq", ["-", "-", "-"]),
    ("default", "version_concurrent", ["hugeleaf", "promo-rearm", "staircase"]),
    ("default", "version_join", ["mirror-narrow", "bigroot", "staircase"]),
    ("default", "version_join_assign", ["mirror-narrow", "bigroot", "staircase"]),
    ("default", "version_meet", ["mirror-narrow", "site-hole", "staircase"]),
    ("default", "version_meet_assign", ["mirror-narrow", "site-hole", "staircase"]),
    ("default", "version_span", ["concurrent-pair", "jump-pair", "concurrent-pair"]),
    ("default", "span_new", ["hugeleaf", "promo-rearm", "staircase"]),
    ("default", "span_union", ["meet-shade", "meet-shade", "stagger-size"]),
    ("default", "span_intersect", ["meet-shade", "benign", "stagger,stagger-arity,stagger-size"]),
    ("default", "span_join", ["meet-shade", "stagger", "stagger-size"]),
    ("default", "span_meet", ["meet-shade", "meet-shade", "stagger-size"]),
    ("default", "span_encode", ["weight-comb", "-", "-"]),
    ("default", "span_decode", ["hoisted-window", "site-hole", "staircase"]),
    ("default", "version_tick", ["memo-comb", "memo-oscillating", "mirror-narrow"]),
    ("default", "version_ticks", ["memo-comb", "memo-oscillating", "mirror-narrow"]),
    ("default", "version_tick_adv_party", ["copy-hole", "id-pair", "-"]),
    ("default", "version_rank", ["arming-train", "promo-rearm", "harmonic"]),
    ("default", "rank_clone", ["pure-comb", "-", "-"]),
    ("default", "rank_cmp", ["-", "-", "-"]),
    ("default", "rank_add", ["hugeleaf", "-", "-"]),
    ("default", "rank_checked_sub", ["cliff", "-", "-"]),
    ("default", "rank_sum", ["mirror-narrow,nested-full", "-", "freeze-pos"]),
    ("default", "rank_encode", ["wide-tooth-comb", "-", "-"]),
    ("default", "rank_decode", ["benign,concurrent-pair,descending-raises", "-", "-"]),
    ("default", "rank_display", ["cancelling-chain", "-", "-"]),
    ("default", "rank_display_precision", ["descending-raises", "-", "-"]),
    ("default", "rank_parse", ["descending-raises", "-", "-"]),
    ("default", "count_clone", ["cliff", "-", "-"]),
    ("default", "count_add", ["hugeleaf", "-", "-"]),
    ("default", "count_sum", ["hugeleaf", "-", "-"]),
    // Target-conditional path: `num-bigint`'s `to_radix_digits_le` takes its base from `FAST_DIV_WIDE`, true only on x86 and x86_64.
    ("default", "count_display", [TARGET_DEPENDENT, "-", "-"]),
    ("default", "count_parse", ["alt-spine,benign,dense,masked-hole,meet-shade,mirror-narrow,nested-full,scatter,stagger,stagger-arity,stagger-size,weave", "-", "-"]),
    ("default", "version_distance", ["arming-train", "promo-rearm", "harmonic"]),
    ("default", "version_lag", ["arming-train", "promo-rearm", "harmonic"]),
    ("default", "ranked_cmp", ["arming-train", "promo-rearm", "harmonic"]),
    ("default", "ranked_encode", ["arming-train", "promo-rearm", "harmonic"]),
    ("default", "ranked_encode_rank", ["arming-train", "promo-rearm", "harmonic"]),
    ("default", "ranked_decode", ["wide-arming", "cancelling-chain", "collapse-hole"]),
    ("default", "version_min_ticks", ["propagate-seam", "promo-rearm", "copy-hole"]),
    ("default", "version_join_all", ["meet-shade", "stagger", "stagger"]),
    ("default", "version_meet_all", ["meet-shade", "meet-shade", "meet-shade"]),
    ("default", "version_span_all", ["meet-shade", "stagger,stagger-arity,stagger-size", "stagger-arity"]),
    ("default", "span_union_all", ["meet-shade", "stagger-size", "scatter"]),
    ("default", "span_intersect_all", ["meet-shade", "weave", "scatter"]),
    ("default", "span_join_all", ["meet-shade", "stagger", "scatter"]),
    ("default", "span_meet_all", ["meet-shade", "meet-shade", "meet-shade"]),
    ("default", "own_version_to_version", ["hugeleaf", "comb-scatter", "collapse-hole,site-hole"]),
    ("default", "own_span_to_span", ["ascend-plateau", "comb-scatter", "copy-hole"]),
    ("default", "own_version_cmp", ["hugeleaf", "promo-rearm", "masked-hole"]),
    ("default", "own_version_pair_cmp", ["hugeleaf", "jump-pair", "raise-hole"]),
    ("default", "version_hash", ["-", "-", "-"]),
    ("default", "version_shape", ["hoisted-window", "promo-rearm", "-"]),
    ("default", "shape_combine_pair", ["hoisted-window", "promo-rearm", "-"]),
    ("default", "shape_combine_many", ["meet-shade", "weave", "-"]),
    ("default", "causally_contains", ["benign", "dense-suffix", "ascend-plateau"]),
    ("default", "query_single_hole", ["benign", "-", "-"]),
    ("default", "span_place", ["hugeleaf", "promo-rearm", "staircase"]),
    ("default", "span_dominance", ["hugeleaf", "promo-rearm", "staircase"]),
    ("default", "span_precedence", ["hugeleaf", "promo-rearm", "staircase"]),
    ("default", "span_contains", ["hugeleaf", "promo-rearm", "staircase"]),
    ("default", "query_contains", ["benign", "promo-rearm", "staircase"]),
    ("default", "query_coverage", ["benign", "propagate-seam", "staircase"]),
    ("default", "query_contains_many", ["meet-shade", "scatter", "meet-shade"]),
    ("default", "query_coverage_many", ["meet-shade", "weave", "weave"]),
    ("default", "query_contains_wide_up", ["scatter", "scatter", "scatter"]),
    ("default", "query_contains_wide_down", ["scatter", "scatter", "scatter"]),
    ("default", "query_coverage_wide_down", ["scatter", "scatter", "scatter"]),
    ("default", "query_coverage_wide_up", ["scatter", "scatter", "scatter"]),
    ("default", "query_conjoin_many", ["meet-shade", "meet-shade", "scatter"]),
    ("default", "query_clone_many", ["meet-shade", "-", "-"]),
    ("default", "party_decode", ["id-pair", "id-pair", "-"]),
    ("default", "party_encode", [PARTY_STREAM_FAMILIES, "-", "-"]),
    ("default", "party_display", [PARTY_STREAM_FAMILIES, "-", "-"]),
    ("default", "party_parse", ["copy-hole", "id-pair", "-"]),
    ("default", "party_fork", ["ascend-cliff,ascend-plateau", "id-pair", "-"]),
    // The fork plan stores its remainder in an integer as wide as the count,
    // and the board's count is as wide as the party, so the remainder adds
    // heap in proportion to every family's input. Copy-hole, the smallest
    // input at this scale, reads highest; the ascend families read within
    // `NEAR_TIE_RATIO` below it.
    ("default", "party_forks", ["copy-hole", "id-pair", "-"]),
    ("default", "party_forks_full", ["meet-shade", "meet-shade", "-"]),
    ("default", "party_split_array", ["raise-hole", "pure-comb", "-"]),
    ("default", "party_join", ["id-pair", "benign", "-"]),
    ("default", "party_join_all", ["meet-shade", "stagger-size", "-"]),
    ("default", "party_covers", ["-", "id-pair", "-"]),
    ("default", "party_disjoint", ["-", "id-pair", "-"]),
    ("default", "party_without", ["copy-hole", "id-pair", "-"]),
    ("default", "party_hash", ["-", "-", "-"]),
    ("default", "party_shape", ["copy-hole", "id-pair", "-"]),
    ("default", "clock_decode", ["id-pair", "promo-rearm", "copy-hole"]),
    ("default", "clock_encode", ["id-pair", "-", "-"]),
    ("default", "clock_tick", ["memo-comb", "memo-oscillating", "mirror-narrow"]),
    ("default", "clock_fork", ["id-pair", "id-pair", "-"]),
    ("default", "clock_forks", ["pure-comb", "id-pair", "-"]),
    ("default", "clock_forks_full", ["meet-shade", "meet-shade", "-"]),
    ("default", "clock_split_array", ["alt-spine,arming-train,benign,bigroot,cancelling-chain,cliff,cliff-fan,concurrent-pair,dense,dense-suffix,freeze-parade,freeze-pos,harmonic,hoisted-window,hugeleaf,jump-comb,jump-pair,latent-ladder,lone-freeze,mask-drift,masked-hole,plateau-puncture,promo-rearm,propagate-seam,tooth-tail,weight-comb,wide-arming,wide-tooth-comb", "pure-comb", "-"]),
    ("default", "clock_join", ["bigroot", "bigroot", "copy-hole"]),
    ("default", "clock_sync", ["bigroot", "bigroot", "copy-hole"]),
    ("default", "clock_sync_all", ["meet-shade", "stagger", "stagger"]),
    ("default", "clock_recv", ["id-pair", "hugeleaf", "copy-hole"]),
    ("default", "clock_recv_all", ["meet-shade", "stagger", "stagger"]),
    ("default", "clock_own_version_to_version", ["id-pair", "comb-scatter", "collapse-hole,site-hole"]),
    ("default", "clock_hash", ["-", "-", "-"]),
    ("default", "clock_shape", ["id-pair", "promo-rearm", "-"]),
    ("default", "version_decode_truncated", ["bigroot", VERSION_STREAM_FAMILIES, "staircase"]),
    ("default", "version_decode_trailing", ["hugeleaf", "promo-rearm", "staircase"]),
    ("default", "version_decode_noncanon", ["hugeleaf", "promo-rearm", "staircase"]),
    ("default", "span_decode_truncated", ["hoisted-window", "jump-pair", "staircase"]),
    ("default", "span_decode_trailing", ["hoisted-window", "site-hole", "staircase"]),
    ("default", "span_decode_crossed", ["hoisted-window", "hugeleaf", "ascend-plateau"]),
    ("default", "party_decode_truncated", ["id-pair,staircase", PARTY_STREAM_FAMILIES, "-"]),
    ("default", "party_decode_trailing", ["id-pair", "id-pair", "-"]),
    ("default", "party_decode_noncanon", ["id-pair", "id-pair", "-"]),
    ("default", "clock_decode_truncated", ["id-pair", "promo-rearm", "copy-hole"]),
    ("default", "clock_decode_trailing", ["id-pair", "promo-rearm", "copy-hole"]),
    ("default", "party_join_overlap", ["id-pair", "mirror-narrow", "-"]),
    ("default", "clock_join_overlap", ["id-pair", "id-pair", "-"]),
    ("default", "clock_sync_overlap", ["id-pair", "id-pair", "-"]),
    ("default", "party_without_none", ["copy-hole", "id-pair", "-"]),
    ("default", "party_serde_deserialize", ["id-pair", "id-pair", "-"]),
    ("default", "version_serde_deserialize", ["hugeleaf", "promo-rearm", "staircase"]),
    ("default", "clock_serde_deserialize", ["id-pair", "promo-rearm", "copy-hole"]),
    ("default", "rank_serde_deserialize", ["benign,concurrent-pair,descending-raises", "-", "-"]),
    ("default", "ranked_serde_deserialize", ["wide-arming", "cancelling-chain", "collapse-hole"]),
    ("default", "span_serde_deserialize", ["hoisted-window", "site-hole", "staircase"]),
    ("default", "count_serde_deserialize", ["alt-spine,benign,dense,descending-raises,masked-hole,meet-shade,mirror-narrow,nested-full,scatter,stagger,stagger-arity,stagger-size,weave", "-", "-"]),
    ("default", "party_borsh_deserialize", ["copy-hole", PARTY_STREAM_FAMILIES, "-"]),
    ("default", "version_borsh_deserialize", ["mirror-narrow,nested-full", VERSION_STREAM_FAMILIES, "staircase"]),
    ("default", "clock_borsh_deserialize", ["id-pair", "alt-spine,arming-train,ascend-cliff,ascend-plateau,benign,bigroot,cancelling-chain,cliff,cliff-fan,collapse-hole,comb-scatter,concurrent-pair,copy-hole,dense,dense-suffix,descending-raises,dominated-undercut,freeze-parade,freeze-pos,harmonic,hoisted-window,hugeleaf,id-pair,jump-comb,jump-pair,latent-ladder,lone-freeze,mask-drift,masked-hole,memo-chain,memo-churn,memo-comb,memo-fanout,memo-oscillating,mirror-narrow,mirror-wide,nested-full,nested-wide,plateau-puncture,promo-rearm,propagate-seam,pure-comb,raise-hole,reveal-comb,reveal-hifloor,site-hole,staircase,tooth-tail,weight-comb,wide-arming,wide-tooth-comb", "copy-hole"]),
    ("default", "rank_borsh_deserialize", ["benign,concurrent-pair,descending-raises", "-", "-"]),
    ("default", "ranked_borsh_deserialize", ["arming-train", "cancelling-chain", "collapse-hole"]),
    ("default", "span_borsh_deserialize", ["mirror-narrow,nested-full", "site-hole", "staircase"]),
    ("default", "count_borsh_deserialize", ["freeze-parade", "-", "-"]),
    ("acceptance", "version_decode", ["hugeleaf", "memo-oscillating", "staircase"]),
    ("acceptance", "version_encode", [VERSION_STREAM_FAMILIES, "-", "-"]),
    ("acceptance", "version_display", [VERSION_STREAM_FAMILIES, "-", "-"]),
    ("acceptance", "version_parse", ["mirror-narrow,nested-full", "memo-oscillating", "staircase"]),
    ("acceptance", "version_cmp", ["hugeleaf", "memo-oscillating", "staircase"]),
    ("acceptance", "version_eq", ["-", "-", "-"]),
    ("acceptance", "version_concurrent", ["hugeleaf", "memo-oscillating", "staircase"]),
    ("acceptance", "version_join", ["mirror-narrow", "bigroot", "staircase"]),
    ("acceptance", "version_join_assign", ["mirror-narrow", "bigroot", "staircase"]),
    ("acceptance", "version_meet", ["mirror-narrow", "site-hole", "staircase"]),
    ("acceptance", "version_meet_assign", ["mirror-narrow", "site-hole", "staircase"]),
    ("acceptance", "version_span", ["jump-pair", "jump-pair", "concurrent-pair"]),
    ("acceptance", "span_new", ["hugeleaf", "memo-oscillating", "staircase"]),
    ("acceptance", "span_union", ["benign", "meet-shade", "stagger-size"]),
    ("acceptance", "span_intersect", ["benign", "stagger-arity", "stagger-size"]),
    ("acceptance", "span_join", ["benign", "stagger", "stagger-size"]),
    ("acceptance", "span_meet", ["benign", "meet-shade", "stagger-size"]),
    ("acceptance", "span_encode", ["collapse-hole,site-hole", "-", "-"]),
    ("acceptance", "span_decode", ["hoisted-window", "site-hole", "staircase"]),
    ("acceptance", "version_tick", ["memo-comb", "memo-oscillating", "mirror-narrow"]),
    ("acceptance", "version_ticks", ["memo-comb", "memo-oscillating", "comb-scatter"]),
    ("acceptance", "version_tick_adv_party", ["ascend-cliff,ascend-plateau", "id-pair", "-"]),
    ("acceptance", "version_rank", ["arming-train", "memo-oscillating", "harmonic"]),
    ("acceptance", "rank_clone", ["pure-comb", "-", "-"]),
    ("acceptance", "rank_cmp", ["-", "-", "-"]),
    ("acceptance", "rank_add", ["hugeleaf", "-", "-"]),
    ("acceptance", "rank_checked_sub", ["cliff", "-", "-"]),
    ("acceptance", "rank_sum", ["bigroot", "-", "freeze-pos"]),
    ("acceptance", "rank_encode", ["wide-tooth-comb", "-", "-"]),
    ("acceptance", "rank_decode", ["concurrent-pair", "-", "-"]),
    ("acceptance", "rank_display", ["cancelling-chain", "-", "-"]),
    ("acceptance", "rank_display_precision", ["descending-raises", "-", "-"]),
    ("acceptance", "rank_parse", ["descending-raises", "-", "-"]),
    ("acceptance", "count_clone", ["cliff", "-", "-"]),
    ("acceptance", "count_add", ["hugeleaf", "-", "-"]),
    ("acceptance", "count_sum", ["hugeleaf", "-", "-"]),
    // Target-conditional path: `num-bigint`'s `to_radix_digits_le` takes its base from `FAST_DIV_WIDE`, true only on x86 and x86_64.
    ("acceptance", "count_display", [TARGET_DEPENDENT, "-", "-"]),
    ("acceptance", "count_parse", ["alt-spine,benign,dense,masked-hole,meet-shade,mirror-narrow,nested-full,scatter,stagger,stagger-arity,stagger-size,weave", "-", "-"]),
    ("acceptance", "version_distance", ["arming-train", "memo-oscillating", "harmonic"]),
    ("acceptance", "version_lag", ["arming-train", "memo-oscillating", "harmonic"]),
    ("acceptance", "ranked_cmp", ["arming-train", "memo-oscillating", "harmonic"]),
    ("acceptance", "ranked_encode", ["arming-train", "memo-oscillating", "harmonic"]),
    ("acceptance", "ranked_encode_rank", ["arming-train", "memo-oscillating", "harmonic"]),
    ("acceptance", "ranked_decode", ["wide-arming", "cancelling-chain", "collapse-hole"]),
    ("acceptance", "version_min_ticks", ["propagate-seam", "memo-oscillating", "copy-hole"]),
    ("acceptance", "version_join_all", ["meet-shade", "stagger", "stagger"]),
    ("acceptance", "version_meet_all", ["meet-shade", "meet-shade", "meet-shade"]),
    ("acceptance", "version_span_all", ["meet-shade", "stagger-size", "stagger-arity"]),
    ("acceptance", "span_union_all", ["meet-shade", "stagger-size", "stagger"]),
    ("acceptance", "span_intersect_all", ["meet-shade", "weave", "stagger"]),
    ("acceptance", "span_join_all", ["meet-shade", "stagger", "stagger-arity"]),
    ("acceptance", "span_meet_all", ["meet-shade", "meet-shade", "meet-shade"]),
    ("acceptance", "own_version_to_version", ["hugeleaf", "comb-scatter", "collapse-hole,site-hole"]),
    ("acceptance", "own_span_to_span", ["ascend-plateau", "comb-scatter", "copy-hole"]),
    ("acceptance", "own_version_cmp", ["hugeleaf", "cancelling-chain", "masked-hole"]),
    ("acceptance", "own_version_pair_cmp", ["hugeleaf", "memo-oscillating", "raise-hole"]),
    ("acceptance", "version_hash", ["-", "-", "-"]),
    ("acceptance", "version_shape", ["hoisted-window", "memo-oscillating", "-"]),
    ("acceptance", "shape_combine_pair", ["hoisted-window", "memo-oscillating", "-"]),
    ("acceptance", "shape_combine_many", ["meet-shade", "weave", "-"]),
    ("acceptance", "causally_contains", ["hugeleaf", "dense-suffix", "ascend-plateau"]),
    ("acceptance", "query_single_hole", ["benign", "-", "-"]),
    ("acceptance", "span_place", ["hugeleaf", "memo-oscillating", "staircase"]),
    ("acceptance", "span_dominance", ["hugeleaf", "memo-oscillating", "staircase"]),
    ("acceptance", "span_precedence", ["hugeleaf", "memo-oscillating", "staircase"]),
    ("acceptance", "span_contains", ["hugeleaf", "memo-oscillating", "staircase"]),
    ("acceptance", "query_contains", ["hugeleaf", "memo-oscillating", "staircase"]),
    ("acceptance", "query_coverage", ["hugeleaf", "propagate-seam", "staircase"]),
    ("acceptance", "query_contains_many", ["meet-shade", "meet-shade", "meet-shade"]),
    ("acceptance", "query_coverage_many", ["meet-shade", "weave", "weave"]),
    ("acceptance", "query_contains_wide_up", ["scatter", "scatter", "scatter"]),
    ("acceptance", "query_contains_wide_down", ["scatter", "scatter", "scatter"]),
    ("acceptance", "query_coverage_wide_down", ["scatter", "scatter", "scatter"]),
    ("acceptance", "query_coverage_wide_up", ["scatter", "scatter", "scatter"]),
    ("acceptance", "query_conjoin_many", ["meet-shade", "scatter", "scatter"]),
    ("acceptance", "query_clone_many", ["meet-shade", "-", "-"]),
    ("acceptance", "party_decode", ["id-pair", "id-pair", "-"]),
    ("acceptance", "party_encode", [PARTY_STREAM_FAMILIES, "-", "-"]),
    ("acceptance", "party_display", [PARTY_STREAM_FAMILIES, "-", "-"]),
    ("acceptance", "party_parse", ["copy-hole", "id-pair", "-"]),
    ("acceptance", "party_fork", ["ascend-cliff,ascend-plateau", "id-pair", "-"]),
    ("acceptance", "party_forks", ["ascend-cliff,ascend-plateau", "id-pair", "-"]),
    ("acceptance", "party_forks_full", ["meet-shade", "meet-shade", "-"]),
    ("acceptance", "party_split_array", ["raise-hole", "pure-comb", "-"]),
    ("acceptance", "party_join", ["id-pair", "benign", "-"]),
    ("acceptance", "party_join_all", ["weave", "stagger-size", "-"]),
    ("acceptance", "party_covers", ["-", "id-pair", "-"]),
    ("acceptance", "party_disjoint", ["-", "id-pair", "-"]),
    ("acceptance", "party_without", ["ascend-cliff,ascend-plateau", "id-pair", "-"]),
    ("acceptance", "party_hash", ["-", "-", "-"]),
    ("acceptance", "party_shape", ["id-pair", "id-pair", "-"]),
    ("acceptance", "clock_decode", ["id-pair", "cancelling-chain", "copy-hole"]),
    ("acceptance", "clock_encode", ["id-pair", "-", "-"]),
    ("acceptance", "clock_tick", ["id-pair", "memo-oscillating", "mirror-narrow"]),
    ("acceptance", "clock_fork", ["id-pair", "id-pair", "-"]),
    ("acceptance", "clock_forks", ["descending-raises", "id-pair", "-"]),
    ("acceptance", "clock_forks_full", ["meet-shade", "meet-shade", "-"]),
    ("acceptance", "clock_split_array", ["alt-spine,arming-train,benign,bigroot,cancelling-chain,cliff,cliff-fan,concurrent-pair,dense,dense-suffix,freeze-parade,freeze-pos,harmonic,hoisted-window,hugeleaf,jump-comb,jump-pair,latent-ladder,lone-freeze,mask-drift,masked-hole,plateau-puncture,promo-rearm,propagate-seam,tooth-tail,weight-comb,wide-arming,wide-tooth-comb", "pure-comb", "-"]),
    ("acceptance", "clock_join", ["bigroot", "bigroot", "copy-hole"]),
    ("acceptance", "clock_sync", ["bigroot", "bigroot", "copy-hole"]),
    ("acceptance", "clock_sync_all", ["meet-shade", "stagger", "stagger"]),
    ("acceptance", "clock_recv", ["id-pair", "hugeleaf", "copy-hole"]),
    ("acceptance", "clock_recv_all", ["meet-shade", "stagger", "stagger"]),
    ("acceptance", "clock_own_version_to_version", ["id-pair", "comb-scatter", "collapse-hole,site-hole"]),
    ("acceptance", "clock_hash", ["-", "-", "-"]),
    ("acceptance", "clock_shape", ["id-pair", "cancelling-chain", "-"]),
    ("acceptance", "version_decode_truncated", ["bigroot", VERSION_STREAM_FAMILIES, "staircase"]),
    ("acceptance", "version_decode_trailing", ["hugeleaf", "memo-oscillating", "staircase"]),
    ("acceptance", "version_decode_noncanon", ["hugeleaf", "cancelling-chain", "staircase"]),
    ("acceptance", "span_decode_truncated", ["hoisted-window", "jump-pair", "staircase"]),
    ("acceptance", "span_decode_trailing", ["hoisted-window", "site-hole", "staircase"]),
    ("acceptance", "span_decode_crossed", ["hoisted-window", "hugeleaf", "ascend-plateau"]),
    ("acceptance", "party_decode_truncated", ["id-pair,staircase", PARTY_STREAM_FAMILIES, "-"]),
    ("acceptance", "party_decode_trailing", ["id-pair", "id-pair", "-"]),
    ("acceptance", "party_decode_noncanon", ["id-pair", "id-pair", "-"]),
    ("acceptance", "clock_decode_truncated", ["id-pair", "cancelling-chain", "copy-hole"]),
    ("acceptance", "clock_decode_trailing", ["id-pair", "cancelling-chain", "copy-hole"]),
    ("acceptance", "party_join_overlap", ["id-pair", "mirror-narrow", "-"]),
    ("acceptance", "clock_join_overlap", ["id-pair", "id-pair", "-"]),
    ("acceptance", "clock_sync_overlap", ["id-pair", "id-pair", "-"]),
    ("acceptance", "party_without_none", ["id-pair", "id-pair", "-"]),
    ("acceptance", "party_serde_deserialize", ["id-pair", "id-pair", "-"]),
    ("acceptance", "version_serde_deserialize", ["hugeleaf", "memo-oscillating", "staircase"]),
    ("acceptance", "clock_serde_deserialize", ["id-pair", "cancelling-chain", "copy-hole"]),
    ("acceptance", "rank_serde_deserialize", ["concurrent-pair", "-", "-"]),
    ("acceptance", "ranked_serde_deserialize", ["wide-arming", "cancelling-chain", "collapse-hole"]),
    ("acceptance", "span_serde_deserialize", ["hoisted-window", "site-hole", "staircase"]),
    ("acceptance", "count_serde_deserialize", ["alt-spine,benign,dense,masked-hole,meet-shade,mirror-narrow,nested-full,scatter,stagger,stagger-arity,stagger-size,weave", "-", "-"]),
    ("acceptance", "party_borsh_deserialize", ["copy-hole", PARTY_STREAM_FAMILIES, "-"]),
    ("acceptance", "version_borsh_deserialize", ["mirror-narrow,nested-full", VERSION_STREAM_FAMILIES, "staircase"]),
    ("acceptance", "clock_borsh_deserialize", ["id-pair", "alt-spine,arming-train,ascend-cliff,ascend-plateau,benign,bigroot,cancelling-chain,cliff,cliff-fan,collapse-hole,comb-scatter,concurrent-pair,copy-hole,dense,dense-suffix,descending-raises,dominated-undercut,freeze-parade,freeze-pos,harmonic,hoisted-window,hugeleaf,id-pair,jump-comb,jump-pair,latent-ladder,lone-freeze,mask-drift,masked-hole,memo-chain,memo-churn,memo-comb,memo-fanout,memo-oscillating,mirror-narrow,mirror-wide,nested-full,nested-wide,plateau-puncture,promo-rearm,propagate-seam,pure-comb,raise-hole,reveal-comb,reveal-hifloor,site-hole,staircase,tooth-tail,weight-comb,wide-arming,wide-tooth-comb", "copy-hole"]),
    ("acceptance", "rank_borsh_deserialize", ["concurrent-pair", "-", "-"]),
    ("acceptance", "ranked_borsh_deserialize", ["arming-train", "cancelling-chain", "collapse-hole"]),
    ("acceptance", "span_borsh_deserialize", ["mirror-narrow,nested-full", "site-hole", "staircase"]),
    ("acceptance", "count_borsh_deserialize", ["freeze-parade", "-", "-"]),
];

/// Entry-compare one sampling scale's folded map against that scale's entries
/// in `pins`, writing one drift line per disagreement to `out`.
///
/// Returns `Ok(true)` when every entry matches. An exact pin (a family set, or
/// `-`) matches only the identical live worst set. A [`TARGET_DEPENDENT`] pin
/// matches any live worst set, but not a row on which no family drives the
/// currency: that drift line says the work went away or the meter stopped
/// seeing it, never that a ranking flipped. Detects both directions of rot: a
/// live row missing from the pin and a pinned row the map no longer has.
///
/// # Panics
///
/// Panics if a mapped counter is not compiled into this run: the pin is stated
/// over all three mapped currencies, so the check requires the `touch-meter`
/// and `scan-meter` features.
pub(super) fn check_scale(
    label: &str,
    map: &[OpWorst],
    pins: &[(&str, &str, [&str; 3])],
    out: &mut dyn Write,
) -> io::Result<bool> {
    let mut clean = true;
    let mut live_ops = BTreeSet::new();
    for row in map {
        live_ops.insert(row.op);
        let pinned = pins
            .iter()
            .find(|(scale, op, _)| *scale == label && *op == row.op);
        for (i, c) in row.per_currency.iter().enumerate() {
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
            let (op, cur) = (row.op, c.currency.label());
            match pinned.map(|(_, _, columns)| columns[i]) {
                Some(TARGET_DEPENDENT) if live == "-" => {
                    clean = false;
                    writeln!(
                        out,
                        "worst-case pin drift: {op} x {cur} at the {label} scale: pinned worst \
                         {TARGET_DEPENDENT}, live worst -: no family drives a currency the pin \
                         declares target-dependent: either the work legitimately went away \
                         (re-pin to -, naming the change) or the meter stopped seeing the work \
                         (investigate first)"
                    )?;
                }
                Some(TARGET_DEPENDENT) => {}
                Some(old) if old == live => {}
                old => {
                    clean = false;
                    writeln!(
                        out,
                        "worst-case pin drift: {op} x {cur} at the {label} scale: pinned worst \
                         {old}, live worst {live}: a ranking flip is news: either a family \
                         legitimately overtook (re-pin deliberately with a movement annotation) \
                         or a code change made some shape relatively worse (investigate first)",
                        old = old.unwrap_or("(no entry)"),
                    )?;
                }
            }
        }
    }
    for (scale, op, _) in pins {
        if *scale == label && !live_ops.contains(op) {
            clean = false;
            writeln!(
                out,
                "worst-case pin drift: the pin names {op} at the {label} scale but the board \
                 produces no such operation row: drop or rename the stale entry"
            )?;
        }
    }
    Ok(clean)
}

/// Entry-compare the live worst-case fold against the committed ranking pin
/// (the `WORST_RANKINGS` table beside the fold), writing one drift line per
/// disagreement to `out`.
///
/// `sweeps` yields one whole board's judged cells per sampling scale — a shard
/// merge under [`check_worst_map`](super::shard::check_worst_map). Each scale's
/// fold is judged by [`check_scale`].
///
/// Returns `Ok(true)` when every entry matches at every scale.
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
        let map = fold(&sweeps(scale)?);
        clean &= check_scale(label, &map, WORST_RANKINGS, out)?;
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
