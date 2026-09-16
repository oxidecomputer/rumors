//! Bytes-level observation of live wire sessions.
//!
//! The hook a debugger, session recorder, or tracing adapter attaches
//! to a [`Peer`](crate::Peer) ([`Peer::observe`](crate::Peer::observe))
//! or a [`Bootstrap`](crate::Bootstrap) builder
//! ([`Bootstrap::observe`](crate::Bootstrap::observe)) to watch every
//! protocol message the peer exchanges, as raw wire bytes. The hook is
//! rumors-blind: no protocol type appears in its signature, each
//! invocation carries exactly one whole CBOR item with its stream
//! identity, and a consumer parses with any CBOR library — or none.
//!
//! Attachment has three levels. Each peer observer supplies its own
//! handlers for the levels below; every level can return `None` to
//! skip what it does not care about:
//!
//! - **Peer**: each [`Observer`] follows the peer through cloning,
//!   bookmarking, and reunion, and is asked for a session handler for
//!   every session the peer enters — gossip, bootstrap, and retire
//!   alike. Repeated attachment adds observers; callbacks visit them
//!   in attachment order.
//! - **Session**: a [`SessionObserver`] lives exactly as long as its
//!   session and is asked for a stream handler for each directed
//!   stream as it opens.
//! - **Stream**: a [`StreamObserver`] receives that one directed
//!   stream's messages, in stream order, one CBOR item per
//!   [`message`](StreamObserver::message) call.
//!
//! The contract:
//!
//! - **Ordering**: within one directed stream, invocations arrive in
//!   the stream's byte order; across streams there is no ordering at
//!   all (a session's streams pump concurrently). To recover the
//!   observed interleaving, stamp each message from a session-scoped
//!   atomic counter shared by the stream handlers.
//! - **Never block**: handlers run synchronously inside the session's
//!   own stream tasks. Blocking in
//!   [`message`](StreamObserver::message) stalls that directed stream,
//!   and waiting on protocol progress deadlocks; hand bytes off to a
//!   channel if the consumer is slow.
//! - **Coverage**: every directed stream of a session, both directions,
//!   control and data streams alike. The stream-open label is stream
//!   *addressing*, not an item, and is not delivered. Only complete
//!   items are observed: a session that dies mid-frame does not deliver
//!   the fragment, and an aborted session may have observed fewer items
//!   than crossed the wire.
//! - **Cost**: unattached (or a level declined), one branch per frame;
//!   attached, one extra contiguous copy of each observed frame. The
//!   wire bytes themselves are unchanged either way.
//!
//! The hook watches the **wire**, synchronously, from inside the
//! session's tasks; the content observers
//! ([`UnorderedMessages`](crate::UnorderedMessages),
//! [`CausalMessages`](crate::CausalMessages), and
//! [`Changes`](crate::Changes)) watch the **set**, asynchronously,
//! from outside.

use std::sync::{Arc, Mutex, PoisonError};

use crate::Protocol;

/// One peer-level observation handler, yielding one [`SessionObserver`]
/// per session the peer enters.
///
/// Attach with [`Peer::observe`](crate::Peer::observe) or
/// [`Bootstrap::observe`](crate::Bootstrap::observe). The handler is
/// shared by every clone of the peer's [`Rumors`](crate::Rumors)
/// handle, and sessions run concurrently, so it is asked for session
/// handlers from concurrent tasks.
pub trait Observer: Send + Sync {
    /// Begin observing one session, or return `None` to skip it.
    ///
    /// Called once per session, before this side writes its first byte or
    /// delivers any item to a handler. The returned handler lives for the session.
    fn session(&self, session: &SessionInfo) -> Option<Box<dyn SessionObserver>>;
}

/// A session-level observation handler: yields one [`StreamObserver`]
/// per directed stream, as each opens.
///
/// A session's streams open and pump concurrently, so
/// [`stream`](Self::stream) is called from concurrent tasks.
pub trait SessionObserver: Send + Sync {
    /// Learn which role this side won in the session's role election.
    ///
    /// Called at most once, when the election is decided — after the
    /// greetings are exchanged and before any data stream opens. Not
    /// called when the greetings carried equal versions (no election
    /// happens: the session ends over the control stream alone). The
    /// role is not part of [`SessionInfo`] because it does not exist
    /// yet when the session begins; a consumer that needs it before
    /// data frames arrive records it here. The default does nothing.
    fn elected(&self, role: Role) {
        let _ = role;
    }

    /// Begin observing one directed stream, or return `None` to skip
    /// it.
    ///
    /// Called once per directed stream, when it opens: for the control
    /// stream's two directions at session start, and for each data
    /// stream when this side first writes (sent) or first reads
    /// (received) it. A data stream the session never speaks yields no
    /// handler. The returned handler's lifetime is the stream's.
    fn stream(&self, stream: &StreamInfo) -> Option<Box<dyn StreamObserver>>;
}

/// A stream-level observation handler: receives one directed stream's
/// messages, in stream order.
pub trait StreamObserver: Send {
    /// Observe one protocol message: exactly one CBOR item of the
    /// wire, as sent or received on this handler's directed stream.
    ///
    /// Invoked synchronously from the stream's own task, after the
    /// item was written and flushed (sent) or completely read and
    /// accepted (received). Blocking here stalls this directed stream;
    /// see the module docs' back-pressure contract.
    fn message(&mut self, bytes: &[u8]);
}

/// What identifies one observed session.
///
/// Deliberately carries no session number: numbering is the observer's
/// own concern, exactly like message interleaving (see the module
/// docs' ordering section). An [`Observer`] that wants "the peer's Nth
/// observed session" counts inside its own
/// [`session`](Observer::session) — the method is `&self`, so it
/// synchronizes internally (an `AtomicU64` suffices), and the count
/// means precisely what that observer defines it to mean.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionInfo {
    /// Which lifecycle operation entered the session.
    pub kind: SessionKind,
    /// The wire dialect the session speaks.
    pub protocol: Protocol,
}

/// The lifecycle operation that entered an observed session, on this
/// side.
///
/// The counterparty's role in the same session may differ: a peer
/// serving a bootstrap or absorbing a retirement observes an ordinary
/// [`Gossip`](Self::Gossip) session, and learns what the remote wants
/// from the remote's preamble — which its control-stream handler sees
/// as bytes.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionKind {
    /// This side is joining the universe ([`Bootstrap::join`](crate::Bootstrap::join)).
    Bootstrap,
    /// This side is gossiping ([`Rumors::gossip`](crate::Rumors::gossip)
    /// and [`Rumors::gossip_once`](crate::Rumors::gossip_once)).
    Gossip,
    /// This side is retiring ([`Peer::retire`](crate::Peer::retire)).
    Retire,
}

/// What identifies one observed directed stream.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamInfo {
    /// Which of the session's streams this is.
    pub id: StreamId,
    /// Whether this side sent or received the stream's messages.
    pub direction: Direction,
}

/// One session stream's identity.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamId {
    /// Session setup, network joins and departures, and completion confirmation.
    Control,
    /// One reconciliation data stream.
    Data {
        /// The elected role that speaks this stream's frames.
        ///
        /// Sent data streams are spoken by this side's elected role;
        /// received ones by the counterparty's.
        speaker: Role,
        /// The stream's wire index, `0..`[`STREAM_COUNT`](crate::link::STREAM_COUNT):
        /// the same index the stream's on-wire open label carries.
        index: u8,
    },
}

/// The direction of one observed stream, from this peer's perspective.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// This side wrote the stream's messages.
    Sent,
    /// This side read the stream's messages.
    Received,
}

/// One side's elected role in a session's reconciliation descent.
///
/// Decided after the greetings are exchanged (the smaller advertised
/// set initiates; see [`SessionObserver::elected`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// This role asks the opening question and absorbs the final
    /// leaves.
    Initiator,
    /// This role answers the opening question.
    Responder,
}

/// The wire observers a peer carries, in attachment order.
///
/// The collection follows the peer through every handle just as its
/// replica state does. Clones share the collection until a builder
/// attaches another observer.
#[derive(Clone, Default)]
pub(crate) struct Attachment {
    /// Observers called for each future session, in attachment order.
    handlers: Arc<[Arc<dyn Observer>]>,
}

impl std::fmt::Debug for Attachment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
    /// The handle is inert — every invocation a no-op branch — when no
    /// observer accepts the session.
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

/// One session's observation handle: cheap to clone, inert when no
/// handler observes the session.
///
/// The session machinery threads a clone to every layer that emits or
/// accepts wire items (the pattern the stats recorder set). Data
/// streams create their own owned [`StreamObserver`]s through
/// [`data`](Self::data) when they open; the control stream's two
/// handlers live here, behind mutexes, because the control stream's
/// items are written from several protocol layers in sequence — the
/// locks are uncontended by construction (each direction's items are
/// protocol-ordered) and absent entirely from the unattached path.
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

    /// Create the handlers for one opening data stream, if any session
    /// observers want it.
    pub(crate) fn data(
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
/// A poisoned lock means an earlier invocation panicked (an
/// application handler's panic, already propagating through the
/// session); keep delivering to the handlers rather than silently
/// dropping the direction.
fn observe_control(slot: &Mutex<Vec<Box<dyn StreamObserver>>>, bytes: &[u8]) {
    let mut guard = slot.lock().unwrap_or_else(PoisonError::into_inner);
    for observer in guard.iter_mut() {
        observer.message(bytes);
    }
}

/// A reader adapter that retains a copy of every delivered byte, so an
/// exact item-shaped read (a frame, a greeting, a hand-off) can hand
/// its observer the item's true wire bytes rather than a re-encoding.
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

impl<R> tokio::io::AsyncRead for CaptureRead<'_, R>
where
    R: tokio::io::AsyncRead + Unpin + ?Sized,
{
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let this = self.get_mut();
        let before = buf.filled().len();
        let poll = std::pin::Pin::new(&mut *this.inner).poll_read(cx, buf);
        if let std::task::Poll::Ready(Ok(())) = &poll {
            this.captured.extend_from_slice(&buf.filled()[before..]);
        }
        poll
    }
}

#[cfg(test)]
mod tests;
