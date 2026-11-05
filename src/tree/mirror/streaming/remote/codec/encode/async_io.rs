//! Direct asynchronous output for validated frame encodings.

use std::io::{self, IoSlice};
use std::mem;
use std::num::NonZeroUsize;
use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::io::{AsyncWrite, AsyncWriteExt};

use crate::observe::DataObservers;

use super::super::{
    error::{EncodeError, EncodeErrorKind, FramePart},
    frame::WireFrame,
    signal::{Speaker, Stream},
};
use super::{FrameEncoding, MAX_PIECES};

/// How many batches in a row must be flushed at their ceiling before the
/// next batch may grow past the segment size.
///
/// One cut shows only that a burst outgrew one segment, which a session
/// between nearly identical replicas meets now and then; the next batch
/// then keeps one-segment records, which the peer can decrypt as each
/// segment arrives, where it decrypts a larger record only once all of it
/// has arrived. A second cut in a row shows frames coming back to back for
/// longer than two segments take, the mark of a session readying frames
/// faster than one-segment writes carry them; there each small write only
/// delays the frames queued behind it, and costs a system call, a record
/// and the peer's receive processing.
pub const CUTS_TO_GROW: usize = 2;

/// Async frame writer over one speaker's direction of the pipe.
///
/// A frame is validated before any of it is written. Its pieces then go
/// directly to the transport as one vectored write, which a transport that
/// supports vectored writes (a TCP socket, or a `BufWriter`) takes in one
/// operation, so a frame is always whole on the transport before the next
/// begins. Writing does not flush: [`flush`](Self::flush) delivers every
/// frame written since the last flush, so frames written together can share
/// one flush on a buffered or encrypting writer. A writer given batch
/// ceilings ([`batched`](Self::batched)) keeps the frames between two
/// flushes within them, flushing by itself where the next frame would
/// exceed its batch's ceiling.
///
/// # Failure attribution
///
/// A transport failure is reported where it was detected: during a frame's
/// write, as a write failure naming that frame and the part being offered,
/// or at a flush, as a flush failure naming the last frame written. Over a
/// transport that buffers, such as the multiplexer's `BufWriter`, that is
/// not always where the failed bytes belong: most failures surface at a
/// flush, and a write that first writes out buffered bytes can fail on an
/// earlier frame's bytes yet name a later frame.
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
    /// The bytes of the frames written since the last flush.
    unflushed_bytes: usize,
    /// The ceiling of a batch whose first frame is at most this many bytes,
    /// after a predecessor that did not wait on the write half and fewer
    /// than [`CUTS_TO_GROW`] cuts in a row: the *segment size*.
    segment: usize,
    /// The ceiling of every other batch, or `None` for none: the *largest
    /// batch*.
    largest: Option<NonZeroUsize>,
    /// The ceiling of the batch being written, fixed by its first frame,
    /// or `None` for none.
    ceiling: Option<usize>,
    /// Whether a call on the write half has returned `Pending` since the
    /// last flush completed.
    waited: bool,
    /// Whether writing out the last flushed batch, its frames and its
    /// flush, waited on the write half.
    last_waited: bool,
    /// How many batches in a row, ending with the last that delivered
    /// frames, were flushed at their ceiling rather than because nothing
    /// more was ready, counted up to [`CUTS_TO_GROW`].
    cuts: usize,
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
            unflushed_bytes: 0,
            segment: 0,
            largest: None,
            ceiling: None,
            waited: false,
            last_waited: false,
            cuts: 0,
        }
    }

    /// Deliver every written frame to its logical stream's observer in
    /// `observe`, when the session is observed.
    pub fn observed(mut self, observe: Option<DataObservers>) -> Self {
        self.observe = observe;
        self
    }

    /// Keep the frames each flush carries within a ceiling chosen per
    /// batch, from `segment` and `largest`.
    ///
    /// A batch whose first frame is at most `segment` bytes has a ceiling
    /// of `segment` bytes, unless writing out its predecessor waited on the
    /// write half, or the [`CUTS_TO_GROW`] batches before it were each
    /// flushed at their ceiling, rather than because nothing more was
    /// ready. Any other batch has a ceiling of `largest` bytes, or none
    /// when it is `None`.
    ///
    /// [`frame`](Self::frame) flushes before a frame that would push the
    /// unflushed frames past their batch's ceiling, and once they reach
    /// it. Without this call, no batch has a ceiling.
    pub fn batched(mut self, segment: usize, largest: Option<NonZeroUsize>) -> Self {
        self.segment = segment;
        self.largest = largest;
        self
    }

    /// Whether a frame has been written since the last flush.
    pub fn is_unflushed(&self) -> bool {
        self.unflushed.is_some()
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
    /// Validate and write one canonical frame.
    ///
    /// Without a ceiling, this never flushes. With one, it first flushes
    /// the frames already written if this frame would push them past
    /// their batch's ceiling, and flushes after writing once they reach
    /// it, since no further frame fits beside them; so a frame larger than
    /// its batch's ceiling is flushed alone. A frame that starts a batch
    /// fixes the batch's ceiling ([`batched`](Self::batched)).
    ///
    /// # Errors
    ///
    /// A frame that does not encode fails before anything is written. A
    /// write the transport fails names this frame and the part being
    /// offered, as the type's docs describe, and a flush failure names the
    /// last frame written before it, as [`flush`](Self::flush)'s does.
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
        let speaker = self.speaker;
        let failed = |kind| EncodeError::new(speaker, *stream, kind);
        let encoding = FrameEncoding::new(*stream, frame).map_err(failed)?;
        let len = encoding.len();
        if self.unflushed_bytes > 0
            && self
                .ceiling
                .is_some_and(|ceiling| self.unflushed_bytes + len > ceiling)
        {
            self.flush_at_ceiling().await?;
        }
        if self.unflushed_bytes == 0 {
            // We start a batch. It keeps to the segment size only if this
            // frame fits a segment, the last batch was written out without
            // waiting, and fewer than `CUTS_TO_GROW` batches in a row filled
            // their ceiling: a larger frame spans segments however it is
            // written; while the write half is full, bytes queue anyway;
            // and once that many batches in a row filled their ceiling,
            // frames are coming back to back, and each one-segment write
            // would only delay the frames queued behind it.
            self.ceiling = if len <= self.segment && !self.last_waited && self.cuts < CUTS_TO_GROW {
                Some(self.segment)
            } else {
                self.largest.map(NonZeroUsize::get)
            };
        }
        let mut write = Watched {
            write: &mut self.write,
            waited: &mut self.waited,
        };
        write_encoding(&mut write, &encoding)
            .await
            .map_err(failed)?;
        self.unflushed = Some(*stream);
        self.unflushed_bytes += len;
        // The write half has taken the whole frame, into its buffer if it
        // has one: deliver its one-item view. The pieces are the bytes just
        // written, so the materialized buffer equals the wire by
        // construction.
        if let Some(observe) = &mut self.observe {
            observe.message(stream.index(), &encoding.to_vec());
        }
        if self
            .ceiling
            .is_some_and(|ceiling| self.unflushed_bytes >= ceiling)
        {
            self.flush_at_ceiling().await?;
        }
        Ok(())
    }

    /// Flush a batch that has reached its ceiling, counting it as one more
    /// batch in a row cut there, for the next batch's ceiling.
    ///
    /// The batch holds frames, so the flush delivers them.
    async fn flush_at_ceiling(&mut self) -> Result<(), EncodeError> {
        let cuts = self.cuts;
        self.flush().await?;
        self.cuts = (cuts + 1).min(CUTS_TO_GROW);
        Ok(())
    }

    /// Flush every frame written since the last flush, doing nothing when
    /// there is none.
    ///
    /// A failure names the last frame written: the failed flush did not
    /// deliver it, nor perhaps the frames written before it. A flush that
    /// completes records whether writing out the batch, its frames or the
    /// flush itself, waited on the write half, for the next batch's
    /// ceiling; a flush called from outside that delivers frames also
    /// records that their batch ended because nothing more was ready,
    /// which ends any run of batches cut at their ceiling. A flush with
    /// nothing to deliver records nothing, so a run of cuts continues
    /// across an instant at which no frame was ready.
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
        let mut write = Watched {
            write: &mut self.write,
            waited: &mut self.waited,
        };
        write.flush().await.map_err(|source| {
            EncodeError::new(self.speaker, stream, EncodeErrorKind::Flush(source))
        })?;
        self.unflushed = None;
        self.unflushed_bytes = 0;
        self.last_waited = mem::take(&mut self.waited);
        self.cuts = 0;
        Ok(())
    }
}

/// A borrowed write half that records whether any call on it returned
/// `Pending`, passing every call through unchanged.
///
/// A `Pending` write or flush means the write half could not take more
/// yet: a socket whose send buffer is full, or a peer reading slower than
/// this side writes. A Tokio socket also returns `Pending`, having woken
/// the task, once the task has spent its cooperative scheduling budget,
/// whatever room the socket has; that counts as a wait too, which only
/// lets the next batch grow to the largest batch.
struct Watched<'a, W> {
    /// The write half beneath.
    write: &'a mut W,
    /// Set once a call returns `Pending`; never cleared here.
    waited: &'a mut bool,
}

impl<W> Watched<'_, W> {
    /// Record whether `poll` waited, passing it through.
    fn note<T>(&mut self, poll: Poll<T>) -> Poll<T> {
        *self.waited |= poll.is_pending();
        poll
    }
}

/// Pass every call through, recording whether it waited.
impl<W: AsyncWrite + Unpin> AsyncWrite for Watched<'_, W> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        let poll = Pin::new(&mut *this.write).poll_write(cx, bytes);
        this.note(poll)
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        slices: &[IoSlice<'_>],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        let poll = Pin::new(&mut *this.write).poll_write_vectored(cx, slices);
        this.note(poll)
    }

    fn is_write_vectored(&self) -> bool {
        self.write.is_write_vectored()
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        let poll = Pin::new(&mut *this.write).poll_flush(cx);
        this.note(poll)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        let poll = Pin::new(&mut *this.write).poll_shutdown(cx);
        this.note(poll)
    }
}

/// Write an encoding's pieces in wire order as one vectored write,
/// continuing with the remainder whenever the transport accepts only part.
///
/// A failure names the piece holding the first byte `out` had not taken
/// when its write failed, which a buffering `out` may report for bytes
/// buffered ahead of the frame ([`FrameWrite`]'s docs).
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
