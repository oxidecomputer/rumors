//! Catch accidental growth of public futures into the large protocol state.
//!
//! Boxed reconciliation boundaries keep the deeply nested protocol futures out
//! of callers' layouts. These checks allow modest growth in session state but
//! catch a missing boundary before callers encounter excessive stack use or
//! compiler recursion limits. The gate checks the development-profile layout.

use std::mem::size_of_val;

use rumors::{Peer, Rumors};

/// Allow small session futures while catching large embedded protocol state.
const PUBLIC_FUTURE_BUDGET: usize = 4096;

/// `Rumors::gossip_once` drives the full mirror protocol against a peer; the
/// public future is type-erased.
///
/// The erasure is `Reconciliation::reconcile`'s boxed future, so the typed
/// phase schedule never appears in the caller's layout query.
#[test]
fn gossip_future_fits_budget() {
    let (mut link, peer) = rumors::link::memory();
    drop(peer);

    let alice: Rumors<()> = Peer::seed().sync_window_floor().into_rumors();
    let fut = alice.gossip_once(&mut link);
    let size = size_of_val(&fut);

    assert!(
        size <= PUBLIC_FUTURE_BUDGET,
        "gossip future is {size} bytes, exceeds budget {PUBLIC_FUTURE_BUDGET}; \
         if a recent change removed the `Pin<Box<dyn Future>>` returned by \
         `Reconciliation::reconcile`, restore it; otherwise downstream crates \
         will hit `recursion_limit` overflow",
    );
}

/// `Peer::retire` adds handoff to a gossip session: the same erasure
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

/// `Rumors::gossip` keeps the continuous driver behind a boxed stream.
///
/// What this test would first observe is a stream handed out unboxed with
/// deep state behind it; the stream cannot see the `Reconciliation::reconcile`
/// boundary, which the gossip and retire tests beside it hold.
#[test]
fn gossip_stream_fits_budget() {
    let (mut link, peer) = rumors::link::memory();
    drop(peer);

    let alice: Rumors<()> = Peer::seed().sync_window_floor().into_rumors();
    let sessions = alice.gossip(&mut link);
    let size = size_of_val(&sessions);

    assert!(
        size <= PUBLIC_FUTURE_BUDGET,
        "gossip stream is {size} bytes, exceeds budget {PUBLIC_FUTURE_BUDGET}; \
         the stream is handed out unboxed with deep state behind it: restore \
         the `Box::pin` around `gossip`'s unfold",
    );
}
