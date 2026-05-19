//! Wire-equivalence helper: drive `Remote::gossip` over an in-memory
//! `tokio::io::duplex` pipe on a current-thread runtime. Used by
//! [`pairwise::process_matches_wire_gossip`] to assert that the wire
//! protocol produces the same per-peer state as bidirectional
//! `Local::process`.
//!
//! [`pairwise::process_matches_wire_gossip`]: crate::pairwise

use std::cell::OnceCell;
use std::future::Future;

use borsh::{BorshDeserialize, BorshSerialize};
use rumors::{Local, Remote};
use tokio::runtime::Runtime;

thread_local! {
    /// One current-thread tokio runtime per test thread, reused
    /// across cases so proptest doesn't pay the cost of spinning a
    /// runtime up per generated example.
    static RT: OnceCell<Runtime> = const { OnceCell::new() };
}

fn block_on<F: Future>(fut: F) -> F::Output {
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

/// Capacity in bytes for the in-memory duplex pipe. The mirror
/// protocol strictly alternates within a session, so a modest buffer
/// is sufficient and naturally exercises backpressure.
const DUPLEX_BUF: usize = 8 * 1024;

/// Gossip two `Local`s through the on-wire protocol and return the
/// reconciled pair. After this returns, the two `Local`s are equal.
pub fn wire_gossip<T>(a: Local<T>, b: Local<T>) -> (Local<T>, Local<T>)
where
    T: Clone + BorshSerialize + BorshDeserialize + Send + 'static,
{
    block_on(async move {
        let (a_side, b_side) = tokio::io::duplex(DUPLEX_BUF);
        let (a_r, a_w) = tokio::io::split(a_side);
        let (b_r, b_w) = tokio::io::split(b_side);

        let mut peer_a = Remote::<T, _, _>::new(a_r, a_w);
        let mut peer_b = Remote::<T, _, _>::new(b_r, b_w);

        let (a_result, b_result) = tokio::join!(
            peer_a.gossip(a, |_, _, _| {}),
            peer_b.gossip(b, |_, _, _| {}),
        );
        (
            a_result.expect("wire gossip A"),
            b_result.expect("wire gossip B"),
        )
    })
}
