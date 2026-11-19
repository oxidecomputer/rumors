//! Restart bookkeeping that limits growth of message versions.
//!
//! Applications provide opaque byte storage through [`Bookmark`]. The crate
//! owns the record format and the rules for recycling a retired peer's
//! identity.
//!
//! A peer's identity is lost when it crashes, and everyone's versions grow
//! until that identity is recycled. The bookmark exists so a restart can resume
//! it. Resuming is dangerous for exactly one reason: emitting a message with a
//! version causally prior to a version the network already knows destroys
//! messages, since peers will infer that they have been redacted. The whole
//! design is therefore three rules that prevent this from occurring, while
//! allowing the prior party identity to be reclaimed when it is safe to do so:
//!
//! - **Store before sharing.** A session stores its own progress, then
//!   transmits from that same frontier. A restart that has caught up with the
//!   stored progress knows every one of its own events which anyone else does.
//! - **Remove before sending.** A donated identity is removed from the record
//!   durably before it can leave the process, because the recipient will own it
//!   from the moment it may have arrived.
//! - **No reclaiming while a fork is reserved.** A fork of the live party which
//!   is speculatively reserved to donate to a joining counterparty may
//!   still exist inside another older recorded identity; reclaiming that
//!   identity would revive the forked identity locally while the newcomer is
//!   about to receive it.
//!
//! The first two rules are structural: the only ways to change the record each
//! return a future which, when awaited, makes the change durable, and the
//! session awaits that future before proceeding to synchronize with a peer. The
//! third is enforced by a count of outstanding speculative forks which, when
//! non-zero, prevents bookmark reclamation.

use std::sync::Arc;

use before::{Party, Version};
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::sync::{Mutex, MutexGuard};

use crate::Network;

mod error;
#[cfg(feature = "fs")]
mod file;
pub(crate) mod format;
mod record;
mod serde;

use record::Record;

pub use error::FormatError;
#[cfg(feature = "fs")]
pub use file::{FileBookmark, RunBlocking, TokioBlocking};
pub use format::BOOKMARK_FORMAT_VERSION;

/// Persistent bookkeeping to limit the growth of message
/// [`Version`]s.
///
/// Every [`Peer`](crate::Peer) created by [`joining`](crate::Peer::join) another
/// obtains a unique donated fragment of the other peer's "identity space",
/// which it then can use to stamp its own messages with [`Version`]s. When that
/// peer finally [`retire`](crate::Peer::retire)s into another peer, this unique
/// identity is *recycled* into the receiving peer, to be reused. If, on the
/// other hand, a peer fails to retire before dropping out of the gossip
/// network, its fragment of identity space is lost and not safely recoverable.
/// Every time such irreversible fragmentation occurs, it increases the required
/// storage for subsequent causal [`Version`]s in the future.
///
/// If you can guarantee that peers will always [`retire`](crate::Peer::retire)
/// at the end of their lifecycle, or you do not care about this kind of
/// potential space leak, you do not need the [`Bookmark`] mechanism. However,
/// if your deployment of `rumors` is both potentially fallible and indefinitely
/// long-lived (the likely situation!), using a [`Bookmark`] can
/// *opportunistically recover identity space* in case of crashes and restarts.
/// By attaching the same persistent [`Bookmark`] to a [`Peer`](crate::Peer),
/// you provide it with the opportunity to remember and recover past fragments
/// of identity, thereby preventing the space leak which would otherwise be
/// caused by any un-graceful shutdowns.
///
/// When a peer starts or restarts, join the gossip network again with the same
/// bookmark selected through
/// [`PeerBuilder::bookmark`](crate::PeerBuilder::bookmark): the peer is now
/// bookmarked; you don't need to do anything else.
///
/// Using a [`Bookmark`] does not allow a peer to persist the contents of the
/// set of messages; it solely recovers the peer's internal identity, which is
/// not directly visible to your application. Peers with and without a bookmark
/// behave the same.
///
/// # Size limits
///
/// A bookmark can retain many identity fragments from any [`Network`]s a peer
/// might have joined. To prevent indefinite growth, bookmarks are size-limited,
/// discarding least-recently-used information. The default size limit is
/// [`default::BOOKMARK_SIZE_LIMIT`](crate::default::BOOKMARK_SIZE_LIMIT), which
/// is configurable with
/// [`PeerBuilder::bookmark_size_limit`](crate::PeerBuilder::bookmark_size_limit).
///
/// # Storage requirements
///
/// Bookmarks help to save significant amounts of storage space when used
/// correctly, but they also introduce hazards if used or implemented
/// incorrectly. When using bookmarks or implementing storage for them:
/// - Always reuse a single, separate bookmark for each peer across every time
///   it restarts.
///
/// - Always replace the bookmark record atomically, preventing torn writes.
/// - Never share bookmarks between concurrently live peers or duplicate them
///   between peers.
/// - Never roll back after successfully storing the bookmark.
///
/// <div class="warning">
///
/// Sharing, duplicating, or rolling back bookmarks can introduce violations of
/// causality which can corrupt the contents of the set of messages across all
/// peers.
///
/// </div>
///
/// You can use `conformance::bookmark` from a dev-dependency with the
/// `conformance` feature to check your own implementation against a variety of
/// simulated scenarios designed to exercise these and other invariants, though
/// you still should use your own backend-specific crash tests to establish
/// durability.
///
/// # Examples
///
/// The `fs` feature provides [`FileBookmark`] as a default way to persist a
/// bookmark to the filesystem. Its constructor takes the application's way to
/// run blocking work, an implementation of [`RunBlocking`]. For
/// [Tokio](https://tokio.rs), use [`TokioBlocking`] here; for other runtimes,
/// implement this trait yourself.
///
/// ```
/// # #[cfg(feature = "fs")]
/// # {
/// use rumors::{FileBookmark, Peer, TokioBlocking};
///
/// # #[expect(dead_code)]
/// # async fn attach() -> Result<(), Box<dyn std::error::Error>> {
/// let bookmark = FileBookmark::new("peer.bookmark", TokioBlocking).await?;
/// let peer = Peer::<String>::builder().bookmark(bookmark).seed().await?;
/// # let _ = peer;
/// # Ok(())
/// # }
/// # }
/// ```
pub trait Bookmark {
    /// The error when failing to open or replace the stored bytes.
    type Error: std::error::Error + Send + Sync + 'static;

    /// The byte source returned by [`load`](Self::load).
    type Reader: AsyncRead + Unpin + Send;

    /// Open the stored bookmark for reading, or `Ok(None)` if the bookmark is
    /// not stored yet.
    ///
    /// This is called before the first update to the bookmark, and again if an
    /// update fails or loading is cancelled. Each call must return the current
    /// bookmark's contents.
    fn load(&self) -> impl Future<Output = Result<Option<Self::Reader>, Self::Error>> + Send;

    /// Atomically replace the stored record with these bytes.
    ///
    /// Treat the supplied bytes as opaque; their format and validation are
    /// handled internally to `rumors`.
    ///
    /// Return `Ok(())` *only once the complete replacement is durable and
    /// irrevocable*, i.e. after an atomic write followed by a filesystem sync.
    /// An error or cancellation is permitted to leave either the previous or
    /// replacement record, but neither may be partial.
    ///
    /// After this method returns `Ok(())`, never roll back the state of the
    /// bookmark to any prior state.
    fn store(&self, bytes: Vec<u8>) -> impl Future<Output = Result<(), Self::Error>> + Send;
}

/// A failure reading or writing a [`Bookmark`].
#[derive(Debug, thiserror::Error)]
pub enum BookmarkIo<E> {
    /// The underlying storage could not open or replace the record.
    #[error(transparent)]
    Io(E),

    /// Reading the bookmark failed, or its format was invalid.
    #[error(transparent)]
    Format(#[from] FormatError),
}

/// The default [`Bookmark`] which persists nothing.
///
/// This is replaced by another type of bookmark persistence upon calling
/// [`PeerBuilder::bookmark`](crate::PeerBuilder::bookmark).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NoBookmark;

/// The storage a peer without a bookmark holds.
mod null {
    use super::Bookmark;

    /// Storage that persists nothing, held by every peer without a bookmark.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
    pub struct NullBookmark;

    ///  Peers that do not use a bookmark get no persistence.
    impl Bookmark for NullBookmark {
        /// This implementation never does anything that could fail.
        type Error = std::convert::Infallible;

        /// No stored bookmark is ever returned.
        type Reader = tokio::io::Empty;

        /// Report that no bookmark is stored.
        async fn load(&self) -> Result<Option<Self::Reader>, Self::Error> {
            Ok(None)
        }

        /// Discard the bookmark when persistence is disabled.
        async fn store(&self, _bytes: Vec<u8>) -> Result<(), Self::Error> {
            Ok(())
        }
    }
}

pub(crate) use null::NullBookmark;

/// Seal [`MaybeBookmark`] to [`NoBookmark`] and the [`Bookmark`]s.
mod sealed {
    /// The bookmark choices a peer can be built with.
    pub trait Sealed {}

    /// No bookmark.
    impl Sealed for super::NoBookmark {}

    /// A bookmark.
    impl<B: super::Bookmark> Sealed for B {}

    /// Permission to call [`MaybeBookmark`](super::MaybeBookmark)'s hidden
    /// methods, which only this crate can construct.
    #[derive(Debug)]
    pub struct Token(pub(crate) ());
}

pub(crate) use sealed::Token;

/// Marker trait for whether a peer has a [`Bookmark`].
///
/// This is [`NoBookmark`] if not, or the selected [`Bookmark`] type.
pub trait MaybeBookmark: sealed::Sealed + Sized {
    /// The storage which backs a bookmark of this type.
    #[doc(hidden)]
    type Storage: Bookmark;

    /// The payload of [`Join::Unbookmarked`](crate::Join::Unbookmarked) for
    /// a bookmark of this type.
    type Unbookmarked<T: Send + Sync + 'static>: std::fmt::Debug;

    /// The payload of [`Error::Bookmark`](crate::Error::Bookmark) for this
    /// choice.
    type Failure: std::error::Error + Send + Sync + 'static;

    /// The storage this choice holds.
    #[doc(hidden)]
    fn into_storage(self, token: Token) -> Self::Storage;

    /// The choice whose storage is `storage`.
    #[doc(hidden)]
    fn from_storage(storage: Self::Storage, token: Token) -> Self;

    /// Report a failure of this choice's storage as an [`Error`](crate::Error).
    #[doc(hidden)]
    fn failure(error: StorageIo<Self>, token: Token) -> Self::Failure;

    /// Report a failure to attach this choice's storage to `peer`.
    #[doc(hidden)]
    fn unbookmarked<T: Send + Sync + 'static>(
        peer: crate::peer::Replica<T, NoBookmark>,
        error: StorageIo<Self>,
        token: Token,
    ) -> Self::Unbookmarked<T>;
}

/// A failure of the storage that `M` holds.
pub(crate) type StorageIo<M> = BookmarkIo<<<M as MaybeBookmark>::Storage as Bookmark>::Error>;

/// No bookmark: the peer holds storage that persists nothing, so nothing about
/// it can fail.
impl MaybeBookmark for NoBookmark {
    /// Storage that persists nothing.
    type Storage = NullBookmark;
    /// Uninhabited: attaching no bookmark cannot fail.
    type Unbookmarked<T: Send + Sync + 'static> = std::convert::Infallible;
    /// Uninhabited: storage that persists nothing cannot fail.
    type Failure = std::convert::Infallible;

    /// Storage that persists nothing.
    fn into_storage(self, _: Token) -> NullBookmark {
        NullBookmark
    }

    /// No bookmark.
    fn from_storage(_: NullBookmark, _: Token) -> Self {
        NoBookmark
    }

    /// Storage that persists nothing never fails.
    fn failure(error: StorageIo<Self>, _: Token) -> std::convert::Infallible {
        null_failure(error)
    }

    /// Storage that persists nothing never fails.
    fn unbookmarked<T: Send + Sync + 'static>(
        _: crate::peer::Replica<T, NoBookmark>,
        error: StorageIo<Self>,
        _: Token,
    ) -> std::convert::Infallible {
        null_failure(error)
    }
}

/// Discharge a failure of storage that persists nothing.
///
/// Its storage error is uninhabited, and it never returns a stored record, so
/// it never decodes one and never reports a format error.
fn null_failure(error: BookmarkIo<std::convert::Infallible>) -> std::convert::Infallible {
    match error {
        BookmarkIo::Io(never) => match never {},
        BookmarkIo::Format(error) => {
            unreachable!("storage that persists nothing never returns a record to decode: {error}")
        }
    }
}

/// Any bookmark is [`MaybeBookmark`], because it is one.
impl<B: Bookmark> MaybeBookmark for B {
    /// The bookmark itself.
    type Storage = B;

    /// The peer without its bookmark, paired with the failure.
    type Unbookmarked<T: Send + Sync + 'static> = crate::Unbookmarked<T, B>;

    /// The failure to load, persist, or decode the stored record.
    type Failure = BookmarkIo<B::Error>;

    /// The bookmark itself.
    fn into_storage(self, _: Token) -> B {
        self
    }

    /// The bookmark itself.
    fn from_storage(storage: B, _: Token) -> Self {
        storage
    }

    /// The storage failure itself.
    fn failure(error: BookmarkIo<B::Error>, _: Token) -> BookmarkIo<B::Error> {
        error
    }

    /// The peer without its bookmark, paired with the failure.
    fn unbookmarked<T: Send + Sync + 'static>(
        peer: crate::peer::Replica<T, NoBookmark>,
        error: BookmarkIo<B::Error>,
        _: Token,
    ) -> crate::Unbookmarked<T, B> {
        crate::Unbookmarked::new(peer, error)
    }
}

/// The donation forks reserved for joining peers and not yet sent, which pause
/// reclamation.
///
/// We can reclaim old identities during a bookmark checkpoint only while no
/// [`Reservation`] is outstanding. That is race-free because `reserve` needs
/// `&mut self`, which only a holder of the replica's write lock has, and a
/// checkpoint runs under that same lock: no fork can be reserved during a
/// checkpoint. Releasing needs no lock because it only lowers the count, and it
/// happens only after the speculative fork rejoins the live party or is
/// permanently gone from the record. A release that races a checkpoint just
/// defers reclamation to the next one.
///
/// A session future leaked instead of dropped never releases its reservation,
/// which would pause reclamation for the replica's lifetime. Reclaiming the
/// peer from its handles forgets such reservations
/// ([`release_abandoned`](Self::release_abandoned)).
#[derive(Default)]
pub(crate) struct Reservations(Arc<()>);

/// One speculatively reserved fork's hold on reclamation, released when it is
/// dropped.
///
/// Drop it once the fork has rejoined the live party, or once its removal from
/// the record is durable; either way the record can no longer offer it.
pub(crate) struct Reservation {
    /// A clone of the replica's counter, so that the count of strong references
    /// is one more than the number of outstanding reservations.
    _hold: Arc<()>,
}

/// Count the speculative forks that are outstanding.
impl Reservations {
    /// Reserve a fork, pausing reclamation until the returned reservation
    /// drops.
    pub(crate) fn reserve(&mut self) -> Reservation {
        Reservation {
            _hold: Arc::clone(&self.0),
        }
    }

    /// Forget every outstanding reservation, so that reclamation resumes.
    ///
    /// Call this only where no session future that reserved a fork can run or
    /// be dropped again, such as on a peer just reclaimed from its handles. A
    /// reservation still outstanding there belongs to a leaked future, and
    /// forgetting it cannot hand a party out twice. That future's fork was
    /// never sent, because a session releases its reservation before it writes
    /// the first byte of the fork, and the future can never send the fork or
    /// rejoin it now. So no live peer holds that region or ever will, and a
    /// checkpoint that reclaims it from an identity still recording it gives it
    /// to the live party alone.
    pub(crate) fn release_abandoned(&mut self) {
        *self = Self::default();
    }

    /// Whether no fork is currently reserved, so that reclamation may proceed.
    fn none_outstanding(&self) -> bool {
        Arc::strong_count(&self.0) == 1
    }
}

/// One peer view's hold on its bookmark: the storage and record it shares with
/// every [`Peer`](crate::Peer) clone, behind the session lock, and the size
/// limit to store under, which is read without that lock.
///
/// The limit is a setting of the peer, like its memory budget: only an
/// exclusively owned [`Peer`](crate::Peer) can change it, and every view shared
/// from that peer copies it. Keeping it outside the lock means that configuring
/// the limit, or replacing the storage, never waits on the lock, so neither
/// depends on whether some session future still holds it.
pub(crate) struct BookmarkHandle<B> {
    /// Maximum encoded record size, including its frame: at least the size of
    /// an empty record.
    size_limit: usize,
    /// The storage and its cached record, locked from a session's load through
    /// its store.
    state: Arc<Mutex<Bookmarked<B>>>,
}

/// Construct, share, configure, and lock a peer's bookmark.
impl<B> BookmarkHandle<B> {
    /// Hold `persist`, without reading it until the first update.
    ///
    /// Select the store limit, raised to the size of an empty record.
    ///
    /// The limit takes effect at the next store, so until then the stored
    /// record may exceed it.
    pub(crate) fn new(persist: B, size_limit: usize) -> Self {
        Self {
            size_limit: size_limit.max(format::record_size(0, 0)),
            state: Arc::new(Mutex::new(Bookmarked::new(persist))),
        }
    }

    /// Share the storage and record with another view of the same peer.
    pub(crate) fn share(&self) -> Self {
        Self {
            size_limit: self.size_limit,
            state: Arc::clone(&self.state),
        }
    }

    /// Select the store limit, raised to the size of an empty record.
    ///
    /// The limit takes effect at the next store, so until then the stored
    /// record may exceed it. A checkpoint stored under another limit is not
    /// current (see [`Checkpoint::size_limit`]), so the next session stores
    /// under this one instead of skipping its store.
    pub(crate) fn set_size_limit(&mut self, bytes: usize) {
        self.size_limit = bytes.max(format::record_size(0, 0));
    }

    /// The configured limit.
    pub(crate) fn size_limit(&self) -> usize {
        self.size_limit
    }

    /// Release the lock if a session future that can never run again still
    /// holds it.
    ///
    /// A session future leaked (with `mem::forget`, say) instead of dropped
    /// never runs its lock's destructor, so the lock stays held and every later
    /// [`lock`](Self::lock) waits forever. While other views exist, a held lock
    /// cannot be told apart from a slow store. So call this only where no live
    /// future can hold the lock, such as on a peer just reclaimed from its
    /// handles, where this view is also the only one. A rebuild needs that:
    /// only the sole owner of the lock can take the record out of it. If
    /// another view remains, this leaves the lock as it is.
    ///
    /// A fresh lock around the same record leaves what a cancellation at the
    /// same await would leave: every change to the record happens between
    /// awaits, and a store takes the cache out of memory as it begins (see
    /// [`Loaded::store`]), so after a leaked store the next load rereads
    /// storage, which [`Bookmark::store`] keeps complete.
    pub(crate) fn release_abandoned_lock(self) -> Self {
        if self.state.try_lock().is_ok() {
            return self;
        }
        let Self { size_limit, state } = self;
        let state = match Arc::try_unwrap(state) {
            Ok(held) => Arc::new(Mutex::new(held.into_inner())),
            Err(shared) => shared,
        };
        Self { size_limit, state }
    }

    /// Return the storage, discarding any record loaded from it.
    ///
    /// # Panics
    ///
    /// Panics if the handle has been shared, which only the peer it belongs to
    /// does.
    pub(crate) fn into_storage(self) -> B {
        let state = Arc::try_unwrap(self.state)
            .ok()
            .expect("a handle is unshared until a peer holds it");
        state.into_inner().persist
    }

    /// Wait for exclusive use of the storage and record.
    pub(crate) async fn lock(&self) -> LockedBookmark<'_, B> {
        LockedBookmark {
            state: self.state.lock().await,
            size_limit: self.size_limit,
        }
    }
}

/// Exclusive use of a peer's bookmark, from loading the record through storing
/// it.
pub(crate) struct LockedBookmark<'a, B> {
    /// The storage and record, held until this lock drops.
    state: MutexGuard<'a, Bookmarked<B>>,
    /// The limit that every store made through this lock encodes under.
    size_limit: usize,
}

/// Load the record through the lock.
impl<B: Bookmark> LockedBookmark<'_, B> {
    /// Read storage if the cache is absent, then lend the cache for one update
    /// stored under this peer's size limit.
    pub(crate) async fn load(&mut self) -> Result<Loaded<'_, B>, BookmarkIo<B::Error>> {
        self.state.load(self.size_limit).await
    }
}

/// Cached restart bookkeeping, protected by the peer's bookmark mutex.
///
/// The mutex spans loading, changing, and storing the record. A change takes
/// the replica lock briefly inside it; storage never holds that lock. This
/// keeps the live identity and the record consistent without blocking local
/// edits on storage I/O.
pub(crate) struct Bookmarked<B> {
    /// Application-owned storage for the encoded record.
    persist: B,
    /// The record in memory: absent before the first load, and again after any
    /// store that did not confirm success.
    cache: Option<Cache>,
}

/// The record in memory, and what the last store checkpointed.
struct Cache {
    /// Identities retained across restarts, grouped by network.
    record: Record,
    /// The live state the last store checkpointed, if it was a checkpoint.
    durable: Option<Checkpoint>,
}

/// A live party and its frontier at a checkpoint, and the limit the
/// checkpoint's record was encoded under.
struct Checkpoint {
    /// The exact live party.
    party: Party,
    /// The causal frontier across all parties.
    ///
    /// Only an individual party's own region is ever compared.
    frontier: Version,
    /// The size limit the checkpoint's record was encoded under.
    ///
    /// Encoding the same record under another limit can keep a different set of
    /// identities, so a checkpoint is current only under the limit it was
    /// stored under.
    size_limit: usize,
}

/// Construct an unloaded bookmark cache.
impl<B> Bookmarked<B> {
    /// Keep the storage without reading it until the first update.
    pub(crate) fn new(persist: B) -> Self {
        Self {
            persist,
            cache: None,
        }
    }
}

/// Load the record on first use.
impl<B: Bookmark> Bookmarked<B> {
    /// Read storage if the cache is absent, then lend the cache for one update
    /// stored under `size_limit`.
    pub(crate) async fn load(
        &mut self,
        size_limit: usize,
    ) -> Result<Loaded<'_, B>, BookmarkIo<B::Error>> {
        if self.cache.is_none() {
            let record = match self.persist.load().await.map_err(BookmarkIo::Io)? {
                None => Record::default(),
                Some(mut reader) => {
                    let mut bytes = Vec::new();
                    reader
                        .read_to_end(&mut bytes)
                        .await
                        .map_err(|error| BookmarkIo::Format(FormatError::Read(error)))?;
                    format::decode(&bytes)?
                }
            };
            self.cache = Some(Cache {
                record,
                durable: None,
            });
        }
        Ok(Loaded {
            bookmark: self,
            size_limit,
        })
    }
}

/// The cache on loan from loading a bookmark until an update stores it or
/// invalidates it by dropping it.
///
/// The only ways to change the record consume the loan and return a future
/// which will make the change durable. Dropping the loan without updating
/// anything changes nothing.
pub(crate) struct Loaded<'a, B> {
    /// The bookmark whose cache is present for the lifetime of the loan.
    bookmark: &'a mut Bookmarked<B>,
    /// The limit the update's store encodes under.
    size_limit: usize,
}

/// The three ways to change the record, each returning the future that stores
/// it.
impl<'a, B: Bookmark> Loaded<'a, B> {
    /// Store before sharing: checkpoint the live party at the frontier a
    /// session is about to share, reclaiming whatever that frontier has caught
    /// up with unless a fork is reserved.
    ///
    /// Returns the future that stores the record, which the session awaits
    /// before transmitting, or `None` when the last store already checkpointed
    /// this exact live state, so that the stored progress already covers
    /// everything this session will share.
    #[must_use = "make bookmark changes durable before the session transmits"]
    pub(crate) fn checkpoint(
        mut self,
        network: Network,
        party: &mut Party,
        frontier: &Version,
        reservations: &Reservations,
    ) -> Option<impl Future<Output = Result<(), BookmarkIo<B::Error>>> + use<'a, B>> {
        if self.checkpointed(party, frontier) {
            return None;
        }
        let record = self.cache_mut().record.network(network);
        if reservations.none_outstanding() {
            record.reclaim(party, frontier);
        }
        record.record(party, frontier);
        let size_limit = self.size_limit;
        Some(self.store(Some(Checkpoint {
            party: party.dangerously_alias(),
            frontier: frontier.clone(),
            size_limit,
        })))
    }

    /// Remove before sending: take a donation out of every identity, and return
    /// the future that stores the record, which the session awaits before the
    /// donation leaves the process.
    #[must_use = "make bookmark changes durable before the session transmits"]
    pub(crate) fn donate(
        mut self,
        network: Network,
        donation: &Party,
    ) -> impl Future<Output = Result<(), BookmarkIo<B::Error>>> + use<'a, B> {
        self.cache_mut().record.donate(network, donation);
        self.store(None)
    }

    /// Record the live party when storage is attached, and return the future
    /// that stores the record.
    ///
    /// Attachment reclaims nothing, because a failed attachment hands the peer
    /// back without a bookmark, and a region reclaimed then would write without
    /// ever being recorded. So attachment is not a checkpoint, and the first
    /// session still reclaims.
    #[must_use = "make bookmark changes durable before the session transmits"]
    pub(crate) fn attach(
        mut self,
        network: Network,
        party: &Party,
        frontier: &Version,
    ) -> impl Future<Output = Result<(), BookmarkIo<B::Error>>> + use<'a, B> {
        self.cache_mut()
            .record
            .network(network)
            .record(party, frontier);
        self.store(None)
    }

    /// Whether the last store checkpointed exactly this live state, under this
    /// update's size limit.
    fn checkpointed(&self, party: &Party, frontier: &Version) -> bool {
        self.cache().durable.as_ref().is_some_and(|checkpoint| {
            checkpoint.size_limit == self.size_limit
                && checkpoint.party == *party
                && &checkpoint.frontier / party == frontier / party
        })
    }

    /// Store the record; on success the cache returns with `durable` set.
    ///
    /// The cache leaves memory as the future is created and comes back only on
    /// success, so that after an error or a cancellation the next load reads
    /// what storage actually kept instead of trusting memory about it.
    #[must_use = "make bookmark changes durable before the session transmits"]
    fn store(
        self,
        durable: Option<Checkpoint>,
    ) -> impl Future<Output = Result<(), BookmarkIo<B::Error>>> + use<'a, B> {
        let Self {
            bookmark,
            size_limit,
        } = self;
        let mut cache = bookmark
            .cache
            .take()
            .expect("a loan exists only while the cache is present");
        let bytes = cache.record.encode_bounded(size_limit);
        async move {
            bookmark
                .persist
                .store(bytes)
                .await
                .map_err(BookmarkIo::Io)?;
            cache.durable = durable;
            bookmark.cache = Some(cache);
            Ok(())
        }
    }

    /// The lent cache.
    fn cache(&self) -> &Cache {
        self.bookmark
            .cache
            .as_ref()
            .expect("a loan exists only while the cache is present")
    }

    /// The lent cache, for changing.
    fn cache_mut(&mut self) -> &mut Cache {
        self.bookmark
            .cache
            .as_mut()
            .expect("a loan exists only while the cache is present")
    }
}

#[cfg(test)]
mod tests;
