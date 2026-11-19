use std::pin::Pin;
use std::task::{Context, Poll};

use futures::Stream;
use tokio::sync::watch;

use crate::{Inner, Version};

use super::channel::Channel;

/// Reports changes to the set of messages without returning the messages
/// themselves.
///
/// To listen to the messages themselves, use
/// [`UnorderedMessages`](crate::UnorderedMessages) or
/// [`CausalMessages`](crate::CausalMessages).
///
/// # Notifications
///
/// The first poll of this stream will yield `()` immediately, even for a
/// [`Peer`](crate::Peer) with no messages at all. Later polls coalesce all
/// changes since the last poll into one `()`.
///
/// The stream ends once every
/// [`Peer`](crate::Peer) handle for the replica has dropped and the final
/// change has been reported to whoever is consuming the stream.
///
/// Holding this stream open does not prevent
/// [`retire`](crate::Peer::retire).
pub struct Changes<T: Send + Sync + 'static> {
    /// The shared replica receiver and its current wait state.
    channel: Channel<T>,
    /// The frontier most recently reported to the consumer: `None` until the
    /// first yield, so the first poll always finds news.
    ///
    /// Content changes advance this frontier, including redactions, so an equal
    /// frontier means there is no new change to report.
    seen: Option<Version>,
}

/// Debug a change stream without requiring its messages to be debuggable.
impl<T: Send + Sync + 'static> std::fmt::Debug for Changes<T> {
    /// Formats the message frontier most recently reported to the consumer.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Changes")
            .field("seen", &self.seen)
            .finish_non_exhaustive()
    }
}

/// The result of [`Changes::try_next`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TryTick {
    /// The set of messages changed since the last poll.
    Tick,
    /// No changes have happened since the last poll, but it would be possible
    /// for some to happen later, because some handles still exist that could
    /// send one.
    Quiet,
    /// Every handle that could cause a change to occur has been dropped, so no
    /// further changes are possible.
    Ended,
}

/// Non-blocking polling for changes.
impl<T: Send + Sync + 'static> Changes<T> {
    /// Subscribe to the replica, reporting its current state on the first poll.
    pub(crate) fn subscribe(inner: &watch::Sender<Inner<T>>) -> Self {
        Self {
            channel: Channel::subscribe(inner),
            seen: None,
        }
    }

    /// Poll for changes.
    ///
    /// This returns [`Tick`] if the set of messages changed since the last
    /// report, [`Quiet`] (ask again later) if not, and [`Ended`] if no further
    /// change is possible.
    ///
    /// [`Tick`]: TryTick::Tick
    /// [`Quiet`]: TryTick::Quiet
    /// [`Ended`]: TryTick::Ended
    pub fn try_next(&mut self) -> TryTick {
        use futures::{FutureExt, StreamExt};
        match self.next().now_or_never() {
            None => TryTick::Quiet,
            Some(None) => TryTick::Ended,
            Some(Some(())) => TryTick::Tick,
        }
    }
}

/// Yields one unit item for each observed change, coalesced.
impl<T: Send + Sync + 'static> Stream for Changes<T> {
    type Item = ();

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        loop {
            let Some(receiver) = std::task::ready!(this.channel.poll_receiver(cx)) else {
                return Poll::Ready(None);
            };
            let latest = receiver.borrow_and_update().tree.latest().clone();
            if this.seen.as_ref() != Some(&latest) {
                this.seen = Some(latest);
                return Poll::Ready(Some(()));
            }
            this.channel.wait();
        }
    }
}
