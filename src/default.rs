//! The library's tuning defaults, grouped by the deployment they suit.
//!
//! A *tuning default* is the value a peer uses for a setting you never
//! set. A default that suits every deployment sits directly in this
//! module. A default that assumes a particular deployment, such as a
//! transport or a link, sits in a submodule named for it, whose docs say
//! what it assumes and how to adjust the setting for other deployments.
//! Each setter's docs cite its default here.
//!
//! Limits and format versions are not tuning defaults, and live at the
//! crate root: a limit, such as
//! [`MIN_SYNC_MEMORY_BUDGET`](crate::MIN_SYNC_MEMORY_BUDGET), bounds what a
//! setting can be, rather than standing in for a setting you leave alone.

pub mod tls;

/// The memory budget for each synchronization session a peer uses unless
/// you set one: 512 MiB.
///
/// See [`Peer::sync_memory_budget`](crate::Peer::sync_memory_budget) for
/// what the budget covers and [the sizing guide](crate::sizing) for choosing
/// one.
pub const SYNC_MEMORY_BUDGET: usize = 512 * 1024 * 1024;

/// The message-size target a peer uses unless you set one
/// ([`Peer::target_message_size`](crate::Peer::target_message_size)):
/// 256 KiB.
///
/// The target bounds each message of supplied records a session builds,
/// and sizes the room every memory budget sets aside for such messages on
/// their way through the connection: about two messages' worth in each
/// direction, about 1 MiB at this target. So a larger default would raise
/// [`MIN_SYNC_MEMORY_BUDGET`](crate::MIN_SYNC_MEMORY_BUDGET) by about four
/// bytes per byte, and a smaller one would lower it.
///
/// A supply message carries records of one subtree a session supplies
/// whole, and batching them into fewer, larger messages saves processor
/// time for each message it removes, on both sides. By this size the
/// saving is spent. Measured in process over TLS
/// (`rumors-netperf duplex`), a peer joining 200,000 messages of 200 bytes
/// sends each supplied subtree, about 170 KB, as one message; one joining
/// 20,000 messages of 4 KiB spends no measurably more processor time than
/// at a target nearly six times larger, while a 64 KiB target costs it
/// about a tenth more. Synchronizations between replicas that differ in
/// scattered messages supply small subtrees, which a target this size
/// leaves whole.
pub const TARGET_MESSAGE_SIZE: usize = 256 * 1024;

/// The payload nesting-depth limit a peer uses unless you set one
/// ([`Peer::payload_depth_limit`](crate::Peer::payload_depth_limit)): 256
/// decode recursion steps.
///
/// This is the decoder's default recursion bound, so the depth check rejects
/// nothing the decoder would otherwise accept.
///
/// Wire interop across releases is governed by the greeting's format,
/// not by this constant.
pub const PAYLOAD_DEPTH_LIMIT: u64 = 256;

/// The largest encoded bookmark a peer accepts unless you set a limit
/// ([`Peer::bookmark_size_limit`](crate::Peer::bookmark_size_limit)):
/// 16 MiB, allocated only as needed.
pub const BOOKMARK_SIZE_LIMIT: usize = 16 * 1024 * 1024;
