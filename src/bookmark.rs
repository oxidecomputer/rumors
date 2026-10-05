//! Restart bookkeeping that limits growth of message versions.
//!
//! Applications provide opaque byte storage through [`Bookmark`]. The crate
//! owns the record format and the rules for recycling a departed peer's identity.
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

use crate::Network;

mod error;
#[cfg(feature = "fs")]
mod file;
pub(crate) mod format;
mod record;
mod serde;

use record::Record;

/// Default maximum encoded bookmark size: 16 MiB, allocated only as needed.
pub const DEFAULT_BOOKMARK_SIZE_LIMIT: usize = 16 * 1024 * 1024;

pub use error::FormatError;
#[cfg(feature = "fs")]
pub use file::FileBookmark;
pub use format::BOOKMARK_FORMAT_VERSION;

/// Persistent restart bookkeeping that limits growth of message versions.
///
/// When peers disappear without retiring, their internal bookkeeping can make
/// [`Version`]s larger. Reusing a bookmark across restarts lets
/// Rumors recover and recycle that bookkeeping as it catches up with the network.
/// This is an optimization: peers can join and gossip without bookmarks.
///
/// The application stores opaque bytes. Rumors owns their format; implement
/// [`load`](Self::load) and [`store`](Self::store) to read and atomically replace
/// them. A bookmark stores no messages. Recover those by joining and gossiping.
///
/// # Restarting a peer
///
/// Join again with the same bookmark selected through
/// [`Bootstrap::bookmark`](crate::Bootstrap::bookmark), or attach it through
/// [`Peer::bookmark`](crate::Peer::bookmark) immediately after joining.
/// Rumors handles recovery automatically during subsequent gossip.
///
/// Each peer has an internal *identity* used to generate distinct message
/// versions. The bookmark records that identity and the progress of its own
/// writes. Rumors can reclaim a prior incarnation's identity once it has
/// caught up with those writes; it need not recover every message that the
/// prior incarnation had merely observed.
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
/// - Replace the record atomically, and never roll back a successful store. A
///   stale but valid record can cause versions to be reused and corrupt the set.
/// - Preserve the complete record across restarts; Rumors handles retention.
///
/// Use `conformance::bookmark` from a dev-dependency with the `conformance`
/// feature to check an implementation's loads, replacements, and interruptions.
/// Backend-specific crash tests must still establish durability.
///
/// # Examples
///
/// The `fs` feature provides [`FileBookmark`]. Its constructor accepts the
/// application's way to run blocking work; this Tokio example supplies
/// `spawn_blocking`; Rumors requires this to be supplied because it does not
/// itself depend on any async runtime.
///
/// ```
/// # #[cfg(feature = "fs")]
/// # {
/// use std::io;
///
/// use rumors::{FileBookmark, Peer};
///
/// # async fn attach(peer: Peer<String>) {
/// let bookmark = FileBookmark::new("peer.bookmark", |job| async move {
///     tokio::task::spawn_blocking(job)
///         .await
///         .map_err(io::Error::other)
/// });
/// let peer = peer.bookmark(bookmark).await;
/// # let _ = peer;
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
/// already replicated elsewhere. Retirement and bookmarks reduce version growth;
/// they do not guarantee that versions stay a fixed size. Spread bootstrap
/// requests across established peers: long chains of joins or repeated joins
/// through a single provider also increase version size.
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
/// and its bookmark; see [`Retire::Uncertain`](crate::Retire::Uncertain).
/// Serving a bootstrap removes the donated identity the same way. If that
/// session fails before sending, the identity returns to the live peer and is
/// recorded again at its next checkpoint; a crash before this occurs would drop
/// it forever.
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
    /// reuse and corrupt the gossip set.
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
    /// Reserve a fork, pausing reclamation until the returned reservation drops.
    pub(crate) fn reserve(&mut self) -> Reservation {
        Reservation {
            _hold: Arc::clone(&self.0),
        }
    }

    /// Whether no fork is currently reserved, so that reclamation may proceed.
    fn none_outstanding(&self) -> bool {
        Arc::strong_count(&self.0) == 1
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
    /// The record in memory: absent before the first load, and again after
    /// any store that did not confirm success.
    cache: Option<Cache>,
    /// Maximum encoded record size, including its frame.
    size_limit: usize,
}

/// The record in memory, and what the last store checkpointed.
struct Cache {
    /// Identities retained across restarts, grouped by network.
    record: Record,
    /// The live state the last store checkpointed, if it was a checkpoint.
    durable: Option<Checkpoint>,
}

/// A live party and its frontier at a checkpoint.
struct Checkpoint {
    /// The exact live party.
    party: Party,
    /// The causal frontier across all parties.
    ///
    /// Only an individual party's own region is ever compared.
    frontier: Version,
}

/// Construct an unloaded bookmark cache.
impl<B> Bookmarked<B> {
    /// Keep the storage without reading it until the first update.
    pub(crate) fn new(persist: B) -> Self {
        Self {
            persist,
            cache: None,
            size_limit: DEFAULT_BOOKMARK_SIZE_LIMIT,
        }
    }

    /// Select the store limit.
    ///
    /// The limit takes effect at the next store, so until then the stored
    /// record may exceed it. We forget the last checkpoint so that the next
    /// session stores under the new limit instead of skipping its store.
    pub(crate) fn set_size_limit(&mut self, bytes: usize) {
        self.size_limit = bytes.max(format::record_size(0, 0));
        if let Some(cache) = &mut self.cache {
            cache.durable = None;
        }
    }

    /// The configured limit, also retained before storage is attached.
    pub(crate) fn size_limit(&self) -> usize {
        self.size_limit
    }
}

/// Load the record on first use.
impl<B: Bookmark> Bookmarked<B> {
    /// Read storage if the cache is absent, then lend the cache for one update.
    pub(crate) async fn load(&mut self) -> Result<Loaded<'_, B>, BookmarkIo<B::Error>> {
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
        Ok(Loaded { bookmark: self })
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
        Some(self.store(Some(Checkpoint {
            party: party.dangerously_alias(),
            frontier: frontier.clone(),
        })))
    }

    /// Remove before sending: take a donation out of every identity, and
    /// return the future that stores the record, which the session awaits
    /// before the donation leaves the process.
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
    /// back without a bookmark, and a region reclaimed then would write
    /// without ever being recorded. So attachment is not a checkpoint, and the
    /// first session still reclaims.
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

    /// Whether the last store checkpointed exactly this live state.
    fn checkpointed(&self, party: &Party, frontier: &Version) -> bool {
        self.cache().durable.as_ref().is_some_and(|checkpoint| {
            checkpoint.party == *party && &checkpoint.frontier / party == frontier / party
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
        let bookmark = self.bookmark;
        let mut cache = bookmark
            .cache
            .take()
            .expect("a loan exists only while the cache is present");
        let bytes = cache.record.encode_bounded(bookmark.size_limit);
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
