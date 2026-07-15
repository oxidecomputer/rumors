//! Deterministic first-error coordination of protocol and physical drivers.

use std::{future::Future, pin::pin};

use borsh::BorshDeserialize;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    select,
};

use super::{Demux, DemuxError, Incoming, Mux, MuxError, Outgoing, incoming, outgoing};
use crate::tree::mirror::streaming::remote::codec::Speaker;

/// The two physical direction drivers for one endpoint.
pub struct Drivers<R, W, T> {
    incoming: Demux<R, T>,
    outgoing: Mux<W, T>,
}

impl<R, W, T> Drivers<R, W, T> {
    /// Split a transport around one local protocol role.
    ///
    /// The outgoing direction is spoken by `local`; the incoming direction is
    /// therefore decoded as the opposite speaker. The returned logical-stream
    /// endpoints are owned by the protocol future passed to [`run`](Self::run).
    pub fn new(local: Speaker, read: R, write: W) -> (Self, Incoming<T>, Outgoing<T>) {
        let (incoming, receives) = incoming(local.other(), read);
        let (outgoing, sends) = outgoing(local, write);
        (Self { incoming, outgoing }, receives, sends)
    }
}

impl<R, W, T> Drivers<R, W, T>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
    T: BorshDeserialize + Send + 'static,
{
    /// Drive the protocol and both physical directions to completion.
    ///
    /// Poll order is deliberate. A protocol fault is observed before the
    /// sender/receiver drops it causes. Conversely, a physical fault is
    /// returned in the poll which discovers it, before its dropped receipt can
    /// wake the protocol as a generic [`SendError`](super::SendError).
    pub async fn run<P, O, E>(self, protocol: P) -> Result<(O, R, W), DriveError<E>>
    where
        P: Future<Output = Result<O, E>>,
    {
        let Self { incoming, outgoing } = self;
        let mut protocol = pin!(protocol);
        let mut incoming = pin!(incoming.run());
        let mut outgoing = pin!(outgoing.run());
        let mut protocol_output = None;
        let mut read = None;
        let mut write = None;

        loop {
            if protocol_output.is_some() && read.is_some() && write.is_some() {
                return Ok((
                    protocol_output.take().expect("the protocol completed"),
                    read.take().expect("the incoming driver completed"),
                    write.take().expect("the outgoing driver completed"),
                ));
            }

            select! {
                // Priority preserves causal errors over the channel closures
                // they trigger; see the method-level invariant above.
                biased;
                result = &mut protocol, if protocol_output.is_none() => {
                    protocol_output = Some(result.map_err(DriveError::Protocol)?);
                }
                result = &mut incoming, if read.is_none() => {
                    read = Some(result.map_err(DriveError::Incoming)?);
                }
                result = &mut outgoing, if write.is_none() => {
                    write = Some(result.map_err(DriveError::Outgoing)?);
                }
            }
        }
    }
}

/// The causal failure which stopped a coordinated remote session.
#[derive(Debug, thiserror::Error)]
pub enum DriveError<E> {
    /// The typed protocol or adapter chain failed.
    #[error("protocol failed")]
    Protocol(#[source] E),
    /// Incoming decoding, lifecycle validation, or transport input failed.
    #[error(transparent)]
    Incoming(#[from] DemuxError),
    /// Outgoing scheduling, encoding, or transport output failed.
    #[error(transparent)]
    Outgoing(#[from] MuxError),
}
