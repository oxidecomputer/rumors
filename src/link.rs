//! The transport contract: independent per-session streams for wire gossip.
//!
//! A [`Link`] is one long-lived connection between two replicas: one
//! [`Rumors`](crate::Rumors) handle on this side, one remote peer on the
//! other. Every wire session — [`gossip`](crate::Rumors::gossip),
//! [`bootstrap`](crate::Peer::bootstrap), [`retire`](crate::Peer::retire),
//! and each session driven by [`gossip_when`](crate::Rumors::gossip_when) —
//! runs *on* a link, and successive sessions on one link are serialized.
//!
//! The link is a bundle of three transport roles, split by how the session
//! uses them concurrently:
//!
//! - a persistent bidirectional **control stream** (its two halves), carrying
//!   each session's preamble, causal-version handshake, trailing identity
//!   hand-off, and (under the default protocol) one closing epilogue marker
//!   byte, in order, for the life of the link;
//! - a [`Connector`], from which the session lazily opens outgoing
//!   unidirectional data streams mid-descent;
//! - an [`Acceptor`], from which the session receives the peer's incoming
//!   unidirectional data streams, in arrival order.
//!
//! Data streams are session-scoped and cheap: a session opens up to
//! [`STREAM_COUNT`] outgoing streams (typically far fewer — streams are
//! opened only when reconciliation has something to say at that level), and
//! every data stream is closed before the session completes. Only the
//! control stream survives from one session to the next.
//!
//! # The contract
//!
//! The protocol's deadlock-freedom argument rests on stream independence, so
//! these clauses are load-bearing for every implementation ("instantiation")
//! of [`Connector`] and [`Acceptor`]:
//!
//! - **Independence.** Each data stream is reliable and ordered internally,
//!   with **no ordering guaranteed — or assumed — across streams**. Writing
//!   to one stream may block only on that stream's receiver; reading one
//!   stream never depends on any other stream's progress.
//! - **Flow control.** Per-stream backpressure must be receiver-paced with a
//!   bounded buffer: a sender that runs ahead of its receiver blocks, and
//!   only that stream blocks with it.
//! - **Concurrency.** Up to [`STREAM_COUNT`] streams per direction may be
//!   open at once, while [`Connector::connect`] calls arrive sparsely and
//!   mid-session. An instantiation must not require a full complement of
//!   streams, nor serialize an open behind unrelated stream progress.
//! - **Half-close.** Dropping a [`Connector::Tx`] ends that stream; the
//!   peer's [`Acceptor::Rx`] then observes end-of-stream after the final
//!   bytes. The control stream outlives all data streams of its session.
//! - **Cancellation.** A pending [`Acceptor::accept`] future is dropped at
//!   session teardown; instantiations must tolerate that and deliver the
//!   affected stream to a later `accept` call or fail it cleanly.
//!
//! Streams are anonymous at this boundary. The session labels each opened
//! stream itself (a session epoch and stream index written as the stream's
//! first bytes) and validates the label on the accepting side, so an
//! [`Acceptor`] may yield streams in any order and needs no routing logic.
//!
//! # Instantiations
//!
//! [`memory`] builds the in-memory instantiation both ends of a test or an
//! in-process pairing use; it is also the reference implementation of the
//! contract. Network instantiations (QUIC connections mapping streams 1:1,
//! TCP with one connection per stream behind a routing listener) live in
//! sibling crates so the core stays free of network dependencies; a
//! deployment can also implement the two traits directly against its own
//! transport.

use std::future::Future;
use std::io;

use tokio::io::{AsyncRead, AsyncWrite, DuplexStream};
use tokio::sync::mpsc;

/// Logical data streams a session may open in one direction.
///
/// One per speaker-owned reply height in the descent schedule: the protocol
/// never opens more, and instantiations must admit this many concurrently
/// (per direction, plus the control stream).
pub const STREAM_COUNT: usize = 17;

/// Opens outgoing unidirectional data streams for one link.
///
/// The session hands an owned clone to each stream producer, and producers
/// connect concurrently — hence `Clone + Send + Sync + 'static`. Natural
/// implementations are handles: a QUIC connection, a channel sender, a
/// dialer around an address. See the [module docs](self) for the contract
/// every implementation must satisfy.
pub trait Connector: Clone + Send + Sync + 'static {
    /// The write half of one outgoing data stream.
    type Tx: AsyncWrite + Unpin + Send + 'static;

    /// Open one outgoing unidirectional stream.
    ///
    /// # Errors
    ///
    /// Fails only for transport reasons (the link is gone); the session
    /// treats any error as fatal to the session, never retries.
    fn connect(&self) -> impl Future<Output = io::Result<Self::Tx>> + Send;
}

/// Accepts incoming unidirectional data streams for one link.
///
/// The session's single accept loop is the sole caller, so `&mut self` —
/// unlike [`Connector`], no sharing is required. See the [module
/// docs](self) for the contract every implementation must satisfy.
pub trait Acceptor: Send {
    /// The read half of one incoming data stream.
    type Rx: AsyncRead + Unpin + Send + 'static;

    /// Accept the next incoming unidirectional stream, in arrival order.
    ///
    /// # Errors
    ///
    /// Fails only for transport reasons (the link is gone); the session
    /// treats any error as fatal to the session.
    ///
    /// # Cancel safety
    ///
    /// The session drops a pending `accept` at teardown. A stream that was
    /// mid-delivery must either surface from a later `accept` call or fail
    /// cleanly; it must not be silently lost while the link stays healthy.
    fn accept(&mut self) -> impl Future<Output = io::Result<Self::Rx>> + Send;
}

impl<A: Acceptor> Acceptor for &mut A {
    type Rx = A::Rx;

    async fn accept(&mut self) -> io::Result<Self::Rx> {
        A::accept(self).await
    }
}

/// One long-lived connection to one remote peer: the transport every wire
/// session runs on.
///
/// Construct one with [`Link::new`] around an implementation of the
/// [module-level contract](self), or use [`memory`] for the in-memory
/// instantiation. The link owns its [`SessionState`] — the session counter
/// used to label data streams, and the poison latch — which is why sessions
/// take the link by `&mut`: serialized sessions are part of the contract,
/// enforced by the borrow.
///
/// A session that fails, or is cancelled mid-flight, leaves the control
/// stream mid-frame and *poisons* the link: every later session on it fails
/// fast with [`Error::LinkPoisoned`](crate::Error::LinkPoisoned) rather
/// than misreading leftover bytes. Discard a poisoned link and reconnect;
/// there is no repair.
pub struct Link<CR, CW, C, A> {
    pub(crate) control_read: CR,
    pub(crate) control_write: CW,
    pub(crate) connector: C,
    pub(crate) acceptor: A,
    /// This link's session counter and poison latch.
    pub(crate) session: SessionState,
}

/// One link's session bookkeeping: the stream-label epoch and the poison
/// latch.
///
/// Owned by [`Link`] and exposed through [`LinkParts::session`], so a link
/// wrapper can carry it across decoration. Both ends of a connection hold
/// equal states: sessions are serialized and both ends run each session, so
/// the fields advance in lockstep.
#[derive(Clone, Copy, Debug)]
pub struct SessionState {
    /// The next session's epoch. Both ends count every session on the link
    /// (sessions are serialized, and both ends run each handshake), so the
    /// counters agree; it wraps, serving as a label tripwire rather than an
    /// identity.
    pub epoch: u8,
    /// Whether a session on this link was interrupted before its boundary.
    ///
    /// Set while a session is in flight and cleared when it completes
    /// cleanly, so between sessions it reads true only on a link whose
    /// control stream rests somewhere mid-frame — a link no further
    /// session can trust. Starting a session on such a link fails fast
    /// with [`Error::LinkPoisoned`](crate::Error::LinkPoisoned).
    pub poisoned: bool,
}

impl SessionState {
    /// Open one session, returning the epoch that labels its data streams.
    ///
    /// Fails fast with [`Error::LinkPoisoned`](crate::Error::LinkPoisoned)
    /// if an earlier session was interrupted, without burning an epoch.
    /// Otherwise the latch is set and the counter advanced *before* any
    /// wire traffic, so a session that fails or is cancelled at any point —
    /// even before its first byte — leaves the link poisoned; only the
    /// session funnels' clean-completion [`finish`](Self::finish) clears it.
    pub(crate) fn begin(&mut self) -> Result<u8, crate::Error> {
        if self.poisoned {
            return Err(crate::Error::LinkPoisoned);
        }
        self.poisoned = true;
        let epoch = self.epoch;
        self.epoch = self.epoch.wrapping_add(1);
        Ok(epoch)
    }

    /// Record the open session's clean completion, clearing the poison
    /// latch: the control stream rests exactly at the session boundary.
    pub(crate) fn finish(&mut self) {
        self.poisoned = false;
    }
}

impl<CR, CW, C, A> Link<CR, CW, C, A>
where
    CR: AsyncRead + Unpin + Send,
    CW: AsyncWrite + Unpin + Send,
    C: Connector,
    A: Acceptor,
{
    /// Bundle a link from its control halves and stream supply.
    ///
    /// Both ends of a connection must construct their links from the same
    /// underlying transport at the same time: the epoch counters start at
    /// zero on both sides and advance in lockstep with each session.
    pub fn new(control_read: CR, control_write: CW, connector: C, acceptor: A) -> Self {
        Self {
            control_read,
            control_write,
            connector,
            acceptor,
            session: SessionState {
                epoch: 0,
                poisoned: false,
            },
        }
    }
}

impl<CR, CW, C, A> Link<CR, CW, C, A> {
    /// Assemble a single session's carrier around already-erased halves.
    ///
    /// Unlike [`new`](Self::new), the epoch is the caller's: the long-lived
    /// link advances its counter per session and lends this carrier the
    /// current value.
    pub(crate) fn for_session(
        control_read: CR,
        control_write: CW,
        connector: C,
        acceptor: A,
        epoch: u8,
    ) -> Self {
        Self {
            control_read,
            control_write,
            connector,
            acceptor,
            // The carrier's own poison flag is inert: it lives for one
            // session and is discarded; the long-lived link's state is the
            // one the funnels consult and clear.
            session: SessionState {
                epoch,
                poisoned: false,
            },
        }
    }

    /// Disassemble into [`LinkParts`], for building a decorated link.
    ///
    /// This is how a wrapper — fault injection, byte capture, an adversity
    /// harness — interposes on an existing link: decorate the parts, then
    /// reassemble with [`LinkParts::into_link`]. The parts carry the
    /// [`SessionState`] so a decorated link stays in lockstep with its
    /// remote peer's counting.
    pub fn into_parts(self) -> LinkParts<CR, CW, C, A> {
        LinkParts {
            control_read: self.control_read,
            control_write: self.control_write,
            connector: self.connector,
            acceptor: self.acceptor,
            session: self.session,
        }
    }
}

/// The dismantled pieces of a [`Link`]; see [`Link::into_parts`].
pub struct LinkParts<CR, CW, C, A> {
    /// The control stream's read half.
    pub control_read: CR,
    /// The control stream's write half.
    pub control_write: CW,
    /// The outgoing stream supply.
    pub connector: C,
    /// The incoming stream supply.
    pub acceptor: A,
    /// The link's session counter and poison latch; see [`SessionState`].
    ///
    /// Preserve it when reassembling a wrapped link: both ends of a
    /// connection count sessions in lockstep, so a reset counter would
    /// mislabel every stream of the next session — and a cleared
    /// [`poisoned`](SessionState::poisoned) latch would let sessions run on
    /// a link whose control stream rests mid-frame.
    pub session: SessionState,
}

impl<CR, CW, C, A> LinkParts<CR, CW, C, A>
where
    CR: AsyncRead + Unpin + Send,
    CW: AsyncWrite + Unpin + Send,
    C: Connector,
    A: Acceptor,
{
    /// Reassemble (possibly decorated) parts into a link.
    pub fn into_link(self) -> Link<CR, CW, C, A> {
        Link {
            control_read: self.control_read,
            control_write: self.control_write,
            connector: self.connector,
            acceptor: self.acceptor,
            session: self.session,
        }
    }
}

/// The in-memory [`Link`] type: both ends of [`memory`].
pub type MemoryLink = Link<DuplexStream, DuplexStream, MemoryConnector, MemoryAcceptor>;

/// Bytes buffered by each in-memory stream (control and data alike) before
/// its writer blocks on its reader: enough to keep honest sessions off the
/// backpressure path while still exercising bounded buffers.
const MEMORY_STREAM_CAPACITY: usize = 8 * 1024;

/// In-flight opened-but-unaccepted streams per direction.
///
/// A session opens at most [`STREAM_COUNT`] streams, sessions are
/// serialized, and the accept loop drains continuously, so this bound is
/// never the limiting factor for an honest peer.
const MEMORY_STREAM_BACKLOG: usize = STREAM_COUNT;

/// Create a connected pair of in-memory links.
///
/// Everything about the pair is deterministic and closed-world: byte
/// transport is [`tokio::io::duplex`] pipes and stream announcement is a
/// bounded channel, so sessions driven over it work under any executor —
/// including the deterministic single-poll harness the crate's own tests
/// use. Each stream buffers 8 KiB; use [`memory_with_capacity`] to pick
/// the buffer size (down to one byte, the contract still holds).
pub fn memory() -> (MemoryLink, MemoryLink) {
    memory_with_capacity(MEMORY_STREAM_CAPACITY)
}

/// [`memory`], with every stream's byte buffer set to `capacity`.
///
/// # Panics
///
/// If `capacity` is zero: a zero-capacity pipe could never carry a byte.
pub fn memory_with_capacity(capacity: usize) -> (MemoryLink, MemoryLink) {
    assert!(capacity > 0, "a link stream must buffer at least one byte");
    let (a_control_write, b_control_read) = tokio::io::duplex(capacity);
    let (b_control_write, a_control_read) = tokio::io::duplex(capacity);
    let (a_announce, b_streams) = mpsc::channel(MEMORY_STREAM_BACKLOG);
    let (b_announce, a_streams) = mpsc::channel(MEMORY_STREAM_BACKLOG);
    (
        Link::new(
            a_control_read,
            a_control_write,
            MemoryConnector {
                announce: a_announce,
                capacity,
            },
            MemoryAcceptor { streams: a_streams },
        ),
        Link::new(
            b_control_read,
            b_control_write,
            MemoryConnector {
                announce: b_announce,
                capacity,
            },
            MemoryAcceptor { streams: b_streams },
        ),
    )
}

/// The in-memory [`Connector`]: each open mints a bounded pipe and announces
/// the read end to the peer's acceptor.
#[derive(Clone)]
pub struct MemoryConnector {
    announce: mpsc::Sender<DuplexStream>,
    capacity: usize,
}

impl Connector for MemoryConnector {
    type Tx = DuplexStream;

    async fn connect(&self) -> io::Result<Self::Tx> {
        let (tx, rx) = tokio::io::duplex(self.capacity);
        self.announce
            .send(rx)
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "peer link is gone"))?;
        Ok(tx)
    }
}

/// The in-memory [`Acceptor`]: receives the streams the peer's connector
/// announced, in open order.
pub struct MemoryAcceptor {
    streams: mpsc::Receiver<DuplexStream>,
}

impl Acceptor for MemoryAcceptor {
    type Rx = DuplexStream;

    async fn accept(&mut self) -> io::Result<Self::Rx> {
        self.streams
            .recv()
            .await
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "peer link is gone"))
    }
}

pub(crate) mod erased;

#[cfg(test)]
mod tests;
