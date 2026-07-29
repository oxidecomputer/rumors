//! Guardrail that the public futures stay type-erased.
//!
//! The mirror protocol's `Levels<Below<…, Below<…, …>>>` chain is ~30 deep,
//! enough that any layout query that traverses it inline blows past the
//! default `recursion_limit = 128` and forces downstream crates to bump
//! their own limit. We defuse that by type-erasing inside the protocol and
//! `tree::traverse::act`, which leaves the public futures (`Rumors::gossip`,
//! `Peer::retire`, `Bootstrap::join`) holding nothing more than a
//! `Pin<Box<dyn Future>>` plus a few locals.
//!
//! If the deep chain is reintroduced inline (say, the `Box::pin` indirection
//! removed, or a new public future driving the protocol directly), the
//! future size jumps from a couple hundred bytes to tens of KiB and trips
//! the budget — before downstream crates discover the `recursion_limit`
//! regression.
//!
//! The budget is enforced only in release builds: debug layouts carry
//! additional state, and they are not what users ship.

#![cfg(not(debug_assertions))]

use std::mem::size_of_val;

use rumors::{Peer, Rumors};

/// Upper bound for the unawaited public futures.
///
/// The budget is set
/// generously above the measured sizes (a few hundred bytes) so legitimate
/// growth — an extra captured local, a slightly fatter error type —
/// doesn't fail the test, but any *order-of-magnitude* growth (i.e. the
/// inner protocol state machine leaking out inline) will.
const PUBLIC_FUTURE_BUDGET: usize = 1024;

/// `Rumors::gossip` drives the full mirror protocol against a peer; the
/// public future is type-erased.
///
/// The erasure is `mirror()`'s internal `Pin<Box<dyn
/// Future>>`, so the protocol's `Levels` chain doesn't appear in the
/// caller's layout query.
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
         if a recent change removed the internal `Pin<Box<dyn Future>>` \
         indirection, restore it — otherwise downstream crates will hit \
         `recursion_limit` overflow",
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
         see gossip_future_fits_budget for rationale",
    );
}

/// `Bootstrap::join` runs the same mirror descent from an empty tree.
/// Same erasure boundary as `gossip`.
#[test]
fn bootstrap_future_fits_budget() {
    let (mut link, peer) = rumors::link::memory();
    drop(peer);

    let fut = Peer::<()>::bootstrap().join(&mut link);
    let size = size_of_val(&fut);

    assert!(
        size <= PUBLIC_FUTURE_BUDGET,
        "bootstrap future is {size} bytes, exceeds budget {PUBLIC_FUTURE_BUDGET}; \
         see gossip_future_fits_budget for rationale",
    );
}

/// `Rumors::send` runs the commit protocol's build against the storage
/// backend; its layout must stay handle-sized.
///
/// `send` carries `Batch::commit`'s state machine whole, so this also
/// guards the commit path: the local traversal seams must not fold a
/// protocol tower or a backend's build state into the public future.
#[test]
fn send_future_fits_budget() {
    let alice: Rumors<u64> = Peer::seed().into_rumors();
    let fut = alice.send(7);
    let size = size_of_val(&fut);

    assert!(
        size <= PUBLIC_FUTURE_BUDGET,
        "send future is {size} bytes, exceeds budget {PUBLIC_FUTURE_BUDGET}; \
         see gossip_future_fits_budget for rationale",
    );
}

/// `Batch::commit` on a multi-action batch is the same commit protocol as
/// `send`, entered through the explicit builder.
#[test]
fn batch_commit_future_fits_budget() {
    let alice: Rumors<u64> = Peer::seed().into_rumors();
    let batch = alice.batch().send(1).send(2);
    let fut = batch.commit();
    let size = size_of_val(&fut);

    assert!(
        size <= PUBLIC_FUTURE_BUDGET,
        "batch commit future is {size} bytes, exceeds budget {PUBLIC_FUTURE_BUDGET}; \
         see gossip_future_fits_budget for rationale",
    );
}

/// `Snapshot::get` descends through the backend's point-lookup seam; the
/// public read future must stay a handle-sized descent, not a tower.
#[test]
fn snapshot_get_future_fits_budget() {
    let alice: Rumors<u64> = Peer::seed().into_rumors();
    let snapshot = alice.snapshot();
    let key = rumors::Key::from([0u8; 32]);
    let fut = snapshot.get(&key);
    let size = size_of_val(&fut);

    assert!(
        size <= PUBLIC_FUTURE_BUDGET,
        "snapshot get future is {size} bytes, exceeds budget {PUBLIC_FUTURE_BUDGET}; \
         see gossip_future_fits_budget for rationale",
    );
}

/// The KV entries' budget: the same order of magnitude, headroom for the
/// honest handle delta.
///
/// A persistent node handle is four pointers wide (record `Arc`, span
/// offset, resident hash) against the in-memory backend's one, and a
/// session future legitimately carries several tree values; the failure
/// mode this file exists to catch — a protocol tower inlined into the
/// public layout — lands in the tens of KiB either way.
const KV_PUBLIC_FUTURE_BUDGET: usize = 2 * 1024;

/// The persistent backend's public futures stay flat.
///
/// The generic towers behind `Store`'s seams are `BoxFuture`-per-level,
/// so a `send`, a batch `commit`, a snapshot `get`, and a `gossip` on a
/// KV-backed set must not inline the 32-level descent into the caller's
/// layout.
#[test]
fn kv_backed_futures_fit_budget() {
    use rumors::{KvBackend, Memory, NoBookmark};
    let alice: Rumors<(), NoBookmark, KvBackend<Memory, ()>> =
        Peer::seed_in(KvBackend::new(Memory::default()))
            .sync_window_floor()
            .into_rumors();

    let send = alice.send(());
    assert!(
        size_of_val(&send) <= KV_PUBLIC_FUTURE_BUDGET,
        "KV send future is {} bytes, exceeds budget {KV_PUBLIC_FUTURE_BUDGET}",
        size_of_val(&send),
    );
    drop(send);

    let batch = alice.batch().send(()).commit();
    assert!(
        size_of_val(&batch) <= KV_PUBLIC_FUTURE_BUDGET,
        "KV batch-commit future is {} bytes, exceeds budget {KV_PUBLIC_FUTURE_BUDGET}",
        size_of_val(&batch),
    );
    drop(batch);

    let snapshot = alice.snapshot();
    let key = rumors::Key::from([0u8; 32]);
    let get = snapshot.get(&key);
    assert!(
        size_of_val(&get) <= KV_PUBLIC_FUTURE_BUDGET,
        "KV snapshot-get future is {} bytes, exceeds budget {KV_PUBLIC_FUTURE_BUDGET}",
        size_of_val(&get),
    );
    drop(get);

    let (mut link, other) = rumors::link::memory();
    drop(other);
    let gossip = alice.gossip(&mut link);
    assert!(
        size_of_val(&gossip) <= KV_PUBLIC_FUTURE_BUDGET,
        "KV gossip future is {} bytes, exceeds budget {KV_PUBLIC_FUTURE_BUDGET}",
        size_of_val(&gossip),
    );
}
