//! Lazily established, independently flow-controlled logical streams.
//!
//! This layer binds the protocol's 17-per-direction logical streams onto a
//! [`Link`](crate::link)'s transport streams, one to one. Nothing multiplexes:
//! each logical stream owns its transport stream outright, so backpressure on
//! one stream is invisible to every other — the independence the capacity-one
//! channel arguments of the materialized walk rely on is supplied by the
//! [link contract](crate::link), not reconstructed here.
//!
//! Streams are established lazily, on both sides, from the same shared
//! fact: a stream carries answers to questions its receiver asked — or,
//! for the opening-supply stream, to the root-level requests the two
//! exchanged listings prove are coming — and each side learns whether any
//! such question exists at a level before it touches the stream. A
//! [`StreamSender`] connects on its first frame — a level that produces no
//! reply never opens its stream — and a [`StreamReceiver`] claims its
//! accepted stream on its first read — a level that asks no question never
//! claims one. Empty streams therefore never exist on the wire, rather
//! than opening only to say so.
//!
//! Because transport streams arrive anonymously and in any order, the sender
//! labels each opened stream with the session epoch and its logical stream
//! index before the first frame. The session's [`AcceptDriver`] — the sole
//! reader of the link's acceptor — validates each label and delivers the
//! stream to the one claim slot it names. Every frame's signal then
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

use std::pin::Pin;
use std::task::{Context, Poll};

use async_stream::stream;
use futures::{StreamExt, stream::BoxStream};
use tokio::{
    io::AsyncWriteExt,
    sync::{mpsc, oneshot},
};

use crate::link::{Acceptor, Connector, Done};
use crate::observe::{Direction, SessionHandle};
use crate::tree::mirror::streaming::stats::{CountedRead, CountedWrite, Recorder};
use crate::tree::mirror::streaming::tasks::cancelled;

use super::codec::{
    DecodeError, EncodeError, End, Frame, FrameRead, FrameWrite, Origin, RunBudget, Speaker, Stream,
};

/// Render the label naming one opened stream: two CBOR unsigned-int
/// items, the session epoch then the stream index.
///
/// The canonical definition of the wire label's spelling: the capture
/// harnesses parse labels through the same head grammar.
fn label(epoch: u8, stream: Stream) -> Vec<u8> {
    use crate::tree::mirror::cbor::{self, MAJOR_UINT};
    let mut label = Vec::with_capacity(cbor::head_len(u64::from(epoch)) + 1);
    cbor::write_head(&mut label, MAJOR_UINT, u64::from(epoch));
    cbor::write_head(&mut label, MAJOR_UINT, u64::from(stream.index()));
    label
}

/// Number of logical streams per direction, as an array dimension.
const STREAM_COUNT: usize = Stream::COUNT as usize;

/// A protocol reply frame, statically excluding stream-end transport control.
///
/// Stream end is a lifecycle event owned by [`StreamSender::finish`]; a
/// producer cannot smuggle one into the middle of its replies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplyFrame(Frame);

impl TryFrom<Frame> for ReplyFrame {
    type Error = ReplyFrameError;

    /// Check that a general wire frame belongs to a protocol reply.
    fn try_from(frame: Frame) -> Result<Self, Self::Error> {
        if matches!(frame, Frame::End(End::Stream)) {
            Err(ReplyFrameError::StreamEnd)
        } else {
            Ok(Self(frame))
        }
    }
}

impl From<ReplyFrame> for Frame {
    /// Recover the general wire frame for transport encoding.
    fn from(frame: ReplyFrame) -> Self {
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
pub struct StreamSender<C: Connector> {
    connector: C,
    epoch: u8,
    /// The local role whose direction this stream carries.
    speaker: Speaker,
    stream: Stream,
    /// The session's stats recorder: the opened stream's codec writes
    /// count as [`bytes_sent`](crate::SessionStats::bytes_sent), the
    /// label excluded (it is written before the counted wrapper wraps).
    stats: Recorder,
    /// The session's observation handle: the stream creates its own
    /// observer from it when it opens, so a sender that never carries
    /// a frame observes nothing.
    observe: SessionHandle,
    state: SendState<C::Tx>,
}

enum SendState<Tx> {
    Unopened,
    Open(FrameWrite<CountedWrite<Tx>>, Done<Tx>),
}

impl<C: Connector> StreamSender<C> {
    /// Bind one outgoing logical stream to a link's stream supply.
    pub fn new(
        connector: C,
        epoch: u8,
        speaker: Speaker,
        stream: Stream,
        stats: Recorder,
        observe: SessionHandle,
    ) -> Self {
        Self {
            connector,
            epoch,
            speaker,
            stream,
            stats,
            observe,
            state: SendState::Unopened,
        }
    }

    /// Write and flush one reply frame, opening the stream on the first.
    ///
    /// # Cancel safety
    ///
    /// Not cancel safe. Dropping the first `frame` future between opening
    /// the transport stream and completing the label write leaves the peer
    /// reading a truncated label: its accept driver classifies the short
    /// read as a failed stream supply and drops every undelivered claim
    /// slot, so all later levels on the peer fail as supply-closed — a
    /// session-wide severance misattributed to the supply rather than to
    /// the cancellation. Retain the future until it resolves, or treat the
    /// session as severed after cancelling it.
    pub async fn frame(&mut self, frame: ReplyFrame) -> Result<(), SendError> {
        self.write(frame.into()).await
    }

    /// End this logical stream after all of its replies, if it ever opened.
    ///
    /// The explicit end control distinguishes a completed stream from one
    /// truncated mid-reply. The transport half is handed to its [`Done`]
    /// right behind it, resting at the frame boundary; failure paths drop
    /// the half instead, the contract's abort.
    pub async fn finish(mut self) -> Result<(), SendError> {
        match self.state {
            SendState::Unopened => Ok(()),
            SendState::Open(..) => {
                self.write(Frame::End(End::Stream)).await?;
                let SendState::Open(write, done) =
                    std::mem::replace(&mut self.state, SendState::Unopened)
                else {
                    unreachable!("the open state was just written through");
                };
                done.complete(write.into_inner().into_inner());
                Ok(())
            }
        }
    }

    /// Write one frame through the open transport stream, opening it first.
    async fn write(&mut self, frame: Frame) -> Result<(), SendError> {
        let stream = self.stream;
        let write = match &mut self.state {
            SendState::Open(write, _) => write,
            state @ SendState::Unopened => {
                let (mut tx, done) =
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
                *state = SendState::Open(
                    FrameWrite::new(self.speaker, CountedWrite::new(tx, self.stats.clone()))
                        .observed(self.observe.data(
                            self.speaker.role(),
                            stream.index(),
                            Direction::Sent,
                        )),
                    done,
                );
                let SendState::Open(write, _) = state else {
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
#[non_exhaustive]
pub enum StreamError {
    /// The incoming frame codec rejected bytes or the transport failed.
    #[error(transparent)]
    Decode(#[from] DecodeError),
    /// A frame's signal named a stream other than the claimed label.
    #[error(
        "{origin}: stream labeled {} carried a frame for {}",
        labeled.index(),
        framed.index()
    )]
    Mislabeled {
        origin: Origin,
        labeled: Stream,
        framed: Stream,
    },
    /// The transport stream ended before its explicit end control.
    #[error("{origin}: transport stream ended before its end control")]
    Truncated { origin: Origin },
    /// The stream supply failed before an awaited stream was delivered.
    ///
    /// `source` carries the supply's own transport failure when the session
    /// observed one; a session reports it exactly once, on the error the
    /// session surfaces as its cause. `None` means the supply closed
    /// without an observed transport failure.
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
pub struct StreamReceiver<Rx> {
    /// The claim and identity, consumed to build `frames` on first poll.
    start: Option<ReceiverStart<Rx>>,
    /// `Some` exactly once the stream has been claimed: the first poll
    /// builds it, and [`finish`](Self::finish) reads its absence as "this
    /// level was never needed".
    frames: Option<BoxStream<'static, Frame>>,
}

struct ReceiverStart<Rx> {
    claim: oneshot::Receiver<(Rx, Done<Rx>)>,
    /// The remote role whose direction this stream carries.
    speaker: Speaker,
    stream: Stream,
    /// The session's run budget, enforced by the claimed stream's codec on
    /// every supply frame the peer delivers.
    budget: RunBudget,
    route: ErrorRoute,
    /// The session's stats recorder: the claimed stream's codec reads
    /// count as [`bytes_received`](crate::SessionStats::bytes_received),
    /// the label excluded (the accept driver consumed it before
    /// delivery).
    stats: Recorder,
    /// The session's observation handle: the stream creates its own
    /// observer from it when its claim resolves, so a stream that is
    /// never claimed observes nothing.
    observe: SessionHandle,
}

impl<Rx> StreamReceiver<Rx>
where
    Rx: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    /// Bind one incoming logical stream to its claim slot.
    pub fn new(
        claim: oneshot::Receiver<(Rx, Done<Rx>)>,
        speaker: Speaker,
        stream: Stream,
        budget: RunBudget,
        route: ErrorRoute,
        stats: Recorder,
        observe: SessionHandle,
    ) -> Self {
        Self {
            start: Some(ReceiverStart {
                claim,
                speaker,
                stream,
                budget,
                route,
                stats,
                observe,
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
    fn frames(&mut self) -> &mut BoxStream<'static, Frame> {
        let start = &mut self.start;
        self.frames.get_or_insert_with(|| {
            let ReceiverStart {
                claim,
                speaker,
                stream,
                budget,
                route,
                stats,
                observe,
            } = start
                .take()
                .expect("the start state is consumed exactly once");
            Box::pin(read_frames(
                claim, speaker, stream, budget, route, stats, observe,
            ))
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

impl<Rx> futures::Stream for StreamReceiver<Rx>
where
    Rx: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    type Item = Frame;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.get_mut().frames().as_mut().poll_next(cx)
    }
}

/// Claim the transport stream, then decode frames until the end control.
///
/// Every failure path publishes to the session error route and parks: the
/// consumer never observes a truncated stream as a clean end.
#[allow(clippy::too_many_arguments)]
fn read_frames<Rx>(
    claim: oneshot::Receiver<(Rx, Done<Rx>)>,
    speaker: Speaker,
    stream: Stream,
    budget: RunBudget,
    route: ErrorRoute,
    stats: Recorder,
    observe: SessionHandle,
) -> impl futures::Stream<Item = Frame> + Send
where
    Rx: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    stream! {
        let Ok((rx, done)) = claim.await else {
            // The claim slot is gone: the link's stream supply failed before
            // the peer's stream for this level arrived. This is the one
            // consumer that provably needed it, so the report comes from
            // here; a supply failure that nothing was waiting on lets the
            // session finish on the streams it already holds. The supply's
            // own I/O failure is not attached here: it stays deposited for
            // the session terminal, which attaches it to whichever error
            // wins selection — a report that loses the terminal's race must
            // not strand the causal transport error.
            route.report(StreamError::SupplyClosed {
                origin: Origin::stream(speaker, stream),
                source: None,
            });
            cancelled().await
        };
        let mut read = FrameRead::new(speaker, budget, CountedRead::new(rx, stats)).observed(
            observe.data(speaker.role(), stream.index(), Direction::Received),
        );
        loop {
            let frame = match read.frame().await {
                Ok(Some((framed, frame))) if framed == stream => frame,
                Ok(Some((framed, _))) => {
                    route.report(StreamError::Mislabeled {
                        origin: Origin::stream(speaker, stream),
                        labeled: stream,
                        framed,
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
                // only complete replies followed by a clean end.
                break;
            }
            yield frame;
        }
        // The end control is exactly where the data ends, so the transport
        // half rests at the link contract's clean boundary: hand it back
        // there, never reading past it. The hand-back is a framing
        // judgment, not a protocol one: a stream the consumer goes on to
        // rule invalid has still completed here. Every failure path above
        // parks instead, dropping the half at teardown, which is the
        // contract's abort.
        done.complete(read.into_inner().into_inner());
    }
}

/// The reporting half of the session's one-slot first-error route.
#[derive(Clone)]
pub struct ErrorRoute {
    send: mpsc::Sender<StreamError>,
    /// The parked accept driver's deposited transport failure.
    ///
    /// Reporters never read it: the session terminal is the slot's sole
    /// consumer ([`FirstStreamError::take_supply_failure`]), so the causal
    /// I/O error cannot be stranded on a report that loses the terminal's
    /// selection.
    supply_failure: std::sync::Arc<std::sync::Mutex<Option<std::io::Error>>>,
}

impl ErrorRoute {
    /// Publish the first incoming-stream error without blocking its reporter.
    ///
    /// A report that loses the one-slot race is dropped as cascade. No
    /// causal detail is lost with it: reports never carry the supply
    /// deposit (the session terminal is the slot's sole consumer), so a
    /// dropped report forfeits only its stream-granularity origin.
    fn report(&self, error: StreamError) {
        let _ = self.send.try_send(error);
    }

    /// Deposit the supply's transport failure for the session terminal.
    fn supply_failed(&self, source: std::io::Error) {
        let mut slot = self.supply_failure.lock().expect("supply failure lock");
        slot.get_or_insert(source);
    }
}

/// The observing half of the session's first-error route.
pub struct FirstStreamError {
    receive: mpsc::Receiver<StreamError>,
    /// The slot the accept driver deposits the supply's transport failure
    /// into; the session terminal is its sole consumer.
    supply_failure: std::sync::Arc<std::sync::Mutex<Option<std::io::Error>>>,
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

    /// Claim the deposited supply failure as the session's cause.
    ///
    /// A deposit precedes every error the dead supply goes on to cause,
    /// and nothing but the session terminal consumes the slot, so an
    /// error selected beside a deposit is the *symptom* of the dead
    /// supply: the terminal reports the deposit as the session's cause.
    pub fn take_supply_failure(&self) -> Option<std::io::Error> {
        self.supply_failure
            .lock()
            .expect("supply failure lock")
            .take()
    }

    /// Recover a queued [`StreamError::SupplyClosed`] the terminal's biased
    /// poll order never received, attaching the deposited cause (reports
    /// leave the deposit in its slot for the terminal to claim).
    ///
    /// When the protocol arm resolves first with a symptom of the dead
    /// supply (a write to a peer that already tore down), the causal report
    /// can be sitting unreceived in the route. Any *other* queued error is
    /// discarded here: it lost to the protocol error by the terminal's
    /// deliberate poll order, exactly as if the select had resolved the
    /// protocol arm alone.
    pub fn queued_supply_closed(&mut self) -> Option<StreamError> {
        while let Ok(error) = self.receive.try_recv() {
            if let StreamError::SupplyClosed { origin, source } = error {
                let source = source.or_else(|| self.take_supply_failure());
                return Some(StreamError::SupplyClosed { origin, source });
            }
        }
        None
    }
}

/// One slot: the route keeps the first error and drops the rest, because
/// the first failure is the session's cause and later ones its cascade.
const ERROR_ROUTE_CAPACITY: usize = 1;

/// Allocate the session's incoming-stream error route.
pub fn error_route() -> (ErrorRoute, FirstStreamError) {
    let (send, receive) = mpsc::channel(ERROR_ROUTE_CAPACITY);
    let supply_failure = std::sync::Arc::new(std::sync::Mutex::new(None));
    (
        ErrorRoute {
            send,
            supply_failure: supply_failure.clone(),
        },
        FirstStreamError {
            receive,
            supply_failure,
        },
    )
}

/// The claim slots the accept driver delivers incoming streams into.
pub struct ClaimSlots<Rx> {
    slots: [Option<oneshot::Sender<(Rx, Done<Rx>)>>; STREAM_COUNT],
}

/// The claim receivers the session's typed states take streams from.
pub struct Claims<Rx> {
    slots: [Option<oneshot::Receiver<(Rx, Done<Rx>)>>; STREAM_COUNT],
}

impl<Rx> Claims<Rx> {
    /// Take the sole claim for `stream`.
    pub fn take(&mut self, stream: Stream) -> oneshot::Receiver<(Rx, Done<Rx>)> {
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
/// to the claim slot it names.
///
/// Delivery is a take-once handoff which never blocks, so a stalled
/// logical stream cannot stall acceptance — the head-of-line coupling a
/// shared reader would reintroduce is structurally absent. The driver runs
/// until the protocol completes and is then dropped.
///
/// An unasked stream with a valid label meets one of two fates, neither
/// prompt. Delivered while the slot's claim receiver is alive but never
/// polled, it is never detected at all: it parks in the slot until session
/// teardown drops slot and stream together, silently. Delivered after the
/// receiver is already gone, it fires [`AcceptError::Unexpected`], late in
/// the termination cascade. This is detection latitude, not a safety gap:
/// unasked replies are never absorbable either way, and a parked stream's
/// memory is bounded by its own link stream's buffers.
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
    /// I/O detail is deposited in the error route before the driver parks;
    /// the session terminal claims it at selection and reports it as the
    /// session's cause, so the causal transport error survives the
    /// deferral no matter which racing symptom wins the terminal's select.
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
        let (mut rx, done) = self
            .acceptor
            .accept()
            .await
            .map_err(AcceptFate::SupplyFailed)?;
        let epoch = label_item(self.speaker, &mut rx).await?;
        if epoch != u64::from(self.epoch) {
            return Err(AcceptError::Epoch {
                origin: Origin::direction(self.speaker),
                expected: self.epoch,
                actual: epoch,
            }
            .into());
        }
        let index = label_item(self.speaker, &mut rx).await?;
        let stream = u8::try_from(index)
            .ok()
            .and_then(|index| Stream::new(index).ok())
            .ok_or(AcceptError::UnknownStream {
                origin: Origin::direction(self.speaker),
                index,
            })?;
        let slot =
            self.slots.slots[usize::from(stream.index())]
                .take()
                .ok_or(AcceptError::Duplicate {
                    origin: Origin::stream(self.speaker, stream),
                })?;
        slot.send((rx, done)).map_err(|_| {
            // The claim's consumer already finished without asking anything
            // at this level, so whatever this stream carries was never
            // asked for.
            AcceptFate::from(AcceptError::Unexpected {
                origin: Origin::stream(self.speaker, stream),
            })
        })
    }
}

/// Read one label item: a canonical unsigned int. A transport failure
/// (a close mid-label included) defers to whoever needed the stream; a
/// present-but-malformed item is the peer's violation.
async fn label_item(
    speaker: Speaker,
    rx: &mut (impl tokio::io::AsyncRead + Unpin),
) -> Result<u64, AcceptFate> {
    use crate::tree::mirror::cbor;
    match cbor::read_head_async(rx).await {
        Ok(Some(head)) if head.major == cbor::MAJOR_UINT => Ok(head.value),
        Ok(Some(_)) => Err(AcceptError::Label {
            origin: Origin::direction(speaker),
            detail: "label item is not an unsigned int",
        }
        .into()),
        Ok(None) => Err(AcceptFate::SupplyFailed(
            std::io::ErrorKind::UnexpectedEof.into(),
        )),
        Err(cbor::HeadReadError::Io(io)) => Err(AcceptFate::SupplyFailed(io)),
        Err(cbor::HeadReadError::Malformed(_)) => Err(AcceptError::Label {
            origin: Origin::direction(speaker),
            detail: "label head is not canonical",
        }
        .into()),
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
#[non_exhaustive]
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
        actual: u64,
    },
    /// A stream's label named no logical stream.
    #[error("{origin}: stream labeled with unknown stream index {index}")]
    UnknownStream { origin: Origin, index: u64 },
    /// A stream's label was not a pair of canonical unsigned-int items.
    #[error("{origin}: stream label is malformed: {detail}")]
    Label {
        origin: Origin,
        detail: &'static str,
    },
    /// A second stream arrived bearing an already-delivered label.
    #[error("{origin}: peer opened the logical stream twice")]
    Duplicate { origin: Origin },
    /// The peer opened a stream for a level where nothing was asked of it.
    #[error("{origin}: peer opened a logical stream that answers no question")]
    Unexpected { origin: Origin },
}

#[cfg(test)]
mod tests;
