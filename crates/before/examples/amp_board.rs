//! Runs the amplification board and prints the red-green matrix to stdout.
//!
//! The board itself — the operation × input-family sweep, the meters, the
//! green/red criterion, and the not-applicable coverage list — lives in
//! `before::meter::board`; this binary only installs the counting allocator
//! the board's peak-heap column reads (a global allocator is per-binary
//! state the library cannot own) and parses the size knob.
//!
//! Usage: `just amp-board` (release, the profile of record — dev runs are
//! a debugging view whose readings are never pinned), or directly
//! `cargo run -p before --example amp_board --features limb-meter -- [scale]`
//! where the optional `scale` (a positive number, default 1) multiplies
//! every input family's base size; the literal `acceptance` selects the
//! acceptance scale (`board::ACCEPTANCE_SCALE`, `just
//! amp-board-acceptance`). The default sizes keep the whole board at
//! seconds of runtime; acceptance requires all green at both the default
//! and acceptance scales, one run each under the board's determinism
//! tripwire. Without the `limb-meter` feature the limb column reads
//! `off`.

use before::meter::board::{self, HeapMeter};
use peak_alloc::PeakAlloc;

#[global_allocator]
static HEAP: PeakAlloc = PeakAlloc;

/// The default size multiplier when no argument is given.
const DEFAULT_SCALE: f64 = 1.0;

fn main() {
    let scale = match std::env::args().nth(1) {
        None => DEFAULT_SCALE,
        Some(arg) if arg == "acceptance" => board::ACCEPTANCE_SCALE,
        Some(arg) => arg
            .parse::<f64>()
            .unwrap_or_else(|_| panic!("amp-board: scale must be a positive number, got {arg:?}")),
    };
    let heap = HeapMeter {
        reset_peak: || HEAP.reset_peak_usage(),
        peak: || HEAP.peak_usage(),
        current: || HEAP.current_usage(),
    };
    board::run(scale, &heap, &mut std::io::stdout().lock()).expect("stdout stays writable");
}
