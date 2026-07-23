//! Shared harness for the fuzz targets: every input runs under a heap cap.
//!
//! The targets' assertions catch wrong *answers*; this harness catches wrong
//! *costs*. A resource amplifier — an input whose computation materializes
//! transient state grossly disproportionate to its packed size — produces no
//! wrong answer, so without a ceiling it stays latent. Running every input
//! through [`under_heap_cap`] turns one into an ordinary crash finding: the
//! fuzzer minimizes and archives the offending input like any panic.
//!
//! The ceiling is generous and absolute: a flat [`PEAK_HEAP_CAP_BYTES`],
//! not yet proportional to input size. The known amplifiers are linear in
//! the input with constants in the hundreds, and libFuzzer's default 4096-byte
//! inputs keep them megabytes below the cap, so a trip means a new class of
//! blowup, not a bigger constant. The peak is read after the body returns
//! rather than enforced inside the allocator, so a spike that outruns the
//! process before returning is stopped by libFuzzer's RSS limit instead; the
//! cap's job is the far more common survivable amplification.

use peak_alloc::PeakAlloc;

/// The binary-wide peak-tracking allocator the cap reads.
///
/// One global allocator exists per fuzz binary, and libFuzzer drives inputs
/// through it sequentially, so a per-input reset-then-read is exact.
#[global_allocator]
static HEAP: PeakAlloc = PeakAlloc;

/// Hard ceiling on one input's peak transient heap: 1 GiB.
pub const PEAK_HEAP_CAP_BYTES: usize = 1 << 30;

/// Run one fuzz input's body and panic if its peak heap exceeded the cap.
///
/// # Panics
///
/// Panics — a deliberate, distinguishable crash finding — when the body's
/// peak heap usage exceeds [`PEAK_HEAP_CAP_BYTES`].
pub fn under_heap_cap<R>(body: impl FnOnce() -> R) -> R {
    HEAP.reset_peak_usage();
    let r = body();
    let peak = HEAP.peak_usage();
    assert!(
        peak <= PEAK_HEAP_CAP_BYTES,
        "before-fuzz: peak heap {peak} B exceeds the {PEAK_HEAP_CAP_BYTES} B cap: \
         resource amplification finding"
    );
    r
}
