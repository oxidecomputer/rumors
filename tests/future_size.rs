//! Guardrail that the public futures stay type-erased.
//!
//! The streaming mirror's typed phase schedule (`streaming::protocol`) is a
//! deep generic type: a layout query that traverses it inline blows past
//! the default `recursion_limit = 128` and forces downstream crates to
//! bump their own limit. Two erasure boundaries keep it out of the public
//! futures. `Reconciliation::reconcile` (`src/peer/gossip.rs`) returns its
//! `#[inline(never)]` body as a `Pin<Box<dyn Future>>`, with
//! `Handshaken::reconcile`'s boxed descent below it, and every public
//! session future (`Rumors::gossip`, `Peer::retire`, `Bootstrap::join`)
//! awaits through it; `gossip_when` boxes its unfold, so its stream carries
//! the driver's in-flight session the same way. Each public future therefore
//! holds one pointer plus its own locals: a layout fact in either profile,
//! which is why the budget is pinned under the dev profile the gate runs.
//!
//! If a boundary is reintroduced inline (a `Box::pin` removed, or a new
//! public future driving the protocol directly), the budget trips here,
//! before downstream crates discover the `recursion_limit` regression.
//! Measured on this tree: unboxing `Reconciliation::reconcile` alone
//! roughly quadruples `gossip` and `retire` (the handshake state comes
//! inline; the descent stays behind `Handshaken::reconcile`'s box), and
//! unboxing every layer down to the descent puts the whole schedule inline
//! at about 95 KiB. `Bootstrap::join` has its own boxed boundary,
//! `bootstrap_reconcile`. The pin has a liveness leg too: the justfile's
//! `future-size` recipe reruns this binary with `--no-tests=fail`, so a
//! `cfg` that compiles it empty fails the gate instead of reading as a
//! pass.

use std::mem::size_of_val;

use futures::stream;
use rumors::{Peer, Rumors};

/// Upper bound for the unawaited public futures and the `gossip_when` stream.
///
/// Measured under both profiles, with identical results: the stream is 8
/// bytes (one boxed pointer), `Bootstrap::join` 336, `Rumors::gossip`
/// 960, and `Peer::retire` 2000. The budget sits half again above the
/// largest, so legitimate growth (an extra captured local, a slightly
/// fatter error type) does not fail the test, and below the roughly 4 KiB
/// that removing `Reconciliation::reconcile`'s box alone produces, so the
/// cheapest boundary regression still trips it.
const PUBLIC_FUTURE_BUDGET: usize = 3072;

/// `Rumors::gossip` drives the full mirror protocol against a peer; the
/// public future is type-erased.
///
/// The erasure is `Reconciliation::reconcile`'s boxed future, so the typed
/// phase schedule never appears in the caller's layout query.
#[test]
fn gossip_future_fits_budget() {
    let (mut link, peer) = rumors::link::memory();
    drop(peer);

    let alice: Rumors<()> = Peer::seed().sync_window_floor().into_rumors();
    let fut = alice.gossip(&mut link);
    let size = size_of_val(&fut);

    assert!(
        size <= PUBLIC_FUTURE_BUDGET,
        "gossip future is {size} bytes, exceeds budget {PUBLIC_FUTURE_BUDGET}; \
         if a recent change removed the `Pin<Box<dyn Future>>` returned by \
         `Reconciliation::reconcile`, restore it; otherwise downstream crates \
         will hit `recursion_limit` overflow",
    );
}

/// `Peer::retire` is `gossip` plus the party hand-off: the same erasure
/// boundary must keep it flat.
#[test]
fn retire_future_fits_budget() {
    let (mut link, peer) = rumors::link::memory();
    drop(peer);

    let alice: Peer<()> = Peer::seed().sync_window_floor();
    let fut = alice.retire(&mut link);
    let size = size_of_val(&fut);

    assert!(
        size <= PUBLIC_FUTURE_BUDGET,
        "retire future is {size} bytes, exceeds budget {PUBLIC_FUTURE_BUDGET}; \
         see gossip_future_fits_budget: the erasure is `Reconciliation::reconcile`",
    );
}

/// `Bootstrap::join` runs the same mirror descent from an empty tree; its
/// erasure is `bootstrap_reconcile`'s boxed future, the same discipline
/// as `Reconciliation::reconcile`.
#[test]
fn bootstrap_future_fits_budget() {
    let (mut link, peer) = rumors::link::memory();
    drop(peer);

    let fut = Peer::<()>::bootstrap().join(&mut link);
    let size = size_of_val(&fut);

    assert!(
        size <= PUBLIC_FUTURE_BUDGET,
        "bootstrap future is {size} bytes, exceeds budget {PUBLIC_FUTURE_BUDGET}; \
         if a recent change removed the `Pin<Box<dyn Future>>` returned by \
         `bootstrap_reconcile`, restore it; see gossip_future_fits_budget",
    );
}

/// `Rumors::gossip_when` hands back a stream, not a future, and its driver
/// keeps a whole session in flight between cues: the boxed unfold is what
/// keeps that session, and the schedule behind it, off the caller's layout.
#[test]
fn gossip_when_stream_fits_budget() {
    let (mut link, peer) = rumors::link::memory();
    drop(peer);

    let alice: Rumors<()> = Peer::seed().sync_window_floor().into_rumors();
    let sessions = alice.gossip_when(stream::empty::<()>(), &mut link);
    let size = size_of_val(&sessions);

    assert!(
        size <= PUBLIC_FUTURE_BUDGET,
        "gossip_when stream is {size} bytes, exceeds budget {PUBLIC_FUTURE_BUDGET}; \
         if a recent change removed the `Box::pin` around `gossip_when`'s \
         unfold, restore it; otherwise downstream crates will hit \
         `recursion_limit` overflow",
    );
}
