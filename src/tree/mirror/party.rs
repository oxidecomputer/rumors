//! Trailing identity hand-off after content reconciliation.

use before::Party;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};

use crate::{
    Error,
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
/// here is a counterparty bug, never an alternate encoding.
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
/// hand-off is one self-delimiting item — the party-atom tag wrapping a
/// byte string of the party's canonical encoding — so its exact boundary
/// leaves a following session preamble untouched.
pub(crate) async fn send<W>(
    party: Party,
    writer: &mut W,
    observe: &SessionHandle,
) -> Result<(), Error>
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
    writer.write_all(&item).await.map_err(Error::Io)?;
    writer.flush().await.map_err(Error::Io)?;
    observe.control_sent(&item);
    Ok(())
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
