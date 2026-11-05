//! Session throughput over a link with bandwidth and delay, on simulated
//! time.
//!
//! Each test reconciles deterministic replica pairs over the
//! [`rumors_testkit::bench::link`] model, whose clock counts byte-times and
//! advances only when both peers wait on the link. A session's duration is
//! therefore an exact function of its fixture and link, unmoved by machine
//! load, and its verdicts compare durations computed in one unit.
//!
//! - [`model`] checks the link model itself, so the session suites can
//!   trust its timing.
//! - [`no_slowdown`] checks that a budget of the session's unlimited-budget
//!   peak leaves the session exactly as it runs with no budget.
//! - [`send_buffer`] checks that bytes queued in the sending kernel do not
//!   slow a session.

/// The link model's own timing and delivery.
#[path = "throughput/model.rs"]
mod model;

/// A budget of the unlimited-budget peak against no budget at all.
#[path = "throughput/no_slowdown.rs"]
mod no_slowdown;

/// Unsent bytes in the sending kernel against a session's duration.
#[path = "throughput/send_buffer.rs"]
mod send_buffer;
