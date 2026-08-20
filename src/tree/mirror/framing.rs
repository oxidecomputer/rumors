//! Exact-read payload buffering, and the V1 wire's length-delimited
//! framing.
//!
//! Two things live here. [`read_payload`] and [`resume_payload`] grow a
//! buffer only as bytes arrive — the memory policy every variable-length
//! body read in either protocol shares, and the one the allocator meters
//! price. Around them, [`FrameRead`] and [`FrameWrite`] carry the V1
//! wire's frames: a 4-byte big-endian length followed by exactly that
//! many payload bytes. (The V2 wire's bodies are self-delimiting CBOR
//! items; only their payload reads come through here.) The reader never
//! consumes a byte beyond the frame requested.
//!
//! That guarantee makes a session boundary a stream position. A buffering
//! reader can slurp leading bytes of traffic belonging after the current
//! session and discard them when its codec is dropped, wedging later sessions
//! on the same connection. With exact reads, a clean session leaves the next
//! session's bytes untouched in the transport.
//!
//! The price is read batching: a header read followed by capacity-bounded
//! payload reads instead of one large buffered read. A caller wanting fewer
//! reads on a raw socket can wrap it in [`tokio::io::BufReader`] sized above
//! [`PAYLOAD_CHUNK_LEN`] — at the default 8 KiB capacity nearly every
//! payload read outsizes the buffer and bypasses it. Caller-owned buffering
//! is safe because it outlives a session and rides into the next one.

use tokio::io::{AsyncRead, AsyncReadExt};
#[cfg(any(test, feature = "protocol-v1"))]
use tokio::io::{AsyncWrite, AsyncWriteExt};

/// Bytes occupied by the big-endian `u32` payload-length header.
#[cfg(any(test, feature = "protocol-v1", feature = "test-internals"))]
pub(crate) const LENGTH_HEADER_LEN: usize = std::mem::size_of::<u32>();

/// The initial reservation granule for framed payload buffers.
///
/// Payload buffers grow only as bytes arrive, so a frame's memory cost
/// tracks what the peer actually delivered: a corrupt or garbage length
/// header costs at most this many reserved bytes ahead of receipt, never
/// the declared length up front. One socket-buffer-scale granule (64 KiB)
/// balances reservation events against ahead-of-receipt overshoot.
pub(crate) const PAYLOAD_CHUNK_LEN: usize = 0x1_0000;

/// Truncation cut offsets exercising every payload chunk boundary within
/// `total`: one byte short of, exactly on, and one byte past each
/// boundary, plus the zero, one-byte, and one-short-of-total cuts.
///
/// Shared by every suite pinning truncation classification at the chunk
/// seams, so the boundary roster cannot drift between them.
#[cfg(test)]
pub(crate) fn chunk_boundary_cuts(total: usize) -> Vec<usize> {
    let mut cuts = vec![0, 1];
    let mut boundary = PAYLOAD_CHUNK_LEN;
    while boundary < total {
        cuts.extend([boundary - 1, boundary, boundary + 1]);
        boundary += PAYLOAD_CHUNK_LEN;
    }
    cuts.push(total - 1);
    cuts
}

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

/// Encode the checked big-endian length header of the V1 wire codec.
#[cfg(any(test, feature = "protocol-v1"))]
pub(crate) fn length_header(len: usize) -> Result<[u8; LENGTH_HEADER_LEN], LengthOverflow> {
    let len = u32::try_from(len).map_err(|source| LengthOverflow { len, source })?;
    Ok(len.to_be_bytes())
}

/// Read exactly `len` payload bytes, growing the buffer as bytes arrive.
///
/// Memory tracks receipt, never the declared length: bytes are read
/// directly into reserved spare capacity (no zero fill), and capacity
/// doubles from one [`PAYLOAD_CHUNK_LEN`] granule only when full — so it
/// never exceeds twice the bytes already received (nor one granule,
/// before any byte arrives) and is clamped to `len`, leaving no excess
/// capacity on the returned buffer. Never consumes a byte beyond `len`.
/// A close mid-payload surfaces as
/// [`UnexpectedEof`](std::io::ErrorKind::UnexpectedEof).
pub(crate) async fn read_payload<R: AsyncRead + Unpin>(
    read: &mut R,
    len: usize,
) -> std::io::Result<Vec<u8>> {
    resume_payload(read, Vec::new(), len).await
}

/// Continue an exact `len`-byte payload read into `payload`, whose
/// existing bytes — a prefix the caller already consumed from the same
/// source — count toward `len`.
///
/// The single-buffer continuation for a caller that had to inspect a
/// payload's leading bytes before deciding to accept the rest (the
/// streaming codec's run-budget ingress check): resuming into the same
/// buffer keeps the whole read at one allocation of the payload's bytes,
/// where a read-then-splice would briefly hold the payload twice. Growth,
/// exactness, and error behavior are [`read_payload`]'s (it is this
/// function from an empty buffer).
pub(crate) async fn resume_payload<R: AsyncRead + Unpin>(
    read: &mut R,
    mut payload: Vec<u8>,
    len: usize,
) -> std::io::Result<Vec<u8>> {
    while payload.len() < len {
        if payload.len() == payload.capacity() {
            let target = (payload.capacity() * 2).max(PAYLOAD_CHUNK_LEN).min(len);
            payload.reserve_exact(target - payload.len());
        }
        if read.read_buf(&mut payload).await? == 0 {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
        }
    }
    Ok(payload)
}

#[cfg(any(test, feature = "protocol-v1", feature = "test-internals"))]
/// The read half of a session's transport, yielding one exact frame at a time.
///
/// Stateless beyond the reader it wraps: it buffers nothing, so dropping it
/// never loses stream bytes.
pub struct FrameRead<R> {
    read: R,
}

#[cfg(any(test, feature = "protocol-v1", feature = "test-internals"))]
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

#[cfg(any(test, feature = "protocol-v1", feature = "test-internals"))]
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
    ///
    /// # Cancel safety
    ///
    /// Not cancel safe. A dropped `frame` future may already have consumed
    /// part of a frame — the reads do not give bytes back — leaving the
    /// transport mid-frame, where the next call would parse payload bytes
    /// as a length header. Either retain the in-flight future across polls
    /// until it resolves, or read nothing further from this transport
    /// after a cancellation.
    pub async fn frame(&mut self) -> std::io::Result<Vec<u8>> {
        let mut header = [0u8; LENGTH_HEADER_LEN];
        self.read.read_exact(&mut header).await?;
        let len = u32::from_be_bytes(header) as usize;
        read_payload(&mut self.read, len).await
    }
}

#[cfg(any(test, feature = "protocol-v1"))]
/// The write half of a session's transport, shipping one frame at a time.
///
/// Every frame is flushed before [`frame`](Self::frame) returns, so dropping
/// the wrapper never strands bytes.
pub struct FrameWrite<W> {
    write: W,
}

#[cfg(any(test, feature = "protocol-v1"))]
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

#[cfg(any(test, feature = "protocol-v1"))]
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
