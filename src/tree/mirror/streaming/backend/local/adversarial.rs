//! Test-only adversarial polling for the in-memory backend.

use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::Stream;

/// One typed asynchronous surface of [`Local`](super::Local).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Role {
    Children { height: usize },
    Parent { height: usize },
}

struct Schedule {
    delays: Vec<u8>,
    step: usize,
}

std::thread_local! {
    static SCHEDULE: RefCell<Option<Schedule>> = const { RefCell::new(None) };
}

/// Run `f` with an explicit sequence of delays at Local backend poll boundaries.
pub fn with_schedule<R>(delays: Vec<u8>, f: impl FnOnce() -> R) -> R {
    struct Restore(Option<Schedule>);

    impl Drop for Restore {
        fn drop(&mut self) {
            SCHEDULE.with(|schedule| schedule.replace(self.0.take()));
        }
    }

    let previous = SCHEDULE.with(|schedule| schedule.replace(Some(Schedule { delays, step: 0 })));
    let _restore = Restore(previous);
    f()
}

/// Delay every poll of one Local backend future according to the schedule.
pub(super) fn future<F: Future>(role: Role, future: F) -> impl Future<Output = F::Output> {
    DelayedFuture {
        inner: Box::pin(future),
        role,
        delay: None,
    }
}

/// Delay every poll of one Local backend stream according to the schedule.
pub(super) fn stream<S: Stream>(role: Role, stream: S) -> impl Stream<Item = S::Item> {
    DelayedStream {
        inner: Box::pin(stream),
        role,
        delay: None,
    }
}

struct DelayedFuture<F> {
    inner: Pin<Box<F>>,
    role: Role,
    delay: Option<u8>,
}

impl<F: Future> Future for DelayedFuture<F> {
    type Output = F::Output;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let role = self.role;
        if delay(&mut self.delay, role, cx) {
            return Poll::Pending;
        }
        self.inner.as_mut().poll(cx)
    }
}

struct DelayedStream<S> {
    inner: Pin<Box<S>>,
    role: Role,
    delay: Option<u8>,
}

impl<S: Stream> Stream for DelayedStream<S> {
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let role = self.role;
        if delay(&mut self.delay, role, cx) {
            return Poll::Pending;
        }
        self.inner.as_mut().poll_next(cx)
    }
}

/// Return true when this poll must suspend before reaching the backend.
fn delay(delay: &mut Option<u8>, role: Role, cx: &mut Context<'_>) -> bool {
    if delay.is_none() {
        *delay = Some(next_delay(role));
    }
    if delay.is_some_and(|delay| delay > 0) {
        *delay = delay.map(|delay| delay - 1);
        cx.waker().wake_by_ref();
        true
    } else {
        *delay = None;
        false
    }
}

fn next_delay(_role: Role) -> u8 {
    SCHEDULE.with(|schedule| {
        let mut schedule = schedule.borrow_mut();
        let Some(current) = schedule.as_mut() else {
            return 0;
        };
        let delay = current.delays.get(current.step).copied().unwrap_or(0);
        current.step += 1;
        delay.min(2)
    })
}

#[cfg(test)]
mod tests {
    use std::future;
    use std::pin::pin;
    use std::task::{Context, Poll, Waker};

    use futures::{Future, Stream};

    use super::{Role, future as delayed_future, stream as delayed_stream, with_schedule};

    #[test]
    fn future_and_stream_delays_self_wake_then_complete() {
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);

        with_schedule(vec![2, 1], || {
            let mut future = pin!(delayed_future(Role::Parent { height: 1 }, future::ready(7)));
            assert!(matches!(future.as_mut().poll(&mut cx), Poll::Pending));
            assert!(matches!(future.as_mut().poll(&mut cx), Poll::Pending));
            assert_eq!(future.as_mut().poll(&mut cx), Poll::Ready(7));

            let mut stream = pin!(delayed_stream(
                Role::Children { height: 0 },
                futures::stream::iter([9]),
            ));
            assert!(matches!(stream.as_mut().poll_next(&mut cx), Poll::Pending));
            assert_eq!(stream.as_mut().poll_next(&mut cx), Poll::Ready(Some(9)));
        });
    }
}
