//! Proptest-generated multi-peer schedules and the executor that runs
//! them against both the live simulation and the spec-shaped oracle.
//!
//! The module separates a schedule's data model, the strategies that
//! generate schedules, and the executors that run them, each in a submodule
//! of its own; this module re-exports the items tests use.
//!
//! When a strategy chooses an index into a fixed fleet, derive its range with
//! `prop_flat_map` so shrinking preserves the index's meaning. Executors may
//! resolve an arbitrary seed modulo the current fleet only when membership
//! changes while the schedule runs.

pub mod arb;
pub mod events;
pub mod executor;

pub use arb::{
    arb_membership_schedule, arb_membership_schedule_with_shadow, arb_schedule,
    arb_schedule_with_shadow,
};
pub use events::{EventIdx, Schedule};
pub use executor::{
    execute_and_quiesce, execute_membership, execute_membership_and_quiesce, execute_with,
};
