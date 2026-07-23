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

/// The board's exact cell count: 42 operation rows over 6 families (252
/// combinations) minus the 92 where the family provides no operand.
///
/// The exclusions: the 19 version rows needing a packed version skip the
/// id-pair family (19), the 10 party rows plus the adversarial-party tick
/// row skip the three event-only families (11 x 3 = 33), and every row but
/// the two projection cells (`version_project`, `clock_own_version`) skips
/// the comb-scatter cross family (40). The table is fixed and applicability
/// depends on the family alone (`board::run` enforces this per cell), so
/// the count is deterministic at every scale; a row added to or dropped
/// from the table must move this pin.
const EXPECTED_CELLS: usize = 160;

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
