//! Choosing a synchronization memory budget.
//!
//! [`Peer::sync_memory_budget`](crate::Peer::sync_memory_budget) trades
//! buffering against time spent waiting for replies. It lets a session
//! compare several parts of the set concurrently. This matters most when
//! both replicas have many differences to reconcile.
//!
//! # Choosing a budget
//!
//! Start with the memory available for synchronization and the number of
//! sessions you expect to run at once. Each session has its own budget;
//! allow room for the replica, observers, and network buffers as well.
//! The setter's [memory accounting](crate::Peer::sync_memory_budget)
//! explains the limits of this setting.
//!
//! If you can afford the default of 512 MiB per session, use it as a starting
//! point. Otherwise, lower it and measure synchronization time with typical
//! and unusually large differences between replicas. Raise it when waiting
//! for replies limits throughput and memory is available. A higher budget
//! has little benefit when the session already has enough work in flight,
//! or when processing speed limits progress.
//!
//! # Why the link and message size matter
//!
//! To use a link fully, a sender needs enough data in flight to cover the
//! wait for replies. The **bandwidth-delay product**, `BDP = bandwidth × RTT`,
//! expresses that amount in bytes. For example, 1 Gb/s with a 100 ms round
//! trip and 100 Gb/s with a 1 ms round trip both have a BDP of 12.5 MB.
//!
//! A link with a larger BDP generally needs more parallel work. Larger
//! messages can need fewer outstanding comparisons to keep it busy, since
//! each transferred message occupies the link for longer. The synchronization
//! budget pays for tracking that work; it is not itself the number of wire
//! bytes in flight. [`Peer::target_message_size`](crate::Peer::target_message_size)
//! controls wire buffering separately.
//!
//! # Example estimates
//!
//! The reference link reflects the default budget's tuning goal: enough
//! parallel work to keep a 1 Gb/s link busy across a 100 ms round trip, even
//! with small messages. The table below uses 100,000 messages per replica
//! as an example set size, and **100 encoded bytes** as a small, round
//! reference message.
//!
//! The table models two completely different, uniformly hashed sets of
//! that size on a link with a 12.5 MB BDP. The *window* counts
//! subtrees that can have comparisons outstanding at a tree level; the
//! *budget* covers the estimated buffering across all levels. `m` is the
//! mean CBOR-encoded message size in bytes, excluding protocol overhead.
//!
//! The window column is the calculation the library performs. The slowdown
//! cells are an idealized model, `max(1, BDP / ((45 + m) × window))`,
//! relative to uninterrupted transfer. The 45 bytes are an approximate
//! per-message share of hashes, versions, framing, and session setup;
//! this overhead varies with the set sizes, shared history, and message
//! sizes. The model has not been validated against measured sessions:
//! measured sessions can run slower than it predicts, even with a window
//! that covers one BDP of messages. Treat the cells as a comparison between
//! budgets, not a prediction of synchronization time.
//!
//! At this set size, the default budget grants a window covering the
//! example's whole population, so a larger budget cannot widen it further.
//! Different set sizes and shared history change the window and the work
//! that uses it. Use the table to understand the trade-off, then measure
//! your workload.
//!
#![doc = include_str!("tree/mirror/streaming/window/tradeoff.md")]
