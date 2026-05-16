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

use std::io::{Read, Write};

use crate::tree::typed::{Node, height::Root};
use crate::{Key, Message, Version};
use borsh::{BorshDeserialize, BorshSerialize};
use protocol::*;

macro_rules! remote {
    ($msg:ident, $remote:ident . $remote_method:ident => $local:ident . $local_method:ident) => {
        // remote.responder(m): writes Initiate, reads Opening.
        #[allow(unused)]
        let ($msg, $local, $remote) = match $remote.$remote_method($msg)? {
            Step::Continue { msg, next } => (msg, $local, next),
            Step::Done { msg, output: () } => {
                #[allow(irrefutable_let_patterns)]
                let Ok(Step::Done { output, .. }) = $local.$local_method(msg) else {
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
        let ($msg, $remote, $local) = match $local.$local_method($msg) {
            Ok(Step::Continue { msg, next }) => (msg, $remote, next),
            Ok(Step::Done { msg, output }) => {
                #[allow(irrefutable_let_patterns)]
                let Step::Done { .. } = $remote.$remote_method(msg)? else {
                    unreachable!("remote did not finish after local was finished");
                };
                return Ok(output);
            }
        };
    };
}

pub fn initiator<P, T, R, W, OnSend, OnRecv>(
    node: Option<Node<P, T, Root>>,
    their_version: &Version<P>,
    on_send: OnSend,
    on_recv: OnRecv,
    reader: R,
    writer: W,
) -> Result<Option<Node<P, T, Root>>, remote::Error>
where
    R: Read,
    W: Write,
    OnRecv: FnMut(&Version<P>, Key, &Message<T>),
    OnSend: FnMut(&Version<P>, Key, &Message<T>),
    P: Clone + Ord + AsRef<[u8]> + BorshDeserialize + BorshSerialize,
    T: Clone + BorshDeserialize + BorshSerialize,
{
    let local = local::Exchange::start(node, their_version, on_send, on_recv);
    let remote = remote::Exchange::start(reader, writer);

    let Ok(Step::Continue { msg, next: local }) = local.initiator();
    remote!(msg, remote.responder => local.open_initiator);
    local!(msg, local.open_initiator => remote.exchange);
    seq_macro::seq!(_ in 0..14 {
        remote!(msg, remote.exchange => local.exchange);
        local!(msg, local.exchange => remote.exchange);
    });
    remote!(msg, remote.exchange => local.close_initiator);
    local!(msg, local.close_initiator => remote.complete_responder);
    remote!(msg, remote.complete_responder => local.complete_initiator);
    let Ok(Step::Done { output, .. }) = local.complete_initiator(msg);

    Ok(output)
}

pub fn responder<P, T, R, W, OnSend, OnRecv>(
    node: Option<Node<P, T, Root>>,
    their_version: &Version<P>,
    on_send: OnSend,
    on_recv: OnRecv,
    reader: R,
    writer: W,
) -> Result<Option<Node<P, T, Root>>, remote::Error>
where
    R: Read,
    W: Write,
    OnRecv: FnMut(&Version<P>, Key, &Message<T>),
    OnSend: FnMut(&Version<P>, Key, &Message<T>),
    P: Clone + Ord + AsRef<[u8]> + BorshDeserialize + BorshSerialize,
    T: Clone + BorshDeserialize + BorshSerialize,
{
    let local = local::Exchange::start(node, their_version, on_send, on_recv);
    let remote = remote::Exchange::<P, T, _, _, _>::start(reader, writer);

    let Step::Continue { msg, next: remote } = remote.initiator()?;
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
