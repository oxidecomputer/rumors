//! Application factories for connection initiation and session deadlines.

use std::future::{Future, pending};
use std::sync::Arc;

use futures::future::{BoxFuture, Either, FutureExt, Shared};
use futures::stream::BoxStream;
use futures::{Stream, StreamExt};

use crate::{Changes, Error, Gossip, bookmark::Bookmark};

/// A new policy stream for one connection, given its own change subscription.
type Initiation<T> = dyn Fn(Changes<T>) -> BoxStream<'static, Gossip> + Send + Sync;

/// A new deadline future for one active session.
type Deadline = dyn Fn() -> BoxFuture<'static, ()> + Send + Sync;

/// Immutable factories shared by a replica's handles; each driver owns its
/// state.
pub(crate) struct Policy<T: Send + Sync + 'static> {
    /// None selects the unmodified change stream.
    when: Option<Arc<Initiation<T>>>,
    /// None leaves sessions untimed without allocating a deadline future.
    deadline: Option<Arc<Deadline>>,
    /// None lets the policy stream run until it ends on its own; once this
    /// resolves, it ends every driver's policy stream.
    until: Option<Shared<BoxFuture<'static, ()>>>,
}

/// Default to push-on-change with no session deadline.
impl<T: Send + Sync + 'static> Default for Policy<T> {
    /// Construct the default policy without allocating a factory.
    fn default() -> Self {
        Self {
            when: None,
            deadline: None,
            until: None,
        }
    }
}

/// Share factories without imposing Clone on the payload.
impl<T: Send + Sync + 'static> Clone for Policy<T> {
    /// Copy settings while each connection retains its own active stream.
    fn clone(&self) -> Self {
        Self {
            when: self.when.clone(),
            deadline: self.deadline.clone(),
            until: self.until.clone(),
        }
    }
}

/// Configure factories without exposing erased futures or streams to callers.
impl<T: Send + Sync + 'static> Policy<T> {
    /// Replace the initiation factory, converting its items at this boundary.
    pub(super) fn set_when<F, S>(&mut self, when: F)
    where
        F: Fn(Changes<T>) -> S + Send + Sync + 'static,
        S: Stream + Send + 'static,
        S::Item: Into<Gossip>,
    {
        self.when = Some(Arc::new(move |changes| {
            Box::pin(when(changes).map(Into::into))
        }));
    }

    /// Replace the deadline factory while keeping the clock application-owned.
    pub(super) fn set_deadline<D, F>(&mut self, deadline: D)
    where
        D: Fn() -> F + Send + Sync + 'static,
        F: Future<Output = ()> + Send + 'static,
    {
        self.deadline = Some(Arc::new(move || Box::pin(deadline())));
    }

    /// Replace the signal that ends every driver's policy stream.
    pub(super) fn set_until<F>(&mut self, until: F)
    where
        F: Future + Send + 'static,
    {
        self.until = Some(until.map(drop).boxed().shared());
    }

    /// Race a complete wire exchange against one application-owned deadline. A
    /// ready session result wins, preserving confirmed completion or a failure.
    pub(super) async fn run<R, B: Bookmark>(
        &self,
        session: impl Future<Output = Result<R, Error<B>>>,
    ) -> Result<R, Error<B>> {
        let deadline = self.deadline();
        tokio::select! {
            biased;
            result = session => result,
            () = deadline => Err(Error::DeadlineExceeded),
        }
    }

    /// Create one deadline, or an allocation-free pending future if disabled.
    fn deadline(&self) -> impl Future<Output = ()> + Send + 'static {
        match &self.deadline {
            Some(deadline) => Either::Left(deadline()),
            None => Either::Right(pending()),
        }
    }

    /// Give the factory sole ownership of this connection's subscription, and
    /// end the resulting stream once the stop signal resolves.
    pub(super) fn when(&self, changes: Changes<T>) -> BoxStream<'static, Gossip> {
        let when = match &self.when {
            Some(when) => when(changes),
            None => Box::pin(changes.map(Into::into)),
        };
        match &self.until {
            Some(until) => Box::pin(when.take_until(until.clone())),
            None => when,
        }
    }
}
