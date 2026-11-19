//! A leaked session future cannot wedge the peer a failed retirement returns,
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
use rumors::{Bookmark, Connection, Join, Peer, Retire};
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
async fn join_from(server: &Peer<u64, StallingBookmark>) -> Peer<u64> {
    let (mut near, mut far) = memory_connection(MEMORY_CAPACITY);
    let (served, joined) = futures::join!(
        server.gossip_once(&mut near.read, &mut near.write),
        Peer::<u64>::join(&mut far.read, &mut far.write),
    );
    served.expect("serving a reliable join succeeds");
    let Join::Connected {
        peer,
        connection: Connection::Reusable { .. },
    } = joined
    else {
        panic!("a reliable join succeeds")
    };
    peer
}

/// Retire every handle of `live`'s generation over a dead connection, with
/// `handles` ordinary handles dropped first and `retirers` handles retiring.
///
/// Each retirement must fail before the point of no return and return a
/// handle to the same identity.
async fn reclaim<B: rumors::MaybeBookmark>(
    live: Peer<u64, B>,
    handles: usize,
    retirers: usize,
) -> Peer<u64, B> {
    let identity = live.dangerously_alias_party();
    let others: Vec<_> = (1..handles).map(|_| live.clone()).collect();
    let retiring: Vec<_> = (0..retirers)
        .map(|_| live.clone().retire(tokio::io::empty(), tokio::io::sink()))
        .collect();
    drop(others);
    drop(live);
    let mut returned = join_all(retiring)
        .await
        .into_iter()
        .map(|retired| match retired {
            Retire::Recovered { peer, .. } => peer,
            other => panic!("a dead connection fails before the point of no return: {other:?}"),
        });
    let peer = returned.next().expect("at least one handle retires");
    for other in returned {
        assert_eq!(
            other.dangerously_alias_party(),
            identity,
            "every handle returned is the same peer"
        );
    }
    peer
}

/// The most rounds a leaked join service is polled when a case leaks it
/// wherever the rounds run out, rather than at its stalled store.
///
/// For the replicas the test builds, the checkpoint's store begins in the
/// second round and the donation's in about the twentieth, so draws below this
/// limit leak the service before its fork is reserved, at its checkpoint's
/// store, and throughout reconciliation.
const MAX_LEAK_POLLS: usize = 24;

/// One session serving a joining peer, leaked, and the failed retirement after
/// it.
#[derive(Clone, Copy, Debug)]
struct Leak {
    /// Which of the session's stores waits forever: its checkpoint's (1) or its
    /// donation's (2). The donated fork is never sent either way.
    stalled_store: usize,
    /// How many rounds to poll the session and its joiner before leaking it, or
    /// `None` to poll them until the stalled store begins waiting.
    polls: Option<usize>,
    /// The generation's ordinary handles when its handles retire.
    handles: usize,
    /// The handles retiring over dead connections.
    retirers: usize,
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
        .prop_map(|(stalled_store, polls, handles, retirers)| Leak {
            stalled_store,
            polls,
            handles,
            retirers,
        })
}

divided_proptest! {
    #![divisor(SESSION_DIVISOR)]

    /// A session leaked anywhere in serving a joining peer pauses identity
    /// reclamation only until a retirement of the peer fails.
    ///
    /// The next session reclaims the fork the leaked session reserved, so
    /// the live parties again partition the whole identity space, none held
    /// twice. The leaked fork was never sent, and its region is still in the
    /// bookmark's record, so reclaiming it is what makes the partition
    /// whole. Each leak is followed by a failed retirement, and a joiner
    /// served at the end receives a fork of the reclaimed space without
    /// overlap.
    #[test]
    fn failed_retirement_resumes_reclamation_after_a_leaked_join(
        leaks in vec(leak(), 1..4),
        network in network_seed(),
    ) {
        run_to_quiescence(async {
            let storage = StallingBookmark::default();
            let mut live = network
                .builder::<u64>()
                .bookmark(storage.clone()).seed()
                .await
                .expect("in-memory storage attaches");
            live.send(0).unwrap();
            let remote = join_from(&live).await;

            for (round, leak) in (1u64..).zip(leaks) {
                // A fresh write makes the leaked session's checkpoint store,
                // so that its stores count from the checkpoint's.
                live.send(round).unwrap();
                storage.arm_at(leak.stalled_store);
                let (mut near, mut far) = memory_connection(MEMORY_CAPACITY);
                let mut leaked = Box::pin(live.gossip_once(&mut near.read, &mut near.write));
                let mut joiner = Box::pin(Peer::<u64>::join(&mut far.read, &mut far.write));
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

                live = reclaim(live, leak.handles, leak.retirers).await;
                live.send(round + 1_000).unwrap();
                let (mut near, mut far) = memory_connection(MEMORY_CAPACITY);
                let (local, partner) = futures::join!(
                    live.gossip_once(&mut near.read, &mut near.write),
                    remote.gossip_once(&mut far.read, &mut far.write),
                );
                local.expect("the returned peer's session completes");
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
        .expect("sessions on the returned peer make progress");
    }
}

divided_proptest! {
    #![divisor(SESSION_DIVISOR)]

    /// The peer a failed retirement returns after a session future leaked
    /// while waiting on bookmark storage completes later sessions.
    ///
    /// The leaked future keeps the bookmark locked. The claim holds whatever
    /// the number of ordinary and retiring handles the generation had.
    #[test]
    fn failed_retirement_releases_a_leaked_sessions_bookmark(
        handles in 1usize..4,
        retirers in 1usize..4,
        network in network_seed(),
    ) {
        run_to_quiescence(async {
            let storage = StallingBookmark::default();
            let live = network
                .builder::<u64>()
                .bookmark(storage.clone()).seed()
                .await
                .expect("in-memory storage attaches");
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

            // Retire every handle over a dead connection while several
            // ordinary handles exist and several handles retire.
            let live = reclaim(live, handles, retirers).await;

            // A later session takes the bookmark lock and completes.
            let (mut near, mut far) = memory_connection(MEMORY_CAPACITY);
            let (local, partner) = futures::join!(
                live.gossip_once(&mut near.read, &mut near.write),
                remote.gossip_once(&mut far.read, &mut far.write),
            );
            local.expect("the returned peer's session completes");
            partner.expect("its partner's session completes");
            assert_eq!(remote.snapshot().len(), 2, "the later session delivers the leaked session's message");
        })
        .expect("sessions on the returned peer make progress");
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
                .builder::<u64>()
                .bookmark(storage.clone()).seed()
                .await
                .expect("in-memory storage attaches");
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
