//! The blocking face of the crate, for applications without an async
//! runtime.
//!
//! Every type here wraps its namesake at the crate root and blocks the
//! calling thread where the original returns a future; wire sessions run
//! over [`std::io::Read`]/[`Write`] instead of tokio's
//! traits. The semantics — lifecycle, session contract, observer
//! guarantees — are identical, and are documented once, on the async item
//! each wrapper names. Read the [crate docs](crate) first; this page only
//! describes what blocking changes.
//!
//! # Which face should you use?
//!
//! Use this module when there is no async runtime in the program at all: a
//! CLI, a plain-threads service, a test harness. If a runtime exists —
//! even a single-threaded one — use the crate root instead. These calls
//! block their thread until a whole wire session or observation completes,
//! and a blocked runtime thread stalls every task scheduled on it; in the
//! worst case it stalls the very task that was about to serve this
//! session's counterparty, which is a deadlock. If async code must call a
//! blocking session anyway, isolate it on a dedicated thread (e.g.
//! tokio's `spawn_blocking`).
//!
//! Blocking calls cannot be cancelled: where the async face lets you drop
//! a session future ([crate docs](crate#what-a-session-promises)), the
//! blocking face returns only when the session has finished or failed. Use
//! the transport's own timeouts (e.g. socket read timeouts, which surface
//! here as session errors) to bound a stalled counterparty.
//!
//! The change-driven driver ([`crate::Rumors::gossip_when`]) has no
//! blocking face: it is one task racing a policy stream against the wire,
//! which is concurrency a blocking call cannot express. A blocking
//! application schedules its own [`gossip`](Rumors::gossip) calls instead.
//!
//! # Example
//!
//! The crate-root example, with no runtime anywhere: plain threads and OS
//! pipes.
//!
//! ```
//! use rumors::sync::Peer;
//!
//! let alice = Peer::<String>::seed().into_rumors();
//! alice.send("the meeting is at noon".to_string());
//!
//! // Any Read/Write pair carries a session; here, two OS pipes.
//! let (mut alice_read, mut bob_write) = std::io::pipe()?;
//! let (mut bob_read, mut alice_write) = std::io::pipe()?;
//!
//! // Alice serves one gossip session from a plain thread...
//! let mut serve = alice.clone();
//! let serving = std::thread::spawn(move || {
//!     serve.gossip(&mut alice_read, &mut alice_write)
//! });
//!
//! // ...and Bob joins through it, blocking until he holds a full replica.
//! let bob = Peer::<String>::bootstrap(&mut bob_read, &mut bob_write)?
//!     .expect("alice is established, not herself bootstrapping");
//! serving.join().expect("serving thread panicked")?;
//!
//! let snapshot = bob.into_rumors().snapshot();
//! let (_key, _version, message) = snapshot.iter().next().expect("one live message");
//! assert_eq!(message.as_str(), "the meeting is at noon");
//! # Ok::<(), rumors::Error>(())
//! ```

use std::io::{Read, Write};
use std::sync::Arc;

use borsh::{BorshDeserialize, BorshSerialize};
use futures::io::AllowStdIo;
use tokio_util::compat::{FuturesAsyncReadCompatExt, FuturesAsyncWriteCompatExt};

pub use crate::{
    Batch, Error, Key, Network, PROTOCOL_MAGIC, PROTOCOL_VERSION, Snapshot, Version, causally,
};
pub use ::before;
pub use ::borsh;

/// The blocking face of [`crate::Peer`]: the unique `!Clone` anchor that
/// seeds, bootstraps, and retires. See [`crate::Peer`] for the model, the
/// lifecycle, and a complete example.
pub struct Peer<T>(crate::Peer<T>);

impl<T> std::fmt::Debug for Peer<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.0, f)
    }
}

/// The blocking face of [`crate::Rumors`]: the cloneable working handle
/// that sends, redacts, observes, and gossips. See [`crate::Rumors`] for
/// the contract of every operation.
pub struct Rumors<T>(crate::Rumors<T>);

impl<T> std::fmt::Debug for Rumors<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.0, f)
    }
}

/// The outcome of [`Peer::retire`], carrying this module's [`Peer`]; the
/// four outcomes and their contracts are those of [`crate::Retire`].
#[must_use = "a declined or recovered retirement hands the Peer back; dropping it leaks the identity"]
#[derive(Debug)]
pub enum Retire<T> {
    /// **Retired**: the peer absorbed our identity. See
    /// [`crate::Retire::Retired`].
    Retired,
    /// **Declined, unchanged**: retry elsewhere. See
    /// [`crate::Retire::Declined`].
    Declined {
        /// The intact retiree.
        peer: Peer<T>,
    },
    /// **Recovered, unchanged**: the session failed before anything was at
    /// stake; retry. See [`crate::Retire::Recovered`].
    Recovered {
        /// The intact retiree.
        peer: Peer<T>,
        /// What failed the session.
        error: Error,
    },
    /// **Uncertain**: the identity may be on either side, so the retiree
    /// is consumed. See [`crate::Retire::Uncertain`].
    Uncertain {
        /// What failed the session.
        error: Error,
    },
}

/// The blocking face of [`crate::Messages`], the arbitrary-order live
/// observer; see it for the delivery and checkpoint contract.
///
/// Three ways to consume it: the [`Iterator`] impl blocks for owned items
/// (`None` is terminal — the set has closed and is fully delivered);
/// [`borrow_next`](Self::borrow_next) blocks but lends instead of cloning;
/// [`try_next`](Self::try_next) never blocks.
pub struct Messages<T>(crate::Messages<T>);

/// The outcome of one non-blocking observer step ([`Messages::try_next`]).
#[derive(Debug)]
pub enum TryNext<'a, T> {
    /// The observer yielded a message, lent until the observer's next call
    /// (exactly as [`Messages::borrow_next`] lends).
    Message((Key, &'a Version, &'a Arc<T>)),
    /// Nothing new to report right now; actors are still live, so more may
    /// come. Ask again later.
    Quiet,
    /// Every handle is gone and the complete final state has been yielded;
    /// no further message is possible.
    Ended,
}

impl<T> Messages<T> {
    /// Block until the next message, lending its version and value until
    /// the following call; `None` once no further message is possible.
    /// The blocking [`crate::Messages::borrow_next`].
    pub fn borrow_next(&mut self) -> Option<(Key, &Version, &Arc<T>)>
    where
        T: Send + Sync,
    {
        pollster::block_on(self.0.borrow_next())
    }

    /// Take one non-blocking step: a message if one is ready, [`Quiet`] (ask
    /// again later) if not, [`Ended`] if no further message is possible.
    ///
    /// [`Quiet`]: TryNext::Quiet
    /// [`Ended`]: TryNext::Ended
    pub fn try_next(&mut self) -> TryNext<'_, T>
    where
        T: Send + Sync,
    {
        use futures::FutureExt;
        match self.0.borrow_next().now_or_never() {
            None => TryNext::Quiet,
            Some(None) => TryNext::Ended,
            Some(Some(message)) => TryNext::Message(message),
        }
    }

    /// The sound resume point; the contract is
    /// [`crate::Messages::checkpoint`]'s.
    pub fn checkpoint(&self) -> &Version {
        self.0.checkpoint()
    }
}

impl<T: Send + Sync + 'static> Iterator for Messages<T> {
    type Item = (Key, Version, Arc<T>);

    fn next(&mut self) -> Option<Self::Item> {
        pollster::block_on(futures::StreamExt::next(&mut self.0))
    }
}

/// The blocking face of [`crate::CausalMessages`], the causal-order live
/// observer; see it for the ordering, cost, and checkpoint contract. Consumed
/// exactly as [`Messages`] is: blocking [`Iterator`], lending
/// [`borrow_next`](Self::borrow_next), non-blocking
/// [`try_next`](Self::try_next).
pub struct CausalMessages<T>(crate::CausalMessages<T>);

impl<T> CausalMessages<T> {
    /// Block until the next message in causal order, lending its version
    /// and value until the following call; `None` once no further message
    /// is possible. The blocking [`crate::CausalMessages::borrow_next`].
    pub fn borrow_next(&mut self) -> Option<(Key, &Version, &Arc<T>)>
    where
        T: Send + Sync,
    {
        pollster::block_on(self.0.borrow_next())
    }

    /// Take one non-blocking step: a message if one is ready, [`Quiet`]
    /// (ask again later) if not, [`Ended`] if no further message is
    /// possible.
    ///
    /// [`Quiet`]: TryNext::Quiet
    /// [`Ended`]: TryNext::Ended
    pub fn try_next(&mut self) -> TryNext<'_, T>
    where
        T: Send + Sync,
    {
        use futures::FutureExt;
        match self.0.borrow_next().now_or_never() {
            None => TryNext::Quiet,
            Some(None) => TryNext::Ended,
            Some(Some(message)) => TryNext::Message(message),
        }
    }

    /// The sound resume point; the contract is
    /// [`crate::CausalMessages::checkpoint`]'s.
    pub fn checkpoint(&self) -> &Version {
        self.0.checkpoint()
    }
}

impl<T: Send + Sync + 'static> Iterator for CausalMessages<T> {
    type Item = (Key, Version, Arc<T>);

    fn next(&mut self) -> Option<Self::Item> {
        pollster::block_on(futures::StreamExt::next(&mut self.0))
    }
}

impl<T> Peer<T> {
    /// Mint a fresh universe. Identical to [`crate::Peer::seed`]: no wire,
    /// nothing to block on.
    pub fn seed() -> Self {
        Peer(crate::Peer::seed())
    }

    #[doc(hidden)]
    pub fn seed_rng<R: rand::RngCore + ?Sized>(rng: &mut R) -> Self {
        Peer(crate::Peer::seed_rng(rng))
    }

    /// The universe's identifier; see [`crate::Peer::network`].
    pub fn network(&self) -> Network {
        self.0.network()
    }

    /// Join an existing universe through a connected peer, blocking until
    /// the session completes: the blocking [`crate::Peer::bootstrap`].
    /// `Ok(None)` means the counterparty was itself still bootstrapping;
    /// try a more established peer.
    pub fn bootstrap<R, W>(read: &mut R, write: &mut W) -> Result<Option<Self>, Error>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync,
        R: Read + Send,
        W: Write + Send,
    {
        let mut read = AllowStdIo::new(read).compat();
        let mut write = AllowStdIo::new(write).compat_write();
        pollster::block_on(crate::Peer::<T>::bootstrap(&mut read, &mut write))
            .map(|known| known.map(Peer))
    }

    /// Leave the universe, donating this identity through any gossiping
    /// peer, blocking until the session completes: the blocking
    /// [`crate::Peer::retire`]. The [`Retire`] outcome says what survived.
    pub fn retire<R, W>(self, read: &mut R, write: &mut W) -> Retire<T>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync,
        R: Read + Send,
        W: Write + Send,
    {
        let mut read = AllowStdIo::new(read).compat();
        let mut write = AllowStdIo::new(write).compat_write();
        match pollster::block_on(self.0.retire(&mut read, &mut write)) {
            crate::Retire::Retired => Retire::Retired,
            crate::Retire::Declined { peer } => Retire::Declined { peer: Peer(peer) },
            crate::Retire::Recovered { peer, error } => Retire::Recovered {
                peer: Peer(peer),
                error,
            },
            crate::Retire::Uncertain { error } => Retire::Uncertain { error },
        }
    }

    /// Trade the anchor for working handles; see
    /// [`crate::Peer::into_rumors`].
    pub fn into_rumors(self) -> Rumors<T> {
        Rumors(self.0.into_rumors())
    }

    #[doc(hidden)]
    pub fn warm_caches(&self) {
        self.0.warm_caches();
    }

    #[cfg(any(test, feature = "test-internals"))]
    #[doc(hidden)]
    pub fn dangerously_alias_party(&self) -> Option<before::Party> {
        self.0.dangerously_alias_party()
    }
}

impl<T> Clone for Rumors<T> {
    fn clone(&self) -> Self {
        Rumors(self.0.clone())
    }
}

impl<T> Rumors<T> {
    /// Send a message. Identical to [`crate::Rumors::send`]: the returned
    /// [`Batch`] commits when dropped.
    pub fn send(&self, message: T) -> Batch<'_, T>
    where
        T: BorshSerialize + Send + Sync,
    {
        self.0.send(message)
    }

    /// Redact a message. Identical to [`crate::Rumors::redact`]: the
    /// returned [`Batch`] commits when dropped.
    pub fn redact(&self, key: Key) -> Batch<'_, T>
    where
        T: Send + Sync,
    {
        self.0.redact(key)
    }

    /// Start an empty [`Batch`]; see [`crate::Rumors::batch`].
    pub fn batch(&self) -> Batch<'_, T>
    where
        T: Send + Sync,
    {
        self.0.batch()
    }

    /// Run one reconciliation session with one peer, blocking until it
    /// completes: the blocking [`crate::Rumors::gossip`].
    pub fn gossip<R, W>(&mut self, read: &mut R, write: &mut W) -> Result<(), Error>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync,
        R: Read + Send,
        W: Write + Send,
    {
        let mut read = AllowStdIo::new(read).compat();
        let mut write = AllowStdIo::new(write).compat_write();
        pollster::block_on(self.0.gossip(&mut read, &mut write))
    }

    /// Give up this handle and reclaim the [`Peer`], blocking until every other
    /// handle has dropped — indefinitely, if another thread holds a clone it
    /// never drops. The blocking [`crate::Rumors::try_into_peer`], including
    /// its exactly-one-winner contract.
    pub fn try_into_peer(self) -> Option<Peer<T>> {
        pollster::block_on(self.0.try_into_peer()).map(Peer)
    }

    /// Observe every message, arbitrary order, from genesis onward; see
    /// [`crate::Rumors::messages`].
    pub fn messages(&self) -> Messages<T>
    where
        T: Send + Sync,
    {
        self.messages_since(Version::new())
    }

    /// Observe every message, arbitrary order, not already contained in
    /// `since`; see [`crate::Rumors::messages_since`].
    pub fn messages_since(&self, since: Version) -> Messages<T>
    where
        T: Send + Sync,
    {
        Messages(self.0.messages_since(since))
    }

    /// Observe every message, causal order, from genesis onward; see
    /// [`crate::Rumors::causal_messages`].
    pub fn causal_messages(&self) -> CausalMessages<T>
    where
        T: Send + Sync,
    {
        self.causal_messages_since(Version::new())
    }

    /// Observe every message, causal order, not already contained in `since`;
    /// see [`crate::Rumors::causal_messages_since`].
    pub fn causal_messages_since(&self, since: Version) -> CausalMessages<T>
    where
        T: Send + Sync,
    {
        CausalMessages(self.0.causal_messages_since(since))
    }

    /// The universe's identifier; see [`crate::Rumors::network`].
    pub fn network(&self) -> Network {
        self.0.network()
    }

    /// Take a consistent point-in-time snapshot; see
    /// [`crate::Rumors::snapshot`].
    pub fn snapshot(&self) -> Snapshot<T> {
        self.0.snapshot()
    }

    #[doc(hidden)]
    pub fn warm_caches(&self) {
        self.0.warm_caches();
    }

    #[cfg(any(test, feature = "test-internals"))]
    #[doc(hidden)]
    pub fn dangerously_alias_party(&self) -> Option<before::Party> {
        self.0.dangerously_alias_party()
    }
}
