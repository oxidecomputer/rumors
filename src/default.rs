//! Default configuration parameters, grouped by deployment context.
//!
//! These settings are used internally to tune the resource utilization and
//! performance of `rumors`. It is usually not required to override them, but
//! the documentation on each one's setter describes when this might be
//! advantageous.

pub mod tls;

/// The default [`Peer::sync_memory_budget`](crate::Peer::sync_memory_budget).
///
/// See [the budget sizing guidance](crate::sizing) for details about when and
/// how to vary from this default.
pub const SYNC_MEMORY_BUDGET: usize = 512 * 1024 * 1024;

/// The default [`Peer::target_message_size`](crate::Peer::target_message_size).
pub const TARGET_MESSAGE_SIZE: usize = 256 * 1024;

/// The default [`Peer::payload_depth_limit`](crate::Peer::payload_depth_limit).
pub const PAYLOAD_DEPTH_LIMIT: u64 = 256;

/// The default [`Peer::bookmark_size_limit`](crate::Peer::bookmark_size_limit).
pub const BOOKMARK_SIZE_LIMIT: usize = 16 * 1024 * 1024;
