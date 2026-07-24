//! Joining an existing universe: the [`Bootstrap`] builder behind
//! [`Peer::bootstrap`], its bookmarked state [`BookmarkedBootstrap`], and
//! the latter's [`Joined`] outcome.

use std::marker::PhantomData;

use borsh::{BorshDeserialize, BorshSerialize};
use tokio::io::{AsyncRead, AsyncWrite};

use crate::bookmark::{Bookmark, BookmarkError};
use crate::link::{Acceptor, Connector, Link};
use crate::tree::mirror::streaming::remote::RunBudget;
use crate::tree::mirror::streaming::window::WindowConfig;
use crate::{Error, Peer, Protocol};

use super::gossip::Unbookmarked;

/// Configuration for joining an existing universe: the builder behind
/// [`Peer::bootstrap`].
///
/// [`join`](Self::join) runs one session against an established peer over
/// a [`Link`], receives that provider's whole live set, and mints the
/// [`Peer`] holding the identity the provider donates. The link chooses
/// the provider — a [`Link`] is a conduit to exactly one counterparty —
/// and joining lands you in whichever [`Network`](crate::Network) the
/// provider belongs to.
///
/// Every setting here is the minted peer's own, selected one session
/// early: [`protocol`](Self::protocol),
/// [`sync_memory_budget`](Self::sync_memory_budget), and
/// [`target_message_size`](Self::target_message_size) each state what they
/// change about the bootstrap session itself, and the minted peer keeps
/// the choice exactly as if selected through the matching [`Peer`] method.
/// [`bookmark`](Self::bookmark) additionally persists the received
/// identity before `join` returns, moving the builder to its
/// [`BookmarkedBootstrap`] state (whose `join` reports outcomes as a
/// [`Joined`], since a persist can fail while the peer lives).
///
/// The builder is `Copy`: after a mutual-bootstrap bail
/// ([`join`](Self::join)'s `Ok(None)`) or a failed session, the same
/// configuration retries against another provider as-is.
///
/// # The provider's side
///
/// Serving a bootstrap takes no provider-side call of its own: it happens
/// automatically inside an ordinary [`gossip`](crate::Rumors::gossip),
/// which forks the provider's identity and donates the fork. The provider
/// neither schedules nor manages the donation — party donation is
/// automatic, and its atomicity is handled internally — and concurrent
/// serves over separate links are legal, like any concurrent gossip.
/// Donation commits on the donor's side before the fork crosses the wire,
/// so a failed serve can leave identity held by no one, never by both
/// sides.
#[must_use = "a `Bootstrap` does nothing until `join` runs it against a link"]
pub struct Bootstrap<T> {
    pub(crate) protocol: Protocol,
    pub(crate) window: WindowConfig,
    pub(crate) run_budget: RunBudget,
    /// Covariant, `Send`/`Sync`-neutral marker for the payload type the
    /// minted [`Peer`] will carry.
    marker: PhantomData<fn() -> T>,
}

// Manual, unbounded impls: the payload type is phantom (the builder holds
// configuration only), so `T: Clone`/`T: Copy` bounds — which `derive`
// would mint — have nothing to constrain.
impl<T> Clone for Bootstrap<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Bootstrap<T> {}

/// The configuration only; the payload type parameter carries no state.
impl<T> std::fmt::Debug for Bootstrap<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Bootstrap")
            .field("protocol", &self.protocol)
            .field("window", &self.window)
            .field("run_budget", &self.run_budget)
            .finish()
    }
}

impl<T> Bootstrap<T> {
    /// The all-defaults configuration behind [`Peer::bootstrap`], the one
    /// constructor.
    pub(crate) fn new() -> Self {
        Self {
            protocol: Protocol::default(),
            window: WindowConfig::default(),
            run_budget: RunBudget::default(),
            marker: PhantomData,
        }
    }

    /// Select the reconciliation protocol for the bootstrap session and
    /// every later session of the minted peer.
    ///
    /// Both endpoints of a connection must select the same protocol, so
    /// use this when joining through a provider which selected a
    /// non-default dialect such as `Protocol::V1` (behind the
    /// `protocol-v1` cargo feature). The default is [`Protocol::V2`]. The
    /// minted peer retains the choice exactly as
    /// [`Peer::protocol`] would select it.
    pub fn protocol(mut self, protocol: Protocol) -> Self {
        self.protocol = protocol;
        self
    }

    /// Bound the memory the minted peer's synchronizations may spend on
    /// pipelining.
    ///
    /// The bootstrap session itself never disputes — a joining replica
    /// holds nothing yet, so there is nothing to reconcile subtree by
    /// subtree — and this setting cannot change it (the transfer's memory
    /// is [`target_message_size`](Self::target_message_size)'s concern).
    /// Selecting it here means the minted peer's very first
    /// synchronization already runs budgeted.
    ///
    /// The default, what the budget prices, and how to choose one are
    /// [`Peer::sync_memory_budget`]'s, which the minted peer behaves
    /// exactly as if it had called. `Protocol::V1` sessions ignore it:
    /// the alternating protocol batches whole levels instead of
    /// pipelining.
    pub fn sync_memory_budget(mut self, budget_bytes: usize) -> Self {
        self.window = WindowConfig::Budget(budget_bytes);
        self
    }

    /// Bound the encoded size of the batched messages the bootstrap
    /// session — and every later session — sends.
    ///
    /// This is the one setting with immediate effect on the bootstrap
    /// session, the session that transfers the provider's entire set as
    /// supply *runs* (batched leaf-record messages). The greeting carries
    /// each side's target and each side's encoder batches within the
    /// **minimum** of the two, so a memory-constrained newcomer's setting
    /// is what the provider's encoder builds the whole transfer within.
    /// Any value is safe, including zero (one leaf per message).
    ///
    /// The default and the full contract — flush accounting, the memory
    /// unit on each side, and the framing ceiling — are
    /// [`Peer::target_message_size`]'s, which the minted peer behaves
    /// exactly as if it had called. `Protocol::V1` sessions ignore it:
    /// the alternating protocol's wire format is frozen.
    pub fn target_message_size(mut self, bytes: usize) -> Self {
        self.run_budget = RunBudget::from_bytes(bytes);
        self
    }

    /// Persist the received identity as part of joining: the minted peer
    /// comes back already [`bookmark`](Peer::bookmark)ed.
    ///
    /// [`Peer::bookmark`]'s contract asks for the attach *immediately*
    /// after an unbookmarked arrival, because a crash before the identity
    /// is recorded strands it. Selecting the bookmark here makes
    /// "immediately" structural: [`join`](BookmarkedBootstrap::join)
    /// returns only after the attach and its eager persist have run — no
    /// caller code can interleave — and its [`Joined`] outcome makes a
    /// persist failure impossible to mistake for success. A joined peer
    /// always has an identity worth recording (the received fork is never
    /// the undivided seed), so the persist always touches storage.
    ///
    /// One bookmark records one peer, handled linearly; the sharing rules
    /// are [`Bookmark`]'s. Like the session settings, this may be selected
    /// in any order with the others.
    ///
    /// # What the bookmark does not protect
    ///
    /// Arrival and persistence remain two steps: a process crash after
    /// the provider commits its donation but before the store commits
    /// still loses the identity. What this removes is the *unbounded*
    /// application-side window after a bare `join` returns. A failed
    /// session is beyond its reach — an identity lost in flight was never
    /// the bookmark's to record — and a bookmark records identity, never
    /// content: messages are recovered by
    /// [`gossip`](crate::Rumors::gossip)ing, like any peer's.
    pub fn bookmark<B: Bookmark>(self, bookmark: B) -> BookmarkedBootstrap<T, B> {
        BookmarkedBootstrap {
            config: self,
            bookmark,
        }
    }

    /// Join the provider's universe: run the bootstrap session over
    /// `link`, minting a brand-new [`Peer`] from the counterparty's
    /// donation.
    ///
    /// `Ok(None)` means the counterparty was itself still bootstrapping,
    /// so neither side had anything to share and no identity moved. It is
    /// a clean session boundary: the link remains usable. Connect to
    /// another peer and try again (the builder is `Copy`, so the same
    /// configuration retries as-is).
    ///
    /// On `Ok(Some(peer))` the provider has confirmed committing its side
    /// of the donation. The confirmation exchange leaves one irreducible
    /// residue — the confirmation itself can be lost, a gap
    /// [`Error::Epilogue`] explains cannot be closed: if the session
    /// fails at the very end with that error, the provider may have
    /// committed while our side reports an error, and the forked identity
    /// is lost. Losing a fork is safe — no invariant depends on it
    /// arriving — but not free: it is identity space gone for good,
    /// unless coordination outside this library reclaims it. What `Err`
    /// and cancellation leave behind is stated in [what a session
    /// promises](crate::link::Link#what-a-session-promises).
    ///
    /// The peer arrives unbookmarked: its identity has been forked away
    /// to us but not yet persisted, so a crash before it is recorded
    /// strands it. To make the received identity durable, attach a
    /// [`Bookmark`] with [`bookmark`](Peer::bookmark)
    /// immediately — or select it before joining with
    /// [`bookmark`](Self::bookmark), which makes the attach structural.
    pub async fn join<CR, CW, C, A>(
        self,
        link: &mut Link<CR, CW, C, A>,
    ) -> Result<Option<Peer<T>>, Error>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync + 'static,
        CR: AsyncRead + Unpin + Send,
        CW: AsyncWrite + Unpin + Send,
        C: Connector,
        A: Acceptor,
    {
        Peer::bootstrap_inner(self, link).await
    }
}

/// A [`Bootstrap`] that will persist the received identity before handing
/// it back: the state [`Bootstrap::bookmark`] selects.
///
/// [`join`](Self::join) runs the same session the plain builder's
/// [`join`](Bootstrap::join) does, then attaches the bookmark and eagerly
/// persists — the exact [`Peer::bookmark`] step, with no room for caller
/// code between the identity's arrival and the persist attempt. A distinct
/// type, rather than a fourth setting, lets each state's `join` state only
/// its own outcomes: without a bookmark, nothing can fail *after* the
/// session, and the plain `Result` says so; with one, the persist can fail
/// while the peer lives, and [`Joined`] carries that arm where it cannot
/// be ignored.
///
/// The session settings remain selectable in this state, order-free; the
/// bookmark itself does not — one bookmark records one peer, so there is
/// nothing coherent for a second selection to mean.
#[must_use = "a `BookmarkedBootstrap` does nothing until `join` runs it against a link"]
pub struct BookmarkedBootstrap<T, B> {
    /// The session settings, exactly as the plain builder holds them.
    config: Bootstrap<T>,
    /// The storage the minted peer's identity will be recorded in.
    bookmark: B,
}

/// The session settings; the bookmark is shown by its type only, since
/// [`Bookmark`] does not require `Debug`.
impl<T, B> std::fmt::Debug for BookmarkedBootstrap<T, B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BookmarkedBootstrap")
            .field("config", &self.config)
            .field("bookmark", &std::any::type_name::<B>())
            .finish()
    }
}

impl<T, B: Bookmark> BookmarkedBootstrap<T, B> {
    /// Select the reconciliation protocol; the contract is
    /// [`Bootstrap::protocol`]'s.
    pub fn protocol(mut self, protocol: Protocol) -> Self {
        self.config = self.config.protocol(protocol);
        self
    }

    /// Bound pipelining memory; the contract is
    /// [`Bootstrap::sync_memory_budget`]'s.
    pub fn sync_memory_budget(mut self, budget_bytes: usize) -> Self {
        self.config = self.config.sync_memory_budget(budget_bytes);
        self
    }

    /// Bound batched message size; the contract is
    /// [`Bootstrap::target_message_size`]'s.
    pub fn target_message_size(mut self, bytes: usize) -> Self {
        self.config = self.config.target_message_size(bytes);
        self
    }

    /// Join the provider's universe and durably record the received
    /// identity, reporting what survived as a [`Joined`].
    ///
    /// The session itself is exactly [`Bootstrap::join`]'s — see it for
    /// the session contract (whom the link chooses, what a failure at the
    /// very end can cost, what `Err` and cancellation leave behind).
    /// This method adds one step after a successful session: the minted
    /// peer takes the bookmark through [`Peer::bookmark`], persisting the
    /// received identity before anything is handed back. Each of the four
    /// ways that can end is a [`Joined`] variant; the bookmark comes back
    /// in every outcome that never used it.
    pub async fn join<CR, CW, C, A>(self, link: &mut Link<CR, CW, C, A>) -> Joined<T, B>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync + 'static,
        CR: AsyncRead + Unpin + Send,
        CW: AsyncWrite + Unpin + Send,
        C: Connector,
        A: Acceptor,
    {
        let Self { config, bookmark } = self;
        match config.join(link).await {
            Ok(Some(peer)) => match peer.bookmark(bookmark).await {
                Ok(peer) => Joined::Joined { peer },
                Err(unbookmarked) => Joined::Unbookmarked(unbookmarked),
            },
            Ok(None) => Joined::Bailed { bookmark },
            Err(error) => Joined::Failed { error, bookmark },
        }
    }
}

/// The outcome of a bookmarked bootstrap: what [`BookmarkedBootstrap::join`]
/// left behind.
///
/// Marked `must_use` because every variant carries something whose silent
/// drop loses state the call existed to preserve: the minted peer
/// ([`Joined`](Self::Joined)), a live peer whose identity is *not yet
/// durable* ([`Unbookmarked`](Self::Unbookmarked)), or the bookmark to
/// retry with ([`Bailed`](Self::Bailed), [`Failed`](Self::Failed)).
#[must_use = "every `Joined` variant carries a peer or the bookmark; dropping it loses one or the other"]
#[derive(Debug)]
pub enum Joined<T, B: BookmarkError> {
    /// **Joined and durable.** The session committed, the received
    /// identity is attached and persisted, and the link rests at a clean
    /// session boundary.
    Joined {
        /// The minted, bookmarked peer.
        peer: Peer<T, B>,
    },
    /// **Bailed, nothing moved.** The counterparty was itself still
    /// bootstrapping, so neither side had a universe to share.
    ///
    /// The bookmark never touched storage and comes back for the retry
    /// against a more established peer. The link remains usable.
    Bailed {
        /// The unused bookmark, for the retry.
        bookmark: B,
    },
    /// **Alive but not durable.** The session committed, but recording
    /// the received identity failed: a crash now strands it.
    ///
    /// The peer inside holds the received identity and the provider's
    /// whole set — the failure cost a persist attempt and nothing else.
    /// This is exactly [`Peer::bookmark`]'s failure: take the peer back
    /// out and retry the attach against healthy storage, or proceed
    /// knowingly unbookmarked.
    Unbookmarked(Unbookmarked<T, B>),
    /// **Failed.** The session failed before any peer was minted; the
    /// bookmark never touched storage and comes back for the retry.
    ///
    /// The link is poisoned — an identity that was in flight when the
    /// session failed is lost, leaking its identity space benignly,
    /// exactly as [`Bootstrap::join`]'s `Err` describes.
    Failed {
        /// What failed the session.
        error: Error,
        /// The unused bookmark, for the retry.
        bookmark: B,
    },
}

#[cfg(test)]
mod tests;
