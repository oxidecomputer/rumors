//! Deterministic deadlock detection for closed, in-memory sessions.

use std::{
    future::Future,
    pin::pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll, Wake, Waker},
};

/// Why polling stopped before the session completed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Quiescence {
    /// The future returned `Pending` without arranging another poll.
    Stalled,
    /// The future kept self-waking beyond the test's runaway guard.
    PollBudget,
}

struct WakeFlag(AtomicBool);

impl Wake for WakeFlag {
    fn wake(self: Arc<Self>) {
        self.0.store(true, Ordering::Release);
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.0.store(true, Ordering::Release);
    }
}

/// Poll a closed, in-memory session until it completes or becomes quiescent.
///
/// Every legitimate suspension is paired with a synchronous channel wake or a
/// test-injected self-wake. A `Pending` poll with no wake is therefore a
/// deterministic deadlock witness rather than a wall-clock guess.
pub fn run_to_quiescence<F: Future>(future: F) -> Result<F::Output, Quiescence> {
    const MAX_POLLS: usize = 1_000_000;

    let wake = Arc::new(WakeFlag(AtomicBool::new(true)));
    let waker = Waker::from(wake.clone());
    let mut cx = Context::from_waker(&waker);
    let mut future = pin!(future);

    for _ in 0..MAX_POLLS {
        wake.0.store(false, Ordering::Release);
        match future.as_mut().poll(&mut cx) {
            Poll::Ready(output) => return Ok(output),
            Poll::Pending if !wake.0.swap(false, Ordering::AcqRel) => {
                return Err(Quiescence::Stalled);
            }
            Poll::Pending => {}
        }
    }
    Err(Quiescence::PollBudget)
}

/// Quiescence distinguishes a self-wake from a permanently parked future.
#[test]
fn observes_wake_contract() {
    let mut first = true;
    let self_waking = std::future::poll_fn(move |cx| {
        if std::mem::take(&mut first) {
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(7)
        }
    });
    assert_eq!(run_to_quiescence(self_waking), Ok(7));
    assert_eq!(
        run_to_quiescence(std::future::pending::<()>()),
        Err(Quiescence::Stalled),
    );
}
