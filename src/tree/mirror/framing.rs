//! Exact-read length-delimited framing shared by the mirror wire protocols.
//!
//! A framed body is a 4-byte big-endian length followed by exactly that many
//! payload bytes. The alternating protocol uses this envelope for every
//! message; the streaming protocol uses it for its variable-width supplied
//! leaves, while its signal-delimited fixed bodies remain bare. The reader
//! never consumes a byte beyond the frame requested.
//!
//! That guarantee makes a session boundary a stream position. A buffering
//! reader can slurp leading bytes of traffic belonging after the current
//! session and discard them when its codec is dropped, wedging later sessions
//! on the same connection. With exact reads, a clean session leaves the next
//! session's bytes untouched in the transport.
//!
//! The price is read batching: two reads per framed body (header, then payload)
//! instead of one large buffered read. A caller wanting fewer reads on a raw
//! socket can wrap it in [`tokio::io::BufReader`]; caller-owned buffering is
//! safe because it outlives a session and rides into the next one.

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Bytes occupied by the big-endian `u32` payload-length header.
pub(crate) const LENGTH_HEADER_LEN: usize = std::mem::size_of::<u32>();

/// A payload length which cannot be represented by the framing header.
#[derive(Debug, thiserror::Error)]
#[error("payload length {len} exceeds the u32 framing limit")]
pub struct LengthOverflow {
    /// The unrepresentable payload length.
    pub len: usize,
    /// The failed integer conversion.
    #[source]
    pub source: std::num::TryFromIntError,
}

/// Encode the checked big-endian length header shared by both wire codecs.
pub(crate) fn length_header(len: usize) -> Result<[u8; LENGTH_HEADER_LEN], LengthOverflow> {
    let len = u32::try_from(len).map_err(|source| LengthOverflow { len, source })?;
    Ok(len.to_be_bytes())
}

/// The read half of a session's transport, yielding one exact frame at a time.
///
/// Stateless beyond the reader it wraps: it buffers nothing, so dropping it
/// never loses stream bytes.
pub struct FrameRead<R> {
    read: R,
}

impl<R> FrameRead<R> {
    /// Wrap `read` for frame-at-a-time reading.
    pub fn new(read: R) -> Self {
        Self { read }
    }
}

impl<R: AsyncRead + Unpin> FrameRead<R> {
    /// Read one frame, allocating room for the peer-declared length.
    ///
    /// The length is peer-supplied and trusted without a cap, so this must only
    /// run after the preamble validates the counterparty. A close mid-frame
    /// surfaces as [`UnexpectedEof`](std::io::ErrorKind::UnexpectedEof).
    pub async fn frame(&mut self) -> std::io::Result<Vec<u8>> {
        let mut header = [0u8; LENGTH_HEADER_LEN];
        self.read.read_exact(&mut header).await?;
        let len = u32::from_be_bytes(header) as usize;
        let mut payload = vec![0u8; len];
        self.read.read_exact(&mut payload).await?;
        Ok(payload)
    }
}

/// The write half of a session's transport, shipping one frame at a time.
///
/// Every frame is flushed before [`frame`](Self::frame) returns, so dropping
/// the wrapper never strands bytes.
pub struct FrameWrite<W> {
    write: W,
}

impl<W> FrameWrite<W> {
    /// Wrap `write` for frame-at-a-time writing.
    pub fn new(write: W) -> Self {
        Self { write }
    }
}

impl<W: AsyncWrite + Unpin> FrameWrite<W> {
    /// Write `payload` as one frame — length header, then bytes — and flush.
    ///
    /// Rejects payloads longer than `u32::MAX` before writing anything.
    pub async fn frame(&mut self, payload: &[u8]) -> std::io::Result<()> {
        let header = length_header(payload.len())
            .map_err(|source| std::io::Error::new(std::io::ErrorKind::InvalidInput, source))?;
        self.write.write_all(&header).await?;
        self.write.write_all(payload).await?;
        self.write.flush().await
    }
}
