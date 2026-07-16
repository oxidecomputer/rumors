//! Guardrail that the public futures stay type-erased.
//!
//! The mirror protocol's `Levels<Below<…, Below<…, …>>>` chain is ~30 deep,
//! enough that any layout query that traverses it inline blows past the
//! default `recursion_limit = 128` and forces downstream crates to bump
//! their own limit. We defuse that by type-erasing inside the protocol and
//! `tree::traverse::act`, which leaves
//! the public futures (`Rumors::gossip`, `Peer::retire`,
//! `Peer::bootstrap`) holding nothing more than a `Pin<Box<dyn Future>>`
//! plus a few locals.
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

use rumors::{Peer, Rumors};

/// Upper bound for the unawaited public futures. The budget is set
/// generously above the measured sizes (a few hundred bytes) so legitimate
/// growth — an extra captured local, a slightly fatter error type —
/// doesn't fail the test, but any *order-of-magnitude* growth (i.e. the
/// inner protocol state machine leaking out inline) will.
const PUBLIC_FUTURE_BUDGET: usize = 1024;

/// `Rumors::gossip` drives the full mirror protocol against a peer; the
/// public future is type-erased via `mirror()`'s internal `Pin<Box<dyn
/// Future>>` so the protocol's `Levels` chain doesn't appear in the
/// caller's layout query.
#[test]
fn gossip_future_fits_budget() {
    let (a, b) = tokio::io::duplex(64);
    let (mut a_r, mut a_w) = tokio::io::split(a);
    drop(b);

    let alice: Rumors<()> = Peer::seed().into_rumors();
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

/// `Peer::retire` is `gossip` plus the party hand-off: the same erasure
/// boundary must keep it flat.
#[test]
fn retire_future_fits_budget() {
    let (a, b) = tokio::io::duplex(64);
    let (mut a_r, mut a_w) = tokio::io::split(a);
    drop(b);

    let alice: Peer<()> = Peer::seed();
    let fut = alice.retire(&mut a_r, &mut a_w);
    let size = size_of_val(&fut);

    assert!(
        size <= PUBLIC_FUTURE_BUDGET,
        "retire future is {size} bytes, exceeds budget {PUBLIC_FUTURE_BUDGET}; \
         see gossip_future_fits_budget for rationale",
    );
}

/// `Peer::bootstrap` runs the same mirror descent from an empty tree.
/// Same erasure boundary as `gossip`.
#[test]
fn bootstrap_future_fits_budget() {
    let (a, b) = tokio::io::duplex(64);
    let (mut a_r, mut a_w) = tokio::io::split(a);
    drop(b);

    let fut = Peer::<()>::bootstrap(&mut a_r, &mut a_w);
    let size = size_of_val(&fut);

    assert!(
        size <= PUBLIC_FUTURE_BUDGET,
        "bootstrap future is {size} bytes, exceeds budget {PUBLIC_FUTURE_BUDGET}; \
         see gossip_future_fits_budget for rationale",
    );
}
