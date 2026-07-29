use std::collections::BTreeMap;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use before::Rank;
use futures::{Stream, StreamExt as _};
use tokio::sync::watch;

use crate::tree::backend::{Leaf as _, Local, Node as _, Store, VersionBounds};
use crate::{Key, StorageError, Version};

use super::unordered::TryNext;

/// An observer of messages sent to a [`Rumors`](crate::Rumors), in some
/// arbitrary yet causal order.
///
/// For any two yielded messages with versions `v` and `w`, if `v < w` then the
/// `v` message is yielded first. Concurrent messages are delivered in arbitrary
/// order, which may differ between [`gossip`](crate::Rumors::gossip)ing
/// replicas of the same [`Rumors`](crate::Rumors).
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
pub struct CausalMessages<T: Send + Sync + 'static, S: Store<T> = Local> {
    /// The watch channel, the in-flight wait for it to change, or the
    /// in-flight ingest of one pass.
    ///
    /// The same owned-state dance as
    /// [`UnorderedMessages`](super::UnorderedMessages) (see its field docs
    /// for why waits are materialized), plus one more owned state because
    /// an ingest drains a whole backend walk and must therefore be
    /// resumable across polls too.
    state: Option<State<T, S>>,
    /// The ingest frontier: the causal past already staged (or delivered).
    /// The next pass walks leaves *not* contained here. Advances at ingest,
    /// so it runs ahead of delivery while the backlog drains.
    ingested: Version,
    /// The public resume point: [`checkpoint`](Self::checkpoint). Trails
    /// [`ingested`](Self::ingested), catching up exactly when the staged
    /// backlog empties, so that resuming from it never skips a staged,
    /// undelivered message.
    checkpoint: Version,
    /// The undelivered backlog, in causal-rank order. Always the residue of
    /// a *single* ingest (a new pass opens only once this empties), whose
    /// range start was `checkpoint` and whose ceiling is `ingested`.
    staged: BTreeMap<(Rank, Key), (Version, Arc<T>)>,
}

/// A wait for the channel to change, owning the receiver; resolves to
/// whether the channel closed, and the receiver itself.
type WaitForChange<T, S> =
    Pin<Box<dyn Future<Output = (bool, watch::Receiver<crate::Inner<T, S>>)> + Send>>;

/// One pass's staged additions: each selected leaf as its rank-keyed
/// owned entry, the backlog's own row shape.
type Additions<T> = Vec<((Rank, Key), (Version, Arc<T>))>;

/// One whole pass over a frozen snapshot, owning the receiver while it
/// runs; resolves to the staged additions, the snapshot's ceiling, and the
/// receiver, or the backend's error.
#[allow(clippy::type_complexity)]
type Ingest<T, S> = Pin<
    Box<
        dyn Future<
                Output = (
                    Result<(Additions<T>, Version), <S as crate::tree::backend::Backend<T>>::Error>,
                    watch::Receiver<crate::Inner<T, S>>,
                ),
            > + Send,
    >,
>;

/// The observer's owned state between items.
enum State<T: Send + Sync + 'static, S: Store<T>> {
    /// The channel is in hand.
    Ready(watch::Receiver<crate::Inner<T, S>>),
    /// A wait for change is in flight.
    Waiting(WaitForChange<T, S>),
    /// A pass over the latest snapshot is being ingested.
    Ingesting(Ingest<T, S>),
}

impl<T: Send + Sync + 'static, S: Store<T>> CausalMessages<T, S> {
    pub(crate) fn subscribe(inner: &watch::Sender<crate::Inner<T, S>>, since: Version) -> Self {
        Self {
            state: Some(State::Ready(inner.subscribe())),
            ingested: since.clone(),
            checkpoint: since,
            staged: BTreeMap::new(),
        }
    }

    /// Start ingesting one whole pass over the latest snapshot: stage every
    /// live leaf not causally contained in the ingest frontier, keyed by
    /// its causal rank.
    ///
    /// Eager where [`UnorderedMessages`](super::UnorderedMessages) is lazy,
    /// by necessity: a pass arrives in key order, so any leaf might
    /// causally precede one staged earlier, and nothing can be delivered
    /// until the pass is complete. The watch read guard lives only long
    /// enough to freeze the walk (a root handle clone) and capture the
    /// ceiling; the walk itself runs unlocked, inside the returned owned
    /// future.
    fn ingest(mut rx: watch::Receiver<crate::Inner<T, S>>, ingested: &Version) -> Ingest<T, S> {
        let since = ingested.clone();
        Box::pin(async move {
            let (walk, ceiling) = {
                let inner = rx.borrow_and_update();
                (
                    inner.tree.range(VersionBounds {
                        start: std::ops::Bound::Excluded(since),
                        end: std::ops::Bound::Unbounded,
                    }),
                    inner.tree.latest().clone(),
                )
            };
            let mut walk = std::pin::pin!(walk);
            let mut additions = Vec::new();
            while let Some(item) = walk.next().await {
                match item {
                    Ok((key, leaf)) => {
                        let version = leaf.span().join().clone();
                        additions.push((
                            (version.rank(), key),
                            (version, leaf.message().as_arc().clone()),
                        ));
                    }
                    Err(e) => return (Err(e), rx),
                }
            }
            (Ok((additions, ceiling)), rx)
        })
    }

    /// Pop the causally least staged message, yielding it owned.
    ///
    /// Lets the resume point catch up when this empties the backlog (the
    /// popped message is in the caller's hands by the time the checkpoint
    /// can be read).
    fn pop(&mut self) -> Option<(Key, Version, Arc<T>)> {
        let ((_, key), (version, value)) = self.staged.pop_first()?;
        if self.staged.is_empty() {
            self.checkpoint = self.ingested.clone();
        }
        Some((key, version, value))
    }

    /// Fold one completed ingest into the backlog and frontier.
    fn absorb(&mut self, additions: Additions<T>, ceiling: Version) {
        self.staged.extend(additions);
        self.ingested |= &ceiling;
    }

    /// Advance to the next message in causal order.
    async fn next_inner(
        &mut self,
    ) -> Result<Option<(Key, Version, Arc<T>)>, StorageError<S::Error>> {
        loop {
            // Deliver the staged backlog before consulting the channel:
            // everything staged became deliverable when its pass finished
            // ingesting.
            if !self.staged.is_empty() {
                break;
            }
            match self.state.as_mut().expect("observer state present") {
                // Finish a wait the `Stream` face left in flight.
                State::Waiting(wait) => {
                    let (closed, rx) = wait.as_mut().await;
                    self.state = Some(State::Ready(rx));
                    if closed {
                        return Ok(None);
                    }
                }
                State::Ready(_) => {
                    let Some(State::Ready(rx)) = self.state.take() else {
                        unreachable!("matched Ready above");
                    };
                    self.state = Some(State::Ingesting(Self::ingest(rx, &self.ingested)));
                }
                State::Ingesting(ingest) => {
                    let (outcome, rx) = ingest.as_mut().await;
                    // The completed ingest is spent: retire it *before* any
                    // further await or return, or a cancelled caller would
                    // leave it in place to be re-polled after completion.
                    self.state = Some(State::Ready(rx));
                    let (additions, ceiling) = match outcome {
                        Ok(done) => done,
                        Err(e) => return Err(StorageError(e)),
                    };
                    self.absorb(additions, ceiling);
                    if self.staged.is_empty() {
                        // Nothing new: the resume point is already current;
                        // await the next change. `Err` means every sender
                        // is gone and the ingest above saw the final state.
                        // Cancellation here parks the state at `Ready`, so
                        // the next call re-ingests — the same re-walk a
                        // fresh pass over an unchanged set always costs.
                        self.checkpoint = self.ingested.clone();
                        let Some(State::Ready(rx)) = self.state.as_mut() else {
                            unreachable!("set to Ready above");
                        };
                        if rx.changed().await.is_err() {
                            return Ok(None);
                        }
                    }
                }
            }
        }
        Ok(self.pop())
    }

    /// The sound resume point: the causal frontier *behind* any internally
    /// staged backlog, suitable for persisting across processes or handing to
    /// another replica of the same network.
    ///
    /// Resuming from this [`Version`] will never skip messages, but it may
    /// replay an arbitrary number of them.
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

    /// Advance to the next message in causal order, yielding it owned.
    ///
    /// Awaits quietly while the set is unchanged; resolves `Ok(None)` once
    /// no further change is possible and the backlog has drained.
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

/// The `Stream` face: the same owned items as [`next`](CausalMessages::next),
/// popped from the same staged backlog.
///
/// `T: 'static` because the quiet-period wait and each pass's ingest are
/// materialized as owned futures, exactly as in
/// [`UnorderedMessages`](super::UnorderedMessages).
impl<T: Send + Sync + 'static, S: Store<T>> Stream for CausalMessages<T, S> {
    type Item = Result<(Key, Version, Arc<T>), StorageError<S::Error>>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        loop {
            if let Some(message) = this.pop() {
                return Poll::Ready(Some(Ok(message)));
            }
            match this.state.as_mut().expect("observer state present") {
                State::Waiting(wait) => match wait.as_mut().poll(cx) {
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready((closed, rx)) => {
                        this.state = Some(State::Ready(rx));
                        if closed {
                            return Poll::Ready(None);
                        }
                    }
                },
                State::Ready(_) => {
                    let Some(State::Ready(rx)) = this.state.take() else {
                        unreachable!("matched Ready above");
                    };
                    this.state = Some(State::Ingesting(Self::ingest(rx, &this.ingested)));
                }
                State::Ingesting(ingest) => match ingest.as_mut().poll(cx) {
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready((outcome, mut rx)) => {
                        let (additions, ceiling) = match outcome {
                            Ok(done) => done,
                            Err(e) => {
                                this.state = Some(State::Ready(rx));
                                return Poll::Ready(Some(Err(StorageError(e))));
                            }
                        };
                        this.absorb(additions, ceiling);
                        if this.staged.is_empty() {
                            // Nothing new: catch the resume point up and
                            // enter the owned wait (the receiver rides
                            // inside the future and comes back with the
                            // result).
                            this.checkpoint = this.ingested.clone();
                            this.state = Some(State::Waiting(Box::pin(async move {
                                let closed = rx.changed().await.is_err();
                                (closed, rx)
                            })));
                        } else {
                            this.state = Some(State::Ready(rx));
                        }
                    }
                },
            }
        }
    }
}
