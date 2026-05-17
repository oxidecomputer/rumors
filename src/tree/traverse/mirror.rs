//! Bidirectional alternating mirror-sync between two replicas of the typed tree.

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

macro_rules! step {
    ($remote:ident . $remote_method:ident : $msg:ident) => {
        let Step::Continue {
            msg: $msg,
            next: $remote,
        } = $remote.$remote_method().map_err(Error::Initiator)?;
    };
    ($remote:ident . $remote_method:ident == $msg:ident => $local:ident . $local_method:ident) => {
        #[allow(unused)]
        let ($msg, $local, $remote) =
            match $remote.$remote_method($msg).map_err(Error::Initiator)? {
                Step::Continue { msg, next } => (msg, $local, next),
                Step::Done {
                    msg,
                    output: remote_output,
                } => {
                    #[allow(irrefutable_let_patterns)]
                    let Step::Done {
                        output: local_output,
                        ..
                    } = $local.$local_method(msg).map_err(Error::Responder)?
                    else {
                        unreachable!("initiator did not finish after responder was finished")
                    };
                    return Ok((remote_output, local_output));
                }
            };
    };
    ($remote:ident . $remote_method:ident <= $msg:ident == $local:ident . $local_method:ident) => {
        #[allow(unused)]
        let ($msg, $remote, $local) = match $local.$local_method($msg).map_err(Error::Responder)? {
            Step::Continue { msg, next } => (msg, $remote, next),
            Step::Done {
                msg,
                output: local_output,
            } => {
                #[allow(irrefutable_let_patterns)]
                let Step::Done {
                    output: remote_output,
                    ..
                } = $remote.$remote_method(msg).map_err(Error::Initiator)?
                else {
                    unreachable!("responder did not finish after initiator was finished");
                };
                return Ok((remote_output, local_output));
            }
        };
    };
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error<I, R> {
    Initiator(I),
    Responder(R),
}

pub fn mirror<I, R, P, T>(i: I, r: R) -> Result<(I::Output, R::Output), Error<I::Error, R::Error>>
where
    P: Clone + Ord + AsRef<[u8]>,
    I: Peer<P, T>,
    R: Peer<P, T>,
{
    step!( i.initiator: x );
    step!( i.open_initiator <=x== r.responder );
    step!( i.open_initiator ==x=> r.exchange  );
    seq_macro::seq!(_ in 0..14 {
        step!( i.exchange <=x== r.exchange );
        step!( i.exchange ==x=> r.exchange );
    });
    step!( i.close_initiator    <=x== r.exchange           );
    step!( i.close_initiator    ==x=> r.complete_responder );
    step!( i.complete_initiator <=x== r.complete_responder );

    match r {}
}
