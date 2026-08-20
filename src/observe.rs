//! Bytes-level observation of live wire sessions.
//!
//! This module is the crate's capture surface: the hook a debugger, a
//! session recorder, or a tracing adapter attaches to a
//! [`Peer`](crate::Peer) (or a [`Bootstrap`](crate::Bootstrap) builder,
//! for the joining session itself) to watch every protocol message a
//! peer exchanges, as raw wire bytes. The hook is *rumors-blind by
//! design*: no protocol type appears in its signature, an invocation
//! carries **exactly one CBOR item** of the wire, and a consumer parses
//! with any CBOR library — or none. What the items mean is the wire
//! format's contract; what this module promises is that you see each
//! one, whole, with its stream identity.
//!
//! # The three levels
//!
//! Attachment mirrors the session machinery's own shape, one handler
//! per level, each minted by the level above:
//!
//! - **Peer**: an [`Observer`] attaches once, at construction
//!   ([`Peer::observe`](crate::Peer::observe),
//!   [`Bootstrap::observe`](crate::Bootstrap::observe)), and follows
//!   the peer through cloning, bookmarking, and reunion. For every
//!   session the peer enters — gossip, bootstrap, and retire alike —
//!   it is asked for a session handler.
//! - **Session**: a [`SessionObserver`] lives exactly as long as its
//!   session. Its [`SessionInfo`] identifies the session (kind and
//!   protocol; numbering sessions is the observer's own concern — see
//!   [`SessionInfo`]), and it is asked for a stream handler for each
//!   directed stream of the session as that stream opens: the control
//!   stream's two directions at session start, and each data stream
//!   when the protocol first speaks or reads it.
//! - **Stream**: a [`StreamObserver`] receives that one directed
//!   stream's messages, in stream order, one whole CBOR item per
//!   [`message`](StreamObserver::message) call.
//!
//! Every level can decline (return `None`) to skip what it does not
//! care about; an unattached peer pays one branch per frame and
//! nothing else.
//!
//! # Ordering
//!
//! Within one directed stream, invocations arrive in the stream's own
//! byte order. **Across streams, the hook imposes no ordering at
//! all**: a session's streams pump concurrently, and the library does
//! not serialize observation across them. A consumer that wants the
//! observed interleaving reconstructs it without a lock by stamping
//! each message from a session-scoped atomic counter:
//!
//! ```
//! use std::sync::Arc;
//! use std::sync::atomic::{AtomicU64, Ordering};
//! use rumors::observe::{SessionObserver, StreamInfo, StreamObserver};
//!
//! struct Session {
//!     order: Arc<AtomicU64>,
//! }
//!
//! struct Stream {
//!     order: Arc<AtomicU64>,
//! }
//!
//! impl SessionObserver for Session {
//!     fn stream(&self, _: &StreamInfo) -> Option<Box<dyn StreamObserver>> {
//!         Some(Box::new(Stream { order: Arc::clone(&self.order) }))
//!     }
//! }
//!
//! impl StreamObserver for Stream {
//!     fn message(&mut self, bytes: &[u8]) {
//!         let ordinal = self.order.fetch_add(1, Ordering::Relaxed);
//!         // record (ordinal, bytes) …
//!         let _ = (ordinal, bytes);
//!     }
//! }
//! ```
//!
//! This is deliberately unlike the crate's *content* observers
//! ([`UnorderedMessages`](crate::UnorderedMessages),
//! [`CausalMessages`](crate::CausalMessages), and
//! [`Changes`](crate::Changes)), which are pull-based streams over the
//! replica's state. The hook watches the **wire**, synchronously, from
//! inside the session's own tasks; the content observers watch the
//! **set**, asynchronously, from outside.
//!
//! # Back-pressure
//!
//! Handlers run synchronously inside the session's stream tasks: a
//! [`message`](StreamObserver::message) call that blocks stalls its
//! own directed stream (and only it) until the call returns. Handlers
//! **must never wait on protocol progress** — deadlock — and should
//! return promptly; hand bytes off to a channel or buffer if the
//! consumer is slow. The same contract the session's internal error
//! reporting follows: never block the reporter.
//!
//! # What is observed, exactly
//!
//! Every directed stream of a `Protocol::V2` session, both directions:
//! the control stream (preamble, greeting, any identity hand-off, and
//! the epilogue marker — each its own item) and every data stream
//! (each reconciliation frame an item, the stream-end control frames
//! included; the stream-open label (two leading unsigned-int items) is
//! stream *addressing*, not an item, and is not delivered). Only complete items are
//! observed: a session that dies mid-frame does not deliver the
//! fragment, and a session aborted by a protocol violation may have
//! observed fewer items than crossed the wire. `Protocol::V1` sessions
//! are not observed: the frozen legacy wire is not a CBOR sequence, so
//! its bytes cannot honor this module's one-item contract.
//!
//! # Cost
//!
//! Unattached (or a level declined): one branch per frame. Attached:
//! each observed frame is additionally materialized once as a
//! contiguous buffer for the handler's `&[u8]`; the wire path itself
//! is unchanged, byte for byte.

use std::sync::{Arc, Mutex, PoisonError};

use crate::Protocol;

/// A peer-level observation handler: attaches once, yields one
/// [`SessionObserver`] per session the peer enters.
///
/// Attach with [`Peer::observe`](crate::Peer::observe) or
/// [`Bootstrap::observe`](crate::Bootstrap::observe). The handler is
/// shared by every clone of the peer's [`Rumors`](crate::Rumors)
/// handle, and sessions run concurrently, so it is asked for session
/// handlers from concurrent tasks.
pub trait Observer: Send + Sync {
    /// Begin observing one session, or return `None` to skip it.
    ///
    /// Called once per session, before the session's first byte
    /// crosses the wire — for sessions of an observable dialect; see
    /// the module docs' `Protocol::V1` exclusion. `session` identifies
    /// it; the returned handler's lifetime is the session's.
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
    /// and [`Rumors::gossip_when`](crate::Rumors::gossip_when)).
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
    /// The session's control stream: preamble, greeting, identity
    /// hand-off, epilogue.
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

/// The observation state a peer carries: the attached handler, if any
/// — shared, like the replica state, by every handle to one peer
/// identity.
#[derive(Clone, Default)]
pub(crate) struct Attachment {
    handler: Option<Arc<dyn Observer>>,
}

impl std::fmt::Debug for Attachment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Attachment")
            .field("attached", &self.handler.is_some())
            .finish()
    }
}

impl Attachment {
    /// Attach `observer`; later sessions ask it for session handlers.
    pub(crate) fn attach(&mut self, observer: Arc<dyn Observer>) {
        self.handler = Some(observer);
    }

    /// Enter one session: mint its handle.
    ///
    /// The handle is inert — every invocation a no-op branch — when no
    /// observer is attached, when the observer declines the session,
    /// or when the dialect is not observable (`Protocol::V1`'s frozen
    /// wire is not a CBOR sequence; the module docs state the
    /// exclusion).
    pub(crate) fn begin(&self, kind: SessionKind, protocol: Protocol) -> SessionHandle {
        let Some(handler) = &self.handler else {
            return SessionHandle::default();
        };
        if protocol != Protocol::V2 {
            return SessionHandle::default();
        }
        let info = SessionInfo { kind, protocol };
        let Some(session) = handler.session(&info) else {
            return SessionHandle::default();
        };
        // The control stream's two directions open with the session
        // itself: mint both handlers now, ahead of the preamble.
        let sent = session.stream(&StreamInfo {
            id: StreamId::Control,
            direction: Direction::Sent,
        });
        let received = session.stream(&StreamInfo {
            id: StreamId::Control,
            direction: Direction::Received,
        });
        SessionHandle {
            inner: Some(Arc::new(HandleInner {
                session,
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
/// streams mint their own owned [`StreamObserver`]s through
/// [`data`](Self::data) when they open; the control stream's two
/// handlers live here, behind mutexes, because the control stream's
/// items are written from several protocol layers in sequence — the
/// locks are uncontended by construction (each direction's items are
/// protocol-ordered) and absent entirely from the unattached path.
#[derive(Clone, Default)]
pub(crate) struct SessionHandle {
    inner: Option<Arc<HandleInner>>,
}

struct HandleInner {
    session: Box<dyn SessionObserver>,
    control_sent: Mutex<Option<Box<dyn StreamObserver>>>,
    control_received: Mutex<Option<Box<dyn StreamObserver>>>,
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
            inner.session.elected(role);
        }
    }

    /// Mint the handler for one opening data stream, if the session
    /// handler wants it.
    pub(crate) fn data(
        &self,
        speaker: Role,
        index: u8,
        direction: Direction,
    ) -> Option<Box<dyn StreamObserver>> {
        let inner = self.inner.as_ref()?;
        inner.session.stream(&StreamInfo {
            id: StreamId::Data { speaker, index },
            direction,
        })
    }
}

/// Invoke one control-direction handler under its lock.
///
/// A poisoned lock means an earlier invocation panicked (an
/// application handler's panic, already propagating through the
/// session); keep delivering to the handler rather than silently
/// dropping the direction.
fn observe_control(slot: &Mutex<Option<Box<dyn StreamObserver>>>, bytes: &[u8]) {
    let mut guard = slot.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(observer) = guard.as_mut() {
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
