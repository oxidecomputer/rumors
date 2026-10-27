//! The bounded single-producer channel behind every protocol edge.
//!
//! Every edge in the streaming protocol has exactly one sender and one
//! receiver, and one session task polls both. [`channel`] builds such an edge
//! on the bounded `mpsc` channel of `futures-channel`, whose shared state has
//! ordinary alignment. Tokio's bounded channel would serve the same edge, but
//! it pads its shared state to 128 bytes, which separates producer and
//! consumer cache lines for threads that run concurrently. An edge's two ends
//! never do, and the system allocator must serve every over-aligned block
//! through the platform's aligned-allocation entry point, which illumos's
//! libumem serializes behind one lock shared by every thread.
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
use std::task::{Context, Poll};

use futures::{Stream, StreamExt as _, channel::mpsc, future::poll_fn};

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

/// The sending half of a protocol edge: the edge's only producer.
pub struct Sender<T>(mpsc::Sender<T>);

impl<T> Sender<T> {
    /// Poll for room to send one item.
    ///
    /// Returns `Ready(Ok(()))` when [`start_send`](Self::start_send) will
    /// accept an item, `Pending` while the channel holds `capacity` items, and
    /// `Ready(Err(SendError(())))` once the receiver is gone. `Pending`
    /// registers the task to be woken when the receiver takes an item or goes
    /// away. Polling reserves nothing, so a caller may poll without sending.
    pub(super) fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), SendError<()>>> {
        self.0.poll_ready(cx).map_err(|_| SendError(()))
    }

    /// Queue `item` after [`poll_ready`](Self::poll_ready) reported room.
    ///
    /// # Errors
    ///
    /// Returns the item if the receiver is gone, or if the channel is full
    /// because the caller skipped `poll_ready`.
    pub(super) fn start_send(&mut self, item: T) -> Result<(), SendError<T>> {
        self.0
            .try_send(item)
            .map_err(|error| SendError(error.into_inner()))
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
        let ready = poll_fn(|cx| {
            let ready = self.poll_ready(cx);
            if let Some(on_full) = on_full.take()
                && ready.is_pending()
            {
                on_full();
            }
            ready
        })
        .await;
        match ready {
            Ok(()) => self.start_send(item),
            Err(SendError(())) => Err(SendError(item)),
        }
    }
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
}

impl<T> Stream for Receiver<T> {
    type Item = T;

    /// Poll for the next item, as [`Receiver::recv`] would return it.
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<T>> {
        self.poll_recv(cx)
    }
}

/// Create a protocol edge that holds at most `capacity` items.
///
/// # Panics
///
/// Panics if `capacity` is zero. A protocol edge needs at least one slot to
/// make progress, so a zero capacity is a programmer error, as it is for
/// Tokio's bounded channel.
pub fn channel<T>(capacity: usize) -> (Sender<T>, Receiver<T>) {
    assert!(capacity > 0, "a protocol channel needs at least one slot");
    // The single sender adds its own slot to the buffer (see the module docs).
    let (sender, receiver) = mpsc::channel(capacity - 1);
    (Sender(sender), Receiver(receiver))
}
