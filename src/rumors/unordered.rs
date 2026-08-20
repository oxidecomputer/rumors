use crate::tree::RangeOwned;
use crate::{Version, causally};
use futures::Stream;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::sync::watch;

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
    /// The watch channel, or the in-flight wait for it to change.
    ///
    /// The wait future owns the receiver and hands it back: a `Stream`
    /// cannot hold a borrowing `changed()` future across polls (recreating
    /// one per poll would drop its waker registration and lose the
    /// wakeup), so the wait is materialized.
    channel: Option<Channel<T>>,
    checkpoint: Version,
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

/// A wait for the channel to change, owning the receiver; resolves to
/// whether the channel closed, and the receiver itself.
type WaitForChange<T> =
    Pin<Box<dyn Future<Output = (bool, watch::Receiver<crate::Inner<T>>)> + Send>>;

/// An observer's hold on the watch channel: either the receiver itself, or
/// the materialized owned wait a quiet poll left in flight (see the
/// [`UnorderedMessages::channel`] field docs for why the wait must be owned).
pub(super) enum Channel<T> {
    /// The channel is in hand.
    Ready(watch::Receiver<crate::Inner<T>>),
    /// A wait for change is in flight.
    Waiting(WaitForChange<T>),
}

/// One in-progress pass: the frozen walk over its snapshot, and the
/// snapshot's ceiling to absorb into the checkpoint when the walk drains.
struct Pass {
    walk: RangeOwned<causally::Down>,
    ceiling: Version,
}

impl<T> UnorderedMessages<T> {
    pub(crate) fn subscribe(inner: &watch::Sender<crate::Inner<T>>, since: Version) -> Self {
        Self {
            channel: Some(Channel::Ready(inner.subscribe())),
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
    /// moves only at pass boundaries, which are.
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
                    Self::open_pass(&mut this.pass, rx, &this.checkpoint);

                    let pass = this.pass.as_mut().expect("opened above");
                    if let Some((_, leaf)) = pass.walk.next() {
                        return Poll::Ready(Some((leaf.version().clone(), leaf.value::<T>())));
                    }

                    // The pass drained: absorb its ceiling, then enter the
                    // owned wait (the receiver rides inside the future and
                    // comes back with the result).
                    let Pass { ceiling, .. } = this.pass.take().expect("opened above");
                    this.checkpoint |= &ceiling;
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
