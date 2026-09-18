//! Bookmark attachment preserves ownership and reports storage failures.
//!
//! Pristine seeds defer storage. Other peers read and checkpoint at attachment;
//! a failure returns the unbookmarked peer for retry without reclaiming rights.
//! These tests distinguish storage errors from invalid records and check that
//! failed reads leave the stored bytes and the peer's state intact.

use rumors_testkit::common;

use std::sync::{Arc, Mutex};

use proptest::prelude::*;
use rumors::{BookmarkIo, Peer, Rumors, Unbookmarked};

use crate::common::flaky::{FaultFeed, FlakyInMemoryBookmark, persisted_record};
use crate::common::wire::{LINK_BUF, block_on};

/// Join the server's network without attaching storage to the new peer.
async fn bootstrap_unbookmarked(server: &Rumors<String, FlakyInMemoryBookmark>) -> Peer<String> {
    let server = server.clone();
    let (boot_link, serve_link) = rumors::link::memory_with_capacity(LINK_BUF);
    let (boot_out, serve_out) = tokio::join!(
        async move {
            let mut link = boot_link;
            Peer::<String>::bootstrap().join(&mut link).await
        },
        async move {
            let mut link = serve_link;
            server.gossip_once(&mut link).await
        },
    );
    serve_out.expect("serve the bootstrap");
    (match boot_out {
        rumors::Joined::Joined { peer } => peer,
        _ => panic!("got a peer"),
    })
    .sync_window_floor()
}

/// A pristine seed defers attachment I/O, even when both operations would fail.
#[test]
fn pristine_seed_attaches_without_touching_storage() {
    block_on(async {
        let store = Arc::new(Mutex::new(None));
        let faults = Arc::new(Mutex::new(FaultFeed::new(vec![true], vec![true])));
        let bookmark = FlakyInMemoryBookmark::new(store.clone(), faults, 0);

        let _peer = Peer::<String>::seed()
            .sync_window_floor()
            .bookmark(bookmark)
            .await
            .expect("a pristine seed attaches without reading or writing");

        assert!(
            store.lock().unwrap().is_none(),
            "a pristine seed must persist nothing at attach time",
        );
    });
}

/// Failure before replacement returns the peer for retry and leaves storage absent.
#[test]
fn failed_persist_returns_peer_for_retry() {
    block_on(async {
        let rumors = Peer::<String>::seed().sync_window_floor().into_rumors();
        rumors.send("the meeting is at noon".to_string()).unwrap();
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
        assert!(matches!(error, BookmarkIo::Io(_)));
        assert_eq!(
            error.to_string(),
            "flaky bookmark: injected write failure",
            "the surfaced error must be the bookmark's own",
        );
        assert!(
            store.lock().unwrap().is_none(),
            "this failure occurs before replacement",
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

/// A load failure returns the original peer and storage error, without writing.
#[test]
fn failed_load_returns_peer_for_retry() {
    block_on(async {
        let rumors = Peer::<String>::seed().sync_window_floor().into_rumors();
        rumors.send("keep this message".into()).unwrap();
        let hash = rumors.snapshot().hash();
        let peer = rumors.try_into_peer().await.unwrap();
        let party = peer.dangerously_alias_party();
        let network = peer.network();
        let store = Arc::new(Mutex::new(None));
        let failing = FlakyInMemoryBookmark::new(
            store.clone(),
            Arc::new(Mutex::new(FaultFeed::new(vec![true], vec![]))),
            0,
        );
        let Unbookmarked { peer, error } = peer.bookmark(failing).await.unwrap_err();
        assert!(matches!(error, BookmarkIo::Io(_)));
        assert_eq!(error.to_string(), "flaky bookmark: injected read failure");
        assert_eq!(peer.dangerously_alias_party(), party);
        assert_eq!(peer.network(), network);
        assert!(store.lock().unwrap().is_none());

        let healthy = FlakyInMemoryBookmark::new(
            store.clone(),
            Arc::new(Mutex::new(FaultFeed::new(vec![], vec![]))),
            0,
        );
        let rumors = peer.bookmark(healthy).await.unwrap().into_rumors();
        assert_eq!(rumors.snapshot().hash(), hash);
        assert!(persisted_record(&store).contains_key(&network));
    });
}

proptest! {
    /// Malformed records report a format error without changing storage or the peer.
    #[test]
    fn corrupt_record_preserves_storage_and_peer(suffix: Vec<u8>) {
        block_on(async {
            let rumors = Peer::<String>::seed().sync_window_floor().into_rumors();
            rumors.send("keep this message".into()).unwrap();
            let hash = rumors.snapshot().hash();
            let peer = rumors.try_into_peer().await.unwrap();
            let party = peer.dangerously_alias_party();
            let network = peer.network();
            // A CBOR break cannot begin the bookmark envelope, regardless of
            // the suffix. No payload can accidentally turn this into a valid record.
            let mut bytes = vec![0xff];
            bytes.extend(suffix);
            let store = Arc::new(Mutex::new(Some(bytes.clone())));
            let bookmark = FlakyInMemoryBookmark::new(
                store.clone(),
                Arc::new(Mutex::new(FaultFeed::new(vec![], vec![]))),
                0,
            );
            let Unbookmarked { peer, error } = peer.bookmark(bookmark).await.unwrap_err();
            assert!(matches!(error, BookmarkIo::Format(_)));
            assert_eq!(peer.dangerously_alias_party(), party);
            assert_eq!(peer.network(), network);
            assert_eq!(*store.lock().unwrap(), Some(bytes));
            assert_eq!(peer.into_rumors().snapshot().hash(), hash);
        });
    }
}

/// Failed attachment must not acquire rights still recorded for another incarnation.
///
/// The returned peer has no bookmark, so subsequent gossip would have no storage
/// check to catch an unsafe acquisition. Its identity must remain exactly the
/// fresh fork it held before attachment.
#[test]
fn failed_attach_does_not_reclaim_into_an_unbookmarked_peer() {
    let reliable = || Arc::new(Mutex::new(FaultFeed::new(vec![], vec![])));
    block_on(async {
        // A seeds network N over a reliable store and gossips it onward.
        let store_a = Arc::new(Mutex::new(None));
        let a = Peer::<String>::seed()
            .sync_window_floor()
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
        let stranded = b.dangerously_alias_party();
        drop(b);

        // Advance the network so a recovering peer's frontier strictly
        // dominates the stranded region's recorded version (the precondition
        // for `reclaim` to fold it in).
        a.send("tick".to_string()).unwrap();

        // B' recovers: a *fresh* fork from A, disjoint from the stranded region.
        let b_prime = bootstrap_unbookmarked(&a).await;
        let fresh = b_prime.dangerously_alias_party();
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

        // These rights remain available to a future incarnation. Acquiring
        // them here would allow two peers to generate versions in the same region.
        let after = peer.dangerously_alias_party();
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
