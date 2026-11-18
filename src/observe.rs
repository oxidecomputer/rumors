//! Instrumentation to observe the wire protocol directly, primarily for
//! debugging.
//!
//! This provides hooks which can be attached to a [`Peer`](crate::Peer)
//! ([`Peer::observe`](crate::Peer::observe)) or a
//! [`Joining`](crate::Joining) builder
//! ([`Joining::observe`](crate::Joining::observe)) to watch every protocol
//! message the peer exchanges, as a stream of bytes on the wire. Because the
//! `rumors` protocol is a stream of CBOR-encoded items, such a hook can
//! be agnostic to `rumors`, and parse the stream of bytes using any CBOR decoder.
//!
//! An observer is defined in terms of hooks at three levels of granularity, and
//! every level can return `None` to skip the levels below:
//!
//! - **Peer**: each [`Observer`] is preserved indefinitely throughout the
//!   lifecycle of the [`Peer`](crate::Peer), and all [`Rumors`](crate::Rumors)
//!   clones inherit it. It is asked to produce a session handler for every
//!   gossip round the peer enters.
//! - **Session**: a [`SessionObserver`] receives the peer's elected role for
//!   this round of gossip. It is asked to produce a handler for each logical
//!   directed stream as it first carries an item in the gossip protocol.
//!   Finally, it receives the session's completion, failure category, or
//!   cancellation information.
//! - **Stream**: a [`StreamObserver`] receives that one logical directed
//!   stream's messages, in order, one CBOR item at a time.
//!
//! **Observers must never block**: Handlers run synchronously inside each
//! gossip session, and blocking will stall the session. Hand bytes off to a
//! non-blocking channel if the consumer is slow or asynchronous.
//!
//! **No inter-stream ordering is directly recorded:** Logical streams are
//! treated as concurrent by the protocol. To recover the true on-the-wire
//! interleaving of messages, use a per-session atomic counter shared by the
//! stream handlers.

use std::{
    fmt, io,
    pin::Pin,
    sync::{Arc, Mutex, PoisonError},
    task::{Context, Poll},
};

use tokio::io::{AsyncRead, ReadBuf};

use crate::{Protocol, SessionStats, Version};

/// A peer-level observation handler: yields one [`SessionObserver`] per
/// session.
///
/// Attach this to a peer using [`Peer::observe`](crate::Peer::observe) or
/// [`Joining::observe`](crate::Joining::observe). The attached handler is
/// shared by every clone of the peer's [`Rumors`](crate::Rumors) handle.
/// Because sessions run concurrently, it may be asked for session handlers from
/// concurrent tasks.
pub trait Observer: Send + Sync {
    /// Start observing one session, or return `None` to skip it.
    ///
    /// This is called once per session. The returned handler is kept alive for
    /// the duration of the session.
    ///
    /// The default implementation skips observation of the session.
    fn session(&self, session: &SessionInfo) -> Option<Box<dyn SessionObserver>> {
        let _ = session;
        None
    }
}

/// A session-level observation handler: yields one [`StreamObserver`] per
/// directed stream.
///
/// The two directions of communication in a session are processed concurrently,
/// so [`stream`](Self::stream) may be called from concurrent tasks.
pub trait SessionObserver: Send + Sync {
    /// Observe which role this side won in the session's role election.
    ///
    /// This is called at most once, when the election is decided, and is not
    /// called if the session aborts early because the two sides are already in
    /// sync.
    ///
    /// The default implementation does nothing.
    fn elected(&self, role: Role) {
        let _ = role;
    }

    /// Observe the outcome of the session.
    ///
    /// This is called exactly once after a completed or failed session, or when
    /// its future is dropped.
    ///
    /// The default implementation does nothing.
    fn finished(&self, outcome: SessionOutcome<'_>) {
        let _ = outcome;
    }

    /// Start observing one directed stream, or return `None` to skip it.
    ///
    /// This is called once per logical directed stream in the gossip protocol.
    ///
    /// A control stream's handler is kept alive until the session ends. A data
    /// stream's handler is kept alive only until this side has written (sent)
    /// or read (received) the last frame of that direction.
    ///
    /// The default implementation skips observation of the stream.
    fn stream(&self, stream: &StreamInfo) -> Option<Box<dyn StreamObserver>> {
        let _ = stream;
        None
    }
}

/// The outcome of an observed session.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionOutcome<'a> {
    /// The session completed and the remote peer confirmed it.
    Completed {
        /// The causal frontier established by the session, before any
        /// concurrent changes are joined into the replica.
        converged: &'a Version,
        /// The statistics collected during reconciliation.
        stats: SessionStats,
    },
    /// The session returned an error of this kind.
    Failed(SessionErrorKind),
    /// The future driving the session was dropped.
    Cancelled,
}

/// The public error kind reported to a [`SessionObserver`].
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionErrorKind {
    /// The transport failed or closed.
    Transport,
    /// A protocol or implementation invariant was violated.
    Protocol,
    /// The two peers advertised incompatible protocols, networks, or settings.
    Mismatch,
    /// The application's specified gossip timeout expired.
    GossipTimeout,
    /// Bookmark storage or parsing failed.
    Bookmark,
}

/// A stream-level observation handler: receives one directed stream's messages
/// in order.
pub trait StreamObserver: Send {
    /// Observe one protocol message.
    ///
    /// This method is provided exactly one CBOR item of the wire protocol, as
    /// sent or received on this handler's logical directed stream.
    ///
    /// This is invoked synchronously from the session's own tasks, after the
    /// item was handed whole to the write half (sent), which may still buffer
    /// it, or read (received).
    ///
    /// The default implementation skips observation of the message.
    fn message(&mut self, bytes: &[u8]) {
        let _ = bytes;
    }
}

/// Identifying information for a single session.
///
/// This deliberately carries no "session number": numbering is left to the
/// observer.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SessionInfo {
    /// Which lifecycle operation started the session.
    pub kind: SessionKind,
    /// The wire protocol version the session used.
    pub protocol: Protocol,
}

/// The lifecycle operation which started an observed session, as seen by the
/// local peer.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SessionKind {
    /// The local peer is joining the gossip network
    /// ([`Peer::join`](crate::Peer::join)).
    Join,
    /// The local peer is gossiping ([`Rumors::gossip`](crate::Rumors::gossip),
    /// [`Rumors::gossip_stream`](crate::Rumors::gossip_stream), and
    /// [`Rumors::gossip_once`](crate::Rumors::gossip_once)).
    Gossip,
    /// The local peer is retiring ([`Peer::retire`](crate::Peer::retire)).
    Retire,
}

/// Identifying information for a logical directed stream within a session.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StreamInfo {
    /// Which one of the session's streams this is.
    pub id: StreamId,
    /// Whether the local peer sent or received the stream's messages.
    pub direction: Direction,
}

/// The identity of a logical stream within a gossip session.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StreamId {
    /// All the items before and after data frames.
    ///
    /// This encompasses session setup, network joins and departures, and
    /// completion confirmation.
    Control,
    /// One logical reconciliation data stream.
    ///
    /// This encompasses the frames of one reconciliation level's replies in one
    /// particular direction.
    Data {
        /// The elected session role that emits this stream's frames, from the
        /// local peer's perspective.
        speaker: Role,
        /// The logical stream's index.
        index: u8,
    },
}

/// The direction of an observed stream, from the local peer's perspective.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Direction {
    /// The local peer wrote the stream's messages.
    Sent,
    /// The local peer read the stream's messages.
    Received,
}

/// One side's elected role in a single gossip session.
///
/// This is elected after the session's greetings are exchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Role {
    /// This role *asks* the "opening question" of the exchange.
    Initiator,
    /// This role *answers* the "opening question" of the exchange.
    Responder,
}

/// The wire observers a peer carries, in attachment order.
///
/// The collection follows the peer through every handle just as its replica
/// state does. Clones share the collection until a builder attaches another
/// observer.
#[derive(Clone, Default)]
pub(crate) struct Attachment {
    /// Observers called for each future session, in attachment order.
    handlers: Arc<[Arc<dyn Observer>]>,
}

/// Summarize an attachment without requiring observer implementations to be
/// debuggable.
impl fmt::Debug for Attachment {
    /// Show the number of attached observers.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Attachment")
            .field("observers", &self.handlers.len())
            .finish()
    }
}

impl Attachment {
    /// Append `observer`; later sessions visit observers in this order.
    pub(crate) fn attach(&mut self, observer: Arc<dyn Observer>) {
        let mut handlers = Vec::with_capacity(self.handlers.len() + 1);
        handlers.extend(self.handlers.iter().cloned());
        handlers.push(observer);
        self.handlers = handlers.into();
    }

    /// Enter one session: create its handle.
    ///
    /// The handle is inert — every invocation a no-op branch — when no observer
    /// accepts the session.
    pub(crate) fn begin(&self, kind: SessionKind) -> SessionHandle {
        let info = SessionInfo {
            kind,
            protocol: Protocol::V2,
        };
        let sessions: Vec<_> = self
            .handlers
            .iter()
            .filter_map(|handler| handler.session(&info))
            .collect();
        if sessions.is_empty() {
            return SessionHandle::default();
        }
        // The control stream's two directions open with the session
        // itself: create both handlers now, ahead of the preamble.
        let sent = stream_handlers(
            &sessions,
            &StreamInfo {
                id: StreamId::Control,
                direction: Direction::Sent,
            },
        );
        let received = stream_handlers(
            &sessions,
            &StreamInfo {
                id: StreamId::Control,
                direction: Direction::Received,
            },
        );
        SessionHandle {
            inner: Some(Arc::new(HandleInner {
                sessions,
                control_sent: Mutex::new(sent),
                control_received: Mutex::new(received),
            })),
        }
    }
}

/// One session's observation handle: cheap to clone, inert when no handler
/// observes the session.
///
/// The session machinery threads a clone to every layer that emits or accepts
/// wire items (the pattern the stats recorder set). The pipe's frame reader and
/// writer create the data streams' own [`StreamObserver`]s through
/// [`data`](Self::data); the control stream's two handlers live here, behind
/// mutexes, because the control items are written from several protocol layers
/// in sequence — the locks are uncontended by construction (each direction's
/// items are protocol-ordered) and absent entirely from the unattached path.
#[derive(Clone, Default)]
pub(crate) struct SessionHandle {
    inner: Option<Arc<HandleInner>>,
}

/// The session handlers and control-stream handlers shared by handle clones.
struct HandleInner {
    /// Session handlers called in their observers' attachment order.
    sessions: Vec<Box<dyn SessionObserver>>,
    /// Handlers for control messages this side sends.
    control_sent: Mutex<Vec<Box<dyn StreamObserver>>>,
    /// Handlers for control messages this side receives.
    control_received: Mutex<Vec<Box<dyn StreamObserver>>>,
}

impl SessionHandle {
    /// Whether any handler observes this session.
    pub(crate) fn attached(&self) -> bool {
        self.inner.is_some()
    }

    /// Observe one item sent on the control stream.
    pub(crate) fn control_sent(&self, bytes: &[u8]) {
        if let Some(inner) = &self.inner {
            observe_control(&inner.control_sent, bytes);
        }
    }

    /// Observe one item received on the control stream.
    pub(crate) fn control_received(&self, bytes: &[u8]) {
        if let Some(inner) = &self.inner {
            observe_control(&inner.control_received, bytes);
        }
    }

    /// Report the session's decided role election.
    pub(crate) fn elected(&self, role: Role) {
        if let Some(inner) = &self.inner {
            for session in &inner.sessions {
                session.elected(role);
            }
        }
    }

    /// Report one terminal outcome to every session handler.
    fn finished(&self, outcome: SessionOutcome<'_>) {
        if let Some(inner) = &self.inner {
            for session in &inner.sessions {
                session.finished(outcome);
            }
        }
    }

    /// Create the observer table for one direction of the session's data
    /// frames, or `None` when no handler observes the session.
    ///
    /// The table asks the session handlers for each logical stream's handler
    /// when that stream first carries a frame, so a logical stream the session
    /// never speaks yields no handler.
    pub(crate) fn data(&self, speaker: Role, direction: Direction) -> Option<DataObservers> {
        self.inner.as_ref()?;
        Some(DataObservers {
            handle: self.clone(),
            speaker,
            direction,
            streams: Vec::new(),
        })
    }

    /// Create the handlers for one logical data stream, if any session
    /// observers want it.
    fn data_stream(
        &self,
        speaker: Role,
        index: u8,
        direction: Direction,
    ) -> Option<Box<dyn StreamObserver>> {
        let inner = self.inner.as_ref()?;
        let handlers = stream_handlers(
            &inner.sessions,
            &StreamInfo {
                id: StreamId::Data { speaker, index },
                direction,
            },
        );
        match handlers.len() {
            0 => None,
            1 => handlers.into_iter().next(),
            _ => Some(Box::new(StreamFanout(handlers))),
        }
    }
}

/// The data-stream handlers of one pipe direction, indexed by logical stream.
///
/// The pipe carries every logical stream of a direction, so one frame reader or
/// writer serves them all. Each logical stream's handler is created when that
/// stream carries its first frame, and lives as long as the table.
pub(crate) struct DataObservers {
    /// The session whose handlers are asked for each stream's handler.
    handle: SessionHandle,
    /// The elected role speaking this direction's frames.
    speaker: Role,
    /// Whether this side sent or received the frames.
    direction: Direction,
    /// Each logical stream's handler once asked for: the outer `None` means not
    /// yet asked, the inner `None` that every handler declined.
    streams: Vec<Option<Option<Box<dyn StreamObserver>>>>,
}

impl DataObservers {
    /// Deliver one frame's wire bytes to its logical stream's handler, creating
    /// the handler on the stream's first frame.
    pub(crate) fn message(&mut self, index: u8, bytes: &[u8]) {
        let slot = usize::from(index);
        if self.streams.len() <= slot {
            self.streams.resize_with(slot + 1, || None);
        }
        let handler = self.streams[slot]
            .get_or_insert_with(|| self.handle.data_stream(self.speaker, index, self.direction));
        if let Some(handler) = handler {
            handler.message(bytes);
        }
    }
}

/// Own one session's terminal callback and report cancellation on drop.
pub(crate) struct SessionCompletion {
    /// The shared callbacks used by the session machinery.
    handle: SessionHandle,
    /// Whether an explicit outcome has already been reported.
    finished: bool,
}

impl SessionCompletion {
    /// Begin terminal-outcome ownership for `handle`.
    pub(crate) fn new(handle: SessionHandle) -> Self {
        Self {
            handle,
            finished: false,
        }
    }

    /// Clone the handle threaded through the session's protocol layers.
    pub(crate) fn handle(&self) -> SessionHandle {
        self.handle.clone()
    }

    /// Report the session's returned outcome exactly once.
    pub(crate) fn finish(mut self, outcome: SessionOutcome<'_>) {
        // Set this before invoking user code so unwinding cannot report a
        // second, contradictory cancellation outcome.
        self.finished = true;
        self.handle.finished(outcome);
    }
}

/// Report cancellation when the future that owns a session is dropped.
impl Drop for SessionCompletion {
    fn drop(&mut self) {
        if !self.finished {
            self.finished = true;
            self.handle.finished(SessionOutcome::Cancelled);
        }
    }
}

/// Ask every session handler about one stream, preserving attachment order.
fn stream_handlers(
    sessions: &[Box<dyn SessionObserver>],
    info: &StreamInfo,
) -> Vec<Box<dyn StreamObserver>> {
    sessions
        .iter()
        .filter_map(|session| session.stream(info))
        .collect()
}

/// Deliver one stream's messages to multiple observers in attachment order.
struct StreamFanout(Vec<Box<dyn StreamObserver>>);

/// Forward every message to each attached stream handler.
impl StreamObserver for StreamFanout {
    /// Forward one message to every handler.
    fn message(&mut self, bytes: &[u8]) {
        for handler in &mut self.0 {
            handler.message(bytes);
        }
    }
}

/// Invoke one control-direction handler under its lock.
///
/// A poisoned lock means an earlier invocation panicked (an application
/// handler's panic, already propagating through the session); keep delivering
/// to the handlers rather than silently dropping the direction.
fn observe_control(slot: &Mutex<Vec<Box<dyn StreamObserver>>>, bytes: &[u8]) {
    let mut guard = slot.lock().unwrap_or_else(PoisonError::into_inner);
    for observer in guard.iter_mut() {
        observer.message(bytes);
    }
}

/// A reader adapter that retains a copy of every delivered byte, so an exact
/// item-shaped read (a frame, a greeting, a hand-off) can hand its observer the
/// item's true wire bytes rather than a re-encoding.
pub(crate) struct CaptureRead<'a, R: ?Sized> {
    captured: Vec<u8>,
    inner: &'a mut R,
}

impl<'a, R: ?Sized> CaptureRead<'a, R> {
    /// Capture everything the wrapped reader delivers from here on.
    pub(crate) fn new(inner: &'a mut R) -> Self {
        Self {
            captured: Vec::new(),
            inner,
        }
    }

    /// The bytes delivered through this adapter so far.
    pub(crate) fn bytes(&self) -> &[u8] {
        &self.captured
    }
}

/// Forward reads while retaining the bytes delivered through them.
impl<R> AsyncRead for CaptureRead<'_, R>
where
    R: AsyncRead + Unpin + ?Sized,
{
    /// Poll the wrapped reader and append newly delivered bytes to the capture.
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        let before = buf.filled().len();
        let poll = Pin::new(&mut *this.inner).poll_read(cx, buf);
        if let Poll::Ready(Ok(())) = &poll {
            this.captured.extend_from_slice(&buf.filled()[before..]);
        }
        poll
    }
}

#[cfg(test)]
mod tests;
