//! Direct asynchronous output for validated frame encodings.

use std::io::{self, IoSlice};

use tokio::io::{AsyncWrite, AsyncWriteExt};

use crate::observe::DataObservers;

use super::super::{
    error::{EncodeError, EncodeErrorKind, FramePart},
    frame::WireFrame,
    signal::{Speaker, Stream},
};
use super::{FrameEncoding, MAX_PIECES};

/// Async frame writer over one speaker's direction of the pipe.
///
/// A frame is validated before any of it is written. Its pieces then go
/// directly to the transport as one vectored write, which a transport that
/// supports vectored writes (a TCP socket, or a `BufWriter`) takes in one
/// operation, so a frame is always whole on the transport before the next
/// begins. Writing does not flush: [`flush`](Self::flush) delivers every
/// frame written since the last flush, so frames written together can share
/// one flush on a buffered or encrypting writer.
pub struct FrameWrite<W> {
    /// The local protocol role attributed to encoding failures.
    speaker: Speaker,
    /// The transport direction that receives encoded frames.
    write: W,
    /// The direction's per-logical-stream observers, if any: each written
    /// frame's contiguous bytes go to its stream's observer, at the cost of
    /// one branch when absent.
    observe: Option<DataObservers>,
    /// The logical stream of the last frame written since the last flush,
    /// if any: a flush failure is attributed to it.
    unflushed: Option<Stream>,
}

/// Constructs frame writers and recovers their transport.
impl<W> FrameWrite<W> {
    /// Bind `write` to the direction spoken by `speaker`.
    pub fn new(speaker: Speaker, write: W) -> Self {
        Self {
            speaker,
            write,
            observe: None,
            unflushed: None,
        }
    }

    /// Deliver every written frame to its logical stream's observer in
    /// `observe`, when the session is observed.
    pub fn observed(mut self, observe: Option<DataObservers>) -> Self {
        self.observe = observe;
        self
    }

    /// Recover the transport writer. The frame writer holds no frame
    /// state of its own: every frame is written whole before
    /// [`frame`](Self::frame) returns, so between frames the writer rests
    /// exactly at a frame boundary.
    #[cfg(test)]
    pub fn into_inner(self) -> W {
        self.write
    }
}

/// Writes complete frames to an asynchronous transport.
impl<W: AsyncWrite + Unpin> FrameWrite<W> {
    /// Validate and write one canonical frame, without flushing it.
    ///
    /// # Cancel safety
    ///
    /// Not cancel safe. A dropped `frame` future may already have written
    /// part of the frame, leaving the direction mid-frame for the peer's
    /// reader. Either retain the in-flight future across polls until it
    /// resolves, or write nothing further on this direction after a
    /// cancellation.
    pub async fn frame(&mut self, wire: &WireFrame) -> Result<(), EncodeError> {
        let (stream, frame) = wire;
        let result = async {
            let encoding = FrameEncoding::new(*stream, frame)?;
            write_encoding(&mut self.write, &encoding).await?;
            self.unflushed = Some(*stream);
            // The transport has the frame: deliver its one-item view. The
            // pieces are the bytes just written, so the materialized
            // buffer equals the wire by construction.
            if let Some(observe) = &mut self.observe {
                observe.message(stream.index(), &encoding.to_vec());
            }
            Ok(())
        }
        .await;
        result.map_err(|kind| EncodeError::new(self.speaker, *stream, kind))
    }

    /// Flush every frame written since the last flush, doing nothing when
    /// there is none.
    ///
    /// A failure names the last frame written: the failed flush did not
    /// deliver it, nor perhaps the frames written before it.
    ///
    /// # Cancel safety
    ///
    /// Cancel safe when the transport's flush is: a dropped flush leaves
    /// the written frames recorded as unflushed, so the next call flushes
    /// them.
    pub async fn flush(&mut self) -> Result<(), EncodeError> {
        let Some(stream) = self.unflushed else {
            return Ok(());
        };
        self.write.flush().await.map_err(|source| {
            EncodeError::new(self.speaker, stream, EncodeErrorKind::Flush(source))
        })?;
        self.unflushed = None;
        Ok(())
    }
}

/// Write an encoding's pieces in wire order as one vectored write,
/// continuing with the remainder whenever the transport accepts only part.
///
/// A failure names the piece holding the first byte the transport did not
/// accept.
async fn write_encoding(
    out: &mut (impl AsyncWrite + Unpin),
    encoding: &FrameEncoding<'_>,
) -> Result<(), EncodeErrorKind> {
    let mut parts = [(FramePart::FrameHead, 0); MAX_PIECES];
    let mut slices = [IoSlice::new(&[]); MAX_PIECES];
    let mut count = 0;
    for (part, piece) in encoding.pieces() {
        parts[count] = (part, piece.len());
        slices[count] = IoSlice::new(piece);
        count += 1;
    }
    let parts = &parts[..count];
    let mut remaining = &mut slices[..count];
    let mut written = 0;
    while !remaining.is_empty() {
        let part = part_at(parts, written);
        let accepted = out
            .write_vectored(remaining)
            .await
            .map_err(|source| EncodeErrorKind::Write { part, source })?;
        if accepted == 0 {
            return Err(EncodeErrorKind::Write {
                part,
                source: io::ErrorKind::WriteZero.into(),
            });
        }
        written += accepted;
        IoSlice::advance_slices(&mut remaining, accepted);
    }
    Ok(())
}

/// The part holding byte `offset` of a frame whose pieces, in wire order,
/// carry `parts` with the given lengths.
fn part_at(parts: &[(FramePart, usize)], offset: usize) -> FramePart {
    let mut end = 0;
    for &(part, len) in parts {
        end += len;
        if offset < end {
            return part;
        }
    }
    unreachable!("a frame with unwritten bytes has a piece holding the first of them")
}
