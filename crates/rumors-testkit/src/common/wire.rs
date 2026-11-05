//! Wire helpers for gossip sessions.
//!
//! These drive `rumors::Rumors::gossip_once` over an in-memory connection
//! ([`rumors::testing::memory_connection`]) with both peers polled
//! concurrently via `tokio::join!`. The two tasks progress directly against
//! each other through the connection's pipes; no runtime is required unless
//! a caller explicitly spawns a task.

use std::cell::OnceCell;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use rumors::testing::{MEMORY_CAPACITY, MemoryEnd, memory_connection};
use rumors::{Bootstrap, Peer, Rumors, testing::run_to_quiescence};
use tokio::io::{AsyncRead, ReadBuf};
use tokio::runtime::Runtime;

use crate::common::budget::BudgetChoice;

use serde::Serialize;
use serde::de::DeserializeOwned;

/// Bytes each direction of an in-memory connection buffers when a test needs
/// room for everything a session writes: 64 KiB.
///
/// [`MEMORY_CAPACITY`] suffices for liveness, since sessions progress at any
/// positive capacity. Some tests need a write to complete without its reader:
/// a side that finishes one session and writes its next preamble before its
/// peer reads, or a side writing to a peer that never reads. The suites'
/// fixtures write well under 64 KiB per session, so on a connection this
/// roomy no such write waits on its reader.
pub const ROOMY_CAPACITY: usize = 64 * 1024;
// clippy's `missing_const_for_thread_local` misreads `thread_local!`'s
// fallback-TLS lowering (illumos among the gate's targets) and denies
// initializers that already sit in `const` blocks; the allow keeps
// `-D warnings` passing on every platform the gate runs.
thread_local! {
    /// One current-thread tokio runtime per test thread, reused across
    /// cases so proptest doesn't pay the cost of spinning a runtime up per
    /// generated example.
    #[allow(clippy::missing_const_for_thread_local)]
    static RT: OnceCell<Runtime> = const { OnceCell::new() };
}

/// Drive a closed, in-memory future until it completes or stops making progress.
///
/// A protocol deadlock therefore fails at its source instead of parking the
/// entire test process indefinitely. Futures which depend on external events
/// must use [`tokio_block_on`] instead.
#[track_caller]
pub fn block_on<F: Future>(fut: F) -> F::Output {
    run_to_quiescence(fut).expect("closed in-memory future became quiescent")
}

/// Block on `future` using this thread's reused current-thread Tokio runtime.
///
/// Tests should use this only when the behavior under test explicitly needs
/// Tokio facilities such as task spawning, timers, or networking. Ordinary
/// protocol futures should use [`block_on`].
pub fn tokio_block_on<F: Future>(fut: F) -> F::Output {
    RT.with(|cell| {
        cell.get_or_init(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("build tokio current-thread runtime")
        })
        .block_on(fut)
    })
}

/// Assert a completed session drained the connection in both directions.
///
/// The invariant: after any *successful* session, each side has consumed
/// every byte its peer wrote — nothing rests buffered toward either end. A
/// leftover byte would sit exactly where the connection's next session
/// reads, surfacing later as a confusing protocol violation; this assert
/// turns that latent desynchronization into an immediate failure at the
/// session that caused it.
///
/// Consumes the pair, so it is the last act of a test (or harness) that
/// owns both ends. The probe is a no-waker poll of each end's read half:
/// `Pending` (nothing buffered, writer still open) and end-of-stream both
/// show a drained direction, while any delivered byte fails the assert
/// with the leftover bytes in the message. Sessions that end in an error
/// are out of scope: they leave the connection mid-item.
#[track_caller]
pub fn assert_connection_drained(a: MemoryEnd, b: MemoryEnd) {
    let toward_a = unread_bytes(a.read);
    let toward_b = unread_bytes(b.read);
    assert!(
        toward_a.is_empty() && toward_b.is_empty(),
        "connection not drained at the session boundary: \
         {} unread byte(s) toward A {:02x?}; {} unread byte(s) toward B {:02x?}",
        toward_a.len(),
        toward_a,
        toward_b.len(),
        toward_b,
    );
}

/// Collect every byte one read half can yield without waiting.
///
/// Polls with a no-op waker, so the probe never blocks: it stops at
/// `Pending` (buffer empty, writer still open) or at end-of-stream. The
/// in-memory connection's duplex pipes deliver written bytes to the
/// reader's buffer synchronously, so "nothing readable now" is "nothing in
/// flight".
fn unread_bytes<R: AsyncRead + Unpin>(mut read: R) -> Vec<u8> {
    let mut cx = Context::from_waker(Waker::noop());
    let mut unread = Vec::new();
    loop {
        let mut chunk = [0u8; 64];
        let mut buf = ReadBuf::new(&mut chunk);
        match Pin::new(&mut read).poll_read(&mut cx, &mut buf) {
            Poll::Pending => return unread,
            Poll::Ready(Ok(())) if buf.filled().is_empty() => return unread,
            Poll::Ready(Ok(())) => unread.extend_from_slice(buf.filled()),
            Poll::Ready(Err(e)) => panic!("connection drain probe failed: {e}"),
        }
    }
}

/// Gossip two async `Rumors` through the on-wire protocol. After this
/// returns, the two rumor sets hold the same live content and version.
///
/// Both ends drive `gossip_once` concurrently over the two ends of one in-memory
/// connection, so the session makes real bidirectional progress rather than
/// serializing one peer behind the other.
#[track_caller]
pub fn wire_gossip<T>(a: &Rumors<T>, b: &Rumors<T>)
where
    T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
{
    block_on(wire_gossip_async(a, b));
}

/// Awaitable core of [`wire_gossip`], for callers already inside an async
/// block on this thread's runtime (where a nested [`block_on`] would panic).
pub async fn wire_gossip_async<T>(a: &Rumors<T>, b: &Rumors<T>)
where
    T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
{
    let _ = gossip_pair_async(a, b).await;
}

/// [`wire_gossip_async`], returning both endpoints' session reports so
/// callers can assert on [`SessionStats`](rumors::SessionStats).
///
/// Checks the connection drained, like every successful in-memory session.
pub async fn gossip_pair_async<T>(
    a: &Rumors<T>,
    b: &Rumors<T>,
) -> (rumors::Gossiped, rumors::Gossiped)
where
    T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
{
    let (mut a_end, mut b_end) = memory_connection(MEMORY_CAPACITY);

    let (a_result, b_result) = tokio::join!(
        a.gossip_once(&mut a_end.read, &mut a_end.write),
        b.gossip_once(&mut b_end.read, &mut b_end.write)
    );
    let (a_report, ..) = a_result.expect("wire gossip A");
    let (b_report, ..) = b_result.expect("wire gossip B");
    assert_connection_drained(a_end, b_end);
    (a_report, b_report)
}

/// Give each side `values_per_side` values the other lacks: `a` draws
/// from one range, `b` from a disjoint one, so the pair ends fully
/// divergent (every leaf disputed in their next session).
pub fn diverge(a: &Rumors<u64>, b: &Rumors<u64>, values_per_side: u64) {
    a.send_all(0..values_per_side).unwrap();
    b.send_all((0..values_per_side).map(|v| 1_000_000 + v))
        .unwrap();
}

/// Join the parent's network with an independent peer holding its messages.
///
/// The new peer runs in the serial configuration ([`BudgetChoice::Serial`])
/// to exercise backpressure. The shared driver checks that bootstrap completes
/// and drains the connection.
#[track_caller]
pub fn bootstrap_fork<T>(parent: &Rumors<T>) -> Rumors<T>
where
    T: Serialize + DeserializeOwned + Eq + Send + Sync + Clone + 'static,
{
    block_on(bootstrap_fork_async(parent))
}

/// Awaitable [`bootstrap_fork`], for composing bootstrap with other work.
pub async fn bootstrap_fork_async<T>(parent: &Rumors<T>) -> Rumors<T>
where
    T: Serialize + DeserializeOwned + Eq + Send + Sync + Clone + 'static,
{
    bootstrap_fork_configured(parent, Peer::bootstrap(), BudgetChoice::Serial).await
}

/// Join the parent's network, then configure the new peer's budget.
#[track_caller]
pub fn bootstrap_fork_with_budget<T>(parent: &Rumors<T>, budget: BudgetChoice) -> Rumors<T>
where
    T: Serialize + DeserializeOwned + Eq + Send + Sync + Clone + 'static,
{
    block_on(bootstrap_fork_with_budget_async(parent, budget))
}

/// Awaitable [`bootstrap_fork_with_budget`].
pub async fn bootstrap_fork_with_budget_async<T>(
    parent: &Rumors<T>,
    budget: BudgetChoice,
) -> Rumors<T>
where
    T: Serialize + DeserializeOwned + Eq + Send + Sync + Clone + 'static,
{
    bootstrap_fork_configured(parent, Peer::bootstrap(), budget).await
}

/// Join through a configured builder, check the connection drained, and set
/// the budget.
///
/// Accepting a builder lets suites observe bootstrap itself. The budget governs
/// later sessions; bootstrap has no disputed subtrees to pipeline.
pub async fn bootstrap_fork_configured<T>(
    parent: &Rumors<T>,
    bootstrap: Bootstrap<T>,
    budget: BudgetChoice,
) -> Rumors<T>
where
    T: Serialize + DeserializeOwned + Eq + Send + Sync + Clone + 'static,
{
    bootstrap_fork_then(parent, bootstrap, |peer| budget.apply(peer)).await
}

/// Join through a configured builder, check the connection drained, and
/// configure the new peer with `configure` before it starts gossiping.
///
/// The general form of [`bootstrap_fork_configured`], for a suite that
/// sets more of the peer than its budget.
pub async fn bootstrap_fork_then<T>(
    parent: &Rumors<T>,
    bootstrap: Bootstrap<T>,
    configure: impl FnOnce(Peer<T>) -> Peer<T>,
) -> Rumors<T>
where
    T: Serialize + DeserializeOwned + Eq + Send + Sync + Clone + 'static,
{
    let (mut parent_end, mut boot_end) = memory_connection(MEMORY_CAPACITY);

    let (server_out, boot_out) = tokio::join!(
        parent.gossip_once(&mut parent_end.read, &mut parent_end.write),
        bootstrap.join(&mut boot_end.read, &mut boot_end.write),
    );
    server_out.expect("bootstrap server gossip");
    let forked = configure(match boot_out {
        rumors::Joined::Joined { peer, .. } => peer,
        _ => panic!("parent served the bootstrap"),
    })
    .into_rumors();
    assert_connection_drained(parent_end, boot_end);
    forked
}
