//! Test-only traversal step counter.
//!
//! Every node-header read calls
//! [`step!`](crate::step); the complexity tests reset it, run one operation,
//! and assert the count is `O(n + m)` — proving no traversal re-scans (which
//! would be `O(n²)` on a deep spine). A deterministic stand-in for wall-clock
//! timing, which would be flaky.
//!
//! This module is `cfg(test)` only; the [`step!`](crate::step) macro that feeds
//! it lives at the crate root (it needs a no-op `cfg(not(test))` twin and the
//! `crate::step` path), so production traversals pay zero cost.

use std::cell::Cell;

thread_local! {
    static STEPS: Cell<u64> = const { Cell::new(0) };
}

/// Reset the step counter to zero.
pub(crate) fn reset() {
    STEPS.with(|c| c.set(0));
}

/// The number of traversal steps recorded since the last [`reset`].
pub(crate) fn taken() -> u64 {
    STEPS.with(|c| c.get())
}

/// Record one traversal step (one node-header read).
pub(crate) fn bump() {
    STEPS.with(|c| c.set(c.get() + 1));
}
