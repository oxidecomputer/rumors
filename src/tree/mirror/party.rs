//! Trailing identity hand-off after content reconciliation.

use before::Party;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};

use crate::{
    Error, Protocol,
    observe::{CaptureRead, SessionHandle},
    tags::PARTY_TAG,
    tree::mirror::cbor::{self, HeadError, MAJOR_BSTR},
};

/// Which part of a delivered identity hand-off failed to parse.
///
/// Carried by [`Error::HandOffMalformed`]: the peer
/// delivered its promised identity hand-off, but the item is not spelled
/// the way the wire demands, or its content is not one canonical party
/// encoding. The hand-off is deterministic-encoding CBOR wrapping a
/// canonical party encoding — one spelling per donation — so every defect
/// here is a counterparty bug, never an alternate encoding. Reachable
/// only for [`Protocol::V2`].
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
    /// [`Error::HandOffTruncated`].
    #[error("the hand-off bytes are not one canonical party encoding: {0}")]
    Undecodable(before::error::Decode),
}

/// Ship a donated party after reconciliation has transferred all content.
///
/// Bootstrapping sends a freshly forked party from provider to newcomer;
/// retirement sends the retiree's whole party toward its absorber. The
/// hand-off's spelling is the selected dialect's: under V2, one
/// self-delimiting item — the party-atom tag wrapping a byte string of
/// the party's canonical encoding — and under the frozen V1 wire, one
/// length-delimited frame of the bare encoding. Either way its exact
/// boundary leaves a following session preamble untouched.
pub(crate) async fn send<W>(
    protocol: Protocol,
    party: Party,
    writer: &mut W,
    observe: &SessionHandle,
) -> Result<(), Error>
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
        cbor::head_len(PARTY_TAG) + cbor::head_len(bytes.len() as u64) + bytes.len(),
    );
    cbor::write_tag(&mut item, PARTY_TAG);
    cbor::write_head(&mut item, MAJOR_BSTR, bytes.len() as u64);
    item.extend_from_slice(bytes);
    writer.write_all(&item).await.map_err(Error::Io)?;
    writer.flush().await.map_err(Error::Io)?;
    observe.control_sent(&item);
    Ok(())
}

/// Receive the identity donation promised by the peer's preamble intent.
pub(crate) async fn receive<R>(
    protocol: Protocol,
    reader: &mut R,
    observe: &SessionHandle,
) -> Result<Party, Error>
where
    R: AsyncRead + Unpin + ?Sized,
{
    #[cfg(any(test, feature = "protocol-v1"))]
    if protocol == Protocol::V1 {
        let bytes = crate::tree::mirror::framing::FrameRead::new(reader)
            .frame()
            .await?;
        return decode_party_v1(&bytes);
    }
    let _ = protocol;
    if observe.attached() {
        let mut capture = CaptureRead::new(reader);
        let party = receive_v2(&mut capture).await?;
        observe.control_received(capture.bytes());
        Ok(party)
    } else {
        receive_v2(reader).await
    }
}

/// Read and decode one V2 hand-off item.
async fn receive_v2<R>(reader: &mut R) -> Result<Party, Error>
where
    R: AsyncRead + Unpin + ?Sized,
{
    let malformed = |defect| Error::HandOffMalformed { defect };
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
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::UnexpectedEof => Error::HandOffTruncated,
            _ => Error::Io(e),
        })?;
    decode_party(&bytes)
}

/// Decode one exact donation body into its canonical party.
///
/// The body arrived whole, so every decode failure is the content's own:
/// a typed hand-off defect, never a transport error. The one exception
/// is the reader's own failure, which passes through — unreachable from
/// a slice, kept total.
fn decode_party(bytes: &[u8]) -> Result<Party, Error> {
    Party::decode(bytes).map_err(|defect| match defect {
        before::error::Decode::Io(e) => Error::Io(e),
        defect => Error::HandOffMalformed {
            defect: HandOffDefect::Undecodable(defect),
        },
    })
}

/// Decode one exact donation body into its canonical party, spelling
/// failures in the frozen V1 dialect's I/O vocabulary.
#[cfg(any(test, feature = "protocol-v1"))]
fn decode_party_v1(bytes: &[u8]) -> Result<Party, Error> {
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

/// Read one canonical head, treating any close as a truncation of the
/// hand-off the peer's preamble intent promised.
///
/// `read_head_async` spells a close inside a head as `UnexpectedEof`, so
/// that kind joins the clean close before the first byte as
/// [`Error::HandOffTruncated`]; a transport failure keeps its own kind
/// and passes through as [`Error::Io`].
async fn read_head<R>(reader: &mut R) -> Result<cbor::Head, Error>
where
    R: AsyncRead + Unpin + ?Sized,
{
    match cbor::read_head_async(reader).await {
        Ok(Some(head)) => Ok(head),
        Ok(None) => Err(Error::HandOffTruncated),
        Err(cbor::HeadReadError::Io(io)) if io.kind() == std::io::ErrorKind::UnexpectedEof => {
            Err(Error::HandOffTruncated)
        }
        Err(cbor::HeadReadError::Io(io)) => Err(Error::Io(io)),
        Err(cbor::HeadReadError::Malformed(head)) => Err(Error::HandOffMalformed {
            defect: HandOffDefect::HeadMalformed(head),
        }),
    }
}

#[cfg(test)]
mod tests;
