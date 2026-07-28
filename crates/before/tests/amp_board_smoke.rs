//! Smoke coverage for the amplification board (`before::meter::board`).
//!
//! The board is the campaign's dashboard, not its enforcement: this test
//! only pins that the whole sweep keeps compiling and running — every
//! operation row prepares, measures at both scales, and renders. It
//! deliberately asserts no colors: red cells are expected while amplifiers
//! remain, and the enforced resource record is the process-isolated
//! envelope suite in `tests/meter.rs`.
//!
//! This binary also holds the board↔band parity pin: the dashboard's
//! family roster and the envelope suite's flatness/adequacy bands must
//! name each other, one committed mapping with documented exemptions,
//! so neither surface can drift structurally blind to a genre the
//! other prices.

use std::collections::{BTreeMap, BTreeSet};

use before::meter::board::{self, HeapMeter};
use peak_alloc::PeakAlloc;

#[global_allocator]
static HEAP: PeakAlloc = PeakAlloc;

/// A fraction of the board's default sizes, small enough that the smoke run
/// stays well under a second.
const SMOKE_SCALE: f64 = 0.02;

/// The board's cell count, pinned per family: how many operation rows
/// each shape's operand bundle supplies.
///
/// The board's coverage is the product of its two axes (the module doc's
/// product section), so the pin lives on the family axis: a row added to
/// or dropped from the operation table moves every count it touches, a
/// shape whose bundle gains or loses a slot moves its own count, and the
/// failure names the family that drifted. The version-only shapes (a
/// version, its derived pairings, and its rejection rows) supply 43
/// rows; the id pair (parties only) 38; the cross shapes (version,
/// mounted party pair, clock, and the id-side rejections) 64; the three
/// fold-only populations (scatter, weave, stagger) exactly the 2 fold
/// rows; and the benign control supplies every row.
const EXPECTED_CELLS_PER_FAMILY: &[(&str, usize)] = &[
    ("dense", 43),
    ("bigroot", 43),
    ("hugeleaf", 43),
    ("cliff", 43),
    ("id-pair", 38),
    ("comb-scatter", 64),
    ("harmonic", 43),
    ("scatter", 2),
    ("weave", 2),
    ("stagger", 2),
    ("nested-full", 64),
    ("nested-wide", 64),
    ("mirror-wide", 64),
    ("mirror-narrow", 64),
    ("staircase", 64),
    ("reveal-comb", 64),
    ("reveal-hifloor", 64),
    ("pure-comb", 64),
    ("ascend-cliff", 64),
    ("ascend-plateau", 64),
    ("jump-pair", 43),
    ("freeze-pos", 43),
    ("promo-rearm", 43),
    ("weight-comb", 43),
    ("freeze-parade", 43),
    ("concurrent-pair", 43),
    ("tooth-tail", 43),
    ("benign", 66),
];

/// The board runs to completion at tiny sizes — every cell prepares,
/// measures, and renders — and the matrix keeps covering the full
/// operation sweep, family by family.
///
/// Each shape's cell count must match its pinned bundle reach. Colors
/// are deliberately not asserted: the board is a dashboard, not a gate.
#[test]
fn board_runs_to_completion() {
    let heap = HeapMeter {
        reset_peak: || HEAP.reset_peak_usage(),
        peak: || HEAP.peak_usage(),
        current: || HEAP.current_usage(),
    };
    let mut rendered = Vec::new();
    let summary = board::run(SMOKE_SCALE, &heap, &mut rendered).expect("writing to a Vec succeeds");
    let text = String::from_utf8(rendered).expect("the board renders UTF-8");
    // Count rendered cells per family: every result row starts with its
    // verdict, then the operation, then the family.
    let mut per_family: BTreeMap<&str, usize> = BTreeMap::new();
    for line in text.lines() {
        let mut cols = line.split_whitespace();
        let verdict = cols.next();
        if !matches!(verdict, Some("GREEN" | "RED")) {
            continue;
        }
        let family = cols
            .nth(1)
            .expect("a verdict row names its operation and family");
        *per_family.entry(family).or_default() += 1;
    }
    let expected: BTreeMap<&str, usize> = EXPECTED_CELLS_PER_FAMILY.iter().copied().collect();
    assert_eq!(
        expected.len(),
        EXPECTED_CELLS_PER_FAMILY.len(),
        "duplicate family in the expectation table"
    );
    assert_eq!(
        per_family, expected,
        "the board's per-family cell counts drifted from the pinned bundle \
         reach: rows were added or lost without moving the pin"
    );
    let cells = summary.green + summary.red;
    let total: usize = EXPECTED_CELLS_PER_FAMILY.iter().map(|(_, n)| n).sum();
    assert_eq!(
        cells, total,
        "the returned summary must agree with the rendered matrix"
    );
    assert!(
        text.contains(&format!("({cells} cells)")),
        "the rendered summary line must agree with the returned summary"
    );
}

// ─── the board↔band parity pin ──────────────────────────────────────────────

/// Each flatness/adequacy band's board family: the envelope-suite test
/// (`tests/meter.rs`) on the left, the family whose operand
/// construction it prices on the right.
///
/// One family may carry several bands (one per priced operation); a
/// band with no family belongs in [`BAND_ONLY`] instead, never
/// unlisted.
const BAND_TO_FAMILY: &[(&str, &str)] = &[
    ("skyline_validate_cliff_cost_is_flat_per_unit", "cliff"),
    ("skyline_cmp_cliff_cost_is_flat_per_unit", "cliff"),
    ("skyline_join_cliff_cost_is_flat_per_unit", "cliff"),
    ("skyline_parse_cliff_touch_cost_is_flat_per_unit", "cliff"),
    ("skyline_min_ticks_pure_comb_is_flat_per_unit", "pure-comb"),
    (
        "skyline_min_ticks_reveal_comb_is_flat_per_unit",
        "reveal-comb",
    ),
    (
        "skyline_rank_freeze_position_is_flat_per_unit",
        "freeze-pos",
    ),
    (
        "skyline_distance_freeze_position_is_flat_per_unit",
        "freeze-pos",
    ),
    (
        "skyline_rank_promotion_rearm_is_flat_per_unit",
        "promo-rearm",
    ),
    (
        "skyline_distance_promotion_rearm_is_flat_per_unit",
        "promo-rearm",
    ),
    ("skyline_distance_jump_pair_is_flat_per_unit", "jump-pair"),
    ("skyline_rank_weight_comb_is_flat_per_unit", "weight-comb"),
    (
        "skyline_rank_freeze_parade_is_flat_per_unit",
        "freeze-parade",
    ),
    (
        "reveal_comb_hifloor_control_is_flat_per_unit",
        "reveal-hifloor",
    ),
    (
        "ascend_cliff_plateau_control_is_flat_per_unit",
        "ascend-plateau",
    ),
    ("skyline_cmp_tooth_tail_is_flat_per_unit", "tooth-tail"),
    (
        "fold_version_stagger_arity_axis_is_flat_per_unit",
        "stagger",
    ),
    ("fold_version_stagger_size_axis_is_flat_per_unit", "stagger"),
    ("fold_party_stagger_arity_axis_is_flat_per_unit", "stagger"),
    ("fold_party_stagger_size_axis_is_flat_per_unit", "stagger"),
];

/// Bands deliberately without a board family, each with its reason: a
/// kernel-seam probe stays in the envelope suite alone (the `FAMILIES`
/// roster criterion), and a multi-family or seam-scoped band names no
/// single column.
const BAND_ONLY: &[(&str, &str)] = &[
    (
        "ticks_flatness_holds_the_log_band",
        "prices the ticks count axis across three already-rostered families, not a shape of its own",
    ),
    (
        "skyline_rank_wide_tooth_freeze_band",
        "wide_tooth_comb is a kernel-seam probe by the FAMILIES roster criterion",
    ),
    (
        "skyline_rank_jump_eviction_is_flat_per_unit",
        "jump_comb is a kernel-seam probe; its whole-surface lift is the jump-pair family",
    ),
    (
        "masked_cmp_drift_cost_is_flat_per_unit",
        "the mask-drift triple is a kernel-seam probe of the masked sweep",
    ),
    (
        "masked_pair_cmp_drift_cost_is_flat_per_unit",
        "the mask-drift quadruple is a kernel-seam probe of the fused four-stream sweep",
    ),
    (
        "memo_chain_shared_control_is_flat_per_unit",
        "the memo_* shapes are kernel-seam probes by the FAMILIES roster criterion",
    ),
    (
        "skyline_rank_dense_suffix_is_flat_per_unit",
        "the ledger-settle witness family prices the settle's suffix-charging seam; its board promotion is a pending owner decision",
    ),
    (
        "skyline_distance_dense_suffix_is_flat_per_unit",
        "as the dense-suffix rank band's",
    ),
];

/// Board families deliberately without a flatness/adequacy band, each
/// with its reason.
const FAMILY_ONLY: &[(&str, &str)] = &[
    (
        "dense",
        "depth/node maximizer; absolute envelope rows carry it, no committed two-point flatness claim",
    ),
    (
        "bigroot",
        "magnitude-over-depth shape; absolute envelope rows carry it",
    ),
    ("hugeleaf", "single-node magnitude maximizer; absolute envelope rows carry it"),
    (
        "id-pair",
        "party-only bundle; the flatness bands price version query and comparison kernels",
    ),
    (
        "comb-scatter",
        "the output-domination cross; its projection rows are I/O-denominated",
    ),
    (
        "harmonic",
        "the rank fold's wide-numerator adversary; the board's harmonic tripwire column and its envelope rows carry it",
    ),
    ("scatter", "fold-only bundle; the fold rows are judged by the declared fold model"),
    ("weave", "fold-only bundle; the fold rows are judged by the declared fold model"),
    ("nested-full", "tick cross; the tick gate pins price its walk"),
    ("nested-wide", "tick cross; the tick gate pins price its walk"),
    ("mirror-wide", "tick cross; the tick gate pins price its walk"),
    ("mirror-narrow", "tick cross; the tick gate pins price its walk"),
    ("staircase", "tick cross; the tick gate pins price its walk"),
    (
        "ascend-cliff",
        "the cascade's red-direction driver; its leveled control (ascend-plateau) carries the committed flatness band",
    ),
    (
        "concurrent-pair",
        "the switch-density pair; absolute envelope pair-query rows carry it",
    ),
    (
        "benign",
        "the organic control population; flatness bands price adversarial constructions",
    ),
];

/// The envelope suite's flatness/adequacy band tests: every
/// `#[test]`-attributed function in `tests/meter.rs` whose name carries
/// the band convention (`_is_flat_per_unit` anywhere, or the `_band`
/// suffix).
///
/// Attribute-gated so helpers and run harnesses never count; a scan
/// that silently matches nothing fails the parity test on every
/// rostered name, so the scanner cannot rot into a clean sweep.
fn band_test_names(source: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut armed = false;
    for line in source.lines() {
        let t = line.trim();
        if t == "#[test]" {
            armed = true;
            continue;
        }
        if t.starts_with("#[") || t.is_empty() {
            // cfg or other attributes between `#[test]` and the fn keep
            // the arming; anything else below drops it.
            continue;
        }
        if armed {
            if let Some(rest) = t.strip_prefix("fn ") {
                if let Some(name) = rest.split('(').next() {
                    if name.contains("_is_flat_per_unit") || name.ends_with("_band") {
                        names.insert(name.to_string());
                    }
                }
            }
            armed = false;
        }
    }
    names
}

/// The board's families and the envelope suite's flatness/adequacy
/// bands name each other, and each failure names the missing side.
///
/// Every band maps to a board family or carries a documented
/// exemption, and every board family carries a band or a documented
/// exemption.
///
/// The pin is the committed replacement for hand-maintained parity
/// between the dashboard and the enforcement suite: a band landed
/// without a board family (or a family landed without a band) fails
/// here until the column, the band, or the reviewed exemption exists.
#[test]
fn board_families_and_flatness_bands_stay_paired() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/meter.rs");
    let source = std::fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("reading the envelope suite at {path} failed: {err}"));
    let scanned = band_test_names(&source);

    let mapped: BTreeMap<&str, &str> = BAND_TO_FAMILY.iter().copied().collect();
    let band_exempt: BTreeMap<&str, &str> = BAND_ONLY.iter().copied().collect();
    assert_eq!(
        mapped.len(),
        BAND_TO_FAMILY.len(),
        "duplicate band in BAND_TO_FAMILY"
    );
    assert_eq!(
        band_exempt.len(),
        BAND_ONLY.len(),
        "duplicate band in BAND_ONLY"
    );
    for band in mapped.keys() {
        assert!(
            !band_exempt.contains_key(band),
            "band `{band}` is both mapped and exempted: drop one entry"
        );
    }

    // Band side: every scanned band is mapped or exempted, and every
    // rostered name still exists in the suite.
    for band in &scanned {
        assert!(
            mapped.contains_key(band.as_str()) || band_exempt.contains_key(band.as_str()),
            "the envelope band `{band}` has no board family: the BOARD side is \
             missing — add the family column (BAND_TO_FAMILY) or the reviewed \
             exemption (BAND_ONLY)"
        );
    }
    for band in mapped.keys().chain(band_exempt.keys()) {
        assert!(
            scanned.contains(*band),
            "the parity roster names `{band}` but the envelope suite declares no \
             such band: the BAND side is missing — restore the band or drop the \
             stale entry"
        );
    }

    // Family side: every board family carries a band or an exemption,
    // exclusively, and no mapping or exemption names a family the board
    // does not run.
    let families: BTreeSet<&str> = EXPECTED_CELLS_PER_FAMILY.iter().map(|(f, _)| *f).collect();
    let banded: BTreeSet<&str> = mapped.values().copied().collect();
    let family_exempt: BTreeMap<&str, &str> = FAMILY_ONLY.iter().copied().collect();
    assert_eq!(
        family_exempt.len(),
        FAMILY_ONLY.len(),
        "duplicate family in FAMILY_ONLY"
    );
    for family in banded.iter() {
        assert!(
            families.contains(family),
            "a band maps to `{family}`, which is not a board family: the BOARD \
             side is missing — add the family or fix the mapping"
        );
    }
    for (family, _) in FAMILY_ONLY {
        assert!(
            families.contains(family),
            "FAMILY_ONLY exempts `{family}`, which is not a board family: drop \
             the stale exemption"
        );
        assert!(
            !banded.contains(family),
            "`{family}` carries a band and an exemption: it earned its band, \
             drop the FAMILY_ONLY entry"
        );
    }
    for family in &families {
        assert!(
            banded.contains(family) || family_exempt.contains_key(family),
            "the board family `{family}` has no flatness/adequacy band: the \
             BAND side is missing — commit the band (BAND_TO_FAMILY) or the \
             reviewed exemption (FAMILY_ONLY)"
        );
    }
}
