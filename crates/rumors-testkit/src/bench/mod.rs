//! Models and fixtures shared by benchmarks and performance tests.

/// Fixtures spanning the reconciliation divergence grid.
pub mod grid;

/// An in-memory connection with deterministic simulated latency.
pub mod latency;

/// A duplex link with bandwidth, delay, and send buffers, on simulated time.
pub mod link;

/// Deterministic replica pairs reconciled over the link model, with what each
/// side's ledger admitted.
pub mod session;

/// A lower estimate of a session's duration at a budget below its
/// unlimited-budget peak, from the unlimited session.
pub mod slowdown;
