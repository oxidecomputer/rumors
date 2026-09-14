//! Catching up permits reclamation; a checkpoint actually performs it.

use proptest::prelude::*;

use super::{Peer, Probe, block_on, bootstrap_fork_peer, plain_gossip};
use crate::common::flaky::persisted_record;

/// A checkpoint during bootstrap must not leave an older overlapping entry
/// that lets a restarted peer reuse versions of writes it has not recovered.
#[test]
fn concurrent_bootstrap_checkpoint_protects_writes_after_restart() {
    block_on(async {
        let seed = Peer::<u64>::seed().sync_window_floor().into_rumors();
        let witness = bootstrap_fork_peer(&seed).await.into_rumors();
        let bookmark = Probe::default();
        let owner = bootstrap_fork_peer(&seed)
            .await
            .bookmark(bookmark.clone())
            .await
            .unwrap()
            .into_rumors();
        plain_gossip(&owner, &seed).await;
        let before = owner.dangerously_alias_party();
        let writer;
        {
            let (mut near, mut far) = rumors::link::memory();
            let mut joining = Box::pin(Peer::<u64>::bootstrap().join(&mut near));
            let mut serving = Box::pin(owner.gossip_once(&mut far));
            // Let the newcomer send its preamble, then let the provider reserve
            // a fork. Neither future runs again while a second session proceeds.
            assert!(futures::poll!(joining.as_mut()).is_pending());
            assert!(futures::poll!(serving.as_mut()).is_pending());
            writer = owner.dangerously_alias_party();
            assert_ne!(writer, before, "the provider must have reserved a fork");
            owner.send(42).unwrap();
            plain_gossip(&owner, &witness).await;
        }
        // Cancelling the bootstrap returns its reserved fork without another
        // store. The ensuing crash preserves only the two overlapping entries.
        drop(owner);
        let restarted = bootstrap_fork_peer(&seed)
            .await
            .bookmark(bookmark.clone())
            .await
            .unwrap()
            .into_rumors();
        plain_gossip(&restarted, &seed).await;
        assert!(restarted.snapshot().is_empty());
        assert!(
            restarted.dangerously_alias_party().is_disjoint(&writer),
            "an older overlapping entry must not authorize reuse of unseen writes"
        );
    });
}

proptest! {
    /// Outstanding bootstrap forks defer reclamation without delaying durable
    /// checkpoints. Cancelling or completing the last fork permits recovery again.
    #[test]
    fn pending_bootstraps_defer_reclamation_but_not_checkpoints(
        releases in proptest::collection::vec((any::<bool>(), any::<usize>()), 1..5),
    ) {
        block_on(async {
            let seed = Peer::<u64>::seed().sync_window_floor().into_rumors();
            let witness = bootstrap_fork_peer(&seed).await.into_rumors();
            let bookmark = Probe::default();
            let departed = bootstrap_fork_peer(&seed).await
                .bookmark(bookmark.clone()).await.unwrap().into_rumors();
            departed.send(42).unwrap();
            plain_gossip(&departed, &witness).await;
            let old_party = departed.dangerously_alias_party();
            drop(departed);

            // Restart without the old writes, so recovery cannot precede the
            // forks we are about to hold. Only the witness has those writes.
            let restarted = bootstrap_fork_peer(&seed).await
                .bookmark(bookmark.clone()).await.unwrap().into_rumors();
            let mut pending = Vec::new();
            for _ in &releases {
                let before = restarted.dangerously_alias_party();
                let (mut near, mut far) = rumors::link::memory();
                let provider = restarted.clone();
                let mut joining = Box::pin(async move {
                    Peer::<u64>::bootstrap().join(&mut near).await
                });
                let mut serving = Box::pin(async move { provider.gossip_once(&mut far).await });
                assert!(futures::poll!(joining.as_mut()).is_pending());
                assert!(futures::poll!(serving.as_mut()).is_pending());
                assert_ne!(before, restarted.dangerously_alias_party(), "a fork must be held");
                pending.push((joining, serving));
            }
            plain_gossip(&restarted, &witness).await;
            assert!(restarted.snapshot().iter().any(|(_, message)| *message == 42));

            let mut newcomers = Vec::new();
            for (step, (complete, pick)) in releases.into_iter().enumerate() {
                // Knowing the old history now permits recovery, but every
                // checkpoint must still leave it alone while any fork is held.
                restarted.send(100 + step as u64).unwrap();
                plain_gossip(&restarted, &witness).await;
                let current = restarted.dangerously_alias_party();
                assert!(current.is_disjoint(&old_party));
                let snapshot = restarted.snapshot();
                let own_writes = (snapshot.latest() / &current).to_version();
                let record = persisted_record(&bookmark.store);
                assert!(record[&restarted.network()].iter().any(|clock|
                    clock.party() == &current && own_writes <= *clock.version()
                ), "the checkpoint must protect local writes even while reclamation waits");

                // Vary both release order and whether each guard returns its
                // fork or hands it off. Keep recipients live to check disjointness.
                let index = pick % pending.len();
                let (joining, serving) = pending.swap_remove(index);
                if complete {
                    let (joined, served) = futures::join!(joining, serving);
                    served.unwrap();
                    let rumors::Joined::Joined { peer } = joined else {
                        panic!("reliable bootstrap must complete");
                    };
                    newcomers.push(peer);
                } else {
                    drop((joining, serving));
                }
                restarted.send(200 + step as u64).unwrap();
                plain_gossip(&restarted, &witness).await;
                let current = restarted.dangerously_alias_party();
                assert_eq!(current.covers(&old_party), pending.is_empty());
                for newcomer in &newcomers {
                    assert!(current.is_disjoint(&newcomer.dangerously_alias_party()));
                }
            }
        });
    }

    /// A restart retains an old identity until a checkpoint starts with enough
    /// history to reclaim it. Remote-only gossip does not force that checkpoint.
    #[test]
    fn reclamation_waits_for_a_checkpoint_after_catching_up(
        messages in 1u64..9,
        caught_up_on_join in any::<bool>(),
        edit_before_catching_up in any::<bool>(),
        remote_rounds in 0u64..5,
        redact in any::<bool>(),
    ) {
        block_on(async {
            // All peers descend from this seed. Only the witness learns the
            // departing peer's writes, so the seed can serve an incomplete restart.
            let seed = Peer::<u64>::seed().sync_window_floor().into_rumors();
            let witness = bootstrap_fork_peer(&seed).await.into_rumors();
            let bookmark = Probe::default();
            let departed = bootstrap_fork_peer(&seed).await
                .bookmark(bookmark.clone()).await.unwrap().into_rumors();
            for message in 0..messages {
                departed.send(message).unwrap();
            }
            plain_gossip(&departed, &witness).await;
            let old_party = departed.dangerously_alias_party();
            let old_version = departed.snapshot().latest().clone();
            drop(departed);

            let source = if caught_up_on_join { &witness } else { &seed };
            let restarted = bootstrap_fork_peer(source).await
                .bookmark(bookmark.clone()).await.unwrap().into_rumors();
            let fresh_party = restarted.dangerously_alias_party();
            assert!(fresh_party.is_disjoint(&old_party));
            assert_eq!(old_version <= *restarted.snapshot().latest(), caught_up_on_join);

            // Attachment records the fresh identity without reclaiming another.
            // This same check below distinguishes retaining a stored claim from
            // actually giving the restarted peer permission to write with it.
            let assert_waiting = || {
                assert_eq!(restarted.dangerously_alias_party(), fresh_party);
                let record = persisted_record(&bookmark.store);
                assert!(record[&restarted.network()].iter()
                    .any(|clock| clock.party() == &old_party));
            };
            assert_waiting();

            if !caught_up_on_join {
                // Establish a current checkpoint while history is still missing.
                plain_gossip(&restarted, &seed).await;
                assert_waiting();
                if edit_before_catching_up {
                    restarted.send(100).unwrap();
                }

                // Whether this session owes a checkpoint or skips it, that
                // decision precedes learning the missing history from the witness.
                plain_gossip(&restarted, &witness).await;
                assert!(old_version <= *restarted.snapshot().latest());
                assert_waiting();

                let stored = bookmark.store.lock().unwrap().clone();
                let calls = bookmark.log.lock().unwrap().len();
                for round in 0..remote_rounds {
                    witness.send(1_000 + round).unwrap();
                    plain_gossip(&restarted, &witness).await;
                    assert_waiting();
                }
                assert_eq!(bookmark.log.lock().unwrap().len(), calls);
                assert_eq!(*bookmark.store.lock().unwrap(), stored);

                // A local edit requires a checkpoint before the next exchange.
                if redact {
                    let snapshot = restarted.snapshot();
                    let (version, _) = snapshot.iter().find(|(_, value)| **value == 0).unwrap();
                    restarted.redact(version);
                } else {
                    restarted.send(2_000).unwrap();
                }
                assert_waiting();
            }

            // On a caught-up join this is the first checkpoint after attachment.
            // Otherwise it is the first one after catching up and making a local edit.
            plain_gossip(&restarted, &witness).await;
            let mut expected_party = fresh_party;
            expected_party.join(old_party).unwrap();
            assert_eq!(restarted.dangerously_alias_party(), expected_party);
            let record = persisted_record(&bookmark.store);
            let clocks = &record[&restarted.network()];
            assert_eq!(clocks.len(), 1);
            assert_eq!(clocks[0].party(), &expected_party);
            assert!(clocks[0].own_version() <= *restarted.snapshot().latest());
            assert_eq!(restarted.snapshot().hash(), witness.snapshot().hash());
        });
    }
}
