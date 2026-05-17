//! Bidirectional alternating mirror-sync between two replicas of the typed tree.

use seq_macro::seq;

pub mod local;
mod message;
pub mod protocol;
mod remote;

use protocol::*;

macro_rules! step {
    ($responder:ident) => {
        match $responder {}
    };
    ($msg:ident, $initiator:ident . $initiator_method:ident) => {
        let Step::Continue {
            msg: $msg,
            next: $initiator,
        } = $initiator.$initiator_method().map_err(Error::Initiator)?;
    };
    ($msg:ident, $initiator:ident . $initiator_method:ident <= $responder:ident . $responder_method:ident) => {
        #[allow(unused)]
        let ($msg, $initiator, $responder) = match $responder
            .$responder_method($msg)
            .map_err(Error::Responder)?
        {
            Step::Continue { msg, next } => (msg, $initiator, next),
            Step::Done {
                msg,
                output: responder_output,
            } => {
                #[allow(irrefutable_let_patterns)]
                let Step::Done {
                    output: initiator_output,
                    ..
                } = $initiator
                    .$initiator_method(msg)
                    .map_err(Error::Initiator)?
                else {
                    unreachable!("initiator did not finish after responder was finished");
                };
                return Ok((initiator_output, responder_output));
            }
        };
    };
    ($msg:ident, $initiator:ident . $initiator_method:ident => $responder:ident . $responder_method:ident) => {
        #[allow(unused)]
        let ($msg, $responder, $initiator) = match $initiator
            .$initiator_method($msg)
            .map_err(Error::Initiator)?
        {
            Step::Continue { msg, next } => (msg, $responder, next),
            Step::Done {
                msg,
                output: initiator_output,
            } => {
                #[allow(irrefutable_let_patterns)]
                let Step::Done {
                    output: responder_output,
                    ..
                } = $responder
                    .$responder_method(msg)
                    .map_err(Error::Responder)?
                else {
                    unreachable!("responder did not finish after initiator was finished")
                };
                return Ok((initiator_output, responder_output));
            }
        };
    };
}

/// An error which occurs during mirroring.
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error<I, R> {
    /// An error due to the initiator of the protocol.
    Initiator(I),
    /// An error due to the responder of the protocol.
    Responder(R),
}

/// Drive two synchronous mirror peers against one another
/// to bring them into synchronization.
pub fn mirror<I, R, P, T>(i: I, r: R) -> Result<(I::Output, R::Output), Error<I::Error, R::Error>>
where
    P: Clone + Ord + AsRef<[u8]>,
    I: Peer<P, T>,
    R: Peer<P, T>,
{
    step!(m, i.initiator);
    step!(m, i.open_initiator <= r.responder);
    step!(m, i.open_initiator => r.exchange);
    seq!(_ in 0..14 {
        step!(m, i.exchange <= r.exchange);
        step!(m, i.exchange => r.exchange);
    });
    step!(m, i.close_initiator <= r.exchange);
    step!(m, i.close_initiator => r.complete_responder);
    step!(m, i.complete_initiator <= r.complete_responder);
    step!(r);
}

#[cfg(test)]
mod message_test;

#[cfg(test)]
mod test;

#[cfg(test)]
mod wire_snapshot;
