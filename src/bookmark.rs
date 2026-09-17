//! Restart bookkeeping that limits growth of message versions.
//!
//! Applications provide opaque byte storage through [`Bookmark`]. The crate
//! owns the record format and the rules for recycling a departed peer's identity.
//!
//! Recovery has three requirements: know all recorded writes in the recovered
//! region, never acquire a live bootstrap's fork, and durably remove donated
//! rights before sending them. `record` owns the first rule; the peer's bootstrap
//! guards and session driver enforce the other two. This module loads and stores
//! the record, treating only a confirmed store as a reusable checkpoint.

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
/// Reclamation happens when Rumors updates the bookmark, so it can lag behind
/// catching up. Ordinary gossip records local progress before exchanging
/// messages, and does not write again just because it learns remote changes. An
/// identity that becomes reclaimable during an exchange can therefore wait
/// until a later session after a local send, redaction, or identity change.
/// Reclamation also waits while bootstrap sessions hold identities for transfer.
/// These delays postpone the reduction in version size; they do not delay
/// checkpointing local writes.
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
/// Retirement removes the departing identity from the local bookmark before
/// sending it. If the transfer then fails or is cancelled, that bookmark cannot
/// recover the removed identity. Recovery depends on what reached the recipient
/// and its bookmark; see [`Retire::Uncertain`](crate::Retire::Uncertain).
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
#[derive(Debug)]
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
    /// An unloaded cache must read storage before making any changes.
    loaded: Option<Loaded>,
    /// Maximum encoded record size, including its frame.
    size_limit: usize,
}

/// A loaded record and the checkpoint used to avoid redundant stores.
pub(crate) struct Loaded {
    /// Identities retained across restarts, grouped by network.
    record: Record,
    /// The checkpoint to install only after its store succeeds.
    staged: Option<Checkpoint>,
    /// The last confirmed checkpoint; removing a donation invalidates it.
    last: Option<Checkpoint>,
}

/// The local ownership and write progress protected by one checkpoint.
struct Checkpoint {
    /// The exact live party at the checkpoint.
    party: Party,
    /// Known progress; only the party's own region matters for skipping stores.
    version: Version,
}

/// Construct an unloaded bookmark cache.
impl<B> Bookmarked<B> {
    /// Keep the storage without reading it until the first update.
    pub(crate) fn new(persist: B) -> Self {
        Self {
            persist,
            loaded: None,
            size_limit: DEFAULT_BOOKMARK_SIZE_LIMIT,
        }
    }

    /// Select the store limit and require a checkpoint to apply it.
    pub(crate) fn set_size_limit(&mut self, bytes: usize) {
        self.size_limit = bytes.max(format::record_size(0, 0));
        if let Some(loaded) = &mut self.loaded {
            loaded.last = None;
            loaded.staged = None;
        }
    }

    /// The configured limit, also retained before storage is attached.
    pub(crate) fn size_limit(&self) -> usize {
        self.size_limit
    }
}

/// Load and persist records while keeping storage failures recoverable.
impl<B: Bookmark> Bookmarked<B> {
    /// Load on first use and return the state that can be safely changed.
    pub(crate) async fn ensure_loaded(&mut self) -> Result<&mut Loaded, BookmarkIo<B::Error>> {
        match self.loaded {
            Some(ref mut loaded) => Ok(loaded),
            None => {
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
                Ok(self.loaded.insert(Loaded {
                    record,
                    staged: None,
                    last: None,
                }))
            }
        }
    }

    /// Store the loaded record and commit its staged checkpoint on success.
    ///
    /// Take the cache out before I/O and restore it only on success. An error
    /// or cancellation then leaves the cache unloaded, so the next update must
    /// read whichever complete record storage retained.
    pub(crate) async fn write(&mut self) -> Result<(), BookmarkIo<B::Error>> {
        let Some(mut loaded) = self.loaded.take() else {
            return Ok(());
        };
        let bytes = loaded.record.bounded_bytes(self.size_limit);
        self.persist.store(bytes).await.map_err(BookmarkIo::Io)?;
        loaded.last = loaded.staged.take();
        self.loaded = Some(loaded);
        Ok(())
    }
}

/// Update identity ownership and decide when it needs another checkpoint.
impl Loaded {
    /// Whether a confirmed store still protects this party's own writes.
    ///
    /// This deliberately requires the same party and own-write progress. A
    /// larger stored party could sometimes cover a smaller current one too,
    /// but checkpointing that ownership change keeps the rule simple. Remote
    /// progress may permit more reclamation without requiring another store.
    pub(crate) fn can_skip_checkpoint(&self, party: &Party, version: &Version) -> bool {
        self.last.as_ref().is_some_and(|checkpoint| {
            checkpoint.party == *party && &checkpoint.version / party == version / party
        })
    }

    /// Remove a donation; the caller must persist this before sending it.
    pub(crate) fn slice(&mut self, network: Network, party: &Party) {
        self.record.slice(network, party);
        self.staged = None;
        self.last = None;
    }

    /// Record the live identity at attachment without acquiring another identity.
    pub(crate) fn record(&mut self, network: Network, party: &Party, version: &Version) {
        self.record.network(network).record(party, version);
        // Attachment does not reclaim; the first session must still try.
        self.staged = None;
    }

    /// Stage the live party's checkpoint, optionally reclaiming caught-up
    /// identities.
    ///
    /// The caller permits reclamation only when no bootstrap guard holds a fork.
    /// While guards remain, their forks may still appear in the bookmark, but
    /// cannot be acquired. Recording local writes must continue either way.
    pub(crate) fn checkpoint(
        &mut self,
        network: Network,
        party: &mut Party,
        version: &Version,
        reclaim: bool,
    ) {
        let record = self.record.network(network);
        if reclaim {
            record.reclaim(party, version);
        }
        record.record(party, version);
        self.staged = Some(Checkpoint {
            party: party.dangerously_alias(),
            version: version.clone(),
        });
    }
}

#[cfg(test)]
mod tests;
