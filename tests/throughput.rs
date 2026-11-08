//! Session throughput over a link with bandwidth and delay, on simulated time.
//!
//! Each test reconciles deterministic replica pairs over the
//! [`rumors_testkit::bench::link`] model, whose clock counts byte-times and
//! advances only when both peers wait on the link. On one platform, a session's
//! duration is therefore an exact function of its fixture and link, unmoved by
//! machine load, and its verdicts compare durations computed in one unit. The
//! link model's own suite checks that timing, so that the session suites can
//! trust it; each module's docs state the claim it checks.

/// The link model's own timing and delivery.
#[path = "throughput/model.rs"]
mod model;

/// A budget at the no-cost point against no budget at all.
#[path = "throughput/no_slowdown.rs"]
mod no_slowdown;

/// The exact properties of a session across a ladder of budgets.
#[path = "throughput/budget_sweep.rs"]
mod budget_sweep;

/// The criterion a duration's movement across budgets must meet.
#[path = "throughput/budget_criterion.rs"]
mod budget_criterion;

/// Unsent bytes in the sending kernel against a session's duration.
#[path = "throughput/send_buffer.rs"]
mod send_buffer;
