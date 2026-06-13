use std::pin::Pin;
use std::task::{Context, Poll};

use futures::Stream;
use tokio::sync::watch;

use crate::Version;

use super::acausal::Channel;

/// A content-free observer of a [`Rumors`](crate::Rumors) set: one `()` per
/// observed change.
///
/// This is the wake-up signal for anything that reacts to "the set changed"
/// without consuming the changes themselves. Above all, this is useful as the
/// `when` input to [`gossip_when`](crate::Rumors::gossip_when), but equally a
/// persist-on-change loop or a UI refresh. For the changes *themselves*, use
/// [`Messages`](crate::Messages) or [`CausalMessages`](crate::CausalMessages).
///
/// # Ticks are a signal, not a ledger
///
/// The stream is *coalescing*: it yields one `()` for everything that happened
/// since the previous poll, however many commits that was, and it yields
/// immediately on first poll (a fresh observer has seen nothing, so whatever
/// the set holds is news). Consequently the number of ticks means nothing; only
/// "at least one tick since I last looked" does. Every change fires it — local
/// [`send`](crate::Rumors::send)s and [`redact`](crate::Rumors::redact)s, and
/// anything learned by [`gossip`](crate::Rumors::gossip).
///
/// The stream ends (`None`) once the [`Peer`](crate::Peer) and every
/// [`Rumors`](crate::Rumors) for the set have dropped and no further change is
/// possible. Like the message observers, holding a `Changes` does not count
/// against the quiescence that lets
/// [`try_into_peer`](crate::Rumors::try_into_peer) reclaim the `Peer`.
///
/// # This signal alone does not make a gossip driver
///
/// `loop { changes.next().await; gossip(..).await }` on both ends of a
/// connection deadlocks: each side's `gossip` leads with its preamble and then
/// waits for the peer's, so the side whose set did *not* change never answers.
/// A driver must also enter a session when the *remote* initiates, which is
/// exactly what [`gossip_when`](crate::Rumors::gossip_when) adds; feed this
/// stream to it rather than calling `gossip` yourself to gossip-on-change.
pub struct Changes<T> {
    /// The watch channel, or the in-flight wait for it to change; the same
    /// materialized-wait dance as [`Messages`](crate::Messages) (see its
    /// `channel` field docs for why the wait must own the receiver).
    channel: Option<Channel<T>>,
    /// The frontier most recently reported to the consumer: `None` until the
    /// first yield, so the first poll always finds news.
    seen: Option<Version>,
}

impl<T> Changes<T> {
    pub(crate) fn subscribe(inner: &watch::Sender<crate::Inner<T>>) -> Self {
        Self {
            channel: Some(Channel::Ready(inner.subscribe())),
            seen: None,
        }
    }
}

/// `T: 'static` because the quiet-period wait is materialized as an owned
/// future, exactly as in [`Messages`](crate::Messages).
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
