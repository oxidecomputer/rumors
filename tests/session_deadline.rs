//! Application-owned deadlines bound active exchanges without timing idle links.

mod common;

use std::sync::{Arc, Mutex};

use futures::channel::oneshot;
use futures::{FutureExt, StreamExt, stream};
use proptest::prelude::*;
use rumors::{Error, Gossip, Peer, Rumors, testing::run_to_quiescence};
use tokio::io::AsyncWriteExt;

use common::wire::{block_on, bootstrap_fork_async};

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
    /// futures cannot reset it. Expiry or dropping the driver poisons the link.
    #[test]
    fn partial_preambles_are_timed_from_the_first_bytes(
        bytes in 1usize..30,
        idle_polls in 0usize..8,
        cancel in any::<bool>(),
    ) {
        block_on(async {
            let deadlines = Deadlines::default();
            let factory = deadlines.clone();
            let a: Rumors<u64> = Peer::seed().sync_window_floor()
                .gossip_when(|_| stream::pending::<()>())
                .session_deadline(move || factory.start()).into_rumors();
            a.send(7).unwrap();
            let before = a.snapshot().hash();
            let (mut link, remote) = rumors::link::memory();
            let mut remote = remote.into_parts();
            let mut sessions = a.gossip(&mut link);
            for _ in 0..idle_polls {
                assert!(sessions.next().now_or_never().is_none());
                assert_eq!(deadlines.count(), 0, "idle is untimed");
            }
            // Fewer than one full preamble: validation cannot yet accept or
            // reject the frame. The deadline still starts on its first byte.
            for _ in 0..bytes {
                remote.control_write.write_all(&[0xd9]).await.unwrap();
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
            assert!(matches!(a.gossip_once(&mut link).await, Err(Error::LinkPoisoned)));
            assert_eq!(deadlines.count(), 1, "poisoned reuse must not start a timer");
        });
    }

    /// Each connection receives its own Changes subscription, including its
    /// initial notification. A learned change reaches the other link but never echoes.
    #[test]
    fn changes_factories_are_independent_and_suppression_stays_per_link(rounds in 1u64..8) {
        block_on(async {
            let deadlines = Deadlines::default();
            let factory = deadlines.clone();
            let a: Rumors<u64> = Peer::seed().sync_window_floor().into_rumors();
            let b = bootstrap_fork_async(&a).await;
            let c = bootstrap_fork_async(&a).await;
            let a = a.try_into_peer().await.unwrap()
                .gossip_when(|changes| changes.map(|()| Gossip::WhenChanged))
                .session_deadline(move || factory.start()).into_rumors();
            let other_handle = a.clone();
            let (mut ab, mut ba) = rumors::link::memory();
            let (mut ac, mut ca) = rumors::link::memory();
            let mut to_b = a.gossip(&mut ab);
            let mut to_c = other_handle.gossip(&mut ac);
            for round in 0..rounds {
                b.send(round).unwrap();
                let (left, right) = futures::join!(to_b.next(), b.gossip_once(&mut ba));
                left.unwrap().unwrap(); right.unwrap();
                let (left, right) = futures::join!(to_c.next(), c.gossip_once(&mut ca));
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
            // Idle driver cancellation leaves the link usable. Reclaim also
            // preserves the factories for explicit one-shot gossip.
            let a = a.try_into_peer().await.unwrap().into_rumors();
            let (left, right) = futures::join!(a.gossip_once(&mut ab), b.gossip_once(&mut ba));
            left.unwrap(); right.unwrap();
            assert_eq!(deadlines.count(), 2 * rounds as usize + 1);
        });
    }
}

/// A one-shot forces initiation under a passive policy and uses its deadline.
#[test]
fn one_shot_uses_deadline_but_ignores_initiation_policy() {
    let deadlines = Deadlines::default();
    let factory = deadlines.clone();
    let a: Rumors<()> = Peer::seed()
        .gossip_when(|_| stream::pending::<()>())
        .session_deadline(move || factory.start())
        .into_rumors();
    let (mut link, _remote) = rumors::link::memory();
    let mut session = Box::pin(a.gossip_once(&mut link));
    assert!(session.as_mut().now_or_never().is_none());
    assert_eq!(deadlines.count(), 1);
    deadlines.expire(0);
    assert!(matches!(
        run_to_quiescence(session).unwrap(),
        Err(Error::DeadlineExceeded)
    ));
    assert!(matches!(
        block_on(a.gossip_once(&mut link)),
        Err(Error::LinkPoisoned)
    ));
}

/// A ready transport failure takes precedence over an equally ready deadline.
#[test]
fn an_available_session_result_wins_the_deadline_race() {
    let a: Rumors<()> = Peer::seed()
        .session_deadline(|| std::future::ready(()))
        .into_rumors();
    let (mut link, remote) = rumors::link::memory();
    drop(remote);
    assert!(matches!(
        block_on(a.gossip_once(&mut link)),
        Err(Error::Transport(_))
    ));
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
    let (mut link, remote) = rumors::link::memory();
    let mut remote = remote.into_parts();
    let mut sessions = a.gossip(&mut link);
    assert!(sessions.next().now_or_never().is_none());
    advance(Duration::from_secs(60)).await;
    assert!(sessions.next().now_or_never().is_none());
    remote.control_write.write_all(&[0xd9]).await.unwrap();
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
    fn bootstrap_retries_preserve_policy(attempts in 1usize..5) {
        use common::flaky::{FaultFeed, FlakyInMemoryBookmark};
        use rumors::Joined;

        block_on(async {
            let a: Rumors<u64> = Peer::seed().sync_window_floor().into_rumors();
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
                let (mut link, _remote) = rumors::link::memory();
                let mut joining = Box::pin(builder.join(&mut link));
                assert!(joining.as_mut().now_or_never().is_none());
                assert_eq!(deadlines.count(), generation + 1);
                deadlines.expire(generation);
                let Joined::Failed { bootstrap, error: Error::DeadlineExceeded } = joining.await else {
                    panic!("expiry must return the builder");
                };
                assert!(store.lock().unwrap().is_none(), "no post-join attachment");
                let Joined::Failed { bootstrap, error: Error::LinkPoisoned } = bootstrap.join(&mut link).await else {
                    panic!("the interrupted link must be poisoned");
                };
                assert_eq!(deadlines.count(), generation + 1, "poisoned reuse starts no deadline");
                builder = bootstrap;
            }
            let (mut near, mut far) = rumors::link::memory();
            let (served, joined) = futures::join!(a.gossip_once(&mut near), builder.join(&mut far));
            served.unwrap();
            let Joined::Joined { peer } = joined else { panic!("join succeeds") };
            assert_eq!(deadlines.count(), attempts + 1);
            assert!(deadlines.dropped(attempts));
            assert!(store.lock().unwrap().is_some());
            let b = peer.into_rumors();
            assert_eq!(a.snapshot().hash(), b.snapshot().hash());
            let mut sessions = b.gossip(&mut far);
            assert!(sessions.next().now_or_never().is_none(), "passive policy survives join");
            assert_eq!(deadlines.count(), attempts + 1, "idle is untimed");
            drop(sessions);
            let mut session = Box::pin(b.gossip_once(&mut far));
            assert!(session.as_mut().now_or_never().is_none());
            deadlines.expire(attempts + 1);
            assert!(matches!(session.await, Err(Error::DeadlineExceeded)));
        });
    }
}

/// Failed bookmark attachment returns a peer with both gossip factories intact.
#[test]
fn failed_bookmark_attachment_preserves_policy() {
    use common::flaky::{FaultFeed, FlakyInMemoryBookmark};

    block_on(async {
        let deadlines = Deadlines::default();
        let factory = deadlines.clone();
        let a: Rumors<u64> = Peer::seed()
            .sync_window_floor()
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
        let (mut link, _remote) = rumors::link::memory();
        let mut sessions = a.gossip(&mut link);
        assert!(sessions.next().now_or_never().is_none());
        assert_eq!(deadlines.count(), 0);
        drop(sessions);
        let mut session = Box::pin(a.gossip_once(&mut link));
        assert!(session.as_mut().now_or_never().is_none());
        deadlines.expire(0);
        assert!(matches!(session.await, Err(Error::DeadlineExceeded)));
    });
}
