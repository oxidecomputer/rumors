//! Lightspeed causal gossip.
//!
//! `rumors` is a protocol for replicating a set of causally versioned messages
//! within a fleet of peers, requiring no central coordination. A group of peers
//! which gossip amongst themselves will eventually converge on the same set of
//! messages; `rumors` works hard to turn *"eventually"* into *"ASAP"*.
//!
//! Each peer in a gossip network acts as a full replica storing every message;
//! each may make arbitrary changes to it locally (by inserting *or* removing
//! messages); and each reconciles its messages pairwise with whichever peer(s)
//! it can reach. When a message is removed, its absence propagates through
//! gossip alongside newly inserted messages, leaving no trace. For this reason,
//! we call removing a message *redacting* it. Because redacted messages occupy
//! no storage, this mechanism can be used to perform garbage collection of old
//! messages without strong consensus.
//!
//! The `rumors` crate is a *building block* for constructing eventually
//! consistent applications, especially those which can be expressed in terms of
//! a state machine driven by a causally ordered stream of messages. You, the
//! application developer, bring your own wire transport, your own async
//! executor, your own mechanism to discover and connect peers --- and of
//! course, your own application logic. But, if you can discover peers and
//! connect with them over an (*authenticated!*)
//! [`AsyncBufRead`](tokio::io::AsyncBufRead)/[`AsyncWrite`](tokio::io::AsyncWrite),
//! `rumors` can do the rest to provide *very fast* synchronization for your
//! application's messages.
//!
//! # When *should* you use it?
//!
//! **If bandwidth is reasonably abundant and latency matters.**
//!
//! Many gossip protocols are designed to be thrifty with bandwidth, trading
//! more rounds of communication for less metadata on the wire. Often, you don't
//! actually want this! In many settings, bandwidth is abundant, while *latency*
//! always remains ever-constrained by the laws of physics. `rumors` makes a
//! somewhat heterodox trade, spending some extra bandwidth whilst making use of
//! aggressive internal pipelining to minimize the number of round trips
//! required for two peers to come to synchrony.
//!
//! Practically speaking, that
//! means:
//!
//! - **Reconciliation costs time and bandwidth in proportion to divergence.**
//!   Two peers which are already synchronized can confirm this almost
//!   instantly, whether they're holding a hundred messages or a million. When
//!   they differ, the work for them to reconcile grows with the differences
//!   between their sets of messages. This means the steady-state case of
//!   frequent gossip between mostly-converged peers has very low latency; at
//!   the same time, one-off synchronizations of many messages achieve high
//!   throughput with comparatively small amortized protocol overhead.
//! - **Network latency is largely irrelevant.** During gossip, neither peer
//!   waits for the other to finish responding: both keep messages in flight
//!   towards the other at once. A high-latency link therefore adds a small,
//!   roughly fixed number of round trips to the duration of gossip, rather than
//!   stretching its total duration multiplicatively.
//! - **Memory stays bounded by a budget you can set.** In order to pipeline
//!   messages during gossip, peers store some information about messages in
//!   flight in memory. However, *you* can control the budget for the maximum
//!   permissible amount of memory to spend on such pipelining
//!   ([`Peer::sync_memory_budget`]). Most gossip sessions will be unaffected by
//!   even the minimum budget, but reconciliations between widely divergent
//!   peers will exert backpressure internally to make sure they don't balloon
//!   memory beyond your budget.
//!
//! # When *shouldn't* you use it?
//!
//! - **If the set of messages is larger than any one peer can store.** Every
//!   peer stores a full replica of all the messages; `rumors` is not a sharded
//!   database, and its design is fundamentally incompatible with sharding.
//! - **If you need consistent total ordering of messages.** `rumors` assigns a
//!   [`Version`] to each message which is [`PartialOrd`] but *not* [`Ord`], and
//!   each peer may observe a different linear sequence of messages. This
//!   protocol ensures only that messages are ordered *causally*: if a peer
//!   received one message, then sent another, the second message is ordered
//!   after the first. Messages which have no such causal relationship (i.e.
//!   causally concurrent messages) may be observed by different peers in
//!   arbitrary order.
//! - **If peers may be malicious.** The `rumors` protocol assumes that all
//!   peers are authorized to make *any changes whatsoever* to the gossip set.
//!   It is not Byzantine fault-tolerant, and is not meant to be. In order to
//!   ensure the integrity of the gossip set, any application using `rumors`
//!   *must* [authenticate connections between
//!   peers](#what-securing-the-transport-means).
//! - **If bandwidth is very scarce.** `rumors` buys low latency in part by
//!   spending bandwidth. If your messages are very small (on the order of less
//!   than 100 B), the overhead of the protocol can exceed your messages
//!   themselves. You may not care about this if you've got bandwidth to spare,
//!   but narrow, metered, or high-loss links may call for a different protocol.
//!
//! # Example
//!
//! This self-contained example runs a single gossip session between two peers
//! "Alice" and "Bob", over an in-memory channel, using the
//! [Tokio](https://tokio.rs) runtime, in a single process. The use of Tokio is
//! incidental here; `rumors` is runtime-agnostic.
//!
//! In a real application, you'd most likely use a TLS connection between two
//! different machines. Outside the territory of Example Land, there are much
//! more efficient ways than `rumors` to synchronize state within a single
//! process on one machine; `rumors` shines for gossip between truly distinct
//! peers.
//! ```
//! use rumors::Peer;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Gossip requires one duplex connection between the two peers.
//!     // Here we create an in-memory pipe, for simplicity.
//!     let (near, far) = tokio::io::duplex(64 * 1024);
//!
//!     // To gossip, each peer needs to supply an `AsyncBufRead` and an `AsyncWrite`.
//!     // Here, we do this by splitting the duplex channel into its halves.
//!     let (alice_read, alice_write) = tokio::io::split(far);
//!     let alice_read = tokio::io::BufReader::new(alice_read);
//!     let (bob_read, bob_write) = tokio::io::split(near);
//!     let bob_read = tokio::io::BufReader::new(bob_read);
//!
//!     // Alice creates a new gossip network using `Peer::seed()`.
//!     // For simplicity, we use `String` as the message type; in a real application,
//!     // you would likely use an application-specific message type.
//!     let alice: Peer<String> = Peer::seed();
//!
//!     // At this point, she could configure the peer further, but in this example,
//!     // she just converts the `Peer` into a `Rumors`, so she can use it for gossip.
//!     let alice = alice.into_rumors();
//!
//!     // She sends a message, to be received by all future peers.
//!     alice.send("the meeting is at noon".to_string())?;
//!
//!     // Alice clones the `Rumors` so that she can concurrently gossip,
//!     // as well as interact with her clone of the set of messages.
//!     let serve = alice.clone();
//!     tokio::spawn(async move {
//!         // In this example, we only gossip once; typically, she'd use
//!         // `Rumors::gossip` to repeatedly gossip on changes, in a loop.
//!         serve.gossip_once(alice_read, alice_write).await.unwrap();
//!     });
//!
//!     // Now, Bob joins the gossip network by *bootstrapping* from Alice.
//!     let rumors::Joined::Joined { peer: bob, .. } =
//!         Peer::<String>::bootstrap().join(bob_read, bob_write).await
//!     else {
//!         panic!("Alice must allow Bob to bootstrap");
//!     };
//!
//!     // This gives Bob a `Peer`, which he converts into a `Rumors`, just like Alice
//!     // did when she seeded the initial network. But now Bob's rumor set is linked
//!     // to Alice's, because he bootstrapped from her.
//!     let bob = bob.into_rumors();
//!
//!     // Now, Bob immediately has the message Alice sent before they ever met.
//!     let (_version, message) = bob.snapshot().iter().next().expect("one message");
//!     assert_eq!(message.as_str(), "the meeting is at noon");
//!     Ok(())
//! }
//! ```
//!
//! # How should you react to messages?
//!
//! A typical application built atop `rumors` will run in some kind of loop
//! where peers [`send`](Rumors::send) and [`redact`](Rumors::redact) messages
//! in response to both local events as well as other messages sent by other
//! peers.
//!
//! Depending on your application, you might want to use any of these
//! interfaces to examine and react to gossiped messages:
//!
//! - [`Rumors::snapshot`] returns a **point-in-time** [`Snapshot`]: you can
//!   iterate over all messages, or over just some using [`Snapshot::range`], or
//!   you can look up an individual message by its [`Version`] using
//!   [`Snapshot::get`].
//! - [`Rumors::causal_messages`] returns a **live stream of new messages, in
//!   causal order**. A message arrives only after everything it causally
//!   depends on. Use it when consumers require causal delivery.
//! - [`Rumors::unordered_messages`] returns a **live stream of new messages, in
//!   arbitrary order**. This is more efficient than iterating over messages in
//!   causal order, but it can result in surprising consequences where an
//!   "effect" arrives before its "cause".
//! - [`Rumors::changes`] is a **signal of change, with no content**. This
//!   stream yields one coalesced `()` each time the set of messages changes.
//!   This can be useful for waking work that reacts to changes without needing
//!   to consume the stream of messages directly.
//!
//! **You cannot observe redaction through these interfaces.** A redacted
//! message simply disappears, and no "tombstone" is left behind to be observed.
//! If your application needs to notify peers when something is deleted, send an
//! ordinary message to this effect.
//!
//! All of the above observe the *set of messages*. For instrumenting the *wire
//! protocol itself*, attach a handler from the [`observe`] module via
//! [`Peer::observe`].
//!
//! # Choosing a message type
//!
//! Messages sent via `rumors` need to be:
//!
//! - [`serde::Serialize`] and [`serde::de::DeserializeOwned`], so they can be
//!   serialized over the wire,
//! - [`Eq`], so they can be checked to ensure serialization round-trips,
//! - [`Send`], [`Sync`], and `'static`, so they can be used in an arbitrary
//!   `async` context, which may involve multiple threads.
//!
//! Your application's messages are serialized internally as [CBOR (RFC
//! 8949)](https://www.rfc-editor.org/rfc/rfc8949), though this is largely an
//! internal detail, since your application only needs to handle your own typed
//! messages through the `rumors` API.
//!
//! That said, you must additionally ensure the following, which Rust
//! can't check:
//!
//! - **Serialization must succeed on every value you send.** An error during
//!   `Serialize` should be considered a bug in the payload type, and
//!   [`send`](Rumors::send) will panic if such an error occurs. Avoid all types
//!   whose `Serialize` is fallible (for example [`std::path::PathBuf`], which
//!   errors on non-UTF-8 paths).
//! - **Serialization must round-trip on every value.** Each invocation of
//!   [`send`](Rumors::send) deserializes the serialized message immediately,
//!   before sending it, and compares it to the original message using `Eq`. A
//!   lossy serializer (for example `Some(None)` in a nested `Option`, which
//!   decodes as `None`) returns an [`EncodeError`], failing at
//!   [`send`](Rumors::send)-time, rather than causing replicas to diverge upon
//!   gossip.
//! - **Nesting depth must be bounded.** Deserializing a message may recur at
//!   most [`Peer::payload_depth_limit`] steps (256 by default). An over-deep
//!   value is rejected at [`send`](Rumors::send)-time, rather than risking a
//!   peer overflowing its stack upon receipt.
//!
//! Because CBOR carries field and variant *names*, reordering `struct` fields
//! or `enum` variants does not break compatibility with prior versions of your
//! messages. However, **renaming fields or variants will break compatibility**.
//! It's worth designing around this from the get-go: consider an outer `enum`
//! indicating the version of your application-level message type, even if it
//! starts out only having one variant like `V1`.
//!
//! # Transport between peers
//!
//! The `rumors` protocol does not specify a particular transport between peers;
//! that's up to your application.
//!
//! For two peers to gossip successfully and safely with `rumors`, they need a
//! single authenticated, reliable, ordered, full-duplex byte stream with
//! buffered reads, back-pressure and visible closes. Think "something shaped
//! like a TLS connection over TCP".
//!
//! Let's unpack this. Your connection must be:
//!
//! - **authenticated**: only peers you trust may be allowed to talk to you, and
//!   your connection must prevent any man-in-the-middle from interfering with
//!   traffic between two peers, or else hostile actors could corrupt the gossip
//!   set;
//! - **reliable and ordered:** bytes are delivered in order, with none missing
//!   or duplicated, or else the protocol will break arbitrarily;
//! - **full-duplex**: reads must proceed while the same side's write end is
//!   blocked, or else gossip may deadlock;
//! - **buffered reads**: reads must be buffered by the provided read-half
//!   (required by the types), or else too many syscalls may result;
//! - **back-pressure**: reads must eventually exert backpressure on writes, or
//!   else gossip may use unbounded memory;
//! - **visible closes**: when a peer drops off, the read half reports end of
//!   stream or an error, and every write, pending or later, eventually
//!   completes or fails, or else gossip may get stuck.
//!
//! Additionally, if you want your messages to remain confidential, your
//! connection must itself be confidential: if someone can read the messages
//! sent during gossip, they can reconstruct everything your application knows.
//!
//! ### Quick transport recipe: TLS over TCP via Tokio
//!
//! Typically, you would run `rumors` over an (m)TLS connection over TCP.
//!
//! In this example, we use a TLS certificate authority to attest to the
//! authorization of each peer to join the gossip network. Any peer with a
//! certificate signed by the "network CA" is considered authorized to join the
//! network.
//!
//! Notable aspects of this example:
//!
//! - **Mutual authentication**: both the server and client are configured to
//!   require/provide mTLS, and check against the network CA's root certificate.
//!   This is required to ensure a man-in-the-middle cannot interfere with
//!   gossip.
//! - **Buffered reads**: the read-end of the connection is wrapped in a
//!   `BufReader`, which is required by the type signature for
//!   [`Rumors::gossip`], and ensures that reads do not invoke a syscall each.
//! - **Nagle's algorithm disabled**: `TcpStream::set_nodelay(true)` is set on
//!   both ends, to prevent needless latency.
//!
//! ```no_run
//! use std::error::Error;
//! use std::net::SocketAddr;
//! use std::sync::Arc;
//!
//! use rumors::Rumors;
//! use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName};
//! use rustls::server::WebPkiClientVerifier;
//! use rustls::{ClientConfig, RootCertStore, ServerConfig};
//! use tokio::io::{AsyncRead, AsyncWrite, BufReader};
//! use tokio::net::{TcpListener, TcpStream};
//! use tokio_rustls::{TlsAcceptor, TlsConnector};
//!
//! // Each peer can connect and accept. On either side, TLS authenticates
//! // the other side against the network's certificate authority.
//! fn tls(
//!     network_ca: RootCertStore,
//!     cert: Vec<CertificateDer<'static>>,
//!     key: PrivateKeyDer<'static>,
//! ) -> Result<(TlsConnector, TlsAcceptor), Box<dyn Error>> {
//!     let network_ca = Arc::new(network_ca);
//!     let client = ClientConfig::builder()
//!         .with_root_certificates(network_ca.clone())
//!         .with_client_auth_cert(cert.clone(), key.clone_key())?;
//!     let verifier = WebPkiClientVerifier::builder(network_ca).build()?;
//!     let server = ServerConfig::builder()
//!         .with_client_cert_verifier(verifier)
//!         .with_single_cert(cert, key)?;
//!     Ok((Arc::new(client).into(), Arc::new(server).into()))
//! }
//!
//! async fn connect(
//!     rumors: &Rumors<String>,
//!     tls: &TlsConnector,
//!     addr: SocketAddr,
//!     name: ServerName<'static>,
//! ) -> Result<(), Box<dyn Error>> {
//!     let tcp = TcpStream::connect(addr).await?;
//!     tcp.set_nodelay(true)?;
//!     let stream = tls.connect(name, tcp).await?;
//!     gossip_over(rumors, stream).await
//! }
//!
//! async fn accept(
//!     rumors: &Rumors<String>,
//!     tls: &TlsAcceptor,
//!     listener: &TcpListener,
//! ) -> Result<(), Box<dyn Error>> {
//!     let (tcp, _) = listener.accept().await?;
//!     tcp.set_nodelay(true)?;
//!     let stream = tls.accept(tcp).await?;
//!     gossip_over(rumors, stream).await
//! }
//!
//! async fn gossip_over(
//!     rumors: &Rumors<String>,
//!     stream: impl AsyncRead + AsyncWrite + Send + Unpin,
//! ) -> Result<(), Box<dyn Error>> {
//!     let (read, write) = tokio::io::split(stream);
//!     rumors.gossip(BufReader::new(read), write).await?;
//!     Ok(())
//! }
//! ```
//!
//! A departed peer that never closes the connection leaves a session waiting,
//! as does a transport whose writes don't complete; configure
//! [`Peer::session_deadline`] to bound such waits.
//!
//! # Features
//!
//! All features are off by default.
//! - `fs`: [`FileBookmark`] and its blocking-I/O bridge.
//! - `conformance`: a validation suite, [`conformance::bookmark`], for
//!   caller-provided bookmark storage. Import this via a dev-dependency into
//!   your own test suite to make sure your storage matches what's expected of
//!   it.
//!
//! # Stability
//!
//! Any breaking change to the wire format will come with an increment to the
//! on-the-wire protocol version.

#![forbid(unsafe_code)]
// docs.rs passes `--cfg docsrs` (Cargo.toml's docs.rs metadata), enabling
// rustdoc's `doc_cfg` feature so feature-gated items label their gate; no
// other build sets the cfg, so the attribute is absent everywhere else. The
// justfile's `docs-docsrs` leg is the same build.
#![cfg_attr(docsrs, feature(doc_cfg))]
// Programmer error in recursive async traits can create large futures, so we
// check to make sure it's not an issue
#![deny(clippy::large_futures)]

mod batch;
mod bookmark;
#[cfg(any(test, feature = "conformance"))]
pub mod conformance;
mod counter;
pub mod default;
pub mod error;
mod message;
mod network;
pub mod observe;
mod peer;
mod protocol;
pub mod reconciliation;
mod rumors;
pub mod sizing;
mod snapshot;
pub mod tags;
#[cfg(any(test, feature = "test-internals"))]
#[doc(hidden)]
pub mod testing;
mod tree;
pub mod tutorial;

#[cfg(test)]
mod tests;

// This path exposes the exact `before` version used in public signatures, so
// applications can name compatible clock types without a separate dependency.
pub use ::before;
pub use batch::{Batch, Committed};
// Linked, not inlined: before's pages carry the measured-growth charts,
// which an inlined copy cannot (its docs come from compiled metadata, and
// the widget's assets are attached to rustdoc's own view of before only).
#[doc(no_inline)]
pub use before::{Count, Version, causally};
pub use bookmark::{BOOKMARK_FORMAT_VERSION, Bookmark, BookmarkIo, FormatError, NoBookmark};
#[cfg(feature = "fs")]
#[cfg_attr(docsrs, doc(cfg(feature = "fs")))]
pub use bookmark::{FileBookmark, RunBlocking, TokioBlocking};
pub use error::Error;
pub use message::EncodeError;
pub use network::Network;
pub(crate) use peer::Inner;
pub use peer::{
    Bootstrap, Connection, Gossip, Gossiped, Joined, Led, MAX_BUDGETED_MESSAGE_SIZE,
    MAX_RUN_BUDGET_BYTES, MIN_SYNC_MEMORY_BUDGET, Peer, Retire, SynchronizationSettings,
    Unbookmarked,
};
pub use protocol::Protocol;
pub use rumors::{CausalMessages, Changes, Rumors, TryNext, TryTick, UnorderedMessages};
pub use snapshot::{Iter, Snapshot};
pub use tree::mirror::streaming::stats::SessionStats;

/// Whether this build of the crate carries the `test-internals` feature.
///
/// This exists only for the workspace's measurement tool, `rumors-netperf`,
/// which refuses to measure with a build that carries the feature: its
/// introspection and scheduling hooks cost time on the session paths the tool
/// times. No application has a use for it, so it is hidden from the
/// documentation.
///
/// The tool depends on this crate with default features, yet the feature can
/// still reach its copy: cargo unifies a package's features across everything
/// one invocation builds, so building the tool beside a crate that enables
/// `test-internals`, such as the test kit or the budget curves, compiles the
/// feature into the tool too. The tool's manifest cannot rule that out, so the
/// tool reads this constant before it measures.
#[doc(hidden)]
pub const TEST_INTERNALS: bool = cfg!(feature = "test-internals");
