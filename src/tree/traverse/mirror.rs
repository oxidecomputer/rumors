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

macro_rules! remote {
    ($msg:ident, $remote:ident . $remote_method:ident => $local:ident . $local_method:ident) => {
        // remote.responder(m): writes Initiate, reads Opening.
        #[allow(unused)]
        let ($msg, $local, $remote) = match $remote.$remote_method($msg).map_err(Error::Remote)? {
            Step::Continue { msg, next } => (msg, $local, next),
            Step::Done { msg, .. } => {
                #[allow(irrefutable_let_patterns)]
                let Step::Done { output, .. } = $local.$local_method(msg).map_err(Error::Local)?
                else {
                    unreachable!("local did not finish after remote was finished")
                };
                return Ok(output);
            }
        };
    };
}

macro_rules! local {
    ($msg:ident, $local:ident . $local_method:ident => $remote:ident . $remote_method:ident) => {
        #[allow(unused)]
        let ($msg, $remote, $local) = match $local.$local_method($msg).map_err(Error::Local)? {
            Step::Continue { msg, next } => (msg, $remote, next),
            Step::Done { msg, output } => {
                #[allow(irrefutable_let_patterns)]
                let Step::Done { .. } = $remote.$remote_method(msg).map_err(Error::Remote)? else {
                    unreachable!("remote did not finish after local was finished");
                };
                return Ok(output);
            }
        };
    };
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error<L, R> {
    Local(L),
    Remote(R),
}

pub fn initiator<Local, Remote, P, T>(
    local: Local,
    remote: Remote,
) -> Result<Local::Output, Error<Local::Error, Remote::Error>>
where
    P: Clone + Ord + AsRef<[u8]>,
    Local: Peer<P, T>,
    Remote: Peer<P, T>,
{
    let Step::Continue { msg, next: local } = local.initiator().map_err(Error::Local)?;
    remote!(msg, remote.responder => local.open_initiator);
    local!(msg, local.open_initiator => remote.exchange);
    seq_macro::seq!(_ in 0..14 {
        remote!(msg, remote.exchange => local.exchange);
        local!(msg, local.exchange => remote.exchange);
    });
    remote!(msg, remote.exchange => local.close_initiator);
    local!(msg, local.close_initiator => remote.complete_responder);
    remote!(msg, remote.complete_responder => local.complete_initiator);
    let Step::Done { output, .. } = local.complete_initiator(msg).map_err(Error::Local)?;
    Ok(output)
}

pub fn responder<Local, Remote, P, T>(
    local: Local,
    remote: Remote,
) -> Result<Local::Output, Error<Local::Error, Remote::Error>>
where
    P: Clone + Ord + AsRef<[u8]>,
    Local: Peer<P, T>,
    Remote: Peer<P, T>,
{
    let Step::Continue { msg, next: remote } = remote.initiator().map_err(Error::Remote)?;
    local!(msg, local.responder => remote.open_initiator);
    remote!(msg, remote.open_initiator => local.exchange);
    seq_macro::seq!(_ in 0..14 {
        local!(msg, local.exchange => remote.exchange);
        remote!(msg, remote.exchange => local.exchange);
    });
    local!(msg, local.exchange => remote.close_initiator);
    remote!(msg, remote.close_initiator => local.complete_responder);
    local!(msg, local.complete_responder => remote.complete_initiator);
    match local {}
}
