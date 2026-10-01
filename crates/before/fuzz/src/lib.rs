//! Deterministic bodies shared by the libFuzzer entry points and seed replay.
//!
//! Coverage-guided runs supply arbitrary bytes to [`targets`]. Ordinary tests
//! replay every committed seed through these same functions, so the target
//! assertions and input framing cannot compile successfully while remaining
//! unexecuted until a manual fuzzing session.
//!
//! [`under_heap_cap`] also turns a survivable allocation spike above 1 GiB into
//! a crash that libFuzzer can minimize. It is an absolute emergency bound, not
//! evidence for the library's proportional auxiliary-space contracts; the
//! resource board and fuzz-fit enforce those contracts directly.

use peak_alloc::PeakAlloc;

#[path = "../../tests/support/fuzz_input.rs"]
pub mod input;
pub mod targets;

/// Process-wide allocation tracker used by each single-threaded fuzz binary.
#[global_allocator]
static HEAP: PeakAlloc = PeakAlloc;

/// Maximum transient heap growth allowed while one fuzz input runs.
pub const PEAK_HEAP_CAP_BYTES: usize = 1 << 30;

/// Run one fuzz input and fail if it grows the live heap by more than 1 GiB.
///
/// The baseline excludes libFuzzer's resident corpus and process-lifetime
/// allocations. This measures only growth above what was live when the input
/// began.
///
/// # Panics
///
/// Panics when peak growth exceeds [`PEAK_HEAP_CAP_BYTES`].
pub fn under_heap_cap<R>(body: impl FnOnce() -> R) -> R {
    under_heap_cap_at(PEAK_HEAP_CAP_BYTES, body)
}

/// Apply an explicit cap, allowing a small allocation to test the instrument.
fn under_heap_cap_at<R>(cap: usize, body: impl FnOnce() -> R) -> R {
    let baseline = HEAP.current_usage();
    HEAP.reset_peak_usage();
    let result = body();
    let growth = HEAP.peak_usage().saturating_sub(baseline);
    assert!(
        growth <= cap,
        "before-fuzz: transient heap grew by {growth} B, exceeding the {cap} B cap"
    );
    result
}

#[cfg(test)]
mod tests;
