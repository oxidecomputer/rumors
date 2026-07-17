//! Lazily established, independently flow-controlled logical streams.
//!
//! This layer binds the protocol's 17-per-direction logical streams onto a
//! [`Link`](crate::link)'s transport streams, one to one. Nothing multiplexes:
//! each logical stream owns its transport stream outright, so backpressure on
//! one stream is invisible to every other — the independence the capacity-one
//! channel arguments of the materialized walk rely on is supplied by the
//! [link contract](crate::link), not reconstructed here.
//!
//! Streams are established lazily, on both sides, from the same local fact: a
//! stream carries answers to questions its receiver asked, and each side
//! learns whether any question exists at a level before it touches the
//! stream. A [`StreamSender`] connects on its first frame — a level that
//! produces no reply never opens its stream — and a [`StreamReceiver`]
//! claims its accepted stream on its first read — a level that asks no
//! question never claims one. Empty streams therefore never exist on the
//! wire, rather than opening only to say so.
//!
//! Because transport streams arrive anonymously and in any order, the sender
//! labels each opened stream with the session epoch and its logical stream
//! index before the first frame. The session's [`AcceptDriver`] — the sole
//! reader of the link's acceptor — validates each label and delivers the
//! stream to the one claim slot it names. Every frame's signal byte then
//! re-states the stream index; [`StreamReceiver`] holds each frame to exact
//! agreement with the claimed label, so a routing mistake in a caller-built
//! link surfaces as a first-frame [`StreamError::Mislabeled`] instead of as
//! garbled protocol.
//!
//! Outgoing failures surface directly from the failing producer's send. An
//! incoming stream cannot fail through that path — its consumer expects an
//! infallible frame stream — so [`StreamReceiver`] reports its failure to
//! the session's one-slot error route and parks, and the session driver
//! observes the route (the same publish-then-park discipline the response
//! streams use).

use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

use async_stream::stream;
use borsh::BorshDeserialize;
use futures::{StreamExt, stream::BoxStream};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::{mpsc, oneshot},
};

use crate::link::{Acceptor, Connector};
use crate::tree::mirror::streaming::tasks::cancelled;

use super::codec::{
    DecodeError, EncodeError, End, Frame, FrameRead, FrameWrite, Origin, Speaker, Stream,
};

/// Bytes of the label a sender writes before its first frame.
const LABEL_LEN: usize = 2;

/// Render the label naming one opened stream: session epoch, stream index.
fn label(epoch: u8, stream: Stream) -> [u8; LABEL_LEN] {
    [epoch, stream.index()]
}

/// Number of logical streams per direction, as an array dimension.
const STREAM_COUNT: usize = Stream::COUNT as usize;

/// A protocol reply frame, statically excluding stream-end transport control.
///
/// Stream end is a lifecycle event owned by [`StreamSender::finish`]; a
/// producer cannot smuggle one into the middle of its replies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplyFrame<T>(Frame<T>);

impl<T> TryFrom<Frame<T>> for ReplyFrame<T> {
    type Error = ReplyFrameError;

    /// Check that a general wire frame belongs to a protocol reply.
    fn try_from(frame: Frame<T>) -> Result<Self, Self::Error> {
        if matches!(frame, Frame::End(End::Stream)) {
            Err(ReplyFrameError::StreamEnd)
        } else {
            Ok(Self(frame))
        }
    }
}

impl<T> From<ReplyFrame<T>> for Frame<T> {
    /// Recover the general wire frame for transport encoding.
    fn from(frame: ReplyFrame<T>) -> Self {
        frame.0
    }
}

/// A general wire frame was transport control rather than a protocol reply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ReplyFrameError {
    /// Stream end is emitted only by the internal `StreamSender::finish` path.
    #[error("stream-end control is not a protocol reply frame")]
    StreamEnd,
}

/// One lazily opened outgoing logical stream.
///
/// The first [`frame`](Self::frame) connects a transport stream, writes the
/// label, and flushes the frame behind it; a sender that never carries a
/// frame never opens a stream. [`finish`](Self::finish) closes an opened
/// stream with an explicit end control and skips silently otherwise.
pub struct StreamSender<C: Connector, T> {
    connector: C,
    epoch: u8,
    /// The local role whose direction this stream carries.
    speaker: Speaker,
    stream: Stream,
    state: SendState<C::Tx>,
    marker: PhantomData<fn(T)>,
}

enum SendState<Tx> {
    Unopened,
    Open(FrameWrite<Tx>),
}

impl<C: Connector, T> StreamSender<C, T> {
    /// Bind one outgoing logical stream to a link's stream supply.
    pub fn new(connector: C, epoch: u8, speaker: Speaker, stream: Stream) -> Self {
        Self {
            connector,
            epoch,
            speaker,
            stream,
            state: SendState::Unopened,
            marker: PhantomData,
        }
    }

    /// Write and flush one reply frame, opening the stream on the first.
    pub async fn frame(&mut self, frame: ReplyFrame<T>) -> Result<(), SendError> {
        self.write(frame.into()).await
    }

    /// End this logical stream after all of its replies, if it ever opened.
    ///
    /// Dropping the transport half afterward is the transport-level
    /// half-close; the explicit end control before it distinguishes a
    /// completed stream from one truncated mid-reply.
    pub async fn finish(mut self) -> Result<(), SendError> {
        match self.state {
            SendState::Unopened => Ok(()),
            SendState::Open(_) => self.write(Frame::End(End::Stream)).await,
        }
    }

    /// Write one frame through the open transport stream, opening it first.
    async fn write(&mut self, frame: Frame<T>) -> Result<(), SendError> {
        let stream = self.stream;
        let write = match &mut self.state {
            SendState::Open(write) => write,
            state @ SendState::Unopened => {
                let mut tx =
                    self.connector
                        .connect()
                        .await
                        .map_err(|source| SendError::Connect {
                            origin: Origin::stream(self.speaker, stream),
                            source,
                        })?;
                tx.write_all(&label(self.epoch, stream))
                    .await
                    .map_err(|source| SendError::Label {
                        origin: Origin::stream(self.speaker, stream),
                        source,
                    })?;
                *state = SendState::Open(FrameWrite::new(self.speaker, tx));
                let SendState::Open(write) = state else {
                    unreachable!("the open state was just stored");
                };
                write
            }
        };
        write
            .frame(&(stream, frame))
            .await
            .map_err(SendError::Frame)
    }
}

/// An outgoing logical stream could not be opened, labeled, or written.
#[derive(Debug, thiserror::Error)]
pub enum SendError {
    /// The link's connector could not open a transport stream.
    #[error("{origin}: opening the transport stream failed")]
    Connect {
        origin: Origin,
        #[source]
        source: std::io::Error,
    },
    /// The opened transport stream rejected its label.
    #[error("{origin}: labeling the transport stream failed")]
    Label {
        origin: Origin,
        #[source]
        source: std::io::Error,
    },
    /// The outgoing frame codec or transport failed.
    #[error(transparent)]
    Frame(#[from] EncodeError),
}

/// A failure on the incoming side of the session's stream layer.
///
/// Reported through the session's one-slot error route: incoming consumers
/// expect infallible frame streams, so the failing stream parks after
/// publishing and the session driver surfaces the cause.
#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    /// The incoming frame codec rejected bytes or the transport failed.
    #[error(transparent)]
    Decode(#[from] DecodeError),
    /// A frame's signal byte named a stream other than the claimed label.
    #[error("{origin}: stream labeled {labeled} carried a frame for {framed}")]
    Mislabeled {
        origin: Origin,
        labeled: u8,
        framed: u8,
    },
    /// The transport stream ended before its explicit end control.
    #[error("{origin}: transport stream ended before its end control")]
    Truncated { origin: Origin },
    /// The peer sent more frames after ending its logical stream.
    #[error("{origin}: peer sent a frame after ending the logical stream")]
    AfterEnd { origin: Origin },
    /// The stream supply failed before an awaited stream was delivered.
    ///
    /// `source` carries the supply's own failure when this reporter was the
    /// first to observe it; a second stream failing on the same supply
    /// reports without it.
    #[error("{origin}: the link's stream supply closed before this stream arrived")]
    SupplyClosed {
        origin: Origin,
        source: Option<std::io::Error>,
    },
}

/// One lazily claimed incoming logical stream, yielding its protocol frames.
///
/// The first poll claims the accepted transport stream delivered for this
/// label; a receiver that is never polled never claims. The stream ends —
/// yields `None` — when the peer's explicit end control arrives, and it
/// consumes that control rather than exposing it. On any failure it reports
/// through the session error route and parks.
pub struct StreamReceiver<Rx, T> {
    /// The claim and identity, consumed to build `frames` on first poll.
    start: Option<ReceiverStart<Rx>>,
    /// `Some` exactly once the stream has been claimed: the first poll
    /// builds it, and [`finish`](Self::finish) reads its absence as "this
    /// level was never needed".
    frames: Option<BoxStream<'static, Frame<T>>>,
}

struct ReceiverStart<Rx> {
    claim: oneshot::Receiver<Rx>,
    /// The remote role whose direction this stream carries.
    speaker: Speaker,
    stream: Stream,
    route: ErrorRoute,
}

impl<Rx, T> StreamReceiver<Rx, T>
where
    Rx: tokio::io::AsyncRead + Unpin + Send + 'static,
    T: BorshDeserialize + Send + Sync + 'static,
{
    /// Bind one incoming logical stream to its claim slot.
    pub fn new(
        claim: oneshot::Receiver<Rx>,
        speaker: Speaker,
        stream: Stream,
        route: ErrorRoute,
    ) -> Self {
        Self {
            start: Some(ReceiverStart {
                claim,
                speaker,
                stream,
                route,
            }),
            frames: None,
        }
    }

    /// Require this stream to be finished: never claimed, or cleanly ended.
    ///
    /// A level that asked no question skips its stream entirely — awaiting
    /// frames from a stream a correct peer never opens would hang — while a
    /// claimed stream must have delivered its end with no further reply.
    /// Returns whether an extra reply arrived instead.
    pub async fn finish(&mut self) -> ReceiverFinish {
        if self.frames.is_none() {
            return ReceiverFinish::Clean;
        }
        match self.next().await {
            None => ReceiverFinish::Clean,
            Some(_) => ReceiverFinish::ExtraReply,
        }
    }

    /// Build the claimed frame stream on first use.
    fn frames(&mut self) -> &mut BoxStream<'static, Frame<T>> {
        let start = &mut self.start;
        self.frames.get_or_insert_with(|| {
            let ReceiverStart {
                claim,
                speaker,
                stream,
                route,
            } = start
                .take()
                .expect("the start state is consumed exactly once");
            Box::pin(read_frames(claim, speaker, stream, route))
        })
    }
}

/// The outcome of draining a receiver at its consumer's boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiverFinish {
    /// The stream was never needed, or ended exactly on time.
    Clean,
    /// The peer sent a reply beyond the last local question.
    ExtraReply,
}

impl<Rx, T> futures::Stream for StreamReceiver<Rx, T>
where
    Rx: tokio::io::AsyncRead + Unpin + Send + 'static,
    T: BorshDeserialize + Send + Sync + 'static,
{
    type Item = Frame<T>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.get_mut().frames().as_mut().poll_next(cx)
    }
}

/// Claim the transport stream, then decode frames until the end control.
///
/// Every failure path publishes to the session error route and parks: the
/// consumer never observes a truncated stream as a clean end.
fn read_frames<Rx, T>(
    claim: oneshot::Receiver<Rx>,
    speaker: Speaker,
    stream: Stream,
    route: ErrorRoute,
) -> impl futures::Stream<Item = Frame<T>> + Send
where
    Rx: tokio::io::AsyncRead + Unpin + Send + 'static,
    T: BorshDeserialize + Send + Sync + 'static,
{
    stream! {
        let Ok(rx) = claim.await else {
            // The claim slot is gone: the link's stream supply failed before
            // the peer's stream for this level arrived. This is the one
            // consumer that provably needed it, so the report comes from
            // here; a supply failure that nothing was waiting on lets the
            // session finish on the streams it already holds.
            let source = route.take_supply_failure();
            route.report(StreamError::SupplyClosed {
                origin: Origin::stream(speaker, stream),
                source,
            });
            cancelled().await
        };
        let mut read = FrameRead::new(speaker, rx);
        loop {
            let frame = match read.frame::<T>().await {
                Ok(Some((framed, frame))) if framed == stream => frame,
                Ok(Some((framed, _))) => {
                    route.report(StreamError::Mislabeled {
                        origin: Origin::stream(speaker, stream),
                        labeled: stream.index(),
                        framed: framed.index(),
                    });
                    cancelled().await
                }
                Ok(None) => {
                    route.report(StreamError::Truncated {
                        origin: Origin::stream(speaker, stream),
                    });
                    cancelled().await
                }
                Err(error) => {
                    route.report(StreamError::Decode(error));
                    cancelled().await
                }
            };
            if matches!(frame, Frame::End(End::Stream)) {
                // The lifecycle control is consumed here; the consumer sees
                // only complete replies followed by a clean end. The sender
                // half-closes immediately after this control, so requiring
                // end-of-stream costs no waiting against an honest peer and
                // catches one that keeps talking past its own end.
                match read.frame::<T>().await {
                    Ok(None) => break,
                    Ok(Some(_)) => {
                        route.report(StreamError::AfterEnd {
                            origin: Origin::stream(speaker, stream),
                        });
                        cancelled().await
                    }
                    Err(error) => {
                        route.report(StreamError::Decode(error));
                        cancelled().await
                    }
                }
            }
            yield frame;
        }
    }
}

/// The reporting half of the session's one-slot first-error route.
#[derive(Clone)]
pub struct ErrorRoute {
    send: mpsc::Sender<StreamError>,
    /// The parked accept driver's deposited transport failure, claimed by
    /// the first [`StreamError::SupplyClosed`] reporter so the causal I/O
    /// error survives the deferral.
    supply_failure: std::sync::Arc<std::sync::Mutex<Option<std::io::Error>>>,
}

impl ErrorRoute {
    /// Publish the first incoming-stream error without blocking its reporter.
    fn report(&self, error: StreamError) {
        let _ = self.send.try_send(error);
    }

    /// Deposit the supply's transport failure for a later reporter.
    fn supply_failed(&self, source: std::io::Error) {
        let mut slot = self.supply_failure.lock().expect("supply failure lock");
        slot.get_or_insert(source);
    }

    /// Claim the deposited supply failure, if this is the first taker.
    fn take_supply_failure(&self) -> Option<std::io::Error> {
        self.supply_failure
            .lock()
            .expect("supply failure lock")
            .take()
    }
}

/// The observing half of the session's first-error route.
pub struct FirstStreamError {
    receive: mpsc::Receiver<StreamError>,
}

impl FirstStreamError {
    /// Resolve to the first reported error, or park if none ever arrives.
    pub async fn first(&mut self) -> StreamError {
        match self.receive.recv().await {
            Some(error) => error,
            // Every reporter is gone without an error: the session is
            // completing; defer to its outcome.
            None => cancelled().await,
        }
    }
}

/// Allocate the session's incoming-stream error route.
pub fn error_route() -> (ErrorRoute, FirstStreamError) {
    let (send, receive) = mpsc::channel(1);
    (
        ErrorRoute {
            send,
            supply_failure: std::sync::Arc::new(std::sync::Mutex::new(None)),
        },
        FirstStreamError { receive },
    )
}

/// The claim slots the accept driver delivers incoming streams into.
pub struct ClaimSlots<Rx> {
    slots: [Option<oneshot::Sender<Rx>>; STREAM_COUNT],
}

/// The claim receivers the session's typed states take streams from.
pub struct Claims<Rx> {
    slots: [Option<oneshot::Receiver<Rx>>; STREAM_COUNT],
}

impl<Rx> Claims<Rx> {
    /// Take the sole claim for `stream`.
    pub fn take(&mut self, stream: Stream) -> oneshot::Receiver<Rx> {
        self.slots[usize::from(stream.index())]
            .take()
            .expect("each incoming logical stream is claimed exactly once")
    }
}

/// Allocate the take-once claim slot for every incoming logical stream.
pub fn claims<Rx>() -> (ClaimSlots<Rx>, Claims<Rx>) {
    let mut senders = Vec::with_capacity(STREAM_COUNT);
    let receivers = std::array::from_fn(|_| {
        let (send, receive) = oneshot::channel();
        senders.push(Some(send));
        Some(receive)
    });
    let slots = senders
        .try_into()
        .unwrap_or_else(|_| unreachable!("one sender exists for every stream"));
    (ClaimSlots { slots }, Claims { slots: receivers })
}

/// The session's sole consumer of the link's acceptor.
///
/// Accepts transport streams, validates each label, and delivers the stream
/// to the claim slot it names. Delivery is a take-once handoff which never
/// blocks, so a stalled logical stream cannot stall acceptance — the
/// head-of-line coupling a shared reader would reintroduce is structurally
/// absent. The driver runs until the protocol completes and is then
/// dropped; a violating stream that arrives after teardown goes undetected,
/// which is detection latitude, not a safety gap — unasked replies were
/// never absorbable.
pub struct AcceptDriver<A: Acceptor> {
    acceptor: A,
    epoch: u8,
    /// The remote role, whose streams this driver routes.
    speaker: Speaker,
    slots: ClaimSlots<A::Rx>,
    route: ErrorRoute,
}

impl<A: Acceptor> AcceptDriver<A> {
    /// Bind the link's acceptor to one session's claim slots.
    pub fn new(
        acceptor: A,
        epoch: u8,
        speaker: Speaker,
        slots: ClaimSlots<A::Rx>,
        route: ErrorRoute,
    ) -> Self {
        Self {
            acceptor,
            epoch,
            speaker,
            slots,
            route,
        }
    }

    /// Accept and route incoming streams until dropped; violations are
    /// terminal, supply failures are deferred to whoever needed a stream.
    ///
    /// Never resolves successfully: session completion cancels it. A
    /// transport-level supply failure — the acceptor erroring, or a stream
    /// dying mid-label — is *not* immediately fatal: a peer that completed
    /// its session cleanly has already delivered every stream this session
    /// will claim, and may drop its link before this side finishes. The
    /// driver instead drops the undelivered claim slots and parks; a pump
    /// that provably needed one then fails the session through the error
    /// route ([`StreamError::SupplyClosed`]), while a session that needed
    /// nothing more completes on the streams it holds. The failure's own
    /// I/O detail is deliberately discarded with the parked driver: by
    /// contract the supply fails only when the link is gone, which is what
    /// `SupplyClosed` says.
    pub async fn run(mut self) -> AcceptError {
        loop {
            match self.accept_one().await {
                Ok(()) => {}
                Err(AcceptFate::Violation(error)) => return error,
                Err(AcceptFate::SupplyFailed(source)) => {
                    self.route.supply_failed(source);
                    drop(self.slots);
                    cancelled().await
                }
            }
        }
    }

    /// Accept one stream, read and validate its label, and deliver it.
    ///
    /// A peer that stalls mid-label stalls only itself: every stream this
    /// driver serves belongs to the same peer, so there is no bystander to
    /// protect with a concurrent label read.
    async fn accept_one(&mut self) -> Result<(), AcceptFate> {
        let mut rx = self
            .acceptor
            .accept()
            .await
            .map_err(AcceptFate::SupplyFailed)?;
        let mut bytes = [0u8; LABEL_LEN];
        rx.read_exact(&mut bytes)
            .await
            .map_err(AcceptFate::SupplyFailed)?;
        let [epoch, index] = bytes;
        if epoch != self.epoch {
            return Err(AcceptError::Epoch {
                origin: Origin::direction(self.speaker),
                expected: self.epoch,
                actual: epoch,
            }
            .into());
        }
        let stream = Stream::new(index).map_err(|_| AcceptError::UnknownStream {
            origin: Origin::direction(self.speaker),
            index,
        })?;
        let slot =
            self.slots.slots[usize::from(stream.index())]
                .take()
                .ok_or(AcceptError::Duplicate {
                    origin: Origin::stream(self.speaker, stream),
                })?;
        slot.send(rx).map_err(|_| {
            // The claim's consumer already finished without asking anything
            // at this level, so whatever this stream carries was never
            // asked for.
            AcceptFate::from(AcceptError::Unexpected {
                origin: Origin::stream(self.speaker, stream),
            })
        })
    }
}

/// How one acceptance attempt failed: a peer violation the session must
/// report, or a transport supply failure deferred to whoever needed it.
enum AcceptFate {
    Violation(AcceptError),
    SupplyFailed(std::io::Error),
}

impl From<AcceptError> for AcceptFate {
    fn from(error: AcceptError) -> Self {
        AcceptFate::Violation(error)
    }
}

/// An incoming transport stream violated the session's stream discipline.
#[derive(Debug, thiserror::Error)]
pub enum AcceptError {
    /// A stream was labeled with another session's epoch.
    ///
    /// Between honest peers the common cause is a link reused after a
    /// cancelled or failed session: the two ends' session counters no
    /// longer agree, so the next session's streams arrive mislabeled. Link
    /// poisoning fails such reuse fast before it reaches this diagnosis; a
    /// wrapper that reassembles [`LinkParts`](crate::link::LinkParts)
    /// without preserving its `session` state can still produce it.
    #[error("{origin}: stream labeled for session epoch {actual}, expected {expected}")]
    Epoch {
        origin: Origin,
        expected: u8,
        actual: u8,
    },
    /// A stream's label named no logical stream.
    #[error("{origin}: stream labeled with unknown stream index {index}")]
    UnknownStream { origin: Origin, index: u8 },
    /// A second stream arrived bearing an already-delivered label.
    #[error("{origin}: peer opened the logical stream twice")]
    Duplicate { origin: Origin },
    /// The peer opened a stream for a level where nothing was asked of it.
    #[error("{origin}: peer opened a logical stream that answers no question")]
    Unexpected { origin: Origin },
}

#[cfg(test)]
mod tests;
