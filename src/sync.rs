//! A synchronous interface to the crate, for applications without an async
//! runtime.
//!
//! Every type here wraps its namesake at the crate root and blocks the calling
//! thread wherever the original would await. Async functions become ordinary
//! functions, streams become [`Iterator`]s, and gossip runs over
//! [`std::io::Read`]/[`Write`] instead of [`tokio`]'s
//! [`AsyncRead`](tokio::io::AsyncRead)/[`AsyncWrite`](tokio::io::AsyncWrite).
//!
//! Modulo blocking, the behavior is identical to the main asynchronous
//! interface. Read the [crate docs](crate) first.
//!
//! # Differences from the asynchronous interface
//!
//! Blocking calls cannot be cancelled: where the main asynchronous interface
//! lets you drop a session future ([crate
//! docs](crate#what-a-session-promises)), the synchronous interface returns
//! only when the session has finished or failed. Use your transport's own
//! timeouts (e.g. socket read timeouts, which surface here as session errors)
//! to bound a stalled counterparty.
//!
//! The change-driven driver ([`crate::Rumors::gossip_when`]) has no blocking
//! equivalent: it is one task racing a policy stream against the wire, which is
//! concurrency a blocking call cannot express. A blocking application schedules
//! its own [`gossip`](Rumors::gossip) calls instead; [`Changes`] can wake other
//! change-driven work, but is not a gossip schedule by itself, because one must
//! wait concurrently for local and remote changes to implement bidirectional
//! push-based gossip.
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

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::sync::Arc;

use ::before::Clock;
use borsh::{BorshDeserialize, BorshSerialize};
use futures::io::AllowStdIo;
use tokio_util::compat::{FuturesAsyncReadCompatExt, FuturesAsyncWriteCompatExt};

pub use crate::{
    Batch, Error, Key, MERKLE_HASH_LEN, Network, NoBookmark, PROTOCOL_MAGIC, PROTOCOL_VERSION,
    Snapshot, Version, causally,
};
pub use ::before;
pub use ::borsh;

/// The synchronous [`crate::Bookmark`].
pub trait Bookmark {
    /// What a failed [`read`](Self::read) or [`write`](Self::write) reports;
    /// see [`crate::Bookmark::Error`].
    type Error: std::error::Error + Send + Sync + 'static;

    /// Read the persisted record, or an empty map if nothing is stored yet.
    fn read(&self) -> Result<BTreeMap<Network, Vec<Clock>>, Self::Error>;

    /// Durably (atomically) replace the persisted record with `bookmarks`.
    fn write(&self, bookmarks: &BTreeMap<Network, Vec<Clock>>) -> Result<(), Self::Error>;
}

/// Adapt a synchronous [`Bookmark`] to be a [`crate::Bookmark`].
///
/// You do not construct this; [`Peer::bookmark`] wraps your [`Bookmark`] in it.
/// It surfaces only in the type of a bookmarked blocking peer, [`Peer<T,
/// Blocking<B>>`](Peer), should you need to name one.
pub struct Blocking<B>(B);

impl<B: Bookmark + Send + Sync> crate::Bookmark for Blocking<B> {
    type Error = B::Error;

    async fn read(&self) -> Result<BTreeMap<Network, Vec<Clock>>, Self::Error> {
        self.0.read()
    }

    async fn write(&self, bookmarks: &BTreeMap<Network, Vec<Clock>>) -> Result<(), Self::Error> {
        self.0.write(bookmarks)
    }
}

/// A synchronous [`crate::Peer`].
pub struct Peer<T, B: crate::Bookmark = NoBookmark>(crate::Peer<T, B>);

impl<T, B: crate::Bookmark> std::fmt::Debug for Peer<T, B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.0, f)
    }
}

/// The synchronous [`crate::Rumors`].
pub struct Rumors<T, B: crate::Bookmark = NoBookmark>(crate::Rumors<T, B>);

impl<T, B: crate::Bookmark> std::fmt::Debug for Rumors<T, B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.0, f)
    }
}

/// The synchronous [`crate::Retire`].
#[must_use = "a declined or recovered retirement hands the Peer back; dropping it leaks the identity"]
#[derive(Debug)]
pub enum Retire<T, B: crate::Bookmark = NoBookmark> {
    /// **Retired**: the peer absorbed our identity. See
    /// [`crate::Retire::Retired`].
    Retired,
    /// **Declined, unchanged**: retry elsewhere. See
    /// [`crate::Retire::Declined`].
    Declined {
        /// The intact retiree.
        peer: Peer<T, B>,
    },
    /// **Recovered, unchanged**: the session failed before anything was at
    /// stake; retry. See [`crate::Retire::Recovered`].
    Recovered {
        /// The intact retiree.
        peer: Peer<T, B>,
        /// What failed the session.
        error: Error<B>,
    },
    /// **Uncertain**: the identity may be on either side, so the retiree
    /// is consumed. See [`crate::Retire::Uncertain`].
    Uncertain {
        /// What failed the session.
        error: Error<B>,
    },
}

/// The synchronous [`crate::Unbookmarked`].
#[must_use = "a failed `Peer::bookmark` hands the `Peer` back; dropping it strands the identity"]
#[derive(Debug)]
pub struct Unbookmarked<T, B: Bookmark> {
    /// The peer, its identity intact and no bookmark attached.
    pub peer: Peer<T>,
    /// What the bookmark's [`read`](Bookmark::read) or
    /// [`write`](Bookmark::write) reported.
    pub error: B::Error,
}

/// The synchronous [`crate::Messages`].
///
/// There are three ways to consume it:
///
/// - The [`Iterator`] impl blocks for owned items (`None` means the set has
///   closed and is fully delivered);
/// - [`borrow_next`](Self::borrow_next) blocks but lends instead of cloning;
/// - [`try_next`](Self::try_next) never blocks, returning [`TryNext`] to provide
///   either a value or a reason why one can't.
pub struct Messages<T>(crate::Messages<T>);

/// The outcome of [`Messages::try_next`] or [`CausalMessages::try_next`].
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
    /// The synchronous [`crate::Messages::borrow_next`].
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

    /// The synchronous [`crate::Messages::checkpoint`].
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

/// The synchronous [`crate::CausalMessages`].
///
/// There are three ways to consume it:
///
/// - The [`Iterator`] impl blocks for owned items (`None` means the set has
///   closed and is fully delivered);
/// - [`borrow_next`](Self::borrow_next) blocks but lends instead of cloning;
/// - [`try_next`](Self::try_next) never blocks, returning [`TryNext`] to provide
///   either a value or a reason why one can't.
pub struct CausalMessages<T>(crate::CausalMessages<T>);

impl<T> CausalMessages<T> {
    /// The synchronous [`crate::CausalMessages::borrow_next`].
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

    /// The synchronous [`crate::CausalMessages::checkpoint`].
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

/// The synchronous [`crate::Changes`].
pub struct Changes<T>(crate::Changes<T>);

/// The outcome of [`Changes::try_next`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TryTick {
    /// The set advanced since the last report (a fresh signal's first step
    /// is always a tick).
    Tick,
    /// No advance since the last report; handles are still live, so more
    /// may come. Ask again later.
    Quiet,
    /// Every handle is gone and no further change is possible.
    Ended,
}

impl<T> Changes<T> {
    /// Take one non-blocking step: [`Tick`] if the set advanced since the
    /// last report, [`Quiet`] (ask again later) if not, [`Ended`] if no
    /// further change is possible.
    ///
    /// [`Tick`]: TryTick::Tick
    /// [`Quiet`]: TryTick::Quiet
    /// [`Ended`]: TryTick::Ended
    pub fn try_next(&mut self) -> TryTick
    where
        T: Send + Sync + 'static,
    {
        use futures::FutureExt;
        match futures::StreamExt::next(&mut self.0).now_or_never() {
            None => TryTick::Quiet,
            Some(None) => TryTick::Ended,
            Some(Some(())) => TryTick::Tick,
        }
    }
}

impl<T: Send + Sync + 'static> Iterator for Changes<T> {
    type Item = ();

    fn next(&mut self) -> Option<Self::Item> {
        pollster::block_on(futures::StreamExt::next(&mut self.0))
    }
}

impl<T> Peer<T> {
    /// The synchronous [`crate::Peer::seed`].
    pub fn seed() -> Self {
        Peer(crate::Peer::seed())
    }

    #[doc(hidden)]
    pub fn seed_rng<R: rand::RngCore + ?Sized>(rng: &mut R) -> Self {
        Peer(crate::Peer::seed_rng(rng))
    }

    /// The synchronous [`crate::Peer::bootstrap`].
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

    /// The synchronous [`crate::Peer::bookmark`].
    pub fn bookmark<B: Bookmark + Send + Sync>(
        self,
        bookmark: B,
    ) -> Result<Peer<T, Blocking<B>>, Unbookmarked<T, B>> {
        match pollster::block_on(self.0.bookmark(Blocking(bookmark))) {
            Ok(peer) => Ok(Peer(peer)),
            Err(crate::Unbookmarked { peer, error }) => Err(Unbookmarked {
                peer: Peer(peer),
                error,
            }),
        }
    }
}

impl<T, B: crate::Bookmark> Peer<T, B> {
    /// The synchronous [`crate::Peer::network`].
    pub fn network(&self) -> Network {
        self.0.network()
    }

    /// The synchronous [`crate::Peer::retire`].
    pub fn retire<R, W>(self, read: &mut R, write: &mut W) -> Retire<T, B>
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

    /// The synchronous [`crate::Peer::into_rumors`].
    pub fn into_rumors(self) -> Rumors<T, B> {
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

impl<T, B: crate::Bookmark> Clone for Rumors<T, B> {
    fn clone(&self) -> Self {
        Rumors(self.0.clone())
    }
}

impl<T, B: crate::Bookmark> Rumors<T, B> {
    /// The synchronous [`crate::Rumors::send`].
    pub fn send(&self, message: T) -> Batch<'_, T>
    where
        T: BorshSerialize + Send + Sync,
    {
        self.0.send(message)
    }

    /// The synchronous [`crate::Rumors::redact`].
    pub fn redact(&self, key: Key) -> Batch<'_, T>
    where
        T: Send + Sync,
    {
        self.0.redact(key)
    }

    /// The synchronous [`crate::Rumors::batch`].
    pub fn batch(&self) -> Batch<'_, T>
    where
        T: Send + Sync,
    {
        self.0.batch()
    }

    /// The synchronous [`crate::Rumors::gossip`].
    pub fn gossip<R, W>(&mut self, read: &mut R, write: &mut W) -> Result<(), Error<B>>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync,
        R: Read + Send,
        W: Write + Send,
    {
        let mut read = AllowStdIo::new(read).compat();
        let mut write = AllowStdIo::new(write).compat_write();
        pollster::block_on(self.0.gossip(&mut read, &mut write))
    }

    /// The synchronous [`crate::Rumors::try_into_peer`].
    pub fn try_into_peer(self) -> Option<Peer<T, B>> {
        pollster::block_on(self.0.try_into_peer()).map(Peer)
    }

    /// The synchronous [`crate::Rumors::messages`].
    pub fn messages(&self) -> Messages<T>
    where
        T: Send + Sync,
    {
        self.messages_since(Version::new())
    }

    /// The synchronous [`crate::Rumors::messages_since].
    pub fn messages_since(&self, since: Version) -> Messages<T>
    where
        T: Send + Sync,
    {
        Messages(self.0.messages_since(since))
    }

    /// The synchronous [`crate::Rumors::causal_messages].
    pub fn causal_messages(&self) -> CausalMessages<T>
    where
        T: Send + Sync,
    {
        self.causal_messages_since(Version::new())
    }

    /// The synchronous [`crate::Rumors::causal_messages_since`].
    pub fn causal_messages_since(&self, since: Version) -> CausalMessages<T>
    where
        T: Send + Sync,
    {
        CausalMessages(self.0.causal_messages_since(since))
    }

    /// The synchronous [`crate::Rumors::changes`].
    pub fn changes(&self) -> Changes<T> {
        Changes(self.0.changes())
    }

    /// The synchronous [`crate::Rumors::network`].
    pub fn network(&self) -> Network {
        self.0.network()
    }

    /// The synchronous [`crate::Rumors::snapshot`].
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
