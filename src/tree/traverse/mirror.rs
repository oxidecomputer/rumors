//! Bidirectional alternating mirror-sync between two replicas of the typed tree.

use std::cmp::Ordering;

use seq_macro::seq;

mod local;
mod message;
pub mod protocol;
pub mod remote;

#[cfg(test)]
mod message_test;

#[cfg(test)]
mod test;

#[cfg(test)]
mod wire_snapshot;

use protocol::*;

macro_rules! x {
    (let $msg:pat = $remote:ident . $remote_method:ident ( $($arg:expr)* ) ) => {
        let Step::Continue {
            msg: $msg,
            next: $remote,
        } = $remote.$remote_method($($arg)*).map_err(Error::Client)?;
    };
    ($remote:ident . $remote_method:ident == $msg:ident => $local:ident . $local_method:ident) => {
        #[allow(unused)]
        let ($msg, $local, $remote) =
            match $remote.$remote_method($msg).map_err(Error::Client)? {
                Step::Continue { msg, next } => (msg, $local, next),
                Step::Done {
                    msg,
                    output: remote_output,
                } => {
                    #[allow(irrefutable_let_patterns)]
                    let Step::Done {
                        output: local_output,
                        ..
                    } = $local.$local_method(msg).map_err(Error::Server)?
                    else {
                        unreachable!("initiator did not finish after responder was finished")
                    };
                    return Ok((remote_output, local_output));
                }
            };
    };
    ($remote:ident . $remote_method:ident <= $msg:ident == $local:ident . $local_method:ident) => {
        #[allow(unused)]
        let ($msg, $remote, $local) = match $local.$local_method($msg).map_err(Error::Server)? {
            Step::Continue { msg, next } => (msg, $remote, next),
            Step::Done {
                msg,
                output: local_output,
            } => {
                #[allow(irrefutable_let_patterns)]
                let Step::Done {
                    output: remote_output,
                    ..
                } = $remote.$remote_method(msg).map_err(Error::Client)?
                else {
                    unreachable!("responder did not finish after initiator was finished");
                };
                return Ok((remote_output, local_output));
            }
        };
    };
}

/// An error which can occur during mirroring: either a client error or a server one.
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error<C, S> {
    Client(C),
    Server(S),
}

/// Drive a mirror protocol client against a server to synchronize both of them.
pub fn mirror<C, S, P, T>(c: C, s: S) -> Result<(C::Output, S::Output), Error<C::Error, S::Error>>
where
    P: Clone + Ord + AsRef<[u8]>,
    C: Client<P, T>,
    S: Server<P, T>,
{
    // Connect the client by getting its version
    x! { let x = c.connect() };
    let client_version = x.clone();

    // Send the client's version to the server and get the server's version
    let (c, s, server_version) = match s.accept(x).map_err(Error::Server)? {
        Step::Continue { msg: x, next: s } => {
            let server_version = x.clone();
            match c.complete_connect(x).map_err(Error::Client)? {
                Step::Continue { msg: (), next: c } => (c, s, server_version),
                Step::Done { .. } => {
                    unreachable!("client and server disagree about whether versions match")
                }
            }
        }
        Step::Done {
            msg: x,
            output: server_output,
        } => match c.complete_connect(x).map_err(Error::Client)? {
            Step::Continue { .. } => {
                unreachable!("client and server disagree about whether versions match")
            }
            Step::Done {
                msg: (),
                output: client_output,
            } => return Ok((client_output, server_output)),
        },
    };

    // We know at this point that the client and server versions are different;
    // otherwise, both would have bailed early during the accept/complete_connect
    // phases. If we compare them lexicographically, we have a natural built-in
    // choice to determine who acts as the initiator vs. responder.

    // The inner mirror protocol, between an initiator and a responder (who may or
    // may not correspond with the original client/server distinction).
    fn mirror_connected<I, R, P, T>(
        i: I,
        r: R,
    ) -> Result<(I::Output, R::Output), Error<I::Error, R::Error>>
    where
        P: Clone + Ord + AsRef<[u8]>,
        I: Peer<P, T>,
        R: Peer<P, T>,
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

    match server_version.versions().cmp(client_version.versions()) {
        // If the server version is less, the client is the initiator:
        Ordering::Less => mirror_connected(c, s),
        // When running the server as the initiator, rearrange the result:
        Ordering::Greater => match mirror_connected(s, c) {
            Ok((s, c)) => Ok((c, s)),
            Err(e) => Err(match e {
                Error::Server(c) => Error::Client(c),
                Error::Client(s) => Error::Server(s),
            }),
        },
        Ordering::Equal => unreachable!("server and client must bail early if versions match"),
    }
}
