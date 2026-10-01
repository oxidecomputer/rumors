//! Smoke coverage for the amplification board (`before::testing::meter::board`).
//!
//! The tests drive every board cell, exercise shard merging, and pin the
//! operation and family coverage maps. Verdicts of record belong to the
//! release-profile board run consumed by the gate.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Mutex, MutexGuard};

use before::testing::meter::board::{self, HeapMeter, BOARD_PRICED};
use before::testing::meter::registry::FamilyId;
use peak_alloc::PeakAlloc;

#[global_allocator]
static HEAP: PeakAlloc = PeakAlloc;

/// Serializes in-process measurements that share [`HEAP`].
static MEASUREMENT: Mutex<()> = Mutex::new(());

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

/// Hold exclusive access to the process-global meter for one smoke test.
fn measurement_guard() -> MutexGuard<'static, ()> {
    MEASUREMENT
        .lock()
        .expect("no smoke test panics while holding the measurement lock")
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

/// Retaining an allocation-free operation's result adds no measured heap.
/// Check both samples of every Version equality row in the existing sweep;
/// boxing the result would make these otherwise allocation-free rows nonzero.
#[test]
fn retaining_results_does_not_allocate_measured_heap() {
    let _guard = measurement_guard();
    let captures =
        in_process_spawn(1, &heap_meter())(SMOKE_SCALE).expect("the in-process sweep succeeds");
    let text = std::str::from_utf8(&captures[0]).expect("the shard protocol is UTF-8");
    let mut checked = 0;
    for line in text.lines() {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.get(1) != Some(&"version_eq") {
            continue;
        }
        // Each sample carries two denominators, three models, three readings,
        // and three floors. Heap is the first reading in the internal protocol.
        for sample in fields[3..].chunks_exact(11) {
            assert_eq!(sample[5], "0", "{} equality allocates no heap", fields[2]);
            checked += 1;
        }
    }
    assert!(checked > 0, "the sweep must exercise Version equality");
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
/// family specification is deliberately re-stated.
fn expected_cells_per_family() -> BTreeMap<&'static str, usize> {
    FamilyId::board()
        .map(|family| (family.name(), family.spec().cells))
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
    let _guard = measurement_guard();
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
         declared reach"
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
    let _guard = measurement_guard();
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

    let whole = String::from_utf8(whole).expect("the board renders UTF-8");
    let split = String::from_utf8(split).expect("the board renders UTF-8");
    let whole_lines = whole.lines().collect::<Vec<_>>();
    let split_lines = split.lines().collect::<Vec<_>>();
    assert_eq!(
        whole_lines.len(),
        split_lines.len(),
        "shard counts must render the same number of lines"
    );
    for (index, (whole, split)) in whole_lines.iter().zip(&split_lines).enumerate() {
        assert_eq!(
            whole,
            split,
            "renders taken at different shard counts differ on line {}",
            index + 1,
        );
    }
}

/// The worst-case map folds totally over the board's sweep at any scale:
/// every operation row renders exactly one line per mapped currency
/// (heap, scan, touch), so a row can neither drop out of the map
/// nor render twice.
///
/// Rankings are deliberately not asserted: they are scale- and
/// profile-dependent readings, and the map of record is the
/// release-profile entry-compare against the committed pin
/// (`just worst-cases-pin`).
#[test]
fn worst_map_covers_every_operation_row() {
    let _guard = measurement_guard();
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
        if marker != "worst" || !matches!(currency, "heap" | "scan" | "touch") {
            continue;
        }
        *per_op.entry(op).or_default() += 1;
    }
    let ops_total = BOARD_PRICED
        .iter()
        .flat_map(|(_, rows)| *rows)
        .collect::<BTreeSet<_>>()
        .len();
    assert_eq!(
        per_op.len(),
        ops_total,
        "the map must carry every operation row exactly once"
    );
    assert!(
        per_op.values().all(|&rows| rows == 3),
        "every operation renders one row per mapped currency: {per_op:?}"
    );
}

/// The merge refuses a silently shrunk grid: a genuine capture with one
/// cell line deleted and the end count restated is refused naming the
/// shorted family, for every family on the board.
///
/// This is the completeness refusal's committed known-bad artifact: each
/// tampered capture is well-formed shard output whose only defect is one
/// missing cell, and the merge must refuse it rather than render the
/// shrunk board. The sweep quantifies over the family axis — the axis the
/// per-family refusal discriminates on; positions within one family short
/// the same count — by dropping each family's first cell, plus the
/// grid's last cell (the boundary the end-count restatement is easiest to
/// forge at). Being a merge-layer check it fires at every scale,
/// including the release acceptance ladder — the completeness the
/// per-family smoke pin (`board_runs_to_completion`) can only attest at
/// its own scale.
#[test]
fn merge_refuses_a_silently_shrunk_grid_for_every_family() {
    let _guard = measurement_guard();
    let heap = heap_meter();
    let honest = in_process_spawn(1, &heap)(SMOKE_SCALE).expect("in-process capture succeeds");
    let text = String::from_utf8(honest[0].clone()).expect("shard captures are UTF-8");
    let lines: Vec<&str> = text.lines().collect();
    let declared: usize = lines
        .last()
        .expect("a capture ends with its end line")
        .strip_prefix("end ")
        .expect("the last line is the end line")
        .parse()
        .expect("the end line declares a count");
    assert_eq!(
        declared,
        lines.len() - 2,
        "the capture is one header, the cell lines, and one end line"
    );
    // Each family's first cell line, plus the grid's last cell.
    let mut positions: Vec<usize> = Vec::new();
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for (position, line) in lines[1..=declared].iter().enumerate() {
        let family = line
            .split('\t')
            .nth(2)
            .expect("a cell line names its operation and family");
        if seen.insert(family) {
            positions.push(position);
        }
    }
    assert_eq!(
        seen.len(),
        expected_cells_per_family().len(),
        "the capture reaches every board family"
    );
    positions.push(declared - 1);
    for dropped in positions {
        // Cell lines sit between the header and the end line; drop one and
        // restate the count, leaving every per-shard invariant intact.
        let mut tampered_lines: Vec<&str> = Vec::with_capacity(lines.len() - 1);
        tampered_lines.push(lines[0]);
        tampered_lines.extend(
            lines[1..=declared]
                .iter()
                .enumerate()
                .filter(|(k, _)| *k != dropped)
                .map(|(_, line)| *line),
        );
        let end_line = format!("end {}", declared - 1);
        tampered_lines.push(&end_line);
        let mut tampered = tampered_lines.join("\n");
        tampered.push('\n');
        let tampered_capture = vec![tampered.into_bytes()];

        let family = lines[1 + dropped]
            .split('\t')
            .nth(2)
            .expect("a cell line names its operation and family");
        let refusal = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let spawn = move |_scale: f64| Ok(tampered_capture.clone());
            let mut rendered = Vec::new();
            board::run(SMOKE_SCALE, 1, &spawn, &mut rendered).expect("writing to a Vec succeeds");
        }))
        .expect_err("the merge must refuse a capture missing one cell");
        let message = refusal
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| refusal.downcast_ref::<&str>().map(|s| (*s).to_string()))
            .expect("the refusal panics with a message");
        assert!(
            message.contains(family),
            "the refusal must name the shorted family {family}: {message}"
        );
    }
}
