//! Concurrent reclamation preserves the unique writable identity.

use std::future::{Future, poll_fn};
use std::task::Poll;

use futures::future::join_all;
use proptest::prelude::*;
use rumors::Peer;
use rumors::testing::run_to_quiescence;

proptest! {
    /// Competing reuniters wait for ordinary handles, survive cancelled
    /// waiters, and return exactly one usable Peer in each new generation.
    #[test]
    fn concurrent_reuniters_return_one_peer(
        competitors in 2usize..10,
        holders in 2usize..6,
        cancel in prop::collection::vec(any::<bool>(), 8),
        reverse in any::<bool>(),
        generations in 1usize..4,
    ) {
        let mut peer = Peer::<u64>::seed();
        let identity = peer.dangerously_alias_party();
        for generation in 0..generations {
            peer = run_to_quiescence(async {
                let live = peer.into_rumors();
                live.send(generation as u64).unwrap();
                let mut keepers: Vec<_> = (1..holders).map(|_| live.clone()).collect();
                let mut waiting: Vec<_> = (0..competitors)
                    .map(|_| Box::pin(live.clone().try_into_peer()))
                    .collect();
                keepers.push(live);
                if reverse {
                    waiting.reverse();
                }
                let mut first = true;
                poll_fn(|cx| {
                    if keepers.is_empty() {
                        return Poll::Ready(());
                    }
                    for waiter in &mut waiting {
                        assert!(waiter.as_mut().poll(cx).is_pending(),
                            "a writable handle still exists");
                    }
                    if first {
                        // These futures have shed their handles but still hold
                        // internal Peer values. Cancelling them must not claim one.
                        let mut index = 0;
                        waiting.retain(|_| {
                            let keep = index < 2 || !cancel[index - 2];
                            index += 1;
                            keep
                        });
                        first = false;
                    }
                    // Return Pending after each drop: progress must come from
                    // the handle's wake, including the last ordinary handle.
                    drop(keepers.pop().unwrap());
                    Poll::Pending
                }).await;
                let mut winners = join_all(waiting).await.into_iter().flatten();
                let winner = winners.next().expect("one reuniter must win");
                assert!(winners.next().is_none(), "the identity cannot be claimed twice");
                let live = winner.into_rumors();
                assert_eq!(live.dangerously_alias_party(), identity);
                assert_eq!(live.snapshot().len(), generation + 1);
                live.try_into_peer().await.unwrap()
            }).expect("reuniters must wake and finish after handles are dropped");
        }
    }
}
