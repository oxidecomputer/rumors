//! [`CausalMessages`]: the [`Messages`](super::Messages) observer with
//! causal delivery — every message is yielded after every message it
//! causally depends on. The contract and the soundness argument live on
//! the type; this module is private.

use std::collections::BTreeMap;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use before::Area;
use futures::Stream;
use tokio::sync::watch;

use crate::tree::Leaf;
use crate::{Key, Version};

use super::Channel;

/// A causal-order observer of one rumor set: every message not causally
/// contained in the starting checkpoint, then every message learned afterwards,
/// each delivered *after* every delivered message it causally depends on —
/// and `None` once the [`Known`](crate::Known) and every
/// [`Broadcast`](crate::Broadcast) have dropped and the staged backlog has
/// drained.
///
/// The causal-delivery contract: for any two yielded messages with versions
/// `v` and `w`, if `v < w` then the `v` message is yielded first.
/// Concurrent messages are delivered in `(`[`Area`]`, `[`Key`]`)` order — a
/// deterministic linear extension of the causal order. Liveness and
/// multiplicity are exactly [`Messages`](super::Messages)': every message
/// live at some pass is yielded once; a message staged and then redacted
/// before delivery is still yielded; redactions are honored silently.
///
/// # How causal order is recovered from unordered passes
///
/// The plain observer runs in *passes*: each pass walks the leaves of a
/// frozen snapshot that are not causally contained in the checkpoint, in key
/// order — which bears no relation to causal order — and then absorbs the
/// snapshot's ceiling into the checkpoint. Causal delivery rests on two facts
/// about that engine:
///
/// 1. **Inversions are confined to a single pass.** A message delivered by
///    a later pass is never a causal predecessor of one delivered earlier:
///    every leaf in a pass's snapshot is `<=` that snapshot's ceiling, the
///    ceiling is absorbed into the frontier, later passes subtract the
///    frontier's causal past — and the tree can never *gain* a leaf whose
///    version its own version already covers, because deletion-honoring
///    treats absent-and-covered as redacted. Old news never arrives.
///
/// 2. **`(`[`Area`]`, `[`Key`]`)` is a linear extension of causality.**
///    The rank is strictly monotone (`v < w` implies `area(v) < area(w)`),
///    so equal areas are never causally ordered and the key tiebreak
///    between them is causally safe.
///
/// So the adapter is small: ingest each pass *whole* into an ordered
/// staging map keyed by the rank, and pop in that order. Within the staged
/// batch, rank order respects causality (fact 2); across batches, pass
/// order already does (fact 1).
///
/// Two faces over one engine, as with [`Messages`](super::Messages):
/// [`borrow_next`](Self::borrow_next) lends `(Key, &Version, &Arc<T>)`, and
/// the [`Stream`] impl (for `T: 'static`) yields owned items; on the sync
/// mirror the same face is an [`Iterator`].
///
/// Unlike [`Messages`](super::Messages), the idle state is **not**
/// constant-size: between items the observer holds every ingested,
/// not-yet-delivered message — the full history on a fresh subscription
/// until it drains, one gossip burst in steady state. Reordering must
/// buffer; this is the floor, not an implementation convenience.
pub struct CausalMessages<T> {
    /// The watch channel or the in-flight wait for it to change — the same
    /// owned-wait dance as [`Messages`](super::Messages) (see its field
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
    staged: BTreeMap<(Area, Key), Leaf<T>>,
    /// The most recently delivered leaf, kept alive so its version and
    /// value can be lent to the caller until the next call.
    current: Option<(Key, Leaf<T>)>,
}

impl<T> CausalMessages<T> {
    pub(crate) fn subscribe(inner: &watch::Sender<crate::Inner<T>>, since: Version) -> Self {
        Self {
            channel: Some(Channel::Ready(inner.subscribe())),
            ingested: since.clone(),
            checkpoint: since,
            staged: BTreeMap::new(),
            current: None,
        }
    }

    /// Ingest one whole pass over the latest snapshot: stage every live
    /// leaf not causally contained in the ingest frontier, keyed by its
    /// causal rank, then absorb the snapshot's ceiling into the frontier.
    ///
    /// Eager where [`Messages`](super::Messages) is lazy, by necessity: a
    /// pass arrives in key order, so any leaf might causally precede one
    /// staged earlier, and nothing can be delivered until the pass is
    /// complete. The watch read guard lives only long enough to freeze the
    /// walk and capture the ceiling; the walk itself runs unlocked.
    fn ingest(
        staged: &mut BTreeMap<(Area, Key), Leaf<T>>,
        ingested: &mut Version,
        rx: &mut watch::Receiver<crate::Inner<T>>,
    ) where
        T: Send + Sync,
    {
        let (mut walk, ceiling) = {
            let inner = rx.borrow_and_update();
            (
                inner.tree.freeze((
                    std::ops::Bound::Excluded(ingested.clone()),
                    std::ops::Bound::Unbounded,
                )),
                inner.tree.latest().clone(),
            )
        };
        while let Some((key, leaf)) = walk.next() {
            staged.insert((leaf.version().area(), key), leaf);
        }
        *ingested |= &ceiling;
    }

    /// Pop the causally least staged message, parking it in `current` so
    /// its borrows survive the return, and let the resume point catch up
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

    /// Advance to the next message in causal order, lending its version and
    /// value until the following call. Awaits quietly while the set is
    /// unchanged; resolves [`None`] once no further change is possible and
    /// the backlog has drained.
    pub async fn borrow_next(&mut self) -> Option<(Key, &Version, &Arc<T>)>
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

    /// The sound resume point: the causal frontier *behind* the staged
    /// backlog, suitable for persisting across processes or handing to
    /// another replica of the same network — a later
    /// [`causal_messages_from(checkpoint)`](crate::Broadcast::causal_messages_from)
    /// re-observes nothing already delivered *and drained*, and everything
    /// not yet delivered. While a backlog is draining the checkpoint holds at
    /// the batch's range start (a [`Version`] can only encode a causally
    /// closed boundary, and a half-delivered rank prefix need not be one),
    /// so a resume re-delivers the partially drained batch; dedup by
    /// [`Key`] across such a resume if re-delivery matters. For the same
    /// reason, folding the yielded versions yourself is *not* a sound
    /// resume point: deliver `a` then `b`, and their fold can cover an
    /// undelivered `c = a | b` that sorts after both, which a resume would
    /// then skip forever.
    ///
    /// After the observer ends (`None`), this is the complete final
    /// frontier.
    pub fn checkpoint(&self) -> &Version {
        &self.checkpoint
    }
}

/// The owned-item face: `(Key, Version, Arc<T>)` per item, popped from the
/// same staged backlog [`borrow_next`](CausalMessages::borrow_next) lends
/// from. `T: 'static` because the quiet-period wait is materialized as an
/// owned future (see [`Messages`](super::Messages)' `channel` field).
impl<T: Send + Sync + 'static> Stream for CausalMessages<T> {
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
