//! Attaching a [`Bookmark`](rumors::Bookmark) after construction, via
//! [`Peer::bookmark`](rumors::Peer::bookmark).
//!
//! The property suite in `bookmark_causality.rs` exercises the eager-persist
//! and fault paths in aggregate; these are point assertions on the two corners
//! of the attach contract that suite never names directly: that a pristine seed
//! is persisted lazily (no write at attach time), and that a failed persist
//! hands the peer back intact for a retry rather than stranding its identity.

mod common;

use std::sync::{Arc, Mutex};

use rumors::{Peer, Rumors, Unbookmarked};

use crate::common::flaky::{FaultFeed, FlakyInMemoryBookmark, persisted_record};
use crate::common::wire::tokio_block_on as block_on;

/// Capacity for the in-memory duplex carrying a bootstrap session.
const DUPLEX_BUF: usize = 64 * 1024;

/// Bootstrap a fresh, still-unbookmarked peer from `server` over a clean
/// in-process duplex. Both sides run as spawned tasks so a finished one drops
/// its halves; the wires are reliable, so the bootstrap succeeds.
async fn bootstrap_unbookmarked(server: &Rumors<String, FlakyInMemoryBookmark>) -> Peer<String> {
    let server = server.clone();
    let (boot_side, serve_side) = tokio::io::duplex(DUPLEX_BUF);
    let (mut boot_r, mut boot_w) = tokio::io::split(boot_side);
    let (mut serve_r, mut serve_w) = tokio::io::split(serve_side);
    let boot =
        tokio::spawn(async move { Peer::<String>::bootstrap(&mut boot_r, &mut boot_w).await });
    let serve = tokio::spawn(async move { server.gossip(&mut serve_r, &mut serve_w).await });
    let (boot_out, serve_out) = tokio::join!(boot, serve);
    serve_out.unwrap().expect("serve the bootstrap");
    boot_out
        .unwrap()
        .expect("bootstrap ok")
        .expect("got a peer")
}

/// Bookmarking a pristine seed touches no storage: a content-free, never-forked
/// seed has no identity worth recording, so the first write is deferred to the
/// first gossip. The fault schedule would *fail* a write, so a clean `Ok` is
/// itself proof that none was attempted.
#[test]
fn pristine_seed_attaches_without_touching_storage() {
    block_on(async {
        let store = Arc::new(Mutex::new(None));
        let faults = Arc::new(Mutex::new(FaultFeed::new(vec![], vec![true])));
        let bookmark = FlakyInMemoryBookmark::new(store.clone(), faults, 0);

        let _peer = Peer::<String>::seed()
            .bookmark(bookmark)
            .await
            .expect("a pristine seed attaches without attempting a write");

        assert!(
            store.lock().unwrap().is_none(),
            "a pristine seed must persist nothing at attach time",
        );
    });
}

/// A failed persist hands the peer back, intact and unbookmarked: the identity
/// is not lost, the store is left untouched, and re-attaching over healthy
/// storage then succeeds and records it. The peer must already *know* something
/// — here, one sent message advancing its frontier — or the pristine-seed
/// shortcut would skip the write the failure rides on.
#[test]
fn failed_persist_returns_peer_for_retry() {
    block_on(async {
        let rumors = Peer::<String>::seed().into_rumors();
        rumors.send("the meeting is at noon".to_string());
        let peer = rumors.try_into_peer().await.expect("sole handle");
        let network = peer.network();

        // Attach over a store whose first write is scheduled to fail.
        let store = Arc::new(Mutex::new(None));
        let failing = FlakyInMemoryBookmark::new(
            store.clone(),
            Arc::new(Mutex::new(FaultFeed::new(vec![], vec![true]))),
            0,
        );
        let Unbookmarked { peer, error } = peer
            .bookmark(failing)
            .await
            .expect_err("the injected write failure must surface");
        assert_eq!(
            error.to_string(),
            "flaky bookmark: injected write failure",
            "the surfaced error must be the bookmark's own",
        );
        assert!(
            store.lock().unwrap().is_none(),
            "a failed write must leave storage untouched",
        );

        // The handed-back peer retries cleanly over healthy storage.
        let healthy = FlakyInMemoryBookmark::new(
            store.clone(),
            Arc::new(Mutex::new(FaultFeed::new(vec![], vec![]))),
            0,
        );
        let _peer = peer
            .bookmark(healthy)
            .await
            .expect("the retry over healthy storage persists");
        assert!(
            persisted_record(&store).contains_key(&network),
            "the retry must record this peer's identity",
        );
    });
}

/// A failed attach must never leave a *reclaimed* region live in the handed-back
/// peer while it stays stranded on disk. This is the recycle hazard the gossip
/// persist gate cannot catch, precisely because the handed-back peer is
/// unbookmarked (its `bookmark_update` is the infallible no-op of `NoBookmark`,
/// so nothing stops it from gossiping the region).
///
/// We force the exact shape: a peer bootstraps a fresh fork, then attaches a
/// store that still holds its *previous* incarnation's disjoint region — whose
/// recorded version its frontier dominates — over a failing write. The attach
/// must reclaim nothing, so the returned party stays disjoint from the stranded
/// region. Were reclaim done at attach, the failed write would strand on disk a
/// region now live in the returned peer: the disagreement that recycles a
/// version once the store is re-attached to a later peer.
#[test]
fn failed_attach_does_not_reclaim_into_an_unbookmarked_peer() {
    let reliable = || Arc::new(Mutex::new(FaultFeed::new(vec![], vec![])));
    block_on(async {
        // A seeds network N over a reliable store and gossips it onward.
        let store_a = Arc::new(Mutex::new(None));
        let a = Peer::<String>::seed()
            .bookmark(FlakyInMemoryBookmark::new(store_a, reliable(), 0))
            .await
            .expect("the seed attaches")
            .into_rumors();

        // B bootstraps a fork from A and records it durably, then "crashes":
        // its region survives only as a stranded entry in B's store.
        let store_b = Arc::new(Mutex::new(None));
        let b = bootstrap_unbookmarked(&a)
            .await
            .bookmark(FlakyInMemoryBookmark::new(store_b.clone(), reliable(), 1))
            .await
            .expect("B records its fork");
        let stranded = b.dangerously_alias_party().expect("B is live");
        drop(b);

        // Advance the network so a recovering peer's frontier strictly
        // dominates the stranded region's recorded version (the precondition
        // for `reclaim` to fold it in).
        a.send("tick".to_string());

        // B' recovers: a *fresh* fork from A, disjoint from the stranded region.
        let b_prime = bootstrap_unbookmarked(&a).await;
        let fresh = b_prime.dangerously_alias_party().expect("B' is live");
        assert!(
            fresh.is_disjoint(&stranded),
            "the fresh fork must be disjoint from the stranded region",
        );

        // Attach B's old store — which still holds the stranded region — over a
        // write scheduled to fail.
        let failing = FlakyInMemoryBookmark::new(
            store_b.clone(),
            Arc::new(Mutex::new(FaultFeed::new(vec![], vec![true]))),
            1,
        );
        let Unbookmarked { peer, error: _ } = b_prime
            .bookmark(failing)
            .await
            .expect_err("the injected write must fail the attach");

        // The handed-back peer must not have reclaimed the stranded region: its
        // party stays exactly the fresh fork, disjoint from what is still
        // recorded on disk. (Reclaim-at-attach would make this region live here
        // yet claimable by the next peer to read the store — a recycle.)
        let after = peer.dangerously_alias_party().expect("the peer is live");
        assert!(
            after.is_disjoint(&stranded),
            "a failed attach must not reclaim the stranded region into the peer",
        );
        assert_eq!(
            after, fresh,
            "a failed attach must leave the live party untouched",
        );
        assert!(
            persisted_record(&store_b)
                .get(&peer.network())
                .is_some_and(|clocks| clocks
                    .iter()
                    .any(|clock| !clock.party().is_disjoint(&stranded))),
            "the stranded region must remain recorded on disk, unreclaimed",
        );
    });
}
