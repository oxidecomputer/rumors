//! Configure a join session and retain its settings when a retry is
//! needed.

use std::{marker::PhantomData, ops::RangeBounds, sync::Arc};

#[cfg(any(test, feature = "test-internals"))]
use rand::CryptoRng;
use rand::rand_core::UnwrapErr;
use rand::rngs::SysRng;
use serde::{Serialize, de::DeserializeOwned};
use tokio::io::{AsyncBufRead, AsyncWrite};

use super::Replica;
use crate::Peer;
use crate::bookmark::{Bookmark, BookmarkHandle, MaybeBookmark, NoBookmark, Token};
use crate::message::MessageDepthLimit;
use crate::network::NetworkSecret;
use crate::observe::Observer;
use crate::tree::mirror::streaming::remote::{RunBudget, WriteBatch};

use super::bookmark::Unbookmarked;
use super::connection::Connection;
use super::join::Join;
use super::{MemoryBudget, SessionSettings, Settings};

/// The configuration to join a gossip network via [`Peer::builder`].
///
/// This is a builder-style API, where [`join`](Self::join) finishes
/// configuration and joins the connected peer's gossip network, returning a
/// [`Peer`]. The newly created peer retains every setting selected
/// in the builder.
///
/// Any failure returns the builder itself within [`Join`]. Retry by calling
/// `join` on the returned builder, perhaps on another connection.
#[must_use = "a `PeerBuilder` does nothing until join runs it over a connection"]
pub struct PeerBuilder<T: Send + Sync + 'static, B = NoBookmark> {
    /// The session settings used during the join and inherited whole by the
    /// joined peer.
    pub(crate) settings: SessionSettings<T>,
    /// Message nesting limit used during and after the join.
    pub(crate) message_depth_limit: MessageDepthLimit,
    /// Storage owned by this attempt, returned if no peer arrives.
    bookmark: B,
    /// Maximum bookmark size inherited by the joined peer.
    pub(crate) bookmark_size_limit: usize,
    /// The network secret drawn by
    /// [`dangerously_override_rng`](Self::dangerously_override_rng), which
    /// [`seed`](PeerBuilder::seed) uses in place of one drawn from the
    /// operating system.
    #[cfg(any(test, feature = "test-internals"))]
    secret: Option<NetworkSecret>,
    /// Marks the payload type without storing a value or constraining auto
    /// traits.
    marker: PhantomData<fn() -> T>,
}

/// Copy a builder without requiring the payload type to be Clone.
impl<T: Send + Sync + 'static> Clone for PeerBuilder<T> {
    /// Copy its settings and share its observation handlers.
    fn clone(&self) -> Self {
        Self {
            #[cfg(any(test, feature = "test-internals"))]
            secret: self.secret.clone(),
            ..self.session_config()
        }
    }
}

/// Show the configuration without `Debug` bounds on the messages or storage.
impl<T: Send + Sync + 'static, B> std::fmt::Debug for PeerBuilder<T, B> {
    /// Describe the builder's settings without reading the bookmark.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sizing = self.settings.sizing;
        let limits = sizing.limits();
        f.debug_struct("PeerBuilder")
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

impl<T: Send + Sync + 'static> PeerBuilder<T> {
    /// The default [`PeerBuilder`].
    pub(crate) fn new() -> Self {
        Self {
            settings: SessionSettings::default(),
            message_depth_limit: MessageDepthLimit::default(),
            bookmark: NoBookmark,
            bookmark_size_limit: crate::default::BOOKMARK_SIZE_LIMIT,
            #[cfg(any(test, feature = "test-internals"))]
            secret: None,
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
    pub fn bookmark<N: Bookmark>(self, bookmark: N) -> PeerBuilder<T, N> {
        self.with_state(bookmark)
    }
}

impl<T: Send + Sync + 'static, B> PeerBuilder<T, B> {
    /// Return the settings as they stand, with the values a peer created from
    /// this builder would report from [`Peer::settings`].
    pub fn settings(&self) -> Settings {
        Settings::effective(self.settings.sizing, self.message_depth_limit)
    }

    /// Copy only session settings; storage stays with the retryable builder.
    fn session_config(&self) -> PeerBuilder<T> {
        PeerBuilder {
            settings: self.settings.clone(),
            message_depth_limit: self.message_depth_limit,
            bookmark: NoBookmark,
            bookmark_size_limit: self.bookmark_size_limit,
            #[cfg(any(test, feature = "test-internals"))]
            secret: None,
            marker: PhantomData,
        }
    }

    /// Separate the selected state from everything else the builder holds.
    fn into_parts(self) -> (PeerBuilder<T>, B) {
        let Self {
            settings,
            message_depth_limit,
            bookmark,
            bookmark_size_limit,
            #[cfg(any(test, feature = "test-internals"))]
            secret,
            marker,
        } = self;
        let builder = PeerBuilder {
            settings,
            message_depth_limit,
            bookmark: NoBookmark,
            bookmark_size_limit,
            #[cfg(any(test, feature = "test-internals"))]
            secret,
            marker,
        };
        (builder, bookmark)
    }

    /// Return a [`PeerBuilder`] in a new state, carrying every setting forward.
    fn with_state<N>(self, state: N) -> PeerBuilder<T, N> {
        let (builder, _) = self.into_parts();
        let PeerBuilder {
            settings,
            message_depth_limit,
            bookmark: NoBookmark,
            bookmark_size_limit,
            #[cfg(any(test, feature = "test-internals"))]
            secret,
            marker,
        } = builder;
        PeerBuilder {
            settings,
            message_depth_limit,
            bookmark: state,
            bookmark_size_limit,
            #[cfg(any(test, feature = "test-internals"))]
            secret,
            marker,
        }
    }

    /// Set the policy for when each [`Peer`] will initiate
    /// gossip with a peer during [`Peer::gossip`](crate::Peer::gossip) or
    /// [`Peer::gossip_stream`](crate::Peer::gossip_stream).
    ///
    /// By default, gossip will initiate immediately any time the local set of
    /// messages changes.
    ///
    /// You might want to alter this policy if you want to debounce or
    /// rate-limit gossip, or gossip only on an interval or in response to some
    /// external, application-specified event.
    ///
    /// To set this policy, provide a function `when`, which accepts a stream of
    /// [`Changes`](crate::Changes) and returns a stream of
    /// [`Gossip`](crate::Gossip). Your function can adapt that stream, combine
    /// it with another, or ignore it entirely and provide its own policy.
    ///
    /// If you return [`Gossip::WhenChanged`](crate::Gossip::WhenChanged) from
    /// your stream, gossip will only happen if there have been changes since
    /// the last gossip session with this peer; if you return
    /// [`Gossip::Unconditionally`](crate::Gossip::Unconditionally), gossip will
    /// happen regardless of local changes. Note that `(): Into<Gossip>` where
    /// `().into() == Gossip::WhenChanged`, so an ordinary stream of `()` acts
    /// as a sequence of "ticks" which initiate gossip only if there are known
    /// changes to broadcast.
    ///
    /// Two extremes to be aware of:
    ///
    /// - A stream which always pends and never yields an item will only gossip
    ///   if the remote side initiates; if everyone has this policy, nobody will
    ///   ever gossip.
    /// - A stream which immediately yields every time it is polled will cause a
    ///   busy-loop. Don't do this!
    ///
    /// When the stream of [`Gossip`](crate::Gossip) ends, `rumors` will stop
    /// each gossip connection cleanly after any active session. That said, to
    /// end every driver's policy on a shared signal, use
    /// [`gossip_until`](Self::gossip_until) rather than building the signal
    /// into the factory.
    pub fn gossip_when<F, St>(mut self, when: F) -> Self
    where
        F: Fn(crate::Changes<T>) -> St + Send + Sync + 'static,
        St: futures::Stream + Send + 'static,
        St::Item: Into<crate::Gossip>,
    {
        self.settings.gossip_policy.set_when(when);
        self
    }

    /// Impose a timeout on the completion of each gossip session. This is
    /// disabled by default.
    ///
    /// Because Rumors is runtime-independent, the application owns whatever
    /// clock is used for the deadline.
    ///
    /// For example, a one-second deadline with Tokio could be configured by
    /// passing the gossip timeout of:
    /// `|| tokio::time::sleep(Duration::from_secs(1))`.
    pub fn gossip_timeout<D, F>(mut self, timeout: D) -> Self
    where
        D: Fn() -> F + Send + Sync + 'static,
        F: Future<Output = ()> + Send + 'static,
    {
        self.settings.gossip_policy.set_deadline(timeout);
        self
    }

    /// Impose a maximum size limit on a stored [`Bookmark`].
    ///
    /// The default is [`default::BOOKMARK_SIZE_LIMIT`](crate::default::BOOKMARK_SIZE_LIMIT).
    ///
    /// Updates to a bookmark discard the oldest and least recently-used
    /// information once the size of the bookmark hits this capacity.
    pub fn bookmark_size_limit(mut self, bytes: usize) -> Self {
        self.bookmark_size_limit = bytes;
        self
    }

    /// Constrain the memory budget for each synchronization session.
    ///
    /// The default is
    /// [`default::SYNC_MEMORY_BUDGET`](crate::default::SYNC_MEMORY_BUDGET), and
    /// the minimum is about 36.4 MiB.
    ///
    /// This setting is preserved indefinitely throughout the lifecycle of the
    /// [`Peer`], and all [`Peer`] clones inherit it.
    ///
    /// For more guidance on how to configure this parameter, read the [guide to
    /// picking a memory budget](crate::sizing).
    pub fn sync_memory_budget(mut self, budget_bytes: usize) -> Self {
        self.settings.sizing.sync_memory_budget = MemoryBudget::Requested(budget_bytes);
        self
    }

    /// Add a wire [`Observer`] to this peer's gossip
    /// sessions.
    ///
    /// For every session the peer enters, each [`Observer`] is given the
    /// capability to see each directed stream's protocol messages as raw CBOR
    /// items. This can be useful to debug, test, or instrument the `rumors`
    /// protocol itself.
    ///
    /// Calling `observe` again adds another, and observers are invoked in
    /// registration order.
    pub fn observe(mut self, observer: Arc<dyn Observer>) -> Self {
        self.settings.observe.attach(observer);
        self
    }

    /// Tune the target encoded size of the batched messages this peer sends.
    ///
    /// When a peer supplies messages which its counterparty lacks, those
    /// messages are coalesced into batches which are flushed if appending the
    /// next message would push the batch's size past the specified target
    /// `bytes`.
    ///
    /// Adjusting this setting can subtly trade between latency, CPU
    /// utilization, and memory. It is recommended to stick with the
    /// default, [`default::TARGET_MESSAGE_SIZE`](crate::default::TARGET_MESSAGE_SIZE),
    /// unless there is a compelling reason to differ. Any value is safe: zero
    /// degrades to one message per batch, and any limit above the maximum
    /// possible ~4 GiB saturates.
    ///
    /// If two peers configure this differently, they interoperate by
    /// negotiating the minimum of their two target message sizes, so that the
    /// lesser-configured peer receives batches sized according to its own
    /// desires.
    ///
    /// This setting is preserved indefinitely throughout the lifecycle of the
    /// [`Peer`], and all [`Peer`] clones inherit it.
    pub fn target_message_size(mut self, bytes: usize) -> Self {
        self.settings.sizing.run_budget = RunBudget::from_bytes(bytes);
        self
    }

    /// Tune write-batching to optimize for your underlying transport.
    ///
    /// This range should usually run from the "largest smallest frame" to the
    /// "largest single frame" your transport allows, which economizes on system
    /// calls and optimizes the fullness of each individual transport frame.
    ///
    /// The default,
    /// [`default::tls::WRITE_BATCH_RANGE`](crate::default::tls::WRITE_BATCH_RANGE),
    /// is tuned for TLS over TCP on a 1,500-byte MTU Ethernet link.
    ///
    /// This setting is preserved indefinitely throughout the lifecycle of the
    /// [`Peer`], and all [`Peer`] clones inherit it.
    ///
    /// # When and how to change this
    ///
    /// Keeping the default will never cause correctness issues, but in other
    /// contexts, you may see improved performance by changing it.
    ///
    /// By way of example:
    ///
    /// - **Plain TCP** does not have a separate record framing layer.
    ///   Consequently, the write batch range should start at the TCP segment
    ///   payload size, 1,448 bytes, and can end wherever you choose. No record
    ///   limit applies in this setting, so a larger maximum will make fewer
    ///   writes, but may increase latency and memory usage if set exceptionally
    ///   high.
    /// - **IPv6** headers are 20 bytes longer than IPv4's, so subtract 20 bytes
    ///   from the minimum of the range, giving 1,406 bytes as an optimal
    ///   minimum for gossip over TLS.
    ///
    /// - **Larger MTUs**, such as 9,000-byte jumbo frames, should raise the
    ///   minimum by the same amount, for example 8,926 bytes as an optimal
    ///   minimum for gossip over TLS.
    ///
    /// # Settings to (probably) avoid
    ///
    /// - A crossed range like `hi..low` is corrected internally to `low..low`.
    ///   You're probably doing something wrong if you do this.
    /// - An unbounded-below range like `..max` or `0..max` means a "largest
    ///   smallest frame" of zero. This means every batch will try to grow to
    ///   the maximum size, rather than dispatching small batches quickly. This
    ///   usually harms performance!
    /// - A maximum set to zero, as in `..=0` or the empty `..0`, turns batching
    ///   off entirely. Sessions will flush immediately when they find no
    ///   message ready. This usually harms performance!
    /// - An unbounded maximum, or a maximum set above the internal write buffer
    ///   size, saturates at this inner capacity. A batch of small messages
    ///   still will flush when it reaches the minimum size, but any batch that
    ///   accumulates past this will continue to grow until the session has
    ///   nothing more ready to send, only flushing when the internal write
    ///   buffer fills. This usually harms performance!
    pub fn write_batch_range(mut self, range: impl RangeBounds<usize>) -> Self {
        self.settings.sizing.write_batch = WriteBatch::from_range(range);
        self
    }

    /// Bound the depth of the messages this peer sends and accepts.
    ///
    /// A message value is accepted only if decoding its CBOR encoding as the
    /// peer's message type recurs at most `limit` steps. The meaning of such a
    /// "recursion step" is internal to `rumors`, but is guaranteed to remain
    /// stable within a single wire protocol.
    ///
    /// Applying such a limit prevents a peer from "poisoning" the set of
    /// messages with an excessively nested encoded message, which otherwise
    /// could trigger a stack overflow on receipt by other peers.
    ///
    /// This limit is checked by peers both prior to
    /// [`send`](crate::Peer::send), and also during message decoding, so that
    /// a conforming peer cannot even send a too-deep message, and a
    /// non-conforming peer which bypasses the internal check will succeed only
    /// at isolating itself from gossip, since everyone else will reject its bad
    /// message.
    ///
    /// The default is
    /// [`default::MESSAGE_DEPTH_LIMIT`](crate::default::MESSAGE_DEPTH_LIMIT),
    /// which should be ample for almost any type.
    ///
    /// This setting is preserved indefinitely throughout the lifecycle of the
    /// [`Peer`], and all [`Peer`] clones inherit it.
    ///
    /// <div class="warning">
    ///
    /// **This setting must be *exactly the same* for all peers in the gossip
    /// network.** Peers will refuse to gossip with one another if they are
    /// configured with different message depth limits, *even if they have
    /// ostensibly compatible sets of messages.* As a result, this setting
    /// cannot be changed after creating a gossip network.
    ///
    /// </div>
    pub fn message_depth_limit(mut self, limit: u64) -> Self {
        self.message_depth_limit = MessageDepthLimit::new(limit);
        self
    }

    /// Set the budget the peer's sessions' ledgers admit work against to exactly
    /// `bytes`, at whatever message-size target the peer runs at.
    ///
    /// Its sessions run on that budget beside the floor allowance and the session
    /// allowance at the peer's target.
    ///
    /// Unlike a budget set through
    /// [`sync_memory_budget`](Self::sync_memory_budget), which lowers
    /// a target it cannot afford and which a session raises to the minimum for its
    /// target ([`MIN_SYNC_MEMORY_BUDGET`](crate::testing::MIN_SYNC_MEMORY_BUDGET) at the default target), this one
    /// sessions run on as given, at the peer's target as given. So it reaches
    /// ledger budgets below the largest grant, zero included, where sessions
    /// pipeline within each kind and depth's bound alone, which the budget
    /// analyses probe. The serial configuration, one unit of each kind and
    /// depth at a time, is [`serial_budget`](Self::serial_budget)'s.
    #[doc(hidden)]
    #[cfg(any(test, feature = "test-internals"))]
    pub fn ledger_budget(mut self, bytes: usize) -> Self {
        self.settings.sizing.sync_memory_budget = MemoryBudget::Ledger(bytes);
        self
    }

    /// Run the peer's sessions' ledgers in the serial configuration: a budget of zero
    /// with every kind and depth's bound zero.
    ///
    /// Admission then grants a unit only while its kind and depth hold nothing, so
    /// each walk has at most one unit of each kind and depth admitted at a time:
    /// the configuration the liveness argument rests on at its most serial. A
    /// ledger budget of zero alone ([`ledger_budget`](Self::ledger_budget)) still lets each kind and
    /// depth hold up to its bound ([`grant_bound`](crate::testing::grant_bound)).
    #[doc(hidden)]
    #[cfg(any(test, feature = "test-internals"))]
    pub fn serial_budget(mut self) -> Self {
        self.settings.sizing.sync_memory_budget = MemoryBudget::Serial;
        self
    }

    /// Create a network with a caller-supplied random source.
    ///
    /// This test-support hook makes network secrets, and with them network
    /// identifiers, reproducible.
    #[doc(hidden)]
    #[cfg(any(test, feature = "test-internals"))]
    pub fn dangerously_override_rng<R: CryptoRng + ?Sized>(mut self, rng: &mut R) -> Self {
        self.secret = Some(NetworkSecret::from_rng(rng));
        self
    }

    /// The secret a seeded network starts from: the one drawn by
    /// [`dangerously_override_rng`](Self::dangerously_override_rng), if any,
    /// else a fresh draw from the operating system.
    fn take_secret(&mut self) -> NetworkSecret {
        #[cfg(any(test, feature = "test-internals"))]
        if let Some(secret) = self.secret.take() {
            return secret;
        }
        // The secret comes from the operating system on every call, never
        // from a generator whose state lives in this process: a forked
        // child inherits that state, so siblings forked after one shared
        // draw would seed networks with one secret, one identifier and one
        // seed party, and gossip as one network.
        NetworkSecret::from_rng(&mut UnwrapErr(SysRng))
    }
}

/// Create a peer without a bookmark by seeding a network.
impl<T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static> PeerBuilder<T> {
    /// Create a new peer belonging to a fresh gossip network which initially
    /// contains only that peer.
    ///
    /// Independently seeded peers cannot ever gossip with each other, so you
    /// must call this function *once per network*. Subsequently, other peers
    /// should join the same gossip network through [`Peer::join`].
    ///
    /// # Panics
    ///
    /// Panics if the operating system's random number generator fails.
    pub fn seed(mut self) -> Peer<T> {
        let secret = self.take_secret();
        Peer::new(Replica::seeded(self, secret))
    }
}

/// Create a peer carrying the selected bookmark by seeding a network.
impl<T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static, B: Bookmark> PeerBuilder<T, B> {
    /// Create a new peer belonging to a fresh gossip network which initially
    /// contains only that peer.
    ///
    /// Independently seeded peers cannot ever gossip with each other, so you
    /// must call this function *once per network*. Subsequently, other peers
    /// should join the same gossip network through [`Peer::join`].
    ///
    /// # Errors
    ///
    /// Any failure returns the original peer, unchanged and without a bookmark
    /// attached, in the [`Unbookmarked`] error. It is safe to retry attaching
    /// the bookmark in case of transient failures.
    ///
    /// # Panics
    ///
    /// Panics if the operating system's random number generator fails.
    pub async fn seed(self) -> Result<Peer<T, B>, Unbookmarked<T, B>> {
        let (mut builder, bookmark) = self.into_parts();
        let secret = builder.take_secret();
        Replica::seeded(builder, secret)
            .attach_bookmark(bookmark)
            .await
            .map(Peer::new)
    }
}

/// Join the gossip network from a peer already in it, using the configured
/// settings.
impl<T, B> PeerBuilder<T, B>
where
    T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
    B: MaybeBookmark,
{
    /// Join a connected peer's gossip network.
    ///
    /// This returns a constructed [`Peer`] with the settings established on
    /// this builder, or (on error) a retryable copy of the original builder,
    /// annotated with the type of failure that led to this situation.
    pub async fn join<R, W>(self, mut read: R, mut write: W) -> Join<T, R, W, B>
    where
        R: AsyncBufRead + Unpin + Send,
        W: AsyncWrite + Unpin + Send,
    {
        let (builder, bookmark) = self.into_parts();
        let handle = BookmarkHandle::new(
            bookmark.into_storage(Token(())),
            builder.bookmark_size_limit,
        );
        let (peer, completed) =
            Replica::join_inner(builder.session_config(), &mut read, &mut write).await;
        let Some(peer) = peer else {
            let builder = builder.with_state(B::from_storage(handle.into_storage(), Token(())));
            return match completed {
                Ok(()) => Join::Bounced {
                    builder,
                    read,
                    write,
                },
                Err(error) => Join::Failed { error, builder },
            };
        };
        // The peer holds its whole identity whether or not the session
        // completed, so the selected bookmark is attached either way; the
        // connection's fate is reported beside it.
        let connection = Connection::from_session(completed, read, write);
        match peer.attach_handle::<B>(handle).await {
            Ok(peer) => Join::Connected {
                peer: Peer::new(peer),
                connection,
            },
            Err(unbookmarked) => Join::Unbookmarked {
                unbookmarked,
                connection,
            },
        }
    }
}

#[cfg(test)]
mod tests;
