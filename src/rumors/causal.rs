use std::collections::BTreeMap;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use std::marker::PhantomData;

use before::Rank;
use futures::Stream;
use tokio::sync::watch;

use crate::mode::{Async, Blocking, Mode};
use crate::tree::Leaf;
use crate::{Key, Version};

use super::unordered::{Channel, TryNext};

/// An observer of messages sent to a [`Rumors`](crate::Rumors), in some
/// arbitrary yet causal order.
///
/// For any two yielded messages with versions `v` and `w`, if `v < w` then the
/// `v` message is yielded first. Concurrent messages are delivered in arbitrary
/// order, which may differ between [`gossip`](crate::Rumors::gossip)ing
/// replicas of the same [`Rumors`](crate::Rumors).
///
/// Unlike [`UnorderedMessages`](super::UnorderedMessages), this imposes an additional logarithmic
/// cost in amortized memory and in the time to retrieve each message, both of
/// which may have arbitrarily large bursts, up to the total size of the
/// messages stored in the underlying [`Rumors`](crate::Rumors).
///
/// This observer does not count against the quiescence that lets
/// [`try_into_peer`](crate::Rumors::try_into_peer) reclaim the
/// [`Peer`](crate::Peer).
pub struct CausalMessages<T, M: Mode = Async> {
    /// The watch channel or the in-flight wait for it to change — the same
    /// owned-wait dance as [`UnorderedMessages`](super::UnorderedMessages) (see its field
    /// docs for why the wait is materialized).
    channel: Option<Channel<T>>,
    /// The ingest frontier: the causal past already staged (or delivered).
    /// The next pass walks leaves *not* contained here. Advances at ingest,
    /// so it runs ahead of delivery while the backlog drains.
    ingested: Version,
    /// The public resume point: [`checkpoint`](Self::checkpoint). Trails
    /// [`ingested`](Self::ingested) — catching up exactly when the staged
    /// backlog empties — so that resuming from it never skips a staged,
    /// undelivered message.
    checkpoint: Version,
    /// The undelivered backlog, in causal-rank order. Always the residue of
    /// a *single* ingest (a new pass opens only once this empties), whose
    /// range start was `checkpoint` and whose ceiling is `ingested`.
    staged: BTreeMap<(Rank, Key), Leaf<T>>,
    /// The most recently delivered leaf, kept alive so its version and
    /// value can be lent to the caller until the next call.
    current: Option<(Key, Leaf<T>)>,
    /// The I/O [`Mode`] witness; see [`Peer`](crate::Peer)'s `marker`.
    marker: PhantomData<fn() -> M>,
}

impl<T, M: Mode> CausalMessages<T, M> {
    pub(crate) fn subscribe(inner: &watch::Sender<crate::Inner<T>>, since: Version) -> Self {
        Self {
            channel: Some(Channel::Ready(inner.subscribe())),
            ingested: since.clone(),
            checkpoint: since,
            staged: BTreeMap::new(),
            current: None,
            marker: PhantomData,
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
        staged: &mut BTreeMap<(Rank, Key), Leaf<T>>,
        ingested: &mut Version,
        rx: &mut watch::Receiver<crate::Inner<T>>,
    ) where
        T: Send + Sync,
    {
        let (mut walk, ceiling) = {
            let inner = rx.borrow_and_update();
            (
                inner.tree.iter_owned((
                    std::ops::Bound::Excluded(ingested.clone()),
                    std::ops::Bound::Unbounded,
                )),
                inner.tree.latest().clone(),
            )
        };
        while let Some((key, leaf)) = walk.next() {
            staged.insert((leaf.version().rank(), key), leaf);
        }
        *ingested |= &ceiling;
    }

    /// Pop the causally least staged message, parking it in `current` so
    /// its borrows survive the return.
    ///
    /// Lets the resume point catch up
    /// when this empties the backlog (the popped message is in the caller's
    /// hands by the time the checkpoint can be read).
    fn pop(&mut self) -> Option<(Key, &Version, &Arc<T>)> {
        let ((_, key), leaf) = self.staged.pop_first()?;
        if self.staged.is_empty() {
            self.checkpoint = self.ingested.clone();
        }
        let (key, leaf) = self.current.insert((key, leaf));
        Some((*key, leaf.version(), leaf.value()))
    }

    /// The mode-agnostic engine behind the async and blocking
    /// [`borrow_next`](CausalMessages::borrow_next): advances to the next
    /// message in causal order and lends it.
    ///
    /// The async face awaits it; the
    /// blocking face drives it to completion.
    pub(crate) async fn borrow_next_inner(&mut self) -> Option<(Key, &Version, &Arc<T>)>
    where
        T: Send + Sync,
    {
        loop {
            // Deliver the staged backlog before consulting the channel:
            // everything staged became deliverable when its pass finished
            // ingesting. (Polonius limitation: returning `self.pop()` here
            // would hold the borrow across the loop, so flag-and-break.)
            if !self.staged.is_empty() {
                break;
            }
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
                    Self::ingest(&mut self.staged, &mut self.ingested, rx);
                    if self.staged.is_empty() {
                        // Nothing new: the resume point is already current;
                        // await the next change. `Err` means every sender
                        // is gone and the ingest above saw the final state.
                        self.checkpoint = self.ingested.clone();
                        if rx.changed().await.is_err() {
                            return None;
                        }
                    }
                }
            }
        }
        self.pop()
    }

    /// The sound resume point: the causal frontier *behind* any internally
    /// staged backlog, suitable for persisting across processes or handing to
    /// another replica of the same network.
    ///
    /// It is guaranteed that resuming from this [`Version`] will never skip
    /// messages; however, it may replay an arbitrary number of them.
    ///
    /// After the observer ends (`None`), this is the final [`Version`] of the
    /// [`Rumors`](crate::Rumors).
    pub fn checkpoint(&self) -> &Version {
        &self.checkpoint
    }
}

impl<T> CausalMessages<T, Async> {
    /// Advance to the next message in causal order, lending its version and
    /// value until the following call.
    ///
    /// Awaits quietly while the set is
    /// unchanged; resolves [`None`] once no further change is possible and
    /// the backlog has drained.
    pub async fn borrow_next(&mut self) -> Option<(Key, &Version, &Arc<T>)>
    where
        T: Send + Sync,
    {
        self.borrow_next_inner().await
    }
}

impl<T> CausalMessages<T, Blocking> {
    /// Blocking [`borrow_next`](CausalMessages::borrow_next): blocks the
    /// calling thread (via [`pollster`], with no async runtime) until a
    /// message is ready or the set has closed, instead of awaiting.
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

/// The owned-item face: `(Key, Version, Arc<T>)` per item, popped from the
/// same staged backlog [`borrow_next`](CausalMessages::borrow_next) lends
/// from.
///
/// `T: 'static` because the quiet-period wait is materialized as an
/// owned future (see [`UnorderedMessages`](super::UnorderedMessages)' `channel` field).
impl<T: Send + Sync + 'static> Stream for CausalMessages<T, Async> {
    type Item = (Key, Version, Arc<T>);

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        loop {
            if let Some(((_, key), leaf)) = this.staged.pop_first() {
                if this.staged.is_empty() {
                    this.checkpoint = this.ingested.clone();
                }
                return Poll::Ready(Some((key, leaf.version().clone(), leaf.value().clone())));
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

/// The blocking owned-item face: the [`Iterator`] analogue of the [`Stream`]
/// impl, cloning each item out of the same staged backlog
/// [`borrow_next`](CausalMessages::borrow_next) lends from.
///
/// [`next`](Iterator::next) blocks the calling thread (via [`pollster`]) until
/// an item is ready; [`None`] means the set has closed and is fully delivered.
impl<T: Send + Sync + 'static> Iterator for CausalMessages<T, Blocking> {
    type Item = (Key, Version, Arc<T>);

    fn next(&mut self) -> Option<Self::Item> {
        let (key, version, value) = pollster::block_on(self.borrow_next_inner())?;
        Some((key, version.clone(), value.clone()))
    }
}
