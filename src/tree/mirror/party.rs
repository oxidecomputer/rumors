//! Trailing identity hand-off after content reconciliation.

use before::Party;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};

use crate::{
    Error, Protocol,
    tree::mirror::cbor::{self, MAJOR_BSTR},
};

/// Ship a donated party after reconciliation has transferred all content.
///
/// Bootstrapping sends a freshly forked party from provider to newcomer;
/// retirement sends the retiree's whole party toward its absorber. The
/// hand-off's spelling is the selected dialect's: under V2, one
/// self-delimiting item — the party-atom tag wrapping a byte string of
/// the party's canonical encoding — and under the frozen V1 wire, one
/// length-delimited frame of the bare encoding. Either way its exact
/// boundary leaves a following session preamble untouched.
pub(crate) async fn send<W>(protocol: Protocol, party: Party, writer: &mut W) -> Result<(), Error>
where
    W: AsyncWrite + Unpin + ?Sized,
{
    #[cfg(any(test, feature = "protocol-v1"))]
    if protocol == Protocol::V1 {
        // The frame delimits, so the body is the party's canonical
        // encoding, bare.
        crate::tree::mirror::framing::FrameWrite::new(writer)
            .frame(party.as_bytes())
            .await?;
        return Ok(());
    }
    let _ = protocol;
    let bytes = party.as_bytes();
    let mut item = Vec::with_capacity(
        cbor::head_len(crate::tags::PARTY_TAG) + cbor::head_len(bytes.len() as u64) + bytes.len(),
    );
    cbor::write_tag(&mut item, crate::tags::PARTY_TAG);
    cbor::write_head(&mut item, MAJOR_BSTR, bytes.len() as u64);
    item.extend_from_slice(bytes);
    writer.write_all(&item).await.map_err(Error::Io)?;
    writer.flush().await.map_err(Error::Io)
}

/// Receive the identity donation promised by the peer's preamble intent.
pub(crate) async fn receive<R>(protocol: Protocol, reader: &mut R) -> Result<Party, Error>
where
    R: AsyncRead + Unpin + ?Sized,
{
    #[cfg(any(test, feature = "protocol-v1"))]
    if protocol == Protocol::V1 {
        let bytes = crate::tree::mirror::framing::FrameRead::new(reader)
            .frame()
            .await?;
        return decode_party(&bytes);
    }
    let _ = protocol;
    let invalid = |message: &'static str| {
        Error::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            message,
        ))
    };
    let head = read_head(reader).await?;
    if head.major != cbor::MAJOR_TAG || head.value != crate::tags::PARTY_TAG {
        return Err(invalid(
            "identity hand-off does not carry the party-atom tag",
        ));
    }
    let head = read_head(reader).await?;
    if head.major != MAJOR_BSTR {
        return Err(invalid("party-atom tag does not wrap a byte string"));
    }
    let Ok(len) = usize::try_from(head.value) else {
        return Err(invalid(
            "identity hand-off declares an unaddressable length",
        ));
    };
    let bytes = crate::tree::mirror::framing::read_payload(&mut &mut *reader, len)
        .await
        .map_err(Error::Io)?;
    decode_party(&bytes)
}

/// Decode one exact donation body into its canonical party.
fn decode_party(bytes: &[u8]) -> Result<Party, Error> {
    Party::decode(bytes)
        .map_err(|e| match e {
            // An item that ends inside the encoding is a truncation, not
            // corruption; the reader's own failures pass through.
            before::error::Decode::Truncated => {
                std::io::Error::new(std::io::ErrorKind::UnexpectedEof, e)
            }
            before::error::Decode::Io(e) => e,
            e => std::io::Error::new(std::io::ErrorKind::InvalidData, e),
        })
        .map_err(Error::Io)
}

/// Read one canonical head, treating any close as an unexpected cut: the
/// hand-off was promised by the peer's preamble intent.
async fn read_head<R>(reader: &mut R) -> Result<cbor::Head, Error>
where
    R: AsyncRead + Unpin + ?Sized,
{
    match cbor::read_head_async(reader).await {
        Ok(Some(head)) => Ok(head),
        Ok(None) => Err(Error::Io(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "peer closed before its promised identity hand-off",
        ))),
        Err(cbor::HeadReadError::Io(io)) => Err(Error::Io(io)),
        Err(cbor::HeadReadError::Malformed(head)) => Err(Error::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            head.to_string(),
        ))),
    }
}

#[cfg(test)]
mod tests;
