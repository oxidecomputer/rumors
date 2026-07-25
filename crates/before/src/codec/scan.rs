//! Process-global counter of packed-stream bits scanned and written.
//!
//! Traversal work over the packed bit streams is invisible to every other
//! meter when it allocates nothing (no heap delta), recurses through an
//! iterative loop (no grown segments), and touches no `Base` arithmetic (no
//! limb operations) — the id-side walks and folds are exactly that shape.
//! The proxy counted here is **bits**, at the packed-stream primitives:
//!
//! - id tag reads and skip steps (`idbits::IdReader`), 2 bits per node;
//! - id-builder bit writes and verbatim splice lengths
//!   (`party::ops`' builder);
//! - event topology cursor advances and gamma code-skips
//!   (the skyline sweeps' cursors over packed bits, `codec::skip_int`);
//! - every sequential decoder/validator bit read (`codec::SliceCursor`,
//!   which carries `decode`, the gamma decoder, and the skyline
//!   validator/decoder cursors).
//!
//! The wire-side `ReaderCursor` (`borsh_impls`) is deliberately unmetered:
//! no board row prices the borsh path today, so recording there would count
//! work nothing judges. Instrumenting it is a conscious future change that
//! carries its own envelope recalibration.
//!
//! An amortized-linear walk therefore counts O(1) bits per packed input or
//! output bit, and a fold that re-scans its accumulator counts
//! quadratically. Relaxed ordering suffices: the metering binaries run one
//! scenario per process and read the counter only after the metered call
//! returns.

#[cfg(feature = "scan-meter")]
mod counter {
    use core::sync::atomic::{AtomicU64, Ordering};

    static SCAN_BITS: AtomicU64 = AtomicU64::new(0);

    /// Add `n` scanned or written bits to the counter.
    pub(super) fn record(n: u64) {
        SCAN_BITS.fetch_add(n, Ordering::Relaxed);
    }

    /// The bits recorded since the last [`reset`].
    pub(crate) fn scan_bits() -> u64 {
        SCAN_BITS.load(Ordering::Relaxed)
    }

    /// Reset the counter to zero.
    pub(crate) fn reset() {
        SCAN_BITS.store(0, Ordering::Relaxed);
    }
}

#[cfg(feature = "scan-meter")]
pub(crate) use counter::{reset, scan_bits};

/// Record `n` packed-stream bits scanned or written.
///
/// Compiles to nothing without the `scan-meter` feature, so every primitive
/// can call it unconditionally.
#[inline(always)]
pub(crate) fn record_bits(n: usize) {
    #[cfg(feature = "scan-meter")]
    counter::record(n as u64);
    #[cfg(not(feature = "scan-meter"))]
    let _ = n;
}
