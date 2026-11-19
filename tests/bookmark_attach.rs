//! Bookmark attachment preserves ownership and reports storage failures.
//!
//! Pristine seeds defer storage. Other peers read and checkpoint at attachment;
//! a failure returns the unbookmarked peer for retry without reclaiming rights.
//! These tests distinguish storage errors from invalid records and check that
//! failed reads leave the stored bytes and the peer's state intact.

use rumors_testkit::common;

use std::sync::{Arc, Mutex};

use proptest::prelude::*;
use rumors::{Bookmark, MaybeBookmark, Peer, Unbookmarked, error::BookmarkIo};

use crate::common::flaky::{FaultFeed, FlakyInMemoryBookmark, RecordThenFail, persisted_record};
use crate::common::wire::block_on;
use rumors::testing::MEMORY_CAPACITY;
use rumors_testkit::common::property::SESSION_DIVISOR;
use rumors_testkit::common::seed::network_seed;
use rumors_testkit::divided_proptest;

/// Join the server's network with `bookmark` selected, returning the joined
/// peer, or the unbookmarked peer if attaching the bookmark failed after the
/// session.
async fn join_bookmarked<S: MaybeBookmark, B: Bookmark>(
    server: &Peer<String, S>,
    bookmark: B,
) -> Result<Peer<String, B>, Unbookmarked<String, B>> {
    let server = server.clone();
    let (joiner_end, serve_end) = rumors::testing::memory_connection(MEMORY_CAPACITY);
    let (join_out, serve_out) = tokio::join!(
        async move {
            let end = joiner_end;
            Peer::<String>::builder()
                .sync_memory_budget(0)
                .bookmark(bookmark)
                .join(end.read, end.write)
                .await
        },
        async move {
            let end = serve_end;
            server.gossip_once(end.read, end.write).await
        },
    );
    serve_out.expect("serve the join");
    match join_out {
        rumors::Join::Connected {
            peer,
            connection: rumors::Connection::Reusable { .. },
        } => Ok(peer),
        rumors::Join::Unbookmarked { unbookmarked, .. } => Err(unbookmarked),
        _ => panic!("got a peer"),
    }
}

divided_proptest! {
    #![divisor(SESSION_DIVISOR)]

    /// A pristine seed defers attachment I/O, even when both operations would fail.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn pristine_seed_attaches_without_touching_storage(network in network_seed()) {
        block_on(async {
            let store = Arc::new(Mutex::new(None));
            let faults = Arc::new(Mutex::new(FaultFeed::new(vec![true], vec![true])));
            let bookmark = FlakyInMemoryBookmark::new(store.clone(), faults, 0);

            let _peer = network.builder::<String>()
                .sync_memory_budget(0)
                .bookmark(bookmark).seed()
                .await
                .expect("a pristine seed attaches without reading or writing");

            assert!(
                store.lock().unwrap().is_none(),
                "a pristine seed must persist nothing at attach time",
            );
        });
    }

    /// Failure before replacement returns the peer for retry and leaves storage absent.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn failed_persist_returns_peer_for_retry(network in network_seed()) {
        block_on(async {
            let server = network.builder::<String>().sync_memory_budget(0).seed();
            server.send("the meeting is at noon".to_string()).unwrap();
            let network = server.network();

            // Join with a store whose first write is scheduled to fail.
            let store = Arc::new(Mutex::new(None));
            let failing = FlakyInMemoryBookmark::new(
                store.clone(),
                Arc::new(Mutex::new(FaultFeed::new(vec![], vec![true]))),
                0,
            );
            let unbookmarked = join_bookmarked(&server, failing)
                .await
                .expect_err("the injected write failure must surface");
            let error = &unbookmarked.error;
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
            let _peer = unbookmarked
                .retry_bookmark(healthy)
                .await
                .expect("the retry over healthy storage persists");
            assert!(
                persisted_record(&store).contains_key(&network),
                "the retry must record this peer's identity",
            );
        });
    }

    /// A load failure returns the original peer and storage error, without writing.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn failed_load_returns_peer_for_retry(network in network_seed()) {
        block_on(async {
            let server = network.builder::<String>().sync_memory_budget(0).seed();
            server.send("keep this message".into()).unwrap();
            let hash = server.snapshot().hash();
            let network = server.network();
            // The first attachment records the joined peer's identity and then
            // fails, so the record names the peer each failure hands back.
            let recorded = Arc::new(Mutex::new(None));
            let unbookmarked = join_bookmarked(&server, RecordThenFail::new(recorded.clone()))
                .await
                .expect_err("the recording store fails");
            let record = persisted_record(&recorded);
            let party = record[&network][0].party();

            let store = Arc::new(Mutex::new(None));
            let failing = FlakyInMemoryBookmark::new(
                store.clone(),
                Arc::new(Mutex::new(FaultFeed::new(vec![true], vec![]))),
                0,
            );
            let unbookmarked = unbookmarked.retry_bookmark(failing).await.unwrap_err();
            assert!(matches!(unbookmarked.error, BookmarkIo::Io(_)));
            assert_eq!(unbookmarked.error.to_string(), "flaky bookmark: injected read failure");
            assert!(store.lock().unwrap().is_none());

            let healthy = FlakyInMemoryBookmark::new(
                store.clone(),
                Arc::new(Mutex::new(FaultFeed::new(vec![], vec![]))),
                0,
            );
            let peer = unbookmarked.retry_bookmark(healthy).await.unwrap();
            assert_eq!(&peer.dangerously_alias_party(), party);
            assert_eq!(peer.network(), network);
            assert_eq!(peer.snapshot().hash(), hash);
            assert!(persisted_record(&store).contains_key(&network));
        });
    }
}

proptest! {
    /// Malformed records report a format error without changing storage or the peer.
    #[test]
    fn corrupt_record_preserves_storage_and_peer(
        suffix in any::<Vec<u8>>(),
        network in network_seed(),
    ) {
        block_on(async {
            let server = network.builder::<String>().sync_memory_budget(0).seed();
            server.send("keep this message".into()).unwrap();
            let hash = server.snapshot().hash();
            let network = server.network();
            // The first attachment records the joined peer's identity and then
            // fails, so the record names the peer the format failure hands back.
            let recorded = Arc::new(Mutex::new(None));
            let unbookmarked = join_bookmarked(&server, RecordThenFail::new(recorded.clone()))
                .await
                .expect_err("the recording store fails");
            let record = persisted_record(&recorded);
            let party = record[&network][0].party();
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
            let unbookmarked = unbookmarked.retry_bookmark(bookmark).await.unwrap_err();
            assert!(matches!(unbookmarked.error, BookmarkIo::Format(_)));
            let peer = unbookmarked.continue_unbookmarked();
            assert_eq!(&peer.dangerously_alias_party(), party);
            assert_eq!(peer.network(), network);
            assert_eq!(*store.lock().unwrap(), Some(bytes));
            assert_eq!(peer.snapshot().hash(), hash);
        });
    }
}

divided_proptest! {
    #![divisor(SESSION_DIVISOR)]

    /// Failed attachment must not acquire rights still recorded for another incarnation.
    ///
    /// The returned peer has no bookmark, so subsequent gossip would have no storage
    /// check to catch an unsafe acquisition. Its identity must remain exactly the
    /// fresh fork it held before attachment.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn failed_attach_does_not_reclaim_into_an_unbookmarked_peer(network in network_seed()) {
        let reliable = || Arc::new(Mutex::new(FaultFeed::new(vec![], vec![])));
        block_on(async {
            // A seeds network N over a reliable store and gossips it onward.
            let store_a = Arc::new(Mutex::new(None));
            let a = network.builder::<String>()
                .sync_memory_budget(0)
                .bookmark(FlakyInMemoryBookmark::new(store_a, reliable(), 0)).seed()
                .await
                .expect("the seed attaches");

            // B joins a fork from A and records it durably, then "crashes":
            // its region survives only as a stranded entry in B's store.
            let store_b = Arc::new(Mutex::new(None));
            let b = join_bookmarked(&a, FlakyInMemoryBookmark::new(store_b.clone(), reliable(), 1))
                .await
                .expect("B records its fork");
            let stranded = b.dangerously_alias_party();
            drop(b);

            // Advance the network so a recovering peer's frontier strictly
            // dominates the stranded region's recorded version (the precondition
            // for `reclaim` to fold it in).
            a.send("tick".to_string()).unwrap();

            // B' recovers: a *fresh* fork from A, disjoint from the stranded
            // region. Its first attachment records that fork and then fails, so
            // the record names it.
            let store_c = Arc::new(Mutex::new(None));
            let b_prime = join_bookmarked(&a, RecordThenFail::new(store_c.clone()))
                .await
                .expect_err("the recording store fails");
            let record_c = persisted_record(&store_c);
            let fresh = record_c[&a.network()][0].party();
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
            let peer = b_prime
                .retry_bookmark(failing)
                .await
                .expect_err("the injected write must fail the attach")
                .continue_unbookmarked();

            // These rights remain available to a future incarnation. Acquiring
            // them here would allow two peers to generate versions in the same region.
            let after = peer.dangerously_alias_party();
            assert!(
                after.is_disjoint(&stranded),
                "a failed attach must not reclaim the stranded region into the peer",
            );
            assert_eq!(
                &after, fresh,
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
}
