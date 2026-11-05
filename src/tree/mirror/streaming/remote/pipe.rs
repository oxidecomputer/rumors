//! One duplex byte stream carrying every logical stream of a session.
//!
//! A session's replies travel on *logical streams*, one per level a speaker
//! answers at ([`Stream`]). Each frame names its logical stream in its
//! signal, so one byte stream per direction carries all of them, their
//! frames interleaved. Two tasks connect the logical streams to that pipe:
//!
//! - The [`Multiplexer`] owns the write half. Each local encoder hands it
//!   frames through its own capacity-1 channel ([`Outgoing`]). It writes
//!   whole frames, choosing a waiting frame that is not a supply run ahead
//!   of any waiting supply run ([`Class`]), flushes whenever no frame is
//!   ready, and returns when every encoder's channel has closed.
//! - The [`Demultiplexer`] owns the read half. It reads frames in pipe order
//!   and routes each, by the logical stream its signal names, to that
//!   stream's decoder over a capacity-1 channel. It stops at the first item
//!   that is not a frame: the peer's closing items (its completion marker,
//!   or a donated identity) follow its last frame on the same pipe.
//!
//! # Why sharing one pipe cannot deadlock
//!
//! The walk is deadlock-free if a full queue stalls only its own producer,
//! never delivery on another queue. One pipe threatens that premise: the
//! demultiplexer delivers frames in pipe order, so if delivering the head
//! frame waited until that frame's stage accepted it, the reader could
//! never reach a later frame that would let the stage accept it. The
//! receive path therefore never waits on the walk. Here is every wait on
//! it:
//!
//! | Waiter | Waits for | Why the wait ends without the walk |
//! | --- | --- | --- |
//! | Demultiplexer | the pipe | The peer writes independently. |
//! | Demultiplexer | its stream's decoder to accept a frame | The decoder is only finishing its current frame. |
//! | Decoder | the local record of the question its next reply answers | The encoder publishes it right after handing the carrying reply's last frame to the multiplexer, without waiting for the write. |
//! | Decoder | the storage backend, absorbing supplied leaves | Backend operations complete without waiting on other session work. |
//! | Decoder | its assembler, finishing a group of supplies | The assembler waits only on the decoder's leaves and the backend, and the decoder takes the group's nodes back at the group's end, with room for a fan of them. |
//! | Decoder | room in parking | Never: parking is unbounded, and every parked chunk lies within the room its question set aside. |
//!
//! A decoder waits for a local record only when that queue is empty. If a
//! reply to one of those questions has arrived, the encoder is already past
//! the carrying reply's last frame, and with the queue empty its
//! publication cannot block, so the decoder waits only for its own side's
//! encoder to be scheduled. A decoder hands on every complete question
//! before it waits for the next frame, so a peer that sends the rest of a
//! reply only once those questions are answered is never kept waiting by
//! this side's receive path.
//!
//! On the sending side, encoders wait for the multiplexer and the
//! multiplexer waits for the pipe, but no receiving stage's consumption
//! order can stop the pipe from draining, because the peer's receive path
//! never waits on its walk. So no cycle can pass through the pipe.
//!
//! # Lazy streams
//!
//! A logical stream carries answers to questions its receiver asked, and
//! both sides know whether any such question exists at a level before the
//! stream carries anything. An encoder that sends no frame therefore sends
//! no end-of-stream control either, and a decoder that received no question
//! never waits for one. Every other logical stream ends with an explicit
//! end-of-stream frame, which the demultiplexer consumes by closing the
//! stream's channel.
//!
//! # Conformance
//!
//! Every frame that answers no outstanding question fails the session. A frame
//! on a logical stream that has ended, or that has no decoder, fails at the
//! demultiplexer: nothing accepts it. So does an end-of-stream frame on a
//! logical stream that carried no frame, since only a started stream ends
//! explicitly. A decoder exists for every level the local side may ask at,
//! before it knows whether it will, so a frame can also reach the channel of a
//! decoder that never receives a question. That frame waits until the decoder's
//! question queue closes; the decoder then closes its channel, so a later frame
//! fails at the demultiplexer, and reports the waiting frame itself. Like every
//! check against a non-conforming peer, these detect conformance bugs; they do
//! not bound how long such a peer can make the session wait.

use std::future::poll_fn;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::StreamExt;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use crate::observe::{Direction, SessionHandle};
use crate::tree::mirror::cbor::MAJOR_ARRAY;
use crate::tree::mirror::streaming::{
    channel::{QueueKind, QueueRole, Receiver, Sender, channel},
    stats::{CountedRead, CountedWrite, Recorder},
    tasks::cancelled,
};

use super::codec::{
    DecodeError, DecodeErrorKind, EncodeError, End, Frame, FrameRead, FrameWrite, Origin, Reaction,
    ReplyFrame, RunBudget, Speaker, Stream,
};

/// Frames each logical stream's channel holds between the pipe and a stage's
/// encoder or decoder.
///
/// One slot is the smallest channel. Beyond it, the multiplexer holds at most
/// one frame per stream, staged or being written, and the demultiplexer at most
/// one frame for the whole direction, while it waits for a full channel.
const LANE_CAPACITY: usize = 1;

/// The bits of a CBOR initial byte that hold its major type.
const MAJOR_TYPE_SHIFT: u32 = 5;

/// Whether `byte` opens a frame: every frame is one CBOR array.
///
/// The session's closing items open with other major types (the
/// completion marker with a text head, a donated identity with a tag
/// head), so the first byte of each item tells a frame from what follows
/// the last one.
fn opens_frame(byte: u8) -> bool {
    byte >> MAJOR_TYPE_SHIFT == MAJOR_ARRAY
}

/// The pipe's read half, holding at most one byte read ahead of the next
/// item.
///
/// The demultiplexer reads the first byte of each item alone, to learn
/// whether a frame follows, and keeps it here; the frame reader or the
/// closing exchange then reads it first. Reading one byte is
/// cancellation-safe, so the demultiplexer can be dropped between frames
/// without losing the closing exchange's first byte. A read error the
/// demultiplexer met there can be kept the same way
/// ([`keep_failure`](Self::keep_failure)), for the closing exchange to
/// report.
pub struct PipeRead<R> {
    /// The transport's read half.
    read: R,
    /// The byte read ahead of the next item, if any.
    pending: Option<u8>,
    /// Whether the demultiplexer has begun a frame it has not yet handed
    /// to its decoder.
    in_frame: bool,
    /// Whether the transport reported end of stream where the next item
    /// would begin.
    ended: bool,
    /// A read error met ahead of the next item and not yet reported to the
    /// reader of that item.
    failure: Option<io::Error>,
}

/// Summarize the read half's lookahead without requiring the transport to
/// be debuggable.
impl<R> std::fmt::Debug for PipeRead<R> {
    /// Formats the lookahead, whether a frame is under way, and how the
    /// transport last ended a read.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PipeRead")
            .field("pending", &self.pending)
            .field("in_frame", &self.in_frame)
            .field("ended", &self.ended)
            .field("failure", &self.failure)
            .finish_non_exhaustive()
    }
}

impl<R> PipeRead<R> {
    /// Wrap a read half resting at an item boundary.
    pub fn new(read: R) -> Self {
        Self {
            read,
            pending: None,
            in_frame: false,
            ended: false,
            failure: None,
        }
    }

    /// Whether every frame the demultiplexer began reached its decoder.
    fn at_frame_boundary(&self) -> bool {
        !self.in_frame
    }

    /// Report `error`, which a read of the transport returned where the
    /// next item would begin, from the next read in place of the
    /// transport.
    ///
    /// A transport need not repeat an error: one that reports a reset once
    /// may report end of stream when read again, so the next item's reader
    /// would otherwise see a different failure. End of stream needs no
    /// keeping, since reading again finds it again.
    pub fn keep_failure(&mut self, error: io::Error) {
        if !self.ended {
            self.failure = Some(error);
        }
    }
}

impl<R: AsyncRead + Unpin> PipeRead<R> {
    /// Return the first byte of the next item without consuming it, or
    /// `None` at end of stream.
    ///
    /// # Cancel safety
    ///
    /// Cancel safe: a byte is either delivered into the lookahead within
    /// one poll or not read at all.
    pub async fn peek(&mut self) -> io::Result<Option<u8>> {
        if let Some(byte) = self.pending {
            return Ok(Some(byte));
        }
        if let Some(error) = self.failure.take() {
            return Err(error);
        }
        poll_fn(|cx| {
            let mut byte = [0u8; 1];
            let mut buf = ReadBuf::new(&mut byte);
            match Pin::new(&mut self.read).poll_read(cx, &mut buf) {
                Poll::Ready(Ok(())) if buf.filled().is_empty() => {
                    self.ended = true;
                    Poll::Ready(Ok(None))
                }
                Poll::Ready(Ok(())) => {
                    self.pending = Some(byte[0]);
                    Poll::Ready(Ok(Some(byte[0])))
                }
                Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
                Poll::Pending => Poll::Pending,
            }
        })
        .await
    }
}

/// Read the lookahead byte or a kept read error first, then the transport.
impl<R: AsyncRead + Unpin> AsyncRead for PipeRead<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        if buf.remaining() > 0
            && let Some(byte) = this.pending.take()
        {
            buf.put_slice(&[byte]);
            return Poll::Ready(Ok(()));
        }
        if let Some(error) = this.failure.take() {
            return Poll::Ready(Err(error));
        }
        Pin::new(&mut this.read).poll_read(cx, buf)
    }
}

/// A failure of the pipe itself, rather than of one reply's content.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PipeError {
    /// The pipe ended or failed at a frame boundary while reconciliation
    /// still needed the peer.
    #[error("peer departed during reconciliation")]
    Departed(#[source] io::Error),
    /// An incoming frame could not be read or decoded.
    #[error(transparent)]
    Decode(#[from] DecodeError),
    /// An outgoing frame could not be written or flushed.
    #[error(transparent)]
    Encode(#[from] EncodeError),
    /// A frame arrived on a logical stream that answers no outstanding
    /// question: one that already ended, or one the local side never asked
    /// on.
    #[error("{origin}: frame on a logical stream that answers no question")]
    Unasked {
        /// The direction, and the logical stream when its frame was read.
        origin: Origin,
    },
    /// The peer's closing items began while a logical stream it had started
    /// was still open.
    #[error("{origin}: closing items began before this logical stream ended")]
    Unfinished {
        /// The logical stream left open.
        origin: Origin,
    },
}

impl PipeError {
    /// Whether this failure shows the peer breaking the protocol, rather
    /// than the transport failing or ending.
    pub fn is_violation(&self) -> bool {
        match self {
            Self::Departed(_) | Self::Encode(_) => false,
            Self::Decode(error) => !matches!(
                error.kind,
                DecodeErrorKind::Read { .. } | DecodeErrorKind::Truncated { .. }
            ),
            Self::Unasked { .. } | Self::Unfinished { .. } => true,
        }
    }
}

/// One local encoder's logical stream into the multiplexer.
pub struct Outgoing {
    /// The multiplexer's channel for this logical stream.
    frames: Sender<Frame>,
    /// Whether any frame was sent, so the stream must end explicitly.
    opened: bool,
}

impl Outgoing {
    /// Hand one reply frame to the multiplexer.
    ///
    /// Returns once the frame is in the multiplexer's channel, before it
    /// reaches the pipe, so the encoder can publish the local records of
    /// the reply's questions without waiting for the write. A multiplexer
    /// that has gone failed; the session's executor reports its error, so
    /// this awaits cancellation instead.
    pub async fn frame(&mut self, frame: ReplyFrame) {
        self.opened = true;
        send_or_cancel(&mut self.frames, frame.into()).await;
    }

    /// End this logical stream after its last reply, if it carried any.
    pub async fn finish(mut self) {
        if self.opened {
            send_or_cancel(&mut self.frames, Frame::End(End::Stream)).await;
        }
    }
}

/// Send one frame to the pipe's other side, or await cancellation if it has
/// gone: its failure is the session's error.
async fn send_or_cancel(frames: &mut Sender<Frame>, frame: Frame) {
    if frames.send(frame).await.is_err() {
        cancelled().await;
    }
}

/// The multiplexer's priority classes, served in declaration order.
///
/// Thin frames (matches, queries, reply ends) carry the descent, which finds
/// what differs; supply runs carry the bulk, which transfers it. Each time the
/// multiplexer chooses its next frame, it takes a thin frame staged on any
/// logical stream ahead of every staged supply run, so the descent never queues
/// behind bulk waiting on another logical stream. Within one logical stream,
/// frames keep their order: a thin frame still waits behind the frame being
/// written, and behind up to two earlier supply runs on its own stream, one
/// staged or being written and one in its channel. A session's traffic is
/// finite, so every frame is still served. Within a class, logical streams are
/// served round-robin from that class's own cursor; a further class takes its
/// place in the order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Class {
    /// Every frame that is not a supply run.
    Descent,
    /// Supply runs.
    Bulk,
}

impl Class {
    /// Every class, in service order, which is declaration order: each
    /// class's [`rank`](Self::rank) is its discriminant.
    const ORDER: [Class; 2] = {
        let order = [Class::Descent, Class::Bulk];
        let mut index = 0;
        while index < order.len() {
            assert!(
                order[index] as usize == index,
                "service order is declaration order"
            );
            index += 1;
        }
        order
    };

    /// The class a frame is served in.
    fn of(frame: &Frame) -> Self {
        match frame {
            Frame::Reaction(Reaction::Supply(_), _) => Class::Bulk,
            Frame::Reaction(..) | Frame::End(_) => Class::Descent,
        }
    }

    /// This class's position in [`ORDER`](Self::ORDER).
    fn rank(self) -> usize {
        self as usize
    }
}

/// One local logical stream's channel and the frame taken from it.
struct OutgoingLane {
    /// The logical stream every frame from this lane names.
    stream: Stream,
    /// The encoder's channel, until it closes.
    frames: Option<Receiver<Frame>>,
    /// The next frame, taken from the channel and awaiting its turn.
    staged: Option<Frame>,
}

/// Writes every local logical stream's frames to the pipe's write half.
#[derive(Default)]
pub struct Multiplexer {
    /// One lane per logical stream an encoder was created for.
    lanes: Vec<OutgoingLane>,
}

impl Multiplexer {
    /// Create the channel for one local logical stream, labeled at the
    /// height of the replies it carries.
    pub fn lane(&mut self, stream: Stream, height: usize) -> Outgoing {
        let (frames, receiver) = channel(
            QueueRole::new(QueueKind::PipeOutgoing, height),
            LANE_CAPACITY,
        );
        self.lanes.push(OutgoingLane {
            stream,
            frames: Some(receiver),
            staged: None,
        });
        Outgoing {
            frames,
            opened: false,
        }
    }

    /// Write frames until every lane's channel has closed.
    ///
    /// Each frame is written whole before the next is chosen. Writing does
    /// not flush: the write half is flushed whenever no frame is ready,
    /// before waiting for one, and before returning. Frames ready together
    /// therefore share a flush, and the multiplexer never waits while a
    /// frame it wrote sits unflushed, so every frame handed to it reaches
    /// the peer without waiting on anything but the pipe. The write half is
    /// borrowed, and rests flushed at a frame boundary on success, ready
    /// for the closing exchange.
    pub async fn run<W: AsyncWrite + Unpin>(
        mut self,
        write: &mut W,
        speaker: Speaker,
        stats: Recorder,
        observe: &SessionHandle,
    ) -> Result<(), EncodeError> {
        let mut out = FrameWrite::new(speaker, CountedWrite::new(write, stats.clone()))
            .observed(observe.data(speaker.role(), Direction::Sent));
        let mut cursors = [0; Class::ORDER.len()];
        loop {
            let ready = poll_fn(|cx| Poll::Ready(self.poll_next(cx, &mut cursors))).await;
            let next = match ready {
                Poll::Ready(next) => next,
                Poll::Pending => {
                    out.flush().await?;
                    poll_fn(|cx| self.poll_next(cx, &mut cursors)).await
                }
            };
            let Some((stream, frame)) = next else {
                break;
            };
            out.frame(&(stream, frame)).await?;
            stats.frame_sent();
        }
        out.flush().await
    }

    /// Take every frame that is ready, then remove and return the next one
    /// to write: `Ready(None)` once every lane has closed with nothing
    /// staged, and `Pending`, with a wakeup registered, while open lanes
    /// have nothing staged.
    fn poll_next(
        &mut self,
        cx: &mut Context<'_>,
        cursors: &mut [usize; Class::ORDER.len()],
    ) -> Poll<Option<(Stream, Frame)>> {
        let mut open = false;
        for lane in &mut self.lanes {
            if lane.staged.is_none()
                && let Some(frames) = &mut lane.frames
            {
                match frames.poll_next_unpin(cx) {
                    Poll::Ready(Some(frame)) => lane.staged = Some(frame),
                    Poll::Ready(None) => lane.frames = None,
                    Poll::Pending => {}
                }
            }
            open |= lane.staged.is_some() || lane.frames.is_some();
        }
        match self.choose(cursors) {
            Some(index) => {
                let lane = &mut self.lanes[index];
                let frame = lane.staged.take().expect("choose returns a staged lane");
                Poll::Ready(Some((lane.stream, frame)))
            }
            None if open => Poll::Pending,
            None => Poll::Ready(None),
        }
    }

    /// The lane whose staged frame goes next: the first class with a staged
    /// frame, round-robin among that class's lanes from its cursor.
    fn choose(&self, cursors: &mut [usize; Class::ORDER.len()]) -> Option<usize> {
        let count = self.lanes.len();
        for class in Class::ORDER {
            let cursor = &mut cursors[class.rank()];
            let found = (0..count)
                .map(|offset| (*cursor + offset) % count)
                .find(|&index| {
                    self.lanes[index]
                        .staged
                        .as_ref()
                        .is_some_and(|frame| Class::of(frame) == class)
                });
            if let Some(index) = found {
                *cursor = index + 1;
                return Some(index);
            }
        }
        None
    }
}

/// One remote logical stream's route to its decoder.
enum IncomingLane {
    /// No decoder was created for this stream: the local side asks no
    /// question its frames could answer.
    Unclaimed,
    /// The decoder's channel, and whether a frame has been routed to it.
    Open {
        /// The logical stream this lane routes.
        stream: Stream,
        /// The decoder's channel.
        frames: Sender<Frame>,
        /// Whether any frame arrived, so the stream must end before the
        /// peer's closing items.
        started: bool,
    },
    /// The stream's end-of-stream frame arrived and its channel closed.
    Ended,
}

/// Reads the peer's frames from the pipe's read half and routes each to its
/// logical stream's decoder.
pub struct Demultiplexer {
    /// One route per remote logical stream, indexed by stream index.
    lanes: Vec<IncomingLane>,
}

impl Default for Demultiplexer {
    /// Every remote logical stream unclaimed.
    fn default() -> Self {
        Self {
            lanes: (0..Stream::COUNT)
                .map(|_| IncomingLane::Unclaimed)
                .collect(),
        }
    }
}

impl Demultiplexer {
    /// Create the channel to the decoder of one remote logical stream,
    /// labeled at the height of the replies it carries.
    ///
    /// The receiver yields the stream's frames and ends when its
    /// end-of-stream frame arrives.
    pub fn lane(&mut self, stream: Stream, height: usize) -> Receiver<Frame> {
        let (frames, receiver) = channel(
            QueueRole::new(QueueKind::PipeIncoming, height),
            LANE_CAPACITY,
        );
        self.lanes[usize::from(stream.index())] = IncomingLane::Open {
            stream,
            frames,
            started: false,
        };
        receiver
    }

    /// Route frames until the peer's first closing item, then wait to be
    /// dropped; return only on failure.
    ///
    /// The read half is borrowed. Whenever this future is dropped between
    /// frames, the read half rests at an item boundary, with the next
    /// item's first byte possibly in its lookahead.
    pub async fn run<R: AsyncRead + Unpin>(
        mut self,
        read: &mut PipeRead<R>,
        speaker: Speaker,
        budget: RunBudget,
        stats: Recorder,
        observe: &SessionHandle,
    ) -> PipeError {
        let mut frames = FrameRead::new(speaker, budget, CountedRead::new(read, stats.clone()))
            .observed(observe.data(speaker.role(), Direction::Received));
        loop {
            let read = frames.get_mut().get_mut();
            match read.peek().await {
                Ok(Some(byte)) if opens_frame(byte) => read.in_frame = true,
                Ok(Some(_)) => return self.stop(speaker).await,
                Ok(None) => {
                    return PipeError::Departed(io::ErrorKind::UnexpectedEof.into());
                }
                Err(source) => return PipeError::Departed(source),
            }
            let (stream, frame) = match frames.frame().await {
                Ok(Some(frame)) => frame,
                // The peeked byte is the frame's first, so the reader
                // cannot find a clean close before it.
                Ok(None) => return PipeError::Departed(io::ErrorKind::UnexpectedEof.into()),
                Err(error) => return PipeError::Decode(error),
            };
            stats.frame_received();
            if let Err(error) = self.route(speaker, stream, frame).await {
                return error;
            }
            frames.get_mut().get_mut().in_frame = false;
        }
    }

    /// Hand one frame to its logical stream's decoder, or close the stream
    /// at its end-of-stream frame.
    ///
    /// An end-of-stream frame on a stream that carried no frame answers no
    /// question: a stream is lazy, so a conforming peer ends only a stream
    /// it started.
    async fn route(
        &mut self,
        speaker: Speaker,
        stream: Stream,
        frame: Frame,
    ) -> Result<(), PipeError> {
        let unasked = || PipeError::Unasked {
            origin: Origin::stream(speaker, stream),
        };
        let lane = &mut self.lanes[usize::from(stream.index())];
        let IncomingLane::Open {
            frames, started, ..
        } = lane
        else {
            return Err(unasked());
        };
        if matches!(frame, Frame::End(End::Stream)) {
            if !*started {
                return Err(unasked());
            }
            *lane = IncomingLane::Ended;
            return Ok(());
        }
        *started = true;
        frames.send(frame).await.map_err(|_| unasked())
    }

    /// Stop at the peer's first closing item.
    ///
    /// The peer sends closing items only after its last frame, so every
    /// stream it started must have ended. Streams it never started close
    /// here: a decoder that still expected a reply finds its stream
    /// truncated.
    async fn stop(self, speaker: Speaker) -> PipeError {
        for lane in &self.lanes {
            if let IncomingLane::Open {
                stream,
                started: true,
                ..
            } = lane
            {
                return PipeError::Unfinished {
                    origin: Origin::stream(speaker, *stream),
                };
            }
        }
        drop(self);
        cancelled().await
    }
}

/// Check, once the protocol has completed, that the demultiplexer holds no
/// frame it has begun: such a frame answers no outstanding question.
pub fn check_drained<R>(read: &PipeRead<R>, speaker: Speaker) -> Result<(), PipeError> {
    if read.at_frame_boundary() {
        Ok(())
    } else {
        Err(PipeError::Unasked {
            origin: Origin::direction(speaker),
        })
    }
}

#[cfg(test)]
mod tests;
