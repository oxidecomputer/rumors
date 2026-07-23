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

/// The board must have a cell for the bulk of the public operation surface:
/// a sweep that silently lost rows would pass a bare completion check.
const MIN_CELLS: usize = 100;

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
    assert!(
        cells >= MIN_CELLS,
        "the board swept only {cells} cells: rows have gone missing"
    );
    let text = String::from_utf8(rendered).expect("the board renders UTF-8");
    assert!(
        text.contains(&format!("({cells} cells)")),
        "the rendered summary line must agree with the returned summary"
    );
}
