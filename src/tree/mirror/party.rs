//! Trailing identity hand-off after content reconciliation.

use before::Party;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::{
    Error,
    tree::mirror::framing::{FrameRead, FrameWrite},
};

/// Ship a donated party after reconciliation has transferred all content.
///
/// Bootstrapping sends a freshly forked party from provider to newcomer;
/// retirement sends the retiree's whole party toward its absorber. The exact
/// frame boundary leaves a following session preamble untouched.
pub(crate) async fn send<W>(party: Party, writer: &mut W) -> Result<(), Error>
where
    W: AsyncWrite + Unpin + ?Sized,
{
    // The frame delimits, so the body is the party's canonical encoding,
    // bare.
    FrameWrite::new(writer).frame(party.as_bytes()).await?;
    Ok(())
}

/// Receive the identity donation promised by the peer's preamble intent.
pub(crate) async fn receive<R>(reader: &mut R) -> Result<Party, Error>
where
    R: AsyncRead + Unpin + ?Sized,
{
    let bytes = FrameRead::new(reader).frame().await?;
    Party::decode(&bytes[..])
        .map_err(|e| match e {
            // A frame that ends inside the encoding is a truncation, not
            // corruption; the reader's own failures pass through.
            before::error::Decode::Truncated => {
                std::io::Error::new(std::io::ErrorKind::UnexpectedEof, e)
            }
            before::error::Decode::Io(e) => e,
            e => std::io::Error::new(std::io::ErrorKind::InvalidData, e),
        })
        .map_err(Error::Io)
}

#[cfg(test)]
mod tests;
