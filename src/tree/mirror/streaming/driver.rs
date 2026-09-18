//! Drive two connected protocol implementations through reconciliation.
//!
//! Each phase consumes a height-typed state and returns the next state at a
//! different height. An ordinary loop cannot hold that changing type, so
//! `mirror!` expands the fixed schedule into straight-line calls. The
//! [`Peer`] bounds make a wrong schedule fail to compile before it reaches a
//! terminal phase.
//!
//! Replies cross between participants as streams. A failed producer cannot
//! end its stream normally: its consumer would interpret that EOF as a clean
//! phase boundary, advance, and later wait forever for the failed participant.
//! [`divert`] records the source error and parks instead. [`race_session`] then
//! returns that error and cancels the rest of the schedule.

use std::{future::Future, pin::pin};

use async_stream::stream;
use futures::{StreamExt, future};
use seq_macro::seq;
use tokio::sync::mpsc;

use crate::tree::mirror::streaming::protocol::*;
use crate::tree::{
    mirror::{
        Error,
        streaming::{Backend, Leaf, tasks::cancelled},
    },
    typed::height::{Height, Z},
};

/// One participant's route into the session's shared error type.
pub(super) struct ErrorRoute<E, S> {
    /// The one-slot channel that retains the first response-stream error.
    sender: mpsc::Sender<S>,
    /// Converts this participant's error into the session error.
    wrap: fn(E) -> S,
}

/// Clone an error route without imposing clone bounds on either error type.
impl<E, S> Clone for ErrorRoute<E, S> {
    /// Share the same first-error channel and conversion function.
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            wrap: self.wrap,
        }
    }
}

/// Report stream errors and map terminal errors through one participant's route.
impl<E, S> ErrorRoute<E, S> {
    /// Report the first response-stream error without blocking its producer.
    pub(super) fn report(&self, error: E) {
        // A full channel already holds the first error. A closed channel means
        // the session race has ended. Neither case needs another report.
        let _ = self.sender.try_send((self.wrap)(error));
    }

    /// Map one terminal future into the session's common error type.
    pub(super) async fn resolve<O>(
        self,
        future: impl Future<Output = Result<O, E>>,
    ) -> Result<O, S> {
        future.await.map_err(self.wrap)
    }
}

/// Allocate both endpoint routes and their shared one-error receiver.
fn error_routes<L, R, E>(
    wrap_left: fn(L) -> E,
    wrap_right: fn(R) -> E,
) -> (ErrorRoute<L, E>, ErrorRoute<R, E>, FirstError<E>) {
    let (sender, receiver) = mpsc::channel(1);
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
    receiver: mpsc::Receiver<E>,
}

/// Race a session against response errors, preserving their causal priority.
async fn race_session<O, E>(
    session: impl Future<Output = Result<O, E>>,
    mut first_error: FirstError<E>,
) -> Result<O, E> {
    let result = tokio::select! {
        biased;
        Some(error) = first_error.receiver.recv() => return Err(error),
        result = session => result,
    };
    match result {
        // `divert` parks after reporting, so a routed error makes successful
        // completion unreachable rather than racing with this result.
        Ok(output) => Ok(output),
        // Polling the session may report a source error and then surface a
        // secondary cancellation error in the same poll. Recover the source.
        Err(secondary) => Err(first_error.receiver.try_recv().unwrap_or(secondary)),
    }
}

/// Fail-fast join two branches after mapping their distinct error types.
pub(super) async fn try_join_mapped<L, R, LO, RO, LE, RE, E, LW, RW>(
    left: L,
    wrap_left: LW,
    right: R,
    wrap_right: RW,
) -> Result<(LO, RO), E>
where
    L: Future<Output = Result<LO, LE>>,
    R: Future<Output = Result<RO, RE>>,
    LW: FnOnce(LE) -> E,
    RW: FnOnce(RE) -> E,
{
    future::try_join(async move { left.await.map_err(wrap_left) }, async move {
        right.await.map_err(wrap_right)
    })
    .await
}

/// Expand the type-level phase schedule into the connected driver's body.
///
/// Each step diverts one response stream, advances its counterparty, and
/// retains the producer's next state. The terminal joins both sides after
/// mapping their distinct errors into the session error type.
macro_rules! mirror {
    (@one $producer:ident >> $consumer:ident.$method:ident) => {
        let ((responses, next), route) = $producer;
        let requests = divert(responses, route.clone());
        let $producer = (next, route);
        let $consumer = ($consumer.0.$method(requests), $consumer.1);
    };
    (@pending($producer:ident) parties($first:ident, $second:ident) $consumer:ident.$method:ident;) => {{
        let ((responses, completion), route) = $producer;
        let requests = divert(responses, route.clone());
        let ($consumer, consumer_route) = $consumer;
        let outputs = future::try_join(
            consumer_route.resolve($consumer.$method(requests)),
            route.resolve(completion),
        )
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
        let $producer = ($producer.0.$method(), $producer.1);
        mirror!(@pending($producer) parties($first, $second) $($rest)*)
    }};
    ($first:ident.$first_method:ident; $second:ident.$second_method:ident; $($rest:tt)*) => {{
        let (first_route, second_route, first_error) =
            error_routes(Error::Client, Error::Server);
        let $first = ($first, first_route);
        let $second = ($second, second_route);
        let session = async {
            mirror!(@run parties($first, $second)
                $first.$first_method;
                $second.$second_method;
                $($rest)*
            )
        };
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
/// phase completion to its consumer. [`race_session`] observes the routed
/// error and cancels the parked schedule.
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
mod tests {
    use std::convert::identity;
    use std::task::Poll;

    use futures::future;

    use super::{error_routes, race_session};

    /// A response error published while polling wins over its terminal symptom.
    #[test]
    fn routed_error_precedes_same_poll_session_error() {
        let (route, _other, first_error) = error_routes(identity::<&str>, identity::<&str>);
        let session = future::poll_fn(move |_| {
            route.report("primary");
            Poll::Ready(Err::<(), _>("secondary"))
        });

        assert_eq!(
            pollster::block_on(race_session(session, first_error)),
            Err("primary")
        );
    }

    /// A terminal error remains authoritative when no response error preceded it.
    #[test]
    fn standalone_session_error_is_preserved() {
        let (_left, _right, first_error) = error_routes(identity::<&str>, identity::<&str>);
        let session = future::ready(Err::<(), _>("terminal"));

        assert_eq!(
            pollster::block_on(race_session(session, first_error)),
            Err("terminal")
        );
    }
}
