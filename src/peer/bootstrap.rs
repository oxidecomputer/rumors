//! Joining an existing universe: the [`Bootstrap`] builder behind
//! [`Peer::bootstrap`].

use std::marker::PhantomData;

use borsh::{BorshDeserialize, BorshSerialize};
use tokio::io::{AsyncRead, AsyncWrite};

use crate::link::{Acceptor, Connector, Link};
use crate::tree::mirror::streaming::remote::RunBudget;
use crate::tree::mirror::streaming::window::WindowConfig;
use crate::{Error, Peer, Protocol};

/// Configuration for joining an existing universe: the builder behind
/// [`Peer::bootstrap`].
///
/// A bootstrap session is the one session a replica runs *before* it
/// exists: [`join`](Self::join) reconciles an empty replica against an
/// established provider, receives the provider's whole live set, and mints
/// the [`Peer`] holding the identity the provider donates. The knobs here
/// are the peer's own session knobs, applied one session early: each
/// governs the bootstrap session where it can act (stated per method), and
/// the minted peer retains every choice for all of its later sessions,
/// exactly as if it had been selected through the matching [`Peer`] method
/// ([`protocol`](Peer::protocol),
/// [`sync_memory_budget`](Peer::sync_memory_budget),
/// [`target_message_size`](Peer::target_message_size)).
///
/// Whom you bootstrap toward is not a knob: a [`Link`] is a conduit to
/// exactly one counterparty, so the provider is chosen by the link handed
/// to [`join`](Self::join). Nor is the universe: membership is identity
/// custody (see the [crate docs](crate)), so joining hands you whichever
/// [`Network`](crate::Network) the provider itself belongs to — there is
/// nothing to request. And nothing here configures the provider: its
/// budget and window are its own, and the one setting the session
/// negotiates — the message-size target — can only *lower* what the
/// provider builds, never raise what it offered.
///
/// The builder is `Copy`, so one configuration can serve several attempts:
/// a mutual-bootstrap bail ([`join`](Self::join)'s `Ok(None)`) or a failed
/// session retries against another provider without rebuilding it.
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
    /// The setting cannot change the bootstrap session itself, and no
    /// budget could: pipelining memory is spent on *disputed* subtrees,
    /// disputes require both replicas to hold something under the same
    /// prefix, and a joining replica holds nothing — the provider's
    /// whole-set transfer streams as supply runs outside the dispute
    /// window (its memory is [`target_message_size`](Self::target_message_size)'s
    /// concern). The knob exists here so the constraint is in force from
    /// the minted peer's *first* post-bootstrap session: the earliest
    /// session at which its replica can dispute is the earliest at which
    /// a budget can bind, and this peer never runs one unbudgeted.
    ///
    /// The default and the full contract — what the budget prices, the
    /// closed form for choosing one, and the trade-off table — are
    /// [`Peer::sync_memory_budget`]'s, which the minted peer behaves
    /// exactly as if it had called. `Protocol::V1` sessions ignore it:
    /// the alternating protocol batches whole levels instead of
    /// pipelining.
    pub fn sync_memory_budget(mut self, budget_bytes: usize) -> Self {
        self.window = WindowConfig::Budget(budget_bytes);
        self
    }

    /// Bound the encoded size of the batched messages the bootstrap
    /// session — and every later session — builds and receives.
    ///
    /// This is the knob with immediate effect on the bootstrap session,
    /// the one session that transfers the provider's entire set: the set
    /// arrives as supply *runs* (batched leaf-record messages), the
    /// greeting carries each side's target, and the session runs at the
    /// **minimum** of the two — so a memory-constrained newcomer's
    /// setting bounds the frames the provider builds *for* it, and one
    /// run's encoded bytes is the newcomer's per-message buffering unit
    /// for the whole transfer. Any value is safe, including zero (one
    /// leaf per message).
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
    /// residue (the two-generals problem): if the session fails at the
    /// very end with [`Error::Epilogue`], the provider may have committed
    /// while our side reports an error, and the forked identity is lost.
    /// Losing a fork is safe — no invariant depends on it arriving — but
    /// not free: its id-region is identity space gone for good, unless
    /// coordination outside this library reclaims it. What `Err` and
    /// cancellation leave behind is stated in [what a session
    /// promises](crate::link::Link#what-a-session-promises).
    ///
    /// The peer arrives unbookmarked: its identity has been forked away
    /// to us but not yet persisted, so a crash before it is recorded
    /// strands it. To make the received identity durable, attach a
    /// [`Bookmark`](crate::Bookmark) with [`bookmark`](Peer::bookmark)
    /// immediately.
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

#[cfg(test)]
mod tests;
