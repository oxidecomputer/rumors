//! Attach bookmark storage to a peer and preserve the peer on failure.

use std::fmt;

use crate::bookmark::{
    Bookmark, BookmarkHandle, MaybeBookmark, NoBookmark, NullBookmark, StorageIo, Token,
};

use super::Replica;
use crate::Peer;

/// The result of failing to attach a [`Bookmark`], with the
/// [`Peer`] returned unchanged.
///
/// This error is returned by [`PeerBuilder::seed`](super::PeerBuilder::seed) or
/// [`Join::Unbookmarked`](super::Join::Unbookmarked). The peer is still
/// usable without a bookmark, though it cannot persist or reclaim identity
/// space. Repair or replace the storage, then try again.
#[derive(thiserror::Error)]
#[error("could not attach bookmark to peer")]
#[must_use = "a failed bookmark attachment returns the peer for continued use or retry"]
pub struct Unbookmarked<T: Send + Sync + 'static, B: MaybeBookmark> {
    /// The unchanged [`Peer`], without any bookmark attached.
    peer: Replica<T, NoBookmark>,
    /// The underlying storage or decoding failure.
    #[source]
    pub error: B::Failure,
}

/// Format a bookmark failure without requiring debuggable message or storage
/// types.
impl<T: Send + Sync + 'static, B: MaybeBookmark> fmt::Debug for Unbookmarked<T, B> {
    /// Format the error.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Unbookmarked")
            .field("peer", &self.peer)
            .field("error", &self.error)
            .finish()
    }
}

/// Construct the unbookmarked state, and leave it by retrying attachment or
/// proceeding without a bookmark.
impl<T: Send + Sync + 'static, B: MaybeBookmark> Unbookmarked<T, B> {
    /// Pair `peer`, which holds no bookmark, with the failure to attach one.
    pub(crate) fn new(peer: Replica<T, NoBookmark>, error: B::Failure) -> Self {
        Self { peer, error }
    }

    /// Retry attaching a bookmark, either the same repaired storage or a
    /// different one.
    pub async fn retry_bookmark<B2: Bookmark>(
        self,
        bookmark: B2,
    ) -> Result<Peer<T, B2>, Unbookmarked<T, B2>> {
        self.peer.attach_bookmark(bookmark).await.map(Peer::new)
    }

    /// Proceed without a bookmark.
    pub fn continue_unbookmarked(self) -> Peer<T> {
        Peer::new(self.peer)
    }
}

/// Attach [`Bookmark`] storage to a [`Peer`] which lacks it.
impl<T: Send + Sync + 'static> Replica<T> {
    /// Attach `bookmark`, recording this peer's identity in it unless the peer
    /// is a pristine seed.
    pub(crate) async fn attach_bookmark<B: Bookmark>(
        self,
        bookmark: B,
    ) -> Result<Replica<T, B>, Unbookmarked<T, B>> {
        let handle = BookmarkHandle::new(bookmark, self.bookmark.size_limit());
        self.attach_handle::<B>(handle).await
    }

    /// Attach the storage `handle` holds, recording this peer's identity in it
    /// unless the peer is a pristine seed.
    ///
    /// A handle whose record is already loaded is not read again.
    pub(crate) async fn attach_handle<B: MaybeBookmark>(
        self,
        handle: BookmarkHandle<B::Storage>,
    ) -> Result<Replica<T, B>, B::Unbookmarked<T>> {
        let peer = self.with_handle::<B>(handle);

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
            Err(error) => {
                let handle = BookmarkHandle::new(NullBookmark, peer.bookmark.size_limit());
                Err(B::unbookmarked(
                    peer.with_handle::<NoBookmark>(handle),
                    error,
                    Token(()),
                ))
            }
        }
    }
}

/// Change a peer's bookmark without disturbing its other configuration.
impl<T: Send + Sync + 'static, B: MaybeBookmark> Replica<T, B> {
    /// Replace the bookmark handle, keeping every other part of the peer.
    ///
    /// The new handle brings its own record and lock, so this never waits on
    /// the old lock or reads through it.
    fn with_handle<N: MaybeBookmark>(self, bookmark: BookmarkHandle<N::Storage>) -> Replica<T, N> {
        let Replica {
            keys,
            settings,
            inner,
            bookmark: _,
            codec,
        } = self;
        Replica {
            keys,
            settings,
            inner,
            bookmark,
            codec,
        }
    }

    /// Record this peer's current ownership without reclaiming stored
    /// identities.
    async fn record_bookmark(&self) -> Result<(), StorageIo<B>> {
        let mut bookmark = self.bookmark.lock().await;
        let loaded = bookmark.load().await?;
        let store = {
            let inner = self.inner.borrow();
            loaded.attach(self.network(), &inner.party, inner.tree.latest())
        };
        store.await
    }
}
