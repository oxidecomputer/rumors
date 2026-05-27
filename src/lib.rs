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
//! // keep party identifiers globally unique.
//! let mut alice: Local<String> = Local::for_party("alice");
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
//! let mut alice: Local<String> = Local::for_party("alice");
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
//! [`Local`] is cheap to clone: the underlying tree is structurally shared and
//! copy-on-write. Clones can mutate independently on separate threads/tasks and
//! recombine via [`Local::process`], which also backs the [`Add`] /
//! [`AddAssign`] operators.
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
//! let mut alice: Local<String> = Local::for_party("alice");
//! alice.message(["hello".to_string()], ignore).await;
//! let bob: Local<String> = Local::for_party("bob");
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

use std::{
    ops::{Add, AddAssign},
    sync::Arc,
};

use borsh::{BorshDeserialize, BorshSerialize};

use message::Message;
use tokio::io::{AsyncRead, AsyncWrite};
use tree::{Action, Tree, mirror};

mod imbl_borsh;
mod message;
pub mod sync;
mod tree;
mod version;

/// A local set of rumors: add to it, redact from it, gossip with peers.
///
/// Cheap to clone (structurally shared, copy-on-write); concurrent code
/// typically holds one clone per thread or task and recombines them via
/// [`Local::process`]. Methods take `AsyncFnMut` callbacks; for synchronous
/// callbacks, see [`sync::Local`].
///
/// # Example
///
/// ```
/// use rumors::{Local, Key};
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// let mut alice: Local<String> = Local::for_party("alice");
/// let mut keys: Vec<Key> = Vec::new();
/// alice.message(
///     ["hello".to_string(), "world".to_string()],
///     async |key, _, _| keys.push(key),
/// ).await;
/// alice.redact([keys[0]]);
/// # }
/// ```
#[derive(Debug, Eq)]
pub struct Local<T>(pub(crate) Tree<T>);

impl<T> Clone for Local<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> PartialEq for Local<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
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
/// let mut alice: Local<String> = Local::for_party("alice");
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
/// let mut alice: Local<String> = Local::for_party("alice");
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
/// let mut alice: Local<Rumor> = Local::for_party("alice");
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
/// use rumors::{Local, ignore};
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// let mut alice: Local<String> = Local::for_party("alice");
/// alice.message(["hello".to_string(), "world".to_string()], ignore).await;
/// # }
/// ```
pub async fn ignore<T>(_key: Key, _version: &Version, _message: &Arc<T>) {}

impl<T> Local<T> {
    /// Create an empty rumor set tagged with the given party identifier.
    ///
    /// Party identifiers must be *globally unique* across the gossip
    /// network; reusing one across peers causes missed messages and other
    /// undefined behavior.
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::Local;
    ///
    /// let _alice: Local<String> = Local::for_party("alice");
    /// ```
    pub fn for_party(party: impl AsRef<[u8]>) -> Self {
        Local(Tree::for_party(party))
    }

    /// Insert messages into the rumor set, invoking `on_message` once per
    /// newly-observed message.
    ///
    /// The callback receives:
    /// - an opaque [`Key`], usable later with [`redact`](Self::redact);
    /// - the causal [`Version`] at which the message was observed;
    /// - an [`Arc<T>`](Arc) holding the message itself.
    ///
    /// Callback order is unspecified and need not match the insertion order.
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::Local;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let mut alice: Local<String> = Local::for_party("alice");
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
    pub async fn message<OnMessage, I>(&mut self, messages: I, mut on_message: OnMessage)
    where
        T: BorshSerialize,
        I: IntoIterator<Item = T>,
        OnMessage: AsyncFnMut(Key, &Version, &Arc<T>),
    {
        self.0
            .act(
                messages.into_iter().map(Message::from).map(Action::Insert),
                async |k, v, m| {
                    if let Some(m) = m {
                        on_message(k, v, AsRef::<Arc<T>>::as_ref(m)).await;
                    }
                },
            )
            .await;
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
    /// let mut alice: Local<String> = Local::for_party("alice");
    /// let mut keys: Vec<Key> = Vec::new();
    /// alice.message(
    ///     ["transient announcement".to_string()],
    ///     async |k, _, _| keys.push(k),
    /// ).await;
    /// alice.redact(keys);
    /// # }
    /// ```
    pub fn redact<I: IntoIterator<Item = Key>>(&mut self, redacted: I) {
        pollster::block_on(
            self.0
                .act(redacted.into_iter().map(Action::Forget), async |_, _, _| {}),
        );
    }

    /// Merge `new` into `self`, invoking `on_message` for each message in
    /// `new` that `self` had not already observed.
    ///
    /// The canonical use is recombining clones that gossiped independently
    /// against different peers. The callback signature matches
    /// [`Local::message`]; messages present in `self` but missing from `new`
    /// do not fire it.
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::{Local, ignore};
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let mut alice: Local<String> = Local::for_party("alice");
    /// let mut helper = alice.clone();
    ///
    /// // Suppose `helper` learned a new rumor on another task.
    /// helper.message(["new rumor".to_string()], ignore).await;
    ///
    /// let mut learned = Vec::new();
    /// alice.process(helper, async |_, _, m| {
    ///     learned.push(m.as_ref().clone())
    /// }).await;
    /// assert_eq!(learned, vec!["new rumor".to_string()]);
    /// # }
    /// ```
    pub async fn process<OnMessage>(&mut self, new: Local<T>, on_message: OnMessage)
    where
        OnMessage: AsyncFnMut(Key, &Version, &Arc<T>),
    {
        // Instantiate the two sides of the mirror exchange, both local
        let l = mirror::local::Exchange::start(self.0.root.clone(), ignore, on_message);
        let r = mirror::local::Exchange::start(new.0.root, ignore, ignore);

        // Drive them to completion: we know they don't need a "real" executor
        Ok((self.0.root, _)) = mirror(l, r).await;
    }

    /// Synchronize rumor sets with a remote peer, invoking `on_message` for
    /// each message learned from the peer.
    ///
    /// `read` and `write` must implement [`AsyncRead`] / [`AsyncWrite`];
    /// both ends of the connection must drive `gossip` concurrently. The
    /// callback signature matches [`Local::message`]. For synchronous I/O,
    /// use [`sync::Local::gossip`].
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
    /// let mut alice: Local<String> = Local::for_party("alice");
    /// alice.message(["hello".to_string()], ignore).await;
    /// let bob: Local<String> = Local::for_party("bob");
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
    pub async fn gossip<OnMessage, R, W>(
        mut self,
        read: &mut R,
        write: &mut W,
        on_message: OnMessage,
    ) -> Result<Self, Error>
    where
        T: BorshDeserialize + BorshSerialize,
        R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin,
        OnMessage: AsyncFnMut(Key, &Version, &Arc<T>),
    {
        // Instantiate the two sides of the mirror exchange: local and remote
        let l = mirror::local::Exchange::start(self.0.root, ignore, on_message);
        let r = mirror::remote::Exchange::start(read, write);

        // Drive them to completion against each other
        (self.0.root, _) = mirror(l, r).await.map_err(|e| {
            // The only possible error is a server error
            let mirror::Error::Server(e) = e;
            e
        })?;

        Ok(self)
    }
}

/// Combine two rumor sets via [`Local::process`].
impl<T> Add for Local<T> {
    type Output = Local<T>;

    fn add(mut self, rhs: Self) -> Self::Output {
        pollster::block_on(self.process(rhs, async |_, _, _| {}));
        self
    }
}

/// Absorb `rhs` into `self` via [`Local::process`].
impl<T> AddAssign for Local<T> {
    fn add_assign(&mut self, rhs: Self) {
        *self = self.clone().add(rhs);
    }
}
