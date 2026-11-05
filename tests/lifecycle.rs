//! End-to-end lifecycle pins for the V2 session epilogue, at the public-API
//! tier.
//!
//! The crate docs' "What a session promises" section states the contract
//! these tests hold the public API to: `Ok` certifies both replicas
//! committed and leaves the connection at the session boundary, reusable;
//! `Err` and cancellation publish no partial reconciliation and leave the
//! connection at an unknown position, to be discarded. Losing the final
//! marker fails in [`rumors::error::Phase::Completion`], after local work
//! has committed. The in-crate unit tests (`src/tests.rs`) pin the same
//! mechanisms at exact byte boundaries with forged peers; these are their
//! integration-tier complements, driving whole sessions between real
//! replicas.

use rumors_testkit::common;
#[path = "lifecycle/reunion.rs"]
mod reunion;

use std::future::Future;
use std::io::{self, IoSlice};
use std::pin::{Pin, pin};
use std::task::{Context, Poll, ready};

use futures::task::noop_waker_ref;
use rumors::testing::{
    COMPLETION_MARKER, IoFault, IoFaultUnit, IoOperation, IoPlan, IoSide, wrap_io,
};
use rumors::{Error, Peer, Rumors};
use tokio::io::AsyncWrite;

use crate::common::wire::{block_on, bootstrap_fork_async, wire_gossip};

/// Messages each side commits before a divergent session: enough that
/// reconciliation carries frames on several logical streams and takes many
/// polls to complete.
const DIVERGENT_MESSAGES: u64 = 32;

/// The most polls granted to the joined in-flight sessions while waiting
/// for both ends to flush frames.
///
/// Under a waker that never fires, each poll moves a session forward by
/// about one frame, and one end writes its first frame only after reading
/// dozens of the other's, so reaching both ends' first frame flush takes
/// dozens of polls. The cap only turns a session that
/// never gets there into a failure instead of a hang.
const MAX_MID_FLIGHT_POLLS: usize = 100_000;

/// Build a party-disjoint pair with [`DIVERGENT_MESSAGES`] unshared
/// messages committed on each side.
///
/// Construction is deterministic apart from the random network id (which
/// has a fixed wire length): versions derive from the bootstrap order and
/// message identity from the version alone, so two calls build pairs whose
/// gossip sessions are byte-for-byte the same size. The epilogue-residue
/// test's measure-then-replay rests on this.
async fn divergent_pair() -> (Rumors<u64>, Rumors<u64>) {
    let a: Rumors<u64> = Peer::seed().sync_memory_budget(0).into_rumors();
    let b = bootstrap_fork_async(&a).await;
    for v in 0..DIVERGENT_MESSAGES {
        a.send(v).unwrap();
        b.send(1_000 + v).unwrap();
    }
    (a, b)
}

/// A write half that records, at each successful flush, how many bytes it
/// had accepted.
struct FlushLog<W> {
    /// The write half the bytes go to.
    inner: W,
    /// Bytes `inner` has accepted.
    written: usize,
    /// The value of `written` at each successful flush, in order.
    flushed_at: Vec<usize>,
}

impl<W> FlushLog<W> {
    /// Wraps `inner`, with nothing written or flushed yet.
    fn new(inner: W) -> Self {
        Self {
            inner,
            written: 0,
            flushed_at: Vec::new(),
        }
    }
}

/// Forwards every operation to the inner write half, counting accepted
/// bytes and logging successful flushes.
impl<W: AsyncWrite + Unpin> AsyncWrite for FlushLog<W> {
    /// Writes through, counting the bytes accepted.
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let written = ready!(Pin::new(&mut self.inner).poll_write(cx, buf))?;
        self.written += written;
        Poll::Ready(Ok(written))
    }

    /// Writes the slices through, counting the bytes accepted.
    fn poll_write_vectored(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[IoSlice<'_>],
    ) -> Poll<io::Result<usize>> {
        let written = ready!(Pin::new(&mut self.inner).poll_write_vectored(cx, bufs))?;
        self.written += written;
        Poll::Ready(Ok(written))
    }

    /// Reports the inner half's vectored-write support.
    fn is_write_vectored(&self) -> bool {
        self.inner.is_write_vectored()
    }

    /// Flushes through, logging the bytes accepted so far on success.
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        ready!(Pin::new(&mut self.inner).poll_flush(cx))?;
        let written = self.written;
        self.flushed_at.push(written);
        Poll::Ready(Ok(()))
    }

    /// Shuts the inner half down.
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

/// The flushes an end makes before its first frame, counted from a session
/// that writes none.
///
/// A session between equal replicas compares nothing, so it writes no
/// frames: every flush either end makes before it writes its completion
/// marker belongs to the handshake that opens every session. This runs one
/// such session, over halves that log each flush, and counts the flushes
/// each end made before the marker's bytes, which are the last it writes.
/// An end of another session that has flushed more often than this has
/// flushed frames.
fn handshake_flushes() -> usize {
    let (a, b) = block_on(async {
        let a: Rumors<u64> = Peer::seed().sync_memory_budget(0).into_rumors();
        let b = bootstrap_fork_async(&a).await;
        (a, b)
    });
    let (a_end, b_end) = rumors::testing::memory_connection(rumors::testing::MEMORY_CAPACITY);
    let (a_out, b_out) = block_on(async {
        tokio::join!(
            a.gossip_once(a_end.read, FlushLog::new(a_end.write)),
            b.gossip_once(b_end.read, FlushLog::new(b_end.write)),
        )
    });
    let ends = [a_out, b_out].map(|out| {
        let (gossiped, _, log) = out.expect("a session between equal replicas completes");
        assert_eq!(
            gossiped.stats.frames_sent, 0,
            "equal replicas exchange no frames"
        );
        let before_marker = log.written - COMPLETION_MARKER.len();
        log.flushed_at
            .iter()
            .filter(|&&written| written <= before_marker)
            .count()
    });
    assert!(
        ends.iter().all(|&flushes| flushes > 0),
        "every end flushes its handshake: {ends:?}"
    );
    ends.into_iter().max().expect("a session has two ends")
}

/// A gossip session cancelled mid-descent leaves both replicas unharmed:
/// nothing of the interrupted session is published, and a fresh
/// connection converges the pair fully.
///
/// The interrupted connection's position is unknown and is discarded, as
/// every cancellation requires.
#[test]
fn a_session_cancelled_mid_descent_leaves_both_replicas_intact() {
    let (a, b) = block_on(divergent_pair());
    let (a_before, b_before) = (a.snapshot(), b.snapshot());
    let (a_end, b_end) = rumors::testing::memory_connection(rumors::testing::MEMORY_CAPACITY);
    let (mut a_read, mut a_write, a_report) = wrap_io(
        IoSide::Left,
        IoPlan::default(),
        a_end.read.into_inner(),
        a_end.write,
    );
    let (mut b_read, mut b_write, b_report) = wrap_io(
        IoSide::Right,
        IoPlan::default(),
        b_end.read.into_inner(),
        b_end.write,
    );

    {
        let mut cx = Context::from_waker(noop_waker_ref());
        let mut session = pin!(async {
            tokio::join!(
                a.gossip_once(&mut a_read, &mut a_write),
                b.gossip_once(&mut b_read, &mut b_write)
            )
        });
        // Poll until each end has flushed frames, asserting at every poll
        // that the joined sessions are still in flight: the cancellation
        // then provably lands mid-descent in both directions.
        let handshake = handshake_flushes();
        let both_flushed_frames =
            || a_report.snapshot().flushes > handshake && b_report.snapshot().flushes > handshake;
        let mut polls = 0;
        while !both_flushed_frames() {
            assert!(
                polls < MAX_MID_FLIGHT_POLLS,
                "both ends must flush frames within {MAX_MID_FLIGHT_POLLS} polls"
            );
            assert!(
                session.as_mut().poll(&mut cx).is_pending(),
                "the joined sessions must still be in flight when cancelled"
            );
            polls += 1;
        }
        // Dropping the joined future here cancels both sessions mid-descent.
    }
    assert_eq!(
        a.snapshot(),
        a_before,
        "A published nothing of the cut session"
    );
    assert_eq!(
        b.snapshot(),
        b_before,
        "B published nothing of the cut session"
    );

    // The replicas themselves are unharmed: a fresh connection converges
    // the pair.
    wire_gossip(&a, &b);
    assert_eq!(a.snapshot().hash(), b.snapshot().hash());
    assert_eq!(a.snapshot().len(), 2 * DIVERGENT_MESSAGES as usize);
}

/// Immediate teardown after `Ok` is safe under the epilogue.
///
/// Each side drops both its halves the instant its own `Ok` lands, so the
/// slower side must conclude its session, through the peer's epilogue
/// marker, from bytes already buffered in the connection. Both sides
/// conclude `Ok`, committed and converged.
#[test]
fn dropping_the_connection_immediately_after_ok_is_clean() {
    let (a, b) = block_on(divergent_pair());
    let (a_end, b_end) = rumors::testing::memory_connection(rumors::testing::MEMORY_CAPACITY);
    let (a_out, b_out) = block_on(async {
        tokio::join!(
            async {
                // A's halves drop right here, at its own outcome.
                a.gossip_once(a_end.read, a_end.write)
                    .await
                    .map(|(gossiped, ..)| gossiped)
            },
            async {
                b.gossip_once(b_end.read, b_end.write)
                    .await
                    .map(|(gossiped, ..)| gossiped)
            },
        )
    });
    a_out.expect("A's session");
    b_out.expect("B must conclude Ok off buffered bytes after A's teardown");
    assert_eq!(a.snapshot().hash(), b.snapshot().hash());
    assert_eq!(a.snapshot().len(), 2 * DIVERGENT_MESSAGES as usize);
}

/// Losing completion confirmation does not undo either peer's local commit.
///
/// With A's final completion marker withheld (B's read budget is one byte
/// short of a clean session, measured from a byte-identical probe run),
/// A's `Ok` still lands, while B fails in [`rumors::error::Phase::Completion`] rather
/// than any pre-commit class, and B's replica nonetheless holds the fully
/// reconciled content. A cut landing anywhere earlier would fail these
/// assertions with a different error class and missing content, so the
/// test also validates its own budget placement.
#[test]
fn a_lost_epilogue_marker_is_distinguished_and_post_commit() {
    // Probe: measure B's total incoming bytes across one clean divergent
    // session. `divergent_pair` builds byte-identical universes, so the
    // replay below sees the same byte schedule.
    let clean_read_bytes = {
        let (a, b) = block_on(divergent_pair());
        let (a_end, b_end) = rumors::testing::memory_connection(rumors::testing::MEMORY_CAPACITY);
        let (b_read, b_write, report) = wrap_io(
            IoSide::Right,
            IoPlan::default(),
            b_end.read.into_inner(),
            b_end.write,
        );
        let (a_out, b_out) = block_on(async {
            tokio::join!(
                a.gossip_once(a_end.read, a_end.write),
                b.gossip_once(b_read, b_write)
            )
        });
        a_out.expect("probe session A");
        let (probe, ..) = b_out.expect("probe session B");
        assert!(
            probe.stats.frames_received >= 2,
            "the divergence must be deep enough to carry several frames, got {}",
            probe.stats.frames_received
        );
        report.snapshot().read_bytes
    };

    // Replay on an identically-built pair, with B's reads failing after
    // every byte but the last: the lost byte is A's completion marker, the
    // final byte a clean session delivers to B.
    let (a, b) = block_on(divergent_pair());
    let (a_end, b_end) = rumors::testing::memory_connection(rumors::testing::MEMORY_CAPACITY);
    let plan = IoPlan {
        fault: Some(IoFault {
            operation: IoOperation::Read,
            after: clean_read_bytes - 1,
            unit: IoFaultUnit::Bytes,
        }),
        ..IoPlan::default()
    };
    let (b_read, b_write, _report) =
        wrap_io(IoSide::Right, plan, b_end.read.into_inner(), b_end.write);
    let (a_out, b_out) = block_on(async {
        // Each future owns its halves: A's teardown after its `Ok` surfaces
        // as end-of-stream to B if it wins the race against the injected
        // cut; either way B's marker read fails, post-commit.
        tokio::join!(
            async {
                a.gossip_once(a_end.read, a_end.write)
                    .await
                    .map(|(gossiped, ..)| gossiped)
            },
            async {
                b.gossip_once(b_read, b_write)
                    .await
                    .map(|(gossiped, ..)| gossiped)
            },
        )
    });

    // B's own marker went out before its failing read, so A's Ok lands:
    // from A's seat this is a fully certified session.
    a_out.expect("A's session concludes Ok; only its marker toward B is lost");
    // B committed before its epilogue, so the failure is the distinguished
    // post-commit residue, not a session-failure class.
    assert!(
        matches!(b_out, Err(Error::Transport(ref error)) if error.context.phase == rumors::error::Phase::Completion),
        "a lost final marker must surface as the completion-phase transport failure, got {b_out:?}"
    );
    // Post-commit means exactly this: B holds the reconciled content.
    assert_eq!(
        a.snapshot().hash(),
        b.snapshot().hash(),
        "B's replica must hold the reconciled content despite the Err"
    );
    assert_eq!(b.snapshot().len(), 2 * DIVERGENT_MESSAGES as usize);
}
