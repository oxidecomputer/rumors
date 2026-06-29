//! Exact-read length-delimited framing for gossip sessions.
//!
//! Everything the protocol puts on the wire — the preamble included — is one
//! *frame*: a 4-byte big-endian length followed by exactly that many payload
//! bytes. What distinguishes this layer from an off-the-shelf framed reader
//! (`tokio_util::codec::FramedRead`) is one guarantee: **the reader never
//! consumes a byte beyond the frame it was asked for.**
//!
//! That guarantee is what makes a session boundary a *stream position*. A
//! buffering reader fills its buffer with whatever the transport has
//! available, so it can slurp the leading bytes of traffic that belongs
//! *after* the current session — the peer's next-session preamble, written
//! eagerly the moment its own session ended — and silently discard them when
//! the session's reader is dropped, wedging every later session on the same
//! connection. With exact reads, a session that returns `Ok` leaves the
//! next session's bytes untouched in the transport, so one connection can
//! host back-to-back sessions indefinitely.
//!
//! The price is read batching: two reads per frame (header, then payload)
//! instead of one large buffered read. A caller who wants fewer reads on a
//! raw socket wraps it in a [`tokio::io::BufReader`] — buffering the
//! *caller* owns is safe precisely because it outlives the session and rides
//! into the next one.

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// The read half of a session's transport, yielding one exact frame at a
/// time.
///
/// Stateless beyond the reader it wraps: it buffers nothing, so
/// dropping it (or constructing a fresh one over the same reader) never
/// loses stream bytes.
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
    /// The length is peer-supplied and trusted without a cap, so this must
    /// only run after the preamble has validated the counterparty (the
    /// preamble itself avoids that trust by arriving through
    /// [`fill_exact`](Self::fill_exact) at its known size). A close
    /// mid-frame surfaces as
    /// [`UnexpectedEof`](std::io::ErrorKind::UnexpectedEof).
    pub async fn frame(&mut self) -> std::io::Result<Vec<u8>> {
        let mut header = [0u8; 4];
        self.read.read_exact(&mut header).await?;
        let len = u32::from_be_bytes(header) as usize;
        let mut payload = vec![0u8; len];
        self.read.read_exact(&mut payload).await?;
        Ok(payload)
    }

    /// Drive `buf` toward full from the stream, *cancel-safely*.
    ///
    /// All progress lives in (`buf`, `filled`), none in the returned future, so
    /// the future can be dropped (say, by losing a `select!`) and a later
    /// call resumes exactly where the read left off. This is how a gossip
    /// driver holds a pending read for a remote-led session while staying
    /// free to initiate one itself.
    ///
    /// Resolves [`Fill::Filled`] once `*filled == buf.len()`, and
    /// [`Fill::Closed`] if the stream ends *before the first byte* — the
    /// one EOF that is a boundary, not a truncation. An EOF with the buffer
    /// part-full is a truncation and surfaces as
    /// [`UnexpectedEof`](std::io::ErrorKind::UnexpectedEof).
    pub async fn fill_exact(
        &mut self,
        buf: &mut [u8],
        filled: &mut usize,
    ) -> std::io::Result<Fill> {
        std::future::poll_fn(|cx| {
            while *filled < buf.len() {
                let mut chunk = tokio::io::ReadBuf::new(&mut buf[*filled..]);
                match std::pin::Pin::new(&mut self.read).poll_read(cx, &mut chunk) {
                    std::task::Poll::Pending => return std::task::Poll::Pending,
                    std::task::Poll::Ready(Err(e)) => return std::task::Poll::Ready(Err(e)),
                    std::task::Poll::Ready(Ok(())) => match chunk.filled().len() {
                        0 if *filled == 0 => {
                            return std::task::Poll::Ready(Ok(Fill::Closed));
                        }
                        0 => {
                            return std::task::Poll::Ready(Err(std::io::Error::new(
                                std::io::ErrorKind::UnexpectedEof,
                                "peer closed mid-frame",
                            )));
                        }
                        n => *filled += n,
                    },
                }
            }
            std::task::Poll::Ready(Ok(Fill::Filled))
        })
        .await
    }
}

/// How a [`fill_exact`](FrameRead::fill_exact) drive ended, separating the
/// two meanings of end-of-stream.
///
/// A peer that hung up *between* frames closed at a boundary
/// ([`Closed`](Fill::Closed)); one that hung up inside a frame truncated it
/// (an error, not a variant).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fill {
    /// The buffer is full.
    Filled,
    /// The stream ended cleanly before the first byte of the buffer.
    Closed,
}

/// The write half of a session's transport, shipping one frame at a time.
/// Stateless beyond the writer it wraps: every frame is flushed before
/// [`frame`](Self::frame) returns, so dropping it never strands bytes.
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
    /// The flush is part of the contract, not a courtesy: the protocol is
    /// lock-step, with the counterparty reading each frame before sending
    /// its next, so a frame held back by a buffering transport (a
    /// compression layer, a TLS record buffer) deadlocks both sides. A raw
    /// socket forwards immediately and masks the problem, but the
    /// `AsyncWrite` contract does not promise it.
    ///
    /// # Errors
    ///
    /// Rejects a payload longer than `u32::MAX` bytes with
    /// [`InvalidInput`](std::io::ErrorKind::InvalidInput) before writing
    /// anything: the length header cannot represent it.
    pub async fn frame(&mut self, payload: &[u8]) -> std::io::Result<()> {
        let len = u32::try_from(payload.len()).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "frame payload exceeds the u32 length header",
            )
        })?;
        self.write.write_all(&len.to_be_bytes()).await?;
        self.write.write_all(payload).await?;
        self.write.flush().await
    }
}
