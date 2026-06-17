//! Lightspeed causal gossip for high-bandwidth networks.
//!
//! `rumors` replicates a set of messages across a fleet of peers with no
//! coordination: every peer holds a full replica, changes it locally (inserting
//! *or* removing messages), and reconciles pairwise with whichever peer(s) it
//! can reach. Replicas which transitively gossip eventually converge on the
//! same set of messages; `rumors` works hard to turn "eventually" into "ASAP".
//!
//! Unlike many gossip protocols, `rumors` features **redaction**. When any peer
//! redacts a message, it is contagiously purged from every peer's memory,
//! allowing superseded messages to be garbage-collected without global
//! coordination. Redaction is effectively free along every axis: it costs little
//! additional communication to convey an arbitrary quantity of redactions, and
//! zero residual local bookkeeping after messages are redacted. This means that
//! memory usage scales up *and down* with the live set of messages, and bandwidth
//! scales up *and down* with the quantity of previously-unknown messages.
//!
//! # When *should* you use it?
//!
//! **If bandwidth is abundant and latency matters.**
//!
//! Most gossip protocols are designed to be thrifty with bandwidth, trading
//! increased rounds of communication for smaller metadata overhead. However,
//! bandwidth is only getting cheaper and more plentiful, whereas *latency* is
//! capped by the laws of physics. `rumors` is designed for today and tomorrow;
//! it optimizes for extremely fast convergence when bandwidth is not a primary
//! constraint.
//!
//! **`rumors` could be a particularly excellent fit if:**
//!
//! - peers produce in total **less than 10,000 messages/second**, and
//! - each peer-to-peer link offers **1 Gb/s or better**.
//!
//! In this regime, every change propagates at the pace of a few network round
//! trips per gossip hop, for any message set size that fits in memory. Required
//! bandwidth scales linearly down with message rate (for example, 100
//! messages/s at 10 Mb/s), and total set size increases cost only by a (very
//! slow-growing) logarithmic factor. These figures price `rumors`' own metadata
//! overhead; message bodies ride on top at their raw byte rate (at 10,000
//! messages/s, about 80 Mb/s per KB of mean body size). That term is a rounding
//! error for sub-KB bodies, and overtakes the metadata around 10 KB; past that,
//! you are paying to move your data, not to coordinate it, a cost no
//! replication scheme escapes.
//!
//! **At the limits:** A link up to roughly an order of magnitude thinner (or a
//! message rate an order of magnitude faster) than these bounds degrades
//! gracefully rather than failing outright; peers may still converge, but may
//! run stale proportionately to approximately the square of the bandwidth
//! shortfall; with even less bandwidth (or even faster message rates) still,
//! they will likely fall behind regardless of gossip frequency. In the other
//! direction, past ~10 Gb/s, the network ceases to be the limit at all: message
//! rate becomes limited by CPU, and set size becomes limited by RAM.
//!
//! # When *shouldn't* you use it?
//!
//! - **If the set of live messages outgrows its smallest peer.** Every peer
//!   replicates the whole set; sharding is not supported.
//! - **If you need a consistently ordered, durable history.** A replicated log
//!   gives you sequencing; `rumors` only gives you causal ordering, which may
//!   be linearized differently between peers.
//! - **If you don't control the peers.** Peers trust one another: the protocol
//!   rejects malformed and mismatched sessions, but it is not Byzantine-tolerant:
//!   a compromised member can fabricate, redact, and deny service. Authenticating
//!   peers and securing the transport are the application's job.
//! - **If bandwidth is your scarce resource.** `rumors` is optimized to minimize
//!   round-trip latency, but it pays for this in bandwidth: when reconciling small
//!   divergences, payloads under ~10 KB use more bandwidth for metadata than for
//!   messages. On the bright side, reconciling larger divergences amortizes much
//!   of this cost: the more catching-up there is to do, the higher throughput
//!   `rumors` can deliver. That notwithstanding, on metered, narrow, or high-loss
//!   links, this crate strikes the wrong balance.
//!
//! # Network membership is identity custody
//!
//! No global shared secret initiates a peer into a gossip network. Instead,
//! membership in the network is contagious, just like messages. Initially, a new
//! gossip network is created by some single call to [`Peer::seed`], and then all
//! other members join via [`Peer::bootstrap`]ping themselves from some
//! already-bootstrapped peer, back along a chain of introductions that ends at the
//! seed.
//!
//! Peers may also [`Peer::retire`] from the network, donating their identity to
//! an arbitrary recipient. Identities are returned to circulation rather than
//! discarded because peer identity consumes *representational space*: every
//! message's [`Version`] is expressed in terms of the tree of bootstrapped
//! identities, so each [`Peer::bootstrap`] widens timestamps a little, and each
//! [`Peer::retire`] narrows them again. A peer that drops off without retiring
//! strands its identity, and the universe's timestamps stay a little wider
//! forever, wasting a few bits of space but not corrupting anything. (The
//! identity machinery is [`before`]'s interval tree clocks; see its docs for
//! the model and for the [paper it
//! implements](https://gsd.di.uminho.pt/members/cbm/ps/itc2008.pdf).)
//!
//! # The shape of the API
//!
//! One replica has two faces, split by functionality. [`Peer`] is the unique
//! `!Clone` anchor that holds the peer's identity; it appears only at the edges
//! of a replica's life, where identity can move between peers: minting a
//! universe ([`Peer::seed`]), joining one ([`Peer::bootstrap`]), leaving it
//! ([`Peer::retire`]).
//!
//! Trading the anchor away ([`Peer::into_rumors`]) opens the working state:
//! [`Rumors`] clones freely, and cloned handles may [`send`](Rumors::send),
//! [`redact`](Rumors::redact), observe [`messages`](Rumors::unordered_messages), and
//! [`gossip`](Rumors::gossip) concurrently with one another, among other
//! operations. When all other clones are gone, [`Rumors::try_into_peer`]
//! recovers the anchor. This temporal partitioning lets the compiler guarantee
//! that your whole peer identity is transferred in or out only when you have
//! exclusive ownership of it.
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
//! # How should you observe messages?
//!
//! - [`Snapshot`] ([`Rumors::snapshot`]) is a **point-in-time value**:
//!   iterate it, look up a [`Key`] ([`Snapshot::get`]), or slice it by
//!   causal range ([`Snapshot::range`]). Taking one is cheap and never
//!   waits.
//! - [`UnorderedMessages`] ([`Rumors::unordered_messages`]) is the **live stream, arbitrary
//!   order**: everything not already inside your starting checkpoint, then
//!   everything learned afterwards, at the lowest cost. Use it by default.
//! - [`CausalMessages`] ([`Rumors::causal_messages`]) is the **live
//!   stream, causal order**: a message arrives only after everything it
//!   causally depends on, for an amortized logarithmic surcharge with
//!   bursts up to the size of the set. Use it only when consumers require
//!   causal delivery.
//! - [`Changes`] ([`Rumors::changes`]) is the **live signal, no content**:
//!   one coalesced `()` per observed advance of the set, for waking work
//!   that reacts to change without consuming it — gossip drivers,
//!   persist-on-change, UI refresh. It is not delivery; pair it with a
//!   checkpoint-bearing observer for that.
//!
//! The live message observers expose a [`checkpoint`](UnorderedMessages::checkpoint):
//! the sound resume point for delivery across restarts. Its docs state exactly
//! what a resume re-observes, and why folding the yielded versions yourself is
//! not a substitute.
//!
//! # Async and sync
//!
//! Everything async here is runtime-agnostic: sessions and observers are plain
//! futures and streams, driven entirely by the caller. The I/O *traits* are
//! tokio's; from another runtime, bridge with [`tokio_util::compat`]. With no
//! async runtime at all, use the [`sync`] module's types.
//!
//! # Wire compatibility
//!
//! Every session opens with a fixed-size preamble frame carrying
//! [`PROTOCOL_MAGIC`] and [`PROTOCOL_VERSION`]; a counterparty that is not
//! speaking `rumors`, or speaks an incompatible version, is rejected before any
//! peer-declared frame length is trusted ([`Error::MagicMismatch`],
//! [`Error::VersionMismatch`]).
//!
//! # Stability and testing
//!
//! Pre-1.0: the Rust API may still reshape. The wire format is steadier by
//! design — pinned byte-for-byte by snapshot tests, changed only with a
//! deliberate [`PROTOCOL_VERSION`] bump.
//!
//! The crate is validated by property tests stating the model's invariants
//! (convergence under arbitrary gossip schedules, deletion honoring, observer
//! soundness); by the wire-format snapshots. Found a gap? An issue or a test is
//! very welcome.

// Static assertions uses #[allow(unsafe_code)], so we allow it only in tests
#![cfg_attr(not(test), forbid(unsafe_code))]
// Programmer error in recursive async traits can create large futures, so we
// check to make sure it's not an issue
#![deny(clippy::large_futures)]

pub mod sync;

mod batch;
mod bookmark;
mod message;
mod mode;
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
pub use bookmark::{Bookmark, BookmarkError};
#[doc(hidden)]
pub use mode::{Async, Blocking, Mode};
pub use network::Network;
pub use peer::{Gossiped, Led, Peer, Retire, Unbookmarked};
pub use rumors::{CausalMessages, Changes, Rumors, UnorderedMessages};
pub use snapshot::Snapshot;
pub use tree::Key;
pub use tree::MERKLE_HASH_LEN;
pub use tree::mirror::remote::Error;

pub(crate) use peer::Inner;
