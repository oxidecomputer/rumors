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

/// A future that self-wakes on every poll without ever completing
/// exhausts the poll budget and is reported as such, not as a stall.
#[test]
fn runaway_self_waking_exhausts_the_poll_budget() {
    let runaway = std::future::poll_fn(|cx: &mut Context<'_>| {
        cx.waker().wake_by_ref();
        Poll::<()>::Pending
    });
    assert_eq!(run_to_quiescence(runaway), Err(Quiescence::PollBudget));
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
