use std::pin::Pin;
use std::task::{Context, Poll};

use futures::Stream;
use tokio::sync::watch;

use crate::Version;

use super::channel::Channel;

/// Reports changes to a replica without returning its messages.
///
/// Use this stream to trigger gossip, persist state, or refresh a display.
/// To read the messages themselves, use [`UnorderedMessages`](crate::UnorderedMessages)
/// or [`CausalMessages`](crate::CausalMessages).
///
/// # Notifications
///
/// The first poll yields `()` immediately, even for an empty replica. Later
/// polls coalesce all changes since the last notification into one `()`;
/// notifications cannot be used to count commits. Changes include local
/// insertions, redactions, and new state learned through gossip. Serving a
/// bootstrap or accepting a retirement notifies only if the message set changes.
///
/// The stream ends (`None`) once the [`Peer`](crate::Peer) and every
/// [`Rumors`](crate::Rumors) handle for the replica have dropped and the final
/// change has been reported. Holding this observer does not prevent
/// [`try_into_peer`](crate::Rumors::try_into_peer) from recovering the `Peer`.
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

/// Summarize a change stream without requiring its payloads to be debuggable.
impl<T: Send + Sync + 'static> std::fmt::Debug for Changes<T> {
    /// Formats the frontier most recently reported to the consumer.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Changes")
            .field("seen", &self.seen)
            .finish_non_exhaustive()
    }
}

/// The outcome of [`Changes::try_next`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TryTick {
    /// The set advanced since the last report (a fresh signal's first step
    /// is always a tick).
    Tick,
    /// No advance since the last report; handles are still live, so more
    /// may come. Ask again later.
    Quiet,
    /// Every handle is gone and no further change is possible.
    Ended,
}

/// Creates change subscriptions and exposes non-blocking polling.
impl<T: Send + Sync + 'static> Changes<T> {
    /// Subscribe to the replica, reporting its current state on the first poll.
    pub(crate) fn subscribe(inner: &watch::Sender<crate::Inner<T>>) -> Self {
        Self {
            channel: Channel::subscribe(inner),
            seen: None,
        }
    }

    /// Take one non-blocking step: [`Tick`] if the set advanced since the
    /// last report, [`Quiet`] (ask again later) if not, [`Ended`] if no
    /// further change is possible.
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

/// Yields one unit item for each observed advance.
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
