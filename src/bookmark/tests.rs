//! Storage failures invalidate the cache, and bookmarks need not be `Sync`.

use std::cell::{Cell, RefCell};
use std::future::{Future, ready};
use std::io::Cursor;

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
        if outcome != Outcome::FailBefore {
            *self.bytes.borrow_mut() = Some(bytes);
        }
        ready(if outcome == Outcome::Success {
            Ok(())
        } else {
            Err(std::io::Error::other("injected store failure"))
        })
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
            prop_assert!(!loaded.is_current(&party, &version));
            loaded.reclaim(network, &mut party, &version);
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
            prop_assert_eq!(loaded.is_current(&party, &version), !fail);
            if fail {
                let expected = expected.unwrap_or_else(|| format::encode(&BTreeMap::new()));
                prop_assert_eq!(format::encode(&loaded.record), expected);
            }
            prop_assert_eq!(bookmark.persist.loads.get(), reads + usize::from(fail));
        }
    }
}
