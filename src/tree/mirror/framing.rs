//! Exact-read payload buffering: the memory policy beneath every
//! variable-length body read.
//!
//! [`read_payload`] grows a buffer only as bytes arrive — never by a
//! peer-declared length up front — and never consumes a byte beyond the
//! exact count requested. Every body whose declared length nothing has
//! bounded yet reads through it: the greeting, which precedes the run
//! budget, and the party hand-off's byte string.
//!
//! A caller that has bounded a declared length and set its bytes aside
//! before the read reserves the whole length at once instead
//! ([`reserve_payload`]) and reads into it ([`resume_payload`]), so the
//! buffer is one block from the start and is never reallocated. The
//! streaming codec reads its supply runs this way: the run budget judges a
//! run's length before any of its body is read, and the receive pool
//! grants a permit for it, so reserving the length costs nothing the
//! memory budget has not already set aside, while growing toward it would
//! hold two blocks at once at every reallocation. (A lone record larger
//! than the pool, which its heads must show before anything else is read,
//! takes the whole pool, and the memory budget documents its excess as
//! outside the budget.)
//!
//! The exactness guarantee makes a session boundary a stream position. A
//! reader that consumed leading bytes of the traffic after the current
//! session would lose them when its codec is dropped, wedging later
//! sessions on the same connection. With exact reads, a clean session
//! consumes nothing past its own bytes: the next session's bytes stay in
//! the caller's read half, in its buffer or in the transport beneath it.
//!
//! Sessions take a buffered read half (`AsyncBufRead`), so these reads
//! copy out of the caller's buffer, and the buffer decides how many reads
//! reach the transport. Tokio's [`tokio::io::BufReader`] refills, up to
//! its capacity, when a read finds it empty; it passes a read of at least
//! its capacity straight to the transport when it is empty; and a read
//! that finds fewer bytes buffered than it asks for takes only those. So at
//! the default 8 KiB capacity a payload read of 8 KiB or more that finds
//! the buffer empty reaches the transport directly, while a smaller
//! payload is copied out of one refill, or two when it straddles the end
//! of the bytes already buffered. The buffer is the caller's and outlives
//! a session, so whatever it holds past a session's end stays there for
//! the next one.

use bytes::BufMut;
use tokio::io::{AsyncRead, AsyncReadExt};

/// The initial reservation granule of the payload buffers
/// [`read_payload`] grows.
///
/// Those buffers grow only as bytes arrive, so a body's memory cost
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

/// An empty buffer that holds `len` payload bytes without growing, for a
/// declared length the caller has already bounded and set aside.
///
/// Reading into it with [`resume_payload`] never reallocates, so the
/// buffer's one block of `len` bytes is the most the read holds at once.
///
/// # Errors
///
/// A reservation the allocator refuses surfaces as
/// [`OutOfMemory`](std::io::ErrorKind::OutOfMemory), so that a length the
/// process cannot hold fails the read instead of aborting the process.
pub(crate) fn reserve_payload(len: usize) -> std::io::Result<Vec<u8>> {
    let mut payload = Vec::new();
    payload
        .try_reserve_exact(len)
        .map_err(|_| std::io::Error::from(std::io::ErrorKind::OutOfMemory))?;
    Ok(payload)
}

/// Continue an exact `len`-byte payload read into `payload`, whose
/// existing bytes -- a prefix the caller already consumed from the same
/// source -- count toward `len`.
///
/// Every read is bounded by the bytes still owed, whatever spare capacity
/// `payload` carries, and the buffer grows only when it is full, as
/// [`read_payload`]'s does (it is this function from an empty buffer). So
/// a buffer [reserved](reserve_payload) for `len` bytes is filled in place,
/// and its length can lead with bytes the caller consumed before it knew
/// whether to accept the rest. Exactness and error behavior are
/// [`read_payload`]'s. A prefix longer than `len` is a caller error,
/// reported as [`InvalidData`](std::io::ErrorKind::InvalidData) before any
/// read.
pub(crate) async fn resume_payload<R: AsyncRead + Unpin>(
    read: &mut R,
    mut payload: Vec<u8>,
    len: usize,
) -> std::io::Result<Vec<u8>> {
    if payload.len() > len {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "payload prefix exceeds the declared length",
        ));
    }
    while payload.len() < len {
        if payload.len() == payload.capacity() {
            let target = (payload.capacity() * 2).max(PAYLOAD_CHUNK_LEN).min(len);
            payload.reserve_exact(target - payload.len());
        }
        let owed = len - payload.len();
        if read.read_buf(&mut (&mut payload).limit(owed)).await? == 0 {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
        }
    }
    Ok(payload)
}

#[cfg(test)]
mod tests;
