//! Executor-agnostic driving of independently runnable protocol work.

use std::future::Future;

use futures::{StreamExt, future, future::BoxFuture, stream::FuturesUnordered};

#[cfg(test)]
mod tests;

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

/// Drive registered work and its terminal operation until both succeed or
/// either fails. A failure drops all remaining work.
///
/// Poll the task set first so a task error it reports wins over a terminal
/// error caused by its closed channels. A fixed poll order also lets
/// in-memory tests replay a cancellation point.
pub async fn complete<O, E>(
    tasks: Vec<BoxFuture<'static, Result<(), E>>>,
    finish: impl Future<Output = Result<O, E>>,
) -> Result<O, E> {
    future::try_join(try_run_all(tasks), finish)
        .await
        .map(|((), output)| output)
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
