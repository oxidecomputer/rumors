//! Materialized protocol access to the shared named-channel infrastructure.

pub use super::super::channel::{QueueKind, QueueRole, Receiver, Sender, channel};

#[cfg(test)]
pub use super::super::channel::{
    ChannelReport, RoleStats, with_capacity_limit, with_kind_capacity, with_observation,
    with_role_capacity, with_schedule,
};
