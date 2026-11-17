//! Configure a join session and retain its settings when a retry is
//! needed.

use std::{fmt, marker::PhantomData, ops::RangeBounds, sync::Arc};

use serde::{Serialize, de::DeserializeOwned};
use tokio::io::{AsyncBufRead, AsyncWrite};

use crate::bookmark::{Bookmark, NoBookmark};
use crate::message::MessageDepthLimit;
use crate::observe::Observer;
use crate::tree::mirror::streaming::remote::{RunBudget, WriteBatch};
use crate::{Error, Peer};

use super::bookmark::Unbookmarked;
use super::connection::Connection;
use super::{MemoryBudget, SessionSettings};

/// The configuration to join a gossip network via [`Peer::joining`].
///
/// This is a builder-style API, where [`join`](Self::join) finishes
/// configuration and joins the connected peer's gossip network, returning a
/// [`Peer`]. The newly created peer retains every setting selected
/// in the builder.
///
/// Any failure returns the builder itself within [`Join`]. Retry by calling
/// `join` on the returned builder, perhaps on another connection.
#[must_use = "a `Joining` does nothing until join runs it over a connection"]
pub struct Joining<T: Send + Sync + 'static, B: Bookmark = NoBookmark> {
    /// The session settings used during the join and inherited whole by the
    /// joined peer.
    pub(crate) settings: SessionSettings<T>,
    /// Message nesting limit used during and after the join.
    pub(crate) message_depth_limit: MessageDepthLimit,
    /// Storage owned by this attempt, returned if no peer arrives.
    bookmark: B,
    /// Maximum bookmark size inherited by the joined peer.
    bookmark_size_limit: usize,
    /// Marks the payload type without storing a value or constraining auto
    /// traits.
    marker: PhantomData<fn() -> T>,
}

/// Copy a builder without requiring the payload type to be Clone.
impl<T: Send + Sync + 'static> Clone for Joining<T> {
    /// Copy its settings and share its observation handlers.
    fn clone(&self) -> Self {
        self.session_config()
    }
}

/// Show the configuration without `Debug` bounds on the messages or storage.
impl<T: Send + Sync + 'static, B: Bookmark> std::fmt::Debug for Joining<T, B> {
    /// Describe the builder's settings without reading the bookmark.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sizing = self.settings.sizing;
        let limits = sizing.limits();
        f.debug_struct("Joining")
            .field("sync_memory_budget", &limits.budget.memory)
            .field("run_budget", &limits.target)
            .field("write_batch", &sizing.write_batch)
            .field("message_depth_limit", &self.message_depth_limit)
            .field("observe", &self.settings.observe)
            .field("bookmark", &std::any::type_name::<B>())
            .field("bookmark_size_limit", &self.bookmark_size_limit)
            .finish()
    }
}

impl<T: Send + Sync + 'static> Joining<T> {
    /// The default [`Joining`].
    pub(crate) fn new() -> Self {
        Self {
            settings: SessionSettings::default(),
            message_depth_limit: MessageDepthLimit::default(),
            bookmark: NoBookmark,
            bookmark_size_limit: crate::default::BOOKMARK_SIZE_LIMIT,
            marker: PhantomData,
        }
    }

    /// Rumors uses a [`Bookmark`] to record identity and local
    /// write progress, reclaiming old "identity space" opportunistically. This
    /// limits the storage used by [`Version`](crate::Version)s across restarts.
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
    pub fn bookmark<B: Bookmark>(self, bookmark: B) -> Joining<T, B> {
        Joining {
            settings: self.settings,
            message_depth_limit: self.message_depth_limit,
            bookmark,
            bookmark_size_limit: self.bookmark_size_limit,
            marker: PhantomData,
        }
    }
}

impl<T: Send + Sync + 'static, B: Bookmark> Joining<T, B> {
    /// Copy only session settings; storage stays with the retryable builder.
    fn session_config(&self) -> Joining<T> {
        Joining {
            settings: self.settings.clone(),
            message_depth_limit: self.message_depth_limit,
            bookmark: NoBookmark,
            bookmark_size_limit: self.bookmark_size_limit,
            marker: PhantomData,
        }
    }

    /// Set [`Peer::bookmark_size_limit`](crate::Peer::bookmark_size_limit)
    /// before joining.
    pub fn bookmark_size_limit(mut self, bytes: usize) -> Self {
        self.bookmark_size_limit = bytes;
        self
    }

    /// Set [`Peer::gossip_when`] before joining.
    pub fn gossip_when<F, S>(mut self, when: F) -> Self
    where
        F: Fn(crate::Changes<T>) -> S + Send + Sync + 'static,
        S: futures::Stream + Send + 'static,
        S::Item: Into<crate::Gossip>,
    {
        self.settings.gossip_policy.set_when(when);
        self
    }

    /// Set [`Peer::gossip_until`] before joining.
    pub fn gossip_until<F>(mut self, stop: F) -> Self
    where
        F: Future + Send + 'static,
    {
        self.settings.gossip_policy.set_until(stop);
        self
    }

    /// Set [`Peer::gossip_timeout`] before joining.
    pub fn gossip_timeout<D, F>(mut self, timeout: D) -> Self
    where
        D: Fn() -> F + Send + Sync + 'static,
        F: Future<Output = ()> + Send + 'static,
    {
        self.settings.gossip_policy.set_deadline(timeout);
        self
    }

    /// Set [`Peer::sync_memory_budget`] before joining.
    pub fn sync_memory_budget(mut self, budget_bytes: usize) -> Self {
        self.settings.sizing.sync_memory_budget = MemoryBudget::Requested(budget_bytes);
        self
    }

    /// Set [`Peer::target_message_size`] before joining.
    pub fn target_message_size(mut self, bytes: usize) -> Self {
        self.settings.sizing.run_budget = RunBudget::from_bytes(bytes);
        self
    }

    /// Set [`Peer::write_batch_range`] before joining.
    pub fn write_batch_range(mut self, range: impl RangeBounds<usize>) -> Self {
        self.settings.sizing.write_batch = WriteBatch::from_range(range);
        self
    }

    /// Set [`Peer::message_depth_limit`] before joining.
    ///
    /// Unlike the other settings, which could in principle wait to be set on
    /// the [`Peer`] once it is created, this option *must* be configured to
    /// exactly match the remote peer prior to bootstrapping; otherwise,
    /// bootstrapping will fail.
    pub fn message_depth_limit(mut self, limit: u64) -> Self {
        self.message_depth_limit = MessageDepthLimit::new(limit);
        self
    }

    /// Add an [`Observer`] for the session in which the peer joins the network,
    /// and all of its later gossip sessions.
    pub fn observe(mut self, observer: Arc<dyn Observer>) -> Self {
        self.settings.observe.attach(observer);
        self
    }
}

/// Join the gossip network from a peer already in it, using the configured
/// settings.
impl<T: Send + Sync + 'static, B: Bookmark> Joining<T, B> {
    /// Join a connected peer's gossip network.
    ///
    /// This returns a constructed [`Peer`] with the settings established on
    /// this builder, or (on error) a retryable copy of the original builder,
    /// annotated with the type of failure that led to this situation.
    pub async fn join<R, W>(self, mut read: R, mut write: W) -> Join<T, R, W, B>
    where
        T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
        R: AsyncBufRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
    {
        // The session needs a copy of the settings, not ownership of storage.
        // Retaining self lets both outcomes without a peer return it unchanged.
        let (peer, completed) =
            Peer::join_inner(self.session_config(), &mut read, &mut write).await;
        let Some(peer) = peer else {
            return match completed {
                Ok(()) => Join::Bounced {
                    joining: self,
                    read,
                    write,
                },
                Err(error) => Join::Failed {
                    error,
                    joining: self,
                },
            };
        };
        // The peer holds its whole identity whether or not the session
        // completed, so the selected bookmark is attached either way; the
        // connection's fate is reported beside it.
        let connection = Connection::from_session(completed, read, write);
        match peer
            .bookmark_size_limit(self.bookmark_size_limit)
            .bookmark(self.bookmark)
            .await
        {
            Ok(peer) => Join::Connected { peer, connection },
            Err(unbookmarked) => Join::Unbookmarked {
                unbookmarked,
                connection,
            },
        }
    }
}

/// The outcome of [`Peer::join`] and [`Joining::join`].
#[must_use = "a `Join` contains a peer or a builder needed for retry"]
pub enum Join<T: Send + Sync + 'static, R, W, B: Bookmark = NoBookmark> {
    /// The peer joined the network, and any selected bookmark was attached and
    /// persisted.
    Connected {
        /// The newly joined peer, carrying the selected configuration and
        /// storage.
        peer: Peer<T, B>,
        /// Whether the connection can be reused to carry another session.
        connection: Connection<R, W>,
    },
    /// The peer joined the network, but attaching the selected bookmark failed.
    ///
    /// The returned peer is connected, but unbookmarked; retry its bookmark
    /// attachment.
    Unbookmarked {
        /// The joined peer, paired with the bookmark failure.
        unbookmarked: Unbookmarked<T, B>,
        /// Whether the connection can be reused to carry another session.
        connection: Connection<R, W>,
    },
    /// Both endpoints were attempting to join each other, so neither could
    /// provide a gossip network to the other.
    Bounced {
        /// The complete builder, ready to retry joining (probably from a
        /// different peer).
        joining: Joining<T, B>,
        /// The connection's read half, which could be used for another session.
        read: R,
        /// The connection's write half, which could be used for another
        /// session.
        write: W,
    },
    /// The session failed.
    ///
    /// Discard the connection and try again.
    Failed {
        /// The error which failed the session.
        error: Error,
        /// The complete builder, ready to retry joining on a fresh connection.
        joining: Joining<T, B>,
    },
}

/// Format the outcome of attempting to join a network without requiring
/// debuggable message, bookmark, or transport types.
impl<T: Send + Sync + 'static, R, W, B: Bookmark> fmt::Debug for Join<T, R, W, B> {
    /// Formats the outcome and the peer/state it carries.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connected { peer, connection } => f
                .debug_struct("Connected")
                .field("peer", peer)
                .field("connection", connection)
                .finish(),
            Self::Bounced { joining, .. } => f
                .debug_struct("Bounced")
                .field("joining", joining)
                .finish_non_exhaustive(),
            Self::Unbookmarked {
                unbookmarked,
                connection,
            } => f
                .debug_struct("Unbookmarked")
                .field("unbookmarked", unbookmarked)
                .field("connection", connection)
                .finish(),
            Self::Failed { error, joining } => f
                .debug_struct("Failed")
                .field("error", error)
                .field("joining", joining)
                .finish(),
        }
    }
}

#[cfg(test)]
mod tests;
