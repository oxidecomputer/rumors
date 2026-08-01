//! The transport contract: independent per-session streams for wire gossip.
//!
//! A [`Link`] is one long-lived connection between two replicas. Every wire
//! session ([`gossip`](crate::Rumors::gossip), each session driven by
//! [`gossip_when`](crate::Rumors::gossip_when),
//! [`bootstrap`](crate::Bootstrap::join), and
//! [`retire`](crate::Peer::retire)) runs on a link, one session at a time.
//!
//! A link bundles three transport roles:
//!
//! - the **control stream**: one persistent bidirectional byte stream,
//!   carrying every session's framing (preamble and greeting through
//!   closing epilogue) in order, for the life of the link;
//! - a [`Connector`]: opens outgoing unidirectional **data streams**,
//!   lazily, mid-session;
//! - an [`Acceptor`]: yields the peer's incoming data streams, in whatever
//!   order the transport delivers them.
//!
//! Data streams are session-scoped and cheap: a session opens them lazily
//! and sparsely (up to [`STREAM_COUNT`], typically far fewer) and closes
//! every one before it completes. Only the control stream survives into
//! the next session.
//!
//! # The contract
//!
//! The protocol's deadlock-freedom argument rests on stream independence, so
//! these clauses are load-bearing for every implementation ("instantiation")
//! of a link, the caller-supplied control halves as much as [`Connector`]
//! and [`Acceptor`]:
//!
//! - **Control duplex.** The control stream's two directions are
//!   independent: a side's control read makes progress while that same
//!   side's control write sits blocked on the peer. The protocol exchanges
//!   its largest control frames (the greeting, the epilogue) as concurrent
//!   write-and-read on both ends precisely because such a frame may exceed
//!   any buffer, so a carrier that couples the directions (a half-duplex
//!   turn protocol, a shared lock across read and write) deadlocks the
//!   first oversized exchange. Receiver-paced backpressure at any positive
//!   capacity is fine, exactly as for data streams.
//! - **Independence.** Each data stream is reliable and ordered internally,
//!   with **no ordering guaranteed, or assumed, across streams**. Writing
//!   to one stream may block only on that stream's receiver; reading one
//!   stream never depends on any other stream's progress.
//! - **Flow control.** Per-stream backpressure must be receiver-paced with a
//!   bounded buffer: a sender that runs ahead of its receiver blocks, and
//!   only that stream blocks with it. Any positive capacity suffices: the
//!   protocol never assumes a frame fits in flight (every phase either
//!   alternates strictly or pairs its write with the peer's concurrent
//!   read), so sessions stay live down to a one-byte window, where the
//!   crate's own tests pin every session shape.
//! - **Concurrency.** Up to [`STREAM_COUNT`] streams per direction may be
//!   open at once, while [`Connector::connect`] calls arrive sparsely and
//!   mid-session. An instantiation must not require a full complement of
//!   streams, nor serialize an open behind unrelated stream progress.
//! - **Half-close.** Dropping a [`Connector::Tx`] ends that stream; the
//!   peer's [`Acceptor::Rx`] then observes end-of-stream after the final
//!   bytes. The control stream outlives all data streams of its session.
//! - **Cancellation.** A pending [`Acceptor::accept`] future may be dropped
//!   at any moment (session teardown is the common source, and the
//!   conformance suite drops them mid-session); instantiations must tolerate
//!   the drop and deliver the affected stream to a later `accept` call: no
//!   delivery may be lost while the link stays healthy.
//!
//! Streams are anonymous at this boundary. The session labels each opened
//! stream itself (a [session epoch](SessionState) and stream index written
//! as the stream's first bytes) and validates the label on the accepting
//! side, so an
//! [`Acceptor`] may yield streams in any order and needs no routing logic.
//!
//! ## Pooled flow control
//!
//! Some transports do not give every stream its own private buffer.
//! QUIC, for example, layers one connection-level window over the
//! per-stream windows, so all streams draw on a shared pool of buffer
//! credit, and a stream can find the pool empty even though its own
//! window has room. The independence clause still holds on such a
//! transport, by sizing alone. The reasoning: a stream never holds more
//! unread bytes than its own buffer allows, so a pool large enough to
//! cover every stream's buffer at once always retains headroom, the pool
//! never binds, and every clause reduces to the per-stream case. That
//! takes two things:
//!
//! - Size the pool to at least **([`STREAM_COUNT`] + 1) × B** per
//!   direction, where B is the per-stream buffering the transport grants:
//!   every data stream plus the control stream (the +1), each sitting
//!   full at the same moment. One pool shared by both directions needs
//!   the sum, twice that.
//! - Credit the pool back as the receiver consumes bytes, not when
//!   streams close. The control stream never closes, so a close-credited
//!   pool eventually spends its whole budget on control traffic and
//!   deadlocks, at any pool size.
//!
//! The conformance suite exercises a pool exactly at the bound (passes
//! the whole suite) and one far below the buffering it must cover (fails
//! the independence check).
//!
//! Sessions at the serialization floor (a one-subtree-in-flight window)
//! have been *observed* live over far smaller pools, down to tens of
//! bytes, with latency degradation. That is observed behavior of the
//! current protocol at that window shape, not a promise: a window wide
//! enough to fill several streams at once can leave a sub-bound pool in
//! a cycle of waits, each stream waiting on pool credit the others hold.
//! Size pools to the bound.
//!
//! # What securing the transport means
//!
//! The trust model (see the [crate docs](crate)) leaves authenticating
//! peers and securing the transport to the application. It is worth
//! being concrete about that division, because the protocol's own
//! validation can look like security and is not. The protocol does
//! reject malformed and mismatched sessions with typed errors, trusts
//! nothing peer-declared before the fixed preamble validates, and leaves
//! the caller's timeout as the sole liveness backstop against a silent
//! peer; all of that machinery exists to catch nonconforming peers, and
//! none of it is a security boundary. What the protocol actually leans
//! on, stated as requirements on the transport:
//!
//! - **Authentication is authorization.** Any counterparty that can
//!   complete a session holds full write authority over the set; the
//!   protocol makes no finer-grained access decision.
//! - **Integrity.** The wire decoders detect malformed frames, not
//!   tampering: a transport that lets a third party modify bytes in
//!   flight has granted that party the session.
//! - **Confidentiality.** Session traffic (message bodies, set sizes,
//!   version structure, the greeting's metadata) crosses this boundary
//!   in cleartext; the protocol adds no encryption of its own.
//! - **Freshness.** The protocol does not authenticate a connection or a
//!   session as new; a transport that could replay recorded traffic into
//!   a live connection must rule that out itself.
//!
//! # Instantiations
//!
//! [`memory`] builds the in-memory instantiation, both ends at once, for
//! tests and in-process pairings; it is also the reference implementation
//! of the contract. A transport with native substreams implements the two
//! traits directly (QUIC connections map streams 1:1); an accept/connect
//! transport with no innate substreams (TCP and everything shaped like
//! it) gets the [`routed`] adapter, which builds the
//! one-connection-per-stream shape behind a per-process router from a
//! caller-supplied dial/listen pair. The core still ships no network
//! code — the adapter is generic, and its instantiations live with the
//! caller — so either way a deployment validates its transport with the
//! `conformance` feature's link suite.

use std::future::Future;
use std::io;

use tokio::io::{AsyncRead, AsyncWrite, DuplexStream};
use tokio::sync::mpsc;

/// Logical data streams a session may open in one direction.
///
/// The protocol never opens more, and instantiations must admit this many
/// concurrently (per direction, plus the control stream). The value is the
/// protocol's own, fixed by its wire schedule (the descent's 32 tree
/// heights at a two-height stride per stream, plus the shared opening
/// stream: `ceil(32 / 2) + 1 = 17`) and pinned against the wire codec by
/// test, so it cannot drift silently.
pub const STREAM_COUNT: usize = 17;

/// Opens outgoing unidirectional data streams for one link.
///
/// Opens may arrive concurrently, through clones and through shared
/// references alike (hence `Clone + Send + Sync + 'static`), and every
/// clone must reach the same peer: clones are handles onto one stream
/// supply, never isolated sub-connections with their own state. Natural
/// implementations are handles already: a QUIC connection, a channel
/// sender, a dialer around an address. See the [module docs](self) for the
/// contract every implementation must satisfy.
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
/// Accepts arrive through `&mut self`: unlike [`Connector`], no sharing
/// is required, so an implementation needs no internal synchronization.
/// See the [module docs](self) for the contract every implementation must
/// satisfy.
pub trait Acceptor: Send {
    /// The read half of one incoming data stream.
    type Rx: AsyncRead + Unpin + Send + 'static;

    /// Accept one incoming unidirectional stream.
    ///
    /// Order across streams is the transport's own: the session pairs
    /// streams by the label written as each stream's first bytes, never by
    /// accept order, so an implementation may yield arrivals in any order.
    ///
    /// # Errors
    ///
    /// Fails only for transport reasons (the link is gone); the session
    /// treats any error as fatal to the session.
    ///
    /// # Cancel safety
    ///
    /// A pending `accept` future may be dropped at any moment (session
    /// teardown is the common source). A stream that was mid-delivery must
    /// surface from a later `accept` call; it must not be lost while the
    /// link stays healthy.
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
/// instantiation. Sessions on one link run one at a time: each takes the
/// link by `&mut`, and the borrow enforces the serialization. Wrappers
/// that decorate a link carry its [`SessionState`] across the rebuild
/// ([`Link::into_parts`]).
///
/// # What a session promises
///
/// Every session on a link resolves in one of three ways:
///
/// - **`Ok`: both replicas committed.** Under the default
///   [`Protocol::V2`](crate::Protocol::V2) the session ends with each side
///   exchanging a completion marker on the control stream, so `Ok`
///   certifies that the *peer* completed and committed too: every message
///   and identity the session moved is applied on both ends. The link rests
///   at the session boundary, ready for this pair's next session. One
///   residue is irreducible (the confirmation itself can be lost), in
///   which case that side observes
///   [`Error::Epilogue`](crate::Error::Epilogue), an `Err` whose local
///   replica is nonetheless fully committed (the error's docs explain why
///   the gap cannot be closed). (The frozen `V1` oracle wire has no marker
///   exchange; its `Ok` certifies only the local commit.)
/// - **`Err`: the local replica is unchanged, and the link is poisoned.**
///   The failed session leaves the control stream mid-frame, so every later
///   session on the link fails fast with
///   [`Error::LinkPoisoned`](crate::Error::LinkPoisoned) rather than
///   misreading leftover bytes: discard the link and reconnect; there is no
///   repair. "Unchanged" has three qualified exceptions, stated where they
///   arise:
///   - the post-commit [`Error::Epilogue`](crate::Error::Epilogue) above;
///   - a bootstrap donation lost in flight, which costs the donated
///     identity space ([`Bootstrap::join`](crate::Bootstrap::join));
///   - a bookmark persist failing after a retiring peer's identity is
///     absorbed, which leaves the session committed with the absorption
///     not yet crash-safe ([`Error::Bookmark`](crate::Error::Bookmark)).
///
///   `retire` reports failure through [`Retire`](crate::Retire)'s variants,
///   which state which side of the identity hand-off the failure landed on,
///   with the same link consequences.
/// - **Cancellation: as `Err`.** Dropping a session future mid-flight never
///   commits a partial session (the replica holds the session's full
///   effect or none of it) and poisons the link the same way. One
///   carve-out: a `retire` future owns its consumed [`Peer`](crate::Peer),
///   so dropping it destroys the peer and loses the identity (recoverable
///   only through an attached bookmark), where `retire`'s `Err` would have
///   handed the peer back through [`Retire`](crate::Retire)'s variants.
///
/// No session imposes its own deadline: against a stalled peer a session
/// waits forever, so the *caller* owns the timeout. Wrap sessions in your
/// runtime's timeout and treat expiry as any other cancellation: replica
/// intact or fully committed, link poisoned, reconnect.
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
/// wrapper can carry it across decoration. The state is sealed: a wrapper
/// carries the value whole (it is `Copy`), and only the link's own sessions
/// advance the counter or clear the latch.
///
/// The epoch advances in lockstep at both ends of a connection (sessions
/// are serialized and both ends run each session), but the poison latch is
/// local by design: one end can conclude a session `Ok` while its peer
/// fails post-commit ([`Error::Epilogue`](crate::Error::Epilogue)), so
/// between sessions the two ends' latches may legitimately disagree. Never
/// mirror one end's carried state onto the other.
#[derive(Clone, Copy, Debug)]
pub struct SessionState {
    /// The next session's epoch.
    ///
    /// Both ends count every session on the link (sessions are serialized,
    /// and both ends run each handshake), so the counters agree; it wraps,
    /// serving as a label tripwire rather than an identity.
    epoch: u8,
    /// Whether a session on this link was interrupted before its boundary.
    ///
    /// Set while a session is in flight and cleared when it completes
    /// cleanly, so between sessions it reads true only on a link whose
    /// control stream rests somewhere mid-frame, a link no further
    /// session can trust. Starting a session on such a link fails fast
    /// with [`Error::LinkPoisoned`](crate::Error::LinkPoisoned).
    poisoned: bool,
}

impl SessionState {
    /// The epoch that will label the next session's data streams.
    pub fn epoch(&self) -> u8 {
        self.epoch
    }

    /// Whether a session on this link was interrupted before its boundary.
    ///
    /// Between sessions this reads true only on a link whose control
    /// stream rests somewhere mid-frame; starting a session on such a link
    /// fails fast with [`Error::LinkPoisoned`](crate::Error::LinkPoisoned).
    pub fn poisoned(&self) -> bool {
        self.poisoned
    }
    /// Open one session, returning the epoch that labels its data streams.
    ///
    /// Fails fast with [`Error::LinkPoisoned`](crate::Error::LinkPoisoned)
    /// if an earlier session was interrupted, without burning an epoch.
    /// Otherwise the latch is set and the counter advanced *before* any
    /// wire traffic, so a session that fails or is cancelled at any point,
    /// even before its first byte, leaves the link poisoned; only the
    /// session funnels' clean-completion [`finish`](Self::finish) clears it.
    pub(crate) fn begin<E>(&mut self) -> Result<u8, crate::Error<crate::bookmark::NoBookmark, E>> {
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

    /// Latch the link as poisoned outside a session funnel: the transport
    /// errored, or a driver was dropped with staged bytes it never
    /// replayed, so the control stream's position can no longer be trusted.
    pub(crate) fn poison(&mut self) {
        self.poisoned = true;
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
    /// Both ends of a connection must construct their links around the
    /// same fresh transport at the same time; the bookkeeping the two ends
    /// then keep in step is [`SessionState`]'s.
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
    /// This is how a wrapper (fault injection, byte capture, an adversity
    /// harness) interposes on an existing link: decorate the parts, then
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
    /// connection count sessions in lockstep, so carrying anything but this
    /// link's own current state (another link's, or a stale copy from
    /// before a session ran) would mislabel every stream of the next
    /// session, or let sessions run on a link whose control stream rests
    /// mid-frame.
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
/// its writer blocks on its reader.
///
/// Sessions are live at any positive capacity (the contract's flow-control
/// clause), so the buffer sets the pipe's granularity, not its correctness:
/// frames smaller than this cross without a blocking round trip, larger
/// transfers stream through in refills, and [`memory_with_capacity`] picks
/// other sizes down to one byte.
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
/// bounded channel, so sessions driven over it work under any executor,
/// including the deterministic single-poll harness the crate's own tests
/// use. Each stream buffers 8 KiB; use [`memory_with_capacity`] to pick
/// the buffer size (down to one byte: sessions stay live at any positive
/// capacity).
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

/// The in-memory [`Connector`]: each open creates a bounded pipe and announces
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
pub mod routed;

#[cfg(test)]
mod tests;
