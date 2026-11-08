//! Shared infrastructure for integration tests.
//!
//! The fixtures stack in layers. At the bottom, every test network starts from
//! a reproducible seed, and every session runs between real peers over a real
//! connection, either an in-memory pipe pair or a loopback TCP socket. The wire
//! helpers that drive a session over such a connection sit beneath every driver
//! above them.
//!
//! The drivers differ in how much of a run's order they fix. A deterministic
//! driver executes a generated multi-peer schedule one session at a time and
//! compares every peer with an oracle that computes the expected converged set
//! from plain maps. A concurrent driver runs a whole plan at once on a
//! multi-thread runtime, with faults injected into the transport, and probes
//! the global invariants as it goes. A stepping driver advances a session round
//! by round, so that a test can change peers between rounds.
//!
//! Generators supply the workloads the drivers run. A budget generator assigns
//! each peer a memory budget, which the drivers that sweep the budget thread
//! into every peer they create. Fault fixtures make a transport or a bookmark
//! store fail on a schedule. Measurement fixtures record what crossed the wire,
//! as per-direction byte counts or as exact captures for the `insta` snapshots.
//! Each module's documentation states its own contract.

pub mod action;
pub mod budget;
pub mod count;
pub mod fault;
pub mod flaky;
pub mod gossip_snapshot;
pub mod observer;
pub mod oracle;
pub mod overlap;
pub mod peer;
pub mod property;
pub mod schedule;
pub mod seed;
pub mod shape;
pub mod sim;
pub mod tcp;
pub mod wire;
