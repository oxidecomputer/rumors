//! Bounded channels for the materialized protocol.
//!
//! Production re-exports Tokio's channel types directly. Unit tests substitute
//! a wrapper which preserves Tokio's capacity and wakeup behavior while an
//! explicit, shrinkable schedule may insert `Pending` polls before operations.

#[cfg(not(test))]
pub use tokio::sync::mpsc::{Receiver, Sender, channel};

#[cfg(test)]
pub use instrumented::{Receiver, Sender, channel, with_capacity_limit, with_schedule};

#[cfg(test)]
mod instrumented {
    use std::cell::RefCell;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    use futures::{Stream, future::poll_fn};
    use tokio::sync::mpsc;

    /// The sending half of a bounded channel.
    pub struct Sender<T>(mpsc::Sender<T>);

    impl<T> Clone for Sender<T> {
        fn clone(&self) -> Self {
            Self(self.0.clone())
        }
    }

    impl<T> Sender<T> {
        /// Send one item after the scheduled suspension points.
        pub async fn send(&self, item: T) -> Result<(), mpsc::error::SendError<T>> {
            perturb().await;
            self.0.send(item).await
        }
    }

    /// The receiving half of a bounded channel.
    pub struct Receiver<T> {
        inner: mpsc::Receiver<T>,
        delay: Option<u8>,
    }

    impl<T> Receiver<T> {
        /// Receive one item after the scheduled suspension points.
        pub async fn recv(&mut self) -> Option<T> {
            poll_fn(|cx| self.poll_recv(cx)).await
        }

        fn poll_recv(&mut self, cx: &mut Context<'_>) -> Poll<Option<T>> {
            if self.delay.is_none() {
                self.delay = Some(next_delay());
            }
            if self.delay.is_some_and(|delay| delay > 0) {
                self.delay = self.delay.map(|delay| delay - 1);
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
            self.delay = None;
            Pin::new(&mut self.inner).poll_recv(cx)
        }
    }

    impl<T> Stream for Receiver<T> {
        type Item = T;

        fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            self.poll_recv(cx)
        }
    }

    /// Create a Tokio channel, optionally capping its capacity for a test.
    pub fn channel<T>(capacity: usize) -> (Sender<T>, Receiver<T>) {
        let (sender, receiver) = mpsc::channel(limit_capacity(capacity));
        (
            Sender(sender),
            Receiver {
                inner: receiver,
                delay: None,
            },
        )
    }

    struct Schedule {
        delays: Vec<u8>,
        step: usize,
    }

    std::thread_local! {
        static SCHEDULE: RefCell<Option<Schedule>> = const { RefCell::new(None) };
        static CAPACITY_LIMIT: RefCell<Option<usize>> = const { RefCell::new(None) };
    }

    /// Run `f` with an explicit sequence of delays at channel boundaries.
    pub fn with_schedule<R>(delays: Vec<u8>, f: impl FnOnce() -> R) -> R {
        struct Restore(Option<Schedule>);

        impl Drop for Restore {
            fn drop(&mut self) {
                SCHEDULE.with(|schedule| schedule.replace(self.0.take()));
            }
        }

        let previous =
            SCHEDULE.with(|schedule| schedule.replace(Some(Schedule { delays, step: 0 })));
        let _restore = Restore(previous);
        f()
    }

    /// Run `f` with every requested channel capacity capped at `limit`.
    pub fn with_capacity_limit<R>(limit: usize, f: impl FnOnce() -> R) -> R {
        struct Restore(Option<usize>);

        impl Drop for Restore {
            fn drop(&mut self) {
                CAPACITY_LIMIT.with(|capacity| capacity.replace(self.0.take()));
            }
        }

        assert!(limit > 0);
        let previous = CAPACITY_LIMIT.with(|capacity| capacity.replace(Some(limit)));
        let _restore = Restore(previous);
        f()
    }

    fn limit_capacity(capacity: usize) -> usize {
        CAPACITY_LIMIT.with(|limit| (*limit.borrow()).map_or(capacity, |limit| capacity.min(limit)))
    }

    fn next_delay() -> u8 {
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

    async fn perturb() {
        for _ in 0..next_delay() {
            let mut yielded = false;
            poll_fn(move |cx| {
                if yielded {
                    Poll::Ready(())
                } else {
                    yielded = true;
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
            })
            .await;
        }
    }
}
