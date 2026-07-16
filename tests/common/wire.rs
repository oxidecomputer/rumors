//! Wire helpers for the *asynchronous* gossip path: drive
//! `rumors::Rumors::gossip` over an in-memory `tokio::io::duplex` pipe with
//! both peers polled concurrently via `tokio::join!`. The two tasks progress
//! directly against each other through the duplex transport; no runtime is
//! required unless a caller explicitly spawns a task.

use std::cell::OnceCell;
use std::future::Future;

use borsh::{BorshDeserialize, BorshSerialize};
use rumors::{Peer, Protocol, Rumors, testing::run_to_quiescence};
use tokio::runtime::Runtime;

thread_local! {
    /// One current-thread tokio runtime per test thread, reused across
    /// cases so proptest doesn't pay the cost of spinning a runtime up per
    /// generated example.
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

/// Capacity in bytes for the in-memory duplex pipe. The mirror protocol
/// strictly alternates within a session, so a modest buffer is sufficient
/// and naturally exercises backpressure.
const DUPLEX_BUF: usize = 8 * 1024;

/// Gossip two async `Rumors` through the on-wire protocol. After this
/// returns, the two rumor sets hold the same live content and version.
///
/// Both ends drive `gossip` concurrently over the two halves of a single
/// `tokio::io::duplex` pipe, so the session makes real bidirectional
/// progress rather than serializing one peer behind the other.
#[track_caller]
pub fn wire_gossip<T>(a: &Rumors<T>, b: &Rumors<T>)
where
    T: BorshSerialize + BorshDeserialize + Send + Sync + 'static,
{
    block_on(wire_gossip_async(a, b));
}

/// Awaitable core of [`wire_gossip`], for callers already inside an async
/// block on this thread's runtime (where a nested [`block_on`] would panic).
pub async fn wire_gossip_async<T>(a: &Rumors<T>, b: &Rumors<T>)
where
    T: BorshSerialize + BorshDeserialize + Send + Sync + 'static,
{
    let (a_side, b_side) = tokio::io::duplex(DUPLEX_BUF);
    let (mut a_r, mut a_w) = tokio::io::split(a_side);
    let (mut b_r, mut b_w) = tokio::io::split(b_side);

    let (a_result, b_result) =
        tokio::join!(a.gossip(&mut a_r, &mut a_w), b.gossip(&mut b_r, &mut b_w),);
    a_result.expect("wire gossip A");
    b_result.expect("wire gossip B");
}

/// Mint a genuine, party-disjoint `Rumors` from `parent` by serving it a
/// bootstrap over an in-memory pipe.
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
pub async fn bootstrap_fork_async<T>(parent: &Rumors<T>) -> Rumors<T>
where
    T: BorshSerialize + BorshDeserialize + Send + Sync + Clone + 'static,
{
    bootstrap_fork_async_with_protocol(parent, Protocol::V2).await
}

/// Mint a disjoint peer using an explicitly selected wire protocol.
pub async fn bootstrap_fork_async_with_protocol<T>(
    parent: &Rumors<T>,
    protocol: Protocol,
) -> Rumors<T>
where
    T: BorshSerialize + BorshDeserialize + Send + Sync + Clone + 'static,
{
    let (a_side, b_side) = tokio::io::duplex(DUPLEX_BUF);
    let (mut a_r, mut a_w) = tokio::io::split(a_side);
    let (mut b_r, mut b_w) = tokio::io::split(b_side);

    let (server_out, boot_out) = tokio::join!(
        parent.gossip(&mut a_r, &mut a_w),
        Peer::<T>::bootstrap_with_protocol(protocol, &mut b_r, &mut b_w),
    );
    server_out.expect("bootstrap server gossip");
    boot_out
        .expect("bootstrap handshake")
        .expect("parent served the bootstrap")
        .into_rumors()
}
