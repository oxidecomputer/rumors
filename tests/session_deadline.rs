//! Application-owned deadlines bound active exchanges without timing idle connections.

use rumors_testkit::common;

use std::sync::{Arc, Mutex};

use futures::channel::oneshot;
use futures::{FutureExt, StreamExt, stream};
use proptest::prelude::*;
use rumors::{Error, Gossip, Peer, Rumors, testing::run_to_quiescence};
use tokio::io::AsyncWriteExt;

use common::wire::{block_on, bootstrap_fork_async};
use rumors_testkit::common::property::SESSION_DIVISOR;
use rumors_testkit::common::seed::network_seed;
use rumors_testkit::divided_proptest;

/// Hand-controlled deadlines, retained by generation to detect accidental resets.
#[derive(Clone, Default)]
struct Deadlines(Arc<Mutex<Vec<Option<oneshot::Sender<()>>>>>);

/// Create and complete timers without an async runtime or a wall clock.
impl Deadlines {
    /// Register a fresh deadline and wait for its explicit expiry.
    fn start(&self) -> impl Future<Output = ()> + Send + use<> {
        let (tx, rx) = oneshot::channel();
        self.0.lock().unwrap().push(Some(tx));
        async { rx.await.expect("test retains each deadline sender") }
    }

    /// Count factory invocations, including completed sessions' deadlines.
    fn count(&self) -> usize {
        self.0.lock().unwrap().len()
    }

    /// Expire a particular session's deadline, proving it is still being awaited.
    fn expire(&self, generation: usize) {
        self.0.lock().unwrap()[generation]
            .take()
            .unwrap()
            .send(())
            .expect("session still owns its deadline");
    }

    /// Check that completion or cancellation released a session's timer.
    fn dropped(&self, generation: usize) -> bool {
        self.0.lock().unwrap()[generation]
            .as_ref()
            .unwrap()
            .is_canceled()
    }
}

proptest! {
    /// The first bytes start one deadline; trickled bytes and discarded next
    /// futures cannot reset it, and an interrupted session commits nothing.
    #[test]
    fn partial_preambles_are_timed_from_the_first_bytes(
        bytes in 1usize..30,
        idle_polls in 0usize..8,
        cancel in any::<bool>(),
        network in network_seed(),
    ) {
        block_on(async {
            let deadlines = Deadlines::default();
            let factory = deadlines.clone();
            let a: Rumors<u64> = network.peer().sync_memory_budget(0)
                .gossip_when(|_| stream::pending::<()>())
                .session_deadline(move || factory.start()).into_rumors();
            a.send(7).unwrap();
            let before = a.snapshot().hash();
            let (mut end, mut remote) = rumors::testing::memory_connection(rumors::testing::MEMORY_CAPACITY);
            let mut sessions = a.gossip(&mut end.read, &mut end.write);
            for _ in 0..idle_polls {
                assert!(sessions.next().now_or_never().is_none());
                assert_eq!(deadlines.count(), 0, "idle is untimed");
            }
            // Fewer than one full preamble: validation cannot yet accept or
            // reject the frame. The deadline still starts on its first byte.
            for _ in 0..bytes {
                remote.write.write_all(&[0xd9]).await.unwrap();
                assert!(sessions.next().now_or_never().is_none());
                assert_eq!(deadlines.count(), 1, "traffic must not reset the timer");
            }
            if cancel {
                drop(sessions);
                assert!(deadlines.dropped(0));
            } else {
                deadlines.expire(0);
                assert!(matches!(sessions.next().await, Some(Err(Error::DeadlineExceeded))));
                assert!(sessions.next().await.is_none(), "expiry is terminal");
                drop(sessions);
            }
            assert_eq!(a.snapshot().hash(), before, "no partial commit");
        });
    }

    /// Each connection receives its own Changes subscription, including its
    /// initial notification. A learned change reaches the other connection but
    /// never echoes.
    #[test]
    fn changes_factories_are_independent_and_suppression_stays_per_connection(
        rounds in 1u64..8,
        network in network_seed(),
    ) {
        block_on(async {
            let deadlines = Deadlines::default();
            let factory = deadlines.clone();
            let a: Rumors<u64> = network.peer().sync_memory_budget(0).into_rumors();
            let b = bootstrap_fork_async(&a).await;
            let c = bootstrap_fork_async(&a).await;
            let a = a.try_into_peer().await.unwrap()
                .gossip_when(|changes| changes.map(|()| Gossip::WhenChanged))
                .session_deadline(move || factory.start()).into_rumors();
            let other_handle = a.clone();
            let (mut ab, mut ba) = rumors::testing::memory_connection(rumors::testing::MEMORY_CAPACITY);
            let (mut ac, mut ca) = rumors::testing::memory_connection(rumors::testing::MEMORY_CAPACITY);
            let mut to_b = a.gossip(&mut ab.read, &mut ab.write);
            let mut to_c = other_handle.gossip(&mut ac.read, &mut ac.write);
            for round in 0..rounds {
                b.send(round).unwrap();
                let (left, right) = futures::join!(to_b.next(), b.gossip_once(&mut ba.read, &mut ba.write));
                left.unwrap().unwrap(); right.unwrap();
                let (left, right) = futures::join!(to_c.next(), c.gossip_once(&mut ca.read, &mut ca.write));
                left.unwrap().unwrap(); right.unwrap();
                assert_eq!(a.snapshot().hash(), b.snapshot().hash());
                assert_eq!(a.snapshot().hash(), c.snapshot().hash());
                assert_eq!(deadlines.count(), 2 * (round as usize + 1));
                assert!(deadlines.dropped(2 * round as usize));
                assert!(deadlines.dropped(2 * round as usize + 1));
                assert!(to_b.next().now_or_never().is_none());
                assert!(to_c.next().now_or_never().is_none());
                assert_eq!(deadlines.count(), 2 * (round as usize + 1), "suppressed requests are untimed");
            }
            drop(to_b); drop(to_c); drop(other_handle);
            // Idle driver cancellation leaves the connection at a session
            // boundary, ready for another session. Reclaim also
            // preserves the factories for explicit one-shot gossip.
            let a = a.try_into_peer().await.unwrap().into_rumors();
            let (left, right) = futures::join!(a.gossip_once(&mut ab.read, &mut ab.write), b.gossip_once(&mut ba.read, &mut ba.write));
            left.unwrap(); right.unwrap();
            assert_eq!(deadlines.count(), 2 * rounds as usize + 1);
        });
    }
}

divided_proptest! {
    #![divisor(SESSION_DIVISOR)]

    /// A one-shot forces initiation under a passive policy and uses its deadline.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn one_shot_uses_deadline_but_ignores_initiation_policy(network in network_seed()) {
        let deadlines = Deadlines::default();
        let factory = deadlines.clone();
        let a: Rumors<()> = network.peer()
            .gossip_when(|_| stream::pending::<()>())
            .session_deadline(move || factory.start())
            .into_rumors();
        let (mut end, _remote) = rumors::testing::memory_connection(rumors::testing::MEMORY_CAPACITY);
        let mut session = Box::pin(a.gossip_once(&mut end.read, &mut end.write));
        assert!(session.as_mut().now_or_never().is_none());
        assert_eq!(deadlines.count(), 1);
        deadlines.expire(0);
        assert!(matches!(
            run_to_quiescence(session).unwrap(),
            Err(Error::DeadlineExceeded)
        ));
    }

    /// A ready transport failure takes precedence over an equally ready deadline.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn an_available_session_result_wins_the_deadline_race(network in network_seed()) {
        let a: Rumors<()> = network.peer()
            .session_deadline(|| std::future::ready(()))
            .into_rumors();
        let (mut end, remote) = rumors::testing::memory_connection(rumors::testing::MEMORY_CAPACITY);
        drop(remote);
        assert!(matches!(
            block_on(a.gossip_once(&mut end.read, &mut end.write)),
            Err(Error::Transport(_))
        ));
    }
}

/// A configured timer is fresh on each session, measured from initiation rather
/// than connection age; idle time can exceed the deadline by any amount.
#[tokio::test(start_paused = true)]
async fn real_timers_leave_idle_connections_untimed() {
    use std::time::Duration;
    use tokio::time::{advance, sleep};

    let a: Rumors<()> = Peer::seed()
        .gossip_when(|_| stream::pending::<()>())
        .session_deadline(|| sleep(Duration::from_secs(1)))
        .into_rumors();
    let (mut end, mut remote) =
        rumors::testing::memory_connection(rumors::testing::MEMORY_CAPACITY);
    let mut sessions = a.gossip(&mut end.read, &mut end.write);
    assert!(sessions.next().now_or_never().is_none());
    advance(Duration::from_secs(60)).await;
    assert!(sessions.next().now_or_never().is_none());
    remote.write.write_all(&[0xd9]).await.unwrap();
    assert!(sessions.next().now_or_never().is_none());
    advance(Duration::from_millis(999)).await;
    assert!(sessions.next().now_or_never().is_none());
    advance(Duration::from_millis(1)).await;
    assert!(matches!(
        sessions.next().await,
        Some(Err(Error::DeadlineExceeded))
    ));
}

proptest! {
    /// Each expired join returns an untouched bookmark and retryable policy.
    /// Successful joining preserves that policy for the peer's later sessions.
    #[test]
    fn bootstrap_retries_preserve_policy(attempts in 1usize..5, network in network_seed()) {
        use common::flaky::{FaultFeed, FlakyInMemoryBookmark};
        use rumors::Joined;

        block_on(async {
            let a: Rumors<u64> = network.peer().sync_memory_budget(0).into_rumors();
            a.send(42).unwrap();
            let deadlines = Deadlines::default();
            let factory = deadlines.clone();
            let config = Peer::<u64>::bootstrap()
                .gossip_when(|_| stream::pending::<()>())
                .session_deadline(move || factory.start());
            let store = Arc::new(Mutex::new(None));
            let mut builder = config.clone().bookmark(FlakyInMemoryBookmark::new(
                store.clone(), Arc::new(Mutex::new(FaultFeed::new(vec![], vec![]))), 0,
            ));
            for generation in 0..attempts {
                let (mut end, _remote) = rumors::testing::memory_connection(rumors::testing::MEMORY_CAPACITY);
                let mut joining = Box::pin(builder.join(&mut end.read, &mut end.write));
                assert!(joining.as_mut().now_or_never().is_none());
                assert_eq!(deadlines.count(), generation + 1);
                deadlines.expire(generation);
                let Joined::Failed { bootstrap, error: Error::DeadlineExceeded } = joining.await else {
                    panic!("expiry must return the builder");
                };
                assert!(store.lock().unwrap().is_none(), "no post-join attachment");
                builder = bootstrap;
            }
            let (mut near, mut far) = rumors::testing::memory_connection(rumors::testing::MEMORY_CAPACITY);
            let (served, joined) = futures::join!(a.gossip_once(&mut near.read, &mut near.write), builder.join(&mut far.read, &mut far.write));
            served.unwrap();
            let Joined::Joined { peer, .. } = joined else { panic!("join succeeds") };
            assert_eq!(deadlines.count(), attempts + 1);
            assert!(deadlines.dropped(attempts));
            assert!(store.lock().unwrap().is_some());
            let b = peer.into_rumors();
            assert_eq!(a.snapshot().hash(), b.snapshot().hash());
            let mut sessions = b.gossip(&mut far.read, &mut far.write);
            assert!(sessions.next().now_or_never().is_none(), "passive policy survives join");
            assert_eq!(deadlines.count(), attempts + 1, "idle is untimed");
            drop(sessions);
            let mut session = Box::pin(b.gossip_once(&mut far.read, &mut far.write));
            assert!(session.as_mut().now_or_never().is_none());
            deadlines.expire(attempts + 1);
            assert!(matches!(session.await, Err(Error::DeadlineExceeded)));
        });
    }
}

divided_proptest! {
    #![divisor(SESSION_DIVISOR)]

    /// Failed bookmark attachment returns a peer with both gossip factories intact.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn failed_bookmark_attachment_preserves_policy(network in network_seed()) {
        use common::flaky::{FaultFeed, FlakyInMemoryBookmark};

        block_on(async {
            let deadlines = Deadlines::default();
            let factory = deadlines.clone();
            let a: Rumors<u64> = network.peer()
                .sync_memory_budget(0)
                .gossip_when(|_| stream::pending::<()>())
                .session_deadline(move || factory.start())
                .into_rumors();
            a.send(42).unwrap();
            let peer = a.try_into_peer().await.unwrap();
            let failure = peer
                .bookmark(FlakyInMemoryBookmark::new(
                    Arc::new(Mutex::new(None)),
                    Arc::new(Mutex::new(FaultFeed::new(vec![], vec![true]))),
                    0,
                ))
                .await
                .expect_err("attachment's write fails");
            let a = failure.peer.into_rumors();
            let (mut end, _remote) =
                rumors::testing::memory_connection(rumors::testing::MEMORY_CAPACITY);
            let mut sessions = a.gossip(&mut end.read, &mut end.write);
            assert!(sessions.next().now_or_never().is_none());
            assert_eq!(deadlines.count(), 0);
            drop(sessions);
            let mut session = Box::pin(a.gossip_once(&mut end.read, &mut end.write));
            assert!(session.as_mut().now_or_never().is_none());
            deadlines.expire(0);
            assert!(matches!(session.await, Err(Error::DeadlineExceeded)));
        });
    }
}
