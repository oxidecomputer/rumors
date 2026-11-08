//! The sync memory budget's progress and pipelining in whole sessions.
//!
//! These modules check that a session completes at every budget, zero included,
//! and that the default budget pipelines an ordinary hash-distributed frontier
//! of disputes.

/// Exercise progress at the budget's boundary cases.
#[path = "budget/corners.rs"]
mod corners;

/// Check that the default budget pipelines an ordinary disputed frontier.
#[path = "budget/pipelining.rs"]
mod pipelining;
