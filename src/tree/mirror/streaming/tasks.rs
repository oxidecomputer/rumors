//! Executor-agnostic driving of independently runnable protocol work.

use std::{future::Future, pin::pin};

use futures::{StreamExt, future, future::BoxFuture, stream::FuturesUnordered};

/// Run every task, cancelling the remainder as soon as any task fails.
///
/// Completion order is deliberately unordered: an error from a later task
/// must not wait behind an earlier parked task.
async fn try_run_all<E>(tasks: Vec<BoxFuture<'static, Result<(), E>>>) -> Result<(), E> {
    let mut tasks = tasks.into_iter().collect::<FuturesUnordered<_>>();
    while let Some(result) = tasks.next().await {
        result?;
    }
    Ok(())
}

/// Race registered work against its terminal operation, failing on either.
pub async fn complete<O, E>(
    tasks: Vec<BoxFuture<'static, Result<(), E>>>,
    finish: impl Future<Output = Result<O, E>>,
) -> Result<O, E> {
    let mut tasks = pin!(try_run_all(tasks));
    let mut finish = pin!(finish);
    tokio::select! {
        finished = &mut tasks => {
            finished?;
            finish.await
        }
        output = &mut finish => {
            let output = output?;
            tasks.await?;
            Ok(output)
        }
    }
}

/// Retain cancellation-sensitive resources until their owner is dropped.
pub async fn cancelled() -> ! {
    loop {
        future::pending::<()>().await;
    }
}

/// Park after publishing an error so it cannot be followed by successful EOF.
pub async fn park_after_published_error(failed: bool) {
    if failed {
        cancelled().await;
    }
}

/// Return the next item or await cancellation after its producer disappears.
pub async fn next_or_cancelled<T>(next: impl Future<Output = Option<T>>) -> T {
    match next.await {
        Some(item) => item,
        None => cancelled().await,
    }
}
