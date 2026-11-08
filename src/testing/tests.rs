//! Tests for closed-world quiescence detection.

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

/// A future that self-wakes on every poll without ever completing exhausts the
/// poll budget and is reported as such, not as a stall.
#[test]
fn runaway_self_waking_exhausts_the_poll_budget() {
    let runaway = std::future::poll_fn(|cx: &mut Context<'_>| {
        cx.waker().wake_by_ref();
        Poll::<()>::Pending
    });
    assert_eq!(run_to_quiescence(runaway), Err(Quiescence::PollBudget));
}

/// An inherited Tokio task budget cannot masquerade as protocol quiescence.
///
/// Inside a Tokio task, each unit of cooperative budget a future spends counts
/// against the task. Once the budget runs out, Tokio yields: the future returns
/// pending and defers its wake to the runtime, so the closed-world poller would
/// see a pending poll with no wake and report a stall. Spending twice one
/// budget inside the poller must instead complete.
#[tokio::test(flavor = "current_thread")]
async fn ignores_tokio_cooperative_yields() {
    // Twice Tokio's per-task budget, so a constrained budget would run out.
    const UNITS: usize = 256;

    let spent = run_to_quiescence(async {
        for _ in 0..UNITS {
            tokio::task::coop::consume_budget().await;
        }
        UNITS
    });

    assert_eq!(spent, Ok(UNITS));
}
