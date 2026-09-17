//! Attach bookmark storage to a peer and preserve the peer on failure.

use std::{fmt, sync::Arc};

use tokio::sync::Mutex;

use crate::bookmark::{Bookmark, BookmarkIo, Bookmarked, NoBookmark};

use super::Peer;

/// A failed bookmark attachment, with the peer returned unchanged.
///
/// Produced by [`Peer::bookmark`] or
/// [`Joined::Unbookmarked`](super::Joined::Unbookmarked). The peer remains
/// usable without a bookmark. Repair or replace the storage, then call
/// `bookmark` on the returned peer to retry.
#[must_use = "a failed bookmark attachment returns the peer for continued use or retry"]
pub struct Unbookmarked<T: Send + Sync + 'static, B: Bookmark> {
    /// The unchanged peer, with no bookmark attached.
    pub peer: Peer<T, NoBookmark>,
    /// The storage or decoding failure.
    pub error: BookmarkIo<B::Error>,
}

/// Format an attachment failure without requiring debuggable payload or storage types.
impl<T: Send + Sync + 'static, B: Bookmark> fmt::Debug for Unbookmarked<T, B> {
    /// Format the returned peer and storage error.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Unbookmarked")
            .field("peer", &self.peer)
            .field("error", &self.error)
            .finish()
    }
}

/// Attach bookmark storage to an unbookmarked peer.
impl<T: Send + Sync + 'static> Peer<T> {
    /// Add durable restart bookkeeping to this peer.
    ///
    /// Reuse the same storage whenever this peer restarts. Rumors records the
    /// peer's identity and local write progress, then reclaims old identity
    /// space once the restarted peer has caught up. This limits the growth of
    /// message [`Version`](crate::Version)s across crashes and restarts.
    ///
    /// A bookmark does not store messages. Recover those by joining and
    /// gossiping. The returned peer owns the storage and updates it as needed;
    /// do not share it with another live peer. [`Bookmark`] gives the complete
    /// storage and restart contract.
    ///
    /// This method validates any existing record and checkpoints the current
    /// peer before returning. An untouched seed postpones its first bookmark I/O
    /// until its first session. To attach storage while joining, use
    /// [`Bootstrap::bookmark`](super::Bootstrap::bookmark).
    ///
    /// # Errors
    ///
    /// A load, validation, or store failure returns the peer without a bookmark
    /// in [`Unbookmarked`]. Its live state is unchanged and attachment may be
    /// retried. As required by [`Bookmark::store`], a failed store may already
    /// have installed its complete replacement record.
    pub async fn bookmark<B: Bookmark>(
        self,
        bookmark: B,
    ) -> Result<Peer<T, B>, Unbookmarked<T, B>> {
        let peer = self.with_bookmark(bookmark);

        // A pristine seed has no identity worth recording yet. Anything the
        // peer knows, through content or changed ownership, must be durable
        // before attachment returns.
        let pristine = {
            let inner = peer.inner.borrow();
            inner.tree.latest().is_empty() && inner.party.is_seed()
        };
        if pristine {
            return Ok(peer);
        }

        // Attachment records ownership without reclaiming stored identities.
        // A failed write leaves only another claim to this peer's own identity,
        // so the unbookmarked peer remains safe to return.
        match peer.record_bookmark().await {
            Ok(()) => Ok(peer),
            Err(error) => Err(Unbookmarked {
                peer: peer.with_bookmark(NoBookmark),
                error,
            }),
        }
    }
}

/// Change a peer's bookmark type without disturbing its other configuration.
impl<T: Send + Sync + 'static, B: Bookmark> Peer<T, B> {
    /// Replace bookmark storage while carrying the configured size limit forward.
    ///
    /// A `Peer` is unique and cannot have a session borrowing it while this
    /// method consumes it, so its bookmark mutex is immediately available.
    fn with_bookmark<N: Bookmark>(self, persist: N) -> Peer<T, N> {
        let size_limit = self
            .bookmark
            .try_lock()
            .expect("a Peer has no running sessions")
            .size_limit();
        let mut bookmark = Bookmarked::new(persist);
        bookmark.set_size_limit(size_limit);
        let Peer {
            network,
            window,
            run_budget,
            gossip_policy,
            inner,
            bookmark: _,
            codec,
            observe,
        } = self;
        Peer {
            network,
            window,
            run_budget,
            gossip_policy,
            inner,
            bookmark: Arc::new(Mutex::new(bookmark)),
            codec,
            observe,
        }
    }

    /// Record this peer's current ownership without reclaiming stored identities.
    async fn record_bookmark(&self) -> Result<(), BookmarkIo<B::Error>> {
        let mut bookmark = self.bookmark.lock().await;
        let loaded = bookmark.ensure_loaded().await?;
        {
            let inner = self.inner.borrow();
            loaded.record(self.network, &inner.party, inner.tree.latest());
        }
        bookmark.write().await
    }
}
