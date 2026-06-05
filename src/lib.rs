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
//! alice.message_then(
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
//! alice.message_then(
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
//! act independently, then reunite via [`Known::join_then`] (observing everything
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
//! use rumors::{Known};
//!
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() {
//! // Stand in for a real network with an in-memory bidirectional pipe.
//! let (a, b) = tokio::io::duplex(1024);
//! let (mut a_r, mut a_w) = tokio::io::split(a);
//! let (mut b_r, mut b_w) = tokio::io::split(b);
//!
//! let mut alice: Known<String> = Known::seed();
//! alice.message(["hello".to_string()]).await;
//! let bob: Known<String> = Known::seed();
//!
//! // Drive both ends concurrently; bob learns "hello".
//! let (alice, bob) = tokio::join!(
//!     alice.gossip(&mut a_r, &mut a_w),
//!     bob.gossip_then(&mut b_r, &mut b_w,
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
//! - **Minor versions** are forward-compatible.
//! - **Major versions** may break the wire incompatibly. The handshake
//!   surfaces such mismatches as [`Error::VersionMismatch`] before any
//!   rumor-set state is touched.
//!
//! Every connection begins with an 8-byte preamble: [`PROTOCOL_MAGIC`]
//! (`b"RUMORS"`) followed by [`PROTOCOL_VERSION`] as a big-endian `u16`.
//! Reading bytes that don't start with `RUMORS` surfaces as
//! [`Error::MagicMismatch`]; the connection is not a `rumors` stream at
//! all.

// Static assertions uses #[allow(unsafe_code)], so we allow it only in tests
#![cfg_attr(not(test), forbid(unsafe_code))]
// Programmer error in recursive async traits can create large futures, so we
// check to make sure it's not an issue
#![deny(clippy::large_futures)]

use std::{
    future::{Future, Ready, ready},
    pin::Pin,
    sync::Arc,
};

use before::Party;
use borsh::{BorshDeserialize, BorshSerialize};
use tokio::io::{AsyncRead, AsyncWrite};

pub mod sync;

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
/// thread or task and recombines them via [`join_then`](Known::join_then) /
/// [`join`](Known::join).
///
/// Methods take `AsyncFnMut` callbacks; for synchronous I/O and callbacks,
/// see [`sync::Known`].
///
/// # Uniqueness of parties
///
/// Every party in one universe must descend from a single [`seed`](Known::seed)
/// by [`fork`](Known::fork). Forking splits a party into two disjoint regions,
/// and [`join_then`](Known::join_then) / [`join`](Known::join) rejoin them; because the
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
/// alice.message_then(
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
/// [`Known::join_then`], and [`Known::gossip`], and are stable across peers:
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
/// alice.message_then(
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
/// [`Known::join_then`], and [`Known::gossip`]. [`PartialOrd`] captures causal
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
/// alice.message_then(
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
/// use rumors::{Known, borsh};
///
/// #[derive(borsh::BorshSerialize, borsh::BorshDeserialize)]
/// struct Rumor { subject: String, count: u32 }
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// let mut alice = Known::seed();
/// alice.message([Rumor { subject: "weather".into(), count: 3 }]).await;
/// # }
/// ```
pub use ::borsh;

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

    /// Get the latest version represented by this [`Known`]: the least upper
    /// bound of every message and redaction it has observed.
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::Known;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let mut alice = Known::seed();
    /// let before = alice.latest().clone();
    /// alice.message(["news".to_string()]).await;
    /// assert!(alice.latest() != &before); // observing a message advanced it
    /// # }
    /// ```
    pub fn latest(&self) -> &Version {
        self.tree.latest()
    }

    /// Get the earliest message version currently present in this [`Known`], or
    /// `None` if it is empty.
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::Known;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let mut alice = Known::seed();
    /// assert!(alice.earliest().is_none());
    /// alice.message(["only".to_string()]).await;
    /// assert!(alice.earliest().is_some());
    /// # }
    /// ```
    pub fn earliest(&self) -> Option<&Version> {
        self.tree.earliest()
    }

    /// Determine if there are any current messages in this [`Known`].
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::Known;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let mut alice = Known::seed();
    /// assert!(alice.is_empty());
    /// alice.message(["news".to_string()]).await;
    /// assert!(!alice.is_empty());
    /// # }
    /// ```
    pub fn is_empty(&self) -> bool {
        self.tree.is_empty()
    }

    /// Get the number of live messages in this [`Known`].
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::Known;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let mut alice = Known::seed();
    /// assert_eq!(alice.len(), 0);
    /// alice.message(["a".to_string(), "b".to_string()]).await;
    /// assert_eq!(alice.len(), 2);
    /// # }
    /// ```
    pub fn len(&self) -> usize {
        self.tree.len()
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
    /// use rumors::Known;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let mut alice = Known::seed();
    /// alice.message(["hello".to_string(), "world".to_string()]).await;
    /// # }
    /// ```
    pub async fn message<'a, I>(&'a mut self, messages: I)
    where
        T: BorshSerialize + Send + Sync + 'a,
        I: IntoIterator<Item = T> + Send,
        I::IntoIter: Send,
    {
        self.message_inner(messages, None::<fn(Key, &Version, &Arc<T>) -> Ready<()>>)
            .await;
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
    /// use rumors::{Known, Key, Version};
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let mut alice = Known::seed();
    /// let mut observed: Vec<(Key, Version, String)> = Vec::new();
    /// alice.message_then(
    ///     ["hello".to_string(), "world".to_string()],
    ///     |key, version, message| {
    ///         observed.push((key, version.clone(), message.as_ref().clone()));
    ///         async {}
    ///     },
    /// ).await;
    /// assert_eq!(observed.len(), 2);
    /// # }
    /// ```
    pub async fn message_then<'a, OnMessage, OnMessageFut, I>(
        &'a mut self,
        messages: I,
        on_message: OnMessage,
    ) where
        T: BorshSerialize + Send + Sync + 'a,
        I: IntoIterator<Item = T> + Send,
        I::IntoIter: Send,
        OnMessage: FnMut(Key, &Version, &Arc<T>) -> OnMessageFut + Send + 'a,
        OnMessageFut: Future<Output = ()> + Send + 'a,
    {
        self.message_inner(messages, Some(on_message)).await;
    }

    /// Shared core of [`message`](Self::message) and
    /// [`message_then`](Self::message_then). A [`None`] `on_message` skips the
    /// per-leaf callback entirely.
    async fn message_inner<'a, OnMessage, OnMessageFut, I>(
        &'a mut self,
        messages: I,
        mut on_message: Option<OnMessage>,
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
                    match (on_message.as_mut(), m) {
                        (Some(on_message), Some(m)) => Box::pin(on_message(k, v, m.as_ref())),
                        _ => Box::pin(ready(())),
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
    /// use rumors::{Known};
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let mut alice = Known::seed();
    /// alice.message(["a".to_string(), "b".to_string()]).await;
    /// let mut live: Vec<String> = alice.iter().map(|(_, _, m)| m.as_ref().clone()).collect();
    /// live.sort();
    /// assert_eq!(live, vec!["a".to_string(), "b".to_string()]);
    /// # }
    /// ```
    pub fn iter(
        &self,
    ) -> impl ExactSizeIterator<Item = (Key, &Version, &Arc<T>)> + DoubleEndedIterator + Send + Sync
    where
        T: Send + Sync,
    {
        self.tree.iter()
    }

    /// The observable root hash of this set: a 32-byte digest of its live
    /// contents that ignores party identity and insertion order.
    ///
    /// Two sets with the same root hash hold the same live messages, so a
    /// gossip session between them converges immediately. It is the first
    /// thing the initiator puts on the wire (see [`gossip`](Self::gossip)).
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::Known;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let mut alice = Known::seed();
    /// let empty = alice.hash();
    /// alice.message(["rumor".to_string()]).await;
    /// assert_ne!(alice.hash(), empty); // new content, new digest
    /// # }
    /// ```
    pub fn hash(&self) -> [u8; 32] {
        self.tree.hash()
    }

    /// Force this set's tree to compute its lazy structural memos (observable
    /// hash and ceiling/floor version bounds), so a subsequent operation is
    /// timed against its own work. For benchmark and test calibration only.
    #[doc(hidden)]
    pub fn warm_caches(&self) {
        self.tree.warm_caches();
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
    /// use rumors::{Known, Key};
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let mut alice = Known::seed();
    /// let mut keys: Vec<Key> = Vec::new();
    /// alice.message_then(
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
    /// fork with [`join_then`](Self::join_then) / [`join`](Self::join), which
    /// merges the trees and rejoins the parties.
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::{Known};
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let mut alice = Known::seed();
    /// alice.message(["hello".to_string()]).await;
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

    /// Reunite `other` into `self`, discarding per-message observations, and
    /// rejoin its [`Party`] back into `self`'s (the inverse of
    /// [`fork`](Self::fork)).
    ///
    /// A blocking convenience: the callback-free counterpart of
    /// [`join_then`](Self::join_then). Because there is no callback, the merge
    /// elides the per-leaf discovery walk, the dominant cost of reconciliation.
    ///
    /// # Errors
    ///
    /// Returns `Err(other)`, handing `other` back untouched, if the two parties
    /// are **not disjoint** — see [`join_then`](Self::join_then).
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::Known;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let mut alice = Known::seed();
    /// let mut bob = alice.fork();
    /// bob.message(["news".to_string()]).await;
    /// alice.join(bob).unwrap();
    /// # }
    /// ```
    pub fn join(&mut self, other: Known<T>) -> Result<(), Known<T>>
    where
        T: Send + Sync,
    {
        // A `None` callback lets the merge elide the per-leaf discovery walk.
        pollster::block_on(self.join_inner(other, None::<fn(Key, &Version, &Arc<T>) -> Ready<()>>))
    }

    /// Merge `other` into `self`, invoking `on_message` for each message in
    /// `other` that `self` had not already observed, and reuniting `other`'s
    /// [`Party`] back into `self`'s.
    ///
    /// The observing counterpart of [`join`](Self::join): use that when you do
    /// not need the [`Key`]s / [`Version`]s of the merged-in messages.
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
    /// use rumors::Known;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let mut alice = Known::seed();
    /// let mut bob = alice.fork();
    /// bob.message(["news from bob".to_string()]).await;
    ///
    /// let mut learned: Vec<String> = Vec::new();
    /// alice.join_then(bob, |_, _, m| {
    ///     learned.push(m.as_ref().clone());
    ///     async {}
    /// }).await.unwrap();
    /// assert_eq!(learned, vec!["news from bob".to_string()]);
    /// # }
    /// ```
    pub async fn join_then<'a, OnMessage, OnMessageFut>(
        &'a mut self,
        other: Known<T>,
        on_message: OnMessage,
    ) -> Result<(), Known<T>>
    where
        T: Send + Sync + 'a,
        OnMessage: FnMut(Key, &Version, &Arc<T>) -> OnMessageFut + Send + 'a,
        OnMessageFut: std::future::Future<Output = ()> + Send + 'a,
    {
        self.join_inner(other, Some(on_message)).await
    }

    /// Shared core of [`join`](Self::join) and [`join_then`](Self::join_then).
    /// A [`None`] `on_message` elides the per-leaf discovery walk.
    async fn join_inner<'a, OnMessage, OnMessageFut>(
        &'a mut self,
        other: Known<T>,
        on_message: Option<OnMessage>,
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

        // Merge the two trees directly, by a simultaneous recursion over both.
        // This is observationally identical to mirroring two local trees (same
        // merged tree, same `on_message` callbacks).
        //
        // This is more efficient than running the mirror protocol in-memory,
        // but observationally equivalent up to message reordering.
        self.tree
            .join(
                other_tree,
                on_message,
                None::<fn(Key, &Version, &Arc<T>) -> Ready<()>>,
            )
            .await;

        Ok(())
    }

    /// Synchronize rumor sets with a remote peer without observing the messages
    /// learned from it.
    ///
    /// The callback-free counterpart of [`gossip_then`](Self::gossip_then);
    /// see that method for the handshake and ordering semantics. For
    /// synchronous I/O, use [`sync::Known::gossip`].
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::Known;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let (a, b) = tokio::io::duplex(1024);
    /// let (mut a_r, mut a_w) = tokio::io::split(a);
    /// let (mut b_r, mut b_w) = tokio::io::split(b);
    ///
    /// let mut alice: Known<String> = Known::seed();
    /// alice.message(["hello".to_string()]).await;
    /// let bob: Known<String> = Known::seed();
    ///
    /// let (alice, bob) = tokio::join!(
    ///     alice.gossip(&mut a_r, &mut a_w),
    ///     bob.gossip(&mut b_r, &mut b_w),
    /// );
    /// let (_alice, _bob) = (alice.unwrap(), bob.unwrap());
    /// # }
    /// ```
    pub async fn gossip<'a, R, W>(self, read: &'a mut R, write: &'a mut W) -> Result<Self, Error>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync + 'a,
        R: AsyncRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
    {
        self.gossip_inner(read, write, None::<fn(Key, &Version, &Arc<T>) -> Ready<()>>)
            .await
    }

    /// Synchronize rumor sets with a remote peer, invoking `on_message` for
    /// each message learned from the peer.
    ///
    /// `read` and `write` must implement [`AsyncRead`] / [`AsyncWrite`];
    /// both ends of the connection must drive gossip concurrently. The
    /// callback signature matches [`Known::message_then`]. To synchronize
    /// without a callback, use [`gossip`](Self::gossip); for synchronous I/O,
    /// use [`sync::Known::gossip_then`].
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
    /// use rumors::Known;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let (a, b) = tokio::io::duplex(1024);
    /// let (mut a_r, mut a_w) = tokio::io::split(a);
    /// let (mut b_r, mut b_w) = tokio::io::split(b);
    ///
    /// let mut alice: Known<String> = Known::seed();
    /// alice.message(["hello".to_string()]).await;
    /// let bob: Known<String> = Known::seed();
    ///
    /// let mut bob_learned: Vec<String> = Vec::new();
    /// let (alice, bob) = tokio::join!(
    ///     alice.gossip(&mut a_r, &mut a_w),
    ///     bob.gossip_then(&mut b_r, &mut b_w, |_, _, m: &Arc<String>| {
    ///         bob_learned.push(m.as_ref().clone());
    ///         async {}
    ///     }),
    /// );
    /// let (_alice, _bob) = (alice.unwrap(), bob.unwrap());
    /// assert_eq!(bob_learned, vec!["hello".to_string()]);
    /// # }
    /// ```
    pub async fn gossip_then<'a, OnMessage, OnMessageFut, R, W>(
        self,
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
        self.gossip_inner(read, write, Some(on_message)).await
    }

    /// Shared core of [`gossip`](Self::gossip) and
    /// [`gossip_then`](Self::gossip_then). A [`None`] `on_message` elides the
    /// per-leaf discovery walk.
    async fn gossip_inner<'a, OnMessage, OnMessageFut, R, W>(
        mut self,
        read: &'a mut R,
        write: &'a mut W,
        on_message: Option<OnMessage>,
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
        let l = mirror::local::Exchange::start(
            self.tree.root,
            None::<mirror::local::Silent<T>>,
            on_message,
        );
        let r = mirror::remote::Exchange::start(read, write);

        // Drive them to completion against each other
        (self.tree.root, _) = mirror(l, r).await.map_err(|e| {
            // The only possible error is a server error
            let mirror::Error::Server(e) = e;
            e
        })?;

        Ok(self)
    }
}
