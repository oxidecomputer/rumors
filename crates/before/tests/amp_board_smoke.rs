//! Smoke coverage for the amplification board (`before::meter::board`).
//!
//! The board is the campaign's dashboard, not its enforcement: this test
//! only pins that the whole sweep keeps compiling and running — every
//! operation row prepares, measures at both scales, and renders. It
//! deliberately asserts no colors: red cells are expected while amplifiers
//! remain, and the enforced resource record is the process-isolated
//! envelope suite in `tests/meter.rs`.

use std::collections::BTreeMap;

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
/// mounted party pair, clock, and the id-side rejections) 64; the two
/// fold-only populations (scatter, weave) exactly the 2 fold rows; and
/// the benign control supplies every row.
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
    ("concurrent-pair", 43),
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
