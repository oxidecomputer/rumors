//! The wire pools' arithmetic and their acquisition order.

use std::pin::pin;
use std::task::{Context, Poll, Waker};

use proptest::prelude::*;

use super::{
    LARGEST_POOL, PERMIT_BYTES, Semaphore, WIRE_POOL_RUNS, WirePermit, WirePool, pool_bytes,
    wire_pools_bytes,
};
use crate::tree::mirror::streaming::{
    ledger::queued,
    remote::{
        Inbound, Outbound,
        codec::{MAX_RUN_BUDGET_BYTES, SUPPLY_FRAME_OVERHEAD},
    },
};

/// One acquisition's `u32` and a semaphore's capacity, as one bound.
fn permit_limit() -> u64 {
    (u32::MAX as u64).min(Semaphore::MAX_PERMITS as u64)
}

/// A permit stands for the least power of two of bytes at which the
/// largest pool any target can size counts within both one acquisition's
/// `u32` and a semaphore's capacity.
#[test]
fn a_permit_is_the_least_granule_that_counts_every_pool() {
    let bytes = PERMIT_BYTES as u64;
    assert!(bytes.is_power_of_two());
    assert!(LARGEST_POOL.div_ceil(bytes) <= permit_limit());
    assert!(bytes == 1 || LARGEST_POOL.div_ceil(bytes / 2) > permit_limit());
    let largest = wire_pools_bytes(usize::MAX);
    assert_eq!(
        largest,
        wire_pools_bytes(MAX_RUN_BUDGET_BYTES),
        "targets past the cap saturate"
    );
}

/// A future's next poll, with a waker that does nothing.
fn poll_once<F: Future>(future: std::pin::Pin<&mut F>) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}

proptest! {
    /// Pools and permits cover exactly their runs, in whole permits.
    ///
    /// A pool stands for whole permits covering its runs' worth at its
    /// target, each run's body and node; an acquisition holds the whole
    /// permits covering its run and node, or the whole pool if that is
    /// less; and keeping a run's bytes returns everything beyond the whole
    /// permits covering that run and its node, and nothing below.
    #[test]
    fn permits_cover_exactly_their_runs(
        target in prop_oneof![0usize..1 << 16, Just(usize::MAX)],
        runs in 1usize..=WIRE_POOL_RUNS,
        run in 0usize..1 << 17,
        kept in 0usize..1 << 17,
    ) {
        let pool = WirePool::receiving(target, runs);
        let node = queued::<Inbound>();
        let whole = |bytes: usize| bytes.div_ceil(PERMIT_BYTES) * PERMIT_BYTES;
        let body = target.min(MAX_RUN_BUDGET_BYTES).saturating_sub(SUPPLY_FRAME_OVERHEAD);
        prop_assert_eq!(pool.bytes(), whole((body + node) * runs));
        prop_assert_eq!(pool.bytes(), pool_bytes(target, runs, node));

        let mut permit = pollster::block_on(pool.acquire(run));
        let held = whole(run + node).min(pool.bytes());
        prop_assert_eq!(permit.bytes(), held);
        prop_assert_eq!(pool.available(), pool.bytes() - held);

        pool.keep(&mut permit, kept);
        prop_assert_eq!(permit.bytes(), held.min(whole(kept + node)));
        drop(permit);
        prop_assert_eq!(pool.available(), pool.bytes());
    }
}

/// The two pools of a session stand for the sum of a send pool and a
/// receive pool at the session's target, each priced at its own node.
#[test]
fn a_session_sets_both_pools_aside() {
    for target in [0, 1, PERMIT_BYTES, 1 << 20, usize::MAX] {
        assert_eq!(
            wire_pools_bytes(target),
            WirePool::sending(target, WIRE_POOL_RUNS).bytes()
                + WirePool::receiving(target, WIRE_POOL_RUNS).bytes(),
        );
        assert_eq!(
            WirePool::sending(target, 1).bytes(),
            pool_bytes(target, 1, queued::<Outbound>())
        );
    }
}

/// A run larger than the pool waits for the whole pool, and a smaller run
/// that arrives after it waits behind it, even while enough of the pool is
/// free for the smaller one: waiting runs never divide the pool.
#[test]
fn acquisitions_are_served_whole_in_order() {
    let pool = WirePool::sending(SUPPLY_FRAME_OVERHEAD + 4096, 1);
    let held = pollster::block_on(pool.acquire(1024));
    assert!(
        pool.available() > held.bytes(),
        "the pool has room for a small run beside the held one"
    );
    let mut whole = pin!(pool.acquire(usize::MAX));
    let mut small = pin!(pool.acquire(0));
    assert!(poll_once(whole.as_mut()).is_pending(), "the pool is held");
    assert!(
        poll_once(small.as_mut()).is_pending(),
        "the small run waits behind the whole one"
    );
    drop(held);
    let Poll::Ready(whole) = poll_once(whole.as_mut()) else {
        panic!("the whole pool is free once the held run returns");
    };
    assert_eq!(whole.bytes(), pool.bytes());
    assert!(
        poll_once(small.as_mut()).is_pending(),
        "the whole pool is taken"
    );
    drop(whole);
    let Poll::Ready(small): Poll<WirePermit> = poll_once(small.as_mut()) else {
        panic!("the small run is served once the whole pool returns");
    };
    assert!(small.bytes() > 0);
}
