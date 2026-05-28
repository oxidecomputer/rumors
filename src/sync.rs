//! Synchronous rumor sets with redaction.
//!
//! `rumors::sync` is a CRDT-backed gossip set with `FnMut` callbacks and
//! [`std::io::Read`] / [`std::io::Write`] I/O. Use this module when your I/O
//! is synchronous (e.g. [`std::net::TcpStream`]).
//!
//! # Quickstart
//!
//! ```
//! use std::sync::{Arc, Mutex};
//! use rumors::sync::Local;
//!
//! // A peer is identified by an arbitrary byte string; the caller must keep
//! // party identifiers globally unique. `start` is the local event counter
//! // to resume from (0 for a fresh party); see [`Local::for_party`].
//! let mut alice = Local::for_party("alice", 0).unwrap();
//!
//! // The callback fires once per newly-observed message with an opaque
//! // `Key` (used later for redaction), the causal `Version`, and the value.
//! // Sync callbacks satisfy `Send + 'static` (so the caller can drive
//! // gossip on a `std::thread::spawn`'d thread); locally-owned counters
//! // therefore travel through `Arc<Mutex<_>>` rather than `&mut`.
//! let observed = Arc::new(Mutex::new(0usize));
//! let observed_in = Arc::clone(&observed);
//! alice.message(
//!     ["hello".to_string(), "world".to_string()],
//!     move |_key, _version, _message| *observed_in.lock().unwrap() += 1,
//! );
//! assert_eq!(*observed.lock().unwrap(), 2);
//! ```
//!
//! # Redaction
//!
//! Any peer can [`redact`](Local::redact) a [`Key`] it holds; the redaction
//! propagates to every connected peer without consensus, so a single peer's
//! local decision evicts the message network-wide.
//!
//! ```
//! use std::sync::{Arc, Mutex};
//! use rumors::sync::{Local, Key};
//!
//! let mut alice = Local::for_party("alice", 0).unwrap();
//! let keys: Arc<Mutex<Vec<Key>>> = Arc::new(Mutex::new(Vec::new()));
//! let keys_in = Arc::clone(&keys);
//! alice.message(
//!     ["stale rumor".to_string()],
//!     move |k, _, _| keys_in.lock().unwrap().push(k),
//! );
//! alice.redact(Arc::try_unwrap(keys).unwrap().into_inner().unwrap());
//! ```
//!
//! # Concurrent rumor sets
//!
//! A [`Local`] is either an [`Original`] (returned by [`Local::for_party`],
//! one per party per process) or a [`Forked`] copy made with [`Local::fork`].
//! Only the [`Original`] may originate new [`message`](Local::message)s or
//! [`redact`](Local::redact)ions; [`Forked`] clones are cheap (the underlying
//! tree is structurally shared and copy-on-write) and exist to be mutated
//! concurrently against peers, then folded back in via [`Local::process`]
//! (or the [`Add`] / [`AddAssign`] operators). This split enforces at the
//! type level that every party acts as a single sequential process.
//!
//! # Gossiping with peers on the network
//!
//! Pass a [`Read`] reader and a [`Write`] writer into [`Local::gossip`]; both
//! ends must drive `gossip` concurrently (typically on separate threads):
//!
//! ```no_run
//! use rumors::sync::{Local, ignore};
//! use std::net::TcpStream;
//!
//! let mut write = TcpStream::connect("127.0.0.1:9000").unwrap();
//! let mut read = write.try_clone().unwrap();
//! let alice: Local<String, _> = Local::for_party("alice", 0).unwrap();
//! let _alice = alice.gossip(&mut read, &mut write, ignore).unwrap();
//! ```
//!
//! # Message serialization
//!
//! Messages are serialized with [`borsh`], which is re-exported so callers
//! can derive [`BorshSerialize`] / [`BorshDeserialize`] on their message
//! types without taking a separate dependency.

use std::io::{Read, Write};
use std::ops::{Add, AddAssign};
use std::sync::Arc;

use borsh::{BorshDeserialize, BorshSerialize};
use futures::io::AllowStdIo;
use tokio_util::compat::{FuturesAsyncReadCompatExt, FuturesAsyncWriteCompatExt};

pub use crate::{
    AlreadyExists, Error, Forked, Key, Original, PROTOCOL_MAGIC, PROTOCOL_VERSION, Version,
};
pub use ::borsh;

/// A local set of rumors: add to it, redact from it, gossip with peers.
///
/// [`Local<T, Forked>`](Local) is cheap to clone (structurally shared,
/// copy-on-write); concurrent code holds one fork per thread and
/// recombines them via [`Local::process`]. [`Local<T, Original>`](Local)
/// deliberately does *not* implement [`Clone`]: only the singleton
/// original may originate new messages and redactions, and duplicating one
/// must be explicit. Use [`Local::fork`] to obtain a [`Forked`] view that
/// can be cloned and moved across threads.
///
/// # Uniqueness of parties
///
/// It is *required* that each party's [`Local::message`] and [`Local::redact`]
/// actions are causally sequential. This is enforced locally within a given
/// process: [`Local::for_party`] returns a type-tagged `Local<T, Original>`
/// (or [`Err(AlreadyExists)`](AlreadyExists) if there is an extant original
/// [`Local`] for this party in the current process). Subsequently,
/// [`Local::fork`] can duplicate an [`Original`] [`Local`] into a [`Forked`]
/// [`Local`], which can still participate in [`gossip`](Local::gossip) and
/// can still [`process`](Local::process) other [`Forked`] [`Local`]s into
/// itself, but crucially which *cannot* originate new messages and
/// redactions: these may only be performed on the original singleton
/// `Local<T, Original>`.
///
/// While these checks enforce consistency within a single process, it is the
/// responsibility of the programmer to ensure that parties act as sequential
/// processes across the network. In particular, if an [`Original`] [`Local`]
/// is ever dropped and then recreated for the same party (e.g. across process
/// restarts), the `start` parameter passed to [`Local::for_party`] must be
/// greater than or equal to the last observable [`event`](Local::event) of
/// the prior instantiation. Persist `event()` durably between runs and feed
/// it back in as `start` to uphold this invariant.
///
/// # Example
///
/// ```
/// use std::sync::{Arc, Mutex};
/// use rumors::sync::{Local, Key};
///
/// let mut alice = Local::for_party("alice", 0).unwrap();
/// let keys: Arc<Mutex<Vec<Key>>> = Arc::new(Mutex::new(Vec::new()));
/// let keys_in = Arc::clone(&keys);
/// alice.message(
///     ["hello".to_string(), "world".to_string()],
///     move |key, _, _| keys_in.lock().unwrap().push(key),
/// );
/// alice.redact([keys.lock().unwrap()[0]]);
/// ```
#[derive(Debug, Eq)]
pub struct Local<T, Identity = Forked>(pub crate::Local<T, Identity>);

/// Only forked `Local`s can be cloned using [`Clone`]; to clone an original
/// `Local` into a non-original one, use [`Local::fork`].
impl<T> Clone for Local<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T, Identity, Other> PartialEq<Local<T, Other>> for Local<T, Identity> {
    fn eq(&self, other: &Local<T, Other>) -> bool {
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
/// use rumors::sync::{Local, ignore};
///
/// let mut alice = Local::for_party("alice", 0).unwrap();
/// alice.message(["hello".to_string(), "world".to_string()], ignore);
/// ```
pub fn ignore<T>(_key: Key, _version: &Version, _message: &Arc<T>) {}

impl<T> Local<T, Original> {
    /// Create an empty rumor set tagged with the given party identifier.
    ///
    /// Party identifiers must be *globally unique* across the gossip network;
    /// reusing one across peers causes missed messages and other undefined
    /// behavior. If a party identifier is ever reused, its `start` must be
    /// greater than or equal to the last observable [`event`](Self::event) of
    /// the prior instantiation.
    ///
    /// Returns [`Err(AlreadyExists)`](AlreadyExists) if an [`Original`]
    /// already exists for this party in the current process.
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::sync::Local;
    ///
    /// let _alice: Local<String, _> = Local::for_party("alice", 0).unwrap();
    /// ```
    pub fn for_party(party: impl AsRef<[u8]>, start: u64) -> Result<Self, AlreadyExists> {
        crate::Local::for_party(party, start).map(Self)
    }

    /// Get this party's local event counter: the count of all operations ever
    /// applied by this party.
    ///
    /// Persist this value durably between process runs and pass it back as
    /// the `start` argument to [`Local::for_party`] on the next invocation.
    /// If a party name is reused, `start >= self.event()` of the prior
    /// instantiation is *required*; violating this invariant can lead to
    /// arbitrary and contagious corruption of the rumor set network-wide.
    pub fn event(&self) -> u64 {
        self.0.event()
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
    /// use std::sync::{Arc, Mutex};
    /// use rumors::sync::{Local, Key, Version};
    ///
    /// let mut alice = Local::for_party("alice", 0).unwrap();
    /// let observed: Arc<Mutex<Vec<(Key, Version, String)>>> =
    ///     Arc::new(Mutex::new(Vec::new()));
    /// let observed_in = Arc::clone(&observed);
    /// alice.message(
    ///     ["hello".to_string(), "world".to_string()],
    ///     move |key, version, message| {
    ///         observed_in
    ///             .lock()
    ///             .unwrap()
    ///             .push((key, version.clone(), message.as_ref().clone()));
    ///     },
    /// );
    /// assert_eq!(observed.lock().unwrap().len(), 2);
    /// ```
    pub fn message<OnMessage, I>(&mut self, messages: I, mut on_message: OnMessage)
    where
        T: BorshSerialize + Send + Sync + 'static,
        I: IntoIterator<Item = T> + Send,
        I::IntoIter: Send,
        OnMessage: FnMut(Key, &Version, &Arc<T>) + Send + 'static,
    {
        // Wrap the sync `FnMut` into the `FnMut(...) -> Send + 'static future`
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
    /// [`Local::message`], [`Local::process`], or [`Local::gossip`].
    /// Redaction is contagious, so a single peer's call evicts the message
    /// network-wide without consensus.
    ///
    /// # Example
    ///
    /// ```
    /// use std::sync::{Arc, Mutex};
    /// use rumors::sync::{Local, Key};
    ///
    /// let mut alice = Local::for_party("alice", 0).unwrap();
    /// let keys: Arc<Mutex<Vec<Key>>> = Arc::new(Mutex::new(Vec::new()));
    /// let keys_in = Arc::clone(&keys);
    /// alice.message(
    ///     ["transient".to_string()],
    ///     move |k, _, _| keys_in.lock().unwrap().push(k),
    /// );
    /// alice.redact(Arc::try_unwrap(keys).unwrap().into_inner().unwrap());
    /// ```
    pub fn redact<I: IntoIterator<Item = Key>>(&mut self, redacted: I)
    where
        T: Send + Sync + 'static,
    {
        self.0.redact(redacted);
    }
}

impl<T, Identity> Local<T, Identity> {
    /// Duplicate this rumor set into a [`Forked`] [`Local`] usable
    /// concurrently.
    ///
    /// Forks share their underlying tree structurally (copy-on-write), so
    /// this is cheap. A fork may [`gossip`](Self::gossip) with peers and
    /// absorb other forks via [`process`](Self::process) or `+`, but it
    /// *cannot* originate new [`message`](Self::message)s or
    /// [`redact`](Self::redact) keys; only the singleton [`Original`] for
    /// the party can.
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::sync::{Local, ignore};
    ///
    /// let mut alice = Local::for_party("alice", 0).unwrap();
    /// alice.message(["hello".to_string()], ignore);
    ///
    /// // A fork can be moved to another thread; only the Original can mutate.
    /// let snapshot = alice.fork();
    /// assert_eq!(alice, snapshot);
    /// ```
    pub fn fork(&self) -> Local<T, Forked> {
        Local(self.0.fork())
    }

    /// Merge `new` into `self`, invoking `on_message` for each message in
    /// `new` that `self` had not already observed.
    ///
    /// `new` must be [`Forked`]: only the [`Original`] for a party can
    /// originate messages, but any number of [`Forked`] copies can carry
    /// observations between peers and recombine. The callback signature
    /// matches [`Local::message`]; messages present in `self` but missing
    /// from `new` do not fire it.
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
    /// use std::sync::{Arc, Mutex};
    /// use rumors::sync::{Local, ignore};
    ///
    /// let mut alice = Local::for_party("alice", 0).unwrap();
    /// let mut bob = Local::for_party("bob", 0).unwrap();
    /// bob.message(["news from bob".to_string()], ignore);
    ///
    /// let learned: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    /// let learned_in = Arc::clone(&learned);
    /// alice.process(
    ///     bob.fork(),
    ///     move |_, _, m| learned_in.lock().unwrap().push(m.as_ref().clone()),
    /// );
    /// assert_eq!(*learned.lock().unwrap(), vec!["news from bob".to_string()]);
    /// ```
    pub fn process<OnMessage>(&mut self, new: Local<T, Forked>, mut on_message: OnMessage)
    where
        T: Send + Sync + 'static,
        Identity: Send,
        OnMessage: FnMut(Key, &Version, &Arc<T>) + Send + 'static,
    {
        pollster::block_on(self.0.process(new.0, move |k, v, m| {
            on_message(k, v, m);
            std::future::ready(())
        }));
    }

    /// Synchronize rumor sets with a remote peer, invoking `on_message` for
    /// each message learned from the peer.
    ///
    /// `read` and `write` must implement [`Read`] / [`Write`]; both ends of
    /// the connection must drive `gossip` concurrently (typically on
    /// separate threads). The callback signature matches [`Local::message`].
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
    /// use rumors::sync::{Local, ignore};
    /// use std::net::TcpStream;
    ///
    /// let mut write = TcpStream::connect("127.0.0.1:9000").unwrap();
    /// let mut read = write.try_clone().unwrap();
    /// let alice: Local<String, _> = Local::for_party("alice", 0).unwrap();
    /// let _alice = alice.gossip(&mut read, &mut write, ignore).unwrap();
    /// ```
    pub fn gossip<OnMessage, R, W>(
        self,
        read: &mut R,
        write: &mut W,
        mut on_message: OnMessage,
    ) -> Result<Self, Error>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync + 'static,
        R: Read + Unpin + Send,
        W: Write + Unpin + Send,
        OnMessage: FnMut(Key, &Version, &Arc<T>) + Send + 'static,
        Identity: Send,
    {
        // Bridge the synchronous reader/writer to the async I/O the protocol
        // expects: `AllowStdIo` adapts `Read`/`Write` to `futures::io`'s async
        // traits, and the tokio-compat layer adapts those to `tokio::io`'s.
        let mut read = AllowStdIo::new(read).compat();
        let mut write = AllowStdIo::new(write).compat_write();
        // Wrap the sync `FnMut` into the `FnMut(...) -> Send + 'static future`
        // shape the async gossip wants. The body runs synchronously; the
        // returned `Ready` future resolves immediately.
        pollster::block_on(self.0.gossip(&mut read, &mut write, move |k, v, m| {
            on_message(k, v, m);
            std::future::ready(())
        }))
        .map(Local)
    }
}

/// Combine two rumor sets via [`Local::process`].
impl<T> Add for Local<T, Forked>
where
    T: Send + Sync + 'static,
{
    type Output = Local<T, Forked>;

    fn add(mut self, rhs: Self) -> Self::Output {
        self.process(rhs, ignore);
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
