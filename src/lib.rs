#![warn(clippy::large_futures)]
//! Unordered gossip with redaction.
//!
//! `rumors` is a CRDT-backed gossip set. Each peer holds a [`Known<T>`] rumor
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
//! use rumors::Known;
//!
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() {
//! // `seed` mints the distinguished root rumor set for a universe of peers;
//! // additional peers are made by [`Known::fork`], never by a second `seed`.
//! let mut alice = Known::seed();
//!
//! // The callback fires once per newly-observed message with an opaque
//! // `Key` (used later for redaction), the causal `Version`, and the value.
//! // It's `FnMut + Send` and may freely borrow local state for the
//! // duration of the await.
//! let mut observed = 0usize;
//! alice.message(
//!     ["hello".to_string(), "world".to_string()],
//!     |_key, _version, _message| {
//!         observed += 1;
//!         async {}
//!     },
//! ).await;
//! assert_eq!(observed, 2);
//! # }
//! ```
//!
//! # Redaction
//!
//! Any peer can [`redact`](Known::redact) a [`Key`] it holds; the redaction
//! propagates to every connected peer without consensus, so a single peer's
//! local decision evicts the message network-wide.
//!
//! ```
//! use rumors::{Known, Key};
//!
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() {
//! let mut alice = Known::seed();
//! let mut keys: Vec<Key> = Vec::new();
//! alice.message(
//!     ["stale rumor".to_string()],
//!     |k, _, _| {
//!         keys.push(k);
//!         async {}
//!     },
//! ).await;
//! alice.redact(keys);
//! # }
//! ```
//!
//! # Concurrent rumor sets
//!
//! Every [`Known`] carries its own Interval Tree Clock party and may originate
//! [`message`](Known::message)s and [`redact`](Known::redact)ions. To work
//! against a peer concurrently, [`fork`](Known::fork) a `Known`: a *true causal
//! fork* that mints a fresh disjoint party sharing the current observations
//! (the underlying tree is structurally shared, copy-on-write). The two halves
//! act independently, then reunite via [`Known::learn`] (observing everything
//! new) or [`Known::join`] (the fallible merge that rejoins their parties). A
//! `Known` is [`!Clone`](Clone) — the only way to get another working copy is
//! [`fork`](Known::fork). See the [`Known`] docs for the full discussion.
//!
//! # Gossiping with peers on the network
//!
//! Pass an [`AsyncRead`] reader and an [`AsyncWrite`] writer into
//! [`Known::gossip`]:
//!
//! ```
//! use rumors::{Known, ignore};
//!
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() {
//! // Stand in for a real network with an in-memory bidirectional pipe.
//! let (a, b) = tokio::io::duplex(1024);
//! let (mut a_r, mut a_w) = tokio::io::split(a);
//! let (mut b_r, mut b_w) = tokio::io::split(b);
//!
//! let mut alice: Known<String> = Known::seed();
//! alice.message(["hello".to_string()], ignore).await;
//! let bob: Known<String> = Known::seed();
//!
//! // Drive both ends concurrently; bob learns "hello".
//! let (alice, bob) = tokio::join!(
//!     alice.gossip(&mut a_r, &mut a_w, ignore),
//!     bob.gossip(&mut b_r, &mut b_w,
//!         move |_, _, m: &std::sync::Arc<String>| {
//!             assert_eq!(m.as_ref(), "hello");
//!             async {}
//!         },
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
//! (`b"RUMORS"`) followed by [`PROTOCOL_VERSION`] as a big-endian `u16`.
//! Reading bytes that don't start with `RUMORS` surfaces as
//! [`Error::MagicMismatch`]; the connection is not a `rumors` stream at
//! all.
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
//! recommended** and is the caller's responsibility.

use std::{
    future::{Future, Ready, ready},
    pin::Pin,
    sync::Arc,
};

use before::Party;
use borsh::{BorshDeserialize, BorshSerialize};
use tokio::io::{AsyncRead, AsyncWrite};

pub mod sync;

mod imbl_borsh;
mod message;
mod tree;
mod version;

use message::Message;
use tree::{Action, Tree, mirror};

/// Magic bytes that prefix every `rumors` gossip session: `b"RUMORS"`.
///
/// Sent as the first six bytes of the [handshake](Known::gossip), a peer
/// whose preamble starts with anything else is rejected with
/// [`Error::MagicMismatch`] before any rumor-set state is touched.
pub const PROTOCOL_MAGIC: [u8; 6] = *b"RUMORS";

/// On-the-wire protocol version, exchanged in the [handshake](Known::gossip)
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
/// Every `Known` owns an Interval Tree Clock party and may originate messages
/// and redactions. It deliberately does *not* implement [`Clone`]: duplicating
/// a live party would break the linearity the clocks require. To obtain another
/// working copy, [`fork`](Known::fork) it — a *true causal fork* that mints a
/// fresh disjoint party sharing the current observations (the underlying tree
/// is structurally shared, copy-on-write). Concurrent code holds one fork per
/// thread or task and recombines them via [`learn`](Known::learn) /
/// [`join`](Known::join).
///
/// Methods take `AsyncFnMut` callbacks; for synchronous I/O and callbacks,
/// see [`sync::Known`].
///
/// # Uniqueness of parties
///
/// Every party in one universe must descend from a single [`seed`](Known::seed)
/// by [`fork`](Known::fork). Forking splits a party into two disjoint regions,
/// and [`learn`](Known::learn) / [`join`](Known::join) rejoin them; because the
/// regions are disjoint, each party's history stays causally well-defined no
/// matter how forks and merges interleave.
///
/// The one rule the caller must uphold is *not mixing universes*: two `Known`s
/// from independent [`seed`](Known::seed) calls share no causal history and
/// must **never** gossip with each other.
///
/// # Example
///
/// ```
/// use rumors::{Known, Key};
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// let mut alice = Known::seed();
/// let mut keys: Vec<Key> = Vec::new();
/// alice.message(
///     ["hello".to_string(), "world".to_string()],
///     |key, _, _| {
///         keys.push(key);
///         async {}
///     },
/// ).await;
/// alice.redact([keys[0]]);
/// # }
/// ```
#[derive(Debug, Eq)]
pub struct Known<T> {
    /// This rumor set's [`Party`]: an Interval Tree Clock identity descended
    /// from a common [`seed`](Known::seed) by disjoint [`fork`](Known::fork)s.
    /// It is `!Clone`, so a `Known` is `!Clone` too: the only way to obtain
    /// another working copy is [`fork`](Known::fork), which mints a fresh
    /// disjoint party. This enforces the ITC linearity law at the type level —
    /// no two live `Known`s ever share a party region.
    party: Party,
    tree: Tree<T>,
}

/// Two rumor sets are equal when they hold the same observations — the same
/// tree — regardless of which [`Party`] observed them. A [`fork`](Known::fork)
/// therefore compares equal to its parent until one of them originates anew.
impl<T> PartialEq for Known<T> {
    fn eq(&self, other: &Self) -> bool {
        self.tree == other.tree
    }
}

/// The error type returned by [`Known::gossip`].
///
/// Surfaces I/O failures from the underlying reader/writer as well as
/// framing errors encountered while parsing messages off the wire.
pub use mirror::remote::Error;

/// An opaque identifier for a single message in a [`Known`] rumor set.
///
/// Keys are produced by the `on_message` callbacks of [`Known::message`],
/// [`Known::learn`], and [`Known::gossip`], and are stable across peers:
/// a key obtained from one peer can redact the message on any other.
///
/// Two content-identical messages always receive distinct keys: every
/// insert advances the local version vector before the key is derived.
///
/// # Example
///
/// ```
/// use rumors::{Known, Key};
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// let mut alice = Known::seed();
/// let mut keys: Vec<Key> = Vec::new();
/// alice.message(
///     ["echo".to_string(), "echo".to_string()],
///     |k, _, _| {
///         keys.push(k);
///         async {}
///     },
/// ).await;
/// assert_ne!(keys[0], keys[1]);
/// # }
/// ```
pub use tree::Key;

/// A causal version vector tagging when a message was observed.
///
/// Surfaced to the `on_message` callbacks of [`Known::message`],
/// [`Known::learn`], and [`Known::gossip`]. [`PartialOrd`] captures causal
/// ordering: `a <= b` iff every party's counter in `a` is at most the
/// corresponding counter in `b`. Versions produced by concurrent events are
/// incomparable (`partial_cmp` returns `None`).
///
/// # Example
///
/// ```
/// use rumors::{Known, Version};
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// let mut alice = Known::seed();
/// let mut versions: Vec<Version> = Vec::new();
/// alice.message(
///     ["first".to_string(), "second".to_string()],
///     |_, v, _| {
///         versions.push(v.clone());
///         async {}
///     },
/// ).await;
/// // Successive messages from the same party are causally *comparable* — one
/// // strictly precedes the other. (Callback order is unspecified, so we don't
/// // assume which one arrives first; concurrent events would be incomparable.)
/// assert!(versions[0] != versions[1]);
/// assert!(versions[0] < versions[1] || versions[1] < versions[0]);
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
/// use rumors::{Known, borsh, ignore};
///
/// #[derive(borsh::BorshSerialize, borsh::BorshDeserialize)]
/// struct Rumor { subject: String, count: u32 }
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// let mut alice = Known::seed();
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
/// # Example
///
/// ```
/// use rumors::{Known, ignore};
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// let mut alice = Known::seed();
/// alice.message(["hello".to_string(), "world".to_string()], ignore).await;
/// # }
/// ```
pub fn ignore<T>(_key: Key, _version: &Version, _message: &Arc<T>) -> Ready<()> {
    ready(())
}

impl<T> Known<T> {
    /// Create the distinguished seed rumor set: the single root [`Party`] from
    /// which every other party in this universe descends by [`fork`](Self::fork).
    ///
    /// Call this exactly once per universe of cooperating peers. Additional
    /// originating peers are minted by [`fork`](Self::fork)ing an existing
    /// `Known`, never by calling `seed` again: two independently-seeded
    /// universes share no causal history and must never gossip (the `before`
    /// crate's Law of Disjointness). The caller owns this uniqueness — unlike
    /// the previous version-vector design, `rumors` no longer tracks parties
    /// process-globally, which lets several independent universes coexist in
    /// one program.
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::Known;
    ///
    /// let _alice: Known<String> = Known::seed();
    /// ```
    pub fn seed() -> Self {
        Known {
            party: Party::seed(),
            tree: Tree::new(),
        }
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
    /// use rumors::{Known, Key, Version};
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let mut alice = Known::seed();
    /// let mut observed: Vec<(Key, Version, String)> = Vec::new();
    /// alice.message(
    ///     ["hello".to_string(), "world".to_string()],
    ///     |key, version, message| {
    ///         observed.push((key, version.clone(), message.as_ref().clone()));
    ///         async {}
    ///     },
    /// ).await;
    /// assert_eq!(observed.len(), 2);
    /// # }
    /// ```
    pub async fn message<'a, OnMessage, OnMessageFut, I>(
        &'a mut self,
        messages: I,
        mut on_message: OnMessage,
    ) where
        T: BorshSerialize + Send + Sync + 'a,
        I: IntoIterator<Item = T> + Send,
        I::IntoIter: Send,
        OnMessage: FnMut(Key, &Version, &Arc<T>) -> OnMessageFut + Send + 'a,
        OnMessageFut: Future<Output = ()> + Send + 'a,
    {
        self.tree
            .act(
                &self.party,
                messages.into_iter().map(Message::from).map(Action::Insert),
                move |k: Key,
                      v: &Version,
                      m: Option<&Message<T>>|
                      -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
                    match m {
                        Some(m) => Box::pin(on_message(k, v, m.as_ref())),
                        None => Box::pin(ready(())),
                    }
                },
            )
            .await;
    }

    /// Lazily iterate every message currently live in this rumor set, as
    /// `(Key, &Version, &Arc<T>)`, in unspecified order.
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::{Known, ignore};
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let mut alice = Known::seed();
    /// alice.message(["a".to_string(), "b".to_string()], ignore).await;
    /// let mut live: Vec<String> = alice.iter().map(|(_, _, m)| m.as_ref().clone()).collect();
    /// live.sort();
    /// assert_eq!(live, vec!["a".to_string(), "b".to_string()]);
    /// # }
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = (Key, &Version, &Arc<T>)> + Send + Sync
    where
        T: Send + Sync,
    {
        self.tree.iter()
    }

    /// Redact the given keys: stop gossiping the corresponding messages, and
    /// instruct every peer we synchronize with to do the same.
    ///
    /// Each [`Key`] was originally surfaced by an `on_message` callback in
    /// [`Known::message`], [`Known::learn`], or [`Known::gossip`].
    /// Redaction is contagious, so a single peer's call evicts the message
    /// network-wide without consensus.
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::{Known, Key};
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let mut alice = Known::seed();
    /// let mut keys: Vec<Key> = Vec::new();
    /// alice.message(
    ///     ["transient announcement".to_string()],
    ///     |k, _, _| {
    ///         keys.push(k);
    ///         async {}
    ///     },
    /// ).await;
    /// alice.redact(keys);
    /// # }
    /// ```
    pub fn redact<I: IntoIterator<Item = Key>>(&mut self, redacted: I)
    where
        T: Send + Sync,
    {
        pollster::block_on(self.tree.act(
            &self.party,
            redacted.into_iter().map(Action::Forget),
            |_, _, _| ready(()),
        ));
    }

    /// Fork off a new rumor set with its own disjoint [`Party`], sharing this
    /// set's current observations.
    ///
    /// This is a *true causal fork*: it splits the underlying ITC [`Party`] in
    /// two, so the returned `Known` is a fully independent peer — it may
    /// [`message`](Self::message), [`redact`](Self::redact),
    /// [`gossip`](Self::gossip), and be [`fork`](Self::fork)ed again, all
    /// concurrently with `self`. The tree is shared structurally
    /// (copy-on-write), so the fork starts from the same observations, but the
    /// two parties' futures are independent and causally concurrent. Reunite a
    /// fork with [`learn`](Self::learn) / [`join`](Self::join), which
    /// merges the trees and rejoins the parties.
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::{Known, ignore};
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let mut alice = Known::seed();
    /// alice.message(["hello".to_string()], ignore).await;
    ///
    /// // The fork shares alice's observations but has its own party.
    /// let snapshot = alice.fork();
    /// assert_eq!(alice, snapshot);
    /// # }
    /// ```
    pub fn fork(&mut self) -> Known<T> {
        Known {
            party: self.party.fork(),
            tree: self.tree.clone(),
        }
    }

    /// Merge `other` into `self`, invoking `on_message` for each message in
    /// `other` that `self` had not already observed, and reuniting `other`'s
    /// [`Party`] back into `self`'s.
    ///
    /// **Delivery is unordered**: callbacks fire in arbitrary order, including
    /// orderings that violate the causal precedence captured by each message's
    /// [`Version`]. Sort by [`Version`] if your application needs causal or
    /// insertion ordering.
    ///
    /// # Errors
    ///
    /// Returns `Err(other)`, handing `other` back untouched, if the two parties
    /// are **not disjoint**, i.e. they do not descend from a common
    /// [`seed`](Self::seed) by un-rejoined forks (for example, they come from
    /// two different seeds). In correct linear usage this never happens; it is
    /// surfaced rather than panicking so a caller who mixes universes can
    /// recover.
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::{Known, ignore};
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let mut alice = Known::seed();
    /// let mut bob = alice.fork();
    /// bob.message(["news from bob".to_string()], ignore).await;
    ///
    /// let mut learned: Vec<String> = Vec::new();
    /// alice.learn(bob, |_, _, m| {
    ///     learned.push(m.as_ref().clone());
    ///     async {}
    /// }).await.unwrap();
    /// assert_eq!(learned, vec!["news from bob".to_string()]);
    /// # }
    /// ```
    pub async fn learn<'a, OnMessage, OnMessageFut>(
        &'a mut self,
        other: Known<T>,
        on_message: OnMessage,
    ) -> Result<(), Known<T>>
    where
        T: Send + Sync + 'a,
        OnMessage: FnMut(Key, &Version, &Arc<T>) -> OnMessageFut + Send + 'a,
        OnMessageFut: std::future::Future<Output = ()> + Send + 'a,
    {
        let Known {
            party: other_party,
            tree: other_tree,
        } = other;

        // `Party::join` *is* the disjointness check: on success it merges the
        // other party's region into ours; on overlap (different seeds, or an
        // already-rejoined pair) it leaves us untouched and hands the party
        // back. Doing it first means a non-disjoint `other` is returned whole
        // with nothing mutated — no separate `is_disjoint` probe needed.
        if let Err(party) = self.party.join(other_party) {
            return Err(Known {
                party,
                tree: other_tree,
            });
        }

        // Instantiate the two sides of the mirror exchange, both local
        let l = mirror::local::Exchange::start(self.tree.root.clone(), ignore, on_message);
        let r = mirror::local::Exchange::start(other_tree.root, ignore, ignore);

        // Drive them to completion: we know they don't need a "real" executor
        Ok((self.tree.root, _)) = mirror(l, r).await;

        Ok(())
    }

    /// Synchronize rumor sets with a remote peer, invoking `on_message` for
    /// each message learned from the peer.
    ///
    /// `read` and `write` must implement [`AsyncRead`] / [`AsyncWrite`];
    /// both ends of the connection must drive `gossip` concurrently. The
    /// callback signature matches [`Known::message`]. For synchronous I/O,
    /// use [`sync::Known::gossip`].
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
    /// use std::sync::Arc;
    /// use rumors::{Known, ignore};
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let (a, b) = tokio::io::duplex(1024);
    /// let (mut a_r, mut a_w) = tokio::io::split(a);
    /// let (mut b_r, mut b_w) = tokio::io::split(b);
    ///
    /// let mut alice: Known<String> = Known::seed();
    /// alice.message(["hello".to_string()], ignore).await;
    /// let bob: Known<String> = Known::seed();
    ///
    /// let mut bob_learned: Vec<String> = Vec::new();
    /// let (alice, bob) = tokio::join!(
    ///     alice.gossip(&mut a_r, &mut a_w, ignore),
    ///     bob.gossip(&mut b_r, &mut b_w, |_, _, m: &Arc<String>| {
    ///         bob_learned.push(m.as_ref().clone());
    ///         async {}
    ///     }),
    /// );
    /// let (_alice, _bob) = (alice.unwrap(), bob.unwrap());
    /// assert_eq!(bob_learned, vec!["hello".to_string()]);
    /// # }
    /// ```
    pub async fn gossip<'a, OnMessage, OnMessageFut, R, W>(
        mut self,
        read: &'a mut R,
        write: &'a mut W,
        on_message: OnMessage,
    ) -> Result<Self, Error>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync + 'a,
        R: AsyncRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
        OnMessage: FnMut(Key, &Version, &Arc<T>) -> OnMessageFut + Send + 'a,
        OnMessageFut: Future<Output = ()> + Send + 'a,
    {
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
    }

    /// Reunite `other` into `self`, discarding per-message observations.
    ///
    /// A blocking, ignore-the-callback convenience for
    /// [`learn`](Self::learn): it merges `other`'s history into `self` and
    /// rejoins their parties (the inverse of [`fork`](Self::fork)). Returns
    /// `Err(other)` if the two parties are not disjoint — see
    /// [`learn`](Self::learn) for when that arises.
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::{Known, ignore};
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let mut alice = Known::seed();
    /// let mut bob = alice.fork();
    /// bob.message(["news".to_string()], ignore).await;
    /// alice.join(bob).unwrap();
    /// # }
    /// ```
    pub fn join(&mut self, other: Known<T>) -> Result<(), Known<T>>
    where
        T: Send + Sync,
    {
        pollster::block_on(self.learn(other, |_, _, _| ready(())))
    }
}
