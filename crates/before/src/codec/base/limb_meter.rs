//! Process-global counters of big-integer limb-scale work.
//!
//! Arithmetic-width cost is invisible to every other meter: a magnitude blowup
//! performs no extra allocations a peak-heap meter would see and scans no extra
//! stream bits the scan meter would see — the work is wider, not more frequent.
//! The proxy counted here is the operands' 64-bit limb counts per `Base`
//! operation — arithmetic, comparison, equality, and hashing all record before
//! they run, and the wide-gamma decode in `codec::gamma` records one
//! value-width count per decoded value — so amortized-linear algorithms count
//! linearly in packed input bits and magnitude-quadratic ones count
//! quadratically. The denomination is the value's width in 64-bit limbs,
//! not any particular storage: the rank numerator's wide arm
//! (`version::rank::num`, magnitudes past the backend's capacity on
//! 32-bit targets) records its operations' operand and materialization
//! widths into this same counter under the same unit, so limb-denominated
//! envelopes read continuously across that arm seam. Relaxed ordering
//! suffices: the metering binaries run one
//! scenario per process and read the counters only after the metered call
//! returns.
//!
//! A second column ([`record_densified`]) counts the query folds' densified
//! cluster images by their zero-filled capacity. That fill is width-scale
//! work the operand-width proxy cannot see — a zeroed byte no digit lands on
//! enters no operand width — and it is memory fill, not `Base` arithmetic, so
//! folding it into the limb count would blend two mechanisms into one
//! reading; the separate column keeps each mechanism priced by its own
//! ceilings.

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

static DENSIFIED_DIGITS: AtomicU64 = AtomicU64::new(0);

/// Add `n` base-2^32 digits of zero-filled densified-image capacity.
pub(crate) fn record_densified(n: u64) {
    DENSIFIED_DIGITS.fetch_add(n, Ordering::Relaxed);
}

/// The densified-image digits recorded since the last [`reset_densified`].
pub(crate) fn densified_digits() -> u64 {
    DENSIFIED_DIGITS.load(Ordering::Relaxed)
}

/// Reset the densified-image counter to zero.
pub(crate) fn reset_densified() {
    DENSIFIED_DIGITS.store(0, Ordering::Relaxed);
}
