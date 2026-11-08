//! Drive two connected protocol implementations through reconciliation.
//!
//! Each phase consumes a height-typed state and returns the next state at a
//! different height. An ordinary loop cannot hold that changing type, so
//! `mirror!` expands the fixed schedule into straight-line calls. The [`Peer`]
//! bounds make a wrong schedule fail to compile before it reaches a terminal
//! phase.
//!
//! Replies cross between participants as streams. A failed producer cannot end
//! its stream normally: its consumer would interpret that EOF as a clean phase
//! boundary, advance, and later wait forever for the failed participant.
//! [`divert`] records the source error and parks instead. [`race_session`] then
//! returns that error and cancels the rest of the schedule.
//!
//! # Holding each value once
//!
//! Every session boxes the state machine this driver builds, and boxing copies
//! all of its bytes, so the code here avoids two ways the compiler would store
//! a value twice. An `async fn` keeps each argument in its state for its whole
//! run, so awaiting a future argument holds that future twice: as the argument,
//! and as the value awaited. A local moved out field by field, rather than
//! whole, still counts as holding its value until its scope ends, so it is
//! saved across every await in that scope. The helpers here therefore return
//! combinators instead of awaiting future arguments, [`race_session`] borrows
//! the session pinned where it is built, and `mirror!` moves each phase's state
//! whole, inside a block that ends before the session's await.

use std::{
    future::Future,
    pin::{Pin, pin},
    sync::{Arc, Mutex},
};

use async_stream::stream;
use futures::{StreamExt, TryFutureExt, channel::oneshot, future};
use seq_macro::seq;

use crate::tree::mirror::streaming::protocol::*;
use crate::tree::{
    mirror::{
        Error,
        streaming::{Backend, Leaf, tasks::cancelled},
    },
    typed::height::{Height, Z},
};

/// One participant's route into the session's shared error type.
///
/// Every route of a session shares one take-once slot holding the sending half
/// of a oneshot channel. The first report takes the sender and delivers its
/// error, so the first error wins, and every later report finds the slot empty
/// and drops its error.
pub(super) struct ErrorRoute<E, S> {
    /// The session's take-once slot for the first-error sender.
    sender: Arc<Mutex<Option<oneshot::Sender<S>>>>,
    /// Converts this participant's error into the session error.
    wrap: fn(E) -> S,
}

/// Clone an error route without imposing clone bounds on either error type.
impl<E, S> Clone for ErrorRoute<E, S> {
    /// Share the same first-error slot and conversion function.
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            wrap: self.wrap,
        }
    }
}

/// Report stream errors and map terminal errors through one participant's
/// route.
impl<E, S> ErrorRoute<E, S> {
    /// Report the first response-stream error without blocking its producer.
    pub(super) fn report(&self, error: E) {
        let sender = self
            .sender
            .lock()
            .expect("taking the sender cannot panic while holding this lock")
            .take();
        // An empty slot means an earlier report already sent the first error.
        // A failed send means the session race has ended. Neither case needs
        // another report.
        if let Some(sender) = sender {
            let _ = sender.send((self.wrap)(error));
        }
    }

    /// Map one terminal future into the session's common error type.
    ///
    /// A combinator, not an `async fn`, so that the result holds `future` once
    /// (see the module docs).
    pub(super) fn resolve<O>(
        self,
        future: impl Future<Output = Result<O, E>>,
    ) -> impl Future<Output = Result<O, S>> {
        future.map_err(self.wrap)
    }
}

/// Allocate both endpoint routes and their shared one-error receiver.
fn error_routes<L, R, E>(
    wrap_left: fn(L) -> E,
    wrap_right: fn(R) -> E,
) -> (ErrorRoute<L, E>, ErrorRoute<R, E>, FirstError<E>) {
    let (sender, receiver) = oneshot::channel();
    let sender = Arc::new(Mutex::new(Some(sender)));
    (
        ErrorRoute {
            sender: sender.clone(),
            wrap: wrap_left,
        },
        ErrorRoute {
            sender,
            wrap: wrap_right,
        },
        FirstError { receiver },
    )
}

/// The receiving side of a session's first-error route.
struct FirstError<E> {
    /// Receives at most the first response-stream error.
    receiver: oneshot::Receiver<E>,
}

/// Race a session against response errors, preserving their causal priority.
///
/// The session is borrowed pinned where its caller built it, so that the race
/// does not hold a second copy of it (see the module docs).
async fn race_session<O, E>(
    session: Pin<&mut impl Future<Output = Result<O, E>>>,
    mut first_error: FirstError<E>,
) -> Result<O, E> {
    let result = tokio::select! {
        biased;
        // A cancelled receiver means every route is gone unreported; the
        // branch is then disabled and the session's own result decides.
        Ok(error) = &mut first_error.receiver => return Err(error),
        result = session => result,
    };
    match result {
        // `divert` parks after reporting, so a routed error makes successful
        // completion unreachable rather than racing with this result.
        Ok(output) => Ok(output),
        // Polling the session may report a source error and then surface a
        // secondary cancellation error in the same poll. Recover the source.
        Err(secondary) => Err(first_error
            .receiver
            .try_recv()
            .ok()
            .flatten()
            .unwrap_or(secondary)),
    }
}

/// Fail-fast join two branches after mapping their distinct error types.
///
/// A combinator, not an `async fn`, so that the join holds each branch once
/// (see the module docs).
pub(super) fn try_join_mapped<L, R, LO, RO, LE, RE, E, LW, RW>(
    left: L,
    wrap_left: LW,
    right: R,
    wrap_right: RW,
) -> impl Future<Output = Result<(LO, RO), E>>
where
    L: Future<Output = Result<LO, LE>>,
    R: Future<Output = Result<RO, RE>>,
    LW: FnOnce(LE) -> E,
    RW: FnOnce(RE) -> E,
{
    future::try_join(left.map_err(wrap_left), right.map_err(wrap_right))
}

/// Expand the type-level phase schedule into the connected driver's body.
///
/// Each step diverts one response stream, advances its counterparty, and
/// retains the producer's next state. The terminal joins both sides after
/// mapping their distinct errors into the session error type.
///
/// Each step moves both participants' `(state, route)` pairs whole (`{ $x }`)
/// and binds their parts only inside a block, so that no moved-out local is
/// still in scope at the session's one await (see the module docs).
macro_rules! mirror {
    (@one $producer:ident >> $consumer:ident.$method:ident) => {
        let ($producer, $consumer) = {
            let ((responses, next), route) = { $producer };
            let (state, consumer_route) = { $consumer };
            let requests = divert(responses, route.clone());
            ((next, route), (state.$method(requests), consumer_route))
        };
    };
    (@pending($producer:ident) parties($first:ident, $second:ident) $consumer:ident.$method:ident;) => {{
        let outputs = {
            let ((responses, completion), route) = { $producer };
            let (state, consumer_route) = { $consumer };
            let requests = divert(responses, route.clone());
            future::try_join(
                consumer_route.resolve(state.$method(requests)),
                route.resolve(completion),
            )
        }
        .await;
        // Bind each output under its participant's identifier, then restore
        // the order in which the participants entered the driver.
        outputs.map(|($consumer, $producer)| ($first, $second))
    }};
    (@pending($producer:ident) parties($first:ident, $second:ident) for _ in $lo:tt..$hi:tt { $($body:tt)* } $($rest:tt)*) => {{
        seq!(_ in $lo..$hi {
            mirror!(@step($producer) $($body)*);
        });
        mirror!(@pending($producer) parties($first, $second) $($rest)*)
    }};
    (@pending($producer:ident) parties($first:ident, $second:ident) $consumer:ident.$method:ident; $($rest:tt)*) => {{
        mirror!(@one $producer >> $consumer.$method);
        mirror!(@pending($consumer) parties($first, $second) $($rest)*)
    }};
    (@step($producer:ident) $consumer:ident.$method:ident; $($rest:tt)*) => {
        mirror!(@one $producer >> $consumer.$method);
        mirror!(@step($consumer) $($rest)*);
    };
    (@step($producer:ident)) => {};
    (@run parties($first:ident, $second:ident) $producer:ident.$method:ident; $($rest:tt)*) => {{
        let $producer = {
            let (state, route) = { $producer };
            (state.$method(), route)
        };
        mirror!(@pending($producer) parties($first, $second) $($rest)*)
    }};
    ($first:ident.$first_method:ident; $second:ident.$second_method:ident; $($rest:tt)*) => {{
        let (first_route, second_route, first_error) =
            error_routes(Error::Client, Error::Server);
        let $first = ($first, first_route);
        let $second = ($second, second_route);
        let session = pin!(async {
            mirror!(@run parties($first, $second)
                $first.$first_method;
                $second.$second_method;
                $($rest)*
            )
        });
        race_session(session, first_error).await
    }};
}

/// Drive the complete reconciliation schedule between two connected peers.
pub(super) async fn mirror_connected<B, I, R>(
    initiator: I,
    responder: R,
) -> Result<(I::Output, R::Output), Error<I::Error, R::Error>>
where
    B: Backend<Node<Z>: Leaf>,
    I: Peer<B>,
    R: Peer<B>,
{
    mirror! {
        initiator.initiator;
        responder.responder;
        for _ in 0..15 {
            initiator.reply;
            responder.reply;
        }
        initiator.reply;
        responder.complete_responder;
        initiator.complete_initiator;
    }
}

/// Divert one producer's errors while forwarding its responses.
///
/// On error the stream parks rather than ending, because EOF means successful
/// phase completion to its consumer. [`race_session`] observes the routed error
/// and cancels the parked schedule.
fn divert<B, H, E, D>(
    responses: impl Responses<B, H, E>,
    route: ErrorRoute<E, D>,
) -> impl Requests<B, H>
where
    B: Backend<Node<Z>: Leaf>,
    H: Height,
    E: Send + 'static,
    D: Send + 'static,
{
    stream! {
        let mut responses = pin!(responses);
        while let Some(item) = responses.next().await {
            match item {
                Ok(message) => yield message,
                Err(error) => {
                    route.report(error);
                    cancelled().await;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
