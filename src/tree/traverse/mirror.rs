//! Bidirectional alternating mirror-sync between two replicas of the typed tree.

use std::cmp::Ordering;

use seq_macro::seq;

pub mod local;
pub mod protocol;
pub mod remote;

mod message;

#[cfg(test)]
mod message_test;

#[cfg(test)]
mod test;

#[cfg(test)]
mod wire_snapshot;

use protocol::*;

// This macro allows defining one communication step of the inner protocol
// between initiator <==> responder (once the client and server have determined
// who plays which role).
macro_rules! x {
    // Any unconditional initiator step that must continue looks like this:
    //
    // ```
    // x! { let message = initiator.method(...) }
    // ```
    //
    // This elides the error handling and irrefutable pattern match.
    (let $msg:pat = $initiator:ident . $initiator_method:ident ( $($arg:expr)* ) ) => {
        let Step::Continue {
            msg: $msg,
            next: $initiator,
        } = $initiator.$initiator_method($($arg)*).await.map_err(Error::Client)?;
    };
    // An initiator step in the protocol:
    //
    // ```
    // x! { initiator.method ==message==> responder.method }
    // ```
    //
    // This feeds the existing binding of `message` into the initiator method,
    // and rebinds `message` to the output. The expected next responder method
    // is specified so that if the initiator signals it is done, the responder
    // can be immediately be given the final message and closed out.
    ($initiator:ident . $initiator_method:ident == $msg:ident => $responder:ident . $responder_method:ident) => {
        #[allow(unused)]
        let ($msg, $responder, $initiator) =
            match $initiator.$initiator_method($msg).await.map_err(Error::Client)? {
                Step::Continue { msg, next } => (msg, $responder, next),
                Step::Done {
                    msg,
                    output: initiator_output,
                } => {
                    #[allow(irrefutable_let_patterns)]
                    let Step::Done {
                        output: responder_output,
                        ..
                    } = $responder.$responder_method(msg).await.map_err(Error::Server)?
                    else {
                        // The protocol is designed so that the two sides will
                        // *always* agree on when the protocol is complete.
                        unreachable!("responder did not finish after initiator was finished")
                    };
                    return Ok((initiator_output, responder_output));
                }
            };
    };
    // An responder step in the protocol:
    //
    // ```
    // x! { initiator.method <=message== responder.method }
    // ```
    //
    // This feeds the existing binding of `message` into the responder method
    // (on the *RIGHT HAND SIDE*), and rebinds `message` to the output. The
    // expected next initiator method is specified so that if the responder
    // signals it is done, the initiator can be immediately be given the final
    // message and closed out.
    ($initiator:ident . $initiator_method:ident <= $msg:ident == $responder:ident . $responder_method:ident) => {
        #[allow(unused)]
        let ($msg, $initiator, $responder) =
            match $responder.$responder_method($msg).await.map_err(Error::Server)? {
                Step::Continue { msg, next } => (msg, $initiator, next),
                Step::Done {
                    msg,
                    output: responder_output,
                } => {
                    #[allow(irrefutable_let_patterns)]
                    let Step::Done {
                        output: initiator_output,
                        ..
                    } = $initiator.$initiator_method(msg).await.map_err(Error::Client)?
                    else {
                        // The protocol is designed so that the two sides will
                        // *always* agree on when the protocol is complete.
                        unreachable!("initiator did not finish after responder was finished");
                    };
                    return Ok((initiator_output, responder_output));
                }
            };
    };
}

// The inner mirror protocol, between an initiator and a responder (who may or
// may not correspond with the original client/server distinction).
async fn mirror_connected<I, R, T>(
    i: I,
    r: R,
) -> Result<(I::Output, R::Output), Error<I::Error, R::Error>>
where
    T: Send + Sync,
    I: Peer<T>,
    R: Peer<T>,
{
    x! { let x = i.initiator() }
    x! { i.open_initiator <=x== r.responder }
    x! { i.open_initiator ==x=> r.exchange  }
    seq!(_ in 0..14 {
        x! { i.exchange <=x== r.exchange }
        x! { i.exchange ==x=> r.exchange }
    });
    x! { i.close_initiator    <=x== r.exchange           }
    x! { i.close_initiator    ==x=> r.complete_responder }
    x! { i.complete_initiator <=x== r.complete_responder }

    match r {}
}

/// An error which can occur during mirroring: either a client error or a server one.
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error<C, S> {
    Client(C),
    Server(S),
}

/// Drive a mirror protocol client against a server to synchronize both of them.
pub async fn mirror<'a, C, S, T>(
    c: C,
    s: S,
) -> Result<(C::Output, S::Output), Error<C::Error, S::Error>>
where
    T: Send + Sync + 'a,
    C: Client<T> + 'a,
    S: Server<T> + 'a,
{
    // Box the future so that callers don't need to handle its big future type
    // (this prevents callers from needing to bump the recursion limit).
    Box::pin(async move {
        // Connect the client by getting its version
        x! { let x = c.connect() };
        let client_version = x.clone();

        // Send the client's version to the server and get the server's version
        let (c, s, server_version) = match s.accept(x).await.map_err(Error::Server)? {
            Step::Continue { msg: x, next: s } => {
                let server_version = x.clone();
                match c.complete_connect(x).await.map_err(Error::Client)? {
                    Step::Continue { msg: (), next: c } => (c, s, server_version),
                    Step::Done { .. } => {
                        unreachable!("client and server disagree about whether versions match")
                    }
                }
            }
            Step::Done {
                msg: x,
                output: server_output,
            } => {
                let server_version = x.clone();
                match c.complete_connect(x).await.map_err(Error::Client)? {
                    Step::Continue { .. } => {
                        unreachable!("client and server disagree about whether versions match")
                    }
                    Step::Done {
                        msg: (),
                        output: client_output,
                    } => {
                        debug_assert!(
                            client_version == server_version,
                            "server and client must agree on version to quit early"
                        );
                        return Ok((client_output, server_output));
                    }
                };
            }
        };

        // We know at this point that the client and server versions are different;
        // otherwise, both would have bailed early during the accept/complete_connect
        // phases. Their causal order is only partial (they may be concurrent), so
        // to pick an initiator we compare their *canonical bytes* lexicographically:
        // an arbitrary but total and deterministic tiebreak (not a causal order).
        // Distinct versions have distinct canonical bytes, so `Equal` is impossible.
        let (c, s) = match server_version.as_bytes().cmp(client_version.as_bytes()) {
            // If the server version is less, the client is the initiator:
            Ordering::Less => mirror_connected(c, s).await,
            // When running the server as the initiator, rearrange the result:
            Ordering::Greater => match mirror_connected(s, c).await {
                Ok((s, c)) => Ok((c, s)),
                Err(e) => Err(match e {
                    Error::Server(c) => Error::Client(c),
                    Error::Client(s) => Error::Server(s),
                }),
            },
            Ordering::Equal => unreachable!("server and client must bail early if versions match"),
        }?;

        Ok((c, s))
    })
    .await
}
