//! Process-global counter of big-integer limb-scale work.
//!
//! Arithmetic-width cost is invisible to every other meter: a magnitude blowup
//! performs no extra allocations a peak-heap meter would see and scans no extra
//! stream bits the scan meter would see — the work is wider, not more frequent.
//! The proxy counted here is the operands' 64-bit limb counts per `Base`
//! operation — arithmetic, comparison, equality, and hashing all record before
//! they run, and the wide-gamma decode in `codec::gamma` records one
//! value-width count per decoded value — so amortized-linear algorithms count
//! linearly in packed input bits and magnitude-quadratic ones count
//! quadratically. Relaxed ordering suffices: the metering binaries run one
//! scenario per process and read the counter only after the metered call
//! returns.

use core::sync::atomic::{AtomicU64, Ordering};

static LIMB_OPS: AtomicU64 = AtomicU64::new(0);

/// Add `n` operand limbs to the counter.
pub(crate) fn record(n: u64) {
    LIMB_OPS.fetch_add(n, Ordering::Relaxed);
}

/// Record the limb width of a raw `UBig` working value.
pub(crate) fn record_wide(n: &dashu_int::UBig) {
    use dashu_int::ops::BitTest;
    record((n.bit_len() as u64).div_ceil(64).max(1));
}

/// The limb operations recorded since the last [`reset`].
pub(crate) fn limb_ops() -> u64 {
    LIMB_OPS.load(Ordering::Relaxed)
}

/// Reset the counter to zero.
pub(crate) fn reset() {
    LIMB_OPS.store(0, Ordering::Relaxed);
}
