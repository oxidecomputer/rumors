//! Attach bookmark storage to a peer and preserve the peer on failure.

use std::fmt;

use crate::bookmark::{Bookmark, BookmarkIo, NoBookmark};

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

/// Format an attachment failure without requiring debuggable payload or storage
/// types.
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
    /// Rumors uses a "bookmark" to record a peer's internal identity and local
    /// write progress, then reclaims old "identity space" opportunistically.
    /// This limits the growth of message [`Version`](crate::Version)s across
    /// crashes and restarts.
    ///
    /// Bookmarks do not store the contents of messages. Their use is a
    /// (recommended!) optional optimization to prevent a space leak in the
    /// event that a peer restarts without first [`retire`](Peer::retire)-ing
    /// into another [`Peer`].
    ///
    /// You should reuse the same underlying bookmark storage whenever this peer
    /// restarts.
    ///
    /// For a simple bookmark implementation that uses a single file to store
    /// the bookmark, supply [`FileBookmark`](crate::FileBookmark).
    ///
    /// To attach a bookmark immediately while joining an existing network
    /// (rather than after the peer bootstraps), use
    /// [`Bootstrap::bookmark`](super::Bootstrap::bookmark).
    ///
    /// # Errors
    ///
    /// A load, validation, or store failure returns the original peer,
    /// unchanged and without a bookmark attached, in the [`Unbookmarked`]
    /// error. It is safe to retry attaching the bookmark in case of transient
    /// failures.
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
    /// Replace bookmark storage while carrying the configured size limit
    /// forward.
    ///
    /// The new storage gets a fresh record and lock, and the size limit lives
    /// outside the old lock, so this never waits on the old lock or reads
    /// through it.
    fn with_bookmark<N: Bookmark>(self, persist: N) -> Peer<T, N> {
        let Peer {
            keys,
            settings,
            inner,
            bookmark,
            codec,
        } = self;
        Peer {
            keys,
            settings,
            inner,
            bookmark: bookmark.replace(persist),
            codec,
        }
    }

    /// Record this peer's current ownership without reclaiming stored
    /// identities.
    async fn record_bookmark(&self) -> Result<(), BookmarkIo<B::Error>> {
        let mut bookmark = self.bookmark.lock().await;
        let loaded = bookmark.load().await?;
        let store = {
            let inner = self.inner.borrow();
            loaded.attach(self.network(), &inner.party, inner.tree.latest())
        };
        store.await
    }
}
