//! Synchronous rumor sets with redaction.
//!
//! `rumors::sync` is a CRDT-backed gossip set with `FnMut` callbacks and
//! [`std::io::Read`] / [`std::io::Write`] I/O. Use this module when your I/O
//! is synchronous (e.g. [`std::net::TcpStream`]).
//!
//! # Quickstart
//!
//! ```
//! use rumors::sync::Known;
//!
//! // The distinguished seed rumor set; further peers are made by `fork`.
//! let mut alice = Known::seed();
//!
//! // The callback fires once per newly-observed message with an opaque
//! // `Key` (used later for redaction), the causal `Version`, and the value.
//! // It's `FnMut + Send` and may freely borrow local state.
//! let mut observed = 0usize;
//! alice.message(
//!     ["hello".to_string(), "world".to_string()],
//!     |_key, _version, _message| observed += 1,
//! );
//! assert_eq!(observed, 2);
//! ```
//!
//! # Redaction
//!
//! Any peer can [`redact`](Known::redact) a [`Key`] it holds; the redaction
//! propagates to every connected peer without consensus, so a single peer's
//! local decision evicts the message network-wide.
//!
//! ```
//! use rumors::sync::{Known, Key};
//!
//! let mut alice = Known::seed();
//! let mut keys: Vec<Key> = Vec::new();
//! alice.message(["stale rumor".to_string()], |k, _, _| keys.push(k));
//! alice.redact(keys);
//! ```
//!
//! # Concurrent rumor sets
//!
//! Every [`Known`] carries its own Interval Tree Clock party and may originate
//! [`message`](Known::message)s and [`redact`](Known::redact)ions. To work
//! against a peer concurrently, [`fork`](Known::fork) a `Known`: this is a
//! *true causal fork* that mints a fresh disjoint party sharing the current
//! observations (copy-on-write), so both halves can act independently. Reunite
//! a fork with [`learn`](Known::learn) / [`join`](Known::join), which
//! merges the histories and rejoins the parties. A `Known` is `!Clone` — the
//! only way to get another working copy is [`fork`](Known::fork).
//!
//! # Gossiping with peers on the network
//!
//! Pass a [`Read`] reader and a [`Write`] writer into [`Known::gossip`]; both
//! ends must drive `gossip` concurrently (typically on separate threads):
//!
//! ```no_run
//! use rumors::sync::{Known, ignore};
//! use std::net::TcpStream;
//!
//! let mut write = TcpStream::connect("127.0.0.1:9000").unwrap();
//! let mut read = write.try_clone().unwrap();
//! let alice: Known<String> = Known::seed();
//! let _alice = alice.gossip(&mut read, &mut write, ignore).unwrap();
//! ```
//!
//! # Message serialization
//!
//! Messages are serialized with [`borsh`], which is re-exported so callers
//! can derive [`BorshSerialize`] / [`BorshDeserialize`] on their message
//! types without taking a separate dependency.

use std::io::{Read, Write};
use std::sync::Arc;

use borsh::{BorshDeserialize, BorshSerialize};
use futures::io::AllowStdIo;
use tokio_util::compat::{FuturesAsyncReadCompatExt, FuturesAsyncWriteCompatExt};

pub use crate::{Error, Key, PROTOCOL_MAGIC, PROTOCOL_VERSION, Version};
pub use ::borsh;

/// A local set of rumors: add to it, redact from it, gossip with peers.
///
/// Each `Known` owns an Interval Tree Clock party and may originate messages
/// and redactions. It is `!Clone`; obtain another working copy with
/// [`fork`](Known::fork), a true causal fork that mints a fresh disjoint party
/// sharing the current observations. Reunite forks with
/// [`learn`](Known::learn) / [`join`](Known::join).
///
/// # Uniqueness of parties
///
/// All parties in one universe must descend from a single [`seed`](Known::seed)
/// by [`fork`](Known::fork). The caller must not let two independently-seeded
/// universes gossip with each other (the `before` crate's Law of Disjointness);
/// `rumors` no longer tracks parties process-globally, so several independent
/// universes may coexist in one program.
///
/// # Example
///
/// ```
/// use rumors::sync::{Known, Key};
///
/// let mut alice = Known::seed();
/// let mut keys: Vec<Key> = Vec::new();
/// alice.message(
///     ["hello".to_string(), "world".to_string()],
///     |key, _, _| keys.push(key),
/// );
/// alice.redact([keys[0]]);
/// ```
#[derive(Debug, Eq)]
pub struct Known<T>(pub crate::Known<T>);

impl<T> PartialEq for Known<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

/// An `on_message` callback that discards every observation.
///
/// Pass this when you only care about mutating the rumor set, not about
/// inspecting individual messages.
///
/// # Example
///
/// ```
/// use rumors::sync::{Known, ignore};
///
/// let mut alice = Known::seed();
/// alice.message(["hello".to_string(), "world".to_string()], ignore);
/// ```
pub fn ignore<T>(_key: Key, _version: &Version, _message: &Arc<T>) {}

impl<T> Known<T> {
    /// Create the distinguished seed rumor set: the single root party from
    /// which every other party in this universe descends by
    /// [`fork`](Self::fork).
    ///
    /// Call this exactly once per universe of cooperating peers; make
    /// additional peers with [`fork`](Self::fork), never by calling `seed`
    /// again.
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::sync::Known;
    ///
    /// let _alice: Known<String> = Known::seed();
    /// ```
    pub fn seed() -> Self {
        Known(crate::Known::seed())
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
    /// use rumors::sync::{Known, Key, Version};
    ///
    /// let mut alice = Known::seed();
    /// let mut observed: Vec<(Key, Version, String)> = Vec::new();
    /// alice.message(
    ///     ["hello".to_string(), "world".to_string()],
    ///     |key, version, message| {
    ///         observed.push((key, version.clone(), message.as_ref().clone()));
    ///     },
    /// );
    /// assert_eq!(observed.len(), 2);
    /// ```
    pub fn message<'a, OnMessage, I>(&'a mut self, messages: I, mut on_message: OnMessage)
    where
        T: BorshSerialize + Send + Sync + 'a,
        I: IntoIterator<Item = T> + Send,
        I::IntoIter: Send,
        OnMessage: FnMut(Key, &Version, &Arc<T>) + Send + 'a,
    {
        // Wrap the sync `FnMut` into the `FnMut(...) -> Send + 'a future`
        // shape the async message wants. The body runs synchronously; the
        // returned `Ready` future resolves immediately.
        pollster::block_on(self.0.message(messages, move |k, v, m| {
            on_message(k, v, m);
            std::future::ready(())
        }));
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
    /// use rumors::sync::{Known, Key};
    ///
    /// let mut alice = Known::seed();
    /// let mut keys: Vec<Key> = Vec::new();
    /// alice.message(["transient".to_string()], |k, _, _| keys.push(k));
    /// alice.redact(keys);
    /// ```
    pub fn redact<I: IntoIterator<Item = Key>>(&mut self, redacted: I)
    where
        T: Send + Sync,
    {
        self.0.redact(redacted);
    }

    /// Fork off a new rumor set with its own disjoint party, sharing this set's
    /// current observations.
    ///
    /// A *true causal fork*: the returned `Known` is a fully independent peer
    /// (it may [`message`](Self::message), [`redact`](Self::redact),
    /// [`gossip`](Self::gossip), and be [`fork`](Self::fork)ed again), sharing
    /// the tree copy-on-write. Reunite with [`learn`](Self::learn) /
    /// [`join`](Self::join).
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::sync::{Known, ignore};
    ///
    /// let mut alice = Known::seed();
    /// alice.message(["hello".to_string()], ignore);
    ///
    /// let snapshot = alice.fork();
    /// assert_eq!(alice, snapshot);
    /// ```
    pub fn fork(&mut self) -> Known<T> {
        Known(self.0.fork())
    }

    /// Lazily iterate every message currently live in this rumor set, as
    /// `(Key, &Version, &Arc<T>)`, in unspecified order.
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::sync::{Known, ignore};
    ///
    /// let mut alice = Known::seed();
    /// alice.message(["a".to_string(), "b".to_string()], ignore);
    /// let mut live: Vec<String> = alice.iter().map(|(_, _, m)| m.as_ref().clone()).collect();
    /// live.sort();
    /// assert_eq!(live, vec!["a".to_string(), "b".to_string()]);
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = (Key, &Version, &Arc<T>)> + Send + Sync
    where
        T: Send + Sync,
    {
        self.0.iter()
    }

    /// Merge `other` into `self`, invoking `on_message` for each message in
    /// `other` that `self` had not already observed, and reuniting `other`'s
    /// party back into `self`'s.
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
    /// use rumors::sync::{Known, ignore};
    ///
    /// let mut alice = Known::seed();
    /// let mut bob = alice.fork();
    /// bob.message(["news from bob".to_string()], ignore);
    ///
    /// let mut learned: Vec<String> = Vec::new();
    /// alice.learn(bob, |_, _, m| learned.push(m.as_ref().clone())).unwrap();
    /// assert_eq!(learned, vec!["news from bob".to_string()]);
    /// ```
    pub fn learn<'a, OnMessage>(
        &'a mut self,
        other: Known<T>,
        mut on_message: OnMessage,
    ) -> Result<(), Known<T>>
    where
        T: Send + Sync + 'a,
        OnMessage: FnMut(Key, &Version, &Arc<T>) + Send + 'a,
    {
        pollster::block_on(self.0.learn(other.0, move |k, v, m| {
            on_message(k, v, m);
            std::future::ready(())
        }))
        .map_err(Known)
    }

    /// Reunite `other` into `self`, discarding per-message observations.
    ///
    /// A callback-free convenience for [`learn`](Self::learn). Returns
    /// `Err(other)` if the two parties are not disjoint.
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::sync::{Known, ignore};
    ///
    /// let mut alice = Known::seed();
    /// let mut bob = alice.fork();
    /// bob.message(["news".to_string()], ignore);
    /// alice.join(bob).unwrap();
    /// ```
    pub fn join(&mut self, other: Known<T>) -> Result<(), Known<T>>
    where
        T: Send + Sync,
    {
        self.0.join(other.0).map_err(Known)
    }

    /// Synchronize rumor sets with a remote peer, invoking `on_message` for
    /// each message learned from the peer.
    ///
    /// `read` and `write` must implement [`Read`] / [`Write`]; both ends of
    /// the connection must drive `gossip` concurrently (typically on
    /// separate threads). The callback signature matches [`Known::message`].
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
    /// ```no_run
    /// use rumors::sync::{Known, ignore};
    /// use std::net::TcpStream;
    ///
    /// let mut write = TcpStream::connect("127.0.0.1:9000").unwrap();
    /// let mut read = write.try_clone().unwrap();
    /// let alice: Known<String> = Known::seed();
    /// let _alice = alice.gossip(&mut read, &mut write, ignore).unwrap();
    /// ```
    pub fn gossip<'a, OnMessage, R, W>(
        self,
        read: &'a mut R,
        write: &'a mut W,
        mut on_message: OnMessage,
    ) -> Result<Self, Error>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync + 'a,
        R: Read + Unpin + Send,
        W: Write + Unpin + Send,
        OnMessage: FnMut(Key, &Version, &Arc<T>) + Send + 'a,
    {
        // Bridge the synchronous reader/writer to the async I/O the protocol
        // expects: `AllowStdIo` adapts `Read`/`Write` to `futures::io`'s async
        // traits, and the tokio-compat layer adapts those to `tokio::io`'s.
        let mut read = AllowStdIo::new(read).compat();
        let mut write = AllowStdIo::new(write).compat_write();
        // Wrap the sync `FnMut` into the `FnMut(...) -> Send future`
        // shape the async gossip wants. The body runs synchronously; the
        // returned `Ready` future resolves immediately.
        pollster::block_on(self.0.gossip(&mut read, &mut write, move |k, v, m| {
            on_message(k, v, m);
            std::future::ready(())
        }))
        .map(Known)
    }
}
