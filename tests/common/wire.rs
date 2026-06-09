//! Wire-equivalence helper for the *asynchronous* gossip path: drive
//! `rumors::Known::gossip` over an in-memory `tokio::io::duplex` pipe with
//! both peers running concurrently via `tokio::join!` on a current-thread
//! runtime. Mirrors `sync_wire.rs`, which drives the synchronous
//! `sync::Known::gossip` path. Used by the `async_wire` integration test.
//!
//! Peers are the public asynchronous [`rumors::Known`] (not the synchronous
//! wrapper), so this helper exercises the genuinely-concurrent async
//! protocol — two tasks making progress against each other through the
//! duplex — rather than a single thread blocking on the bridged future.

use std::cell::OnceCell;
use std::future::Future;

use borsh::{BorshDeserialize, BorshSerialize};
use rumors::Known;
use tokio::runtime::Runtime;

thread_local! {
    /// One current-thread tokio runtime per test thread, reused across
    /// cases so proptest doesn't pay the cost of spinning a runtime up per
    /// generated example.
    static RT: OnceCell<Runtime> = const { OnceCell::new() };
}

/// Block on `fut` using this thread's reused current-thread runtime.
///
/// Exposed so the `async_wire` test can build its asynchronous peers (whose
/// `message_then` inserts are `async`) on the same runtime it later gossips
/// them over.
pub fn block_on<F: Future>(fut: F) -> F::Output {
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

/// Gossip two async `Known`s through the on-wire protocol and return the
/// reconciled pair. After this returns, the two `Known`s are equal.
///
/// Both ends drive `gossip` concurrently over the two halves of a single
/// `tokio::io::duplex` pipe, so the session makes real bidirectional
/// progress rather than serializing one peer behind the other.
pub fn wire_gossip<T>(a: Known<T>, b: Known<T>) -> (Known<T>, Known<T>)
where
    T: BorshSerialize + BorshDeserialize + Send + Sync + 'static,
{
    block_on(async move {
        let (a_side, b_side) = tokio::io::duplex(DUPLEX_BUF);
        let (mut a_r, mut a_w) = tokio::io::split(a_side);
        let (mut b_r, mut b_w) = tokio::io::split(b_side);

        let (a_result, b_result) =
            tokio::join!(a.gossip(&mut a_r, &mut a_w), b.gossip(&mut b_r, &mut b_w),);
        (
            a_result.expect("wire gossip A"),
            b_result.expect("wire gossip B"),
        )
    })
}

/// Mint a genuine, party-disjoint `Known` from `parent` by serving it a
/// bootstrap over an in-memory pipe.
///
/// Now that `fork` is gone, this is how a test obtains a second *originator*:
/// the returned peer descends from `parent`'s universe (same [`Network`]) with
/// its own disjoint party region and a copy of `parent`'s content, exactly as a
/// real process joining over the network would. `parent` keeps its own party
/// (the bootstrap hands the newcomer a freshly-forked slice of it).
///
/// Use this — not [`rumors`](Known::rumors) — wherever the second peer must go
/// on to [`message`](Known::message) or [`redact`](Known::redact): a `rumors`
/// snapshot is non-canonical and shares `parent`'s party, so two such snapshots
/// would originate *non-concurrent* versions and break the gossip merge.
pub fn bootstrap_fork<T>(parent: &Known<T>) -> Known<T>
where
    T: BorshSerialize + BorshDeserialize + Send + Sync + Clone + 'static,
{
    block_on(bootstrap_fork_async(parent))
}

/// Awaitable core of [`bootstrap_fork`], for callers already inside an async
/// block on this thread's runtime (where a nested [`block_on`] would panic).
pub async fn bootstrap_fork_async<T>(parent: &Known<T>) -> Known<T>
where
    T: BorshSerialize + BorshDeserialize + Send + Sync + Clone + 'static,
{
    let (a_side, b_side) = tokio::io::duplex(DUPLEX_BUF);
    let (mut a_r, mut a_w) = tokio::io::split(a_side);
    let (mut b_r, mut b_w) = tokio::io::split(b_side);

    // `parent.rumors()` shares parent's party; serving the bootstrap forks a
    // disjoint slice off it for the newcomer (mutating the shared party in
    // place), so afterward parent and newcomer are pairwise disjoint.
    let server = parent.rumors();
    let (server_out, boot_out) = tokio::join!(
        server.gossip(&mut a_r, &mut a_w),
        Known::<T>::bootstrap(&mut b_r, &mut b_w),
    );
    server_out.expect("bootstrap server gossip");
    boot_out
        .expect("bootstrap handshake")
        .expect("parent served the bootstrap")
}
