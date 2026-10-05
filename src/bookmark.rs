//! Restart bookkeeping that limits growth of message versions.
//!
//! Applications provide opaque byte storage through [`Bookmark`]. The crate
//! owns the record format and the rules for recycling a departed peer's identity.
//!
//! The invariant everything serves, and the two conditions under which an
//! identity may be reclaimed, are stated in `record`. That module checks the
//! catch-up condition; the peer's reservation gate and session driver keep
//! reclaimed regions exclusive, by withholding reclamation while a fork is
//! reserved and by removing donations durably before sending them. This
//! module loads and stores the record, and treats only a confirmed store as
//! the checkpoint a session may rely on.

use std::ops::{Deref, DerefMut};

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
/// updates it only to record local progress before exchanging messages, never
/// because it learned remote changes. An identity that becomes reclaimable
/// during an exchange therefore waits for the session after the next local
/// send, redaction, or identity change. That delay costs one event at most:
/// reclaiming alters no version until the peer's next own event, so the change
/// that ends the wait still uses the old identity and the one after it uses
/// the reclaimed one. Reclamation also waits while bootstrap sessions hold
/// identities for transfer. These delays postpone the reduction in version
/// size; they do not delay checkpointing local writes.
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
/// Checkpoints precede transmission, and a checkpoint can outlive the writes
/// it recorded. If a session fails after its checkpoint and the peer crashes
/// before another session shares those writes, no live peer holds what the
/// record requires, so that identity can never be caught up. It stays recorded
/// and unused until the size limit evicts it. The same holds for an identity
/// absorbed from a retiring peer whose writes the absorber had not yet shared.
/// Neither case reuses a version; both cost version size.
///
/// Retirement removes the departing identity from the local bookmark before
/// sending it. If the transfer then fails or is cancelled, that bookmark cannot
/// recover the removed identity. Recovery depends on what reached the recipient
/// and its bookmark; see [`Retire::Uncertain`](crate::Retire::Uncertain).
/// Serving a bootstrap removes the donated identity the same way. If that
/// session fails before sending, the identity returns to the live peer and is
/// recorded again at its next checkpoint; a crash before then loses it.
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

/// Cached restart bookkeeping, protected by the peer's bookmark mutex.
///
/// The mutex spans loading, mutation, and storage. Changes to the live party
/// take the replica lock briefly inside it; storage never holds that lock.
/// This keeps the live identity and the record consistent without blocking
/// local edits on storage I/O.
pub(crate) struct Bookmarked<B> {
    /// Application-owned storage for the encoded record.
    persist: B,
    /// The record in memory: absent before the first load, and again after
    /// any store that did not confirm success.
    cache: Option<Cache>,
    /// Maximum encoded record size, including its frame.
    size_limit: usize,
}

/// The record in memory, and which live states it and the durable record are
/// full checkpoints of.
pub(crate) struct Cache {
    /// Identities retained across restarts, grouped by network.
    record: Record,
    /// The live state this record is a full checkpoint of, if it is one.
    staged: Option<Checkpoint>,
    /// The live state the durable record is a full checkpoint of, if it is one.
    durable: Option<Checkpoint>,
}

/// The live state a full checkpoint captured.
///
/// A full checkpoint attempts reclamation and then records the live party.
/// Attachment records without reclaiming, and a donation changes the record
/// without recording, so neither produces one.
struct Checkpoint {
    /// The exact live party.
    party: Party,
    /// The frontier; only the party's own region is ever compared.
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

    /// Select the store limit, which the next session's store must apply.
    ///
    /// The limit shapes what a store keeps, so a checkpoint stored under the
    /// old limit no longer counts as stored.
    pub(crate) fn set_size_limit(&mut self, bytes: usize) {
        self.size_limit = bytes.max(format::record_size(0, 0));
        if let Some(cache) = &mut self.cache {
            cache.staged = None;
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
    /// Read storage unless the cache is present, then lend the cache.
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
                staged: None,
                durable: None,
            });
        }
        Ok(Loaded { bookmark: self })
    }
}

/// The cache, lent from a successful load until it is stored or released.
///
/// Only a `Loaded` can store, so every store follows a load. Dropping it
/// without storing keeps the cache for the next update.
pub(crate) struct Loaded<'a, B> {
    /// The bookmark whose cache is present for the lifetime of the loan.
    bookmark: &'a mut Bookmarked<B>,
}

/// Read the lent cache.
impl<B> Deref for Loaded<'_, B> {
    type Target = Cache;

    /// The cache is present for as long as the loan exists.
    fn deref(&self) -> &Cache {
        self.bookmark
            .cache
            .as_ref()
            .expect("a loan exists only while the cache is present")
    }
}

/// Change the lent cache.
impl<B> DerefMut for Loaded<'_, B> {
    /// The cache is present for as long as the loan exists.
    fn deref_mut(&mut self) -> &mut Cache {
        self.bookmark
            .cache
            .as_mut()
            .expect("a loan exists only while the cache is present")
    }
}

/// Store the lent cache.
impl<B: Bookmark> Loaded<'_, B> {
    /// Store the record, then note what the durable record is a checkpoint of.
    ///
    /// The cache leaves memory before the store and returns only on success.
    /// An error or cancellation therefore leaves the bookmark unloaded: the
    /// next update reads whichever complete record storage kept, and no token
    /// can describe a store that did not confirm.
    pub(crate) async fn write(self) -> Result<(), BookmarkIo<B::Error>> {
        let bookmark = self.bookmark;
        let mut cache = bookmark
            .cache
            .take()
            .expect("a loan exists only while the cache is present");
        let bytes = cache.record.bounded_bytes(bookmark.size_limit);
        bookmark
            .persist
            .store(bytes)
            .await
            .map_err(BookmarkIo::Io)?;
        cache.durable = cache.staged.take();
        bookmark.cache = Some(cache);
        Ok(())
    }
}

/// Change the record and decide whether it needs another store.
impl Cache {
    /// Checkpoint unless the durable record already is one for this live state.
    ///
    /// Returns whether the caller must store. Deciding and checkpointing in
    /// one call means a session can neither checkpoint without consulting the
    /// rule nor skip without it.
    pub(crate) fn checkpoint_if_needed(
        &mut self,
        network: Network,
        party: &mut Party,
        frontier: &Version,
        reclaim: bool,
    ) -> bool {
        if self.is_checkpointed(party, frontier) {
            return false;
        }
        self.checkpoint(network, party, frontier, reclaim);
        true
    }

    /// Whether the durable record is a full checkpoint of this live state.
    ///
    /// If it is, nothing reclaimable lies below what the session is about to
    /// transmit: either the record holds this party with `written` at or
    /// above its own progress, or the limit evicted that identity and nothing
    /// in its region is reclaimable at all. Either way the store is redundant.
    ///
    /// The comparison is exact on the party and on own progress only. Remote
    /// progress may have caught the frontier up with further identities; those
    /// wait for the session after the next local change. Reclaiming earlier
    /// would alter no version before that change in any case: the wait defers
    /// the benefit by one own event.
    fn is_checkpointed(&self, party: &Party, frontier: &Version) -> bool {
        self.durable.as_ref().is_some_and(|checkpoint| {
            checkpoint.party == *party && &checkpoint.frontier / party == frontier / party
        })
    }

    /// Remove a donation; the caller stores this before the donation leaves.
    ///
    /// The record is no longer a checkpoint of any live state.
    pub(crate) fn slice(&mut self, network: Network, donation: &Party) {
        self.record.slice(network, donation);
        self.staged = None;
    }

    /// Record the live party at attachment, without reclaiming.
    ///
    /// Not a full checkpoint: the first session must still attempt
    /// reclamation, so the store leaves no token behind.
    pub(crate) fn record(&mut self, network: Network, party: &Party, frontier: &Version) {
        self.record.network(network).record(party, frontier);
        self.staged = None;
    }

    /// Reclaim what the frontier has caught up with, when permitted, then
    /// record the live party: a full checkpoint, staged for the next store.
    ///
    /// The caller permits reclamation only while no bootstrap fork is
    /// reserved; see [`NetworkRecord::reclaim`](record::NetworkRecord::reclaim).
    /// Recording local writes continues either way.
    pub(crate) fn checkpoint(
        &mut self,
        network: Network,
        party: &mut Party,
        frontier: &Version,
        reclaim: bool,
    ) {
        let record = self.record.network(network);
        if reclaim {
            record.reclaim(party, frontier);
        }
        record.record(party, frontier);
        self.staged = Some(Checkpoint {
            party: party.dangerously_alias(),
            frontier: frontier.clone(),
        });
    }
}

#[cfg(test)]
mod tests;
