//! Process-global counter for arithmetic-core regression tests.
//!
//! Available with the `touch-meter` feature. The counter records bounded units
//! of accumulator arithmetic and input reading. Its exact denomination is an
//! implementation detail: it excludes allocation and some indexing work, so it
//! neither measures elapsed time nor independently establishes the public
//! complexity bounds. Counts are deterministic for a fixed implementation and
//! operation sequence, but individual totals are not an API compatibility
//! promise.
//!
//! Run measured scenarios serially: [`reset`] the counter, perform the
//! operations, then read [`touches`]. Relaxed atomic updates make counting
//! thread-safe, but concurrent scenarios contribute to the same total.

use core::sync::atomic::{AtomicU64, Ordering};

/// Process-wide total; callers isolate measured scenarios themselves.
static TOUCHES: AtomicU64 = AtomicU64::new(0);

/// Add `count` implementation work units to the counter.
pub(crate) fn record(count: u64) {
    TOUCHES.fetch_add(count, Ordering::Relaxed);
}

/// The work units recorded since process start or the last
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
