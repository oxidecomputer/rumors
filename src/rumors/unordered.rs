use crate::mode::{Async, Blocking, Mode};
use crate::tree::{Frozen, Leaf};
use crate::{Key, Version};
use futures::Stream;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::sync::watch;

/// An observer of messages sent to a [`Rumors`](crate::Rumors), in completely
/// arbitrary (*non-causal*) order.
///
/// This enumerates every message not causally contained in the starting
/// checkpoint, then every message learned afterwards — by local
/// [`send`](crate::Rumors::send), by gossip, from any handle — and `None` once
/// the [`Peer`](crate::Peer) and every [`Rumors`](crate::Rumors) have dropped
/// and no further change is possible, after yielding every message learned
/// prior.
///
/// There are two ways to use it:
///
/// - [`borrow_next`](Self::borrow_next) lends each message as `(Key,
///   &Version, &Arc<T>)`, the borrows living until the next call.
/// - The [`Stream`] impl (for `T: 'static`) yields owned `(Key, Version,
///   Arc<T>)`.
///
/// Order is unspecified and does *not* follow the causal order: a message may
/// be yielded before another that causally precedes it; use
/// [`CausalMessages`](super::CausalMessages) if you want causal iteration order
/// (at an amortized logarithmic cost in extra internal bookkeeping).
///
/// This observer does not count against the quiescence that lets
/// [`try_into_peer`](crate::Rumors::try_into_peer) reclaim the
/// [`Peer`](crate::Peer).
pub struct UnorderedMessages<T, M: Mode = Async> {
    /// The watch channel, or the in-flight wait for it to change. The wait
    /// future owns the receiver and hands it back: the `Stream` face cannot
    /// hold a borrowing `changed()` future across polls (recreating one per
    /// poll would drop its waker registration and lose the wakeup), so the
    /// wait is materialized; `borrow_next` enters it only to finish what a
    /// `Stream` poll started.
    channel: Option<Channel<T>>,
    checkpoint: Version,
    pass: Option<Pass<T>>,
    /// The most recently yielded leaf, kept alive so its version and value
    /// can be lent to the caller until the next call.
    current: Option<(Key, Leaf<T>)>,
    /// The I/O [`Mode`] witness; see [`Peer`](crate::Peer)'s `marker`.
    marker: PhantomData<fn() -> M>,
}

/// The outcome of [`UnorderedMessages::try_next`] or [`CausalMessages::try_next`].
///
/// A non-blocking step that either yields a message or says why it can't.
///
/// [`CausalMessages::try_next`]: super::CausalMessages::try_next
#[derive(Debug)]
pub enum TryNext<'a, T> {
    /// A message was ready, lent until the next call (as
    /// [`borrow_next`](UnorderedMessages::borrow_next) lends it).
    Message((Key, &'a Version, &'a Arc<T>)),
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
/// the materialized owned wait the `Stream` face left in flight (see the
/// [`UnorderedMessages::channel`] field docs for why the wait must be owned).
pub(super) enum Channel<T> {
    /// The channel is in hand.
    Ready(watch::Receiver<crate::Inner<T>>),
    /// A wait for change is in flight.
    Waiting(WaitForChange<T>),
}

/// One in-progress pass: the frozen walk over its snapshot, and the
/// snapshot's ceiling to absorb into the checkpoint when the walk drains.
struct Pass<T> {
    walk: Frozen<T, (std::ops::Bound<Version>, std::ops::Bound<Version>)>,
    ceiling: Version,
}

impl<T, M: Mode> UnorderedMessages<T, M> {
    pub(crate) fn subscribe(inner: &watch::Sender<crate::Inner<T>>, since: Version) -> Self {
        Self {
            channel: Some(Channel::Ready(inner.subscribe())),
            checkpoint: since,
            pass: None,
            current: None,
            marker: PhantomData,
        }
    }

    /// Open a pass over the latest snapshot if none is in progress. The
    /// watch read guard lives only long enough to freeze the walk (a root
    /// handle clone) and capture the ceiling.
    fn open_pass(
        pass: &mut Option<Pass<T>>,
        rx: &mut watch::Receiver<crate::Inner<T>>,
        checkpoint: &Version,
    ) where
        T: Send + Sync,
    {
        if pass.is_none() {
            let inner = rx.borrow_and_update();
            *pass = Some(Pass {
                walk: inner.tree.freeze((
                    std::ops::Bound::Excluded(checkpoint.clone()),
                    std::ops::Bound::Unbounded,
                )),
                ceiling: inner.tree.latest().clone(),
            });
        }
    }

    /// The mode-agnostic engine behind the async and blocking
    /// [`borrow_next`](UnorderedMessages::borrow_next): advances to the next message and
    /// lends it. The async face awaits it; the blocking face drives it to
    /// completion.
    pub(crate) async fn borrow_next_inner(&mut self) -> Option<(Key, &Version, &Arc<T>)>
    where
        T: Send + Sync,
    {
        loop {
            match self.channel.as_mut().expect("channel state present") {
                // Finish a wait the `Stream` face left in flight.
                Channel::Waiting(wait) => {
                    let (closed, rx) = wait.as_mut().await;
                    self.channel = Some(Channel::Ready(rx));
                    if closed {
                        return None;
                    }
                }
                Channel::Ready(rx) => {
                    Self::open_pass(&mut self.pass, rx, &self.checkpoint);

                    // Lend the next leaf out of the walk, parking it in
                    // `current` so the borrows survive the return.
                    let pass = self.pass.as_mut().expect("opened above");
                    if let Some((key, leaf)) = pass.walk.next() {
                        let (key, leaf) = self.current.insert((key, leaf));
                        return Some((*key, leaf.version(), leaf.value()));
                    }

                    // The pass drained: absorb its ceiling as completed,
                    // then await the next change; `Err` means every sender
                    // is gone and the drain above already saw the final
                    // state.
                    let Pass { ceiling, .. } = self.pass.take().expect("opened above");
                    self.checkpoint |= &ceiling;
                    if rx.changed().await.is_err() {
                        return None;
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
    /// rumors.send("one".to_string());
    ///
    /// let mut observer = rumors.unordered_messages();
    /// let (_key, _version, m) = observer.borrow_next().await.expect("one message");
    /// assert_eq!(m.as_str(), "one");
    ///
    /// // Mid-pass, the checkpoint has not moved: resuming here would
    /// // re-deliver "one" (a partial pass is not a causally closed boundary).
    /// assert_eq!(observer.checkpoint(), &Version::new());
    ///
    /// // One more step finds nothing ready — completing the pass and
    /// // absorbing its frontier into the checkpoint.
    /// assert!(observer.borrow_next().now_or_never().is_none());
    /// let checkpoint = observer.checkpoint().clone();
    ///
    /// // A resume from it re-observes nothing from the completed pass and
    /// // everything not yet delivered.
    /// rumors.send("two".to_string());
    /// let mut resumed = rumors.unordered_messages_since(checkpoint);
    /// let (_key, _version, m) = resumed.borrow_next().await.expect("only the new message");
    /// assert_eq!(m.as_str(), "two");
    /// # });
    /// ```
    pub fn checkpoint(&self) -> &Version {
        &self.checkpoint
    }
}

impl<T> UnorderedMessages<T, Async> {
    /// Advance to the next message, lending its version and value until the
    /// following call. Awaits quietly while the set is unchanged; resolves
    /// [`None`] once no further change is possible.
    pub async fn borrow_next(&mut self) -> Option<(Key, &Version, &Arc<T>)>
    where
        T: Send + Sync,
    {
        self.borrow_next_inner().await
    }
}

impl<T> UnorderedMessages<T, Blocking> {
    /// Blocking [`borrow_next`](UnorderedMessages::borrow_next): blocks the calling
    /// thread (via [`pollster`], with no async runtime) until a message is
    /// ready or the set has closed, instead of awaiting.
    pub fn borrow_next(&mut self) -> Option<(Key, &Version, &Arc<T>)>
    where
        T: Send + Sync,
    {
        pollster::block_on(self.borrow_next_inner())
    }

    /// Take one non-blocking step: a message if one is ready, [`Quiet`] (ask
    /// again later) if not, [`Ended`] if no further message is possible.
    ///
    /// [`Quiet`]: TryNext::Quiet
    /// [`Ended`]: TryNext::Ended
    pub fn try_next(&mut self) -> TryNext<'_, T>
    where
        T: Send + Sync,
    {
        use futures::FutureExt;
        match self.borrow_next_inner().now_or_never() {
            None => TryNext::Quiet,
            Some(None) => TryNext::Ended,
            Some(Some(message)) => TryNext::Message(message),
        }
    }
}

/// The owned-item face: `(Key, Version, Arc<T>)` per item, cloned out of
/// the same engine [`borrow_next`](UnorderedMessages::borrow_next) lends from.
/// `T: 'static` because the quiet-period wait is materialized as an owned
/// future (see the `channel` field).
impl<T: Send + Sync + 'static> Stream for UnorderedMessages<T, Async> {
    type Item = (Key, Version, Arc<T>);

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
                    if let Some((key, leaf)) = pass.walk.next() {
                        return Poll::Ready(Some((
                            key,
                            leaf.version().clone(),
                            leaf.value().clone(),
                        )));
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

/// The blocking owned-item face: the [`Iterator`] analogue of the [`Stream`]
/// impl, cloning each item out of the same engine
/// [`borrow_next`](UnorderedMessages::borrow_next) lends from. [`next`](Iterator::next)
/// blocks the calling thread (via [`pollster`]) until an item is ready;
/// [`None`] means the set has closed and is fully delivered.
impl<T: Send + Sync + 'static> Iterator for UnorderedMessages<T, Blocking> {
    type Item = (Key, Version, Arc<T>);

    fn next(&mut self) -> Option<Self::Item> {
        let (key, version, value) = pollster::block_on(self.borrow_next_inner())?;
        Some((key, version.clone(), value.clone()))
    }
}
