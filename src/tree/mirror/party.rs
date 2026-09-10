//! Trailing identity hand-off after content reconciliation.

use before::Party;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};

use crate::{
    Error,
    error::{Phase, TransportOperation as Op},
    observe::{CaptureRead, SessionHandle},
    tags::PARTY_TAG,
    tree::mirror::cbor::{self, HeadError, MAJOR_BSTR},
};

/// A framing or content defect in an identity donation.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum HandOffDefect {
    /// The item does not open with the party-atom tag.
    #[error("the hand-off does not carry the party-atom tag")]
    NotPartyTagged,

    /// The party-atom tag wraps something other than a byte string.
    #[error("the party-atom tag does not wrap a byte string")]
    NotAByteString,

    /// The byte string declares a length this host cannot address.
    ///
    /// A 64-bit host addresses any declarable length, so this arises
    /// only on narrower targets (e.g. `wasm32`).
    #[error("the hand-off declares an unaddressable length")]
    UnaddressableLength,

    /// A head violates the wire's deterministic-encoding contract.
    #[error("a hand-off head is not canonical: {0}")]
    HeadMalformed(HeadError),

    /// The byte string's content is not one canonical party encoding.
    ///
    /// The body arrived whole — exactly the length its head declared —
    /// so this is never a transport cut: the content itself is wrong.
    /// An encoding the declared length cuts short is
    /// [`Truncated`](before::error::Decode::Truncated) here, not
    /// a transport truncation.
    #[error("the hand-off bytes are not one canonical party encoding: {0}")]
    Undecodable(before::error::Decode),
}

/// Encode a donation now and return the future that transmits it.
///
/// The future owns the encoded frame and borrows only the writer and observer,
/// so the caller can release an identity lock before polling it. Once polling
/// begins, the caller must not resume using or recover the donated identity,
/// even if transmission fails: the peer may already have received it.
pub(crate) fn send<'a, W>(
    party: &Party,
    writer: &'a mut W,
    observe: &'a SessionHandle,
) -> impl Future<Output = Result<(), Error>> + use<'a, W>
where
    W: AsyncWrite + Unpin + ?Sized,
{
    let bytes = party.as_bytes();
    let mut item = Vec::with_capacity(
        cbor::head_len(PARTY_TAG) + cbor::head_len(bytes.len() as u64) + bytes.len(),
    );
    cbor::write_tag(&mut item, PARTY_TAG);
    cbor::write_head(&mut item, MAJOR_BSTR, bytes.len() as u64);
    item.extend_from_slice(bytes);
    async move {
        writer
            .write_all(&item)
            .await
            .map_err(|source| Error::transport(Phase::IdentityTransfer, Op::Write, source))?;
        writer
            .flush()
            .await
            .map_err(|source| Error::transport(Phase::IdentityTransfer, Op::Flush, source))?;
        observe.control_sent(&item);
        Ok(())
    }
}

/// Receive the identity donation promised by the peer's preamble intent.
pub(crate) async fn receive<R>(reader: &mut R, observe: &SessionHandle) -> Result<Party, Error>
where
    R: AsyncRead + Unpin + ?Sized,
{
    if observe.attached() {
        let mut capture = CaptureRead::new(reader);
        let party = receive_item(&mut capture).await?;
        observe.control_received(capture.bytes());
        Ok(party)
    } else {
        receive_item(reader).await
    }
}

/// Read and decode one hand-off item.
async fn receive_item<R>(reader: &mut R) -> Result<Party, Error>
where
    R: AsyncRead + Unpin + ?Sized,
{
    let malformed = |defect| Error::violation(Phase::IdentityTransfer, defect);
    let head = read_head(reader).await?;
    if head.major != cbor::MAJOR_TAG || head.value != PARTY_TAG {
        return Err(malformed(HandOffDefect::NotPartyTagged));
    }
    let head = read_head(reader).await?;
    if head.major != MAJOR_BSTR {
        return Err(malformed(HandOffDefect::NotAByteString));
    }
    let Ok(len) = usize::try_from(head.value) else {
        return Err(malformed(HandOffDefect::UnaddressableLength));
    };
    // `read_payload` spells a close mid-payload as `UnexpectedEof`; a
    // transport failure keeps its own kind and passes through.
    let bytes = crate::tree::mirror::framing::read_payload(&mut &mut *reader, len)
        .await
        .map_err(|source| Error::transport(Phase::IdentityTransfer, Op::Read, source))?;
    decode_party(&bytes)
}

/// Decode a complete donation body; failures describe content, not transport.
fn decode_party(bytes: &[u8]) -> Result<Party, Error> {
    Party::decode(bytes).map_err(|defect| {
        Error::violation(Phase::IdentityTransfer, HandOffDefect::Undecodable(defect))
    })
}

/// Read a canonical head, preserving I/O failures and premature closes.
async fn read_head<R>(reader: &mut R) -> Result<cbor::Head, Error>
where
    R: AsyncRead + Unpin + ?Sized,
{
    match cbor::read_head_async(reader).await {
        Ok(Some(head)) => Ok(head),
        Ok(None) => Err(Error::transport(
            Phase::IdentityTransfer,
            Op::Read,
            std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "identity donation ended before its item head",
            ),
        )),
        Err(cbor::HeadReadError::Io(source)) => {
            Err(Error::transport(Phase::IdentityTransfer, Op::Read, source))
        }
        Err(cbor::HeadReadError::Malformed(head)) => Err(Error::violation(
            Phase::IdentityTransfer,
            HandOffDefect::HeadMalformed(head),
        )),
    }
}

#[cfg(test)]
mod tests;
