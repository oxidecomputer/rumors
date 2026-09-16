//! Shared waiting state for replica-content observers.
//!
//! A watch receiver cannot lend itself to `changed()` across stream polls.
//! This state machine moves the receiver into an owned future so its waker
//! registration survives until a change arrives, then returns the receiver to
//! the observer.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::sync::watch;

/// An owned wait for a replica change, returning its receiver when ready.
type WaitForChange<T> =
    Pin<Box<dyn Future<Output = (bool, watch::Receiver<crate::Inner<T>>)> + Send>>;

/// Holds an observer's receiver while it is ready, waiting, or closed.
pub(super) enum Channel<T> {
    /// The observer may inspect the replica's latest state.
    Ready(watch::Receiver<crate::Inner<T>>),
    /// The observer has registered its waker and awaits a change.
    Waiting(WaitForChange<T>),
    /// Every sender has dropped, so no later state can arrive.
    Closed,
}

/// Creates the receiver shared by each content-observer implementation.
impl<T> Channel<T> {
    /// Subscribes to `inner`, initially ready to inspect its current state.
    pub(super) fn subscribe(inner: &watch::Sender<crate::Inner<T>>) -> Self {
        Self::Ready(inner.subscribe())
    }
}

/// Moves content observers between ready, waiting, and terminal states.
impl<T: Send + Sync + 'static> Channel<T> {
    /// Returns the receiver once ready, or `None` once permanently closed.
    pub(super) fn poll_receiver(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Option<&mut watch::Receiver<crate::Inner<T>>>> {
        loop {
            match self {
                Self::Ready(receiver) => return Poll::Ready(Some(receiver)),
                Self::Waiting(wait) => {
                    let (closed, receiver) = std::task::ready!(wait.as_mut().poll(cx));
                    *self = if closed {
                        Self::Closed
                    } else {
                        Self::Ready(receiver)
                    };
                }
                Self::Closed => return Poll::Ready(None),
            }
        }
    }

    /// Registers a persistent wait after the observer has consumed current state.
    ///
    /// The future owns the receiver so its waker registration survives until
    /// the observer is polled again.
    pub(super) fn wait(&mut self) {
        let Self::Ready(mut receiver) = std::mem::replace(self, Self::Closed) else {
            unreachable!("poll_receiver returned a non-ready state");
        };
        *self = Self::Waiting(Box::pin(async move {
            let closed = receiver.changed().await.is_err();
            (closed, receiver)
        }));
    }
}
