use crate::tree::RangeOwned;
use crate::{Version, causally};
use futures::Stream;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::sync::watch;

use super::channel::Channel;

/// An observer of messages sent to a [`Rumors`](crate::Rumors), in completely
/// arbitrary (*non-causal*) order.
///
/// This enumerates every message not causally contained in the starting
/// checkpoint, then every message learned afterwards: by local
/// [`send`](crate::Rumors::send), by gossip, through any handle. Once the
/// [`Peer`](crate::Peer) and every [`Rumors`](crate::Rumors) have dropped
/// and no further change is possible, it yields whatever remains and ends
/// with `None`.
///
/// Each message arrives as an owned `(Version, Arc<T>)` — both handles are
/// cheap reference bumps into shared storage — through either face:
///
/// - the [`Stream`] impl, awaiting quietly while the set is unchanged;
/// - [`try_next`](Self::try_next), one non-blocking step at a time.
///
/// Order is unspecified and does *not* follow the causal order: a message may
/// be yielded before another that causally precedes it; use
/// [`CausalMessages`](super::CausalMessages) if you want causal iteration order
/// (at an amortized logarithmic cost in extra internal bookkeeping).
///
/// This observer does not count against the quiescence that lets
/// [`try_into_peer`](crate::Rumors::try_into_peer) reclaim the
/// [`Peer`](crate::Peer).
pub struct UnorderedMessages<T> {
    /// The shared replica receiver and its current wait state.
    channel: Channel<T>,
    /// The frontier covered by every completed pass.
    checkpoint: Version,
    /// The snapshot currently being delivered, if any.
    pass: Option<Pass>,
}

/// The outcome of [`UnorderedMessages::try_next`] or [`CausalMessages::try_next`].
///
/// A non-blocking step that either yields a message or says why it can't.
///
/// [`CausalMessages::try_next`]: super::CausalMessages::try_next
#[derive(Debug)]
pub enum TryNext<T> {
    /// A message was ready: the same owned `(Version, Arc<T>)` pair the
    /// [`Stream`] face yields.
    Message((Version, Arc<T>)),
    /// No message is ready yet, but handles are still live: ask again later.
    Quiet,
    /// Every handle is gone and no further message is possible.
    Ended,
}

/// One in-progress pass: the frozen walk over its snapshot, and the
/// snapshot's ceiling to absorb into the checkpoint when the walk drains.
struct Pass {
    /// The remaining messages beyond the pass's starting checkpoint.
    walk: RangeOwned<causally::Down>,
    /// The frontier earned once `walk` is fully delivered.
    ceiling: Version,
}

/// Creates passes and exposes their completed frontier.
impl<T> UnorderedMessages<T> {
    /// Observes messages beyond `since`, starting from the current snapshot.
    pub(crate) fn subscribe(inner: &watch::Sender<crate::Inner<T>>, since: Version) -> Self {
        Self {
            channel: Channel::subscribe(inner),
            checkpoint: since,
            pass: None,
        }
    }

    /// Open a pass over the latest snapshot if none is in progress. The
    /// watch read guard lives only long enough to freeze the walk (a root
    /// handle clone) and capture the ceiling.
    fn open_pass(
        pass: &mut Option<Pass>,
        rx: &mut watch::Receiver<crate::Inner<T>>,
        checkpoint: &Version,
    ) where
        T: Send + Sync,
    {
        if pass.is_none() {
            let inner = rx.borrow_and_update();
            *pass = Some(Pass {
                walk: inner.tree.range_owned(causally::since(checkpoint.clone())),
                ceiling: inner.tree.latest().clone(),
            });
        }
    }

    /// The sound resume point: the causal frontier of the last *completed*
    /// pass, suitable for persisting across processes or handing to another
    /// replica of the same network.
    ///
    /// Resuming from this checkpoint will never skip messages, but it may
    /// replay an arbitrary number of them.
    ///
    /// Folding the yielded versions yourself is not a substitute: the
    /// causal order is partial, not total, so "the last version I saw" is
    /// not well-defined, and such a fold is not a causally closed
    /// boundary: resuming from it could skip messages. This checkpoint
    /// moves only after a complete pass.
    ///
    /// After the observer ends (`None`), this is the complete final
    /// frontier. To merely pause in-process, just hold the observer: its
    /// idle state is constant-size, and the checkpoint stays inside it.
    ///
    /// # Examples
    ///
    /// ```
    /// use futures::{FutureExt, StreamExt};
    /// use rumors::{Peer, Version};
    ///
    /// # tokio::runtime::Builder::new_current_thread()
    /// #     .build()
    /// #     .unwrap()
    /// #     .block_on(async {
    /// let rumors = Peer::<String>::seed().into_rumors();
    /// rumors.send("one".to_string());
    ///
    /// let mut observer = rumors.unordered_messages();
    /// let (_version, m) = observer.next().await.expect("one message");
    /// assert_eq!(m.as_str(), "one");
    ///
    /// // Mid-pass, the checkpoint has not moved: resuming here would
    /// // re-deliver "one" (a partial pass is not a causally closed boundary).
    /// assert_eq!(observer.checkpoint(), &Version::new());
    ///
    /// // One more step finds nothing ready, completing the pass and
    /// // absorbing its frontier into the checkpoint.
    /// assert!(observer.next().now_or_never().is_none());
    /// let checkpoint = observer.checkpoint().clone();
    ///
    /// // A resume from it re-observes nothing from the completed pass and
    /// // everything not yet delivered.
    /// rumors.send("two".to_string());
    /// let mut resumed = rumors.unordered_messages_since(checkpoint);
    /// let (_version, m) = resumed.next().await.expect("only the new message");
    /// assert_eq!(m.as_str(), "two");
    /// # });
    /// ```
    pub fn checkpoint(&self) -> &Version {
        &self.checkpoint
    }
}

/// Provides one-step, non-blocking observation.
impl<T: Send + Sync + 'static> UnorderedMessages<T> {
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

/// Yields owned `(Version, Arc<T>)` pairs: cheap handles into the shared
/// storage (the version's buffer and the message's allocation are shared,
/// not copied).
///
/// `T: 'static` because the quiet-period wait is materialized as an owned
/// future.
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
