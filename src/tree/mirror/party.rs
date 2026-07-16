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
    let mut bytes = Vec::new();
    borsh::BorshSerialize::serialize(&party, &mut bytes)?;
    FrameWrite::new(writer).frame(&bytes).await?;
    Ok(())
}

/// Receive the identity donation promised by the peer's preamble intent.
pub(crate) async fn receive<R>(reader: &mut R) -> Result<Party, Error>
where
    R: AsyncRead + Unpin + ?Sized,
{
    use borsh::BorshDeserialize as _;

    let bytes = FrameRead::new(reader).frame().await?;
    Party::try_from_slice(&bytes).map_err(Error::Io)
}
