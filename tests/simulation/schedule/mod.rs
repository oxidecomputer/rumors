//! Proptest-generated multi-peer schedules and the executor that runs
//! them against both the live simulation and the spec-shaped oracle.
//!
//! Split into three submodules with a single responsibility each:
//!
//! * [`events`] — the `Event<T>`, `Schedule<T>`, and `EventIdx` data
//!   model.
//! * [`arb`] — `arb_schedule` and the shadow simulator backing it.
//! * [`executor`] — `execute`, `execute_and_quiesce`, and the
//!   gossip-filterable `execute_with` primitive used by the
//!   partition tests.

pub mod arb;
pub mod events;
pub mod executor;

pub use arb::{arb_schedule, arb_schedule_with_shadow};
pub use events::{EventIdx, Schedule};
pub use executor::{execute_and_quiesce, execute_with};
