//! Restart bookkeeping that limits growth of message versions.
//!
//! Applications provide opaque byte storage through [`Bookmark`]. The crate
//! owns the record format and the rules for recycling a departed peer's identity.

use std::collections::BTreeMap;

use before::{Clock, Party, Version};
use tokio::io::{AsyncRead, AsyncReadExt};

use crate::Network;

pub(crate) mod format;

pub use format::{BOOKMARK_FORMAT_VERSION, FormatError, FrameDefect, RecordDefect};

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
/// Unreclaimed incarnations remain in the bookmark, so repeated restarts can
/// grow the record. Records also stay separate by [`Network`]: joining another
/// network is supported, and its entries coexist with those of earlier networks.
/// Entries from another network remain dormant until the peer rejoins it.
///
/// # Storage obligations
///
/// - Use one bookmark for one peer across its restarts. Never share it between
///   concurrently live peers or duplicate it to start another peer.
/// - Replace the record atomically, and never roll back a successful store. A
///   stale but valid record can cause versions to be reused and corrupt the set.
/// - Preserve the record across restarts, including entries for earlier networks.
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
}

/// A loaded record and the checkpoint used to avoid redundant stores.
pub(crate) struct Loaded {
    /// Identities retained across restarts, grouped by network.
    record: BTreeMap<Network, Vec<Clock>>,
    /// The checkpoint to install only after its store succeeds.
    staged: Option<(Party, Version)>,
    /// The last confirmed checkpoint; removing a donation invalidates it.
    last: Option<(Party, Version)>,
}

/// Construct an unloaded bookmark cache.
impl<B> Bookmarked<B> {
    /// Keep the storage without reading it until the first update.
    pub(crate) fn new(persist: B) -> Self {
        Self {
            persist,
            loaded: None,
        }
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
                    None => BTreeMap::new(),
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
    /// Failure discards the cache so the next update reloads whichever complete
    /// record storage retained. Neither outcome establishes a current checkpoint.
    /// Cancellation leaves the staged checkpoint uncommitted: the next update
    /// must retry rather than skip a store whose completion is unknown.
    pub(crate) async fn write(&mut self) -> Result<(), BookmarkIo<B::Error>> {
        let Some(loaded) = &mut self.loaded else {
            return Ok(());
        };
        let bytes = format::encode(&loaded.record);
        match self.persist.store(bytes).await {
            Ok(()) => {
                if let Some(checkpoint) = loaded.staged.take() {
                    loaded.last = Some(checkpoint);
                }
                Ok(())
            }
            Err(error) => {
                self.loaded = None;
                Err(BookmarkIo::Io(error))
            }
        }
    }
}

/// Update identity ownership and decide when it needs another checkpoint.
impl Loaded {
    /// Whether the stored checkpoint covers this party's own writes.
    pub(crate) fn is_current(&self, party: &Party, version: &Version) -> bool {
        self.last.as_ref().is_some_and(|(p, v)| {
            // Learning about other parties does not change our checkpoint.
            // Identity changes and advances in our own interval do.
            p == party && v / p == version / p
        })
    }

    /// Remove a donation; the caller must persist this before sending it.
    pub(crate) fn slice(&mut self, network: Network, party: &Party) {
        if let Some(clocks) = self.record.remove(&network) {
            let clocks: Vec<_> = clocks
                .into_iter()
                .filter_map(|clock| {
                    let (p, v) = clock.into_parts();
                    Some(Clock::from_parts(p.without(party)?, v))
                })
                .collect();
            if !clocks.is_empty() {
                self.record.insert(network, clocks);
            }
        }
        // A checkpoint of the larger identity cannot justify skipping a store
        // after donation, even if that identity later returns through retirement.
        self.staged = None;
        self.last = None;
    }

    /// Record the live identity at attachment without growing it by reclamation.
    pub(crate) fn record(&mut self, network: Network, party: &Party, version: &Version) {
        self.record
            .entry(network)
            .or_default()
            .push(Clock::from_parts(
                party.dangerously_alias(),
                version.clone(),
            ));
        // The first gossip must still attempt reclamation. Attachment records
        // ownership but does not establish a checkpoint that can skip that work.
        self.staged = None;
    }

    /// Reclaim caught-up identities and stage a checkpoint of the resulting party.
    ///
    /// Run inside the replica's critical section so the party and its recorded
    /// version change together. The checkpoint becomes current only after storage.
    pub(crate) fn reclaim(&mut self, network: Network, party: &mut Party, version: &Version) {
        let clocks = self.record.entry(network).or_default();
        let mut overlapping = Vec::new();
        // Reuse requires knowing everything the old identity wrote, not
        // everything it observed. Compare only its owned version interval.
        for clock in clocks.extract_if(.., |clock| clock.own_version() <= *version) {
            let (p, v) = clock.into_parts();
            if let Err(p) = party.join(p) {
                overlapping.push(Clock::from_parts(p, v));
            }
        }
        // Some stored aliases overlap rather than join. Remove only those
        // covered by the fully grown party; a larger outstanding alias must stay.
        clocks.extend(
            overlapping
                .into_iter()
                .filter(|clock| !party.covers(clock.party())),
        );
        clocks.push(Clock::from_parts(
            party.dangerously_alias(),
            version.clone(),
        ));
        self.staged = Some((party.dangerously_alias(), version.clone()));
    }
}

#[cfg(test)]
mod tests;
