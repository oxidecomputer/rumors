//! Synchronous rumor sets with redaction.
//!
//! `rumors` is a protocol for efficient unordered gossip for sets of causally
//! versioned messages with redaction. Each peer holds a [`Known<T>`] rumor set;
//! peers reconcile by exchanging only the parts that differ. Redacting a
//! message stops it propagating, and redactions spread contagiously to every
//! peer the redactor (transitively) gossips with.
//!
//! This is the synchronous interface: blocking [`std::io::Read`] /
//! [`std::io::Write`] I/O and `FnMut` callbacks. For an async interface
//! (`AsyncRead`/`AsyncWrite`, async callbacks), see the [`rumors`](crate) crate
//! root.
//!
//! # Quickstart
//!
//! ```
//! use rumors::sync::Known;
//!
//! // `seed` mints the distinguished root rumor set for a universe of peers;
//! // additional originating peers are made by [`Known::bootstrap`], never by a
//! // second `seed`.
//! let mut alice = Known::seed();
//!
//! // The callback fires once per newly-observed message with an opaque
//! // `Key` (used later for redaction), the causal `Version`, and the value.
//! // It's `FnMut + Send` and may freely borrow local state.
//! let mut observed = 0usize;
//! alice.message_then(
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
//! alice.message_then(["stale rumor".to_string()], |k, _, _| keys.push(k));
//! alice.redact(keys);
//! ```
//!
//! # Concurrent rumor sets
//!
//! A `Known` is [`!Clone`](Clone). For a cheap, copy-on-write working snapshot
//! that shares the originator's party, take a [`rumors`](Known::rumors)
//! snapshot; gossip with it, then [`join`](Known::join) it back into its
//! originator to absorb what it learned. A second *originating* peer, with
//! its own disjoint party, comes from [`Known::bootstrap`] over the wire.
//! A snapshot cannot originate [`message`](Known::message)s or
//! [`redact`](Known::redact)ions; only the originating `Known` can, which keeps
//! its events linearized. A [`rumors`](Known::rumors) snapshot is reunited with
//! its originator via [`Known::join_then`] (observing everything new from it) or
//! [`Known::join`].
//!
//! # Gossiping over the network
//!
//! On a real network, a brand-new process does not have a [`Known`]. It
//! acquires one by *bootstrapping* from an established peer. The newcomer
//! drives [`bootstrap`](Known::bootstrap); the established peer drives its
//! usual [`gossip`](Known::gossip), which transparently serves the bootstrap
//! request. Once the newcomer has a [`Known`], it can then
//! [`gossip`](Known::gossip) with others, including allowing others to
//! [`bootstrap`](Known::bootstrap) from itself.
//!
//! Both [`bootstrap`](Known::bootstrap) and [`gossip`](Known::gossip) take a
//! [`Read`] reader and a [`Write`] writer. Because the calls block, the two
//! ends must run concurrently — typically on separate threads. Here two
//! [`std::io::pipe`]s stand in for a TCP connection:
//!
//! ```
//! use rumors::sync::Known;
//! use std::io::pipe;
//! use std::thread;
//!
//! // One pipe carries alice -> bob, the other bob -> alice.
//! let (mut a2b_r, mut a2b_w) = pipe().unwrap();
//! let (mut b2a_r, mut b2a_w) = pipe().unwrap();
//!
//! // `alice` is an established peer with some content.
//! let mut alice: Known<String> = Known::seed();
//! alice.message(["hello".to_string()]);
//!
//! // `bob` is a fresh process on its own thread. It bootstraps from alice,
//! // who serves a copy of her tree and a freshly-forked party. The thread
//! // hands the pipe halves back so we can reuse the connection below.
//! let bob = thread::spawn(move || {
//!     let bob = Known::<String>::bootstrap(&mut a2b_r, &mut b2a_w)
//!         .expect("handshake")
//!         .expect("alice served the bootstrap");
//!     (bob, a2b_r, b2a_w)
//! });
//! let mut alice = alice.gossip(&mut b2a_r, &mut a2b_w).unwrap();
//! let (mut bob, mut a2b_r, mut b2a_w) = bob.join().unwrap();
//!
//! // bob now belongs to alice's network and holds her observations.
//! assert_eq!(alice, bob);
//!
//! // If both add messages they diverge, but gossiping reconciles them again:
//! alice.message(["from alice".to_string()]);
//! bob.message(["from bob".to_string()]);
//! assert!(alice != bob);
//!
//! let bob = thread::spawn(move || bob.gossip(&mut a2b_r, &mut b2a_w).unwrap());
//! let alice = alice.gossip(&mut b2a_r, &mut a2b_w).unwrap();
//! assert_eq!(alice, bob.join().unwrap());
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

pub use crate::{
    Error, Facts, Key, Network, PROTOCOL_MAGIC, PROTOCOL_VERSION, RetireError, Rumors, Version,
};
pub use ::borsh;

/// A local set of rumors: add to it, redact from it, gossip with peers.
///
/// A canonical `Known` owns an Interval Tree Clock party and may originate
/// messages and redactions. It is `!Clone`; for a working copy take a
/// [`rumors`](Known::rumors) snapshot, a cheap copy-on-write view that
/// shares the originator's party and cannot originate, then fold it back
/// with [`join_then`](Known::join_then) or [`join`](Known::join). A second
/// originating peer, with its own disjoint party, comes from
/// [`bootstrap`](Known::bootstrap).
///
/// # Uniqueness of parties
///
/// Every party in one universe descends from a single [`seed`](Known::seed),
/// either by a [`rumors`](Known::rumors) snapshot (sharing it) or by a
/// [`bootstrap`](Known::bootstrap)ed peer (a fresh disjoint region). Parties
/// are not tracked process-globally, so several independent universes may
/// coexist in one program; the caller must not let two independently-seeded
/// universes gossip with each other (the `before` crate's Law of
/// Disjointness).
///
/// # Example
///
/// ```
/// use rumors::sync::{Known, Key};
///
/// let mut alice = Known::seed();
/// let mut keys: Vec<Key> = Vec::new();
/// alice.message_then(
///     ["hello".to_string(), "world".to_string()],
///     |key, _, _| keys.push(key),
/// );
/// alice.redact([keys[0]]);
/// ```
pub struct Known<T, S = Facts>(pub(crate) crate::Known<T, S>);

impl<T, S> Eq for Known<T, S> {}
impl<T, S, U> PartialEq<Known<T, U>> for Known<T, S> {
    fn eq(&self, other: &Known<T, U>) -> bool {
        self.0 == other.0
    }
}

impl<T: std::fmt::Debug, S> std::fmt::Debug for Known<T, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.0, f)
    }
}

/// Adapt a synchronous `on_message` into the asynchronous shape the inner
/// [`crate::Known`] expects: the body runs synchronously and the returned
/// future resolves immediately.
///
/// The return-position `impl FnMut(..) -> Ready<()>` pins the adapted callback
/// to a higher-ranked signature, which is what lets the adapted closure flow
/// into the async layer without a "not general enough" lifetime error.
fn into_async<T, F>(
    mut on_message: F,
) -> impl FnMut(Key, &Version, &Arc<T>) -> std::future::Ready<()>
where
    F: FnMut(Key, &Version, &Arc<T>),
{
    move |k: Key, v: &Version, m: &Arc<T>| {
        on_message(k, v, m);
        std::future::ready(())
    }
}

impl<T> Known<T> {
    /// Create the distinguished seed rumor set: the single root party from
    /// which every other party in this universe descends.
    ///
    /// Call this exactly once per universe of cooperating peers; make
    /// additional originating peers with [`bootstrap`](Self::bootstrap),
    /// never by calling `seed` again.
    ///
    /// The universe's [`Network`] identifier is drawn from the operating
    /// system's secure RNG; use [`seed_rng`](Self::seed_rng) to supply your
    /// own.
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

    /// Like [`seed`](Self::seed), but draws the universe's [`Network`] identifier
    /// from a caller-supplied RNG instead of the OS RNG (e.g. a deterministic
    /// RNG in tests).
    ///
    /// # Example
    ///
    /// ```
    /// use rand::SeedableRng;
    /// use rand::rngs::StdRng;
    /// use rumors::sync::Known;
    ///
    /// let mut rng = StdRng::seed_from_u64(42);
    /// let _alice: Known<String> = Known::seed_rng(&mut rng);
    /// ```
    pub fn seed_rng<R: rand::RngCore + ?Sized>(rng: &mut R) -> Self {
        Known(crate::Known::seed_rng(rng))
    }

    /// Insert messages into the rumor set without observing them.
    ///
    /// The callback-free counterpart to [`message_then`](Self::message_then):
    /// use it when you only care about mutating the rumor set, not about the
    /// [`Key`]s or [`Version`]s of the inserted messages.
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::sync::Known;
    ///
    /// let mut alice = Known::seed();
    /// alice.message(["hello".to_string(), "world".to_string()]);
    /// ```
    pub fn message<'a, I>(&'a mut self, messages: I)
    where
        T: BorshSerialize + Send + Sync + 'a,
        I: IntoIterator<Item = T> + Send,
        I::IntoIter: Send,
    {
        pollster::block_on(self.0.message(messages));
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
    /// through the callback. To insert without a callback, use
    /// [`message`](Self::message).
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::sync::{Known, Key, Version};
    ///
    /// let mut alice = Known::seed();
    /// let mut observed: Vec<(Key, Version, String)> = Vec::new();
    /// alice.message_then(
    ///     ["hello".to_string(), "world".to_string()],
    ///     |key, version, message| {
    ///         observed.push((key, version.clone(), message.as_ref().clone()));
    ///     },
    /// );
    /// assert_eq!(observed.len(), 2);
    /// ```
    pub fn message_then<'a, OnMessage, I>(&'a mut self, messages: I, on_message: OnMessage)
    where
        T: BorshSerialize + Send + Sync + 'a,
        I: IntoIterator<Item = T> + Send,
        I::IntoIter: Send,
        OnMessage: FnMut(Key, &Version, &Arc<T>) + Send + 'a,
    {
        pollster::block_on(self.0.message_then(messages, into_async(on_message)));
    }

    /// Redact the given keys: stop gossiping the corresponding messages, and
    /// instruct every peer we synchronize with to do the same.
    ///
    /// Each [`Key`] was originally surfaced by an `on_message` callback in
    /// [`Known::message`], [`Known::join_then`], or [`Known::gossip`].
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
    /// alice.message_then(["transient".to_string()], |k, _, _| keys.push(k));
    /// alice.redact(keys);
    /// ```
    pub fn redact<I: IntoIterator<Item = Key>>(&mut self, redacted: I)
    where
        T: Send + Sync,
    {
        self.0.redact(redacted);
    }

    /// Take a snapshot of this [`Known`] that can gossip but cannot
    /// originate new [`message`](Known::message)s or
    /// [`redact`](Known::redact) existing ones. Gossip with the snapshot,
    /// then [`join`](Known::join) it back into its originator to absorb what
    /// it learned.
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::sync::{Known};
    ///
    /// let mut alice = Known::seed();
    /// alice.message(["hello".to_string()]);
    ///
    /// let snapshot = alice.rumors();
    /// assert_eq!(alice, snapshot);
    /// ```
    pub fn rumors(&self) -> Known<T, Rumors> {
        Known(self.0.rumors())
    }

    /// Merge `other`'s content into `self`, discarding per-message observations.
    ///
    /// The callback-free counterpart of [`join_then`](Self::join_then); because
    /// there is no callback, the merge elides the per-leaf discovery walk.
    /// Returns `Err(other)`, handing `other` back untouched, on a [`Network`]
    /// mismatch (two independently-seeded universes).
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::sync::Known;
    ///
    /// let mut alice = Known::seed();
    /// alice.message(["hello".to_string()]);
    ///
    /// // A snapshot shares alice's observations; joining it back is a content
    /// // union (in real use the snapshot would gossip and learn first).
    /// let snapshot = alice.rumors();
    /// alice.join(snapshot).unwrap();
    /// ```
    pub fn join(&mut self, other: Known<T, Rumors>) -> Result<(), Known<T, Rumors>>
    where
        T: Send + Sync,
    {
        self.0.join(other.0).map_err(Known)
    }

    /// Merge `other`'s content into `self`, invoking `on_message` for each
    /// message in `other` that `self` had not already observed.
    ///
    /// Delivery is unordered: callbacks fire in arbitrary order, including
    /// orderings that violate the causal precedence captured by each message's
    /// [`Version`]. Sort by [`Version`] if your application needs causal or
    /// insertion ordering.
    ///
    /// # Errors
    ///
    /// Returns `Err(other)`, handing `other` back untouched, on a [`Network`]
    /// mismatch: `self` and `other` descend from two different
    /// [`seed`](Self::seed)s and share no causal history. In correct usage this
    /// never happens; it is surfaced rather than panicking so a caller who mixes
    /// universes can recover.
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::sync::Known;
    ///
    /// let mut alice = Known::seed();
    /// alice.message(["hello".to_string()]);
    ///
    /// // A snapshot shares alice's observations; merging it back observes only
    /// // messages alice had not already seen (here none — in real use the
    /// // snapshot would gossip with a peer and learn first).
    /// let snapshot = alice.rumors();
    /// let mut learned: Vec<String> = Vec::new();
    /// alice.join_then(snapshot, |_, _, m| learned.push(m.as_ref().clone())).unwrap();
    /// assert!(learned.is_empty());
    /// ```
    pub fn join_then<'a, OnMessage>(
        &'a mut self,
        other: Known<T, Rumors>,
        on_message: OnMessage,
    ) -> Result<(), Known<T, Rumors>>
    where
        T: Send + Sync + 'a,
        OnMessage: FnMut(Key, &Version, &Arc<T>) + Send + 'a,
    {
        pollster::block_on(self.0.join_then(other.0, into_async(on_message))).map_err(Known)
    }

    /// Bootstrap a brand-new rumor set from a remote peer.
    ///
    /// The peer must already have a [`Known`] in hand; bootstrapping from a
    /// peer who is also bootstrapping results in `Ok(None)`.
    ///
    /// # Returns
    ///
    /// - `Ok(Some(known))`: success.
    /// - `Ok(None)`: the peer was *also* bootstrapping, so neither side had
    ///   anything to give the other.
    /// - `Err(_)`: handshake or transfer failure (see [`Error`]).
    ///
    /// To observe each message in the received tree, use
    /// [`bootstrap_then`](Self::bootstrap_then).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rumors::sync::Known;
    /// use std::net::TcpStream;
    ///
    /// let mut write = TcpStream::connect("127.0.0.1:9000").unwrap();
    /// let mut read = write.try_clone().unwrap();
    /// let bob: Option<Known<String>> = Known::bootstrap(&mut read, &mut write).unwrap();
    /// ```
    pub fn bootstrap<'a, R, W>(read: &'a mut R, write: &'a mut W) -> Result<Option<Self>, Error>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync + 'a,
        R: Read + Unpin + Send,
        W: Write + Unpin + Send,
    {
        // Bridge the synchronous reader/writer to the async I/O the protocol
        // expects, exactly as `gossip` does.
        let mut read = AllowStdIo::new(read).compat();
        let mut write = AllowStdIo::new(write).compat_write();
        pollster::block_on(crate::Known::<T>::bootstrap(&mut read, &mut write))
            .map(|known| known.map(Known))
    }

    /// Bootstrap a brand-new rumor set from a remote peer, invoking
    /// `on_message` once per message in the received [`Known`].
    ///
    /// The peer must already have a [`Known`] in hand; bootstrapping from a
    /// peer who is also bootstrapping results in `Ok(None)`.
    ///
    /// Message delivery is unordered: callbacks fire in arbitrary order,
    /// including orderings that violate the causal precedence captured by each
    /// message's [`Version`]. Sort by [`Version`] if your application needs
    /// causal ordering.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::sync::Arc;
    /// use rumors::sync::Known;
    /// use std::net::TcpStream;
    ///
    /// let mut write = TcpStream::connect("127.0.0.1:9000").unwrap();
    /// let mut read = write.try_clone().unwrap();
    /// let bob: Option<Known<String>> = Known::bootstrap_then(
    ///     &mut read,
    ///     &mut write,
    ///     |_, _, m: &Arc<String>| println!("{}", m.len()),
    /// )
    /// .unwrap();
    /// ```
    pub fn bootstrap_then<'a, OnMessage, R, W>(
        read: &'a mut R,
        write: &'a mut W,
        on_message: OnMessage,
    ) -> Result<Option<Self>, Error>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync + 'a,
        R: Read + Unpin + Send,
        W: Write + Unpin + Send,
        OnMessage: FnMut(Key, &Version, &Arc<T>) + Send + 'a,
    {
        // Bridge the synchronous reader/writer to the async I/O the protocol
        // expects, exactly as `gossip_then` does.
        let mut read = AllowStdIo::new(read).compat();
        let mut write = AllowStdIo::new(write).compat_write();
        pollster::block_on(crate::Known::<T>::bootstrap_then(
            &mut read,
            &mut write,
            into_async(on_message),
        ))
        .map(|known| known.map(Known))
    }

    /// Retire this rumor set into a remote peer, handing it our party so our
    /// id-region is reclaimed rather than leaked, then leaving the universe.
    ///
    /// The session begins with a round of gossip: the two peers reconcile
    /// content exactly as [`gossip`](Self::gossip) would, so everything we hold
    /// that the peer had not yet seen survives in it. Once reconciliation
    /// completes the peer *causally dominates* us by construction, and it
    /// absorbs our party. No prior synchronization is required; retiring into
    /// an already-converged peer simply skips the content transfer.
    ///
    /// # Returns
    ///
    /// - `Ok(None)`: **retired.** The peer reconciled with us and absorbed our
    ///   party; we have left the universe and dropped ourselves.
    /// - `Ok(Some(self))`: **declined, unchanged.** The peer cannot absorb a
    ///   party — it was itself retiring, or was bootstrapping — so nothing
    ///   happened and we are handed back intact to retry elsewhere.
    /// - `Err(`[`RetireError::Recovered`]`)`: the session failed *before* our
    ///   party ever crossed the wire; the error carries the intact retiree to
    ///   retry elsewhere. Nothing was lost.
    /// - `Err(`[`RetireError::Uncertain`]`)`: the session failed while sending
    ///   the trailing party frame; the peer may hold our party, so the retiree
    ///   is consumed. See the async [`Known::retire`](crate::Known::retire)
    ///   for the commitment model.
    ///
    /// A peer running ordinary [`gossip`](Self::gossip) absorbs a retiree
    /// transparently, so the counterparty needs no special call.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rumors::sync::Known;
    /// use std::net::TcpStream;
    ///
    /// let mut write = TcpStream::connect("127.0.0.1:9000").unwrap();
    /// let mut read = write.try_clone().unwrap();
    /// let alice: Known<String> = Known::seed();
    /// // `None` => we successfully retired; `Some(alice)` => declined, retry.
    /// let _retired: Option<Known<String>> = alice.retire(&mut read, &mut write).unwrap();
    /// ```
    pub fn retire<'a, R, W>(
        self,
        read: &'a mut R,
        write: &'a mut W,
    ) -> Result<Option<Self>, RetireError<Self>>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync + 'a,
        R: Read + Unpin + Send,
        W: Write + Unpin + Send,
    {
        // Bridge the synchronous reader/writer to the async I/O the protocol
        // expects, exactly as `gossip` does.
        let mut read = AllowStdIo::new(read).compat();
        let mut write = AllowStdIo::new(write).compat_write();
        pollster::block_on(self.0.retire(&mut read, &mut write))
            .map(|known| known.map(Known))
            .map_err(|e| match e {
                // Re-wrap the recovered retiree in the synchronous surface.
                RetireError::Recovered { error, known } => RetireError::Recovered {
                    error,
                    known: Known(known),
                },
                RetireError::Uncertain { error } => RetireError::Uncertain { error },
            })
    }
}

/// Methods available regardless of type-state: read-only observers and the wire
/// protocol. Both a canonical [`Known`] and a non-canonical [`rumors`](Known::rumors)
/// snapshot can inspect their content and [`gossip`](Self::gossip); only an
/// originator (`Known<T, Facts>`) can [`message`](Known::message) or
/// [`redact`](Known::redact).
impl<T, S> Known<T, S> {
    /// This rumor set's [`Network`]: the identifier shared by every peer
    /// descended from the same [`seed`](Known::seed). Combining operations
    /// require matching networks.
    pub fn network(&self) -> Network {
        self.0.network()
    }

    /// Lazily iterate every message currently live in this rumor set, as
    /// `(Key, &Version, &Arc<T>)`, in unspecified order.
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::sync::{Known};
    ///
    /// let mut alice = Known::seed();
    /// alice.message(["a".to_string(), "b".to_string()]);
    /// let mut live: Vec<String> = alice.iter().map(|(_, _, m)| m.as_ref().clone()).collect();
    /// live.sort();
    /// assert_eq!(live, vec!["a".to_string(), "b".to_string()]);
    /// ```
    pub fn iter(
        &self,
    ) -> impl ExactSizeIterator<Item = (Key, &Version, &Arc<T>)> + DoubleEndedIterator + Send + Sync
    where
        T: Send + Sync,
    {
        self.0.iter()
    }

    /// The number of live messages in this rumor set.
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::sync::Known;
    ///
    /// let mut alice = Known::seed();
    /// assert_eq!(alice.len(), 0);
    /// alice.message(["a".to_string(), "b".to_string()]);
    /// assert_eq!(alice.len(), 2);
    /// ```
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether this rumor set holds no live messages.
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::sync::Known;
    ///
    /// let mut alice = Known::seed();
    /// assert!(alice.is_empty());
    /// alice.message(["news".to_string()]);
    /// assert!(!alice.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// This set's latest causal [`Version`]: the least upper bound of every
    /// message and redaction it has observed.
    ///
    /// This is the timestamp a peer ships in its greeting when it
    /// [`gossip`](Self::gossip)s, and the one the protocol compares to decide
    /// what each side is missing. Two sets with equal versions have already
    /// converged.
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::sync::Known;
    ///
    /// let mut alice = Known::seed();
    /// let before = alice.latest().clone();
    /// alice.message(["news".to_string()]);
    /// assert!(alice.latest() != &before); // observing a message advanced it
    /// ```
    pub fn latest(&self) -> &Version {
        self.0.latest()
    }

    /// The earliest message [`Version`] currently live in this set, or `None`
    /// if it is empty.
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::sync::Known;
    ///
    /// let mut alice = Known::seed();
    /// assert!(alice.earliest().is_none());
    /// alice.message(["only".to_string()]);
    /// assert!(alice.earliest().is_some());
    /// ```
    pub fn earliest(&self) -> Option<&Version> {
        self.0.earliest()
    }

    /// The observable root hash of this set: a 32-byte digest of its live
    /// contents that ignores party identity and insertion order.
    ///
    /// Two sets with the same root hash hold the same live messages. Gossip
    /// converges on causal versions rather than hashes: peers with equal
    /// hashes but different versions (for example, after an insert that was
    /// then redacted) still run a reconciliation pass.
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::sync::Known;
    ///
    /// let mut alice = Known::seed();
    /// let empty = alice.hash();
    /// alice.message(["rumor".to_string()]);
    /// assert_ne!(alice.hash(), empty); // new content, new digest
    /// ```
    pub fn hash(&self) -> [u8; 32] {
        self.0.hash()
    }

    /// Force this set's lazy structural memos (observable hash and
    /// ceiling/floor version bounds), so a subsequent operation is timed
    /// against its own work. For benchmark and test calibration only.
    #[doc(hidden)]
    pub fn warm_caches(&self) {
        self.0.warm_caches();
    }

    /// Synchronize rumor sets with a remote peer without observing the messages
    /// learned from it.
    ///
    /// Available on both a canonical [`Known`] and a [`rumors`](Known::rumors)
    /// snapshot: the documented idiom is to gossip with a snapshot and then
    /// [`join`](Known::join) it back into its originator.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rumors::sync::Known;
    /// use std::net::TcpStream;
    ///
    /// let mut write = TcpStream::connect("127.0.0.1:9000").unwrap();
    /// let mut read = write.try_clone().unwrap();
    /// let alice: Known<String> = Known::seed();
    /// let _alice = alice.gossip(&mut read, &mut write).unwrap();
    /// ```
    pub fn gossip<'a, R, W>(self, read: &'a mut R, write: &'a mut W) -> Result<Self, Error>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync + 'a,
        R: Read + Unpin + Send,
        W: Write + Unpin + Send,
    {
        // Bridge the synchronous reader/writer to the async I/O the protocol
        // expects: `AllowStdIo` adapts `Read`/`Write` to `futures::io`'s async
        // traits, and the tokio-compat layer adapts those to `tokio::io`'s.
        let mut read = AllowStdIo::new(read).compat();
        let mut write = AllowStdIo::new(write).compat_write();
        pollster::block_on(self.0.gossip(&mut read, &mut write)).map(Known)
    }

    /// Synchronize rumor sets with a remote peer, invoking `on_message` for
    /// each message learned from the peer.
    ///
    /// Message delivery is unordered: callbacks fire in arbitrary order,
    /// including orderings that violate the causal precedence captured by each
    /// message's [`Version`]. Sort by [`Version`] if your application needs
    /// causal ordering.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rumors::sync::Known;
    /// use std::net::TcpStream;
    ///
    /// let mut write = TcpStream::connect("127.0.0.1:9000").unwrap();
    /// let mut read = write.try_clone().unwrap();
    /// let alice: Known<String> = Known::seed();
    /// let _alice = alice
    ///     .gossip_then(&mut read, &mut write, |_, _, m| println!("{}", m.len()))
    ///     .unwrap();
    /// ```
    pub fn gossip_then<'a, OnMessage, R, W>(
        self,
        read: &'a mut R,
        write: &'a mut W,
        on_message: OnMessage,
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
        pollster::block_on(
            self.0
                .gossip_then(&mut read, &mut write, into_async(on_message)),
        )
        .map(Known)
    }
}
