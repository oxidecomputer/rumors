use crate::tree::RangeOwned;
use crate::{Inner, Version, causally};
use futures::Stream;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::sync::watch;

use super::channel::Channel;

/// A stream yielding messages from a [`Rumors`](crate::Rumors), in completely
/// arbitrary (*non-causal*) order.
///
/// This stream yields items of type `(Version, Arc<T>)`.
///
/// Order is unspecified and does *not* follow the causal order: a message may
/// be yielded before another that causally precedes it. Use
/// [`CausalMessages`](super::CausalMessages) if you want causal iteration
/// order.
///
/// Holding this stream open does not prevent
/// [`try_into_peer`](crate::Rumors::try_into_peer) from reclaiming the
/// [`Peer`](crate::Peer).
pub struct UnorderedMessages<T: Send + Sync + 'static> {
    /// The shared replica receiver and its current wait state.
    channel: Channel<T>,
    /// The frontier covered by every completed pass.
    checkpoint: Version,
    /// The snapshot currently being delivered, if any.
    pass: Option<Pass>,
}

/// Debug the stream without requiring its payloads to be debuggable.
impl<T: Send + Sync + 'static> std::fmt::Debug for UnorderedMessages<T> {
    /// Format the stream's completed frontier and whether a snapshot is being
    /// delivered.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UnorderedMessages")
            .field("checkpoint", &self.checkpoint)
            .field("delivering", &self.pass.is_some())
            .finish_non_exhaustive()
    }
}

/// The result of [`UnorderedMessages::try_next`] or
/// [`CausalMessages::try_next`].
///
/// This is a non-blocking outcome that either yields a message or describes why
/// it can't.
///
/// [`CausalMessages::try_next`]: super::CausalMessages::try_next
#[derive(Debug)]
pub enum TryNext<T> {
    /// A message is ready: a version and an `Arc<T>` message.
    Message((Version, Arc<T>)),
    /// No message is ready yet, but it would be possible for one to arrive
    /// later, because some handles still exist that could send one.
    Quiet,
    /// Every handle that could cause a new message to arrive has been dropped,
    /// so no further messages are possible.
    Ended,
}

/// One in-progress pass: the frozen walk over its snapshot, and the snapshot's
/// ceiling to absorb into the checkpoint when the walk drains.
struct Pass {
    /// The remaining messages beyond the pass's starting checkpoint.
    walk: RangeOwned<causally::Down>,
    /// The frontier earned once `walk` is fully delivered.
    ceiling: Version,
}

/// Subscribe to messages, and query the safe resume point.
impl<T: Send + Sync + 'static> UnorderedMessages<T> {
    /// Observes messages beyond `since`, starting from the current snapshot.
    pub(crate) fn subscribe(inner: &watch::Sender<Inner<T>>, since: Version) -> Self {
        Self {
            channel: Channel::subscribe(inner),
            checkpoint: since,
            pass: None,
        }
    }

    /// Open a pass over the latest snapshot if none is in progress. The watch
    /// read guard lives only long enough to freeze the walk (a root handle
    /// clone) and capture the ceiling.
    fn open_pass(
        pass: &mut Option<Pass>,
        rx: &mut watch::Receiver<Inner<T>>,
        checkpoint: &Version,
    ) {
        if pass.is_none() {
            let inner = rx.borrow_and_update();
            *pass = Some(Pass {
                walk: inner.tree.range_owned(causally::since(checkpoint.clone())),
                ceiling: inner.tree.latest().clone(),
            });
        }
    }

    /// The latest causal resumption point.
    ///
    /// The checkpoint is the causal frontier *behind* any internally-staged
    /// backlog. Resuming from this [`Version`] will never skip messages, but it
    /// may replay any number of them.
    ///
    /// After the stream ends (i.e. yields `None`), this is the final
    /// [`Version`] of the underlying [`Rumors`](crate::Rumors).
    pub fn checkpoint(&self) -> &Version {
        &self.checkpoint
    }
}

/// Provides non-blocking observation.
impl<T: Send + Sync + 'static> UnorderedMessages<T> {
    /// Return one stream item without blocking, if one is ready.
    ///
    /// This returns a versioned message if one is ready, [`Quiet`] (ask again
    /// later) if not, or [`Ended`] if no further message is possible.
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

/// Yields owned `(Version, Arc<T>)` pairs.
impl<T: Send + Sync + 'static> Stream for UnorderedMessages<T> {
    type Item = (Version, Arc<T>);

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        loop {
            let Some(receiver) = std::task::ready!(this.channel.poll_receiver(cx)) else {
                return Poll::Ready(None);
            };
            Self::open_pass(&mut this.pass, receiver, &this.checkpoint);

            let pass = this.pass.as_mut().expect("opened above");
            if let Some((_, leaf)) = pass.walk.next() {
                return Poll::Ready(Some((leaf.version().clone(), leaf.value::<T>())));
            }

            // A checkpoint advances only once its whole snapshot has been
            // delivered. With no current message left, wait for a new state.
            let Pass { ceiling, .. } = this.pass.take().expect("opened above");
            this.checkpoint |= &ceiling;
            this.channel.wait();
        }
    }
}
