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
//! // additional originating peers are made by [`Known::bootstrap`], never by a
//! // second `seed`.
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
//! A `Known` is [`!Clone`](Clone). For a cheap, copy-on-write working snapshot
//! that shares the originator's party, take a [`rumors`](Known::rumors)
//! snapshot; gossip with it, then [`join`](Known::join) it back into its
//! originator to absorb what it learned. A second *originating* peer, with
//! its own disjoint party, comes from [`Known::bootstrap`] over the wire.
//! A snapshot cannot originate [`message`](Known::message)s or
//! [`redact`](Known::redact)ions; only the originating `Known` can, which
//! keeps its events linearized. Any [`rumors`](Known::rumors) snapshot may
//! be reunited with its originator via [`Known::join_then`] (observing
//! everything new from it) or [`Known::join`].
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
    fmt::Debug,
    future::{Future, Ready, ready},
    marker::PhantomData,
    pin::Pin,
    sync::{Arc, RwLock},
};

use before::Party;
use borsh::{BorshDeserialize, BorshSerialize};
use rand::{RngCore, rngs::OsRng};
use tokio::io::{AsyncRead, AsyncWrite};

pub mod sync;

// Not yet wired to the public surface; see the module docs.
#[allow(dead_code)]
mod bookmark;
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
/// Bumped whenever the wire format changes. A peer whose version differs is
/// rejected with [`Error::VersionMismatch`]; a future release may introduce
/// compatibility across a range of versions, but none exists today.
pub const PROTOCOL_VERSION: u16 = 1;

/// A local set of rumors: add to it, redact from it, gossip with peers.
///
/// A canonical `Known` owns an Interval Tree Clock party and may originate
/// messages and redactions. It does not implement [`Clone`]: duplicating a
/// live party would break the linearity the clocks require. For another
/// working copy, take a [`rumors`](Known::rumors) snapshot, a cheap
/// copy-on-write view that shares the originator's party and cannot
/// originate. Concurrent code gossips with a snapshot, then folds it back
/// via [`join_then`](Known::join_then) or [`join`](Known::join). A second
/// originating peer, with its own disjoint party, comes from
/// [`bootstrap`](Known::bootstrap) over the wire.
///
/// Methods take `AsyncFnMut` callbacks; for synchronous I/O and callbacks,
/// see [`sync::Known`].
///
/// # Uniqueness of parties
///
/// Every party in one universe descends from a single [`seed`](Known::seed).
/// A [`rumors`](Known::rumors) snapshot shares its originator's party, so
/// the two remain one linear history; [`join_then`](Known::join_then) and
/// [`join`](Known::join) merge a snapshot's content back (network-guarded)
/// and never touch parties. A second peer with its own disjoint party comes
/// from [`bootstrap`](Known::bootstrap), and [`retire`](Known::retire) hands
/// a party region back. Because every region stays disjoint, each party's
/// history is causally well-defined no matter how peers interleave.
///
/// The one rule the caller must uphold is not to mix universes: two `Known`s
/// from independent [`seed`](Known::seed) calls share no causal history and
/// must never gossip with each other.
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
pub struct Known<T, S = Facts> {
    /// The universe this rumor set belongs to: a 128-bit id minted at
    /// [`seed`](Known::seed) and inherited by every [`rumors`](Known::rumors)
    /// snapshot and [`bootstrap`](Known::bootstrap)ed peer. Every
    /// combining operation checks it matches before merging, ruling out
    /// coincidentally-disjoint parties from unrelated seeds. See [`Network`].
    network: Network,
    /// This rumor set's [`Party`]: an Interval Tree Clock identity descended
    /// from a common [`seed`](Known::seed).
    ///
    /// We *share* a party between every instance of this [`Known`], only
    /// mutating the shared party in the event that we transmit a portion of it
    /// to help someone else bootstrap.
    party: Arc<RwLock<Party>>,
    /// The inner tree holding everything we know.
    tree: Tree<T>,
    /// The type-state of this [`Known`], indicating whether it has the ability
    /// to originate new state changes.
    canonical: PhantomData<fn() -> S>,
}

impl<T: Debug, S> Debug for Known<T, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Known")
            .field("network", &self.network)
            .field("party", &self.party.read().unwrap())
            .field("tree", &self.tree)
            .finish()
    }
}

/// Marker type indicating that a [`Known`] is a non-canonical copy which cannot
/// create new [`message`](Known::message)s or [`redact`](Known::redact)
/// existing ones.
#[repr(transparent)]
pub struct Rumors;

/// Marker type indicating that a [`Known`] is the *canonical* set of knowledge;
/// it can create [`message`](Known::message)s or [`redact`](Known::redact)
/// existing ones.
#[repr(transparent)]
pub struct Facts;

/// Two rumor sets are equal when they belong to the same [`Network`] and
/// hold the same observations (the same tree), regardless of which
/// [`Party`] observed them. The party is excluded because parties are
/// linear: no two live `Known`s ever share one, so including it would make
/// equality nearly always false. A [`rumors`](Known::rumors) snapshot
/// therefore compares equal to its originator until one of them originates
/// anew.
impl<T> Eq for Known<T> {}
impl<T, S, U> PartialEq<Known<T, U>> for Known<T, S> {
    fn eq(&self, other: &Known<T, U>) -> bool {
        self.network == other.network && self.tree == other.tree
    }
}

/// The error type returned by [`Known::gossip`].
///
/// Surfaces I/O failures from the underlying reader/writer as well as
/// framing errors encountered while parsing messages off the wire.
pub use mirror::remote::Error;

/// The error type returned by [`Known::retire`] (and [`sync::Known::retire`],
/// with `K` the synchronous wrapper).
///
/// A retiring peer's party crosses the wire as a single trailing frame, sent
/// only after reconciliation completes, so the id-region is at risk for
/// exactly that one frame. This error distinguishes failures by which side of
/// that frame they struck.
#[derive(Debug, thiserror::Error)]
pub enum RetireError<K> {
    /// The session failed strictly before the party frame was sent: the peer
    /// cannot hold our party, so the retiree is handed back intact (party
    /// live, content as of the start of the session) to retry elsewhere.
    /// Nothing was lost.
    #[error("retire failed before the party hand-off: {error}")]
    Recovered {
        /// The underlying session failure.
        #[source]
        error: Error,
        /// The intact retiree.
        known: K,
    },
    /// The session failed while sending the party frame: the peer may hold
    /// our party, and no acknowledgement could tell us whether it does (the
    /// two-generals problem), so the party must be treated as handed off and
    /// the retiree is consumed. Keeping a live copy when the peer might have
    /// absorbed the frame would duplicate the region, breaking the linearity
    /// the clocks require; the worst case is therefore a leaked region,
    /// never a duplicated one.
    #[error("retire failed during the party hand-off: {error}")]
    Uncertain {
        /// The underlying session failure.
        #[source]
        error: Error,
    },
}

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
/// [`Known::join_then`], and [`Known::gossip`]. [`PartialOrd`] captures
/// causal ordering: `a <= b` iff `a`'s history is contained in `b`'s.
/// Versions produced by concurrent events are incomparable (`partial_cmp`
/// returns `None`).
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

use crate::tree::mirror::{
    local::{self, Silent},
    remote,
};

impl<T> Known<T> {
    /// Create the distinguished seed rumor set: the single root [`Party`] from
    /// which every other party in this universe descends.
    ///
    /// Call this exactly once per universe of cooperating peers. Additional
    /// originating peers are minted by [`bootstrap`](Self::bootstrap)ping
    /// from an existing `Known`, never by calling `seed` again: two
    /// independently-seeded universes share no causal history and must never
    /// gossip (the `before` crate's Law of Disjointness). Parties are not
    /// tracked process-globally, so several independent universes may
    /// coexist in one program; keeping them apart is the caller's
    /// responsibility.
    ///
    /// The network identifier is drawn from the operating system's secure RNG
    /// ([`OsRng`]); use [`seed_rng`](Self::seed_rng) to supply
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
    /// from a caller-supplied RNG instead of [`OsRng`].
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
            party: Arc::new(RwLock::new(Party::seed())),
            tree: Tree::new(),
            canonical: PhantomData,
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
    /// Message delivery is unordered: callbacks fire in arbitrary order,
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
        remote::preamble(read, write).await?;

        // We hold nothing: run the ordinary mirror protocol from an *empty*
        // tree, declaring ourselves bootstrapping with the placeholder network
        // and no retire-intent. The empty side pulls all of the provider's
        // content through the usual descent, firing `on_message` per received
        // leaf.
        let l = local::Exchange::start(
            tree::Root::default(),
            Network::ZERO,
            mirror::message::Intent::Remain,
            None::<Silent<T>>,
            on_message,
        );
        let r = remote::Exchange::start(read, write);

        // After the connect phase, a peer that is *also* bootstrapping, or one
        // that is *retiring* (it will not serve), means there is nothing to
        // receive: bail symmetrically.
        let handshaken = mirror::handshake(l, r).await.map_err(server_error)?;
        if handshaken.peer().network.is_bootstrap() || handshaken.peer().intent.retiring() {
            return Ok(None);
        }

        // Otherwise reconcile — pulling the provider's whole tree through the
        // descent — then read the provider's fork-last party frame off the same
        // reader, and adopt its network alongside.
        let (root, (mut reader, _writer), peer) =
            handshaken.reconcile().await.map_err(server_error)?;
        let party = remote::recv_party(&mut reader).await?;
        Ok(Some(Known {
            network: peer.network,
            party: Arc::new(RwLock::new(party)),
            tree: Tree { root },
            canonical: PhantomData,
        }))
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
                |batch| {
                    batch.tick(&self.party.read().unwrap());
                },
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
            |batch| {
                batch.tick(&self.party.read().unwrap());
            },
            redacted.into_iter().map(Action::Forget),
            |_, _, _| ready(()),
        ));
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
    /// use rumors::{Known};
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let mut alice = Known::seed();
    /// alice.message(["hello".to_string()]).await;
    ///
    /// // The snapshot shares alice's observations and her party.
    /// let snapshot = alice.rumors();
    /// assert_eq!(alice, snapshot);
    /// # }
    /// ```
    pub fn rumors(&self) -> Known<T, Rumors> {
        Known {
            network: self.network,
            party: self.party.clone(),
            tree: self.tree.clone(),
            canonical: PhantomData,
        }
    }

    /// Merge `other`'s content into `self`, discarding per-message observations:
    /// the in-process twin of [`gossip`](Self::gossip) for absorbing a
    /// [`rumors`](Self::rumors) snapshot back into its originator.
    ///
    /// A blocking convenience: the callback-free counterpart of
    /// [`join_then`](Self::join_then). Because there is no callback, the merge
    /// elides the per-leaf discovery walk, the dominant cost of reconciliation.
    ///
    /// # Errors
    ///
    /// Returns `Err(other)`, handing `other` back untouched, on a [`Network`]
    /// mismatch — see [`join_then`](Self::join_then).
    ///
    /// # Example
    ///
    /// ```
    /// use rumors::Known;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let mut alice = Known::seed();
    /// alice.message(["hello".to_string()]).await;
    ///
    /// // A snapshot shares alice's observations; joining it back is a content
    /// // union (in real use the snapshot would gossip and learn first).
    /// let snapshot = alice.rumors();
    /// alice.join(snapshot).unwrap();
    /// # }
    /// ```
    pub fn join(&mut self, other: Known<T, Rumors>) -> Result<(), Known<T, Rumors>>
    where
        T: Send + Sync,
    {
        // A `None` callback lets the merge elide the per-leaf discovery walk.
        pollster::block_on(self.join_inner(other, None::<fn(Key, &Version, &Arc<T>) -> Ready<()>>))
    }

    /// Merge `other`'s content into `self`, invoking `on_message` for each
    /// message in `other` that `self` had not already observed.
    ///
    /// The observing counterpart of [`join`](Self::join): use that when you do
    /// not need the [`Key`]s / [`Version`]s of the merged-in messages.
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
    /// use rumors::Known;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let (a, b) = tokio::io::duplex(1024);
    /// let (mut a_r, mut a_w) = tokio::io::split(a);
    /// let (mut b_r, mut b_w) = tokio::io::split(b);
    ///
    /// // `bob` bootstraps from alice: a genuine peer in the same universe with
    /// // its own disjoint party.
    /// let mut alice = Known::seed();
    /// let (alice, bob) = tokio::join!(
    ///     alice.gossip(&mut a_r, &mut a_w),
    ///     Known::<String>::bootstrap(&mut b_r, &mut b_w),
    /// );
    /// let mut alice = alice.unwrap();
    /// let mut bob = bob.unwrap().expect("served");
    /// bob.message(["news from bob".to_string()]).await;
    ///
    /// // Merge bob's content back into alice, observing each new message.
    /// let mut learned: Vec<String> = Vec::new();
    /// alice.join_then(bob.rumors(), |_, _, m| {
    ///     learned.push(m.as_ref().clone());
    ///     async {}
    /// }).await.unwrap();
    /// assert_eq!(learned, vec!["news from bob".to_string()]);
    /// # }
    /// ```
    pub async fn join_then<'a, OnMessage, OnMessageFut>(
        &'a mut self,
        other: Known<T, Rumors>,
        on_message: OnMessage,
    ) -> Result<(), Known<T, Rumors>>
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
        other: Known<T, Rumors>,
        on_message: Option<OnMessage>,
    ) -> Result<(), Known<T, Rumors>>
    where
        T: Send + Sync + 'a,
        OnMessage: FnMut(Key, &Version, &Arc<T>) -> OnMessageFut + Send + 'a,
        OnMessageFut: std::future::Future<Output = ()> + Send + 'a,
    {
        // `join` only replicates *content*: it merges `other`'s tree into ours
        // and never touches either party (handing a party region across is
        // [`retire`](Self::retire)'s job, not a content merge's). So the sole
        // precondition is a shared universe. A `Network` mismatch means the two
        // descend from unrelated [`seed`](Self::seed)s and share no causal
        // history; we hand `other` back whole with nothing mutated.
        if self.network != other.network {
            return Err(other);
        }

        let Known {
            tree: other_tree, ..
        } = other;

        // Merge the two trees directly, by a simultaneous recursion over both.
        // This is observationally identical to mirroring two local trees.
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

    /// Retire this rumor set into a remote peer, handing it our [`Party`] so our
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
    ///   party ever crossed the wire, so nothing is lost: the error carries the
    ///   intact retiree (content as of the start of the session) to retry
    ///   elsewhere.
    /// - `Err(`[`RetireError::Uncertain`]`)`: the session failed while sending
    ///   the party frame itself; the peer may hold our party, so the retiree is
    ///   consumed.
    ///
    /// # Commitment
    ///
    /// Our party crosses the wire as a single trailing frame, sent only
    /// after our reconciliation completes: the same fork-last structure as
    /// serving a [`bootstrap`](Self::bootstrap), in the opposite direction.
    /// A failure anywhere before that frame cannot have given the peer our
    /// party, and hands us back via [`RetireError::Recovered`]. Once the
    /// frame is in flight we are committed: no acknowledgement could
    /// distinguish "the peer got it" from "it was lost" (the two-generals
    /// problem), so we must assume delivery and drop ourselves. A failure
    /// there ([`RetireError::Uncertain`]) can at worst leak the one frame's
    /// region, never duplicate it. A peer running ordinary
    /// [`gossip`](Self::gossip) absorbs a retiree transparently, so the
    /// counterparty needs no special call.
    pub async fn retire<'a, R, W>(
        mut self,
        read: &'a mut R,
        write: &'a mut W,
    ) -> Result<Option<Self>, RetireError<Self>>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync + 'a,
        R: AsyncRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
    {
        // Boxed: the reconciliation descent makes this future large, exactly
        // as in `gossip` and `bootstrap`.
        Box::pin(async move {
            if let Err(error) = remote::preamble(read, write).await {
                return Err(RetireError::Recovered { error, known: self });
            }

            // The session takes our tree root by value; keep a copy-on-write
            // backup (structurally shared, not deep-copied) so a failure
            // before the party hand-off can return the retiree intact.
            let backup = self.tree.root.clone();
            let l = local::Exchange::start(
                self.tree.root,
                self.network,
                mirror::message::Intent::Retire,
                None::<Silent<T>>,
                None::<Silent<T>>,
            );
            let r = remote::Exchange::start(read, write);

            // The connect phase rejects a real-network mismatch; past it, the
            // peer's greeting decides whether it can absorb us.
            let handshaken = match mirror::handshake(l, r).await {
                Ok(handshaken) => handshaken,
                Err(e) => {
                    self.tree.root = backup;
                    return Err(RetireError::Recovered {
                        error: server_error(e),
                        known: self,
                    });
                }
            };

            // Decline against a peer that cannot absorb a party: one that is
            // itself retiring (it is leaving too) or bootstrapping (it has no
            // party to join ours into). Both sides skip the descent; collapse
            // our (un-descended) tree back and keep our live party.
            if handshaken.peer().intent.retiring() || handshaken.peer().network.is_bootstrap() {
                let (root, _peer) = handshaken.stop();
                self.tree.root = root;
                return Ok(Some(self));
            }

            // Otherwise reconcile to convergence — the round of gossip (a
            // no-op if the versions already match). The descent hands the peer
            // everything it lacked, after which it causally dominates us and
            // is owed the party our greeting promised. Whatever we learned
            // from the peer in return is dropped along with our tree.
            let (_root, (_reader, mut writer), _peer) = match Box::pin(handshaken.reconcile()).await
            {
                Ok(reconciled) => reconciled,
                Err(e) => {
                    self.tree.root = backup;
                    return Err(RetireError::Recovered {
                        error: server_error(e),
                        known: self,
                    });
                }
            };

            // The hand-off: ship our party as one trailing frame on the same
            // writer the descent used. We alias rather than move it out (a
            // `rumors` snapshot may share the `Arc`), then drop our copy by
            // dropping ourselves: exactly one side treats the party as live.
            // From the moment this send begins we are committed — the peer may
            // have received it even if the send errors.
            let alias = self.party.read().unwrap().dangerously_alias();
            if let Err(error) = remote::send_party(alias, &mut writer).await {
                return Err(RetireError::Uncertain { error });
            }
            Ok(None)
        })
        .await
    }
}

impl<T, S> Known<T, S> {
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
    /// Two sets with the same root hash hold the same live messages. Gossip
    /// converges on causal versions rather than hashes: peers with equal
    /// hashes but different versions (for example, after an insert that was
    /// then redacted) still run a reconciliation pass.
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
    /// // `bob` shares alice's universe via `rumors` (a second `seed` would be a
    /// // different network, which gossip rejects).
    /// let bob = alice.rumors();
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
    /// Message delivery is unordered: callbacks fire in arbitrary order,
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
    /// // `bob` shares alice's universe via `rumors` before the insert, so it
    /// // shares the network and has "hello" to learn over the wire.
    /// let bob = alice.rumors();
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
        remote::preamble(read, write).await?;

        // Run the connect phase, which exchanges `message::Handshake`s (network
        // + version + retire-intent). We are gossiping, so we are not retiring.
        let l = local::Exchange::start(
            self.tree.root,
            self.network,
            mirror::message::Intent::Remain,
            None::<Silent<T>>,
            on_message,
        );
        let r = remote::Exchange::start(read, write);

        // Run the initial handshake to determine if and how to gossip.
        let handshaken = mirror::handshake(l, r).await.map_err(server_error)?;
        let bootstrapper = handshaken.peer().network.is_bootstrap();

        // Reconcile — descending if our versions differ. For an ordinary peer
        // this is the whole session; for a retiring peer it is the round of
        // gossip that brings us to causal dominance before we absorb it; for a
        // bootstrapper it hands over all our content.
        let (root, (mut reader, mut writer), peer) =
            handshaken.reconcile().await.map_err(server_error)?;
        self.tree.root = root;

        // The descent moved content but not parties; settle the party hand-off
        // the greeting promised, in whichever direction.
        if peer.intent.retiring() {
            // The peer is retiring: the reconciliation just made us a causal
            // superset of it, so it now ships its party as one trailing frame
            // on the same wire the descent used, and drops its own copy.
            // Absorb the region.
            let party = remote::recv_party(&mut reader).await?;
            self.party
                .write()
                .unwrap()
                .join(party)
                .map_err(|_| Error::PartyOverlap)?;
        } else if bootstrapper {
            // Serve a bootstrapper the forked party it still needs.
            let give = self.party.write().unwrap().fork();
            remote::send_party(give, &mut writer).await?;
        }
        Ok(self)
    }
}

/// Collapse a mirror error down to the wire-bound server error. The client side
/// of every wire session is the in-memory local exchange, whose error type is
/// [`Infallible`](std::convert::Infallible), so the
/// [`Client`](mirror::Error::Client) arm is uninhabitable.
fn server_error(e: mirror::Error<std::convert::Infallible, Error>) -> Error {
    match e {
        mirror::Error::Server(e) => e,
        mirror::Error::Client(never) => match never {},
    }
}
