//! Exact-read length-delimited framing shared by the mirror wire protocols.
//!
//! A framed body is a 4-byte big-endian length followed by exactly that many
//! payload bytes. The streaming protocol uses it for its greeting (the
//! causal-version and root-fan listing frames), variable-width supply runs
//! and their leaf records, and the trailing identity hand-off;
//! signal-delimited fixed bodies remain bare. The reader never consumes a byte
//! beyond the frame requested.
//!
//! That guarantee makes a session boundary a stream position. A buffering
//! reader can slurp leading bytes of traffic belonging after the current
//! session and discard them when its codec is dropped, wedging later sessions
//! on the same connection. With exact reads, a clean session leaves the next
//! session's bytes untouched in the transport.
//!
//! The price is read batching: a header read followed by chunked payload
//! reads instead of one large buffered read. A caller wanting fewer reads on
//! a raw socket can wrap it in [`tokio::io::BufReader`]; caller-owned
//! buffering is safe because it outlives a session and rides into the next
//! one.

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Bytes occupied by the big-endian `u32` payload-length header.
pub(crate) const LENGTH_HEADER_LEN: usize = std::mem::size_of::<u32>();

/// Bytes zero-filled and read per step while receiving a framed payload.
///
/// Payload buffers grow only as bytes arrive, so a frame's memory cost
/// tracks what the peer actually delivered: a corrupt or garbage length
/// header costs at most this many pre-touched bytes ahead of receipt,
/// never the declared length up front. One socket-buffer-scale chunk
/// balances loop iterations against pre-touch overshoot.
pub(crate) const PAYLOAD_CHUNK_LEN: usize = 64 * 1024;

/// Bytes of one negotiated size word in the greeting's version frame: a
/// little-endian `u64`.
pub(crate) const GREETING_WORD_LEN: usize = std::mem::size_of::<u64>();

/// The greeting version frame's fixed prefix: three size words (the
/// sender's set size, version-size bound, and message-size target) ahead
/// of the version encoding.
///
/// Sender, receiver, and every fixture measuring greeting frames must
/// agree on this width; it is defined once here so the layout can only
/// change in one place.
pub(crate) const GREETING_SIZE_WORDS_LEN: usize = 3 * GREETING_WORD_LEN;

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

/// Read exactly `len` payload bytes, growing the buffer as bytes arrive.
///
/// Memory tracks receipt, never the declared length: each step zero-fills
/// and reads at most [`PAYLOAD_CHUNK_LEN`] bytes, and the buffer's growth
/// is driven by bytes already received. Never consumes a byte beyond
/// `len`. A close mid-payload surfaces as
/// [`UnexpectedEof`](std::io::ErrorKind::UnexpectedEof).
pub(crate) async fn read_payload<R: AsyncRead + Unpin>(
    read: &mut R,
    len: usize,
) -> std::io::Result<Vec<u8>> {
    let mut payload = Vec::new();
    while payload.len() < len {
        let filled = payload.len();
        let step = PAYLOAD_CHUNK_LEN.min(len - filled);
        payload.resize(filled + step, 0);
        read.read_exact(&mut payload[filled..]).await?;
    }
    Ok(payload)
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

    /// Unwrap the exact-frame reader at a frame boundary.
    ///
    /// Only the alternating protocol hands framed halves back to raw
    /// transport, so this rides its feature gate.
    #[cfg(any(test, feature = "protocol-v1"))]
    pub fn into_inner(self) -> R {
        self.read
    }
}

impl<R: AsyncRead + Unpin> FrameRead<R> {
    /// Read one frame, growing the payload buffer as its bytes arrive.
    ///
    /// The length is peer-supplied and uncapped — a post-preamble trust
    /// decision, so this must only run after the preamble validates the
    /// counterparty — but it only bounds how many bytes are read: memory
    /// tracks bytes actually received ([`read_payload`]), so a garbage
    /// length costs I/O-proportional memory, never its declared size up
    /// front. A close mid-frame surfaces as
    /// [`UnexpectedEof`](std::io::ErrorKind::UnexpectedEof).
    pub async fn frame(&mut self) -> std::io::Result<Vec<u8>> {
        let mut header = [0u8; LENGTH_HEADER_LEN];
        self.read.read_exact(&mut header).await?;
        let len = u32::from_be_bytes(header) as usize;
        read_payload(&mut self.read, len).await
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

    /// Unwrap the frame writer after its last flushed frame.
    ///
    /// Only the alternating protocol hands framed halves back to raw
    /// transport, so this rides its feature gate.
    #[cfg(any(test, feature = "protocol-v1"))]
    pub fn into_inner(self) -> W {
        self.write
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

#[cfg(test)]
mod tests;
