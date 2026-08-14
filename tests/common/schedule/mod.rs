//! Proptest-generated multi-peer schedules and the executor that runs
//! them against both the live simulation and the spec-shaped oracle.
//!
//! Split into three submodules with a single responsibility each:
//!
//! * [`events`] — the `Event<T>`, `Schedule<T>`, and `EventIdx` data
//!   model, membership events (mid-schedule bootstrap, retire) included.
//! * [`arb`] — `arb_schedule` (and its membership-alphabet sibling
//!   `arb_membership_schedule`) and the shadow simulator backing them.
//! * [`executor`] — `execute`, `execute_and_quiesce`, the
//!   gossip-filterable `execute_with` primitive used by the
//!   partition tests, and the membership entry points over a slotted
//!   fleet.

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
