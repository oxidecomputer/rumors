//! Configure a bootstrap session and retain its settings when a retry is
//! needed.

use std::{fmt, marker::PhantomData, ops::RangeBounds, sync::Arc};

use serde::{Serialize, de::DeserializeOwned};
use tokio::io::{AsyncBufRead, AsyncWrite};

use crate::bookmark::{Bookmark, NoBookmark};
use crate::message::PayloadDepthLimit;
use crate::observe::Observer;
use crate::tree::mirror::streaming::remote::{RunBudget, WriteBatch};
use crate::{Error, Peer};

use super::bookmark::Unbookmarked;
use super::connection::Connection;
use super::{MemoryBudget, SessionSettings};

/// Configuration for joining a gossip network through [`Peer::bootstrap`].
///
/// [`join`](Self::join) joins the connected peer's gossip network and receives
/// its current message set. The new peer retains every setting selected here.
///
/// A failed session or a meeting between two bootstrappers returns this builder
/// in [`Joined`], including any selected bookmark. Retry by calling `join` on
/// the returned builder on another connection. No configuration or storage
/// clone is needed. Builders without a bookmark also implement [`Clone`].
///
/// Select [`bookmark`](Self::bookmark) to attach restart bookkeeping before
/// returning the joined peer. A storage failure after joining instead returns
/// the live, unbookmarked peer through [`Joined::Unbookmarked`].
///
/// # Serving a bootstrap
///
/// An established peer automatically serves bootstrappers through ordinary
/// [`gossip`](crate::Rumors::gossip); no special invocation is required.
#[must_use = "a Bootstrap does nothing until join runs it over a connection"]
pub struct Bootstrap<T: Send + Sync + 'static, B: Bookmark = NoBookmark> {
    /// The session settings used during the join and inherited whole by the
    /// joined peer.
    pub(crate) settings: SessionSettings<T>,
    /// Payload nesting limit used during and after the join.
    pub(crate) payload_depth_limit: PayloadDepthLimit,
    /// Storage owned by this attempt, returned if no peer arrives.
    bookmark: B,
    /// Maximum bookmark size inherited by the joined peer.
    bookmark_size_limit: usize,
    /// Marks the payload type without storing a value or constraining auto
    /// traits.
    marker: PhantomData<fn() -> T>,
}

/// Copy an unbookmarked builder without requiring the payload type to be Clone.
impl<T: Send + Sync + 'static> Clone for Bootstrap<T> {
    /// Copy the settings and share the observation handlers.
    fn clone(&self) -> Self {
        self.session_config()
    }
}

/// Show configuration without Debug bounds on the payload or storage.
impl<T: Send + Sync + 'static, B: Bookmark> std::fmt::Debug for Bootstrap<T, B> {
    /// Describe the settings without reading the bookmark.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sizing = self.settings.sizing;
        let limits = sizing.limits();
        f.debug_struct("Bootstrap")
            .field("sync_memory_budget", &limits.budget.memory)
            .field("run_budget", &limits.target)
            .field("write_batch", &sizing.write_batch)
            .field("payload_depth_limit", &self.payload_depth_limit)
            .field("observe", &self.settings.observe)
            .field("bookmark", &std::any::type_name::<B>())
            .field("bookmark_size_limit", &self.bookmark_size_limit)
            .finish()
    }
}

/// Construct an unbookmarked builder or select its storage.
impl<T: Send + Sync + 'static> Bootstrap<T> {
    /// Use the default session settings without a bookmark.
    pub(crate) fn new() -> Self {
        Self {
            settings: SessionSettings::default(),
            payload_depth_limit: PayloadDepthLimit::default(),
            bookmark: NoBookmark,
            bookmark_size_limit: crate::default::BOOKMARK_SIZE_LIMIT,
            marker: PhantomData,
        }
    }

    /// Attach a bookmark before returning the joined peer.
    ///
    /// Reuse the bookmark across restarts to limit version growth. It stores
    /// restart bookkeeping, not messages; the join receives those from the
    /// provider. See [`Bookmark`] for storage and ownership requirements.
    ///
    /// This calls [`Peer::bookmark`] once the session has ended with this side
    /// holding its whole identity, whether or not the session then completed.
    /// If attachment fails, [`Joined::Unbookmarked`] returns the live peer and
    /// the storage error. If the session fails before the identity arrives, or
    /// bails, the returned builder owns the untouched bookmark and can retry.
    pub fn bookmark<B: Bookmark>(self, bookmark: B) -> Bootstrap<T, B> {
        Bootstrap {
            settings: self.settings,
            payload_depth_limit: self.payload_depth_limit,
            bookmark,
            bookmark_size_limit: self.bookmark_size_limit,
            marker: PhantomData,
        }
    }
}

/// Configure the session independently of whether storage has been selected.
impl<T: Send + Sync + 'static, B: Bookmark> Bootstrap<T, B> {
    /// Copy only session settings; storage stays with the retryable builder.
    fn session_config(&self) -> Bootstrap<T> {
        Bootstrap {
            settings: self.settings.clone(),
            payload_depth_limit: self.payload_depth_limit,
            bookmark: NoBookmark,
            bookmark_size_limit: self.bookmark_size_limit,
            marker: PhantomData,
        }
    }

    /// Limit restart bookkeeping before attachment; see
    /// [`Peer::bookmark_size_limit`].
    pub fn bookmark_size_limit(mut self, bytes: usize) -> Self {
        self.bookmark_size_limit = bytes;
        self
    }

    /// Select the joined peer's initiation policy; see [`Peer::gossip_when`].
    pub fn gossip_when<F, S>(mut self, when: F) -> Self
    where
        F: Fn(crate::Changes<T>) -> S + Send + Sync + 'static,
        S: futures::Stream + Send + 'static,
        S::Item: Into<crate::Gossip>,
    {
        self.settings.gossip_policy.set_when(when);
        self
    }

    /// Set the signal that ends the joined peer's gossip drivers; see
    /// [`Peer::gossip_until`].
    pub fn gossip_until<F>(mut self, stop: F) -> Self
    where
        F: Future + Send + 'static,
    {
        self.settings.gossip_policy.set_until(stop);
        self
    }

    /// Set the join's session deadline, also inherited by the joined peer. See
    /// [`Peer::session_deadline`] for timing and expiry behavior. Bookmark
    /// attachment after the wire session is not covered.
    pub fn session_deadline<D, F>(mut self, deadline: D) -> Self
    where
        D: Fn() -> F + Send + Sync + 'static,
        F: Future<Output = ()> + Send + 'static,
    {
        self.settings.gossip_policy.set_deadline(deadline);
        self
    }

    /// Set the joined peer's pipelining memory budget.
    ///
    /// Bootstrap receives into an empty replica and has no disputed subtrees to
    /// pipeline, so the join's own session holds little beyond what every
    /// budget sets aside, its room for messages on their way included; the
    /// budget matters most to its later synchronizations.
    /// [`target_message_size`](Self::target_message_size) bounds bootstrap's
    /// supply runs. As with [`Peer::sync_memory_budget`], a session lowers a
    /// [`target_message_size`](Self::target_message_size) the budget cannot
    /// afford, and raises a budget below the minimum for its target to that
    /// minimum, at most about 36.4 MiB, whichever order the two are set in.
    /// See [`Peer::sync_memory_budget`] for the contract and [the sizing
    /// guide](crate::sizing) for tuning guidance.
    pub fn sync_memory_budget(mut self, budget_bytes: usize) -> Self {
        self.settings.sizing.sync_memory_budget = MemoryBudget::Requested(budget_bytes);
        self
    }

    /// Set the batched-message size target during and after bootstrap.
    ///
    /// The provider uses the smaller of its target and ours, ours lowered to
    /// what our [`sync_memory_budget`](Self::sync_memory_budget) affords. Zero
    /// sends one leaf per message. See [`Peer::target_message_size`] for
    /// defaults and what the target accounts for.
    pub fn target_message_size(mut self, bytes: usize) -> Self {
        self.settings.sizing.run_budget = RunBudget::from_bytes(bytes);
        self
    }

    /// Set the write batch range during and after bootstrap.
    ///
    /// See [`Peer::write_batch_range`] for the default, how the bounds convert,
    /// and how to choose a range.
    pub fn write_batch_range(mut self, range: impl RangeBounds<usize>) -> Self {
        self.settings.sizing.write_batch = WriteBatch::from_range(range);
        self
    }

    /// Set the payload nesting limit during and after bootstrap.
    ///
    /// See [`Peer::payload_depth_limit`] for defaults and fleet coordination.
    pub fn payload_depth_limit(mut self, limit: u64) -> Self {
        self.payload_depth_limit = PayloadDepthLimit::new(limit);
        self
    }

    /// Add an observer for this bootstrap and the joined peer's later sessions.
    ///
    /// Calling `observe` again adds another observer; callbacks visit them in
    /// registration order. Every observer also survives failed attempts in the
    /// returned builder. See [`observe`](crate::observe) for the observation
    /// contract.
    pub fn observe(mut self, observer: Arc<dyn Observer>) -> Self {
        self.settings.observe.attach(observer);
        self
    }
}

/// Run a configured bootstrap and attach its selected bookmark.
impl<T: Send + Sync + 'static, B: Bookmark> Bootstrap<T, B> {
    /// Join the connected peer's gossip network, returning a peer or a
    /// retryable builder.
    ///
    /// `read` and `write` are the two halves of one connection to an
    /// established member; see the [transport requirements](crate#transport).
    /// Pass `&mut` halves to keep the connection after the session.
    ///
    /// Success receives the provider's current message set. The provider may be
    /// gossiping or retiring. Two bootstrappers cannot supply each other; both
    /// return [`Joined::Bailed`] with their builders.
    ///
    /// [`Joined::Failed`] returns the builder when the session fails before
    /// this side has received its whole identity, including on
    /// [`session_deadline`](Self::session_deadline) expiry. Discard the
    /// connection and retry on another. Once the whole identity has arrived,
    /// this side has joined however the session ends, because the provider gave
    /// that identity up before sending it. A failure after that point, while
    /// the two sides confirm completion, returns the live peer with a
    /// [`Connection::Lost`] carrying the error.
    ///
    /// If a bookmark was selected, joining then attaches it through
    /// [`Peer::bookmark`], whether or not the session completed. A failed
    /// attachment returns [`Joined::Unbookmarked`], preserving the joined peer.
    ///
    /// Cancelling drops the builder and any joined peer. Cancellation during
    /// the session leaves the connection at an unknown position; once bookmark
    /// attachment begins, the session has ended, and the connection stays where
    /// the session left it: at its boundary if the session completed, at an
    /// unknown position if it failed. See [what a session
    /// promises](crate#what-a-session-promises).
    ///
    /// The provider gives up the identity it donates before it starts to send
    /// it, so cancelling a join after that point leaves the identity held by no
    /// peer. No message is lost and no identity is duplicated, but the identity
    /// can never be recycled, which lets versions grow as a crashed peer's
    /// identity does (see [`Bookmark`]). To bound a join's duration, configure
    /// [`session_deadline`](Self::session_deadline) rather than dropping the
    /// future: a deadline that expires after the whole identity has arrived
    /// returns the live peer with a [`Connection::Lost`], where dropping the
    /// future would discard it.
    pub async fn join<R, W>(self, mut read: R, mut write: W) -> Joined<T, R, W, B>
    where
        T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
        R: AsyncBufRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
    {
        // The session needs a copy of the settings, not ownership of storage.
        // Retaining self lets both outcomes without a peer return it unchanged.
        let (peer, completed) =
            Peer::bootstrap_inner(self.session_config(), &mut read, &mut write).await;
        let Some(peer) = peer else {
            return match completed {
                Ok(()) => Joined::Bailed {
                    bootstrap: self,
                    read,
                    write,
                },
                Err(error) => Joined::Failed {
                    error,
                    bootstrap: self,
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
            Ok(peer) => Joined::Joined { peer, connection },
            Err(unbookmarked) => Joined::Unbookmarked {
                unbookmarked,
                connection,
            },
        }
    }
}

/// A bootstrap's outcome: a new peer and its connection's fate, or the builder
/// for a retry.
///
/// Once this side has received its whole identity it has joined the network,
/// however the session then ends. [`Joined`](Self::Joined) and
/// [`Unbookmarked`](Self::Unbookmarked) carry the new peer beside a
/// [`Connection`] that says whether the connection can carry another session.
/// Without an identity, [`Bailed`](Self::Bailed) completed the session and
/// returns the connection's halves, resting at the session boundary, and
/// [`Failed`](Self::Failed) returns only the builder.
#[must_use = "Joined contains a peer or a builder needed for retry"]
pub enum Joined<T: Send + Sync + 'static, R, W, B: Bookmark = NoBookmark> {
    /// This side joined the network, and any selected bookmark was attached and
    /// persisted.
    Joined {
        /// The new peer, carrying the selected configuration and storage.
        peer: Peer<T, B>,
        /// Whether the connection can carry another session.
        connection: Connection<R, W>,
    },
    /// Both endpoints were bootstrapping; neither could provide a gossip
    /// network. Storage was not touched.
    Bailed {
        /// The complete builder, ready to try another provider.
        bootstrap: Bootstrap<T, B>,
        /// The connection's read half, ready for another session.
        read: R,
        /// The connection's write half, ready for another session.
        write: W,
    },
    /// This side joined the network, but attaching the selected bookmark
    /// failed. The returned peer is live and unbookmarked; retry its bookmark
    /// attachment.
    Unbookmarked {
        /// The joined peer and the attachment's failure.
        unbookmarked: Unbookmarked<T, B>,
        /// Whether the connection can carry another session.
        connection: Connection<R, W>,
    },
    /// The session failed, and this side received no whole identity. Storage
    /// was not touched; discard the connection.
    Failed {
        /// The session error.
        error: Error,
        /// The complete builder, ready to retry on another connection.
        bootstrap: Bootstrap<T, B>,
    },
}

/// Format join outcomes without requiring debuggable payload, bookmark, or
/// transport types.
impl<T: Send + Sync + 'static, R, W, B: Bookmark> fmt::Debug for Joined<T, R, W, B> {
    /// Formats the outcome and the peer or retry state it carries.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Joined { peer, connection } => f
                .debug_struct("Joined")
                .field("peer", peer)
                .field("connection", connection)
                .finish(),
            Self::Bailed { bootstrap, .. } => f
                .debug_struct("Bailed")
                .field("bootstrap", bootstrap)
                .finish_non_exhaustive(),
            Self::Unbookmarked {
                unbookmarked,
                connection,
            } => f
                .debug_struct("Unbookmarked")
                .field("unbookmarked", unbookmarked)
                .field("connection", connection)
                .finish(),
            Self::Failed { error, bootstrap } => f
                .debug_struct("Failed")
                .field("error", error)
                .field("bootstrap", bootstrap)
                .finish(),
        }
    }
}

#[cfg(test)]
mod tests;
