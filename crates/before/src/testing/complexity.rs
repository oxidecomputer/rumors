//! Step-count scaling for the complexity proptests.
//!
//! Each measured traversal is wrapped in [`steps_of`] (a fresh
//! [`super::metrics`] counter), and the two results — at input sizes whose node
//! counts differ by `4×` — are fed to [`assert_linear_scaling`], which fails if
//! step growth looks quadratic rather than linear.

use super::metrics;

/// Smallest spine scale a complexity proptest measures at; below this the step
/// count is too noisy for the ratio to be meaningful. The big input is always
/// `4×` this (see [`assert_linear_scaling`]).
pub(crate) const MIN_SCALE: usize = 64;

/// Steps taken by `f`, measured on a fresh traversal-step counter. The
/// complexity proptests wrap each measured traversal in this and feed the two
/// results to [`assert_linear_scaling`].
pub(crate) fn steps_of(f: impl FnOnce()) -> u64 {
    metrics::reset();
    f();
    metrics::taken()
}

/// Assert that `steps`, measured at two input sizes whose node counts differ by
/// `4×`, grows roughly linearly rather than quadratically. Linear predicts
/// `~4×` more steps; quadratic predicts `~16×`. The `6×` threshold sits
/// comfortably between, independent of any constant factor.
pub(crate) fn assert_linear_scaling(small_steps: u64, big_steps: u64) {
    assert!(
        big_steps <= 6 * small_steps,
        "steps grew super-linearly: {small_steps} -> {big_steps} for a 4x larger input \
         (linear would be ~4x; this is {:.1}x)",
        big_steps as f64 / small_steps.max(1) as f64,
    );
}
