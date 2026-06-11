//! Unordered gossip with redaction.
//!
//! `rumors` replicates a set of messages across a fleet of peers with no
//! coordinator and no reliable connectivity: every peer holds a full
//! replica, changes it locally without asking anyone, and reconciles
//! pairwise with whichever peer it can reach next. Replicas that gossip
//! converge on the same set no matter the order, pairing, or repetition of
//! their sessions, and a session is priced by divergence, not by history:
//! bytes on the wire scale with the *difference* between the two replicas,
//! round trips are bounded by the content trie's fixed depth, and neither
//! side rescans what the two already share. (The internal protocol docs
//! quantify each axis; see [Internals](#internals).)
//!
//! Reach for it when shared state must survive partition and peer churn,
//! and when deleting an entry has to actually delete it:
//!
//! - The set is **unordered**: no leader, no quorum, no sequence numbers,
//!   so any two peers that can reach each other make progress alone.
//!   Causality is still tracked — every message carries a [`Version`] —
//!   and can be imposed at the observer when a consumer needs it.
//! - **Redaction is real deletion.** A redacted message is gone, not
//!   masked: replicas spend no memory or bandwidth remembering it, yet
//!   gossip still tells "deleted here" apart from "never arrived" on every
//!   replica. Use it for retraction, expiry, and data that must not
//!   outlive its purpose.
//!
//! # Should you use it?
//!
//! No, if any of these hold:
//!
//! - **The set outgrows its smallest peer.** Every peer replicates the
//!   whole set, in memory. Sharding, spill-to-disk, or unbounded growth
//!   call for a database instead.
//! - **You need an ordered, durable history.** A replicated log gives you
//!   sequence, replay, and audit; `rumors` deliberately has none of them.
//! - **You don't control the peers.** Peers in one universe trust one
//!   another: the protocol rejects malformed and mismatched sessions, but
//!   it is not Byzantine-tolerant — a compromised member can fabricate and
//!   redact at will. Authenticating peers and securing the transport are
//!   the application's job; run sessions only over channels you already
//!   trust.
//!
//! # The shape of the API
//!
//! One replica has two faces. [`Peer`] is the unique `!Clone` anchor,
//! present only at the edges of a replica's life, where *identity* moves:
//! minting a universe ([`Peer::seed`]), joining one through any connected
//! member ([`Peer::bootstrap`]), leaving it ([`Peer::retire`]). Trading
//! the anchor away ([`Peer::into_rumors`]) opens the working state:
//! [`Rumors`] clones freely, and clones send, redact, observe, and gossip
//! concurrently. When the clones are gone, [`Rumors::try_into_peer`]
//! recovers the anchor. The split is what lets the compiler — rather than
//! a runtime check — guarantee that identity moves only while nothing else
//! is touching the replica.
//!
//! Day to day:
//!
//! - [`Rumors::send`] and [`Rumors::redact`] change the set;
//!   [`Rumors::batch`] groups changes into one atomic commit ([`Batch`]).
//!   Messages are any `T` serializable with [`borsh`] (re-exported, so
//!   application and crate agree on its version), and every send mints a
//!   distinct [`Key`] — even for equal bytes — so a redaction targets one
//!   occurrence, never a set of values.
//! - [`Rumors::gossip`] runs one reconciliation session over any
//!   [`AsyncRead`](tokio::io::AsyncRead) /
//!   [`AsyncWrite`](tokio::io::AsyncWrite) pair. `rumors` never opens
//!   connections, spawns tasks, or sets timers: transport, scheduling, and
//!   who talks to whom are the application's.
//! - [`Rumors::messages`], [`Rumors::causal_messages`], and
//!   [`Rumors::snapshot`] observe the set; see
//!   [below](#which-observer-should-you-use).
//!
//! The [`Peer`] docs walk the full lifecycle as one runnable example,
//! including every retirement outcome and bootstrapping a universe without
//! a distinguished first peer.
//!
//! # Example
//!
//! Two peers, one universe, one message, one gossip session:
//!
//! ```
//! use rumors::Peer;
//!
//! # tokio::runtime::Builder::new_current_thread()
//! #     .build()
//! #     .unwrap()
//! #     .block_on(async {
//! // The universe's first peer mints it; every later peer bootstraps in.
//! let alice = Peer::<String>::seed().into_rumors();
//!
//! // A bare `send` statement commits when its `Batch` drops, right here.
//! alice.send("the meeting is at noon".to_string());
//!
//! // Any AsyncRead/AsyncWrite pair carries a session; here, an in-memory
//! // duplex. Alice serves one gossip session...
//! let (near, far) = tokio::io::duplex(64 * 1024);
//! let serve = alice.clone();
//! tokio::spawn(async move {
//!     let (mut read, mut write) = tokio::io::split(far);
//!     serve.gossip(&mut read, &mut write).await.unwrap();
//! });
//!
//! // ...and Bob joins the universe through it, arriving as a full replica.
//! let (mut read, mut write) = tokio::io::split(near);
//! let bob = Peer::<String>::bootstrap(&mut read, &mut write)
//!     .await?
//!     .expect("alice is established, not herself bootstrapping");
//! let bob = bob.into_rumors();
//!
//! // Convergence: Bob holds the message Alice sent before they ever met.
//! let snapshot = bob.snapshot();
//! let (_key, _version, message) = snapshot.iter().next().expect("one live message");
//! assert_eq!(message.as_str(), "the meeting is at noon");
//! # Ok::<(), rumors::Error>(())
//! # })?;
//! # Ok::<(), rumors::Error>(())
//! ```
//!
//! # What a session promises
//!
//! A [`gossip`](Rumors::gossip) that returns `Ok` leaves both replicas
//! holding every message either one held when the session began; changes
//! made concurrently with the session are simply not part of it, and ride
//! a later one. A session that fails — or whose future is dropped —
//! commits nothing: the replica is never left partially merged. The
//! *connection* is dead mid-frame after any failure or cancellation;
//! discard it and dial again.
//!
//! The moves that carry identity each need one more sentence. Cancelling a
//! [`bootstrap`](Peer::bootstrap) is free: no identity exists yet.
//! Dropping a [`retire`](Peer::retire) mid-session abandons the identity
//! exactly as a crash would (see the crash note below); let it finish and
//! inspect the returned [`Retire`], which reports the one genuinely
//! uncertain outcome explicitly instead of guessing. And a failed or
//! cancelled session that was serving a bootstrapper can waste the
//! identity it had already shipped — waste, again, as a crash wastes, with
//! nothing corrupted.
//!
//! # One rule the types cannot enforce
//!
//! **Seed once per universe.** Every cooperating deployment must descend
//! from exactly one [`Peer::seed`]; peers seeded separately belong to
//! disjoint universes, and the wire refuses to mix them
//! ([`Error::NetworkMismatch`]). If no distinguished first peer exists,
//! seed everywhere and let a deterministic tie-break pick the survivors —
//! the [`Peer`] docs give a complete uncoordinated recipe.
//!
//! Relatedly, with no correctness at stake: a peer that vanishes without
//! [`retire`](Peer::retire)-ing permanently costs the universe a little
//! bookkeeping (survivors' versions stay larger than they needed to be).
//! Retire on the way out when you can; crashes merely waste, they do not
//! corrupt.
//!
//! # Which observer should you use?
//!
//! - [`Snapshot`] ([`Rumors::snapshot`]) is a **point-in-time value**:
//!   iterate it, look up a [`Key`] ([`Snapshot::get`]), or slice it by
//!   causal range ([`Snapshot::range`]). Taking one is cheap and never
//!   waits.
//! - [`Messages`] ([`Rumors::messages`]) is the **live stream, arbitrary
//!   order**: everything not already inside your starting checkpoint, then
//!   everything learned afterwards, at the lowest cost. Use it by default.
//! - [`CausalMessages`] ([`Rumors::causal_messages`]) is the **live
//!   stream, causal order**: a message arrives only after everything it
//!   causally depends on, for an amortized logarithmic surcharge with
//!   bursts up to the size of the set. Use it only when consumers require
//!   causal delivery.
//!
//! The live observers expose a [`checkpoint`](Messages::checkpoint): the
//! sound resume point for delivery across restarts. Its docs state exactly
//! what a resume re-observes, and why folding the yielded versions
//! yourself is not a substitute.
//!
//! # Async and sync
//!
//! Everything async here is runtime-agnostic: sessions and observers are
//! plain futures and streams, driven entirely by the caller. The I/O
//! *traits* are tokio's; from another runtime, bridge with
//! [`tokio_util::compat`]. With no async runtime at all, use the [`sync`]
//! module — the same engine behind blocking calls over
//! [`std::io::Read`]/[`Write`](std::io::Write). Do not call that blocking
//! face from async context.
//!
//! # Wire compatibility
//!
//! Every session opens with [`PROTOCOL_MAGIC`] and [`PROTOCOL_VERSION`]; a
//! counterparty that is not speaking `rumors`, or speaks an incompatible
//! version, is rejected before any length-prefixed frame is trusted
//! ([`Error::MagicMismatch`], [`Error::VersionMismatch`]).
//!
//! # Stability and testing
//!
//! Pre-1.0: the Rust API may still reshape. The wire format is steadier by
//! design — pinned byte-for-byte by snapshot tests, changed only with a
//! deliberate [`PROTOCOL_VERSION`] bump.
//!
//! The crate is validated by property tests stating the model's invariants
//! (convergence under arbitrary gossip schedules, deletion honoring,
//! observer soundness), with every discovered counterexample's seed
//! committed under `proptest-regressions/`; by the wire-format snapshots;
//! and, underneath, by the differential oracles and fuzzed codecs of the
//! clock library it is built on. Found a gap? An issue or a test is very
//! welcome.
//!
//! # Feature flags
//!
//! - `test-internals` — introspection hooks for this crate's own test
//!   suites, all `#[doc(hidden)]`. Never enable it in an application.
//!
//! # Internals
//!
//! The *why* of the design — the content-addressed Merkle radix trie, the
//! mirror reconciliation protocol and its phase schedule, and the interval
//! tree clocks ([`before`]) that carry causality — is documented in
//! rustdoc beside the code, in private modules the public build does not
//! render. `just docs-internal` (`cargo doc --document-private-items`)
//! renders all of it.

// Static assertions uses #[allow(unsafe_code)], so we allow it only in tests
#![cfg_attr(not(test), forbid(unsafe_code))]
// Programmer error in recursive async traits can create large futures, so we
// check to make sure it's not an issue
#![deny(clippy::large_futures)]

pub mod sync;

mod batch;
mod bookmark;
mod message;
mod network;
mod peer;
mod rumors;
mod snapshot;
mod tree;

#[cfg(test)]
mod tests;

pub use crate::peer::{PROTOCOL_MAGIC, PROTOCOL_VERSION};
pub use ::before;
pub use ::borsh;
pub use batch::Batch;
pub use before::{Version, causally};
pub use network::Network;
pub use peer::{Peer, Retire};
pub use rumors::{CausalMessages, Messages, Rumors};
pub use snapshot::Snapshot;
pub use tree::Key;
pub use tree::mirror::remote::Error;

pub(crate) use peer::Inner;
