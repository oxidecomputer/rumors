//! Smoke coverage for the amplification board (`before::meter::board`).
//!
//! The board is the campaign's dashboard, not its enforcement: this test
//! only pins that the whole sweep keeps compiling and running — every
//! operation row prepares, measures at both scales, and renders. It
//! deliberately asserts no colors: red cells are expected while amplifiers
//! remain, and the enforced resource record is the process-isolated
//! envelope suite in `tests/meter.rs`.

use before::meter::board::{self, HeapMeter};
use peak_alloc::PeakAlloc;

#[global_allocator]
static HEAP: PeakAlloc = PeakAlloc;

/// A fraction of the board's default sizes, small enough that the smoke run
/// stays well under a second.
const SMOKE_SCALE: f64 = 0.02;

/// The board's exact cell count: 66 operation rows over 21 shapes (1386
/// combinations) minus the 275 where the shape's bundle supplies no
/// operand for the row's signature.
///
/// Derived per shape from the operand bundles. A version-only shape
/// (dense, bigroot, hugeleaf, cliff, harmonic, and the two version-pair
/// shapes jump-pair and concurrent-pair, whose bundles carry their own
/// comparison counterpart in the same slots) runs the 13 version rows
/// (8 of them consuming the pair slot),
/// the 4 linear-functional rows, the 2 rank rows, its tick, ticks,
/// and 3 projection cells (the materialization row and the two fused
/// comparison rows), the 11 clock rows, and its 8 rejection rows (the
/// 5 version rejections and the 3 clock rejections): 43 each. The id pair
/// (parties only) runs the 10 party rows, the adversarial-party tick
/// row, its 3 projection cells, the 11 clock rows, and its 13 rejection
/// rows (5 party rejections, `party_without_none`, 3 clock rejections,
/// and the 4 overlap rows its id source mints): 38. The eleven cross
/// shapes (comb-scatter and the ten tick-walk crosses: nested-full,
/// nested-wide, mirror-wide, mirror-narrow, staircase, reveal-comb,
/// reveal-hifloor, pure-comb, ascend-cliff, ascend-plateau) carry a
/// version, a mounted party pair, and a clock, so each runs the
/// version-only 43 plus the adversarial-party tick row, the 10 party
/// rows, and the 10 id-side rejection rows: 64 each. Scatter runs its 2
/// fold rows (its bundle carries fold operands alone, so neither a
/// projection nor a rejection row applies); benign runs every row (66,
/// both fold controls included).
/// 43 x 7 + 38 + 64 x 11 + 2 + 66 = 1111.
/// The table is fixed and applicability depends on the
/// family alone (`board::run` enforces this per cell), so the count is
/// deterministic at every scale; a row added to or dropped from the table
/// must move this pin.
const EXPECTED_CELLS: usize = 1111;

/// The board runs to completion at tiny sizes: every cell prepares,
/// measures, and renders, and the matrix keeps covering the full operation
/// sweep (colors are deliberately not asserted; the board is a dashboard,
/// not a gate).
#[test]
fn board_runs_to_completion() {
    let heap = HeapMeter {
        reset_peak: || HEAP.reset_peak_usage(),
        peak: || HEAP.peak_usage(),
        current: || HEAP.current_usage(),
    };
    let mut rendered = Vec::new();
    let summary = board::run(SMOKE_SCALE, &heap, &mut rendered).expect("writing to a Vec succeeds");
    let cells = summary.green + summary.red;
    assert_eq!(
        cells, EXPECTED_CELLS,
        "the board swept {cells} cells, not the pinned {EXPECTED_CELLS}: \
         rows were added or lost without moving the pin"
    );
    let text = String::from_utf8(rendered).expect("the board renders UTF-8");
    assert!(
        text.contains(&format!("({cells} cells)")),
        "the rendered summary line must agree with the returned summary"
    );
}
