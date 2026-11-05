//! One duplex byte stream carrying every logical stream of a session.
//!
//! A session's replies travel on *logical streams*, one per level a speaker
//! answers at ([`Stream`]). Each frame names its logical stream in its
//! signal, so one byte stream per direction carries all of them, their
//! frames interleaved. Two futures connect the logical streams to that pipe:
//!
//! - The [`Multiplexer`] owns the write half. Each local encoder hands it
//!   frames through its own channel of [`LANE_CAPACITY`] frames
//!   ([`Outgoing`]). It writes whole frames, choosing a waiting frame that
//!   is not a supply run ahead of any waiting supply run ([`Class`]). It
//!   keeps the frames each flush carries within a ceiling its
//!   [`WriteBatch`] sets, and flushes once a poll finds no frame ready
//!   after it has let the session's other woken work run
//!   ([`Multiplexer::run`]). It returns when every encoder's channel has
//!   closed.
//! - The [`Demultiplexer`] owns the read half. It reads frames in pipe order
//!   and routes each, by the logical stream its signal names, to that
//!   stream's decoder over a channel of [`LANE_CAPACITY`] frames. It stops
//!   at the first item that is not a frame: the peer's closing items (its
//!   completion marker, or a donated identity) follow its last frame on the
//!   same pipe.
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
//! | Demultiplexer | the pipe | The peer writes and flushes independently. |
//! | Demultiplexer | its stream's decoder to accept a frame | The decoder is only finishing its current frame. |
//! | Decoder | the local record of the question its next reply answers | The encoder publishes it right after handing the frame carrying the question to the multiplexer, without waiting for the write, into an unbounded queue. |
//! | Decoder | the storage backend, absorbing supplied leaves | Backend operations complete without waiting on other session work. |
//! | Decoder | its assembler, finishing a group of supplies | The assembler waits only on the decoder's leaves and the backend, and the decoder takes the group's nodes back at the group's end, with room for a fan of them. |
//! | Decoder | room in parking | Never: parking is unbounded, and every parked chunk lies within the room its question set aside. |
//!
//! A decoder waits for a local record only when that queue is empty. If a
//! reply to one of those questions has arrived, the encoder is already past
//! the frame carrying the question, and its publication cannot block, so
//! the decoder waits only for its own side's encoder to be scheduled. A
//! decoder hands on every complete question before it waits for the next
//! frame, so a peer that sends the rest of a reply only once those
//! questions are answered is never kept waiting by this side's receive
//! path.
//!
//! The session polls the demultiplexer, the multiplexer and the walk's
//! tasks as members of one task set (a
//! [`FuturesUnordered`](futures::stream::FuturesUnordered)), not as tasks
//! of their own. That changes when a member is polled, never what it waits
//! for. The set polls a member once it has woken, in the order members
//! woke; it yields after a bounded number of member polls, waking the
//! session as it does, so a woken member is polled within a bounded number
//! of the session's polls. Each wait in the table therefore ends when what
//! it waits for wakes it, whatever the walk's members are doing, provided
//! only that every poll returns.
//!
//! The multiplexer never waits while a frame it wrote sits in its buffer or
//! unflushed. Finding no frame ready, it may first *yield*: wake itself and
//! return `Pending`, so that the set polls the members already woken
//! before polling it again. A yield is not a wait, since the set polls the
//! multiplexer again within a bounded number of the session's polls,
//! whatever else happens. [`EMPTY_PASSES`] yields in a row that bring no
//! new frame are followed by a flush, and a yield that brings a frame
//! grows the batch by at least a byte, so within [`EMPTY_PASSES`] times as
//! many yields as the batch's ceiling has bytes, and [`EMPTY_PASSES`] more
//! (the ceiling being the segment size for a batch that keeps one-segment
//! records, the largest batch for any other, as [`WriteBatch`] sets them),
//! the batch is flushed, before the multiplexer waits. So a peer that
//! wrote a frame flushes it within a bounded number of its session's
//! polls, without waiting on its walk: that is what "independently" means
//! in the table.
//! A peer whose largest batch is unbounded keeps that bound only for
//! batches within its segment size; any other batch it flushes after the
//! first run of [`EMPTY_PASSES`] passes that bring no frame, which comes
//! without waiting on this side, since without new frames from here its
//! walk readies only finitely many frames.
//!
//! On the sending side, encoders wait for the multiplexer and the
//! multiplexer waits for the pipe, but no receiving stage's consumption
//! order can stop the pipe from draining, because the peer's receive path
//! never waits on its walk. So no cycle can pass through the pipe. A
//! chunk's charge rides its last frame ([`Outbound`]) and returns once the
//! multiplexer has written that frame, so it too returns without waiting
//! on the walk. The
//! walk's own waits, for admission of the work it builds, end by the
//! induction in the ledger's admission module
//! ([`ledger`](super::super::ledger)), which rests on this one: every
//! question a side sends reaches the peer's stage, whatever that side's
//! walk is waiting for.
//!
//! # Covered matches
//!
//! A reply's first match names every later listed child that also matches,
//! and those children take no frame. The argument above holds for it
//! premise by premise:
//!
//! - A reply sends a frame per reaction it holds, and it holds no reaction
//!   for a covered child, so it sends at most one frame per child it reacts
//!   to, and never more frames than it would with a frame per match. Every
//!   lane and channel therefore holds a subset of the frames it would hold
//!   then, under the same capacities.
//! - The demultiplexer still holds at most one frame for the whole
//!   direction, while it waits for a full channel.
//! - The decoder infers nothing. It checks the covered radixes against the
//!   question's record at the match's frame and parks the match with them;
//!   the covered children are resolved by whoever consumes the reply, the
//!   take as it derives the reply's questions and the walk's resolver as it
//!   pairs reactions with its children, each as it reaches them. No queue
//!   fills with inferred matches.
//! - The decoder's one new step, telling a covered position from one a
//!   frame must take, waits for nothing: an uncovered position's frame is
//!   one it waits for anyway, and a covered position needs none. At the
//!   reply's end it checks that every position left is covered, which
//!   reads only the record.
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
use std::mem;
use std::num::NonZeroUsize;
use std::ops::{Bound, RangeBounds};
use std::pin::Pin;
#[cfg(any(test, feature = "test-internals"))]
use std::sync::OnceLock;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::task::{Context, Poll, Wake, Waker};

use futures::StreamExt;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncWrite, BufWriter, ReadBuf};

use crate::default;
use crate::observe::{Direction, SessionHandle};
use crate::tree::mirror::cbor::MAJOR_ARRAY;
use crate::tree::mirror::streaming::{
    FAN,
    channel::{QueueKind, QueueRole, Receiver, Sender, channel},
    ledger::Charge,
    stats::{CountedRead, CountedWrite, Recorder},
    tasks::cancelled,
};

use super::codec::{
    DecodeError, DecodeErrorKind, EncodeError, End, FULL_FAN_QUERY_FRAME_LEN, Frame, FrameRead,
    FrameWrite, Origin, Reaction, ReplyFrame, RunBudget, Speaker, Stream,
};

/// Frames each logical stream's channel holds between the pipe and a
/// stage's encoder or decoder: a fan, [`FAN`].
///
/// The capacity follows from what one input makes an encoder produce. An
/// encoder takes one chunk of a reply at a time, and a chunk holds at most
/// a fan of reactions. Each reaction ends in exactly one frame: a match, a
/// query, or a supply's last run; an empty reply's one chunk is one bare
/// end. A supply whose leaves fill more than one run hands over its
/// earlier runs as they fill, so only the frames closing a chunk's
/// reactions are bounded per chunk, at a fan. With that many slots, an
/// encoder hands over every closing frame of a chunk into an empty lane
/// without waiting on the multiplexer, and the demultiplexer delivers a peer's chunk of thin
/// frames without waiting on the decoder. A smaller lane would finish a
/// chunk over several polls, each pushing a few frames toward the
/// multiplexer, which writes them in as many small batches; a deep lane
/// also lets the encoders keep producing while the multiplexer waits on a
/// full write half, so a backed-up pipe takes larger writes.
///
/// Nothing in this module's progress argument needs more than one slot.
/// A channel allocates a node for each frame it holds, never for a slot it
/// leaves empty, so the capacity reserves no memory by itself. Beyond the
/// lane, the multiplexer holds at most one frame per stream, staged or
/// being written, and copies the frames smaller than
/// [`WRITE_BUFFER_BYTES`] into a buffer of that size; the demultiplexer
/// holds at most one frame for the whole direction, while it waits for a
/// full channel.
pub(crate) const LANE_CAPACITY: usize = FAN;

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

/// The pipe's read half: the session's buffered read half, and what the
/// demultiplexer has learned at the next item's boundary.
///
/// The demultiplexer starts each item by looking at its first byte in the
/// read half's buffer ([`peek`](Self::peek)) without consuming it, to learn
/// whether a frame follows. The frame reader and the closing exchange then
/// consume exactly the bytes of each item they read, so the pipe never
/// consumes a byte that is not the session's: whatever follows the peer's
/// closing items stays in the read half's buffer for the application.
///
/// A peek either finds the next byte in the buffer or consumes nothing, so
/// the demultiplexer can be dropped between frames without losing a byte
/// the closing exchange needs. A read error the demultiplexer met there can
/// be kept ([`keep_failure`](Self::keep_failure)), for the closing exchange
/// to report.
pub struct PipeRead<R> {
    /// The transport's buffered read half.
    read: R,
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

/// Summarize the read half's state without requiring the transport to be
/// debuggable.
impl<R> std::fmt::Debug for PipeRead<R> {
    /// Formats whether a frame is under way and how the transport last
    /// ended a read.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PipeRead")
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
            in_frame: false,
            ended: false,
            failure: None,
        }
    }

    /// Recover the transport's read half, holding every byte the pipe has
    /// not consumed.
    #[cfg(test)]
    pub fn into_inner(self) -> R {
        self.read
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

impl<R: AsyncBufRead + Unpin> PipeRead<R> {
    /// Return the first byte of the next item without consuming it, or
    /// `None` at end of stream.
    ///
    /// # Cancel safety
    ///
    /// Cancel safe: filling the read half's buffer consumes nothing.
    pub async fn peek(&mut self) -> io::Result<Option<u8>> {
        if let Some(error) = self.failure.take() {
            return Err(error);
        }
        let first = self.read.fill_buf().await?.first().copied();
        self.ended = first.is_none();
        Ok(first)
    }
}

/// Report a kept read error first, then read the transport.
impl<R: AsyncRead + Unpin> AsyncRead for PipeRead<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
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

/// One frame on its way to the multiplexer, and the charge it carries.
///
/// The charge pays for what the frame's chunk holds until the frame is
/// written: an encoder hands a chunk's charge over with the chunk's last
/// frame, and every other frame carries no charge. The multiplexer drops
/// the item, and so the charge, once it has written the frame, or copied
/// it into its write buffer. A lane delivers its frames in order, so every
/// earlier frame of the chunk is written by then.
pub struct Outbound {
    /// The frame to write.
    frame: Frame,
    /// The charge returned once `frame` is written.
    charge: Charge,
}

/// One local encoder's logical stream into the multiplexer.
pub struct Outgoing {
    /// The multiplexer's channel for this logical stream.
    frames: Sender<Outbound>,
    /// Whether any frame was sent, so the stream must end explicitly.
    opened: bool,
}

impl Outgoing {
    /// Hand one reply frame to the multiplexer, with the `charge` it
    /// returns once written ([`Outbound`]).
    ///
    /// Returns once the frame is in the multiplexer's channel, before it
    /// reaches the pipe, so the encoder can publish the local records of
    /// the reply's questions without waiting for the write. A multiplexer
    /// that has gone failed; the session's executor reports its error, so
    /// this awaits cancellation instead.
    pub async fn frame(&mut self, frame: ReplyFrame, charge: Charge) {
        self.opened = true;
        send_or_cancel(
            &mut self.frames,
            Outbound {
                frame: frame.into(),
                charge,
            },
        )
        .await;
    }

    /// End this logical stream after its last reply, if it carried any.
    pub async fn finish(mut self) {
        if self.opened {
            send_or_cancel(
                &mut self.frames,
                Outbound {
                    frame: Frame::End(End::Stream),
                    charge: Charge::default(),
                },
            )
            .await;
        }
    }
}

/// Send one item to the pipe's other side, or await cancellation if it has
/// gone: its failure is the session's error.
async fn send_or_cancel(frames: &mut Sender<Outbound>, item: Outbound) {
    if frames.send(item).await.is_err() {
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
/// written, and behind the earlier supply runs on its own stream, one staged or
/// being written and up to [`LANE_CAPACITY`] in its channel. A session's
/// traffic is finite, so every frame is still served. Within a class, logical
/// streams are served round-robin from that class's own cursor; a further class
/// takes its place in the order.
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
    frames: Option<Receiver<Outbound>>,
    /// The next frame, taken from the channel and awaiting its turn.
    staged: Option<Outbound>,
    /// Whether the channel must be polled: it has never been polled, or
    /// it has woken the multiplexer or yielded a frame since its last poll
    /// ([`Wakes`]).
    marked: bool,
    /// The waker the channel is polled with, which records this lane in
    /// the multiplexer's [`Wakes`].
    waker: Waker,
}

/// The lanes whose channels have woken the multiplexer since its last
/// poll, and the task to wake: the state every lane's waker shares.
///
/// The multiplexer polls a lane's channel only while the lane is marked
/// ([`OutgoingLane::marked`]). Each poll begins, under this state's lock,
/// by registering the polling task here and moving every lane recorded
/// here onto its mark. It unmarks a lane just before polling the lane's
/// channel, and marks it again unless the poll returns `Pending`, which
/// registers the lane's waker ([`LaneWaker`]) with the channel. At the
/// channel's next frame or close, that waker records the lane here and
/// takes the registered task to wake, under the same lock.
///
/// So an unmarked lane's channel found nothing at its last poll, and has
/// received nothing since that its waker has not recorded: every lane
/// whose channel received a frame or its close before a poll begins is
/// marked once the poll has collected the records. A poll that polls only
/// marked lanes therefore stages exactly the frames, and observes exactly
/// the closes, that polling every open lane with nothing staged would,
/// while an idle lane costs it one flag instead of a channel poll and a
/// waker registration. A lane recorded after the poll has collected the
/// records finds the poll's task registered, and wakes it for the next
/// poll.
#[derive(Default)]
struct Wakes {
    /// Whether each lane's channel has woken the multiplexer since the
    /// last poll collected the records, by lane index.
    woken: Vec<bool>,
    /// The task that last polled the multiplexer, until a lane's wake
    /// takes it.
    task: Option<Waker>,
}

/// One lane's waker: it records the lane in the multiplexer's [`Wakes`],
/// then wakes the task registered there.
struct LaneWaker {
    /// The lane's index among the multiplexer's lanes.
    index: usize,
    /// The state every lane's waker shares.
    wakes: Arc<Mutex<Wakes>>,
}

/// Record the lane, then wake the multiplexer's task.
impl Wake for LaneWaker {
    /// Wakes as [`wake_by_ref`](Wake::wake_by_ref) does, then drops this
    /// reference.
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    /// Records the lane and takes the registered task under the lock, then
    /// wakes the task after releasing it, so nothing the wake runs can
    /// wait on the lock.
    fn wake_by_ref(self: &Arc<Self>) {
        let task = {
            let mut wakes = lock(&self.wakes);
            wakes.woken[self.index] = true;
            wakes.task.take()
        };
        if let Some(task) = task {
            task.wake();
        }
    }
}

/// Lock the state every lane's waker shares.
///
/// A poisoned lock still guards consistent state: each critical section
/// sets flags and moves or clones a waker, and a panic there, which only a
/// waker's own clone can raise, leaves every flag and waker whole.
fn lock(wakes: &Mutex<Wakes>) -> MutexGuard<'_, Wakes> {
    wakes.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Writes every local logical stream's frames to the pipe's write half.
///
/// Each lane's channel wakes the multiplexer through the lane's own waker,
/// which records the lane ([`Wakes`]) and then wakes the task that last
/// polled the multiplexer. Each poll registers the polling task and
/// collects the recorded lanes, then polls only marked lanes' channels.
/// The multiplexer may be polled for wakes that are not its own, but it
/// polls a lane's channel only when that channel may have something to
/// give.
#[derive(Default)]
pub struct Multiplexer {
    /// One lane per logical stream an encoder was created for.
    lanes: Vec<OutgoingLane>,
    /// The lanes whose channels have woken the multiplexer, and the task
    /// they wake.
    wakes: Arc<Mutex<Wakes>>,
    /// How many bytes of frames one flush may carry.
    batch: WriteBatch,
}

impl Multiplexer {
    /// A multiplexer with no lanes yet, batching its writes within `batch`.
    pub fn new(batch: WriteBatch) -> Self {
        Self {
            batch,
            ..Self::default()
        }
    }

    /// Create the channel for one local logical stream, labeled at the
    /// height of the replies it carries.
    pub fn lane(&mut self, stream: Stream, height: usize) -> Outgoing {
        let (frames, receiver) = channel(
            QueueRole::new(QueueKind::PipeOutgoing, height),
            LANE_CAPACITY,
        );
        let index = self.lanes.len();
        lock(&self.wakes).woken.push(false);
        let waker = Waker::from(Arc::new(LaneWaker {
            index,
            wakes: Arc::clone(&self.wakes),
        }));
        self.lanes.push(OutgoingLane {
            stream,
            frames: Some(receiver),
            staged: None,
            // A new lane's channel must be polled once to register its
            // waker.
            marked: true,
            waker,
        });
        Outgoing {
            frames,
            opened: false,
        }
    }

    /// Write frames until every lane's channel has closed.
    ///
    /// Each frame is written whole before the next is chosen, through a
    /// Tokio `BufWriter` of [`WRITE_BUFFER_BYTES`]: a frame smaller than the
    /// buffer is copied into it, and one at least its size passes it.
    /// Writing does not flush by itself; when the multiplexer flushes
    /// depends on its [`WriteBatch`].
    ///
    /// The frames written since the last flush form one *batch*, which the
    /// multiplexer keeps within its ceiling, the segment size or the
    /// largest batch, as [`WriteBatch`] describes. It flushes before
    /// writing a frame that would push the batch past its ceiling, and as
    /// soon as the batch reaches it, since no further frame fits beside it
    /// then; a frame larger than the ceiling therefore goes out alone, and
    /// a frame larger than the segment size starts a batch of its own
    /// after the one before it is flushed. When a poll finds no frame
    /// ready while a batch is unflushed, the multiplexer yields to the
    /// session's task set before deciding, up to [`EMPTY_PASSES`] times:
    /// the frames it took woke their encoders, and frames the session read
    /// woke the members that turn them into replies, which all run only
    /// once its poll returns, so lanes it found empty may hold frames again
    /// after a few passes of the set. As soon as a poll finds a frame, it
    /// keeps writing; if none does, it flushes, and only then waits for a
    /// frame.
    ///
    /// With a largest batch of zero, it never yields: it flushes whenever a
    /// poll finds no frame ready, so only the frames ready within one poll
    /// share a flush, and the buffer writes itself out whenever the next
    /// frame does not fit beside the buffered ones.
    ///
    /// Either way it flushes before returning, and a flush with nothing
    /// written since the last one does nothing. A flushed batch reaches
    /// the write half as one plain write, continued wherever the write half
    /// takes only part of one.
    ///
    /// A frame the multiplexer wrote reaches a flush within a bounded
    /// number of passes of the session's task set: after at most
    /// [`EMPTY_PASSES`] passes in a row that bring no frame the
    /// multiplexer flushes, and every pass that brings one adds at least
    /// one frame, of at least one byte, to the batch, so the batch reaches
    /// its ceiling, and is flushed, within [`EMPTY_PASSES`] times as many
    /// passes as the ceiling has bytes, and [`EMPTY_PASSES`] more. That is
    /// the segment size for a batch that keeps one-segment records, and
    /// the largest batch for any other. Under an unbounded largest batch, a
    /// batch past the segment size has no ceiling: the buffer writes
    /// itself out whenever the next frame does not fit, and the flush
    /// comes after the first run of [`EMPTY_PASSES`] passes that bring no
    /// frame. Without new frames from the peer, this side's walk readies
    /// only finitely many frames, so that run comes without waiting on the
    /// peer, though no bound in bytes holds. The multiplexer never waits
    /// for a frame while a frame it wrote sits in its buffer or unflushed:
    /// a yield is not a wait. While it writes and flushes, it waits only on
    /// the write half. A frame handed to a lane wakes the waiting
    /// multiplexer through that lane's waker, so every frame handed to it
    /// reaches the peer without waiting on anything but the pipe. The
    /// write half is borrowed, and rests flushed at a frame boundary on
    /// success, ready for the closing exchange.
    pub async fn run<W: AsyncWrite + Unpin>(
        mut self,
        write: &mut W,
        speaker: Speaker,
        stats: Recorder,
        observe: &SessionHandle,
    ) -> Result<(), EncodeError> {
        let write =
            BufWriter::with_capacity(WRITE_BUFFER_BYTES, CountedWrite::new(write, stats.clone()));
        let mut out = FrameWrite::new(speaker, write)
            .observed(observe.data(speaker.role(), Direction::Sent))
            .batched(self.batch.segment(), self.batch.largest());
        let yields = self.batch.yields();
        let mut cursors = [0; Class::ORDER.len()];
        loop {
            let mut ready = poll_fn(|cx| Poll::Ready(self.poll_next(cx, &mut cursors))).await;
            if ready.is_pending() && yields && out.is_unflushed() {
                // We found no frame, but the members we woke, the encoders
                // whose frames we took and those turning the frames the
                // session read into replies, may not have run yet, so we let
                // every member woken so far run, a few passes in a row,
                // before deciding to flush.
                for _ in 0..EMPTY_PASSES {
                    yield_to_members().await;
                    ready = poll_fn(|cx| Poll::Ready(self.poll_next(cx, &mut cursors))).await;
                    if ready.is_ready() {
                        break;
                    }
                }
            }
            let next = match ready {
                Poll::Ready(next) => next,
                Poll::Pending => {
                    out.flush().await?;
                    poll_fn(|cx| self.poll_next(cx, &mut cursors)).await
                }
            };
            let Some((stream, Outbound { frame, charge })) = next else {
                break;
            };
            out.frame(&(stream, frame)).await?;
            // The frame is written, or copied into the write buffer, so
            // what its chunk held is no longer needed.
            drop(charge);
            stats.frame_sent();
        }
        out.flush().await
    }

    /// Take every frame that is ready, then remove and return the next one
    /// to write: `Ready(None)` once every lane has closed with nothing
    /// staged, and `Pending`, with a wakeup registered, while open lanes
    /// have nothing staged.
    ///
    /// Only marked lanes' channels are polled ([`Wakes`]), which stages
    /// the same frames as polling every open lane with nothing staged.
    fn poll_next(
        &mut self,
        cx: &mut Context<'_>,
        cursors: &mut [usize; Class::ORDER.len()],
    ) -> Poll<Option<(Stream, Outbound)>> {
        {
            // Register and collect under one lock: a lane recorded after
            // this finds this task registered, and wakes it.
            let mut wakes = lock(&self.wakes);
            if !wakes
                .task
                .as_ref()
                .is_some_and(|task| task.will_wake(cx.waker()))
            {
                wakes.task = Some(cx.waker().clone());
            }
            for (lane, woken) in self.lanes.iter_mut().zip(&mut wakes.woken) {
                lane.marked |= mem::take(woken);
            }
        }
        let mut open = false;
        let mut tally = MultiplexerPoll::default();
        for lane in &mut self.lanes {
            if lane.staged.is_none()
                && let Some(frames) = &mut lane.frames
            {
                tally.unstaged += 1;
                if mem::take(&mut lane.marked) {
                    tally.polled += 1;
                    match frames.poll_next_unpin(&mut Context::from_waker(&lane.waker)) {
                        Poll::Ready(Some(frame)) => {
                            lane.staged = Some(frame);
                            // A channel that yields a frame registers no
                            // waker, so nothing else would mark the lane for
                            // its next frame or its close.
                            lane.marked = true;
                        }
                        Poll::Ready(None) => lane.frames = None,
                        Poll::Pending => {}
                    }
                }
            }
            open |= lane.staged.is_some() || lane.frames.is_some();
        }
        let next = match self.choose(cursors) {
            Some(index) => {
                let lane = &mut self.lanes[index];
                let item = lane.staged.take().expect("choose returns a staged lane");
                Poll::Ready(Some((lane.stream, item)))
            }
            None if open => Poll::Pending,
            None => Poll::Ready(None),
        };
        tally.frame = matches!(next, Poll::Ready(Some(_)));
        observe_poll(tally);
        next
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
                        .is_some_and(|item| Class::of(&item.frame) == class)
                });
            if let Some(index) = found {
                *cursor = index + 1;
                return Some(index);
            }
        }
        None
    }
}

/// Return `Pending` once, waking the polling task first, then `Ready`.
///
/// Within the session's task set, a
/// [`FuturesUnordered`](futures::stream::FuturesUnordered), the wake
/// queues the multiplexer behind every member already woken: the set keeps
/// one first-in, first-out queue of woken members and never queues a
/// member twice. So the multiplexer's next poll follows a poll of each
/// member woken before the yield, the encoders it woke by taking their
/// frames among them. It is a yield, not a wait: nothing outside the set
/// need happen for the multiplexer to be polled again.
///
/// The set counts this wake as a yield of the multiplexer's own. Once two
/// members have yielded in one poll of the set, or it has polled as many
/// members as it holds, it wakes the session and returns `Pending` to the
/// executor, keeping its queue as it is. Either way the order holds:
/// the members queued ahead of the multiplexer are polled before it,
/// whether in this poll of the set or the next. The yield cannot spin,
/// because the multiplexer yields only while a batch is unflushed, at most
/// [`EMPTY_PASSES`] times in a row without finding a frame, and again only
/// after a poll that found one, which grows the batch toward its ceiling
/// ([`WriteBatch`]); once that many yields in a row find no frame, it
/// flushes and then waits.
async fn yield_to_members() {
    let mut yielded = false;
    poll_fn(|cx| {
        if mem::replace(&mut yielded, true) {
            Poll::Ready(())
        } else {
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    })
    .await;
}

/// The most passes in a row of the session's task set, each finding no
/// frame, that the multiplexer lets run before it flushes an unflushed
/// batch ([`Multiplexer::run`]).
///
/// A frame this side writes is most often caused by a frame it read, and it
/// reaches the multiplexer only once each member on the way has run in
/// turn: the demultiplexer hands the read frame to its decoder, the decoder
/// parks the reply, and the encoder of the stage that answers it polls the
/// stage's walk for the answer and hands its frame to a lane. Each yield
/// lets the members woken so far run once, so a frame read before the
/// multiplexer first finds nothing can arrive by its second pass, after the
/// decoder's and the encoder's turns, and flushing after the first pass
/// sends a short batch of what was ready while the replies the session is
/// already building are a pass away. Four passes also take in the frames
/// read up to two passes later. A pass that finds the set at rest costs one
/// empty poll of the multiplexer, and the set returns to its executor once
/// two of its members have yielded in one of its polls
/// ([`yield_to_members`]), so a session with nothing coming flushes after
/// four empty polls and two trips of its task through the executor's
/// queue: microseconds on an executor with nothing else to run, and longer
/// behind other ready tasks.
pub(crate) const EMPTY_PASSES: usize = 4;

/// How the multiplexer gathers frames into flushes: its *write batch
/// range*, from a *segment size* `S` to a *largest batch* `R`.
///
/// The frames written between two flushes form a *batch*, and each batch
/// has a *ceiling*, fixed by its first frame and the batches before it:
///
/// - `S`, when its first frame is at most `S` bytes, writing out the batch
///   before it did not wait on the write half, and fewer than
///   [`CUTS_TO_GROW`](super::codec::CUTS_TO_GROW) batches in a row before
///   it were flushed at their ceiling rather than because nothing more was
///   ready. Every frame in it is then at most `S`, so the batch keeps
///   one-segment records.
/// - `R`, or no ceiling when `R` is unbounded, otherwise: a frame larger
///   than `S` spans segments however it is written, so it shares a write
///   with the frames after it; while the write half is full, bytes queue
///   anyway, so small writes buy no latency; and batches filled to their
///   ceiling that many times in a row show frames coming back to back,
///   where each small write would only delay the frames queued behind it.
///   A single cut does not suffice: a burst of a little over one segment,
///   which a session between nearly identical replicas meets now and then,
///   keeps one-segment records, which the peer can decrypt as each segment
///   arrives.
///
/// A ceiling bounds a batch, never makes the multiplexer wait for more:
/// it flushes before a frame that would push the batch past its ceiling,
/// as soon as the batch reaches it, and whenever its task set brings no
/// new frame in [`EMPTY_PASSES`] yields in a row ([`Multiplexer::run`]). A frame larger than its
/// batch's ceiling therefore goes out alone. `R` = 0 instead turns
/// batching across polls off: the multiplexer flushes whenever a poll
/// finds no frame ready, without yielding first, and no ceiling applies.
///
/// The range only chooses when the buffer is flushed. The buffer itself
/// stays [`WRITE_BUFFER_BYTES`], so the range moves no memory bound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WriteBatch {
    /// The segment size `S`: at most `largest`, or at most
    /// [`WRITE_BUFFER_BYTES`] when `largest` is unbounded.
    segment: usize,
    /// The largest batch `R`, at most [`WRITE_BUFFER_BYTES`], or `None`
    /// for no ceiling.
    largest: Option<usize>,
}

impl WriteBatch {
    /// The write batch `range` describes, as
    /// [`Peer::write_batch_range`](crate::Peer::write_batch_range)
    /// documents the conversion.
    ///
    /// Each bound converts to an inclusive size: an excluded start `s` to
    /// `s + 1` (saturating), an excluded end `e` to `e - 1` (and an
    /// excluded end of zero, an empty range, to zero), an unbounded start
    /// to zero. An unbounded end means no ceiling. The end saturates at
    /// [`WRITE_BUFFER_BYTES`], since a batch past the buffer writes itself
    /// out before any flush, and the start is lowered to the end, or to
    /// the buffer's capacity under an unbounded end.
    pub fn from_range(range: impl RangeBounds<usize>) -> Self {
        let largest = match range.end_bound() {
            Bound::Included(&end) => Some(end.min(WRITE_BUFFER_BYTES)),
            Bound::Excluded(&end) => Some(end.saturating_sub(1).min(WRITE_BUFFER_BYTES)),
            Bound::Unbounded => None,
        };
        let start = match range.start_bound() {
            Bound::Included(&start) => start,
            Bound::Excluded(&start) => start.saturating_add(1),
            Bound::Unbounded => 0,
        };
        Self {
            segment: start.min(largest.unwrap_or(WRITE_BUFFER_BYTES)),
            largest,
        }
    }

    /// The range as the multiplexer applies it: the segment size, included,
    /// to the largest batch, included, or unbounded.
    ///
    /// Converting it back gives the same write batch.
    pub fn range(self) -> (Bound<usize>, Bound<usize>) {
        (
            Bound::Included(self.segment),
            self.largest.map_or(Bound::Unbounded, Bound::Included),
        )
    }

    /// The segment size `S` in bytes.
    pub const fn segment(self) -> usize {
        self.segment
    }

    /// The ceiling on a batch that does not keep to the segment size: the
    /// largest batch `R`, or `None` for no ceiling, which `R` = 0 also
    /// means.
    pub fn largest(self) -> Option<NonZeroUsize> {
        self.largest.and_then(NonZeroUsize::new)
    }

    /// Whether the multiplexer yields to its task set before flushing, so
    /// that batches grow across passes: always, except at `R` = 0.
    pub fn yields(self) -> bool {
        self.largest != Some(0)
    }
}

/// The batch range of [`default::tls::WRITE_BATCH_RANGE`].
impl Default for WriteBatch {
    fn default() -> Self {
        Self::from_range(crate::default::tls::WRITE_BATCH_RANGE)
    }
}

/// The capacity of the buffer the multiplexer writes frames into: the
/// largest whole number of full-fan query frames within rustls's 64 KiB
/// send limit.
///
/// Every transport pays per write: a TCP socket a system call, an
/// unbuffered TLS stream a record and a system call. A session's frames
/// are mostly a few bytes, so the multiplexer copies those it writes
/// between flushes into one buffer and writes the buffer out at once;
/// copying a few bytes costs less than a write, or than an entry in a
/// vectored write. A frame that does not fit beside the buffered ones
/// writes them out first. A frame at least this large
/// passes the buffer: Tokio's `BufWriter` hands it whole, uncopied, to a
/// write half that takes vectored writes, and to one that does not, it
/// copies the frame's leading pieces that fit and writes the large piece
/// directly.
///
/// The size meets three bounds, which the assertion below checks:
///
/// - It exceeds [`FULL_FAN_QUERY_FRAME_LEN`], the largest frame that
///   carries one comparison, so every comparison frame is copied.
/// - It is at most [`TLS_SEND_LIMIT`], so tokio-rustls, at rustls's default
///   limit, takes a whole buffer in one write when no earlier records wait
///   to be sent, and sends its records together. A larger buffer would take
///   two rounds of encryption and socket writes on that path.
/// - It is at most [`default::TARGET_MESSAGE_SIZE`], so a supply run that
///   reaches the default target bypasses the buffer instead of being
///   copied.
///
/// The size follows the frame: it is as many whole full-fan query frames
/// as fit within the TLS bound. That meets the second bound by
/// construction, and the first whenever two such frames fit. Nothing else
/// depends on the buffer holding a whole number of them: any size between
/// the first two bounds meets all three.
pub(crate) const WRITE_BUFFER_BYTES: usize =
    TLS_SEND_LIMIT / FULL_FAN_QUERY_FRAME_LEN * FULL_FAN_QUERY_FRAME_LEN;

/// The most plaintext rustls takes from one write while it has no encrypted
/// records waiting to be sent: its default limit on queued outgoing records,
/// `DEFAULT_BUFFER_LIMIT` in rustls 0.23, 64 KiB.
///
/// rustls compares the plaintext a write offers against that limit less the
/// records already queued, and takes no more than fits; tokio-rustls then
/// writes the queued records to the socket in one vectored write, and
/// returns a short write if the socket takes only part of them.
///
/// This mirrors a private rustls default that no check here can see, so a
/// rustls release that changed it would leave this value stale without any
/// failure. Only performance depends on it: a larger buffer, or a smaller
/// limit, takes more rounds of encryption and socket writes, never a wrong
/// byte.
const TLS_SEND_LIMIT: usize = 64 * 1024;

const _: () = assert!(
    FULL_FAN_QUERY_FRAME_LEN < WRITE_BUFFER_BYTES
        && WRITE_BUFFER_BYTES <= TLS_SEND_LIMIT
        && WRITE_BUFFER_BYTES <= default::TARGET_MESSAGE_SIZE,
    "the write buffer copies every comparison frame, fits one TLS write, and passes a default-target run through"
);

/// What one poll of a multiplexer did, as the process's multiplexer hook
/// sees it.
///
/// A poll stages frames from lane channels, then returns at most one frame
/// to write. Each lane that is open with nothing staged is a lane whose
/// channel may hold a frame or its close; the poll polls those of them
/// whose channels have woken the multiplexer, or yielded a frame, since
/// their last poll.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MultiplexerPoll {
    /// The lanes whose channels the poll polled.
    pub polled: usize,
    /// The lanes that were open with nothing staged when the poll reached
    /// them: the lanes a poll of every lane's channel polls.
    pub unstaged: usize,
    /// Whether the poll returned a frame to write.
    pub frame: bool,
}

/// The process's multiplexer observation hook, called after every poll of
/// any multiplexer.
#[cfg(any(test, feature = "test-internals"))]
pub type MultiplexerHook = fn(MultiplexerPoll);

/// The hook every multiplexer in the process reports through, once
/// installed.
#[cfg(any(test, feature = "test-internals"))]
static MULTIPLEXER_HOOK: OnceLock<MultiplexerHook> = OnceLock::new();

/// Install `hook` as the process's multiplexer observation hook, returning
/// whether it is now the installed one.
///
/// The first installation wins for the life of the process, as for the
/// ledger observation hook.
#[cfg(any(test, feature = "test-internals"))]
pub fn observe_multiplexers(hook: MultiplexerHook) -> bool {
    std::ptr::fn_addr_eq(*MULTIPLEXER_HOOK.get_or_init(|| hook), hook)
}

/// Report one poll of a multiplexer to the installed hook.
#[cfg(any(test, feature = "test-internals"))]
fn observe_poll(poll: MultiplexerPoll) {
    if let Some(hook) = MULTIPLEXER_HOOK.get() {
        hook(poll);
    }
}

/// Report one poll of a multiplexer: nothing outside test builds.
#[cfg(not(any(test, feature = "test-internals")))]
#[inline(always)]
fn observe_poll(_: MultiplexerPoll) {}

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
    /// frames, the read half rests at an item boundary, the next item's
    /// first bytes, if any arrived, unconsumed in its buffer. The routes are
    /// borrowed too, so a failure leaves every decoder's channel open until
    /// the demultiplexer itself is dropped.
    pub async fn run<R: AsyncBufRead + Unpin>(
        &mut self,
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
    async fn stop(&mut self, speaker: Speaker) -> PipeError {
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
        for lane in &mut self.lanes {
            if matches!(lane, IncomingLane::Open { .. }) {
                *lane = IncomingLane::Ended;
            }
        }
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
