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

/// Deterministic pairs and the session runner.
#[path = "throughput/fixture.rs"]
mod fixture;

/// The link model's own timing and delivery.
#[path = "throughput/model.rs"]
mod model;
