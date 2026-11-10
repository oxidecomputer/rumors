//! Restart bookkeeping that limits growth of message versions.
//!
//! Applications provide opaque byte storage through [`Bookmark`]. The crate
//! owns the record format and the rules for recycling a departed peer's
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
//!   is speculatively reserved to donate to a bootstrapping counterparty may
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

use crate::{Network, default};

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

/// Persistent restart bookkeeping that limits growth of message versions.
///
/// When peers disappear without retiring, their internal bookkeeping can make
/// [`Version`]s larger. Reusing a bookmark across restarts lets Rumors recover
/// and recycle that bookkeeping as it catches up with the network. This is an
/// optimization: peers can join and gossip without bookmarks.
///
/// The application stores opaque bytes. Rumors owns their format; implement
/// [`load`](Self::load) and [`store`](Self::store) to read and atomically
/// replace them. A bookmark stores no messages. Recover those by joining and
/// gossiping.
///
/// # Restarting a peer
///
/// Join again with the same bookmark selected through
/// [`Bootstrap::bookmark`](crate::Bootstrap::bookmark), or attach it through
/// [`Peer::bookmark`](crate::Peer::bookmark) immediately after joining. Rumors
/// handles recovery automatically during subsequent gossip.
///
/// Each peer has an internal *identity* used to generate distinct message
/// versions. The bookmark records that identity and the progress of its own
/// writes. Rumors can reclaim a prior incarnation's identity once it has caught
/// up with those writes; it need not recover every message that the prior
/// incarnation had merely observed.
///
/// Reclamation happens when Rumors updates the bookmark, and ordinary gossip
/// updates it only to record local progress before exchanging locally
/// originated messages, not merely when it learned remote changes. An identity
/// that becomes reclaimable during an exchange therefore waits for the session
/// after the next local send, redaction, or identity change. Reclamation also
/// waits while bootstrap sessions hold identities for transfer. These delays
/// postpone the potential to reduce the representational size of transmitted
/// versions; they do not delay checkpointing local writes.
///
/// # Bounded retention
///
/// A bookmark can retain identities from several [`Network`]s, so rejoining a
/// network can recover its earlier bookkeeping. The default size limit is 16
/// MiB; configure
/// [`Peer::bookmark_size_limit`](crate::Peer::bookmark_size_limit) or
/// [`Bootstrap::bookmark_size_limit`](crate::Bootstrap::bookmark_size_limit).
/// Rumors discards the oldest identities from the least recently used network,
/// one at a time until the record fits. It removes a network only when no
/// identities remain there. Both recency orders survive restarts.
///
/// Discarding recovery rights is safe: it can increase future version sizes,
/// but cannot erase replicated messages or permit version reuse. The limit
/// includes the encoded identities, their write progress, and the integrity
/// frame. It bounds stored bytes rather than peak memory used to load or encode
/// them.
///
/// # Storage obligations
///
/// - Use one bookmark for one peer across its restarts. Never share it between
///   concurrently live peers or duplicate it to start another peer.
///   [`FileBookmark`] refuses to open a bookmark that another live
///   `FileBookmark` on the same machine holds.
/// - Replace the record atomically, and never roll back a successful store. A
///   stale but valid record can cause versions to be reused and corrupt the
///   set.
/// - Preserve the complete record across restarts; Rumors handles retention.
///
/// Use `conformance::bookmark` from a dev-dependency with the `conformance`
/// feature to check an implementation's loads, replacements, and interruptions.
/// Backend-specific crash tests must still establish durability.
///
/// # Examples
///
/// The `fs` feature provides [`FileBookmark`]. Its constructor takes the
/// application's way to run blocking work, a [`RunBlocking`] runner; a Tokio
/// application passes [`TokioBlocking`]. It locks the bookmark, and fails if
/// another live peer holds it:
///
/// ```
/// # #[cfg(feature = "fs")]
/// # {
/// use rumors::{FileBookmark, Peer, TokioBlocking};
///
/// # async fn attach(peer: Peer<String>) -> Result<(), std::fs::TryLockError> {
/// let bookmark = FileBookmark::new("peer.bookmark", TokioBlocking).await?;
/// let peer = peer.bookmark(bookmark).await;
/// # let _ = peer;
/// # Ok(())
/// # }
/// # }
/// ```
///
/// A slow store delays synchronization or completion when accepting a
/// retirement. A session may already have exchanged its connection preamble
/// while waiting for storage. Local sends continue; storage errors are reported
/// through [`BookmarkIo`].
///
/// # Limits of recovery
///
/// A crash before a new peer's bookmark is persisted can still lose the
/// opportunity to recycle its identity. This affects version size, not messages
/// already replicated elsewhere. Retirement and bookmarks reduce version
/// growth; they do not guarantee that versions stay a fixed size. Spread
/// bootstrap requests across established peers: long chains of joins or
/// repeated joins through a single provider also increase version size.
///
/// If a session fails after it writes its bookmark and the peer crashes before
/// another session shares those writes, no live peer holds what the record
/// requires to reclaim the crashed party's identity, so that identity can never
/// be safely caught up. It stays recorded and unused until the bookmark size
/// limit evicts it.
///
/// Retirement removes the departing identity from the local bookmark before
/// sending it. If the transfer then fails or is cancelled, that bookmark cannot
/// recover the removed identity. Recovery depends on what reached the recipient
/// and its bookmark; see [`Retire::Retired`](crate::Retire::Retired). Serving a
/// bootstrap removes the donated identity the same way. If that session fails
/// before sending, the identity returns to the live peer and is recorded again
/// at its next checkpoint; a crash before this occurs would drop it forever.
pub trait Bookmark {
    /// Failure to open or replace the stored bytes.
    type Error: std::error::Error + Send + Sync + 'static;

    /// The byte source [`load`](Self::load) hands back.
    type Reader: AsyncRead + Unpin + Send;

    /// Open the stored record for reading, or `Ok(None)` if nothing is stored.
    ///
    /// Called before the first update, and again if an update fails or loading
    /// is cancelled. Each call must return the current record; loading must not
    /// consume it. `Ok(None)` means no record exists. Return present bytes even
    /// if they are short or malformed: Rumors validates their format.
    fn load(&self) -> impl Future<Output = Result<Option<Self::Reader>, Self::Error>> + Send;

    /// Atomically replace the stored record with these owned, encoded bytes.
    ///
    /// Treat the bytes as opaque; Rumors handles their format and validation.
    ///
    /// Return `Ok(())` only once the complete replacement is durable, i.e.
    /// after an atomic write followed by a filesystem sync. An error or
    /// cancellation may leave either the previous or replacement record;
    /// neither may be partial.
    ///
    /// Never roll back the durable state of a bookmark to before a successful
    /// store. Work that continues after an error or cancellation must not
    /// overwrite a later successful store: stale records can permit version
    /// reuse and corrupt the gossip set. A future leaked instead of dropped
    /// counts as cancelled: once the application reclaims its peer
    /// ([`Rumors::try_into_peer`](crate::Rumors::try_into_peer)), later stores
    /// proceed while any work the leaked store started may still be running.
    fn store(&self, bytes: Vec<u8>) -> impl Future<Output = Result<(), Self::Error>> + Send;
}

/// A storage failure or an unreadable bookmark record.
#[derive(Debug, thiserror::Error)]
pub enum BookmarkIo<E> {
    /// The storage implementation could not open or replace the record.
    #[error(transparent)]
    Io(E),

    /// Reading the returned stream failed, or its bytes failed validation.
    #[error(transparent)]
    Format(#[from] FormatError),
}

/// The placeholder [`Bookmark`] that persists nothing.
///
/// The default for every [`Peer`](crate::Peer). Ordinary gossip needs no
/// bookmark, but repeated crashes can grow message versions without one.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NoBookmark;

/// Disable persistence for peers that do not use a bookmark.
impl Bookmark for NoBookmark {
    /// This implementation performs no fallible operation.
    type Error = std::convert::Infallible;
    /// No stored record is returned.
    type Reader = tokio::io::Empty;

    /// Report that no record is stored.
    async fn load(&self) -> Result<Option<Self::Reader>, Self::Error> {
        Ok(None)
    }

    /// Discard the record when persistence is disabled.
    async fn store(&self, _bytes: Vec<u8>) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// The forks reserved for bootstraps and not yet sent, which pause reclamation.
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
/// every [`Rumors`](crate::Rumors) clone, behind the session lock, and the size
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
    /// Hold `persist` at the default size limit, without reading it until the
    /// first update.
    pub(crate) fn new(persist: B) -> Self {
        Self {
            size_limit: default::BOOKMARK_SIZE_LIMIT,
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

    /// Hold `persist` instead, with a fresh record and lock, carrying the size
    /// limit forward.
    pub(crate) fn replace<N>(&self, persist: N) -> BookmarkHandle<N> {
        BookmarkHandle {
            size_limit: self.size_limit,
            state: Arc::new(Mutex::new(Bookmarked::new(persist))),
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

    /// The configured limit, for tests that check where it reached.
    #[cfg(test)]
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
