//! Process-global counter of accumulator digit touches.
//!
//! Present only with the `touch-meter` cargo feature. Counts one per
//! digit read-modify-write in [`Accumulator`](crate::Accumulator)'s own
//! code (a sign-fold step counts one touch per digit read plus one per
//! digit its collapse zeroes; a top-settlement scan counts one per zero
//! digit it steps past, and one — total — per certified zero run it
//! skips whole; a wide operation adds one per operand limb read): the
//! unit every cost on the crate page is denominated in. Because the
//! counter is process-global with relaxed ordering, readings are
//! meaningful only when metered scenarios run serially — [`reset`]
//! between them, read after the metered call returns; a default-parallel
//! test runner interleaves scenarios into one count.

use core::sync::atomic::{AtomicU64, Ordering};

static TOUCHES: AtomicU64 = AtomicU64::new(0);

/// Add `n` digit touches to the counter.
pub(crate) fn record(n: u64) {
    TOUCHES.fetch_add(n, Ordering::Relaxed);
}

/// The digit touches recorded since process start or the last
/// [`reset`], whichever is later.
///
/// # Complexity
///
/// `O(1)`.
pub fn touches() -> u64 {
    TOUCHES.load(Ordering::Relaxed)
}

/// Reset the counter to zero.
///
/// # Complexity
///
/// `O(1)`.
pub fn reset() {
    TOUCHES.store(0, Ordering::Relaxed);
}
