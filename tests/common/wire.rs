//! Wire-equivalence helper: drive `Known::gossip` over an in-memory
//! `tokio::io::duplex` pipe on a current-thread runtime. Used by
//! [`pairwise::process_matches_wire_gossip`] to assert that the wire
//! protocol produces the same per-peer state as bidirectional
//! `Known::learn`.
//!
//! Inputs and outputs are [`rumors::sync::Known`] so the helper plugs
//! into the rest of the simulation suite, which is built around the
//! synchronous surface; the bridge to the async wire happens inside.
//!
//! [`pairwise::process_matches_wire_gossip`]: crate::pairwise

use std::cell::OnceCell;
use std::future::Future;

use borsh::{BorshDeserialize, BorshSerialize};
use rumors::sync::Known;
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

/// Gossip two `sync::Known`s through the on-wire protocol and return
/// the reconciled pair. After this returns, the two `Known`s are equal.
pub fn wire_gossip<T>(a: Known<T>, b: Known<T>) -> (Known<T>, Known<T>)
where
    T: Clone + BorshSerialize + BorshDeserialize + Send + Sync + 'static,
{
    block_on(async move {
        let (a_side, b_side) = tokio::io::duplex(DUPLEX_BUF);
        let (mut a_r, mut a_w) = tokio::io::split(a_side);
        let (mut b_r, mut b_w) = tokio::io::split(b_side);

        // Bridge sync::Known -> async Known for the wire, then wrap back
        // into sync::Known on the way out.
        let (a_result, b_result) = tokio::join!(
            a.0.gossip(&mut a_r, &mut a_w),
            b.0.gossip(&mut b_r, &mut b_w),
        );
        (
            Known(a_result.expect("wire gossip A")),
            Known(b_result.expect("wire gossip B")),
        )
    })
}
