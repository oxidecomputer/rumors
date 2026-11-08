//! A leaked session future cannot wedge the peer reclaimed from its handles,
//! nor keep the replica alive for its observers.

use std::future::{Future, pending, poll_fn};
use std::io::Cursor;
use std::mem;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

use futures::StreamExt;
use futures::future::join_all;
use proptest::collection::vec;
use proptest::prelude::*;
use rumors::testing::{MEMORY_CAPACITY, memory_connection, run_to_quiescence};
use rumors::{Bookmark, Connection, Joined, Peer, Rumors};
use rumors_testkit::common::property::SESSION_DIVISOR;
use rumors_testkit::common::seed::network_seed;
use rumors_testkit::common::sim::assert_parties_partition;
use rumors_testkit::divided_proptest;

/// In-memory storage one of whose stores, once armed, waits forever before it
/// stores anything.
#[derive(Clone, Debug, Default)]
struct StallingBookmark {
    /// The complete record last stored.
    record: Arc<Mutex<Option<Vec<u8>>>>,
    /// Which store from now waits forever, counting the next as 1, or 0 when
    /// none will.
    countdown: Arc<AtomicUsize>,
    /// Whether an armed store has begun waiting.
    stalled: Arc<AtomicBool>,
}

/// Arm the stall and observe it.
impl StallingBookmark {
    /// Make the next store wait forever.
    fn arm(&self) {
        self.arm_at(1);
    }

    /// Make the `nth` store from now wait forever, counting the next as 1.
    fn arm_at(&self, nth: usize) {
        self.stalled.store(false, Ordering::SeqCst);
        self.countdown.store(nth, Ordering::SeqCst);
    }

    /// Let every store complete, whether or not an armed one was reached.
    fn disarm(&self) {
        self.countdown.store(0, Ordering::SeqCst);
    }

    /// Whether a store is waiting forever.
    fn stalled(&self) -> bool {
        self.stalled.load(Ordering::SeqCst)
    }
}

/// Keep the record in memory, stalling the one store
/// [`arm_at`](StallingBookmark::arm_at) names.
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
        let counted = self
            .countdown
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_sub(1));
        if counted == Ok(1) {
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
    let Joined::Joined {
        peer,
        connection: Connection::Reusable { .. },
    } = joined
    else {
        panic!("a reliable bootstrap joins")
    };
    peer.into_rumors()
}

/// Reclaim the peer from `live`'s generation while it has `handles` handles and
/// `reuniters` reuniters compete, checking that exactly one of them reclaims
/// it.
async fn reclaim<B: Bookmark>(
    live: Rumors<u64, B>,
    handles: usize,
    reuniters: usize,
) -> Peer<u64, B> {
    let others: Vec<_> = (1..handles).map(|_| live.clone()).collect();
    let waiting: Vec<_> = (0..reuniters)
        .map(|_| live.clone().try_into_peer())
        .collect();
    drop(others);
    drop(live);
    let mut claimed = join_all(waiting).await.into_iter().flatten();
    let peer = claimed.next().expect("one reuniter reclaims the peer");
    assert!(claimed.next().is_none(), "the peer cannot be claimed twice");
    peer
}

/// The most rounds a leaked bootstrap service is polled when a case leaks it
/// wherever the rounds run out, rather than at its stalled store.
///
/// For the replicas the test builds, the checkpoint's store begins in the
/// second round and the donation's in about the twentieth, so draws below this
/// limit leak the service before its fork is reserved, at its checkpoint's
/// store, and throughout reconciliation.
const MAX_LEAK_POLLS: usize = 24;

/// One session serving a joining peer, leaked, and the reunion after it.
#[derive(Clone, Copy, Debug)]
struct Leak {
    /// Which of the session's stores waits forever: its checkpoint's (1) or its
    /// donation's (2). The donated fork is never sent either way.
    stalled_store: usize,
    /// How many rounds to poll the session and its joiner before leaking it, or
    /// `None` to poll them until the stalled store begins waiting.
    polls: Option<usize>,
    /// The generation's handles when the peer is reclaimed.
    handles: usize,
    /// The reuniters competing to reclaim it.
    reuniters: usize,
}

/// Leaks at every point of a joiner's service, from before its fork is reserved
/// through its donation's store.
fn leak() -> impl Strategy<Value = Leak> {
    (
        1usize..=2,
        prop::option::of(0..MAX_LEAK_POLLS),
        1usize..4,
        1usize..4,
    )
        .prop_map(|(stalled_store, polls, handles, reuniters)| Leak {
            stalled_store,
            polls,
            handles,
            reuniters,
        })
}

divided_proptest! {
    #![divisor(SESSION_DIVISOR)]

    /// A session leaked anywhere in serving a joining peer pauses identity
    /// reclamation only until the peer is reclaimed.
    ///
    /// The next session reclaims the fork the leaked session reserved, so
    /// the live parties again partition the whole identity space, none held
    /// twice. The leaked fork was never sent, and its region is still in the
    /// bookmark's record, so reclaiming it is what makes the partition
    /// whole. Each leak is followed by a reunion, and a joiner served at the
    /// end receives a fork of the reclaimed space without overlap.
    #[test]
    fn reclaiming_resumes_reclamation_after_a_leaked_bootstrap(
        leaks in vec(leak(), 1..4),
        network in network_seed(),
    ) {
        run_to_quiescence(async {
            let storage = StallingBookmark::default();
            let mut live = network
                .peer::<u64>()
                .bookmark(storage.clone())
                .await
                .expect("in-memory storage attaches")
                .into_rumors();
            live.send(0).unwrap();
            let remote = join_from(&live).await;

            for (round, leak) in (1u64..).zip(leaks) {
                // A fresh write makes the leaked session's checkpoint store,
                // so that its stores count from the checkpoint's.
                live.send(round).unwrap();
                storage.arm_at(leak.stalled_store);
                let (mut near, mut far) = memory_connection(MEMORY_CAPACITY);
                let mut leaked = Box::pin(live.gossip_once(&mut near.read, &mut near.write));
                let mut joiner = Box::pin(Peer::<u64>::bootstrap().join(&mut far.read, &mut far.write));
                let mut cx = Context::from_waker(Waker::noop());
                let mut joined = false;
                let mut polls = 0;
                while !storage.stalled() && leak.polls.is_none_or(|limit| polls < limit) {
                    assert!(
                        leaked.as_mut().poll(&mut cx).is_pending(),
                        "serving a joiner cannot finish past a stalled store"
                    );
                    if !joined {
                        joined = joiner.as_mut().poll(&mut cx).is_ready();
                    }
                    polls += 1;
                }
                assert!(!joined, "the joiner cannot finish before receiving its fork");
                mem::forget(leaked);
                drop(joiner);
                storage.disarm();

                live = reclaim(live, leak.handles, leak.reuniters).await.into_rumors();
                live.send(round + 1_000).unwrap();
                let (mut near, mut far) = memory_connection(MEMORY_CAPACITY);
                let (local, partner) = futures::join!(
                    live.gossip_once(&mut near.read, &mut near.write),
                    remote.gossip_once(&mut far.read, &mut far.write),
                );
                local.expect("the reclaimed peer's session completes");
                partner.expect("its partner's session completes");
                assert_parties_partition(
                    vec![live.dangerously_alias_party(), remote.dangerously_alias_party()],
                    0,
                );
            }

            let joined = join_from(&live).await;
            assert_parties_partition(
                vec![
                    live.dangerously_alias_party(),
                    remote.dangerously_alias_party(),
                    joined.dangerously_alias_party(),
                ],
                0,
            );
        })
        .expect("sessions on the reclaimed peer make progress");
    }
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
            let peer = reclaim(live, handles, reuniters).await;

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

    /// Once every handle is gone, the replica's observers drain and end,
    /// even though a session future leaked while waiting on bookmark storage.
    ///
    /// The leaked future can never run again, so nothing it holds may keep
    /// the replica alive for its observers.
    #[test]
    fn observers_end_after_a_leaked_session(network in network_seed()) {
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
            let mut observer = live.unordered_messages();

            // Run a session until the live side waits on its checkpoint's
            // store, then leak that side's future.
            storage.arm();
            live.send(1).unwrap();
            let (mut near, mut far) = memory_connection(MEMORY_CAPACITY);
            let mut leaked = Box::pin(live.gossip_once(&mut near.read, &mut near.write));
            let mut partner = Box::pin(remote.gossip_once(&mut far.read, &mut far.write));
            poll_fn(|cx| {
                assert!(leaked.as_mut().poll(cx).is_pending(), "a stalled store never completes");
                let _ = partner.as_mut().poll(cx);
                if storage.stalled() { Poll::Ready(()) } else { Poll::Pending }
            })
            .await;
            mem::forget(leaked);
            drop(partner);

            drop(live);
            let mut drained = 0;
            while observer.next().await.is_some() {
                drained += 1;
            }
            assert_eq!(drained, 2, "the observer drains both messages before it ends");
        })
        .expect("the observer ends once every handle is gone");
    }
}
