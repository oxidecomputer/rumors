//! Reconciliation-window sizing, progress, and latency behavior.
//!
//! These modules check the sizing model from its arithmetic census through
//! whole sessions: the configured budget stays bounded, sessions at the
//! floor still make progress, and the default budget pipelines ordinary
//! hash-distributed disputes.

/// Check the memory census and the budget-to-capacity solve.
#[path = "window/census.rs"]
mod census;

/// Exercise progress and boundedness at the configuration's boundary cases.
#[path = "window/corners.rs"]
mod corners;

/// Check that the default window pipelines an ordinary disputed frontier.
#[path = "window/pipelining.rs"]
mod pipelining;

/// Check the window choices used throughout generated protocol schedules.
#[path = "window/sweep.rs"]
mod sweep;
