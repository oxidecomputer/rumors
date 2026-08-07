//! Smoke coverage for the amplification board (`before::meter::board`).
//!
//! The board is the campaign's dashboard, not its enforcement: this test
//! only pins that the whole sweep keeps compiling and running — every
//! operation row prepares, measures at both of its window's sizes, and
//! renders. It
//! deliberately asserts no colors: verdicts of record belong to the
//! release-profile board runs the gate's board leg consumes (any red
//! cell there fails it), and the enforced resource record is the
//! process-isolated envelope suite in `tests/meter.rs`.
//!
//! This binary also holds the registry's band-name parity survivor (the
//! `before::meter::registry` module doc names it): the envelope suite's
//! band-named tests live in a separate test binary, where test function
//! names are strings the compiler cannot resolve, so the scan below
//! holds them equal, name for name, to the registry's committed band
//! citations.

use std::collections::{BTreeMap, BTreeSet};

use before::meter::board::{self, HeapMeter};
use before::meter::registry::{Bands, Coverage, FamilyId, AXIS_BANDS};
use peak_alloc::PeakAlloc;

#[global_allocator]
static HEAP: PeakAlloc = PeakAlloc;

/// A fraction of the board's default sizes, small enough that the smoke run
/// stays well under a second.
const SMOKE_SCALE: f64 = 0.02;

/// The peak-heap readers over this binary's global allocator.
fn heap_meter() -> HeapMeter {
    HeapMeter {
        reset_peak: || HEAP.reset_peak_usage(),
        peak: || HEAP.peak_usage(),
        current: || HEAP.current_usage(),
    }
}

/// A shard spawner that measures every slice in this process instead of
/// spawning children.
///
/// The runner's spawner launches one copy of the `amp_board` binary per
/// slice; the seam it feeds — emit, parse, reorder, re-judge — is the
/// same either way, so the smoke suite drives it without processes.
fn in_process_spawn(
    shards: usize,
    heap: &HeapMeter,
) -> impl Fn(f64) -> std::io::Result<Vec<Vec<u8>>> + '_ {
    move |scale: f64| {
        (0..shards)
            .map(|index| {
                let mut capture = Vec::new();
                board::emit_shard(scale, index, shards, heap, &mut capture)?;
                Ok(capture)
            })
            .collect()
    }
}

/// The board's per-family cell expectations, derived from the registry:
/// each board family's declared bundle reach, keyed by its name of
/// record.
///
/// The board's coverage is the product of its two axes (the board module
/// doc's product section), so the expectation lives on the family axis: a
/// row added to or dropped from the operation table moves every reach it
/// touches, a shape whose bundle gains or loses a slot moves its own, and
/// either drift fails against the registry's committed answer until the
/// variant's `Coverage::Board` declaration is deliberately re-stated.
fn expected_cells_per_family() -> BTreeMap<&'static str, usize> {
    FamilyId::board()
        .map(|family| {
            let Coverage::Board { cells } = family.spec().coverage else {
                unreachable!("FamilyId::board() filters on the Board coverage answer")
            };
            (family.name(), cells)
        })
        .collect()
}

/// The board runs to completion at tiny sizes — every cell prepares,
/// measures, and renders — and the matrix keeps covering the full
/// operation sweep, family by family.
///
/// Each shape's cell count must match the bundle reach its registry
/// variant declares. Colors are deliberately not asserted: the board is
/// a dashboard, not a gate.
#[test]
fn board_runs_to_completion() {
    let heap = heap_meter();
    let spawn = in_process_spawn(1, &heap);
    let mut rendered = Vec::new();
    let summary =
        board::run(SMOKE_SCALE, 1, &spawn, &mut rendered).expect("writing to a Vec succeeds");
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
    let expected = expected_cells_per_family();
    assert_eq!(
        per_family, expected,
        "the board's per-family cell counts drifted from the registry's declared \
         bundle reach: rows were added or lost without re-stating the variant's \
         Coverage::Board answer"
    );
    let cells = summary.green + summary.red;
    let total: usize = expected.values().sum();
    assert_eq!(
        cells, total,
        "the returned summary must agree with the rendered matrix"
    );
    assert!(
        text.contains(&format!("({cells} cells)")),
        "the rendered summary line must agree with the returned summary"
    );
}

/// The shard protocol round-trips: a board dealt across three shards
/// emits, parses, reorders, and re-judges into the same matrix as one
/// undivided shard.
///
/// This is the merge's coverage — the wire form's field order, the
/// ownership and count guards, and the board-order reconstruction — and
/// byte-identity is how it asserts, since every judged quantity is a
/// deterministic counter over state each shard owns privately. Three
/// shards deal the operation × family grid unevenly, so the
/// reconstruction cannot pass by coincidence of an even split.
#[test]
fn shard_protocol_round_trips() {
    let heap = heap_meter();
    let mut whole = Vec::new();
    board::run(SMOKE_SCALE, 1, &in_process_spawn(1, &heap), &mut whole)
        .expect("writing to a Vec succeeds");

    // Three shards over the cell grid: uneven slices unless the grid
    // size happens to be a multiple.
    const SHARDS: usize = 3;
    let mut split = Vec::new();
    board::run(
        SMOKE_SCALE,
        SHARDS,
        &in_process_spawn(SHARDS, &heap),
        &mut split,
    )
    .expect("writing to a Vec succeeds");

    assert_eq!(
        String::from_utf8(whole).expect("the board renders UTF-8"),
        String::from_utf8(split).expect("the board renders UTF-8"),
        "renders taken at different shard counts must be byte-identical"
    );
}

/// The worst-case map folds totally over the board's sweep at any scale:
/// every operation row renders exactly one line per mapped currency
/// (heap, limb, scan, touch), so a row can neither drop out of the map
/// nor render twice.
///
/// Rankings are deliberately not asserted: they are scale- and
/// profile-dependent readings, and the map of record is the
/// release-profile entry-compare against the committed pin
/// (`just worst-cases-pin`).
#[test]
fn worst_map_covers_every_operation_row() {
    let heap = heap_meter();
    let spawn = in_process_spawn(1, &heap);
    let mut rendered = Vec::new();
    board::worst_map("smoke", SMOKE_SCALE, 1, &spawn, &mut rendered)
        .expect("writing to a Vec succeeds");
    let text = String::from_utf8(rendered).expect("the map renders UTF-8");
    let mut per_op: BTreeMap<&str, usize> = BTreeMap::new();
    for line in text.lines() {
        let mut cols = line.split_whitespace();
        let (Some(op), Some(currency), Some(marker)) = (cols.next(), cols.next(), cols.next())
        else {
            continue;
        };
        if marker != "worst" || !matches!(currency, "heap" | "limb" | "scan" | "touch") {
            continue;
        }
        *per_op.entry(op).or_default() += 1;
    }
    // The benign control supplies every operation row (the registry's
    // declared reach), so its count is the operation axis's length.
    let expected = expected_cells_per_family();
    let ops_total = expected
        .get("benign")
        .expect("the benign control is on the board roster");
    assert_eq!(
        per_op.len(),
        *ops_total,
        "the map must carry every operation row exactly once"
    );
    assert!(
        per_op.values().all(|&rows| rows == 4),
        "every operation renders one row per mapped currency: {per_op:?}"
    );
}

// ─── the band-name parity survivor ──────────────────────────────────────────

/// The envelope suite's flatness/adequacy band tests: every
/// `#[test]`-attributed function in `tests/meter.rs` whose name carries
/// the band convention (`_is_flat_per_unit` anywhere, or the `_band`
/// suffix).
///
/// Attribute-gated so helpers and run harnesses never count; a scan
/// that silently matches nothing fails the parity test on every
/// registry-cited name, so the scanner cannot rot into a clean sweep.
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

/// The envelope suite's band-named tests and the registry's band
/// citations name each other, name for name, and each failure names the
/// missing side.
///
/// This is the registry's named parity survivor for band names (the
/// `before::meter::registry` module doc): the bands live in this crate's
/// separate test binary, where test function names are not items the
/// compiler can resolve, so the seam is pinned here — every band-named
/// test is cited by exactly one family's `Bands::Priced` roster or by
/// `AXIS_BANDS`, and every citation resolves to a live test. Citation
/// uniqueness itself is pinned by the registry's own tests; the
/// band-to-family *construction* link needs no pin at all, because a
/// band can only mint its operands through `registry::Shape`.
#[test]
fn band_tests_and_registry_citations_stay_paired() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/meter.rs");
    let source = std::fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("reading the envelope suite at {path} failed: {err}"));
    let scanned = band_test_names(&source);

    let mut cited: BTreeMap<&str, &str> = BTreeMap::new();
    for family in FamilyId::ALL {
        if let Bands::Priced(bands) = family.spec().bands {
            for band in bands {
                cited.insert(band, family.name());
            }
        }
    }
    for (band, _) in AXIS_BANDS {
        cited.insert(band, "AXIS_BANDS");
    }

    for band in &scanned {
        assert!(
            cited.contains_key(band.as_str()),
            "the envelope band `{band}` has no registry answer: cite it on its \
             family's spec (Bands::Priced) or, if it prices an operation-argument \
             axis rather than a shape, in registry::AXIS_BANDS"
        );
    }
    for (band, owner) in &cited {
        assert!(
            scanned.contains(*band),
            "the registry ({owner}) cites band `{band}` but the envelope suite \
             declares no such test: restore the band or drop the citation"
        );
    }
}
