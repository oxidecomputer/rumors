//! Deterministic simulation test suite for the `rumors` public API.
//!
//! Test modules, each pinning down one family of invariants:
//!
//! * `single_peer` — single-peer correctness for `Local`: callback
//!   fan-out, key distinctness, monotonic local versions.
//! * `pairwise` — pairwise `Local::process` semantics: convergence,
//!   symmetry, idempotence, the algebraic laws of `+`, and
//!   equivalence with `Remote::gossip` over a real `tokio::io::duplex`.
//! * `multi_peer` — multi-peer eventual consistency: after a
//!   randomised schedule and a full-mesh quiesce, every peer matches
//!   every other and matches the spec-shaped oracle.
//! * `redaction` — redaction-specific corners (contagion from any
//!   peer, order-independence of concurrent redactions).
//! * `partition` — partition tolerance: gossip restricted during a
//!   split then healed, asserting only self-consistency (see the
//!   module header for why we don't compare against an unpartitioned
//!   twin).
//! * `sanity` — arbitrary schedules don't panic, clones merge back
//!   cleanly, degenerate inputs to `quiesce` are no-ops.
//!
//! Shared infrastructure:
//!
//! * `oracle` — a pure-data reference state (no `Local`, no
//!   `process`) plus a `readout` lens that projects a `Local<T>`
//!   into its currently-live `(Key, T)` map.
//! * `peer` — a `Local<T>` paired with an observation log, plus
//!   `gossip_step` and `quiesce` helpers.
//! * `schedule` — proptest-generated schedules that are valid by
//!   construction (every `Redact` references a `Key` the redacting
//!   peer has already observed).
//! * `wire` — drives `Remote::gossip` over `tokio::io::duplex` on a
//!   shared current-thread runtime.

mod action;
mod oracle;
mod peer;
mod schedule;
mod sync_wire;
mod wire;

mod multi_peer;
mod pairwise;
mod partition;
mod redaction;
mod sanity;
mod shadow_validity;
mod single_peer;
