//! A leaked session future cannot wedge the peer reclaimed from its handles.

use std::future::{Future, pending, poll_fn};
use std::io::Cursor;
use std::mem;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::task::Poll;

use futures::future::join_all;
use rumors::testing::{MEMORY_CAPACITY, memory_connection, run_to_quiescence};
use rumors::{Bookmark, Joined, Peer, Rumors};
use rumors_testkit::common::property::SESSION_DIVISOR;
use rumors_testkit::common::seed::network_seed;
use rumors_testkit::divided_proptest;

/// In-memory storage whose next store, once armed, waits forever.
#[derive(Clone, Debug, Default)]
struct StallingBookmark {
    /// The complete record last stored.
    record: Arc<Mutex<Option<Vec<u8>>>>,
    /// Whether the next store waits forever instead of storing.
    armed: Arc<AtomicBool>,
    /// Whether an armed store has begun waiting.
    stalled: Arc<AtomicBool>,
}

/// Arm the stall and observe it.
impl StallingBookmark {
    /// Make the next store wait forever.
    fn arm(&self) {
        self.armed.store(true, Ordering::SeqCst);
    }

    /// Whether a store is waiting forever.
    fn stalled(&self) -> bool {
        self.stalled.load(Ordering::SeqCst)
    }
}

/// Keep the record in memory, stalling the one store after [`arm`](StallingBookmark::arm).
impl Bookmark for StallingBookmark {
    /// This storage never fails.
    type Error = std::io::Error;
    /// A snapshot of the stored bytes.
    type Reader = Cursor<Vec<u8>>;

    /// Return the record last stored.
    async fn load(&self) -> Result<Option<Self::Reader>, Self::Error> {
        Ok(self.record.lock().unwrap().clone().map(Cursor::new))
    }

    /// Replace the record, unless this store was armed to wait forever.
    async fn store(&self, bytes: Vec<u8>) -> Result<(), Self::Error> {
        if self.armed.swap(false, Ordering::SeqCst) {
            self.stalled.store(true, Ordering::SeqCst);
            pending::<()>().await;
        }
        *self.record.lock().unwrap() = Some(bytes);
        Ok(())
    }
}

/// Join a fresh unbookmarked peer from `server` over a clean connection.
async fn join_from(server: &Rumors<u64, StallingBookmark>) -> Rumors<u64> {
    let (mut near, mut far) = memory_connection(MEMORY_CAPACITY);
    let (served, joined) = futures::join!(
        server.gossip_once(&mut near.read, &mut near.write),
        Peer::<u64>::bootstrap().join(&mut far.read, &mut far.write),
    );
    served.expect("serving a reliable bootstrap succeeds");
    let Joined::Joined { peer, .. } = joined else {
        panic!("a reliable bootstrap joins")
    };
    peer.into_rumors()
}

divided_proptest! {
    #![divisor(SESSION_DIVISOR)]

    /// The peer reclaimed after a session future leaked while waiting on
    /// bookmark storage configures its bookmark and completes later sessions.
    ///
    /// The leaked future keeps the bookmark locked. The claim holds whatever
    /// the number of handles and reuniters the generation had.
    #[test]
    fn reclaiming_releases_a_leaked_sessions_bookmark(
        handles in 1usize..4,
        reuniters in 1usize..4,
        network in network_seed(),
    ) {
        run_to_quiescence(async {
            let storage = StallingBookmark::default();
            let live = network
                .peer::<u64>()
                .bookmark(storage.clone())
                .await
                .expect("in-memory storage attaches")
                .into_rumors();
            live.send(0).unwrap();
            let remote = join_from(&live).await;

            // Run a session until the live side waits on its checkpoint's
            // store, then leak that side's future.
            storage.arm();
            live.send(1).unwrap();
            let (mut near, mut far) = memory_connection(MEMORY_CAPACITY);
            let mut leaked = Box::pin(live.gossip_once(&mut near.read, &mut near.write));
            let mut partner = Box::pin(remote.gossip_once(&mut far.read, &mut far.write));
            poll_fn(|cx| {
                assert!(leaked.as_mut().poll(cx).is_pending(), "a stalled store never completes");
                // The partner waits on the leaked side from here on, and is
                // dropped below.
                let _ = partner.as_mut().poll(cx);
                if storage.stalled() { Poll::Ready(()) } else { Poll::Pending }
            })
            .await;
            mem::forget(leaked);
            drop(partner);

            // Reclaim the peer while several handles exist and several
            // reuniters compete.
            let others: Vec<_> = (1..handles).map(|_| live.clone()).collect();
            let waiting: Vec<_> = (0..reuniters).map(|_| live.clone().try_into_peer()).collect();
            drop(others);
            drop(live);
            let mut claimed = join_all(waiting).await.into_iter().flatten();
            let peer = claimed.next().expect("one reuniter reclaims the peer");
            assert!(claimed.next().is_none(), "the peer cannot be claimed twice");

            // Configuring the bookmark neither waits nor panics, and a later
            // session takes the bookmark lock and completes.
            let live = peer
                .bookmark_size_limit(rumors::default::BOOKMARK_SIZE_LIMIT)
                .into_rumors();
            let (mut near, mut far) = memory_connection(MEMORY_CAPACITY);
            let (local, partner) = futures::join!(
                live.gossip_once(&mut near.read, &mut near.write),
                remote.gossip_once(&mut far.read, &mut far.write),
            );
            local.expect("the reclaimed peer's session completes");
            partner.expect("its partner's session completes");
            assert_eq!(remote.snapshot().len(), 2, "the later session delivers the leaked session's message");
        })
        .expect("sessions on the reclaimed peer make progress");
    }
}
