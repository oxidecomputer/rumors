//! Wakeup and result preservation for scheduled backend polls.

use std::future;
use std::pin::pin;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::task::{Context, Poll, Wake, Waker};

use futures::{Future, Stream};

use super::{future as delayed_future, stream as delayed_stream, with_schedule};
use crate::testing::schedule::MAX_SCHEDULED_DELAY;

/// Count wakeups requested by the delayed wrappers.
struct WakeCount(AtomicUsize);

/// Record every requested wakeup.
impl Wake for WakeCount {
    /// Count a wakeup that consumes the waker.
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }

    /// Count a wakeup through a shared waker.
    fn wake_by_ref(self: &Arc<Self>) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}

/// Every permitted delay self-wakes once per suspension and preserves both outputs.
#[test]
fn every_bounded_delay_self_wakes_then_completes() {
    let wakes = Arc::new(WakeCount(AtomicUsize::new(0)));
    let waker = Waker::from(wakes.clone());
    let mut cx = Context::from_waker(&waker);

    for delay in 0..=MAX_SCHEDULED_DELAY {
        with_schedule(vec![delay, delay], || {
            let mut future = pin!(delayed_future(future::ready(7)));
            for _ in 0..delay {
                assert!(matches!(future.as_mut().poll(&mut cx), Poll::Pending));
            }
            assert_eq!(future.as_mut().poll(&mut cx), Poll::Ready(7));

            let mut stream = pin!(delayed_stream(futures::stream::iter([9])));
            for _ in 0..delay {
                assert!(matches!(stream.as_mut().poll_next(&mut cx), Poll::Pending));
            }
            assert_eq!(stream.as_mut().poll_next(&mut cx), Poll::Ready(Some(9)));
        });
    }
    let expected_wakes = 2 * (0..=MAX_SCHEDULED_DELAY).map(usize::from).sum::<usize>();
    assert_eq!(wakes.0.load(Ordering::Relaxed), expected_wakes);
}
