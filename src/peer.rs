//! The local rumor set: [`Peer`] and its synchronized state, plus the local
//! API for sending, redacting, and observing messages. The wire-session
//! drivers (bootstrap, gossip, retire) live in [`gossip`].

use std::io::{Read, Write};
use std::marker::PhantomData;
use std::sync::Arc;

use before::Party;
use borsh::{BorshDeserialize, BorshSerialize};
use futures::io::AllowStdIo;
use rand::{RngCore, rngs::OsRng};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::{Mutex, watch};
use tokio_util::compat::{FuturesAsyncReadCompatExt as _, FuturesAsyncWriteCompatExt as _};

use crate::bookmark::{BookmarkError, Bookmarked, NoBookmark};
use crate::mode::{Async, Mode};
use crate::tree::Tree;
use crate::{
    Batch, Bookmark, CausalMessages, Error, Key, Network, Rumors, Snapshot, UnorderedMessages,
    Version,
};

mod gossip;

pub use gossip::{Gossiped, Led, PROTOCOL_MAGIC, PROTOCOL_VERSION, Retire, Unbookmarked};

/// The start and end of the lifecycle of a [`Rumors`].
///
/// A [`Peer`] is the unique `!Clone` anchor for the identity of a participant
/// in the gossip protocol. Peer identity in [`rumors`](crate) is *not*
/// self-sovereign; you might say that in order to have a sense of `self`, you
/// must derive it from your community of [`Peer`]s. Exactly *one* [`Peer`]
/// should call [`Peer::seed`] to establish the unique [`Network`]; peers whose
/// identity descends ultimately from different initial calls to [`Peer::seed`]
/// will never be able to [`gossip`](Rumors::gossip) with one another.
///
/// You can only get your hands on a [`Peer`] when there are no existing
/// [`Rumors`] handles outstanding, which ensures that it is statically
/// impossible to [`retire`](Peer::retire) a [`Peer`] out from under another
/// extant handle to the same identity.
///
/// # Example
///
/// The lifecycle of a [`Peer`] usually looks something like this:
///
/// ```
/// use rumors::{Peer, Retire};
///
/// # tokio::runtime::Builder::new_current_thread()
/// #     .build()
/// #     .unwrap()
/// #     .block_on(async {
/// # // The counterparty this example talks to: the universe's seed, serving
/// # // the bootstrap and later absorbing the retirement, over in-memory pipes.
/// # let counterparty = Peer::<String>::seed().into_rumors();
/// # let (near, far) = tokio::io::duplex(64 * 1024);
/// # let (mut r, mut w) = tokio::io::split(near);
/// # let serve = counterparty.clone();
/// # tokio::spawn(async move {
/// #     let (mut r, mut w) = tokio::io::split(far);
/// #     serve.gossip(&mut r, &mut w).await.unwrap();
/// # });
/// # async fn bootstrap_from_another_peer() -> Result<Peer<String>, rumors::Error> {
/// #     unreachable!("the example's counterparty is the established seed")
/// # }
/// // Join an existing universe through any connected peer. (The universe's
/// // very first peer is created with `Peer::seed()` instead.)
/// let peer = match Peer::<String>::bootstrap(&mut r, &mut w).await? {
///     Some(peer) => peer,
///     // The counterparty was *itself* bootstrapping: neither side holds
///     // a universe to share yet, and nothing was exchanged. Connect to a
///     // different, more established peer and try again.
///     None => bootstrap_from_another_peer().await?,
/// };
///
/// // A `Peer` is `!Clone`; trade it for `Rumors` handles to send and gossip.
/// let rumors = peer.into_rumors();
/// let other = rumors.clone();
/// // ... send, redact, and gossip concurrently through the clones ...
///
/// // Once every other handle is gone, the unique `Peer` can be reclaimed.
/// drop(other);
/// let Some(peer) = rumors.try_into_peer().await else {
///     unreachable!("all other handles were dropped already");
/// };
///
/// // Leave the universe, donating our identity to any gossiping peer (it
/// // does not need to be the one we bootstrapped from).
/// # let (near, far) = tokio::io::duplex(64 * 1024);
/// # let (mut r, mut w) = tokio::io::split(near);
/// # tokio::spawn(async move {
/// #     let (mut r, mut w) = tokio::io::split(far);
/// #     counterparty.gossip(&mut r, &mut w).await.unwrap();
/// # });
/// //
/// // Each outcome tells us whether our identity survived the attempt:
/// // `Declined` and `Recovered` hand the peer back to retry elsewhere,
/// // while `Retired` and `Uncertain` consume it.
/// let retry = match peer.retire(&mut r, &mut w).await {
///     // The peer absorbed our identity; nothing more to do.
///     Retire::Retired => None,
///     // The peer was itself retiring, so it could not absorb us;
///     // retry against a different peer.
///     Retire::Declined { peer } => Some(peer),
///     // The session failed before we sent our identity to the peer;
///     // retry here or elsewhere.
///     Retire::Recovered { peer, error: _ } => Some(peer),
///     // The session failed after we sent our identity: the peer may
///     // hold it, so we cannot safely retry.
///     Retire::Uncertain { error } => return Err(error),
/// };
/// assert!(retry.is_none(), "the example's retirement succeeds");
/// # Ok(())
/// # })?;
/// # Ok::<(), rumors::Error>(())
/// ```
///
/// # Bootstrapping without consensus
///
/// If your application admits a distinguished "first peer" (for example, via
/// leader election or another consensus mechanism), have that peer call
/// [`Peer::seed`].
///
/// Absent any true consensus mechanism, another reasonable approach to
/// bootstrapping a [`Network`] is for *every* [`Peer`] to initially call
/// [`Peer::seed`] and attempt to [`gossip`](crate::Rumors::gossip) with all
/// others. At first, this will lead to many [`Error::NetworkMismatch`]es;
/// whenever a peer observes one, it can use a deterministic metric to decide
/// whether it or its peer should dominate.
///
/// A reasonable such metric would be to compare the `remote_min_ticks` reported
/// by the peer in the error with your own
/// [`Snapshot::latest`](crate::Snapshot::latest)'s [`Version::min_ticks`], so
/// that whichever network with greater minimal event count wins, with total
/// comparison on [`Network`] breaking ties. Based on this comparison, both
/// sides can come to uncoordinated consensus on which will persist in its
/// [`Peer`] identity (the greater), and which will attempt to
/// re-[`bootstrap`](Peer::bootstrap) into the dominating [`Network`] (the
/// lesser).
///
/// If peers are reasonably well-connected to one another as the network gets
/// started, this will quickly lead to a stable and steady state which can only
/// be disrupted if a group of new peers join only with one another and spend a
/// long time partitioned from the rest of the network before reuniting with it.
pub struct Peer<T, B: BookmarkError = NoBookmark, M: Mode = Async> {
    pub(crate) network: Network,
    pub(crate) inner: watch::Sender<Inner<T>>,
    /// The identity bookmark: persistence handle and its in-memory record,
    /// behind an async mutex and shared with every [`Rumors`] clone. Separate
    /// from `inner` because persisting is `async` and the record is `!Clone`;
    /// see [`Bookmarked`].
    pub(crate) bookmark: Arc<Mutex<Bookmarked<B, M>>>,
    /// The I/O [`Mode`] witness: selects the async or blocking face. Purely
    /// type-level — it carries no data and never affects the engine, only which
    /// `impl` block a caller reaches. `fn() -> M` so `M` constrains neither
    /// variance nor auto-traits.
    pub(crate) marker: PhantomData<fn() -> M>,
}

/// The replica's shared mutable state, behind the `watch` channel every
/// handle and observer subscribes to: the identity (absent only while a
/// retirement has it in flight) and the content tree. Mutations happen
/// inside `send_if_modified` critical sections so observers wake exactly
/// once per committed change.
pub(crate) struct Inner<T> {
    pub(crate) party: Option<Party>,
    pub(crate) tree: Tree<T>,
}

/// A summary view (network, latest version, live-message count), independent
/// of `T: Debug`: the messages themselves are not printed.
impl<T, B: BookmarkError, M: Mode> std::fmt::Debug for Peer<T, B, M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner = self.inner.borrow();
        f.debug_struct("Peer")
            .field("network", &self.network)
            .field("latest", inner.tree.latest())
            .field("len", &inner.tree.len())
            .finish_non_exhaustive()
    }
}

impl<T, M: Mode> Peer<T, NoBookmark, M> {
    /// Create the distinguished seed rumor set: the single root from which
    /// every other participant must [`bootstrap`](Peer::bootstrap).
    ///
    /// Call this exactly once per universe of cooperating peers.
    pub fn seed() -> Self {
        Self::seed_rng(&mut OsRng)
    }

    /// Like [`seed`](Self::seed), but draws the universe's [`Network`]
    /// identifier from a caller-supplied RNG instead of [`OsRng`].
    #[doc(hidden)]
    pub fn seed_rng<R: RngCore + ?Sized>(rng: &mut R) -> Self {
        Self {
            network: Network::from_rng(rng),
            inner: watch::Sender::new(Inner {
                party: Some(Party::seed()),
                tree: Tree::new(),
            }),
            bookmark: Arc::new(Mutex::new(Bookmarked::new(NoBookmark))),
            marker: PhantomData,
        }
    }
}

impl<T> Peer<T> {
    /// Bootstrap a brand-new rumor set from a remote peer.
    ///
    /// `Ok(None)` means the counterparty was itself still bootstrapping, so
    /// neither side had anything to share and nothing was exchanged. Connect to
    /// another peer and try again.
    ///
    /// The peer arrives unbookmarked: its identity has been forked away to us
    /// but not yet persisted, so a crash before it is recorded strands it. To
    /// make the received identity durable, attach a [`Bookmark`] with
    /// [`bookmark`](Peer::bookmark) immediately.
    pub async fn bootstrap<'a, R, W>(
        read: &'a mut R,
        write: &'a mut W,
    ) -> Result<Option<Self>, Error>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync + 'a,
        R: AsyncRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
    {
        Self::bootstrap_inner(read, write).await
    }

    /// Attach `bookmark` to this [`Peer`], persisting its identity before
    /// returning.
    ///
    /// This peer's own identity is [`read`](crate::Bookmark::read) into the
    /// record and [`written`](crate::Bookmark::write) back *eagerly*, here, so a
    /// freshly received fork cannot strand on a crash before the first gossip.
    /// Reclaiming *other* stranded identities — which grows the live party — is
    /// left to the first gossip, behind that path's persist gate, never done at
    /// attach.
    ///
    /// A pristine [`seed`](Peer::seed), with nothing sent and no identity yet
    /// donated or absorbed, has nothing worth persisting, so this touches
    /// storage only once the peer *knows* something: any content, or any
    /// identity beyond the undivided seed.
    ///
    /// # Errors
    ///
    /// If the bookmark cannot be read or written, nothing reaches storage and
    /// the peer is handed back **untouched**, still unbookmarked, inside
    /// [`Unbookmarked`], to drop or retry. Because the attach never reclaims, the
    /// live party is exactly as it was: a failed attach cannot leave a reclaimed
    /// region live in this peer yet stranded on disk.
    pub async fn bookmark<B: Bookmark>(
        self,
        bookmark: B,
    ) -> Result<Peer<T, B, Async>, Unbookmarked<T, B, Async>> {
        self.bookmark_inner(bookmark).await
    }
}

impl<T, B: Bookmark> Peer<T, B> {
    /// Retire this rumor set into a remote peer, handing it our identity so
    /// that it can be recycled by the network.
    ///
    /// See the [type-level lifecycle example](Peer) for how the four
    /// [`Retire`] outcomes are handled; in brief, a session reconciles content
    /// exactly as [`gossip`](crate::Rumors::gossip) would and then the peer
    /// absorbs our identity, with the outcome reporting what survived.
    pub async fn retire<'a, R, W>(self, read: &'a mut R, write: &'a mut W) -> Retire<T, B, Async>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync + 'a,
        R: AsyncRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
    {
        self.retire_inner(read, write).await
    }
}

impl<T, B: BookmarkError, M: Mode> Peer<T, B, M> {
    /// The globally unique identifier for this network of gossiping [`Peer`]s.
    pub fn network(&self) -> Network {
        self.network
    }

    /// Convert the [`Peer`] into a [`Rumors`] so it can [`send`](Rumors::send),
    /// [`redact`](Rumors::redact), and [`gossip`](Rumors::gossip).
    ///
    /// Unlike [`Peer`], [`Rumors`] is [`Clone`], so that gossip may proceed
    /// concurrently. Once only one remaining [`Rumors`] exists again, it can be
    /// converted back into a [`Peer`] using
    /// [`try_into_peer`](Rumors::try_into_peer).
    pub fn into_rumors(self) -> Rumors<T, B, M> {
        Rumors::new(self)
    }

    pub(crate) fn send(&self, message: T) -> Batch<'_, T>
    where
        T: BorshSerialize + Send + Sync,
    {
        let mut batch = self.batch();
        batch.send(message);
        batch
    }

    pub(crate) fn redact(&self, key: Key) -> Batch<'_, T>
    where
        T: Send + Sync,
    {
        let mut batch = self.batch();
        batch.redact(key);
        batch
    }

    pub(crate) fn batch(&self) -> Batch<'_, T>
    where
        T: Send + Sync,
    {
        Batch::new(&self.inner)
    }

    pub(crate) fn snapshot(&self) -> Snapshot<T> {
        Snapshot::new(self.network, self.inner.borrow().tree.clone())
    }

    pub(crate) fn unordered_messages(&self) -> UnorderedMessages<T, M>
    where
        T: Send + Sync,
    {
        self.messages_since(Version::new())
    }

    pub(crate) fn messages_since(&self, since: Version) -> UnorderedMessages<T, M>
    where
        T: Send + Sync,
    {
        UnorderedMessages::subscribe(&self.inner, since)
    }

    pub(crate) fn causal_messages(&self) -> CausalMessages<T, M>
    where
        T: Send + Sync,
    {
        self.causal_messages_since(Version::new())
    }

    pub(crate) fn causal_messages_since(&self, since: Version) -> CausalMessages<T, M>
    where
        T: Send + Sync,
    {
        CausalMessages::subscribe(&self.inner, since)
    }

    /// Force this set's tree to compute its lazy structural memos (observable
    /// hash and ceiling/floor version bounds), so a subsequent operation is
    /// timed against its own work. For benchmark and test calibration only.
    #[doc(hidden)]
    pub fn warm_caches(&self) {
        self.inner.borrow().tree.warm_caches();
    }

    /// Alias this set's live party for invariant assertions in tests:
    /// compare it, [`join`](Party::join) it into an accounting fold, or test
    /// [`is_disjoint`](Party::is_disjoint) — never use it as an identity.
    /// The alias shares the live party's id-region without forking it, so
    /// treating it as a participant violates the linearity everything else
    /// rests on. `None` only while a retirement has the party in flight.
    #[cfg(any(test, feature = "test-internals"))]
    #[doc(hidden)]
    pub fn dangerously_alias_party(&self) -> Option<Party> {
        self.inner
            .borrow()
            .party
            .as_ref()
            .map(Party::dangerously_alias)
    }
}

impl<T> crate::sync::Peer<T> {
    /// Blocking [`bootstrap`](Peer::bootstrap), over [`std::io`].
    pub fn bootstrap<R, W>(read: &mut R, write: &mut W) -> Result<Option<Self>, Error>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync,
        R: Read + Send,
        W: Write + Send,
    {
        let mut read = AllowStdIo::new(read).compat();
        let mut write = AllowStdIo::new(write).compat_write();
        pollster::block_on(Self::bootstrap_inner(&mut read, &mut write))
    }

    /// Blocking [`bookmark`](Peer::bookmark), attaching a
    /// [`sync::Bookmark`](crate::sync::Bookmark).
    pub fn bookmark<B: crate::sync::Bookmark + Send + Sync>(
        self,
        bookmark: B,
    ) -> Result<crate::sync::Peer<T, B>, crate::sync::Unbookmarked<T, B>> {
        pollster::block_on(self.bookmark_inner(bookmark))
    }
}

impl<T, B: crate::sync::Bookmark + Send + Sync> crate::sync::Peer<T, B> {
    /// Blocking [`retire`](Peer::retire), over [`std::io`].
    pub fn retire<R, W>(self, read: &mut R, write: &mut W) -> crate::sync::Retire<T, B>
    where
        T: BorshDeserialize + BorshSerialize + Send + Sync,
        R: Read + Send,
        W: Write + Send,
    {
        let mut read = AllowStdIo::new(read).compat();
        let mut write = AllowStdIo::new(write).compat_write();
        pollster::block_on(self.retire_inner(&mut read, &mut write))
    }
}
