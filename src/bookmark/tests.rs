//! Storage failures invalidate the cache, and bookmarks need not be `Sync`.

use std::cell::{Cell, RefCell};
use std::future::{Future, pending, ready};
use std::io::Cursor;

use super::record::NetworkRecord;
use before::Clock;
use proptest::prelude::*;

use super::*;
use crate::{Joined, Peer, link, testing::run_to_quiescence};

/// Whether a store confirms durability or fails on either side of replacement.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Outcome {
    /// Replace the record and confirm durability.
    #[default]
    Success,
    /// Leave storage unchanged and return an error.
    FailBefore,
    /// Replace the record, then return an error.
    FailAfter,
    /// Leave storage unchanged and wait until the caller cancels.
    PauseBefore,
    /// Replace the record and wait until the caller cancels.
    PauseAfter,
}

/// Owned storage with synchronous access and a one-shot store failure.
///
/// `RefCell` makes this `Send` but not `Sync`: the trait's returned futures
/// own their results and must not require sharing the bookmark itself.
#[derive(Debug, Default)]
struct Memory {
    /// The complete record currently in storage, acknowledged or not.
    bytes: RefCell<Option<Vec<u8>>>,
    /// The next store's result and whether it changes storage.
    outcome: Cell<Outcome>,
    /// Reads performed, used to check cache invalidation.
    loads: Cell<usize>,
}

/// Return owned futures without capturing a shared reference to this storage.
impl Bookmark for Memory {
    /// The injected storage fault.
    type Error = std::io::Error;
    /// A snapshot of the stored bytes.
    type Reader = Cursor<Vec<u8>>;

    /// Count the read and copy the stored bytes into an independent reader.
    fn load(&self) -> impl Future<Output = Result<Option<Self::Reader>, Self::Error>> + Send {
        self.loads.set(self.loads.get() + 1);
        ready(Ok(self.bytes.borrow().clone().map(Cursor::new)))
    }

    /// Apply the scheduled outcome, then resume successful writes.
    fn store(&self, bytes: Vec<u8>) -> impl Future<Output = Result<(), Self::Error>> + Send {
        let outcome = self.outcome.take();
        if !matches!(outcome, Outcome::FailBefore | Outcome::PauseBefore) {
            *self.bytes.borrow_mut() = Some(bytes);
        }
        async move {
            match outcome {
                Outcome::Success => Ok(()),
                Outcome::FailBefore | Outcome::FailAfter => {
                    Err(std::io::Error::other("injected store failure"))
                }
                Outcome::PauseBefore | Outcome::PauseAfter => pending().await,
            }
        }
    }
}

/// Require the public future to remain usable on an executor that needs `Send`.
fn sendable<F: Future + Send>(future: F) -> F {
    future
}

/// A non-Sync bookmark supports Send attachment and gossip futures.
#[test]
fn non_sync_bookmark_can_attach_and_gossip() {
    run_to_quiescence(async {
        let rumors = Peer::<u64>::seed().into_rumors();
        rumors.send(7).unwrap();
        let peer = rumors.try_into_peer().await.unwrap();
        let peer = sendable(peer.bookmark(Memory::default())).await.unwrap();
        let rumors = peer.into_rumors();
        let (mut near, mut far) = link::memory();
        let (served, joined) = futures::join!(
            sendable(rumors.gossip_once(&mut near)),
            sendable(Peer::<u64>::bootstrap().join(&mut far)),
        );
        served.unwrap();
        let Joined::Joined { peer } = joined else {
            panic!("the peer must join")
        };
        let received = peer.into_rumors().snapshot();
        assert_eq!(
            received.iter().map(|(_, value)| *value).collect::<Vec<_>>(),
            [7]
        );
    })
    .expect("the session must make progress");
}

proptest! {
    /// Cancelling before or after replacement forces a reload and a durable retry,
    /// for both a checkpoint and removal of transferred rights.
    #[test]
    fn cancelled_stores_reload_before_retry(after: bool, donation: bool, ticks in 1u64..12) {
        let network = Network::from_bytes([0x63; 16]);
        let (mut party, mut version) = Clock::seed().into_parts();
        let mut bookmark = Bookmarked::new(Memory::default());
        let loaded = run_to_quiescence(bookmark.ensure_loaded()).unwrap().unwrap();
        loaded.checkpoint(network, &mut party, &version, true);
        run_to_quiescence(bookmark.write()).unwrap().unwrap();
        let prior = bookmark.persist.bytes.borrow().clone().unwrap();

        version.ticks(&party, ticks);
        let loaded = run_to_quiescence(bookmark.ensure_loaded()).unwrap().unwrap();
        if donation {
            loaded.slice(network, &party);
        } else {
            loaded.checkpoint(network, &mut party, &version, true);
        }
        let replacement = format::encode(&loaded.record);
        bookmark.persist.outcome.set(if after { Outcome::PauseAfter } else { Outcome::PauseBefore });
        prop_assert!(matches!(run_to_quiescence(bookmark.write()), Err(crate::testing::Quiescence::Stalled)));

        // Cancellation cannot tell us which complete record reached storage.
        // Reload that record rather than trusting either cached candidate.
        let reads = bookmark.persist.loads.get();
        let loaded = run_to_quiescence(bookmark.ensure_loaded()).unwrap().unwrap();
        prop_assert!(!loaded.can_skip_checkpoint(&party, &version));
        prop_assert_eq!(format::encode(&loaded.record), if after { replacement } else { prior });
        if donation { loaded.slice(network, &party); }
        else { loaded.checkpoint(network, &mut party, &version, true); }
        let expected = format::encode(&loaded.record);
        prop_assert_eq!(bookmark.persist.loads.get(), reads + 1);
        run_to_quiescence(bookmark.write()).unwrap().unwrap();
        prop_assert_eq!(bookmark.persist.bytes.borrow().clone(), Some(expected));
    }

    /// After any failed checkpoint, the next update reloads the durable record
    /// and cannot use an in-memory checkpoint to suppress its retry.
    #[test]
    fn failed_checkpoints_reload_before_retry(
        attempts in proptest::collection::vec((1usize..=4, prop_oneof![
            Just(Outcome::Success), Just(Outcome::FailBefore), Just(Outcome::FailAfter),
        ]), 1..24),
    ) {
        let network = Network::from_bytes([0x5a; 16]);
        let (mut party, mut version) = Clock::seed().into_parts();
        let mut bookmark = Bookmarked::new(Memory::default());
        for (ticks, outcome) in attempts {
            for _ in 0..ticks { version.tick(&party); }
            let prior = bookmark.persist.bytes.borrow().clone();
            let loaded = run_to_quiescence(bookmark.ensure_loaded()).unwrap().unwrap();
            prop_assert!(!loaded.can_skip_checkpoint(&party, &version));
            loaded.checkpoint(network, &mut party, &version, true);
            let replacement = format::encode(&loaded.record);
            bookmark.persist.outcome.set(outcome);
            let stored = run_to_quiescence(bookmark.write()).unwrap();
            let fail = outcome != Outcome::Success;
            prop_assert_eq!(stored.is_err(), fail);
            let expected = if outcome == Outcome::FailBefore { prior } else { Some(replacement) };
            prop_assert_eq!(bookmark.persist.bytes.borrow().clone(), expected.clone());
            if fail {
                prop_assert!(matches!(stored, Err(BookmarkIo::Io(_))));
            }
            let reads = bookmark.persist.loads.get();
            let loaded = run_to_quiescence(bookmark.ensure_loaded()).unwrap().unwrap();
            prop_assert_eq!(loaded.can_skip_checkpoint(&party, &version), !fail);
            if fail {
                let expected = expected.unwrap_or_else(|| format::encode(&Record::default()));
                prop_assert_eq!(format::encode(&loaded.record), expected);
            }
            prop_assert_eq!(bookmark.persist.loads.get(), reads + usize::from(fail));
        }
    }
}

proptest! {
    /// Every retained alias must protect all recorded writes in its region,
    /// including when storage contains overlapping claims with different ages.
    #[test]
    fn overlapping_identities_share_recovery_requirements(
        ticks in proptest::collection::vec(1u64..8, 1..9),
        caught_up in any::<bool>(),
    ) {
        let network = Network::from_bytes([0x7c; 16]);
        let (mut provider, mut version) = Clock::seed().into_parts();
        let mut owner = provider.fork();
        let owned = owner.dangerously_alias();
        let mut record = NetworkRecord::default();
        record.record(&owner, &version);
        let mut reserved = Vec::new();
        for count in ticks {
            reserved.push(owner.fork());
            version.ticks(&owner, count);
            record.record(&owner, &version);
        }
        // There is only one write frontier: even the oldest, largest identity
        // must wait for writes recorded by the newer, smaller ones.
        let mut records = Record::default();
        assert!(records.networks.insert(network, record));
        let mut bookmark = Bookmarked::new(Memory::default());
        *bookmark.persist.bytes.borrow_mut() = Some(format::encode(&records));
        drop((owner, reserved));
        let known = if caught_up { version.clone() } else { Version::new() };
        let mut restarted = provider.fork();
        let loaded = run_to_quiescence(bookmark.ensure_loaded()).unwrap().unwrap();
        loaded.checkpoint(network, &mut restarted, &known, true);
        prop_assert!(&version / &restarted <= known);
        if caught_up {
            prop_assert!(restarted.covers(&owned));
        }
    }

    /// Discarding any subset of recovery claims cannot authorize an identity
    /// whose previous writes the restarted peer has not yet learned.
    #[test]
    fn discarding_records_does_not_allow_version_reuse(
        checkpoints in proptest::collection::vec((1u64..8, any::<bool>()), 1..9),
        keep_initial in any::<bool>(),
        catch_up_after in any::<usize>(),
    ) {
        let network = Network::from_bytes([0x7c; 16]);
        let mut provider = Party::seed();
        let mut owner = provider.fork();
        let mut written = Version::new();
        let mut record = NetworkRecord::default();
        record.record(&owner, &written);
        let mut pending = Vec::new();
        let mut emitted = Vec::new();

        // Each fork leaves an older, larger recovery claim in the bookmark.
        // Later writes use the smaller identity that remains with the owner.
        for (ticks, _) in &checkpoints {
            pending.push(owner.fork());
            for _ in 0..*ticks {
                written.tick(&owner);
                emitted.push(written.clone());
            }
            record.record(&owner, &written);
        }

        // Choose each claim independently, including the initial large one.
        // Compaction may forget writes only where no retained claim owns rights.
        let mut keep = std::iter::once(keep_initial).chain(checkpoints.iter().map(|(_, keep)| *keep));
        record.identities.retain(|_| keep.next().unwrap());
        record.compact();
        let mut records = Record::default();
        assert!(records.networks.insert(network, record));
        let stored = format::encode(&records);
        drop((owner, pending, records)); // The old incarnation has crashed.

        // Restart with a fresh, disjoint identity and any prefix of the old
        // writes. Keep the complete history separately as the test's oracle.
        let learned = catch_up_after % (emitted.len() + 1);
        let known = emitted[..learned].last().cloned().unwrap_or_default();
        let mut restarted = provider.fork();
        let mut restored = format::decode(&stored).unwrap();
        restored.networks.get(&network).unwrap().reclaim(&mut restarted, &known);

        // Projection selects every old write in the identity we can now use.
        // We must already know all of them before we may issue another version.
        // Checking only the pruned bookmark would hide forgotten writes. Checking
        // that the next whole version differs is also insufficient: the fresh
        // identity can make it different even when recovery was unsafe.
        prop_assert!(&written / &restarted <= known,
            "recovery granted an identity whose previous writes are still unknown");
    }

}

proptest! {
    /// The public limit applies to the first bootstrap checkpoint, survives the
    /// Rumors-to-Peer round trip, and lowering it forces another store.
    #[test]
    fn configured_limit_controls_checkpoints(limit in 0usize..300, configure_first in any::<bool>()) {
        run_to_quiescence(async {
            let seed = Peer::<u64>::seed().into_rumors();
            let bootstrap = if configure_first {
                Peer::<u64>::bootstrap().bookmark_size_limit(limit).bookmark(Memory::default())
            } else {
                Peer::<u64>::bootstrap().bookmark(Memory::default()).bookmark_size_limit(limit)
            };
            let (mut near, mut far) = link::memory();
            let (served, joined) = futures::join!(seed.gossip_once(&mut near), bootstrap.join(&mut far));
            served.unwrap();
            let Joined::Joined { peer } = joined else { panic!("reliable bootstrap must join") };
            {
                let bookmark = peer.bookmark.lock().await;
                assert_eq!(bookmark.size_limit(), limit.max(format::record_size(0, 0)));
                assert!(bookmark.persist.bytes.borrow().as_ref().unwrap().len() <= bookmark.size_limit());
            }
            let rumors = peer.into_rumors();
            drop(rumors.clone());
            let peer = rumors.try_into_peer().await.unwrap().bookmark_size_limit(0);
            let rumors = peer.into_rumors();
            let (mut near, mut far) = link::memory();
            let (a, b) = futures::join!(seed.gossip_once(&mut near), rumors.gossip_once(&mut far));
            a.unwrap(); b.unwrap();
            let peer = rumors.try_into_peer().await.unwrap();
            let bookmark = peer.bookmark.lock().await;
            let bytes = bookmark.persist.bytes.borrow();
            let bytes = bytes.as_ref().unwrap();
            assert_eq!(bytes.len(), format::record_size(0, 0));
            assert!(format::decode(bytes).unwrap().networks.is_empty());
        }).expect("sessions must make progress");
    }

    /// Failed attachment retains the chosen limit even if the failed store
    /// replaced bytes.
    #[test]
    fn failed_attachment_preserves_limit(limit in 0usize..300, after in any::<bool>()) {
        run_to_quiescence(async {
            let rumors = Peer::<u64>::seed().into_rumors();
            rumors.send(7).unwrap();
            let peer = rumors.try_into_peer().await.unwrap().bookmark_size_limit(limit);
            let storage = Memory::default();
            storage.outcome.set(if after { Outcome::FailAfter } else { Outcome::FailBefore });
            let unbookmarked = peer.bookmark(storage).await.unwrap_err();
            let peer = unbookmarked.peer.bookmark(Memory::default()).await.unwrap();
            let bookmark = peer.bookmark.lock().await;
            assert_eq!(bookmark.size_limit(), limit.max(format::record_size(0, 0)));
            assert!(bookmark.persist.bytes.borrow().as_ref().unwrap().len() <= bookmark.size_limit());
        }).unwrap();
    }
}
