//! Configure a bootstrap session and retain its settings when a retry is needed.

use std::marker::PhantomData;
use std::sync::Arc;

use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::bookmark::{Bookmark, BookmarkError, NoBookmark};
use crate::link::{Acceptor, Connector, Link};
use crate::message::PayloadDepthLimit;
use crate::observe::{Attachment, Observer};
use crate::tree::mirror::streaming::remote::RunBudget;
use crate::tree::mirror::streaming::window::WindowConfig;
use crate::{Error, Peer};

use super::gossip::Unbookmarked;

/// Configuration for joining a gossip network through [`Peer::bootstrap`].
///
/// [`join`](Self::join) joins the connected peer's gossip network and receives
/// its current message set. The new peer retains every setting selected here.
///
/// A failed session or a meeting between two bootstrappers returns this builder
/// in [`Joined`], including any selected bookmark. Retry by calling `join` on
/// the returned builder with another link. No configuration or storage clone
/// is needed. Builders without a bookmark also implement [`Clone`].
///
/// Select [`bookmark`](Self::bookmark) to attach restart bookkeeping before
/// returning the joined peer. A storage failure after joining instead
/// returns the live, unbookmarked peer through [`Joined::Unbookmarked`].
///
/// # Serving a bootstrap
///
/// An established peer automatically serves bootstrappers through ordinary
/// [`gossip`](crate::Rumors::gossip); no special invocation is required.
#[must_use = "a Bootstrap does nothing until join runs it against a link"]
pub struct Bootstrap<T, B: BookmarkError = NoBookmark> {
    /// Pipelining policy inherited by the joined peer.
    pub(crate) window: WindowConfig,
    /// Supply-run size target used during and after the join.
    pub(crate) run_budget: RunBudget,
    /// Initiation policy and session deadline, both inherited by the joined peer.
    pub(crate) gossip_policy: super::policy::Policy<T>,
    /// Payload nesting limit used during and after the join.
    pub(crate) payload_depth_limit: PayloadDepthLimit,
    /// Observation handlers retained by the joined peer.
    pub(crate) observe: Attachment,
    /// Storage owned by this attempt, returned if no peer arrives.
    bookmark: B,
    /// Marks the payload type without storing a value or constraining auto traits.
    marker: PhantomData<fn() -> T>,
}

/// Copy an unbookmarked builder without requiring the payload type to be Clone.
impl<T> Clone for Bootstrap<T> {
    /// Copy the settings and share the observation handlers.
    fn clone(&self) -> Self {
        self.session_config()
    }
}

/// Show configuration without Debug bounds on the payload or storage.
impl<T, B: BookmarkError> std::fmt::Debug for Bootstrap<T, B> {
    /// Describe the settings without reading the bookmark.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Bootstrap")
            .field("window", &self.window)
            .field("run_budget", &self.run_budget)
            .field("payload_depth_limit", &self.payload_depth_limit)
            .field("bookmark", &std::any::type_name::<B>())
            .finish()
    }
}

/// Construct an unbookmarked builder or select its storage.
impl<T> Bootstrap<T> {
    /// Use the default session settings without a bookmark.
    pub(crate) fn new() -> Self {
        Self {
            window: WindowConfig::default(),
            run_budget: RunBudget::default(),
            gossip_policy: super::policy::Policy::default(),
            payload_depth_limit: PayloadDepthLimit::default(),
            observe: Attachment::default(),
            bookmark: NoBookmark,
            marker: PhantomData,
        }
    }

    /// Attach a bookmark before returning the joined peer.
    ///
    /// Reuse the bookmark across restarts to limit version growth. It stores
    /// restart bookkeeping, not messages; the join receives those from the
    /// provider. See [`Bookmark`] for storage and ownership requirements.
    ///
    /// This calls [`Peer::bookmark`] after the session. If attachment fails,
    /// [`Joined::Unbookmarked`] returns the live peer and the storage error.
    /// If the session fails or bails, the returned builder owns the untouched
    /// bookmark and can retry.
    pub fn bookmark<B: Bookmark>(self, bookmark: B) -> Bootstrap<T, B> {
        Bootstrap {
            window: self.window,
            run_budget: self.run_budget,
            gossip_policy: self.gossip_policy,
            payload_depth_limit: self.payload_depth_limit,
            observe: self.observe,
            bookmark,
            marker: PhantomData,
        }
    }
}

/// Configure the session independently of whether storage has been selected.
impl<T, B: BookmarkError> Bootstrap<T, B> {
    /// Copy only session settings; storage stays with the retryable builder.
    fn session_config(&self) -> Bootstrap<T> {
        Bootstrap {
            window: self.window,
            run_budget: self.run_budget,
            gossip_policy: self.gossip_policy.clone(),
            payload_depth_limit: self.payload_depth_limit,
            observe: self.observe.clone(),
            bookmark: NoBookmark,
            marker: PhantomData,
        }
    }

    /// Select the joined peer's initiation policy; see [`Peer::gossip_when`].
    pub fn gossip_when<F, S>(mut self, when: F) -> Self
    where
        F: Fn(crate::Changes<T>) -> S + Send + Sync + 'static,
        S: futures::Stream + Send + 'static,
        S::Item: Into<crate::Gossip>,
    {
        self.gossip_policy.set_when(when);
        self
    }

    /// Set the join's session deadline, also inherited by the joined peer.
    /// See [`Peer::session_deadline`] for timing and expiry behavior.
    /// Bookmark attachment after the wire session is not covered.
    pub fn session_deadline<D, F>(mut self, deadline: D) -> Self
    where
        D: Fn() -> F + Send + Sync + 'static,
        F: Future<Output = ()> + Send + 'static,
    {
        self.gossip_policy.set_deadline(deadline);
        self
    }

    /// Set the joined peer's pipelining memory budget.
    ///
    /// Bootstrap receives into an empty replica and has no disputed subtrees
    /// to pipeline. This budget takes effect on its later synchronizations;
    /// [`target_message_size`](Self::target_message_size) bounds bootstrap's
    /// supply runs. Defaults and sizing guidance are in [`Peer::sync_memory_budget`].
    pub fn sync_memory_budget(mut self, budget_bytes: usize) -> Self {
        self.window = WindowConfig::Budget(budget_bytes);
        self
    }

    /// Set the batched-message size target during and after bootstrap.
    ///
    /// The provider uses the smaller of its target and ours. Zero sends one
    /// leaf per message. See [`Peer::target_message_size`] for defaults and
    /// what the target accounts for.
    pub fn target_message_size(mut self, bytes: usize) -> Self {
        self.run_budget = RunBudget::from_bytes(bytes);
        self
    }

    /// Set the payload nesting limit during and after bootstrap.
    ///
    /// See [`Peer::payload_depth_limit`] for defaults and fleet coordination.
    pub fn payload_depth_limit(mut self, limit: PayloadDepthLimit) -> Self {
        self.payload_depth_limit = limit;
        self
    }

    /// Observe this bootstrap session and the joined peer's later sessions.
    ///
    /// The handler also survives failed attempts in the returned builder.
    /// See [`observe`](crate::observe) for the observation contract.
    pub fn observe(mut self, observer: Arc<dyn Observer>) -> Self {
        self.observe.attach(observer);
        self
    }
}

/// Run a configured bootstrap and attach its selected bookmark.
impl<T, B: Bookmark> Bootstrap<T, B> {
    /// Join the connected peer's gossip network, returning a peer or a retryable builder.
    ///
    /// Success receives the provider's current message set. The provider may
    /// be gossiping or retiring. Two bootstrappers cannot supply each other;
    /// both return [`Joined::Bailed`] with their builders.
    ///
    /// [`Joined::Failed`] returns the builder on session failure, including
    /// [`session_deadline`](Self::session_deadline) expiry. Discard the
    /// poisoned link and retry on another.
    ///
    /// If a bookmark was selected, joining then attaches it through
    /// [`Peer::bookmark`]. A failed attachment returns [`Joined::Unbookmarked`],
    /// preserving the joined peer.
    ///
    /// Cancelling drops the builder and any joined peer. Cancellation during
    /// the session poisons the link; once bookmark attachment begins, the
    /// session is complete and the link remains usable. See the
    /// [session contract](crate::link::Link#what-a-session-promises).
    pub async fn join<CR, CW, C, A>(self, link: &mut Link<CR, CW, C, A>) -> Joined<T, B>
    where
        T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
        CR: AsyncRead + Unpin + Send,
        CW: AsyncWrite + Unpin + Send,
        C: Connector,
        A: Acceptor,
    {
        // The session needs a copy of the settings, not ownership of storage.
        // Retaining self lets both unsuccessful outcomes return it unchanged.
        match Peer::bootstrap_inner(self.session_config(), link).await {
            Ok(Some(peer)) => match peer.bookmark(self.bookmark).await {
                Ok(peer) => Joined::Joined { peer },
                Err(unbookmarked) => Joined::Unbookmarked(unbookmarked),
            },
            Ok(None) => Joined::Bailed { bootstrap: self },
            Err(error) => Joined::Failed {
                error,
                bootstrap: self,
            },
        }
    }
}

/// A bootstrap's outcome, preserving either its new peer or its retry configuration.
#[must_use = "Joined contains a peer or a builder needed for retry"]
#[derive(Debug)]
pub enum Joined<T, B: BookmarkError = NoBookmark> {
    /// The session succeeded and any selected bookmark was attached and persisted.
    /// The link remains usable.
    Joined {
        /// The new peer, carrying the selected configuration and storage.
        peer: Peer<T, B>,
    },
    /// Both endpoints were bootstrapping; neither could provide a gossip network.
    /// The link remains usable and storage was not touched.
    Bailed {
        /// The complete builder, ready to try another provider.
        bootstrap: Bootstrap<T, B>,
    },
    /// The session succeeded, but attaching the bookmark failed.
    /// The returned peer is live and unbookmarked; retry its bookmark attachment.
    /// The link remains usable.
    Unbookmarked(Unbookmarked<T, B>),
    /// The session failed and poisoned the link. Storage was not touched.
    Failed {
        /// The session error.
        error: Error,
        /// The complete builder, ready to retry on another link.
        bootstrap: Bootstrap<T, B>,
    },
}

#[cfg(test)]
mod tests;
