//! Executor-agnostic test support shared across protocol and API suites.

mod transport;

pub use transport::{
    AdversarialRead, AdversarialWrite, FaultUnit as IoFaultUnit, InjectedIo, IoFault, IoPlan,
    IoReport, IoReportHandle, Operation as IoOperation, Side as IoSide, wrap_io,
};

/// Render captured V2 directions grouped by deterministic logical streams.
pub fn render_v2_capture(a_to_b: &[u8], b_to_a: &[u8]) -> String {
    crate::tree::mirror::streaming::remote::render_v2_capture(a_to_b, b_to_a)
}

use std::{
    future::Future,
    pin::pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll, Wake, Waker},
};

/// Why polling stopped before a closed in-memory future completed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Quiescence {
    /// The future returned `Pending` without arranging another poll.
    Stalled,
    /// The future kept self-waking beyond the runaway guard.
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

/// Poll a closed, in-memory future until it completes or becomes quiescent.
///
/// Every legitimate suspension must arrange another wake. A `Pending` poll
/// without one is therefore a deterministic deadlock witness rather than a
/// wall-clock guess. Futures waiting on external events do not satisfy this
/// closed-world premise and should use their real liveness mechanism instead.
/// Tokio's cooperative budget is disabled around the subject so that invoking
/// this detector from within a Tokio task cannot turn a scheduler yield into a
/// false deadlock report.
pub fn run_to_quiescence<F: Future>(future: F) -> Result<F::Output, Quiescence> {
    const MAX_POLLS: usize = 1_000_000;

    let wake = Arc::new(WakeFlag(AtomicBool::new(true)));
    let waker = Waker::from(wake.clone());
    let mut cx = Context::from_waker(&waker);
    let mut future = pin!(tokio::task::coop::unconstrained(future));

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

#[cfg(test)]
mod tests {
    use super::*;

    /// A self-wake is progress while a permanently parked future is stalled.
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

    /// An inherited Tokio task budget cannot masquerade as protocol quiescence.
    #[tokio::test(flavor = "current_thread")]
    async fn ignores_tokio_cooperative_yields() {
        const ITEMS: usize = 256;

        let (send, mut receive) = tokio::sync::mpsc::channel(ITEMS);
        for item in 0..ITEMS {
            send.try_send(item).expect("channel has room");
        }

        let received = run_to_quiescence(async move {
            let mut items = Vec::with_capacity(ITEMS);
            while let Some(item) = receive.recv().await {
                items.push(item);
                if items.len() == ITEMS {
                    break;
                }
            }
            items
        });

        assert_eq!(received, Ok((0..ITEMS).collect()));
    }
}
