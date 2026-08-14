//! Wire helpers for the *asynchronous* gossip path.
//!
//! These drive `rumors::Rumors::gossip` over an in-memory [`rumors::link`]
//! pair with both peers polled concurrently via `tokio::join!`. The two
//! tasks progress directly against each other through the link's streams;
//! no runtime is required unless a caller explicitly spawns a task.

use std::cell::OnceCell;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use borsh::{BorshDeserialize, BorshSerialize};
use rumors::link::MemoryLink;
use rumors::{Bookmark, Peer, Protocol, Rumors, testing::run_to_quiescence};
use tokio::io::{AsyncRead, ReadBuf};
use tokio::runtime::Runtime;

// clippy's `missing_const_for_thread_local` misreads `thread_local!`'s
// fallback-TLS lowering (illumos among the gate's targets) and denies
// initializers that already sit in `const` blocks; the allow keeps
// `-D warnings` honest on every platform the gate runs.
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

/// Capacity in bytes for each in-memory link stream. A modest buffer is
/// sufficient and naturally exercises per-stream backpressure.
pub const LINK_BUF: usize = 8 * 1024;

/// Assert a completed session drained the control stream in both directions.
///
/// The invariant: after any *successful* session, each side has consumed
/// every control byte its peer wrote — nothing rests buffered toward either
/// end. A leftover byte would sit exactly where the link's next session (or
/// this session's epilogue marker) reads, surfacing later as a confusing
/// protocol violation; this assert turns that latent desynchronization into
/// an immediate failure at the session that caused it.
///
/// Consumes the pair, so it is the last act of a test (or harness) that
/// owns both ends. The probe is a no-waker poll of each end's control read
/// half: `Pending` (nothing buffered, writer still open) and end-of-stream
/// both witness a drained direction, while any delivered byte fails the
/// assert with the leftover bytes in the message. Sessions that end in an
/// error are out of scope — they poison the link mid-frame by design.
#[track_caller]
pub fn assert_control_drained(a: MemoryLink, b: MemoryLink) {
    let toward_a = unread_control_bytes(a.into_parts().control_read);
    let toward_b = unread_control_bytes(b.into_parts().control_read);
    assert!(
        toward_a.is_empty() && toward_b.is_empty(),
        "control stream not drained at the session boundary: \
         {} unread byte(s) toward A {:02x?}; {} unread byte(s) toward B {:02x?}",
        toward_a.len(),
        toward_a,
        toward_b.len(),
        toward_b,
    );
}

/// Collect every byte one control read half can yield without waiting.
///
/// Polls with a no-op waker, so the probe never blocks: it stops at
/// `Pending` (buffer empty, writer still open) or at end-of-stream. The
/// in-memory link's duplex pipes deliver written bytes to the reader's
/// buffer synchronously, so "nothing readable now" is "nothing in flight".
fn unread_control_bytes<R: AsyncRead + Unpin>(mut read: R) -> Vec<u8> {
    let mut cx = Context::from_waker(Waker::noop());
    let mut unread = Vec::new();
    loop {
        let mut chunk = [0u8; 64];
        let mut buf = ReadBuf::new(&mut chunk);
        match Pin::new(&mut read).poll_read(&mut cx, &mut buf) {
            Poll::Pending => return unread,
            Poll::Ready(Ok(())) if buf.filled().is_empty() => return unread,
            Poll::Ready(Ok(())) => unread.extend_from_slice(buf.filled()),
            Poll::Ready(Err(e)) => panic!("control-stream drain probe failed: {e}"),
        }
    }
}

/// Gossip two async `Rumors` through the on-wire protocol. After this
/// returns, the two rumor sets hold the same live content and version.
///
/// Both ends drive `gossip` concurrently over the two ends of one in-memory
/// link, so the session makes real bidirectional progress rather than
/// serializing one peer behind the other.
#[track_caller]
pub fn wire_gossip<T>(a: &Rumors<T>, b: &Rumors<T>)
where
    T: BorshSerialize + BorshDeserialize + Send + Sync + 'static,
{
    block_on(wire_gossip_async(a, b));
}

/// Awaitable core of [`wire_gossip`], for callers already inside an async
/// block on this thread's runtime (where a nested [`block_on`] would panic).
///
/// Generic over each side's bookmark, so bookmark suites drive their
/// instrumented peers through it too.
pub async fn wire_gossip_async<T, BA, BB>(a: &Rumors<T, BA>, b: &Rumors<T, BB>)
where
    T: BorshSerialize + BorshDeserialize + Send + Sync + 'static,
    BA: Bookmark + Send + Sync + std::fmt::Debug + 'static,
    BA::Error: std::fmt::Debug,
    BB: Bookmark + Send + Sync + std::fmt::Debug + 'static,
    BB::Error: std::fmt::Debug,
{
    let (mut a_link, mut b_link) = rumors::link::memory_with_capacity(LINK_BUF);

    let (a_result, b_result) = tokio::join!(a.gossip(&mut a_link), b.gossip(&mut b_link));
    a_result.expect("wire gossip A");
    b_result.expect("wire gossip B");
    assert_control_drained(a_link, b_link);
}

/// Mint a genuine, party-disjoint `Rumors` from `parent` by serving it a
/// bootstrap over an in-memory link.
///
/// This is how a test obtains a second *originator*: the returned peer
/// descends from `parent`'s universe (same [`Network`](rumors::Network))
/// with its own disjoint party region and a copy of `parent`'s content,
/// exactly as a real process joining over the network would. `parent` keeps
/// its own party (the bootstrap hands the newcomer a freshly-forked slice
/// of it, in the same critical section that snapshots the served tree).
#[track_caller]
pub fn bootstrap_fork<T>(parent: &Rumors<T>) -> Rumors<T>
where
    T: BorshSerialize + BorshDeserialize + Send + Sync + Clone + 'static,
{
    block_on(bootstrap_fork_async_with_protocol(parent, Protocol::V2))
}

/// Awaitable core of [`bootstrap_fork`], for callers already inside an async
/// block on this thread's runtime (where a nested [`block_on`] would panic).
///
/// Generic over the parent's bookmark, so bookmark suites fork from their
/// instrumented peers too; the minted peer itself arrives unbookmarked.
pub async fn bootstrap_fork_async<T, B>(parent: &Rumors<T, B>) -> Rumors<T>
where
    T: BorshSerialize + BorshDeserialize + Send + Sync + Clone + 'static,
    B: Bookmark + Send + Sync + std::fmt::Debug + 'static,
    B::Error: std::fmt::Debug,
{
    bootstrap_fork_async_with_protocol(parent, Protocol::V2).await
}

/// Mint a disjoint peer using an explicitly selected wire protocol.
pub async fn bootstrap_fork_async_with_protocol<T, B>(
    parent: &Rumors<T, B>,
    protocol: Protocol,
) -> Rumors<T>
where
    T: BorshSerialize + BorshDeserialize + Send + Sync + Clone + 'static,
    B: Bookmark + Send + Sync + std::fmt::Debug + 'static,
    B::Error: std::fmt::Debug,
{
    let (mut parent_link, mut boot_link) = rumors::link::memory_with_capacity(LINK_BUF);

    let (server_out, boot_out) = tokio::join!(
        parent.gossip(&mut parent_link),
        Peer::<T>::bootstrap()
            .protocol(protocol)
            .join(&mut boot_link),
    );
    server_out.expect("bootstrap server gossip");
    // Test peers pin the serialization floor explicitly, keeping the
    // capacity-one orderings the deadlock-freedom argument certifies
    // exercised; suites that want a wider window configure a budget.
    let minted = boot_out
        .expect("bootstrap handshake")
        .expect("parent served the bootstrap")
        .sync_window_floor()
        .into_rumors();
    assert_control_drained(parent_link, boot_link);
    minted
}
