//! Phase scheduling and causal error routing for connected peers.

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

/// One endpoint's typed route into the shared session error.
pub struct ErrorRoute<E, S> {
    send: mpsc::Sender<S>,
    wrap: fn(E) -> S,
}

impl<E, S> Clone for ErrorRoute<E, S> {
    fn clone(&self) -> Self {
        Self {
            send: self.send.clone(),
            wrap: self.wrap,
        }
    }
}

impl<E, S> ErrorRoute<E, S> {
    /// Report the first response-stream error without blocking its producer.
    pub fn report(&self, error: E) {
        let _ = self.send.try_send((self.wrap)(error));
    }

    /// Map one terminal future into the session's common error type.
    pub async fn resolve<O>(self, future: impl Future<Output = Result<O, E>>) -> Result<O, S> {
        future.await.map_err(self.wrap)
    }
}

/// Allocate both endpoint routes and their shared one-error receiver.
fn error_routes<L, R, E>(
    wrap_left: fn(L) -> E,
    wrap_right: fn(R) -> E,
) -> (ErrorRoute<L, E>, ErrorRoute<R, E>, FirstError<E>) {
    let (send, receive) = mpsc::channel(1);
    (
        ErrorRoute {
            send: send.clone(),
            wrap: wrap_left,
        },
        ErrorRoute {
            send,
            wrap: wrap_right,
        },
        FirstError(receive),
    )
}

/// The receiving side of a session's first-error route.
pub struct FirstError<E>(mpsc::Receiver<E>);

/// Race a session against response errors, preserving their causal priority.
pub async fn race_session<O, E>(
    session: impl Future<Output = Result<O, E>>,
    mut first_error: FirstError<E>,
) -> Result<O, E> {
    let results = tokio::select! {
        biased;
        Some(error) = first_error.0.recv() => return Err(error),
        results = session => results,
    };
    match results {
        Ok(output) => Ok(output),
        Err(secondary) => Err(first_error.0.try_recv().unwrap_or(secondary)),
    }
}

/// Fail-fast join two branches after mapping their distinct error types.
pub async fn try_join_mapped<L, R, LO, RO, LE, RE, E, LW, RW>(
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
    futures::future::try_join(async move { left.await.map_err(wrap_left) }, async move {
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
    (@one $a:ident >> $b:ident.$m:ident) => {
        let ((msgs, state), route) = $a;
        let msgs = divert(msgs, route.clone());
        let $a = (state, route);
        let $b = ($b.0.$m(msgs), $b.1);
    };
    (@pending($a:ident) parties($p:ident, $q:ident) $b:ident.$m:ident;) => {{
        let ((msgs, state), route) = $a;
        let msgs = divert(msgs, route.clone());
        let ($b, route_b) = $b;
        future::try_join(route_b.resolve($b.$m(msgs)), route.resolve(state))
            .await
            .map(|($b, $a)| ($p, $q))
    }};
    (@pending($a:ident) parties($p:ident, $q:ident) for _ in $lo:tt..$hi:tt { $($body:tt)* } $($rest:tt)*) => {{
        seq!(_ in $lo..$hi {
            mirror!(@step($a) $($body)*);
        });
        mirror!(@pending($a) parties($p, $q) $($rest)*)
    }};
    (@pending($a:ident) parties($p:ident, $q:ident) $b:ident.$m:ident; $($rest:tt)*) => {{
        mirror!(@one $a >> $b.$m);
        mirror!(@pending($b) parties($p, $q) $($rest)*)
    }};
    (@step($a:ident) $b:ident.$m:ident; $($rest:tt)*) => {
        mirror!(@one $a >> $b.$m);
        mirror!(@step($b) $($rest)*);
    };
    (@step($a:ident)) => {};
    (@run parties($p:ident, $q:ident) $a:ident.$m:ident; $($rest:tt)*) => {{
        let $a = ($a.0.$m(), $a.1);
        mirror!(@pending($a) parties($p, $q) $($rest)*)
    }};
    ($a:ident.$m:ident; $b:ident.$n:ident; $($rest:tt)*) => {{
        let (route_a, route_b, first_error) = error_routes(Error::Client, Error::Server);
        let $a = ($a, route_a);
        let $b = ($b, route_b);
        let session = async { mirror!(@run parties($a, $b) $a.$m; $b.$n; $($rest)*) };
        race_session(session, first_error).await
    }};
}

/// Drive the complete reconciliation schedule between two connected peers.
pub(super) async fn mirror_connected<B, I, R, T>(
    i: I,
    r: R,
) -> Result<(I::Output, R::Output), Error<I::Error, R::Error>>
where
    T: Send + Sync + 'static,
    B: Backend<T, Node<Z>: Leaf<T>>,
    I: Peer<B, T>,
    R: Peer<B, T>,
{
    mirror! {
        i.initiator;
        r.responder;
        for _ in 0..15 {
            i.reply;
            r.reply;
        }
        i.reply;
        r.complete_responder;
        i.complete_initiator;
    }
}

/// Divert one producer's typed errors while forwarding its responses.
///
/// On error the stream parks rather than ending, because EOF means successful
/// phase completion to its consumer. [`race_session`] observes the routed
/// error and cancels the parked schedule.
fn divert<B, T, H, E, D>(
    messages: impl Responses<B, T, H, E>,
    route: ErrorRoute<E, D>,
) -> impl Requests<B, T, H>
where
    T: Send + Sync + 'static,
    B: Backend<T, Node<Z>: Leaf<T>>,
    H: Height,
    E: Send + 'static,
    D: Send + 'static,
{
    stream! {
        let mut messages = pin!(messages);
        while let Some(item) = messages.next().await {
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
