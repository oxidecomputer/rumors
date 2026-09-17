//! Process-global count of encoded bits read and written.
//!
//! This meter observes traversal that performs no allocation or arithmetic.
//! Recording happens at the primitive bit reads, writes, and skips, independent
//! of whether an implementation processes one bit or a machine word at a time.
//! Relaxed ordering suffices because each measurement runs in one process and
//! reads the counter after the operation completes.

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

/// Record `n` encoded bits read or written.
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

/// Record a count already expressed at the meter's `u64` width.
#[inline(always)]
pub(crate) fn record_bits_u64(n: u64) {
    #[cfg(feature = "scan-meter")]
    counter::record(n);
    #[cfg(not(feature = "scan-meter"))]
    let _ = n;
}
