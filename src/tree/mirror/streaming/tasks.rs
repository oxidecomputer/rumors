//! Executor-agnostic driving of independently runnable protocol work.
//!
//! A session runs its work as tasks inside one future, so whoever polls the
//! session cannot choose which task runs. A backend can: an operation that
//! waits on storage holds back the task that calls it while every other task
//! proceeds. With `test-internals`, a test can pace each task by its
//! [`TaskRole`] the same way: a hook installed with [`pace_tasks`] holds back
//! any task [`TaskRole::paced`] marks, to drive a session's queues into the
//! states a slow backend's latencies would.

use std::future::Future;
#[cfg(any(test, feature = "test-internals"))]
use std::{future::poll_fn, pin::pin, sync::OnceLock, task::Poll, task::Waker};

use futures::{StreamExt, future, future::BoxFuture, stream::FuturesUnordered};

/// What one of a session's tasks does, for a test that holds tasks back.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum TaskRole {
    /// A walk stage, which takes the peer's replies, answers the questions
    /// in them with replies at typed height `height`, and asks its own.
    Walk {
        /// The typed height of the replies the stage sends.
        height: usize,
    },
    /// An assembler completing resolutions whose children sit at typed
    /// height `height`.
    Assembly {
        /// The typed height of the children it assembles.
        height: usize,
    },
    /// The proxy's encoder of this side's replies at one height.
    Encoder,
    /// The proxy's decoder and parking of the peer's replies at typed height
    /// `height`.
    Decoder {
        /// The typed height of the replies it decodes.
        height: usize,
    },
}

/// The process's task pacer: whether a task of the given role may run now.
/// A pacer that answers no keeps the waker and wakes it once the task may
/// run.
#[cfg(any(test, feature = "test-internals"))]
pub type Pacer = fn(TaskRole, &Waker) -> bool;

/// The pacer every session in the process consults, once installed.
#[cfg(any(test, feature = "test-internals"))]
static PACER: OnceLock<Pacer> = OnceLock::new();

/// Install `pacer` as the process's task pacer, returning whether it is now
/// the installed one.
///
/// The first installation wins for the life of the process; a later call
/// with the same function succeeds, and one with another function fails and
/// changes nothing.
#[cfg(any(test, feature = "test-internals"))]
pub fn pace_tasks(pacer: Pacer) -> bool {
    std::ptr::fn_addr_eq(*PACER.get_or_init(|| pacer), pacer)
}

impl TaskRole {
    /// Run `task` as a task of this role, polling it only while the
    /// installed pacer lets this role run.
    #[cfg(any(test, feature = "test-internals"))]
    pub(crate) async fn paced<F: Future>(self, task: F) -> F::Output {
        let mut task = pin!(task);
        poll_fn(|cx| match PACER.get() {
            Some(pacer) if !pacer(self, cx.waker()) => Poll::Pending,
            _ => task.as_mut().poll(cx),
        })
        .await
    }

    /// Run `task` as a task of this role, polling it only while the
    /// installed pacer lets this role run.
    #[cfg(not(any(test, feature = "test-internals")))]
    #[inline(always)]
    pub(crate) fn paced<F: Future>(self, task: F) -> F {
        task
    }
}

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

/// Return the next item or await cancellation after its producer disappears.
pub async fn next_or_cancelled<T>(next: impl Future<Output = Option<T>>) -> T {
    match next.await {
        Some(item) => item,
        None => cancelled().await,
    }
}
