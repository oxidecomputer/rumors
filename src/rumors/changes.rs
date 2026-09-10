use std::pin::Pin;
use std::task::{Context, Poll};

use futures::Stream;
use tokio::sync::watch;

use crate::Version;

use super::unordered::Channel;

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
/// insertions, redactions, and new state learned through gossip. Transferring
/// identity alone does not count as a change.
///
/// The stream ends (`None`) once the [`Peer`](crate::Peer) and every
/// [`Rumors`](crate::Rumors) handle for the replica have dropped and the final
/// change has been reported. Holding this observer does not prevent
/// [`try_into_peer`](crate::Rumors::try_into_peer) from recovering the `Peer`.
///
/// # Driving gossip
///
/// Pass this stream as the `when` input to [`gossip_when`](crate::Rumors::gossip_when),
/// which also answers sessions initiated by the remote peer. A loop that waits
/// for a local change before calling `gossip` can deadlock: a replica with no
/// local changes never enters the session its peer is waiting to start.
pub struct Changes<T> {
    /// The watch channel, or the in-flight wait for it to change; the same
    /// materialized-wait dance as [`UnorderedMessages`](crate::UnorderedMessages) (see its
    /// `channel` field docs for why the wait must own the receiver).
    channel: Option<Channel<T>>,
    /// The frontier most recently reported to the consumer: `None` until the
    /// first yield, so the first poll always finds news.
    ///
    /// Content changes advance this frontier, including redactions, so an equal
    /// frontier means there is no new change to report.
    seen: Option<Version>,
}

/// The outcome of [`Changes::try_next`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

impl<T> Changes<T> {
    /// Subscribe to the replica, reporting its current state on the first poll.
    pub(crate) fn subscribe(inner: &watch::Sender<crate::Inner<T>>) -> Self {
        Self {
            channel: Some(Channel::Ready(inner.subscribe())),
            seen: None,
        }
    }

    /// Await the next coalesced change, sharing the [`Stream`] state machine.
    pub(crate) async fn next_inner(&mut self) -> Option<()>
    where
        T: Send + Sync + 'static,
    {
        loop {
            match self.channel.as_mut().expect("channel state present") {
                Channel::Waiting(wait) => {
                    let (closed, rx) = wait.as_mut().await;
                    self.channel = Some(Channel::Ready(rx));
                    if closed {
                        return None;
                    }
                }
                Channel::Ready(rx) => {
                    let latest = rx.borrow_and_update().tree.latest().clone();
                    if self.seen.as_ref() != Some(&latest) {
                        self.seen = Some(latest);
                        return Some(());
                    }
                    // Frontier unchanged since the last report: await the next
                    // change. `Err` means every sender is gone and the
                    // comparison above already saw the final state.
                    if rx.changed().await.is_err() {
                        return None;
                    }
                }
            }
        }
    }
    /// Take one non-blocking step: [`Tick`] if the set advanced since the
    /// last report, [`Quiet`] (ask again later) if not, [`Ended`] if no
    /// further change is possible.
    ///
    /// [`Tick`]: TryTick::Tick
    /// [`Quiet`]: TryTick::Quiet
    /// [`Ended`]: TryTick::Ended
    pub fn try_next(&mut self) -> TryTick
    where
        T: Send + Sync + 'static,
    {
        use futures::FutureExt;
        match self.next_inner().now_or_never() {
            None => TryTick::Quiet,
            Some(None) => TryTick::Ended,
            Some(Some(())) => TryTick::Tick,
        }
    }
}

/// `T: 'static` because the quiet-period wait is materialized as an owned
/// future, exactly as in [`UnorderedMessages`](crate::UnorderedMessages).
impl<T: Send + Sync + 'static> Stream for Changes<T> {
    type Item = ();

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        loop {
            match this.channel.as_mut().expect("channel state present") {
                Channel::Waiting(wait) => match wait.as_mut().poll(cx) {
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready((closed, rx)) => {
                        this.channel = Some(Channel::Ready(rx));
                        if closed {
                            // Every sender is gone, and the comparison below
                            // already ran against the final state before this
                            // wait began: nothing further to report.
                            return Poll::Ready(None);
                        }
                    }
                },
                Channel::Ready(rx) => {
                    let latest = rx.borrow_and_update().tree.latest().clone();
                    if this.seen.as_ref() != Some(&latest) {
                        this.seen = Some(latest);
                        return Poll::Ready(Some(()));
                    }

                    // Frontier unchanged since the last report: enter the
                    // owned wait (the receiver rides inside the future and
                    // comes back with the result).
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
