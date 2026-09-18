//! Reconciliation-window sizing, progress, and latency behavior.
//!
//! These modules check the sizing model from its arithmetic census through
//! whole sessions: the configured budget stays bounded, floor windows still
//! make progress, wider windows pipeline ordinary hash-distributed disputes,
//! and measured serialization follows the model around its predicted knee.

/// Check the memory census and the budget-to-capacity solve.
#[path = "window/census.rs"]
mod census;

/// Exercise progress and boundedness at the configuration's boundary cases.
#[path = "window/corners.rs"]
mod corners;

/// Check the transition from pipelined to serialized sessions.
#[path = "window/knee.rs"]
mod knee;

/// Compare the slowdown model with measured sessions.
#[path = "window/operator.rs"]
mod operator;

/// Check that the default window pipelines an ordinary disputed frontier.
#[path = "window/pipelining.rs"]
mod pipelining;

/// Check the window choices used throughout generated protocol schedules.
#[path = "window/sweep.rs"]
mod sweep;

/// Provide the hand-run closed-form trade-off validation.
#[path = "window/tradeoff_probe.rs"]
mod tradeoff_probe;
