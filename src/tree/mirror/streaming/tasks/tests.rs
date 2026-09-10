//! Progress and cancellation when task completion depends on terminal work.

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::task::Poll;

use futures::{FutureExt, channel::oneshot, future};
use proptest::prelude::*;

use super::complete;
use crate::testing::run_to_quiescence;

/// Yield for the requested polls, waking the driver for each remaining step.
async fn delay(mut polls: u8) {
    future::poll_fn(move |cx| {
        if polls == 0 {
            Poll::Ready(())
        } else {
            polls -= 1;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    })
    .await;
}

proptest! {
    /// Tasks and terminal work can wait on each other; success waits for every
    /// task even when it has more work after the terminal operation completes.
    #[test]
    fn work_and_terminal_make_progress_together(
        delays in proptest::collection::vec(0u8..=8, 0..=12),
        terminal_delay in 0u8..=8,
    ) {
        let completed = Arc::new(AtomicUsize::new(0));
        let mut tasks = Vec::new();
        let mut terminals = Vec::new();
        for &polls in &delays {
            let (ready, wait_ready) = oneshot::channel();
            let (resume, wait_resume) = oneshot::channel();
            terminals.push((wait_ready, resume));
            let completed = completed.clone();
            tasks.push(async move {
                delay(polls).await;
                ready.send(()).unwrap();
                wait_resume.await.unwrap();
                delay(polls).await;
                completed.fetch_add(1, Ordering::Relaxed);
                Ok::<_, ()>(())
            }.boxed());
        }
        let finish = async move {
            delay(terminal_delay).await;
            for (ready, resume) in terminals {
                ready.await.unwrap();
                resume.send(()).unwrap();
            }
            Ok(42)
        };
        prop_assert_eq!(run_to_quiescence(complete(tasks, finish)), Ok(Ok(42)));
        prop_assert_eq!(completed.load(Ordering::Relaxed), delays.len());
    }

    /// An error from any task or the terminal operation cancels all siblings
    /// and releases their resources, even when earlier tasks never finish.
    #[test]
    fn any_failure_cancels_stalled_work(
        delays in proptest::collection::vec(0u8..=8, 1..=12),
        failure in any::<usize>(),
    ) {
        let failure = failure % delays.len();
        let held = Arc::new(());
        let mut work: Vec<_> = delays.into_iter().enumerate().map(|(index, polls)| {
            let held = held.clone();
            async move {
                let _held = held;
                delay(polls).await;
                if index == failure {
                    Err(index)
                } else {
                    future::pending::<Result<(), usize>>().await
                }
            }.boxed()
        }).collect();
        let finish = work.pop().unwrap();
        prop_assert_eq!(run_to_quiescence(complete(work, finish)), Ok(Err(failure)));
        prop_assert_eq!(Arc::strong_count(&held), 1);
    }
}

/// A ready task failure is reported before a simultaneous terminal failure.
#[test]
fn task_error_precedes_terminal_error() {
    let tasks = vec![future::ready(Err("task")).boxed()];
    let finish = future::ready(Err::<(), _>("terminal"));
    assert_eq!(run_to_quiescence(complete(tasks, finish)), Ok(Err("task")));
}
