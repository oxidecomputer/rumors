//! Guardrail that the public futures stay type-erased.
//!
//! The mirror protocol's `Levels<Below<…, Below<…, …>>>` chain is ~30 deep,
//! enough that any layout query that traverses it inline blows past the
//! default `recursion_limit = 128` and forces downstream crates to bump
//! their own limit. We defuse that by type-erasing inside the protocol
//! (`tree::traverse::mirror::mirror`, `tree::traverse::act`), which leaves
//! the public futures (`Known::gossip`, `Known::join`, `Known::message`)
//! holding nothing more than a `Pin<Box<dyn Future>>` plus a few locals.
//!
//! This test pins down that arrangement: if someone reintroduces the deep
//! chain inline (e.g. by removing the `Box::pin` indirection or by adding
//! a new public future that drives the protocol directly), the future
//! size jumps from a couple hundred bytes to tens of KiB and trips the
//! budget — alerting us before downstream crates discover the
//! `recursion_limit` regression.
//!
//! The budget is enforced only in release builds: debug layouts carry
//! additional state, and they are not what users ship.

#![cfg(not(debug_assertions))]

use std::mem::size_of_val;

use rumors::Known;

/// Upper bound for the unawaited public futures. At time of writing, all
/// three measure ~170 bytes. The budget is set generously above that
/// (≈6×) so legitimate growth — an extra captured local, a slightly
/// fatter error type — doesn't fail the test, but any *order-of-
/// magnitude* growth (i.e. the inner protocol state machine leaking out
/// inline) will.
const PUBLIC_FUTURE_BUDGET: usize = 1024;

/// `Known::gossip` drives the full mirror protocol against a peer; the
/// public future is type-erased via `mirror()`'s internal `Pin<Box<dyn
/// Future>>` so the protocol's `Levels` chain doesn't appear in the
/// caller's layout query.
#[test]
fn gossip_future_fits_budget() {
    let (a, b) = tokio::io::duplex(64);
    let (mut a_r, mut a_w) = tokio::io::split(a);
    drop(b);

    let alice: Known<()> = Known::seed();
    let fut = alice.gossip(&mut a_r, &mut a_w);
    let size = size_of_val(&fut);

    assert!(
        size <= PUBLIC_FUTURE_BUDGET,
        "gossip future is {size} bytes, exceeds budget {PUBLIC_FUTURE_BUDGET}; \
         if a recent change removed the internal `Pin<Box<dyn Future>>` \
         indirection, restore it — otherwise downstream crates will hit \
         `recursion_limit` overflow",
    );
}

/// `Known::join` merges a snapshot in-process. Same erasure boundary as
/// `gossip`, via `tree::traverse::join`.
#[test]
fn join_future_fits_budget() {
    let mut alice: Known<()> = Known::seed();
    let helper = alice.rumors();
    let fut = alice.join(helper);
    let size = size_of_val(&fut);

    assert!(
        size <= PUBLIC_FUTURE_BUDGET,
        "join future is {size} bytes, exceeds budget {PUBLIC_FUTURE_BUDGET}; \
         see gossip_future_fits_budget for rationale",
    );
}

/// `Known::message` drives `Tree::act`, which goes through the recursive
/// `Act` trait's 32-level chain. Type-erased via `traverse::act`'s
/// internal `Pin<Box<dyn Future>>`.
#[test]
fn message_future_fits_budget() {
    let mut alice: Known<()> = Known::seed();
    let fut = alice.message([()]);
    let size = size_of_val(&fut);

    assert!(
        size <= PUBLIC_FUTURE_BUDGET,
        "message future is {size} bytes, exceeds budget {PUBLIC_FUTURE_BUDGET}; \
         see gossip_future_fits_budget for rationale",
    );
}
