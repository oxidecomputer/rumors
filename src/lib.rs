#![recursion_limit = "256"]
#![warn(clippy::large_futures)]
//! Unordered gossip with redaction.
//!
//! `rumors` is a CRDT-backed gossip set. Each peer holds a [`Local<T>`] rumor
//! set; peers reconcile by exchanging only the parts that differ. Redacting
//! a message stops it propagating, and redactions spread contagiously to
//! every peer the redactor (transitively) gossips with.
//!
//! This is the asynchronous surface. For synchronous I/O (e.g.
//! [`std::net::TcpStream`]) see the parallel [`rumors::sync`](sync) module.
//!
//! # Quickstart
//!
//! ```
//! use rumors::Local;
//!
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() {
//! // A peer is identified by an arbitrary byte string; the caller must
//! // keep party identifiers globally unique. `start` is the local event
//! // counter to resume from (0 for a fresh party); see [`Local::for_party`].
//! let mut alice = Local::for_party("alice", 0).unwrap();
//!
//! // The callback fires once per newly-observed message with an opaque
//! // `Key` (used later for redaction), the causal `Version`, and the value.
//! let mut observed = 0;
//! alice.message(
//!     ["hello".to_string(), "world".to_string()],
//!     async |_key, _version, _message| observed += 1,
//! ).await;
//! assert_eq!(observed, 2);
//! # }
//! ```
//!
//! # Redaction
//!
//! Any peer can [`redact`](Local::redact) a [`Key`] it holds; the redaction
//! propagates to every connected peer without consensus, so a single peer's
//! local decision evicts the message network-wide.
//!
//! ```
//! use rumors::{Local, Key};
//!
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() {
//! let mut alice = Local::for_party("alice", 0).unwrap();
//! let mut keys: Vec<Key> = Vec::new();
//! alice.message(
//!     ["stale rumor".to_string()],
//!     async |k, _, _| keys.push(k),
//! ).await;
//! alice.redact(keys);
//! # }
//! ```
//!
//! # Concurrent rumor sets
//!
//! A [`Local`] is either an [`Original`] (returned by [`Local::for_party`], one
//! per party per process) or a [`Forked`] copy made with [`Local::fork`]. Only
//! the [`Original`] may originate new [`message`](Local::message)s or
//! [`redact`](Local::redact)ions; [`Forked`] clones are cheap (the underlying
//! tree is structurally shared and copy-on-write) and exist to be mutated
//! concurrently against peers, then folded back in via [`Local::process`] (or
//! the [`Add`] / [`AddAssign`] operators). This split enforces at the type
//! level that every party acts as a single sequential process. See the
//! [`Local`] docs for the full discussion.
//!
//! # Gossiping with peers on the network
//!
//! Pass an [`AsyncRead`] reader and an [`AsyncWrite`] writer into
//! [`Local::gossip`]:
//!
//! ```
//! use rumors::{Local, ignore};
//!
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() {
//! // Stand in for a real network with an in-memory bidirectional pipe.
//! let (a, b) = tokio::io::duplex(1024);
//! let (mut a_r, mut a_w) = tokio::io::split(a);
//! let (mut b_r, mut b_w) = tokio::io::split(b);
//!
//! let mut alice: Local<String, _> = Local::for_party("alice", 0).unwrap();
//! alice.message(["hello".to_string()], ignore).await;
//! let bob: Local<String, _> = Local::for_party("bob", 0).unwrap();
//!
//! // Drive both ends concurrently; bob learns "hello".
//! let (alice, bob) = tokio::join!(
//!     alice.gossip(&mut a_r, &mut a_w, ignore),
//!     bob.gossip(&mut b_r, &mut b_w,
//!         async |_, _, m| assert_eq!(m.as_ref(), "hello"),
//!     ),
//! );
//! let (_alice, _bob) = (alice.unwrap(), bob.unwrap());
//! # }
//! ```
//!
//! # Message serialization
//!
//! Messages are serialized with [`borsh`], which is re-exported so callers
//! can derive [`BorshSerialize`] / [`BorshDeserialize`] on their message
//! types without taking a separate dependency.
//!
//! # Stability
//!
//! Pre-1.0. The on-the-wire protocol is part of the public API:
//!
//! - **Patch versions** are wire-identical: two peers on the same minor
//!   version, regardless of patch, always interoperate.
//! - **Minor versions** are forward-compatible. Wire changes between
//!   minor versions are strictly additive, and the version negotiated by
//!   the handshake is `min(local, remote)`, so a peer on a newer minor
//!   version can gossip with a peer on an older one.
//! - **Major versions** may break the wire incompatibly. The handshake
//!   surfaces such mismatches as [`Error::VersionMismatch`] before any
//!   rumor-set state is touched.
//!
//! Every connection begins with an 8-byte preamble: [`PROTOCOL_MAGIC`]
//! (`b"RUMR"`), [`PROTOCOL_VERSION`] as a big-endian `u16`, and a
//! reserved `u16` (zero in v1). Reading bytes that don't start with
//! `RUMR` surfaces as [`Error::MagicMismatch`]; the connection is not a
//! `rumors` stream at all.
//!
//! # Compression
//!
//! The wire protocol implemented in this crate is uncompressed. Party
//! identifiers appear inline in every version vector exchanged during gossip,
//! and version vectors are exchanged frequently: this metadata channel is
//! highly redundant and compresses easily. Payload bytes (Blake3 hashes, your
//! borsh-encoded messages) generally do not compress further. We do not
//! compress internally because the best algorithm could depend on the content
//! of gossiped messages; however, **compressing the wire is strongly
//! recommended** and is the caller's responsibility. See the
//! [`compress`](crate::guide::compress) how-to for a working recipe.

use std::{
    future::Future,
    ops::{Add, AddAssign},
    pin::Pin,
    sync::{Arc, LazyLock, Mutex, Weak},
};

use borsh::{BorshDeserialize, BorshSerialize};
use bytes::Bytes;
use tokio::io::{AsyncRead, AsyncWrite};
use weak_table2::WeakHashSet;

pub mod explanation;
pub mod guide;
pub mod sync;
pub mod tutorial;

mod imbl_borsh;
mod message;
mod tree;
mod version;

use message::Message;
use tree::{Action, Tree, mirror};

/// Magic bytes that prefix every `rumors` gossip session: `b"RUMR"`.
///
/// Sent as the first four bytes of the [handshake](Local::gossip), a peer
/// whose preamble starts with anything else is rejected with
/// [`Error::MagicMismatch`] before any rumor-set state is touched.
pub const PROTOCOL_MAGIC: [u8; 4] = *b"RUMR";

/// On-the-wire protocol version, exchanged in the [handshake](Local::gossip)
/// right after [`PROTOCOL_MAGIC`].
///
/// Bumped whenever the wire format changes. Patch versions of `rumors` never
/// change this; minor versions may bump it but remain wire-compatible
/// downstream (see the crate-level `# Stability` section); major versions
/// may bump it incompatibly, in which case a peer running an incompatible
/// version is rejected with [`Error::VersionMismatch`].
pub const PROTOCOL_VERSION: u16 = 1;

/// A local set of rumors: add to it, redact from it, gossip with peers.
///
/// [`Local<T, Forked>`](Local) is cheap to clone (structurally shared,
/// copy-on-write); concurrent code holds one fork per thread or task and
/// recombines them via [`Local::process`]. [`Local<T, Original>`](Local)
/// deliberately does *not* implement [`Clone`]: only the singleton
/// original may originate new messages and redactions, and duplicating one
/// must be explicit. Use [`Local::fork`] to obtain a [`Forked`] view that
/// can be cloned and moved across tasks.
///
/// Methods take `AsyncFnMut` callbacks; for synchronous I/O and callbacks,
/// see [`sync::Local`].
///
/// # Uniqueness of parties
///
/// It is *required* that each party's [`Local::message`] and [`Local::redact`]
/// actions are causally sequential. This is enforced locally within a given
/// process: [`Local::for_party`] returns a type-tagged `Local<T, Original>` (or
/// [`Err(AlreadyExists)`](AlreadyExists) if there is an extant original
/// [`Local`] for this party in the current process). Subsequently,
/// [`Local::fork`] can duplicate an [`Original`] [`Local`] into a [`Forked`]
/// [`Local`], which can still participate in [`gossip`](Local::gossip) and can
/// still [`process`](Local::process) other [`Forked`] [`Local`]s into itself,
/// but crucially which *cannot* originate new messages and redactions: these
/// may only be performed on the original singleton `Local<T, Original>`.
///
/// While these checks enforce consistency within a single process, it is the
/// responsibility of the programmer to ensure that parties act as sequential
/// processes across the network. In particular, if an [`Original`] [`Local`]
/// is ever dropped and then recreated for the same party (e.g. across process
/// restarts), the `start` parameter passed to [`Local::for_party`] must be
/// greater than or equal to the last observable [`event`](Local::event) of the
/// prior instantiation. Persist `event()` durably between runs and feed it back
/// in as `start` to uphold this invariant.
///
/// # Example
///
/// ```
/// use rumors::{Local, Key};
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// let mut alice = Local::for_party("alice", 0).unwrap();
/// let mut keys: Vec<Key> = Vec::new();
/// alice.message(
///     ["hello".to_string(), "world".to_string()],
///     async |key, _, _| keys.push(key),
/// ).await;
/// alice.redact([keys[0]]);
/// # }
/// ```
#[derive(Debug, Eq)]
pub struct Local<T, Identity = Forked> {
    tree: Tree<T>,
    identity: Identity,
}

/// Marker type indicating that this [`Local`] is the original created by
/// [`Local::for_party`], empowering it to be used for adding new
/// [`message`](Local::message)s and [`redact`](Local::redact)ions.
#[derive(Debug, PartialEq, Eq)]
pub struct Original(Arc<Bytes>);

/// Marker type indicating that this [`Local`] is a fork of another, meaning it
/// can only be used for [`process`](Local::process)ing and
/// [`gossip`](Local::gossip)ing about messages and redactions originated by
/// other [`Original`] [`Local`]s in the system.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Forked(());

/// Error returned from [`Local::for_party`] when an [`Original`] [`Local`]
/// already exists for that party in the current process.
///
/// The check is intentionally per-process: it enforces locally that there is
/// only one [`Original`] per party at any moment. Releasing the [`Original`]
/// (dropping it) frees the party identifier; reusing the identifier on a fresh
/// [`Local::for_party`] call is allowed but the new `start` must be `>=` the
/// dropped party's last observable [`event`](Local::event).
///
/// # Example
///
/// ```
/// use rumors::{Local, AlreadyExists};
///
/// let alice: Local<String, _> = Local::for_party("solo", 0).unwrap();
/// // A second Original for the same party would violate uniqueness.
/// let result: Result<Local<String, _>, _> = Local::for_party("solo", 0);
/// assert!(matches!(result.unwrap_err(), AlreadyExists { .. }));
///
/// // Dropping the Original frees the party slot.
/// drop(alice);
/// let _alice_again: Local<String, _> = Local::for_party("solo", 0).unwrap();
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("original local rumor set for this party already exists")]
pub struct AlreadyExists {}

/// Only forked `Local`s can be cloned using [`Clone`]; to clone an original
/// `Local` into a non-original one, use [`Local::fork`].
impl<T> Clone for Local<T> {
    fn clone(&self) -> Self {
        Self {
            tree: self.tree.clone(),
            identity: self.identity,
        }
    }
}

impl<T, Identity, Other> PartialEq<Local<T, Other>> for Local<T, Identity> {
    fn eq(&self, other: &Local<T, Other>) -> bool {
        self.tree == other.tree
    }
}

/// The error type returned by [`Local::gossip`].
///
/// Surfaces I/O failures from the underlying reader/writer as well as
/// framing errors encountered while parsing messages off the wire.
pub use mirror::remote::Error;

/// An opaque identifier for a single message in a [`Local`] rumor set.
///
/// Keys are produced by the `on_message` callbacks of [`Local::message`],
/// [`Local::process`], and [`Local::gossip`], and are stable across peers:
/// a key obtained from one peer can redact the message on any other.
///
/// Two content-identical messages always receive distinct keys: every
/// insert advances the local version vector before the key is derived.
///
/// # Example
///
/// ```
/// use rumors::{Local, Key};
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// let mut alice = Local::for_party("alice", 0).unwrap();
/// let mut keys: Vec<Key> = Vec::new();
/// alice.message(
///     ["echo".to_string(), "echo".to_string()],
///     async |k, _, _| keys.push(k),
/// ).await;
/// assert_ne!(keys[0], keys[1]);
/// # }
/// ```
pub use tree::Key;

/// A causal version vector tagging when a message was observed.
///
/// Surfaced to the `on_message` callbacks of [`Local::message`],
/// [`Local::process`], and [`Local::gossip`]. [`PartialOrd`] captures causal
/// ordering: `a <= b` iff every party's counter in `a` is at most the
/// corresponding counter in `b`. Versions produced by concurrent events are
/// incomparable (`partial_cmp` returns `None`).
///
/// # Example
///
/// ```
/// use rumors::{Local, Version};
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// let mut alice = Local::for_party("alice", 0).unwrap();
/// let mut versions: Vec<Version> = Vec::new();
/// alice.message(
///     ["first".to_string(), "second".to_string()],
///     async |_, v, _| versions.push(v.clone()),
/// ).await;
/// // Successive messages from the same party are causally ordered.
/// assert!(versions[0] < versions[1]);
/// # }
/// ```
pub use version::Version;

/// The [`borsh`] crate, re-exported.
///
/// Message types must implement [`BorshSerialize`] and [`BorshDeserialize`];
/// re-exporting borsh here lets callers derive both without a separate
/// dependency.
///
/// # Example
///
/// ```
/// use rumors::{Local, borsh, ignore};
///
/// #[derive(borsh::BorshSerialize, borsh::BorshDeserialize)]
/// struct Rumor { subject: String, count: u32 }
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// let mut alice = Local::for_party("alice", 0).unwrap();
/// alice.message(
///     [Rumor { subject: "weather".into(), count: 3 }],
///     ignore,
/// ).await;
/// # }
/// ```
pub use ::borsh;

/// An `on_message` callback that discards every observation.
///
/// Pass this when you only care about mutating the rumor set, not about
/// inspecting individual messages. See [`sync::ignore`] for the sync
/// equivalent.
///
/// Returns [`std::future::Ready<()>`], a concrete `Send + 'static` future,
/// so it satisfies the `OnMessage: FnMut(...) -> _ + Send + 'static`
/// bound that [`Local::gossip`] requires.
///
/// # Example
///
/// ```
/// use rumors::{Local, ignore};
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// let mut alice = Local::for_party("alice", 0).unwrap();
/// alice.message(["hello".to_string(), "world".to_string()], ignore).await;
/// # }
/// ```
pub fn ignore<T>(_key: Key, _version: &Version, _message: &Arc<T>) -> std::future::Ready<()> {
    std::future::ready(())
}

impl<T> Local<T, Original> {
    /// Create an empty rumor set tagged with the given party identifier.
    ///
    /// Party identifiers must be *globally unique* across the gossip network;
    /// reusing one across peers causes missed messages and other undefined
    /// behavior. If a party identifier is ever reused, its `start` must be
    /// greater than or equal to the last observable [`event`](Local::event) of
    /// the last instantiation of that party.
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::Local;
    ///
    /// let _alice: Local<String, _> = Local::for_party("alice", 0).unwrap();
    /// ```
    pub fn for_party(party: impl AsRef<[u8]>, start: u64) -> Result<Self, AlreadyExists> {
        // Weak pointers to all the parties which have been created.
        static PARTIES: LazyLock<Mutex<WeakHashSet<Weak<Bytes>>>> =
            LazyLock::new(|| Mutex::new(WeakHashSet::new()));

        let tree = Tree::for_party(party, start);
        let identity = Original(Arc::new(tree.party().clone()));

        // Ensure this party isn't live right now, then track it.
        let mut parties = PARTIES.lock().unwrap();
        if !parties.contains(&*identity.0) {
            parties.insert(identity.0.clone());
            Ok(Self { identity, tree })
        } else {
            Err(AlreadyExists {})
        }
    }

    /// Get this party's local event counter: the count of all operations ever
    /// applied by this party.
    ///
    /// Persist this value durably between process runs and pass it back as the
    /// `start` argument to [`Local::for_party`] on the next invocation. If a
    /// party name is reused, `start >= self.event()` of the prior instantiation
    /// is *required*; violating this invariant can lead to arbitrary and
    /// contagious corruption of the rumor set network-wide.
    pub fn event(&self) -> u64 {
        self.tree.version().for_party(self.tree.party())
    }

    /// Insert messages into the rumor set, invoking `on_message` once per
    /// newly-observed message.
    ///
    /// The callback receives:
    ///
    /// - an opaque [`Key`], usable later with [`redact`](Self::redact);
    /// - the causal [`Version`] at which the message was observed;
    /// - an [`Arc<T>`](Arc) holding the message itself.
    ///
    /// Callback order is unspecified and need not match insertion order. If
    /// your application needs an ordering, sort by the [`Version`] threaded
    /// through the callback.
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::Local;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let mut alice = Local::for_party("alice", 0).unwrap();
    /// let mut observed = Vec::new();
    /// alice.message(
    ///     ["hello".to_string(), "world".to_string()],
    ///     async |key, version, message| {
    ///         observed.push((key, version.clone(), message.as_ref().clone()));
    ///     },
    /// ).await;
    /// assert_eq!(observed.len(), 2);
    /// # }
    /// ```
    pub async fn message<OnMessage, OnMessageFut, I>(
        &mut self,
        messages: I,
        mut on_message: OnMessage,
    ) where
        T: BorshSerialize + Send + Sync + 'static,
        I: IntoIterator<Item = T> + Send,
        I::IntoIter: Send,
        OnMessage: FnMut(Key, &Version, &Arc<T>) -> OnMessageFut + Send + 'static,
        OnMessageFut: Future<Output = ()> + Send + 'static,
    {
        // Box-and-Send-erase the protocol future at the API boundary so the
        // auto-trait check is discharged here (under the lib's
        // `#![recursion_limit]`) rather than at every call site.
        let fut: Pin<Box<dyn Future<Output = ()> + Send + '_>> = Box::pin(async move {
            self.tree
                .act(
                    messages.into_iter().map(Message::from).map(Action::Insert),
                    move |k: Key, v: &Version, m: Option<&Message<T>>|
                        -> Pin<Box<dyn Future<Output = ()> + Send + 'static>> {
                        match m {
                            Some(m) => Box::pin(on_message(k, v, m.as_ref())),
                            None => Box::pin(std::future::ready(())),
                        }
                    },
                )
                .await;
        });
        fut.await
    }

    /// Redact the given keys: stop gossiping the corresponding messages, and
    /// instruct every peer we synchronize with to do the same.
    ///
    /// Each [`Key`] was originally surfaced by an `on_message` callback in
    /// [`Local::message`], [`Local::process`], or [`Local::gossip`].
    /// Redaction is contagious, so a single peer's call evicts the message
    /// network-wide without consensus.
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::{Local, Key};
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let mut alice = Local::for_party("alice", 0).unwrap();
    /// let mut keys: Vec<Key> = Vec::new();
    /// alice.message(
    ///     ["transient announcement".to_string()],
    ///     async |k, _, _| keys.push(k),
    /// ).await;
    /// alice.redact(keys);
    /// # }
    /// ```
    pub fn redact<I: IntoIterator<Item = Key>>(&mut self, redacted: I)
    where
        T: Send + Sync + 'static,
    {
        pollster::block_on(
            self.tree
                .act(redacted.into_iter().map(Action::Forget), |_, _, _| {
                    std::future::ready(())
                }),
        );
    }
}

impl<T, Identity> Local<T, Identity> {
    /// Duplicate a rumor set into a [`Forked`] [`Local`] usable concurrently.
    ///
    /// Forks share their underlying tree structurally (copy-on-write), so this
    /// is cheap. The fork may [`gossip`](Local::gossip) and merge other forks
    /// into itself via [`process`](Local::process) or `+`, but it *cannot*
    /// originate new [`message`](Local::message)s or [`redact`](Local::redact)
    /// keys; only the singleton [`Original`] for the party can. The fork can
    /// later be folded back in with `original.process(fork)`. See the
    /// [`Local`] docs for why this split exists.
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::{Local, ignore};
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let mut alice = Local::for_party("alice", 0).unwrap();
    /// alice.message(["hello".to_string()], ignore).await;
    ///
    /// // A fork can be moved to another task; only the Original can mutate.
    /// let snapshot = alice.fork();
    /// assert_eq!(alice, snapshot);
    /// # }
    /// ```
    pub fn fork(&self) -> Local<T, Forked> {
        Local {
            tree: self.tree.clone(),
            identity: Forked(()),
        }
    }

    /// Merge `new` into `self`, invoking `on_message` for each message in
    /// `new` that `self` had not already observed.
    ///
    /// `new` must be [`Forked`]: only the [`Original`] for a party can
    /// originate messages, but any number of [`Forked`] copies can carry
    /// observations between peers and recombine. The callback signature
    /// matches [`Local::message`]; messages present in `self` but missing from
    /// `new` do not fire it.
    ///
    /// **Delivery is unordered**: callbacks fire in arbitrary order,
    /// including orderings that violate the causal precedence captured by
    /// each message's [`Version`]. `rumors` is not a causal-delivery
    /// primitive; if your application requires causal or insertion
    /// ordering, sort the observations by [`Version`] before consuming
    /// them.
    ///
    /// # Example
    ///
    /// Two parties, each holding their own [`Original`], can exchange state
    /// by forking and processing:
    ///
    /// ```
    /// use rumors::{Local, ignore};
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let mut alice = Local::for_party("alice", 0).unwrap();
    /// let mut bob = Local::for_party("bob", 0).unwrap();
    /// bob.message(["news from bob".to_string()], ignore).await;
    ///
    /// // `bob.fork()` produces a Forked copy that alice can absorb.
    /// let mut learned = Vec::new();
    /// alice.process(bob.fork(), async |_, _, m| {
    ///     learned.push(m.as_ref().clone())
    /// }).await;
    /// assert_eq!(learned, vec!["news from bob".to_string()]);
    /// # }
    /// ```
    pub async fn process<OnMessage, OnMessageFut>(
        &mut self,
        new: Local<T, Forked>,
        on_message: OnMessage,
    ) where
        T: Send + Sync + 'static,
        Identity: Send,
        OnMessage: FnMut(Key, &Version, &Arc<T>) -> OnMessageFut + Send + 'static,
        OnMessageFut: std::future::Future<Output = ()> + Send + 'static,
    {
        // Box-and-Send-erase the protocol future at the API boundary so the
        // auto-trait check is discharged here (under the lib's
        // `#![recursion_limit]`) rather than at every call site.
        let fut: Pin<Box<dyn Future<Output = ()> + Send + '_>> = Box::pin(async move {
            // Instantiate the two sides of the mirror exchange, both local
            let l = mirror::local::Exchange::start(self.tree.root.clone(), ignore, on_message);
            let r = mirror::local::Exchange::start(new.tree.root, ignore, ignore);

            // Drive them to completion: we know they don't need a "real" executor
            Ok((self.tree.root, _)) = mirror(l, r).await;
        });
        fut.await
    }

    /// Synchronize rumor sets with a remote peer, invoking `on_message` for
    /// each message learned from the peer.
    ///
    /// `read` and `write` must implement [`AsyncRead`] / [`AsyncWrite`];
    /// both ends of the connection must drive `gossip` concurrently. The
    /// callback signature matches [`Local::message`]. For synchronous I/O,
    /// use [`sync::Local::gossip`].
    ///
    /// The session begins with the 8-byte protocol handshake described in
    /// the crate-level `# Stability` section; a peer with the wrong
    /// [`PROTOCOL_MAGIC`] or [`PROTOCOL_VERSION`] is rejected as
    /// [`Error::MagicMismatch`] or [`Error::VersionMismatch`] before any
    /// rumor-set state is touched. After the handshake, message delivery
    /// is **unordered**: callbacks fire in arbitrary order, including
    /// orderings that violate the causal precedence captured by each
    /// message's [`Version`]. Sort by [`Version`] if your application
    /// needs causal or insertion ordering.
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::{Local, ignore};
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let (a, b) = tokio::io::duplex(1024);
    /// let (mut a_r, mut a_w) = tokio::io::split(a);
    /// let (mut b_r, mut b_w) = tokio::io::split(b);
    ///
    /// let mut alice: Local<String, _> = Local::for_party("alice", 0).unwrap();
    /// alice.message(["hello".to_string()], ignore).await;
    /// let bob: Local<String, _> = Local::for_party("bob", 0).unwrap();
    ///
    /// let mut bob_learned: Vec<String> = Vec::new();
    /// let (alice, bob) = tokio::join!(
    ///     alice.gossip(&mut a_r, &mut a_w, ignore),
    ///     bob.gossip(&mut b_r, &mut b_w,
    ///         async |_, _, m| bob_learned.push(m.as_ref().clone()),
    ///     ),
    /// );
    /// let (_alice, _bob) = (alice.unwrap(), bob.unwrap());
    /// assert_eq!(bob_learned, vec!["hello".to_string()]);
    /// # }
    /// ```
    pub async fn gossip<OnMessage, OnMessageFut, R, W>(
        mut self,
        read: &mut R,
        write: &mut W,
        on_message: OnMessage,
    ) -> Result<Self, Error>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync + 'static,
        R: AsyncRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
        OnMessage: FnMut(Key, &Version, &Arc<T>) -> OnMessageFut + Send + 'static,
        OnMessageFut: Future<Output = ()> + Send + 'static,
        Identity: Send,
    {
        // Box-and-Send-erase the protocol future at the API boundary so the
        // auto-trait check is discharged here (under the lib's
        // `#![recursion_limit]`) rather than at every call site.
        let fut: Pin<Box<dyn Future<Output = Result<Self, Error>> + Send + '_>> =
            Box::pin(async move {
                // Protocol-version handshake: both sides exchange a fixed
                // 8-byte preamble before any other traffic. An incompatible
                // peer is rejected here without touching the local rumor set.
                mirror::remote::handshake(read, write).await?;

                // Instantiate the two sides of the mirror exchange: local and remote
                let l = mirror::local::Exchange::start(self.tree.root, ignore, on_message);
                let r = mirror::remote::Exchange::start(read, write);

                // Drive them to completion against each other
                (self.tree.root, _) = mirror(l, r).await.map_err(|e| {
                    // The only possible error is a server error
                    let mirror::Error::Server(e) = e;
                    e
                })?;

                Ok(self)
            });
        fut.await
    }
}

/// Combine two rumor sets via [`Local::process`].
impl<T> Add for Local<T, Forked>
where
    T: Send + Sync + 'static,
{
    type Output = Local<T, Forked>;

    fn add(mut self, rhs: Self) -> Self::Output {
        pollster::block_on(self.process(rhs, |_, _, _| std::future::ready(())));
        self
    }
}

/// Absorb `rhs` into `self` via [`Local::process`].
impl<T> AddAssign for Local<T, Forked>
where
    T: Send + Sync + 'static,
{
    fn add_assign(&mut self, rhs: Self) {
        *self = self.clone().add(rhs);
    }
}
