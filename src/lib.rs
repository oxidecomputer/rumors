//! Unordered gossip with redaction.
//!
//! `rumors` is a protocol for efficient unordered gossip for sets of causally
//! versioned messages with redaction. Each peer holds a [`Known<T>`] rumor set;
//! peers reconcile by exchanging only the parts that differ. Redacting a
//! message stops it propagating, and redactions spread contagiously to every
//! peer the redactor (transitively) gossips with.
//!
//! This crate supports an async interface (this module) and a synchronous
//! interface, in the [`rumors::sync`](sync) module.
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
//! A `Known` is [`!Clone`](Clone); the only way to duplicate one is
//! [`fork`](Known::fork), which creates a cheap, copy-on-write *causal fork*,
//! or its networked counterpart, [`Known::bootstrap`]. The fork may originate
//! [`message`](Known::message)s and [`redact`](Known::redact)ions independently
//! of its original. Any two causal forks ultimately descended from the same
//! [`Known::seed`] may be reunited via [`Known::join_then`] (observing
//! everything new from one side) or [`Known::join`].
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
//! Both [`bootstrap`](Known::bootstrap) and [`gossip`](Known::gossip) pass an
//! [`AsyncRead`] reader and [`AsyncWrite`] writer (here, the two ends of an
//! in-memory pipe standing in for a TCP connection):
//!
//! ```
//! use rumors::Known;
//!
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() {
//! let (a, b) = tokio::io::duplex(1024);
//! let (mut a_r, mut a_w) = tokio::io::split(a);
//! let (mut b_r, mut b_w) = tokio::io::split(b);
//!
//! // `alice` is an established peer with some content.
//! let mut alice: Known<String> = Known::seed();
//! alice.message(["hello".to_string()]).await;
//!
//! // `bob` is a fresh process. It bootstraps from alice, who serves a copy of
//! // her tree and a freshly-forked party; both ends drive the session at once.
//! let (alice, bob) = tokio::join!(
//!     alice.gossip(&mut a_r, &mut a_w),
//!     Known::<String>::bootstrap(&mut b_r, &mut b_w),
//! );
//! let mut alice = alice.unwrap();
//! let mut bob = bob.unwrap().expect("alice served the bootstrap");
//!
//! // bob now belongs to alice's network and holds her observations.
//! assert_eq!(alice, bob);
//!
//! // If bob and alice add messages, they are no longer equal:
//! alice.message(["bob".to_string()]).await;
//! bob.message(["alice".to_string()]).await;
//! assert!(alice != bob);
//!
//! // But they can gossip to synchronize again:
//! let (alice, bob) = tokio::join!(
//!     alice.gossip(&mut a_r, &mut a_w),
//!     bob.gossip(&mut b_r, &mut b_w),
//! );
//! assert_eq!(alice.unwrap(), bob.unwrap());
//! # }
//! ```
//!
//! # Message serialization
//!
//! Messages are serialized with [`borsh`], which is re-exported so callers
//! can derive [`BorshSerialize`] / [`BorshDeserialize`] on their message
//! types without taking a separate dependency.

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
use rand::{RngCore, rngs::OsRng};
use tokio::io::{AsyncRead, AsyncWrite};

pub mod sync;

mod message;
mod network;
mod tree;
mod version;

#[cfg(test)]
mod tests;

use message::Message;
use tree::{Action, Tree, mirror};

pub use network::Network;

/// Magic bytes that prefix every `rumors` gossip session: `b"RUMORS"`.
///
/// Sent as the first six bytes of the raw preamble that opens every session
/// ([`gossip`](Known::gossip)/[`bootstrap`](Known::bootstrap)/[`retire`](Known::retire)),
/// ahead of the framed greeting. A peer whose preamble starts with anything else
/// is rejected with [`Error::MagicMismatch`] before any framed traffic is read.
pub const PROTOCOL_MAGIC: [u8; 6] = *b"RUMORS";

/// On-the-wire protocol version, the big-endian `u16` that follows
/// [`PROTOCOL_MAGIC`] in the raw preamble (before the framed greeting).
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
    /// The universe this rumor set belongs to: a 128-bit id minted at
    /// [`seed`](Known::seed) and inherited by every [`fork`](Known::fork). Every
    /// combining operation checks it matches before merging, ruling out
    /// coincidentally-disjoint parties from unrelated seeds. See [`Network`].
    network: Network,
    /// This rumor set's [`Party`]: an Interval Tree Clock identity descended
    /// from a common [`seed`](Known::seed) by disjoint [`fork`](Known::fork)s.
    /// It is `!Clone`, so a `Known` is `!Clone` too: the only way to obtain
    /// another working copy is [`fork`](Known::fork), which mints a fresh
    /// disjoint party. This enforces the ITC linearity law at the type level —
    /// no two live `Known`s ever share a party region.
    party: Party,
    tree: Tree<T>,
}

/// Two rumor sets are equal when they belong to the same [`Network`] and hold
/// the same observations — the same tree — regardless of which [`Party`]
/// observed them. (The party is deliberately excluded: parties are linear, so
/// no two live `Known`s ever share one, and including it would make equality
/// essentially never hold.) A [`fork`](Known::fork) therefore compares equal to
/// its parent until one of them originates anew.
impl<T> PartialEq for Known<T> {
    fn eq(&self, other: &Self) -> bool {
        self.network == other.network && self.tree == other.tree
    }
}

/// The error type returned by [`Known::gossip`].
///
/// Surfaces I/O failures from the underlying reader/writer as well as
/// framing errors encountered while parsing messages off the wire.
pub use mirror::remote::Error;

use mirror::message::Handshake;

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
    /// Collapse a mirror error down to the wire-bound server error. The client
    /// side of every wire session is the in-memory local exchange, whose error
    /// type is [`Infallible`](std::convert::Infallible), so the
    /// [`Client`](mirror::Error::Client) arm is uninhabitable.
    fn unwrap_server(e: mirror::Error<std::convert::Infallible, Error>) -> Error {
        match e {
            mirror::Error::Server(e) => e,
            mirror::Error::Client(never) => match never {},
        }
    }

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
    /// The network identifier is drawn from the operating system's secure RNG
    /// ([`OsRng`](rand::rngs::OsRng)); use [`seed_rng`](Self::seed_rng) to supply
    /// your own source (e.g. a deterministic RNG in tests).
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::Known;
    ///
    /// let _alice: Known<String> = Known::seed();
    /// ```
    pub fn seed() -> Self {
        Self::seed_rng(&mut OsRng)
    }

    /// Like [`seed`](Self::seed), but draws the universe's [`Network`] identifier
    /// from a caller-supplied RNG instead of [`OsRng`](rand::rngs::OsRng).
    ///
    /// Useful when a deterministic network id is needed — for example a test
    /// that pins exact handshake bytes, or a caller that derives the id from its
    /// own entropy source. The party and tree are identical to [`seed`](Self::seed)'s.
    ///
    /// # Example
    ///
    /// ```
    /// use rand::SeedableRng;
    /// use rand::rngs::StdRng;
    /// use rumors::Known;
    ///
    /// let mut rng = StdRng::seed_from_u64(42);
    /// let _alice: Known<String> = Known::seed_rng(&mut rng);
    /// ```
    pub fn seed_rng<R: RngCore + ?Sized>(rng: &mut R) -> Self {
        Known {
            network: Network::from_rng(rng),
            party: Party::seed(),
            tree: Tree::new(),
        }
    }

    /// This rumor set's [`Network`]: the identifier shared by every peer that
    /// descends from the same [`seed`](Self::seed). Two `Known`s combine (locally
    /// via [`join`](Self::join), remotely via [`gossip`](Self::gossip)) only when
    /// their networks match.
    pub fn network(&self) -> Network {
        self.network
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
            network: self.network,
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
            network: other_network,
            party: other_party,
            tree: other_tree,
        } = other;

        // Networks must match before anything merges: apparently-disjoint
        // parties from unrelated seeds would otherwise pass the disjointness
        // check below despite sharing no causal history. A mismatch hands
        // `other` back whole with nothing mutated.
        if self.network != other_network {
            return Err(Known {
                network: other_network,
                party: other_party,
                tree: other_tree,
            });
        }

        // `Party::join` *is* the disjointness check: on success it merges the
        // other party's region into ours; on overlap (different seeds, or an
        // already-rejoined pair) it leaves us untouched and hands the party
        // back. Doing it first means a non-disjoint `other` is returned whole
        // with nothing mutated — no separate `is_disjoint` probe needed.
        if let Err(party) = self.party.join(other_party) {
            return Err(Known {
                network: other_network,
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
    /// // `bob` shares alice's universe via `fork` (a second `seed` would be a
    /// // different network, which gossip rejects).
    /// let bob = alice.fork();
    /// alice.message(["hello".to_string()]).await;
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
        Box::pin(self.gossip_inner(read, write, None::<fn(Key, &Version, &Arc<T>) -> Ready<()>>))
            .await
    }

    /// Synchronize rumor sets with a remote peer, invoking `on_message` for
    /// each message learned from the peer.
    ///
    /// Message delivery is **unordered**: callbacks fire in arbitrary order,
    /// including orderings that violate the causal precedence captured by each
    /// message's [`Version`]. Sort by [`Version`] if your application needs
    /// causal ordering.
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
    /// // Fork `bob` from alice's universe before the insert, so it shares the
    /// // network and has "hello" to learn over the wire.
    /// let bob = alice.fork();
    /// alice.message(["hello".to_string()]).await;
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
        // Raw magic/version preamble: reject a non-rumors or incompatible peer
        // before the codec trusts any peer-supplied frame length.
        mirror::remote::preamble(read, write).await?;

        // Our latest version, captured before the tree moves into the exchange.
        let our_version = self.tree.latest().clone();

        // Run the connect phase, which exchanges `message::Handshake`s (network
        // + version + retire-intent). We are gossiping, so we offer no party.
        let l = mirror::local::Exchange::start(
            self.tree.root,
            self.network,
            None,
            None::<mirror::local::Silent<T>>,
            on_message,
        );
        let r = mirror::remote::Exchange::start(read, write);

        match mirror::connect_phase(l, r)
            .await
            .map_err(Self::unwrap_server)?
        {
            // Versions already equal: the trees are reconciled. Absorb a
            // retiring peer (we necessarily dominate it), serve a bootstrapper,
            // or simply finish.
            mirror::Phase::Converged {
                local_root,
                remote_out: (_reader, mut writer),
                peer,
            } => {
                let Handshake {
                    network: peer_net,
                    version: peer_version,
                    party: peer_party,
                } = peer;
                self.tree.root = local_root;
                if let Some(party) = peer_party {
                    if peer_version <= our_version {
                        self.party.join(party).map_err(|_| Error::PartyOverlap)?;
                    }
                } else if peer_net.is_bootstrap() {
                    mirror::remote::send_party_fork(&mut self.party, &mut writer).await?;
                }
                Ok(self)
            }
            // Versions differ. A retiring peer ends the session with no descent;
            // otherwise descend, then serve a bootstrapper if it was one.
            mirror::Phase::Diverged {
                local,
                remote,
                our_version: _,
                peer,
            } => {
                let Handshake {
                    network: peer_net,
                    version: peer_version,
                    party: peer_party,
                } = peer;
                if let Some(party) = peer_party {
                    let root = local.into_root();
                    if peer_version <= our_version {
                        self.party.join(party).map_err(|_| Error::PartyOverlap)?;
                    }
                    self.tree.root = root;
                    return Ok(self);
                }
                let bootstrapper = peer_net.is_bootstrap();
                let (root, (_reader, mut writer)) =
                    mirror::descend(local, remote, our_version, peer_version)
                        .await
                        .map_err(Self::unwrap_server)?;
                self.tree.root = root;
                if bootstrapper {
                    mirror::remote::send_party_fork(&mut self.party, &mut writer).await?;
                }
                Ok(self)
            }
        }
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
    ///
    /// let (alice, bob) = tokio::join!(
    ///     alice.gossip(&mut a_r, &mut a_w),
    ///     Known::<String>::bootstrap(&mut b_r, &mut b_w),
    /// );
    /// let (_alice, bob) = (alice.unwrap(), bob.unwrap().expect("served"));
    /// let mut live: Vec<String> = bob.iter().map(|(_, _, m)| m.as_ref().clone()).collect();
    /// live.sort();
    /// assert_eq!(live, vec!["hello".to_string()]);
    /// # }
    /// ```
    pub async fn bootstrap<'a, R, W>(
        read: &'a mut R,
        write: &'a mut W,
    ) -> Result<Option<Self>, Error>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync + 'a,
        R: AsyncRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
    {
        Box::pin(Self::bootstrap_inner(
            read,
            write,
            None::<fn(Key, &Version, &Arc<T>) -> Ready<()>>,
        ))
        .await
    }

    /// Bootstrap a brand-new rumor set from a remote peer, invoking
    /// `on_message` once per message in the received [`Known`].
    ///
    /// The peer must already have a [`Known`] in hand; bootstrapping from a
    /// peer who is also bootstrapping results in `Ok(None)`.
    ///
    /// Message delivery is **unordered**: callbacks fire in arbitrary order,
    /// including orderings that violate the causal precedence captured by each
    /// message's [`Version`]. Sort by [`Version`] if your application needs
    /// causal ordering.
    ///
    /// # Returns
    ///
    /// - `Ok(Some(known))`: success.
    /// - `Ok(None)`: the peer was *also* bootstrapping, so neither side had
    ///   anything to give the other.
    /// - `Err(_)`: handshake or transfer failure (see [`Error`]).
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
    ///
    /// let mut learned: Vec<String> = Vec::new();
    /// let (alice, bob) = tokio::join!(
    ///     alice.gossip(&mut a_r, &mut a_w),
    ///     Known::<String>::bootstrap_then(&mut b_r, &mut b_w, |_, _, m: &Arc<String>| {
    ///         learned.push(m.as_ref().clone());
    ///         async {}
    ///     }),
    /// );
    /// let (_alice, _bob) = (alice.unwrap(), bob.unwrap().expect("served"));
    /// assert_eq!(learned, vec!["hello".to_string()]);
    /// # }
    /// ```
    pub async fn bootstrap_then<'a, OnMessage, OnMessageFut, R, W>(
        read: &'a mut R,
        write: &'a mut W,
        on_message: OnMessage,
    ) -> Result<Option<Self>, Error>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync + 'a,
        R: AsyncRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
        OnMessage: FnMut(Key, &Version, &Arc<T>) -> OnMessageFut + Send + 'a,
        OnMessageFut: Future<Output = ()> + Send + 'a,
    {
        Self::bootstrap_inner(read, write, Some(on_message)).await
    }

    /// Shared core of [`bootstrap`](Self::bootstrap) and
    /// [`bootstrap_then`](Self::bootstrap_then).
    ///
    /// Declares us bootstrapping in the handshake. If the peer is bootstrapping
    /// too, both sides bail with `Ok(None)`. Otherwise it reads the peer's tree
    /// and forked party, assembles the [`Known`], and — when `on_message` is
    /// [`Some`] — replays every leaf through the callback off
    /// [`iter`](Self::iter) (the whole tree arrives at once, so there is no
    /// incremental merge to hook).
    async fn bootstrap_inner<'a, OnMessage, OnMessageFut, R, W>(
        read: &'a mut R,
        write: &'a mut W,
        on_message: Option<OnMessage>,
    ) -> Result<Option<Self>, Error>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync + 'a,
        R: AsyncRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
        OnMessage: FnMut(Key, &Version, &Arc<T>) -> OnMessageFut + Send + 'a,
        OnMessageFut: Future<Output = ()> + Send + 'a,
    {
        // Raw magic/version preamble first.
        mirror::remote::preamble(read, write).await?;

        // We hold nothing: run the ordinary mirror protocol from an *empty*
        // tree, declaring ourselves bootstrapping with the placeholder network
        // and no party. The empty side pulls all of the provider's content
        // through the usual descent, firing `on_message` per received leaf.
        let l = mirror::local::Exchange::start(
            tree::Root::default(),
            Network::ZERO,
            None,
            None::<mirror::local::Silent<T>>,
            on_message,
        );
        let r = mirror::remote::Exchange::start(read, write);

        // After the connect phase, a peer that is *also* bootstrapping, or one
        // that is *retiring* (it will not serve), means there is nothing to
        // receive: bail symmetrically. Otherwise reconcile, then read the
        // provider's fork-last party frame off the same reader.
        let (network, root, mut reader) = match mirror::connect_phase(l, r)
            .await
            .map_err(Self::unwrap_server)?
        {
            mirror::Phase::Converged {
                local_root,
                remote_out: (reader, _writer),
                peer,
            } => {
                if peer.network.is_bootstrap() || peer.party.is_some() {
                    return Ok(None);
                }
                (peer.network, local_root, reader)
            }
            mirror::Phase::Diverged {
                local,
                remote,
                our_version,
                peer,
            } => {
                if peer.network.is_bootstrap() || peer.party.is_some() {
                    return Ok(None);
                }
                let network = peer.network;
                let (root, (reader, _writer)) =
                    mirror::descend(local, remote, our_version, peer.version)
                        .await
                        .map_err(Self::unwrap_server)?;
                (network, root, reader)
            }
        };

        // Adopt the provider's network and the forked party it ships last.
        let party = mirror::remote::recv_party(&mut reader).await?;
        Ok(Some(Known {
            network,
            party,
            tree: Tree { root },
        }))
    }

    /// Retire this rumor set into a remote peer, handing it our [`Party`] so our
    /// id-region is reclaimed rather than leaked, then leaving the universe.
    ///
    /// We may only retire into a peer that *causally dominates* us (its
    /// [`Version`] is `>=` ours): such a peer already holds a superset of our
    /// content, so handing over only the party — with no content transfer — is
    /// safe. The intended pattern is therefore to [`gossip`](Self::gossip) with
    /// a peer first (so its version comes to dominate ours), then `retire` into
    /// it; if that fails, pick another peer and try again.
    ///
    /// # Returns
    ///
    /// - `Ok(None)`: **retired.** The peer dominated us and absorbed our party;
    ///   we have left the universe and dropped ourselves.
    /// - `Ok(Some(self))`: **declined, unchanged.** The peer did not dominate
    ///   us, was itself retiring, or was bootstrapping — nothing happened and we
    ///   are handed back intact to retry elsewhere.
    /// - `Err(_)`: an I/O, handshake, or network-mismatch failure (see
    ///   [`Error`]). As with the other wire methods, an error here consumes
    ///   `self`.
    ///
    /// # Commitment
    ///
    /// Once we read a dominating version from the peer we are *committed*: we
    /// must assume the peer received the party we put on the wire, so we drop
    /// ourselves. A peer running ordinary [`gossip`](Self::gossip) absorbs a
    /// retiree transparently, so the counterparty needs no special call.
    pub async fn retire<'a, R, W>(
        mut self,
        read: &'a mut R,
        write: &'a mut W,
    ) -> Result<Option<Self>, Error>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync + 'a,
        R: AsyncRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
    {
        let our_version = self.tree.latest().clone();
        let our_network = self.network;
        // Alias our party onto the wire while keeping the live one: exactly one
        // side ends up treating it as live (the peer on commit, us on decline).
        let alias = self.party.dangerously_alias();

        mirror::remote::preamble(read, write).await?;
        let l = mirror::local::Exchange::start(
            self.tree.root,
            our_network,
            Some(alias),
            None::<mirror::local::Silent<T>>,
            None::<mirror::local::Silent<T>>,
        );
        let r = mirror::remote::Exchange::start(read, write);

        // Retire never descends: the connect-phase handshake decides everything.
        // Collapse our (un-descended) tree back to a root in case we decline.
        let (root, peer) = match mirror::connect_phase(l, r)
            .await
            .map_err(Self::unwrap_server)?
        {
            mirror::Phase::Converged {
                local_root, peer, ..
            } => (local_root, peer),
            mirror::Phase::Diverged { local, peer, .. } => (local.into_root(), peer),
        };

        // The connect phase already rejected a real-network mismatch.
        //
        // Commit only into a real, non-retiring peer that dominates us: it will
        // have absorbed the party we aliased onto the wire. A bootstrapping peer
        // cannot dominate (and would not absorb), and a peer that is itself
        // retiring declines symmetrically.
        let committed =
            peer.party.is_none() && !peer.network.is_bootstrap() && our_version <= peer.version;
        if committed {
            // Drop ourselves and our now-handed-off party: we have retired.
            return Ok(None);
        }

        // Declined: keep our party and restore our tree, unchanged.
        self.tree.root = root;
        Ok(Some(self))
    }
}
