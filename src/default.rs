//! Default configuration parameters, grouped by deployment context.
//!
//! These settings are used internally to tune the resource utilization and
//! performance of `rumors`. It is usually not required to override them, but
//! the documentation on each one's setter describes when this might be
//! advantageous.

pub mod tls;

/// The default [`PeerBuilder::sync_memory_budget`](crate::PeerBuilder::sync_memory_budget).
///
/// See [the budget sizing guidance](crate::sizing) for details about when and
/// how to vary from this default.
pub const SYNC_MEMORY_BUDGET: usize = 512 * 1024 * 1024;

/// The default [`PeerBuilder::target_message_size`](crate::PeerBuilder::target_message_size).
pub const TARGET_MESSAGE_SIZE: usize = 256 * 1024;

/// The default [`PeerBuilder::message_depth_limit`](crate::PeerBuilder::message_depth_limit).
pub const MESSAGE_DEPTH_LIMIT: u64 = 256;

/// The default [`PeerBuilder::bookmark_size_limit`](crate::PeerBuilder::bookmark_size_limit).
pub const BOOKMARK_SIZE_LIMIT: usize = 16 * 1024 * 1024;
