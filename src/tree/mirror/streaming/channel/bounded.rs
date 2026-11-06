//! The bounded single-producer channel behind every protocol edge.
//!
//! Every edge in the streaming protocol has exactly one sender and one
//! receiver, and one session task polls both. [`channel`] builds such an edge
//! on the bounded `mpsc` channel of `futures-channel`, whose shared state has
//! ordinary alignment. Tokio's bounded channel would serve the same edge, but
//! it aligns two fields of its shared state to the target's cache-line size
//! (128 bytes on x86-64, AArch64, and powerpc64, and between 32 and 256
//! bytes elsewhere), which separates producer and consumer cache lines for
//! threads that run concurrently. An edge's two ends never do, and the
//! system allocator must serve every over-aligned block through the
//! platform's aligned-allocation entry point, which is slow on some
//! allocators, illumos's libumem (Rust's default allocator there) among
//! them.
//!
//! The channel behaves as Tokio's bounded channel of the same capacity:
//!
//! - It holds at most `capacity` items. A send waits while it is full and
//!   proceeds after the receiver takes an item.
//! - The receiver yields items in the order they were sent.
//! - Once the receiver is dropped or closed, every send fails and returns its
//!   item, a waiting send included.
//! - Once the sender is dropped, the receiver yields the items still queued
//!   and then `None`.
//!
//! `futures-channel` gives each sender one slot beyond its configured buffer,
//! so its capacity is the buffer plus the number of senders. [`Sender`] is
//! deliberately not [`Clone`]: with exactly one sender, a buffer of
//! `capacity - 1` holds exactly `capacity` items.

use std::fmt;
use std::pin::Pin;
#[cfg(any(test, feature = "test-internals"))]
use std::sync::OnceLock;
#[cfg(any(test, feature = "test-internals"))]
use std::task::Waker;
use std::task::{Context, Poll};

use futures::{Stream, StreamExt as _, channel::mpsc, future::poll_fn};

#[cfg(any(test, feature = "test-internals"))]
#[cfg(any(test, feature = "test-internals"))]
use super::QueueKind;

#[cfg(test)]
mod tests;

/// A send that failed because the receiver is gone, returning the item that
/// was not sent.
#[derive(Eq, PartialEq)]
pub struct SendError<T>(pub T);

impl<T> fmt::Debug for SendError<T> {
    /// Name the failure without requiring the item to be printable, so a
    /// caller can `expect` a send of any item type.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SendError(..)")
    }
}

impl<T> fmt::Display for SendError<T> {
    /// Name the failure without requiring the item to be printable.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("channel closed")
    }
}

impl<T> std::error::Error for SendError<T> {}

/// A send that could not proceed at once, returning the item that was not
/// sent, as Tokio's `TrySendError` does.
#[derive(Eq, PartialEq)]
pub enum TrySendError<T> {
    /// The channel holds `capacity` items.
    Full(T),
    /// The receiver is gone.
    Closed(T),
}

impl<T> fmt::Debug for TrySendError<T> {
    /// Name the failure without requiring the item to be printable, so a
    /// caller can `expect` a send of any item type.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Full(_) => f.write_str("Full(..)"),
            Self::Closed(_) => f.write_str("Closed(..)"),
        }
    }
}

impl<T> fmt::Display for TrySendError<T> {
    /// Name the failure without requiring the item to be printable.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Full(_) => f.write_str("channel full"),
            Self::Closed(_) => f.write_str("channel closed"),
        }
    }
}

impl<T> std::error::Error for TrySendError<T> {}

/// The sending half of a protocol edge: the edge's only producer.
pub struct Sender<T> {
    /// The `futures-channel` sender.
    inner: mpsc::Sender<T>,
    /// The edge this sender serves, when set: each send's channel node is
    /// attributed to the edge's place, and each send is reported to the
    /// process's send hook under the edge's name.
    #[cfg(any(test, feature = "test-internals"))]
    kind: Option<QueueKind>,
}

impl<T> Sender<T> {
    /// Poll for room to send one item.
    ///
    /// Returns `Ready(Ok(()))` when [`start_send`](Self::start_send) will
    /// accept an item, `Pending` while the channel holds `capacity` items, and
    /// `Ready(Err(SendError(())))` once the receiver is gone. `Pending`
    /// registers the task to be woken when the receiver takes an item or goes
    /// away. Polling reserves nothing, so a caller may poll without sending.
    pub(super) fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), SendError<()>>> {
        self.inner.poll_ready(cx).map_err(|_| SendError(()))
    }

    /// Queue `item` after [`poll_ready`](Self::poll_ready) reported room.
    ///
    /// `waited` says whether the send found the channel full when it
    /// arrived, which test builds report to the process's send hook.
    ///
    /// # Errors
    ///
    /// Returns the item if the receiver is gone, or if the channel is full
    /// because the caller skipped `poll_ready`.
    pub(super) fn start_send(&mut self, item: T, waited: bool) -> Result<(), SendError<T>> {
        let sent = self
            .queue(item)
            .map_err(|error| SendError(error.into_inner()));
        if sent.is_ok() {
            self.report(waited);
        }
        sent
    }

    /// Hand `item` to the channel, which allocates its node, as work for
    /// the place of this sender's edge when it has one.
    fn queue(&mut self, item: T) -> Result<(), mpsc::TrySendError<T>> {
        #[cfg(any(test, feature = "test-internals"))]
        if let Some(holding) = self.kind.and_then(QueueKind::holding) {
            return holding.create(|| self.inner.try_send(item));
        }
        self.inner.try_send(item)
    }

    /// Name the edge this sender serves: attribute each send's channel node
    /// to its place ([`QueueKind::holding`]), and report each send to the
    /// process's send hook ([`observe_edge_sends`]) under its name.
    #[cfg(any(test, feature = "test-internals"))]
    pub fn edge(self, kind: QueueKind) -> Self {
        Self {
            kind: Some(kind),
            ..self
        }
    }

    /// Report one completed send to the process's send hook, if one is
    /// installed and this sender names its edge.
    ///
    /// Whether the send parked its sender is read from the channel itself,
    /// right after the send, by polling for room with a waker that does
    /// nothing. That poll changes nothing the channel does next. A sender
    /// that did not park answers without touching the channel's state. A
    /// sender that parked holds no waker, since parking clears it, so the
    /// poll stores the inert waker in that empty slot: the receiver's next
    /// take, which unparks the sender, wakes it and so wakes nothing, as it
    /// would have woken nothing before, and a later send that finds the
    /// sender still parked registers its own task in its place.
    ///
    /// The reading is exact when the receiver runs on the sending thread,
    /// or is idle while the send completes. A receiver on another thread
    /// that takes the item between the send and the poll unparks the
    /// sender first, and the send then reads as not parked.
    #[cfg(any(test, feature = "test-internals"))]
    fn report(&mut self, waited: bool) {
        let (Some(kind), Some(hook)) = (self.kind, EDGE_SEND_HOOK.get()) else {
            return;
        };
        let parked = self
            .inner
            .poll_ready(&mut Context::from_waker(Waker::noop()))
            .is_pending();
        hook(EdgeSend {
            kind,
            waited,
            parked,
        });
    }

    /// Report one completed send: nothing outside test builds.
    #[cfg(not(any(test, feature = "test-internals")))]
    #[inline(always)]
    fn report(&mut self, _waited: bool) {}

    /// Send `item` if the channel has room, without waiting.
    ///
    /// # Errors
    ///
    /// Returns [`TrySendError::Closed`] with the item if the receiver is
    /// gone, whether or not the channel was full when it went, and
    /// [`TrySendError::Full`] with the item if the receiver is present and
    /// the channel holds `capacity` items.
    pub fn try_send(&mut self, item: T) -> Result<(), TrySendError<T>> {
        let sent = self.queue(item).map_err(|error| {
            if error.is_disconnected() {
                TrySendError::Closed(error.into_inner())
            } else {
                TrySendError::Full(error.into_inner())
            }
        });
        if sent.is_ok() {
            self.report(false);
        }
        sent
    }

    /// Send `item`, waiting while the channel is full.
    ///
    /// # Errors
    ///
    /// Returns the item if the receiver is gone, whether it was gone on
    /// arrival or went away while the send waited.
    pub async fn send(&mut self, item: T) -> Result<(), SendError<T>> {
        self.send_with(item, || ()).await
    }

    /// Send `item` as [`send`](Self::send) does, calling `on_full` first if
    /// the channel is full when the send arrives.
    ///
    /// `on_full` runs at most once, before the send waits. A channel that is
    /// closed on arrival fails the send without calling it.
    ///
    /// # Errors
    ///
    /// Returns the item if the receiver is gone, as [`send`](Self::send)
    /// does.
    pub async fn send_with(&mut self, item: T, on_full: impl FnOnce()) -> Result<(), SendError<T>> {
        let mut on_full = Some(on_full);
        // Only test builds report whether the send waited, so only they
        // keep the flag across the wait; production futures stay as large
        // as they would be without the send hook.
        #[cfg(any(test, feature = "test-internals"))]
        let mut waited = false;
        let ready = poll_fn(|cx| {
            let ready = self.poll_ready(cx);
            if let Some(on_full) = on_full.take()
                && ready.is_pending()
            {
                #[cfg(any(test, feature = "test-internals"))]
                {
                    waited = true;
                }
                on_full();
            }
            ready
        })
        .await;
        #[cfg(not(any(test, feature = "test-internals")))]
        let waited = false;
        match ready {
            Ok(()) => self.start_send(item, waited),
            Err(SendError(())) => Err(SendError(item)),
        }
    }
}

/// What one completed send on a protocol edge did, as the process's send
/// hook sees it.
///
/// `futures-channel` parks a sender on the send that *fills* its channel:
/// it boxes a node holding the sender's task, takes the task's lock, and
/// the receiver's next take wakes it. A send into an edge that still has
/// room after it costs only the item's own node.
#[cfg(any(test, feature = "test-internals"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EdgeSend {
    /// The edge the item was sent on.
    pub kind: QueueKind,
    /// Whether the send found the edge full when it arrived, and waited for
    /// the receiver to take an item before it could proceed.
    pub waited: bool,
    /// Whether the send filled the edge, so that `futures-channel` parked
    /// the sender.
    pub parked: bool,
}

/// The process's send observation hook, called after every completed send
/// on a protocol edge.
#[cfg(any(test, feature = "test-internals"))]
pub type EdgeSendHook = fn(EdgeSend);

/// The hook every protocol edge in the process reports its sends through,
/// once installed.
#[cfg(any(test, feature = "test-internals"))]
static EDGE_SEND_HOOK: OnceLock<EdgeSendHook> = OnceLock::new();

/// Install `hook` as the process's send observation hook, returning whether
/// it is now the installed one.
///
/// The first installation wins for the life of the process, as for the
/// ledger and multiplexer observation hooks. Only senders that name their
/// edge report, which every protocol edge's sender does in test builds.
#[cfg(any(test, feature = "test-internals"))]
pub fn observe_edge_sends(hook: EdgeSendHook) -> bool {
    std::ptr::fn_addr_eq(*EDGE_SEND_HOOK.get_or_init(|| hook), hook)
}

/// The receiving half of a protocol edge: the edge's only consumer.
pub struct Receiver<T>(mpsc::Receiver<T>);

impl<T> Receiver<T> {
    /// Receive the next item, or `None` once the sender is dropped and every
    /// queued item has been received.
    pub async fn recv(&mut self) -> Option<T> {
        self.0.next().await
    }

    /// Poll for the next item, as [`recv`](Self::recv) would return it.
    pub fn poll_recv(&mut self, cx: &mut Context<'_>) -> Poll<Option<T>> {
        self.0.poll_next_unpin(cx)
    }

    /// Refuse every further item, then take one already queued without
    /// waiting, or `None` when nothing was queued.
    ///
    /// After this call each send fails as it would after the receiver is
    /// dropped, so every item is either returned by a take or refused at
    /// its sender; none is dropped unseen.
    pub fn close_and_try_recv(&mut self) -> Option<T> {
        self.0.close();
        self.0.try_recv().ok()
    }
}

impl<T> Stream for Receiver<T> {
    type Item = T;

    /// Poll for the next item, as [`Receiver::recv`] would return it.
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<T>> {
        self.poll_recv(cx)
    }
}

/// The capacity of an edge whose sends never wait: the largest the
/// `futures-channel` buffer accepts, a count of items no session can hold.
///
/// An edge built with it is bounded only by what its items' producers
/// can create, which an edge chooses when another account (the ledger, for
/// the protocol's edges) already limits those items.
pub const UNBOUNDED: usize = usize::MAX >> 2;

/// The capacity of an edge that hands one item at a time from its producer
/// to its consumer: two.
///
/// Such an edge's progress argument asks for one slot and holds at any
/// larger capacity, so the second slot is for speed alone. `futures-channel`
/// parks a sender on the send that *fills* its channel: it boxes a second
/// node holding the sender's task, takes the task's lock, and the
/// receiver's next take wakes the task. A send into a one-slot edge always
/// fills it, so every send would park. With a second slot, a send that
/// finds the edge empty leaves room behind it and completes without
/// parking; only a send that finds the previous item still queued parks.
///
/// An edge built with it can hold a second item, so every count of what
/// such an edge holds counts two.
pub const HANDOFF_CAPACITY: usize = 2;

/// Create a protocol edge that holds at most `capacity` items.
///
/// # Panics
///
/// Panics if `capacity` is zero or exceeds [`UNBOUNDED`]. A protocol edge needs at least one slot to
/// make progress, so a zero capacity is a programmer error, as it is for
/// Tokio's bounded channel.
pub fn channel<T>(capacity: usize) -> (Sender<T>, Receiver<T>) {
    assert!(capacity > 0, "a protocol channel needs at least one slot");
    assert!(
        capacity <= UNBOUNDED,
        "a protocol channel holds at most `UNBOUNDED` items"
    );
    // The single sender adds its own slot to the buffer (see the module docs).
    let (sender, receiver) = mpsc::channel(capacity - 1);
    let sender = Sender {
        inner: sender,
        #[cfg(any(test, feature = "test-internals"))]
        kind: None,
    };
    (sender, Receiver(receiver))
}
