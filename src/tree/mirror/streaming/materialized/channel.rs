//! Materialized protocol access to the shared named-channel infrastructure.

pub use super::super::channel::{QueueKind, QueueRole, Receiver, Sender, channel};

#[cfg(test)]
pub use super::super::channel::{with_kind_capacity, with_observation, with_schedule};
