use crate::tree::backend::{Leaf as _, Local, Store, VersionBounds};
use crate::{Key, StorageError, Version};
use futures::{Stream, StreamExt as _};
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
/// There are two equivalent faces:
///
/// - [`next`](Self::next) yields each message as an owned
///   `(Key, Version, Arc<T>)`, or the storage backend's error.
/// - The [`Stream`] impl (for `T: 'static`) yields the same items as
///   `Result`s.
///
/// Order is unspecified and does *not* follow the causal order: a message may
/// be yielded before another that causally precedes it; use
/// [`CausalMessages`](super::CausalMessages) if you want causal iteration order
/// (at an amortized logarithmic cost in extra internal bookkeeping).
///
/// This observer does not count against the quiescence that lets
/// [`try_into_peer`](crate::Rumors::try_into_peer) reclaim the
/// [`Peer`](crate::Peer).
pub struct UnorderedMessages<T: Send + Sync + 'static, S: Store<T> = Local> {
    /// The watch channel, or the in-flight wait for it to change.
    ///
    /// The wait future owns the receiver and hands it back: the `Stream`
    /// face cannot hold a borrowing `changed()` future across polls
    /// (recreating one per poll would drop its waker registration and lose
    /// the wakeup), so the wait is materialized; [`next`](Self::next)
    /// enters it only to finish what a `Stream` poll started.
    channel: Option<Channel<T, S>>,
    checkpoint: Version,
    pass: Option<Pass<T, S>>,
}

/// The outcome of [`UnorderedMessages::try_next`] or [`CausalMessages::try_next`].
///
/// A non-blocking step that either yields a message or says why it can't.
///
/// [`CausalMessages::try_next`]: super::CausalMessages::try_next
#[derive(Debug)]
pub enum TryNext<T> {
    /// A message was ready, yielded owned (as [`next`](UnorderedMessages::next)
    /// yields it).
    Message((Key, Version, Arc<T>)),
    /// No message is ready yet, but handles are still live: ask again
    /// later. With a backend whose reads suspend, a fetch still in flight
    /// also reads as `Quiet`; with the in-memory backend, `Quiet` means
    /// exactly "nothing new".
    Quiet,
    /// Every handle is gone and no further message is possible.
    Ended,
}

/// A wait for the channel to change, owning the receiver; resolves to
/// whether the channel closed, and the receiver itself.
type WaitForChange<T, S> =
    Pin<Box<dyn Future<Output = (bool, watch::Receiver<crate::Inner<T, S>>)> + Send>>;

/// An observer's hold on the watch channel: either the receiver itself, or
/// the materialized owned wait the `Stream` face left in flight (see the
/// [`UnorderedMessages::channel`] field docs for why the wait must be owned).
pub(super) enum Channel<T: Send + Sync + 'static, S: Store<T> = Local> {
    /// The channel is in hand.
    Ready(watch::Receiver<crate::Inner<T, S>>),
    /// A wait for change is in flight.
    Waiting(WaitForChange<T, S>),
}

/// One in-progress pass: the frozen walk over its snapshot, and the
/// snapshot's ceiling to absorb into the checkpoint when the walk drains.
struct Pass<T: Send + Sync + 'static, S: Store<T>> {
    walk: S::Walk,
    ceiling: Version,
}

impl<T: Send + Sync + 'static, S: Store<T>> UnorderedMessages<T, S> {
    pub(crate) fn subscribe(inner: &watch::Sender<crate::Inner<T, S>>, since: Version) -> Self {
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
        pass: &mut Option<Pass<T, S>>,
        rx: &mut watch::Receiver<crate::Inner<T, S>>,
        checkpoint: &Version,
    ) {
        if pass.is_none() {
            let inner = rx.borrow_and_update();
            *pass = Some(Pass {
                walk: inner.tree.range(VersionBounds {
                    start: std::ops::Bound::Excluded(checkpoint.clone()),
                    end: std::ops::Bound::Unbounded,
                }),
                ceiling: inner.tree.latest().clone(),
            });
        }
    }

    /// Advance to the next message.
    async fn next_inner(
        &mut self,
    ) -> Result<Option<(Key, Version, Arc<T>)>, StorageError<S::Error>> {
        loop {
            match self.channel.as_mut().expect("channel state present") {
                // Finish a wait the `Stream` face left in flight.
                Channel::Waiting(wait) => {
                    let (closed, rx) = wait.as_mut().await;
                    self.channel = Some(Channel::Ready(rx));
                    if closed {
                        return Ok(None);
                    }
                }
                Channel::Ready(rx) => {
                    Self::open_pass(&mut self.pass, rx, &self.checkpoint);

                    let pass = self.pass.as_mut().expect("opened above");
                    if let Some(item) = pass.walk.next().await {
                        let (key, leaf) = item.map_err(StorageError)?;
                        return Ok(Some((
                            key,
                            leaf.version().clone(),
                            leaf.message().as_arc().clone(),
                        )));
                    }

                    // The pass drained: absorb its ceiling as completed,
                    // then await the next change; `Err` means every sender
                    // is gone and the drain above already saw the final
                    // state.
                    let Pass { ceiling, .. } = self.pass.take().expect("opened above");
                    self.checkpoint |= &ceiling;
                    if rx.changed().await.is_err() {
                        return Ok(None);
                    }
                }
            }
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
    /// use futures::FutureExt;
    /// use rumors::{Peer, Version};
    ///
    /// # tokio::runtime::Builder::new_current_thread()
    /// #     .build()
    /// #     .unwrap()
    /// #     .block_on(async {
    /// let rumors = Peer::<String>::seed().into_rumors();
    /// rumors.send("one".to_string()).await?;
    ///
    /// let mut observer = rumors.unordered_messages();
    /// let (_key, _version, m) = observer.next().await?.expect("one message");
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
    /// rumors.send("two".to_string()).await?;
    /// let mut resumed = rumors.unordered_messages_since(checkpoint);
    /// let (_key, _version, m) = resumed.next().await?.expect("only the new message");
    /// assert_eq!(m.as_str(), "two");
    /// # Ok::<(), rumors::StorageError<std::convert::Infallible>>(())
    /// # })?;
    /// # Ok::<(), rumors::StorageError<std::convert::Infallible>>(())
    /// ```
    pub fn checkpoint(&self) -> &Version {
        &self.checkpoint
    }
}

impl<T: Send + Sync + 'static, S: Store<T>> UnorderedMessages<T, S> {
    /// Advance to the next message, yielding it owned. Awaits quietly while
    /// the set is unchanged; resolves `Ok(None)` once no further change is
    /// possible.
    pub async fn next(&mut self) -> Result<Option<(Key, Version, Arc<T>)>, StorageError<S::Error>> {
        self.next_inner().await
    }

    /// Take one non-blocking step: a message if one is ready, [`Quiet`] (ask
    /// again later) if not, [`Ended`] if no further message is possible.
    ///
    /// [`Quiet`]: TryNext::Quiet
    /// [`Ended`]: TryNext::Ended
    pub fn try_next(&mut self) -> Result<TryNext<T>, StorageError<S::Error>> {
        use futures::FutureExt;
        match self.next_inner().now_or_never() {
            None => Ok(TryNext::Quiet),
            Some(Ok(None)) => Ok(TryNext::Ended),
            Some(Ok(Some(message))) => Ok(TryNext::Message(message)),
            Some(Err(e)) => Err(e),
        }
    }
}

/// The `Stream` face: the same owned items as [`next`](UnorderedMessages::next),
/// one `Result` per message.
///
/// `T: 'static` because the quiet-period wait is materialized as an owned
/// future.
impl<T: Send + Sync + 'static, S: Store<T>> Stream for UnorderedMessages<T, S> {
    type Item = Result<(Key, Version, Arc<T>), StorageError<S::Error>>;

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
                    match pass.walk.poll_next_unpin(cx) {
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(Some(Ok((key, leaf)))) => {
                            return Poll::Ready(Some(Ok((
                                key,
                                leaf.version().clone(),
                                leaf.message().as_arc().clone(),
                            ))));
                        }
                        Poll::Ready(Some(Err(e))) => {
                            return Poll::Ready(Some(Err(StorageError(e))));
                        }
                        Poll::Ready(None) => {}
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
