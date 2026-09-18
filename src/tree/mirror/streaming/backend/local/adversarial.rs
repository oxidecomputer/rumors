//! Test-only adversarial polling for the in-memory backend.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::Stream;

use crate::testing::schedule::{BACKEND_SCHEDULE, Countdown};

/// Run `f` with an explicit sequence of delays at Local backend poll boundaries.
pub fn with_schedule<R>(delays: Vec<u8>, f: impl FnOnce() -> R) -> R {
    BACKEND_SCHEDULE.with(delays, f)
}

/// Delay every poll of one Local backend future according to the schedule.
pub(super) fn future<F: Future>(future: F) -> impl Future<Output = F::Output> {
    DelayedFuture {
        inner: Box::pin(future),
        delay: Countdown::default(),
    }
}

/// Delay every poll of one Local backend stream according to the schedule.
pub(super) fn stream<S: Stream>(stream: S) -> impl Stream<Item = S::Item> {
    DelayedStream {
        inner: Box::pin(stream),
        delay: Countdown::default(),
    }
}

/// A backend future with scheduled pending polls.
struct DelayedFuture<F> {
    /// The wrapped backend operation.
    inner: Pin<Box<F>>,
    /// Pending polls left for the current operation.
    delay: Countdown,
}

/// Poll a backend future through its scheduled delay.
impl<F: Future> Future for DelayedFuture<F> {
    type Output = F::Output;

    /// Spend a scheduled delay or poll the wrapped future.
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.delay.suspend(|| BACKEND_SCHEDULE.next(), cx) {
            return Poll::Pending;
        }
        self.delay.clear();
        self.inner.as_mut().poll(cx)
    }
}

/// A backend stream with scheduled pending polls.
struct DelayedStream<S> {
    /// The wrapped backend stream.
    inner: Pin<Box<S>>,
    /// Pending polls left for the current item.
    delay: Countdown,
}

/// Poll a backend stream through its scheduled delay.
impl<S: Stream> Stream for DelayedStream<S> {
    type Item = S::Item;

    /// Spend a scheduled delay or poll the wrapped stream.
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.delay.suspend(|| BACKEND_SCHEDULE.next(), cx) {
            return Poll::Pending;
        }
        self.delay.clear();
        self.inner.as_mut().poll_next(cx)
    }
}

#[cfg(test)]
mod tests;
