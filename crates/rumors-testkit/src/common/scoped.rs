//! An observer a peer selects on its builder before it exists, inert until a
//! test polls one of the peer's sessions with an observer in scope.
//!
//! A peer's observers are fixed when its builder seeds or joins, but a test
//! that captures or traces a session usually wants to observe only that
//! session, not the setup sessions before it. Such a peer selects
//! [`scoped_observer`] on its builder. A session the peer starts while the
//! test polls it through [`in_scope`] goes to the observer in scope; a session
//! started outside any scope, such as a setup session, is not observed at all.
//!
//! The routing rests on one fact about the crate: a session asks its peer's
//! observers for a session handler synchronously, while the future that runs
//! the session is being polled, and the session runs inside that future
//! without spawning onto another task. The handler then keeps the observer it
//! was routed to for the rest of the session.

use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use rumors::observe::{Observer, SessionInfo, SessionObserver};

thread_local! {
    /// The observer in scope on this thread, while [`InScope`] polls a future.
    static IN_SCOPE: RefCell<Option<Arc<dyn Observer>>> = const { RefCell::new(None) };
}

/// The observer a capturable peer selects on its builder: it forwards each
/// session to the observer in scope when that session starts, and observes
/// nothing outside a scope.
#[derive(Debug, Default)]
pub struct ScopedObserver;

/// Forward a session to the observer in scope, if any.
impl Observer for ScopedObserver {
    /// Hand the session to the observer in scope, or skip it outside a scope.
    fn session(&self, session: &SessionInfo) -> Option<Box<dyn SessionObserver>> {
        let in_scope = IN_SCOPE.with(|scope| scope.borrow().clone());
        in_scope.and_then(|observer| observer.session(session))
    }
}

/// The observer for a capturable peer's builder; see [`ScopedObserver`].
pub fn scoped_observer() -> Arc<ScopedObserver> {
    Arc::new(ScopedObserver)
}

/// A future polled with an observer in scope.
pub struct InScope<F> {
    /// The observer in scope while the future is polled.
    observer: Arc<dyn Observer>,
    /// The future, which runs the sessions to observe.
    future: Pin<Box<F>>,
}

/// Poll `future` with `observer` in scope, so that every session a
/// [`scoped_observer`] peer starts within it goes to `observer`.
pub fn in_scope<F: Future>(observer: Arc<dyn Observer>, future: F) -> InScope<F> {
    InScope {
        observer,
        future: Box::pin(future),
    }
}

/// Poll the inner future with the observer in scope, restoring the scope that
/// was in place before.
impl<F: Future> Future for InScope<F> {
    type Output = F::Output;

    /// Put the observer in scope for exactly this poll of the inner future.
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<F::Output> {
        let observer = Arc::clone(&self.observer);
        let outer = IN_SCOPE.with(|scope| scope.replace(Some(observer)));
        let polled = self.future.as_mut().poll(cx);
        IN_SCOPE.with(|scope| scope.replace(outer));
        polled
    }
}
