//! Direct asynchronous output for validated frame encodings.

use tokio::io::{AsyncWrite, AsyncWriteExt};

use super::super::{
    error::{EncodeError, EncodeErrorKind, FramePart},
    frame::WireFrame,
    signal::Speaker,
};
use super::{BodyEncoding, FrameEncoding};

/// Async frame writer over one speaker's transport direction.
///
/// A frame is validated before its signal is written. Its pieces then go
/// directly to the transport and are flushed before [`frame`](Self::frame)
/// returns, so the caller can safely publish the corresponding internal work.
pub struct FrameWrite<W> {
    speaker: Speaker,
    write: W,
}

impl<W> FrameWrite<W> {
    /// Bind `write` to the direction spoken by `speaker`.
    pub fn new(speaker: Speaker, write: W) -> Self {
        Self { speaker, write }
    }

    /// Recover the transport writer without buffered frame state.
    #[cfg(test)]
    pub fn into_inner(self) -> W {
        self.write
    }
}

impl<W: AsyncWrite + Unpin> FrameWrite<W> {
    /// Validate, write, and flush one canonical frame.
    ///
    /// # Cancel safety
    ///
    /// Not cancel safe. A dropped `frame` future may already have written
    /// part of the frame, leaving the direction mid-frame for the peer's
    /// reader. Either retain the in-flight future across polls until it
    /// resolves, or write nothing further on this direction after a
    /// cancellation.
    pub async fn frame<T>(&mut self, wire: &WireFrame<T>) -> Result<(), EncodeError> {
        let (stream, frame) = wire;
        let result = async {
            let encoding = FrameEncoding::new(*stream, frame)?;
            write_encoding(&mut self.write, &encoding).await?;
            self.write.flush().await.map_err(EncodeErrorKind::Flush)
        }
        .await;
        result.map_err(|kind| EncodeError::new(self.speaker, *stream, kind))
    }
}

async fn write_encoding<T>(
    out: &mut (impl AsyncWrite + Unpin),
    encoding: &FrameEncoding<'_, T>,
) -> Result<(), EncodeErrorKind> {
    write(out, FramePart::Signal, &encoding.signal).await?;
    match &encoding.body {
        BodyEncoding::Empty => {}
        BodyEncoding::Query { count, children } => {
            write(out, FramePart::QueryCount, count).await?;
            for (radix, hash) in *children {
                write(out, FramePart::QueryChildren, std::slice::from_ref(radix)).await?;
                write(out, FramePart::QueryChildren, hash.as_bytes()).await?;
            }
        }
        BodyEncoding::Supply { header, run } => {
            write(out, FramePart::SupplyLength, header).await?;
            write(out, FramePart::SupplyRun, run.as_bytes()).await?;
        }
    }
    Ok(())
}

async fn write(
    out: &mut (impl AsyncWrite + Unpin),
    part: FramePart,
    bytes: &[u8],
) -> Result<(), EncodeErrorKind> {
    out.write_all(bytes)
        .await
        .map_err(|source| EncodeErrorKind::Write { part, source })
}
