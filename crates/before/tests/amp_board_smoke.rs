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

/// The board's exact cell count: 46 operation rows over 19 shapes (874
/// combinations) minus the 154 where the shape's bundle supplies no
/// operand for the row's signature.
///
/// Derived per shape from the operand bundles. A version-only shape
/// (dense, bigroot, hugeleaf, cliff, harmonic) runs the 18 version-pair
/// rows, the 4 linear-functional rows, the 2 rank rows, its tick and
/// projection cells, and the 11 clock rows: 33 each. The id pair
/// (parties only) runs the 10 party rows, the adversarial-party tick
/// row, its projection cell, and the 11 clock rows: 23. The eleven cross
/// shapes (comb-scatter and the ten tick-walk crosses: nested-full,
/// nested-wide, mirror-wide, mirror-narrow, staircase, reveal-comb,
/// reveal-hifloor, pure-comb, ascend-cliff, ascend-plateau) carry a
/// version, a mounted party pair, and a clock, so each runs the
/// version-only 33 plus the adversarial-party tick row and the 10 party
/// rows: 44 each. Scatter runs its 2 fold rows; benign runs every row
/// (46, both fold controls included).
/// 33 x 5 + 23 + 44 x 11 + 2 + 46 = 720.
/// The table is fixed and applicability depends on the
/// family alone (`board::run` enforces this per cell), so the count is
/// deterministic at every scale; a row added to or dropped from the table
/// must move this pin.
const EXPECTED_CELLS: usize = 720;

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
