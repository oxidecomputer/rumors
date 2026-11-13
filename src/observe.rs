//! Instrument gossip sessions to observe the CBOR-encoded bytes on the wire.
//!
//! The hook a debugger, session recorder, or tracing adapter attaches to a
//! [`Peer`](crate::Peer) ([`Peer::observe`](crate::Peer::observe)) or a
//! [`Bootstrap`](crate::Bootstrap) builder
//! ([`Bootstrap::observe`](crate::Bootstrap::observe)) to watch every protocol
//! message the peer exchanges, as raw wire bytes. The hook is rumors-blind: no
//! protocol type appears in its signature, each invocation carries exactly one
//! whole CBOR item with its stream identity, and a consumer parses with any
//! CBOR library — or none.
//!
//! Attachment has three levels. Each peer observer supplies its own handlers
//! for the levels below; every level can return `None` to skip what it does not
//! care about:
//!
//! - **Peer**: each [`Observer`] follows the peer through cloning, bookmarking,
//!   and reunion, and is asked for a session handler for every session the peer
//!   enters — gossip, bootstrap, and retire alike. Repeated attachment adds
//!   observers; callbacks visit them in attachment order.
//! - **Session**: a [`SessionObserver`] receives the elected role, is asked for
//!   a handler for each directed stream as it first carries an item, and
//!   finally receives the session's completion, failure category, or
//!   cancellation.
//! - **Stream**: a [`StreamObserver`] receives that one directed stream's
//!   messages, in stream order, one CBOR item per
//!   [`message`](StreamObserver::message) call.
//!
//! The contract:
//!
//! - **Ordering**: within one directed stream, invocations arrive in the
//!   stream's order; across streams no ordering is promised (the session's two
//!   directions progress concurrently). To recover the observed interleaving,
//!   stamp each message from a session-scoped atomic counter shared by the
//!   stream handlers. [`SessionObserver::finished`] is the final callback for a
//!   session, after every stream handler's last message.
//! - **Never block**: handlers run synchronously inside the session's own
//!   tasks. Blocking in [`message`](StreamObserver::message) stalls the
//!   session's connection in that direction, every logical stream of it at
//!   once, and waiting on protocol progress deadlocks; hand bytes off to a
//!   channel if the consumer is slow.
//! - **Coverage**: every item of a session, both directions, control items and
//!   data frames alike. One connection carries them all: the control items,
//!   every item before and after the data frames, form the
//!   [`Control`](StreamId::Control) stream, and each logical stream's frames
//!   form one [`Data`](StreamId::Data) stream. Only complete items are
//!   observed: a session that dies mid-frame does not deliver the fragment, and
//!   an aborted session may have observed fewer items than crossed the wire. A
//!   sent frame is observed once the session has written it whole, into its own
//!   write buffer or to the write half, which may buffer it in turn, so an
//!   aborted session may also have observed a sent frame that never reached the
//!   peer.
//! - **Cost**: unattached (or a level declined), one branch per frame;
//!   attached, one extra contiguous copy of each observed frame. The wire bytes
//!   themselves are unchanged either way.
//!
//! The hook watches the **wire**, synchronously, from inside the session's
//! tasks; the content observers
//! ([`UnorderedMessages`](crate::UnorderedMessages),
//! [`CausalMessages`](crate::CausalMessages), and [`Changes`](crate::Changes))
//! watch the **set**, asynchronously, from outside.

use std::{
    fmt, io,
    pin::Pin,
    sync::{Arc, Mutex, PoisonError},
    task::{Context, Poll},
};

use tokio::io::{AsyncRead, ReadBuf};

use crate::{Protocol, SessionStats, Version};

/// One peer-level observation handler, yielding one [`SessionObserver`] per
/// session the peer enters.
///
/// Attach with [`Peer::observe`](crate::Peer::observe) or
/// [`Bootstrap::observe`](crate::Bootstrap::observe). The handler is shared by
/// every clone of the peer's [`Rumors`](crate::Rumors) handle, and sessions run
/// concurrently, so it is asked for session handlers from concurrent tasks.
pub trait Observer: Send + Sync {
    /// Begin observing one session, or return `None` to skip it.
    ///
    /// Called once per session, before this side writes its first byte or
    /// delivers any item to a handler. The returned handler lives for the
    /// session.
    fn session(&self, session: &SessionInfo) -> Option<Box<dyn SessionObserver>>;
}

/// A session-level observation handler: yields one [`StreamObserver`] per
/// directed stream, as each first carries an item.
///
/// A session's two directions progress concurrently, so
/// [`stream`](Self::stream) is called from concurrent tasks.
pub trait SessionObserver: Send + Sync {
    /// Learn which role this side won in the session's role election.
    ///
    /// Called at most once, when the election is decided — after the greetings
    /// are exchanged and before any data frame. Not called when the greetings
    /// carried equal versions (no election happens: the session ends with its
    /// control items alone). The role is not part of [`SessionInfo`] because it
    /// does not exist yet when the session begins; a consumer that needs it
    /// before data frames arrive records it here. The default does nothing.
    fn elected(&self, role: Role) {
        let _ = role;
    }

    /// Observe how the session ended.
    ///
    /// Called exactly once after a completed or failed session, or when its
    /// future is dropped. A successful outcome borrows the frontier and copies
    /// the small statistics record only for this synchronous call. The default
    /// does nothing.
    fn finished(&self, outcome: SessionOutcome<'_>) {
        let _ = outcome;
    }

    /// Begin observing one directed stream, or return `None` to skip it.
    ///
    /// Called once per directed stream: for the control stream's two directions
    /// at session start, and for each logical data stream when this side first
    /// writes (sent) or first reads (received) one of its frames. A data stream
    /// the session never speaks yields no handler.
    ///
    /// A control stream's handler lives until the session ends. A data stream's
    /// handler lives only until this side has written (sent) or read (received)
    /// the last frame of that direction, which happens before the closing items
    /// and before [`finished`](Self::finished).
    fn stream(&self, stream: &StreamInfo) -> Option<Box<dyn StreamObserver>>;
}

/// How an observed session ended.
///
/// The callback receives this after all preceding callbacks for the session. It
/// is a diagnostic summary: methods that drove the session still return the
/// full success value or error.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionOutcome<'a> {
    /// The session completed and the peer confirmed completion.
    Completed {
        /// The causal frontier established by the session, before concurrent
        /// local changes are joined into the replica.
        converged: &'a Version,
        /// The work measured during reconciliation.
        stats: SessionStats,
    },
    /// The session returned an error of this category.
    Failed(SessionErrorKind),
    /// The future driving the session was dropped before it returned.
    Cancelled,
}

/// The public error category reported to a [`SessionObserver`].
///
/// The session method retains the detailed error, including transport and
/// protocol context. This copyable category lets observers label telemetry
/// without depending on a bookmark's error type.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionErrorKind {
    /// The transport failed or closed before an operation completed.
    Transport,
    /// A protocol or implementation invariant failed.
    Protocol,
    /// The peers advertised incompatible protocols, networks, or settings.
    Mismatch,
    /// The application's session deadline expired.
    DeadlineExceeded,
    /// Bookmark storage or decoding failed.
    Bookmark,
}

/// A stream-level observation handler: receives one directed stream's messages,
/// in stream order.
pub trait StreamObserver: Send {
    /// Observe one protocol message: exactly one CBOR item of the wire, as sent
    /// or received on this handler's directed stream.
    ///
    /// Invoked synchronously from the session's own tasks, after the item was
    /// handed whole to the write half (sent), which may still buffer it, or
    /// read (received). A data stream's handler runs in the task that writes or
    /// reads every frame of that direction, so blocking here stalls the whole
    /// direction, not only this stream; see the module docs' never-block rule.
    fn message(&mut self, bytes: &[u8]);
}

/// What identifies one observed session.
///
/// Deliberately carries no session number: numbering is the observer's own
/// concern, exactly like message interleaving (see the module docs' ordering
/// section). An [`Observer`] that wants "the peer's Nth observed session"
/// counts inside its own [`session`](Observer::session) — the method is
/// `&self`, so it synchronizes internally (an `AtomicU64` suffices), and the
/// count means precisely what that observer defines it to mean.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SessionInfo {
    /// Which lifecycle operation entered the session.
    pub kind: SessionKind,
    /// The wire dialect the session speaks.
    pub protocol: Protocol,
}

/// The lifecycle operation that entered an observed session, on this side.
///
/// The counterparty's role in the same session may differ: a peer serving a
/// bootstrap or absorbing a retirement observes an ordinary
/// [`Gossip`](Self::Gossip) session, and learns what the remote wants from the
/// remote's preamble — which its control-stream handler sees as bytes.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SessionKind {
    /// This side is joining the universe
    /// ([`Bootstrap::join`](crate::Bootstrap::join)).
    Bootstrap,
    /// This side is gossiping ([`Rumors::gossip`](crate::Rumors::gossip),
    /// [`Rumors::gossip_stream`](crate::Rumors::gossip_stream), and
    /// [`Rumors::gossip_once`](crate::Rumors::gossip_once)).
    Gossip,
    /// This side is retiring ([`Peer::retire`](crate::Peer::retire)).
    Retire,
}

/// What identifies one observed directed stream.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StreamInfo {
    /// Which of the session's streams this is.
    pub id: StreamId,
    /// Whether this side sent or received the stream's messages.
    pub direction: Direction,
}

/// One session stream's identity.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StreamId {
    /// Session setup, network joins and departures, and completion
    /// confirmation: the items before and after the data frames.
    Control,
    /// One logical reconciliation data stream: the frames of one level's
    /// replies in one direction.
    Data {
        /// The elected role that speaks this stream's frames.
        ///
        /// Sent data streams are spoken by this side's elected role; received
        /// ones by the counterparty's.
        speaker: Role,
        /// The logical stream's wire index: the index every one of its frames
        /// names.
        index: u8,
    },
}

/// The direction of one observed stream, from this peer's perspective.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Direction {
    /// This side wrote the stream's messages.
    Sent,
    /// This side read the stream's messages.
    Received,
}

/// One side's elected role in a session's reconciliation descent.
///
/// Decided after the greetings are exchanged (the smaller advertised set
/// initiates; see [`SessionObserver::elected`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Role {
    /// This role asks the opening question and absorbs the final leaves.
    Initiator,
    /// This role answers the opening question.
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
