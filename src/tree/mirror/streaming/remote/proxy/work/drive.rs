//! One task set for a session's work and its pipe, and how the set's failures
//! become the session's outcome.
//!
//! The proxy's terminal operation polls the proxy's part of a session's work in
//! one [`FuturesUnordered`], whose *members* are the proxy's walk tasks, its
//! terminal future, the multiplexer, and the demultiplexer. The local
//! participant's walk is not a member: it runs beside the set ("What rest
//! sees", below). The set polls a member when it wakes, and polls a member
//! woken during one of its polls within that same poll, as far as its yield
//! policy allows. So handing a frame between an encoder and the multiplexer, or
//! between the demultiplexer and a decoder, costs one member's poll rather than
//! a poll of the whole session. Where wakes are served is all that changes:
//! every member waits on exactly what it would wait on alone, so no more units
//! are outstanding than before.
//!
//! # Failure routing
//!
//! The set completes members in the order they wake, and no rule here depends
//! on that order. A failure is classified by its value:
//!
//! - a *write failure* is a failure to write or flush the pipe
//!   ([`PipeError::Encode`]), which the multiplexer reports;
//! - a *receive failure* is the demultiplexer's, the only way it completes;
//! - every other error, from a walk task or the terminal future, is a *walk
//!   error*.
//!
//! The session's outcome follows from three rules.
//!
//! 1. **A walk error fails the session at once.** A member that fails drops its
//!    channels when it completes, so an error the closure causes in another
//!    member can only follow it: the first walk error found is the cause.
//! 2. **A pipe failure fails the session once the walk stops.** After a write
//!    or receive failure, the session keeps polling the set, across as many
//!    polls of the session as that takes, until the walk completes or comes to
//!    *rest*: a poll of the set finds no member ready and notices no wake. Rule
//!    1 still applies meanwhile. The session then fails with a receive failure
//!    that shows the peer breaking the protocol if there is one, since a peer
//!    that detected a problem may have closed the connection and so caused the
//!    write failure; otherwise with the write failure; otherwise with the
//!    receive failure, the peer's departure included. Neither pipe failure
//!    causes a walk error: once the multiplexer has gone, encoders handing it
//!    frames await cancellation, and the demultiplexer's channels to the
//!    decoders stay open, because its owner outlives the set.
//! 3. **A completed walk succeeds unless a pipe member failed.** Once every
//!    walk task, the terminal future and the multiplexer have completed, one
//!    last poll of the set collects a receive failure the demultiplexer had
//!    ready, without waiting. If the demultiplexer has reported a failure, rule
//!    2 decides; otherwise the session succeeds. A failure the demultiplexer
//!    has not met falls to the closing exchange, which reads next and reports
//!    it in its own phase.
//!
//! So a failure the pipe reports during the walk fails the session, whatever
//! the walk could still have completed from work already within the session.
//!
//! # What rest sees
//!
//! Rest is inferred from the set alone, and the set does not hold all of the
//! session's work. The local participant's walk runs beside it: the session's
//! task polls the local walk and this set one after the other, never one inside
//! the other, and the local walk feeds some members, the encoders, through
//! channels. A member waiting on the local walk can be woken only once the
//! local walk runs, so a poll of the set that ends with that member waiting
//! notices no wake, even when the local walk holds the work that would wake it.
//! Rest therefore means that no member can make progress until something
//! outside the set does, and the local walk may still hold work.
//!
//! The rules consult rest only to decide when a session that has already failed
//! stops. A rest that comes early therefore never changes whether a session
//! fails, nor whether it publishes: rule 2 fails it either way, and rule 3 lets
//! only a walk with no pipe failure succeed. It can change which failure the
//! session reports: a walk error the local walk would still have raised, or a
//! violation a decoder would have found in a frame it could not yet hand on,
//! gives way to the pipe failure.

use std::{
    future::poll_fn,
    pin::Pin,
    sync::{Arc, Mutex, MutexGuard, PoisonError},
    task::{Context, Poll, Wake, Waker},
};

use futures::{StreamExt, future::BoxFuture, stream::FuturesUnordered};

use crate::tree::mirror::streaming::remote::{pipe::PipeError, proxy::Error};

/// Drive a session's members to its outcome, by the rules in the module docs.
///
/// `send` is the multiplexer, its failures already wrapped as [`Error::Pipe`];
/// `receive` is the demultiplexer, which completes only with a failure.
pub(super) async fn drive<O, E>(
    tasks: Vec<BoxFuture<'static, Result<(), Error<E>>>>,
    finish: BoxFuture<'_, Result<O, Error<E>>>,
    send: BoxFuture<'_, Result<(), Error<E>>>,
    receive: BoxFuture<'_, PipeError>,
) -> Result<O, Error<E>> {
    let mut routing = Routing {
        walking: tasks.len() + 2,
        output: None,
        written: None,
        received: None,
    };
    let mut set: FuturesUnordered<Member<'_, O, E>> = tasks.into_iter().map(Member::Task).collect();
    set.push(Member::Finish(finish));
    set.push(Member::Send(send));
    set.push(Member::Receive(receive));
    poll_fn(|cx| routing.poll(&mut set, cx)).await
}

/// One member of a session's task set.
///
/// Every variant is boxed, so the member is [`Unpin`] and the set holds the
/// walk's tasks in the boxes they arrive in.
enum Member<'a, O, E> {
    /// A walk task.
    Task(BoxFuture<'static, Result<(), Error<E>>>),
    /// The terminal future, whose output is the session's.
    Finish(BoxFuture<'a, Result<O, Error<E>>>),
    /// The multiplexer.
    Send(BoxFuture<'a, Result<(), Error<E>>>),
    /// The demultiplexer.
    Receive(BoxFuture<'a, PipeError>),
}

/// What a member completed with.
enum Done<O, E> {
    /// A walk task's result.
    Task(Result<(), Error<E>>),
    /// The terminal future's result.
    Finish(Result<O, Error<E>>),
    /// The multiplexer's result.
    Sent(Result<(), Error<E>>),
    /// The demultiplexer's failure.
    Received(PipeError),
}

/// Poll the boxed future inside, tagging its output with the member's kind.
impl<O, E> Future for Member<'_, O, E> {
    type Output = Done<O, E>;

    /// Polls the member's future once.
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.get_mut() {
            Member::Task(task) => task.as_mut().poll(cx).map(Done::Task),
            Member::Finish(finish) => finish.as_mut().poll(cx).map(Done::Finish),
            Member::Send(send) => send.as_mut().poll(cx).map(Done::Sent),
            Member::Receive(receive) => receive.as_mut().poll(cx).map(Done::Received),
        }
    }
}

/// What the set has completed so far, from which the session's outcome follows.
struct Routing<O, E> {
    /// The walk's tasks, the terminal future and the multiplexer that have not
    /// yet completed successfully.
    walking: usize,
    /// The terminal future's output, once it has completed.
    output: Option<O>,
    /// The first write failure found.
    written: Option<Error<E>>,
    /// The demultiplexer's failure, once it has reported one.
    received: Option<PipeError>,
}

impl<O, E> Routing<O, E> {
    /// Poll the set toward the session's outcome.
    ///
    /// Until a pipe failure, the set is polled with the session's own waker,
    /// and `Pending` passes straight out, as the set's yield policy requires.
    /// After one, it is polled with a waker that notices wakes, to tell a set
    /// at rest from one that yielded or woke a member; `Pending` still passes
    /// out unless the set is at rest, so a member that yields to its executor
    /// runs again only after the executor polls the session again.
    fn poll(
        &mut self,
        set: &mut FuturesUnordered<Member<'_, O, E>>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<O, Error<E>>> {
        let mut notice = self.failed().then(|| Notice::new(cx.waker()));
        loop {
            let polled = match &notice {
                None => set.poll_next_unpin(cx),
                Some((notice, waker)) => {
                    notice.clear();
                    set.poll_next_unpin(&mut Context::from_waker(waker))
                }
            };
            match polled {
                Poll::Ready(Some(done)) => {
                    if let Err(error) = self.record(done) {
                        return Poll::Ready(Err(error));
                    }
                    if self.walking == 0 {
                        return Poll::Ready(self.complete(set, cx));
                    }
                    if notice.is_none() && self.failed() {
                        notice = Some(Notice::new(cx.waker()));
                    }
                }
                // The demultiplexer completes only by failing, so the set
                // empties only after a pipe failure, with nothing left to
                // poll.
                Poll::Ready(None) => return Poll::Ready(Err(self.ranked())),
                Poll::Pending => {
                    return match &notice {
                        Some((notice, _)) if !notice.woken() => Poll::Ready(Err(self.ranked())),
                        _ => Poll::Pending,
                    };
                }
            }
        }
    }

    /// Record one member's completion, returning a walk error, which fails the
    /// session at once.
    fn record(&mut self, done: Done<O, E>) -> Result<(), Error<E>> {
        let result = match done {
            Done::Task(result) | Done::Sent(result) => result,
            Done::Finish(result) => result.map(|output| self.output = Some(output)),
            Done::Received(failure) => {
                self.received = Some(failure);
                return Ok(());
            }
        };
        match result {
            Ok(()) => {
                self.walking -= 1;
                Ok(())
            }
            Err(failure @ Error::Pipe(PipeError::Encode(_))) => {
                self.written.get_or_insert(failure);
                Ok(())
            }
            Err(error) => Err(error),
        }
    }

    /// Whether a pipe member has failed.
    fn failed(&self) -> bool {
        self.written.is_some() || self.received.is_some()
    }

    /// The session's outcome once the walk has completed (rule 3).
    fn complete(
        &mut self,
        set: &mut FuturesUnordered<Member<'_, O, E>>,
        cx: &mut Context<'_>,
    ) -> Result<O, Error<E>> {
        if self.received.is_none() {
            // Only the demultiplexer is left, so this polls it if it has
            // woken since its last poll, and never waits.
            match set.poll_next_unpin(cx) {
                Poll::Ready(Some(Done::Received(failure))) => self.received = Some(failure),
                Poll::Ready(Some(_)) => {
                    unreachable!("only the demultiplexer is left once the walk has completed")
                }
                Poll::Ready(None) | Poll::Pending => {}
            }
        }
        if self.failed() {
            return Err(self.ranked());
        }
        Ok(self
            .output
            .take()
            .expect("the walk completes only once the terminal future has"))
    }

    /// The failure of a session whose walk stopped after a pipe failure (rule
    /// 2).
    fn ranked(&mut self) -> Error<E> {
        match (self.received.take(), self.written.take()) {
            (Some(received), _) if received.is_violation() => Error::Pipe(received),
            (_, Some(written)) => written,
            (Some(received), None) => Error::Pipe(received),
            (None, None) => {
                unreachable!("a session fails at rest or completion only after a pipe failure")
            }
        }
    }
}

/// A waker that notes that it was woken, then wakes the session.
///
/// The flag sits under a lock, so every wake is ordered before or after each
/// read of the flag, whichever thread it comes from.
struct Notice {
    /// Whether a wake arrived since the flag was last cleared.
    woken: Mutex<bool>,
    /// The session's waker.
    session: Waker,
}

impl Notice {
    /// A notice passing wakes on to `session`, and the waker that notes them.
    fn new(session: &Waker) -> (Arc<Self>, Waker) {
        let notice = Arc::new(Self {
            woken: Mutex::new(false),
            session: session.clone(),
        });
        let waker = Waker::from(Arc::clone(&notice));
        (notice, waker)
    }

    /// Forget the wakes noted so far.
    fn clear(&self) {
        *self.flag() = false;
    }

    /// Whether a wake arrived since the flag was last cleared.
    fn woken(&self) -> bool {
        *self.flag()
    }

    /// Lock the flag. A poisoned lock still holds a whole flag: nothing can
    /// panic while holding it.
    fn flag(&self) -> MutexGuard<'_, bool> {
        self.woken.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Note the wake, then pass it on to the session.
impl Wake for Notice {
    /// Wakes as [`wake_by_ref`](Wake::wake_by_ref) does.
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    /// Notes the wake before waking the session; a wake from another thread
    /// that is ordered after the set's poll reads the flag is an event outside
    /// the session, which wakes the session as usual.
    fn wake_by_ref(self: &Arc<Self>) {
        *self.flag() = true;
        self.session.wake_by_ref();
    }
}

#[cfg(test)]
mod tests;
