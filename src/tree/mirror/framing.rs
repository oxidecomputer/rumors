//! Exact-read payload buffering: the memory policy beneath every
//! variable-length body read.
//!
//! [`read_payload`] and [`resume_payload`] grow a buffer only as bytes
//! arrive — never by a peer-declared length up front — and never consume
//! a byte beyond the exact count requested; they are the policy the
//! allocator meters price, and every declared-length body read (the
//! codec's framed bodies, the party hand-off's byte string) funnels
//! through them.
//!
//! The exactness guarantee makes a session boundary a stream position. A
//! buffering reader can slurp leading bytes of traffic belonging after
//! the current session and discard them when its codec is dropped,
//! wedging later sessions on the same connection. With exact reads, a
//! clean session leaves the next session's bytes untouched in the
//! transport.
//!
//! The price is read batching: capacity-bounded payload reads instead of
//! one large buffered read. A caller wanting fewer reads on a raw socket
//! can wrap it in [`tokio::io::BufReader`] sized above
//! [`PAYLOAD_CHUNK_LEN`] — at the default 8 KiB capacity nearly every
//! payload read outsizes the buffer and bypasses it. Caller-owned
//! buffering is safe because it outlives a session and rides into the
//! next one.

use tokio::io::{AsyncRead, AsyncReadExt};

/// The initial reservation granule for framed payload buffers.
///
/// Payload buffers grow only as bytes arrive, so a frame's memory cost
/// tracks what the peer actually delivered: a corrupt or garbage length
/// header costs at most this many reserved bytes ahead of receipt, never
/// the declared length up front. One socket-buffer-scale granule balances
/// reservation events against ahead-of-receipt overshoot.
pub(crate) const PAYLOAD_CHUNK_LEN: usize = 64 * 1024;

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

/// A payload length which cannot be represented by a `u32` wire length
/// header.
#[derive(Debug, thiserror::Error)]
#[error("payload length {len} exceeds the u32 framing limit")]
pub struct LengthOverflow {
    /// The unrepresentable payload length.
    pub len: usize,
    /// The failed integer conversion.
    #[source]
    pub source: std::num::TryFromIntError,
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

#[cfg(test)]
mod tests;
