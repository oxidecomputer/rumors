use std::collections::BTreeMap;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use before::Rank;
use futures::Stream;
use tokio::sync::watch;

use crate::tree::Leaf;
use crate::{Version, causally};

use super::unordered::{Channel, TryNext};

/// An observer of messages sent to a [`Rumors`](crate::Rumors), in some
/// arbitrary yet causal order.
///
/// For any two yielded messages with versions `v` and `w`, if `v < w` then the
/// `v` message is yielded first. Concurrent messages are delivered in arbitrary
/// order, which may differ between [`gossip`](crate::Rumors::gossip)ing
/// replicas of the same [`Rumors`](crate::Rumors).
///
/// Each message arrives as an owned `(Version, Arc<T>)` — both handles are
/// cheap reference bumps into shared storage — through the [`Stream`] impl
/// or [`try_next`](Self::try_next), exactly as for
/// [`UnorderedMessages`](super::UnorderedMessages).
///
/// Unlike [`UnorderedMessages`](super::UnorderedMessages), this costs an
/// extra amortized factor, logarithmic in the number of messages the set
/// holds, in memory and in the time to retrieve each message, and both
/// costs may burst arbitrarily large, up to the total size of the messages
/// stored in the underlying [`Rumors`](crate::Rumors).
///
/// This observer does not count against the quiescence that lets
/// [`try_into_peer`](crate::Rumors::try_into_peer) reclaim the
/// [`Peer`](crate::Peer).
pub struct CausalMessages<T> {
    /// The watch channel or the in-flight wait for it to change: the same
    /// owned-wait dance as [`UnorderedMessages`](super::UnorderedMessages) (see its field
    /// docs for why the wait is materialized).
    channel: Option<Channel<T>>,
    /// The ingest frontier: the causal past already staged (or delivered).
    /// The next pass walks leaves *not* contained here. Advances at ingest,
    /// so it runs ahead of delivery while the backlog drains.
    ingested: Version,
    /// The public resume point: [`checkpoint`](Self::checkpoint).
    ///
    /// Trails [`ingested`](Self::ingested), catching up on the call *after*
    /// the staged backlog drains — never in the step that hands over the
    /// backlog's last message — so that resuming from it skips neither a
    /// staged, undelivered message nor the delivered message still in the
    /// caller's hands.
    checkpoint: Version,
    /// The undelivered backlog in rank-then-canonical-bytes order.
    ///
    /// The same total order as [`before::Ranked`], with the [`Rank`]
    /// materialized once per leaf so repeated map comparisons stay cheap.
    /// Rank extends the causal order and the byte tiebreak fires only
    /// between concurrent messages, so delivery order is causal and
    /// deterministic. Always the residue of a *single* ingest (a new pass
    /// opens only once this empties), whose range start was `checkpoint`
    /// and whose ceiling is `ingested`.
    staged: BTreeMap<(Rank, Vec<u8>), Leaf<T>>,
}

impl<T> CausalMessages<T> {
    pub(crate) fn subscribe(inner: &watch::Sender<crate::Inner<T>>, since: Version) -> Self {
        Self {
            channel: Some(Channel::Ready(inner.subscribe())),
            ingested: since.clone(),
            checkpoint: since,
            staged: BTreeMap::new(),
        }
    }

    /// Ingest one whole pass over the latest snapshot: stage every live
    /// leaf not causally contained in the ingest frontier, keyed by its
    /// causal rank, then absorb the snapshot's ceiling into the frontier.
    ///
    /// Eager where [`UnorderedMessages`](super::UnorderedMessages) is lazy, by necessity: a
    /// pass arrives in key order, so any leaf might causally precede one
    /// staged earlier, and nothing can be delivered until the pass is
    /// complete. The watch read guard lives only long enough to freeze the
    /// walk and capture the ceiling; the walk itself runs unlocked.
    fn ingest(
        staged: &mut BTreeMap<(Rank, Vec<u8>), Leaf<T>>,
        ingested: &mut Version,
        rx: &mut watch::Receiver<crate::Inner<T>>,
    ) where
        T: Send + Sync,
    {
        let (mut walk, ceiling) = {
            let inner = rx.borrow_and_update();
            (
                inner.tree.range_owned(causally::since(ingested.clone())),
                inner.tree.latest().clone(),
            )
        };
        while let Some((_, leaf)) = walk.next() {
            let version = leaf.version();
            staged.insert((version.rank(), version.as_bytes().to_vec()), leaf);
        }
        *ingested |= &ceiling;
    }

    /// The sound resume point: the causal frontier *behind* any internally
    /// staged backlog, suitable for persisting across processes or handing to
    /// another replica of the same network.
    ///
    /// Resuming from this [`Version`] will never skip messages, but it may
    /// replay an arbitrary number of them. It covers a yielded message
    /// only from the *following* call onward, so a checkpoint persisted
    /// after handling each message replays — never skips — the message in
    /// flight at a crash.
    ///
    /// Folding the yielded versions yourself is not a substitute: the
    /// causal order is partial, not total, so "the last version I saw" is
    /// not well-defined, and such a fold is not a causally closed
    /// boundary.
    ///
    /// After the observer ends (`None`), this is the final [`Version`] of the
    /// [`Rumors`](crate::Rumors).
    pub fn checkpoint(&self) -> &Version {
        &self.checkpoint
    }
}

impl<T: Send + Sync + 'static> CausalMessages<T> {
    /// Take one non-blocking step: a message if one is ready, [`Quiet`] (ask
    /// again later) if not, [`Ended`] if no further message is possible.
    ///
    /// One [`Stream`] poll with a no-op waker, rendered as the trichotomy.
    ///
    /// [`Quiet`]: TryNext::Quiet
    /// [`Ended`]: TryNext::Ended
    pub fn try_next(&mut self) -> TryNext<T> {
        use futures::{FutureExt, StreamExt};
        match self.next().now_or_never() {
            None => TryNext::Quiet,
            Some(None) => TryNext::Ended,
            Some(Some(message)) => TryNext::Message(message),
        }
    }
}

/// Yields owned `(Version, Arc<T>)` pairs popped from the staged backlog
/// in causal order: cheap handles into the shared storage.
///
/// `T: 'static` because the quiet-period wait is materialized as an
/// owned future, exactly as in [`UnorderedMessages`](super::UnorderedMessages).
impl<T: Send + Sync + 'static> Stream for CausalMessages<T> {
    type Item = (Version, Arc<T>);

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        loop {
            // The resume point stays put even when this pop empties the
            // backlog: the yielded message is unhandled until the stream
            // is polled again, so the catch-up defers to the next poll's
            // ingest, exactly as UnorderedMessages defers a drained pass's
            // ceiling.
            if let Some((_, leaf)) = this.staged.pop_first() {
                return Poll::Ready(Some((leaf.version().clone(), leaf.value())));
            }
            match this.channel.as_mut().expect("channel state present") {
                Channel::Waiting(wait) => match wait.as_mut().poll(cx) {
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready((closed, rx)) => {
                        this.channel = Some(Channel::Ready(rx));
                        if closed {
                            return Poll::Ready(None);
                        }
                    }
                },
                Channel::Ready(rx) => {
                    // The backlog is empty here (the pop above returns
                    // otherwise): the previous pass is fully yielded, so
                    // the deferred catch-up runs before the next pass
                    // opens against the caught-up boundary.
                    this.checkpoint = this.ingested.clone();
                    Self::ingest(&mut this.staged, &mut this.ingested, rx);
                    if this.staged.is_empty() {
                        // Nothing new: catch the resume point up and enter
                        // the owned wait (the receiver rides inside the
                        // future and comes back with the result).
                        this.checkpoint = this.ingested.clone();
                        let Some(Channel::Ready(mut rx)) = this.channel.take() else {
                            unreachable!("matched Ready above");
                        };
                        this.channel = Some(Channel::Waiting(Box::pin(async move {
                            let closed = rx.changed().await.is_err();
                            (closed, rx)
                        })));
                    }
                }
            }
        }
    }
}
