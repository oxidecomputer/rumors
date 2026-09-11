//! Bootstrap configuration survives retries and reaches the joined peer.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use proptest::prelude::*;

use super::{Bootstrap, Joined};
use crate::Peer;
use crate::bookmark::{Bookmark, BookmarkError, Serialized};
use crate::observe::{Observer, SessionInfo, SessionObserver};
use crate::testing::run_to_quiescence;
use crate::tree::mirror::streaming::remote::RunBudget;
use crate::tree::mirror::streaming::window::{DEFAULT_SYNC_MEMORY_BUDGET, WindowConfig};

/// Read the byte budget chosen through the public builder.
fn budget_bytes(window: WindowConfig) -> usize {
    match window {
        WindowConfig::Budget(bytes) => bytes,
        WindowConfig::Fixed(_) => panic!("the builder selects byte budgets"),
    }
}

/// Record session starts without collecting individual protocol events.
#[derive(Default)]
struct Sessions(AtomicUsize);

/// Count each attempt and each subsequent gossip on the joined peer.
impl Observer for Sessions {
    /// Record a start without installing a per-session handler.
    fn session(&self, _: &SessionInfo) -> Option<Box<dyn SessionObserver>> {
        self.0.fetch_add(1, Ordering::Relaxed);
        None
    }
}

/// Storage state exposed to assertions without sharing the bookmark itself.
#[derive(Default)]
struct Stored {
    /// Committed bytes, absent until the first successful store.
    bytes: Mutex<Option<Vec<u8>>>,
    /// Loads attempted through the bookmark.
    loads: AtomicUsize,
    /// Stores committed through the bookmark.
    stores: AtomicUsize,
}

/// A non-Clone bookmark that owns access to the inspected storage.
struct Storage(Arc<Stored>);

/// Propagate errors from serialization into the in-memory writer.
impl BookmarkError for Storage {
    /// A serialization failure before the record is committed.
    type Error = std::io::Error;
}

/// Model atomic replacement while allowing the test to inspect committed bytes.
impl Bookmark for Storage {
    /// An owned snapshot of the committed bytes.
    type Reader = std::io::Cursor<Vec<u8>>;

    /// Count the read and return a snapshot of committed bytes.
    async fn load(&self) -> Result<Option<Self::Reader>, Self::Error> {
        self.0.loads.fetch_add(1, Ordering::Relaxed);
        Ok(self
            .0
            .bytes
            .lock()
            .unwrap()
            .clone()
            .map(std::io::Cursor::new))
    }

    /// Publish the serialized record only after the entire write succeeds.
    async fn store<F>(&self, write: F) -> Result<(), Self::Error>
    where
        F: for<'a> FnOnce(&'a mut (dyn tokio::io::AsyncWrite + Unpin + Send)) -> Serialized<'a>
            + Send,
    {
        let mut bytes = Vec::new();
        write(&mut bytes).await?;
        *self.0.bytes.lock().unwrap() = Some(bytes);
        self.0.stores.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}

/// Retry failed and mutually bootstrapping sessions, then join and gossip.
fn retry_then_join<B: Bookmark>(
    mut bootstrap: Bootstrap<u64, B>,
    fail: &[bool],
    sessions: &Sessions,
    stored: Option<&Stored>,
    expected: (usize, RunBudget, crate::PayloadDepthLimit),
) {
    let (budget, target, depth) = expected;
    assert_eq!(budget_bytes(bootstrap.window), budget);
    assert_eq!(bootstrap.run_budget, target);
    assert_eq!(bootstrap.payload_depth_limit, depth);
    run_to_quiescence(async {
        for (attempt, &fail) in fail.iter().enumerate() {
            let (mut near, mut far) = crate::link::memory();
            bootstrap = if fail {
                drop(far);
                let Joined::Failed { bootstrap, .. } = bootstrap.join(&mut near).await else {
                    panic!("a closed provider must fail the attempt");
                };
                bootstrap
            } else {
                let (ours, theirs) = tokio::join!(
                    bootstrap.join(&mut near),
                    Peer::<u64>::bootstrap()
                        .payload_depth_limit(depth)
                        .join(&mut far),
                );
                assert!(matches!(theirs, Joined::Bailed { .. }));
                let Joined::Bailed { bootstrap } = ours else {
                    panic!("two bootstrappers must both return their builders");
                };
                bootstrap
            };
            assert_eq!(budget_bytes(bootstrap.window), budget);
            assert_eq!(bootstrap.run_budget, target);
            assert_eq!(bootstrap.payload_depth_limit, depth);
            assert_eq!(sessions.0.load(Ordering::Relaxed), attempt + 1);
            if let Some(stored) = stored {
                assert_eq!(stored.loads.load(Ordering::Relaxed), 0);
                assert_eq!(stored.stores.load(Ordering::Relaxed), 0);
            }
        }

        let provider = Peer::<u64>::seed().payload_depth_limit(depth).into_rumors();
        provider.send(42).unwrap();
        let (mut near, mut far) = crate::link::memory();
        let (joined, served) =
            tokio::join!(bootstrap.join(&mut near), provider.gossip_once(&mut far));
        served.unwrap();
        let Joined::Joined { peer } = joined else {
            panic!("the returned builder must remain usable with an established provider");
        };
        assert_eq!(budget_bytes(peer.window), budget);
        assert_eq!(peer.run_budget, target);
        assert_eq!(peer.codec.limit(), depth);
        assert_eq!(sessions.0.load(Ordering::Relaxed), fail.len() + 1);
        if let Some(stored) = stored {
            assert_eq!(stored.loads.load(Ordering::Relaxed), 1);
            assert_eq!(stored.stores.load(Ordering::Relaxed), 1);
            let bytes = stored.bytes.lock().unwrap();
            let record = crate::bookmark::format::decode(bytes.as_ref().unwrap()).unwrap();
            assert!(record.contains_key(&provider.network()));
        }

        // Exercise the retained observer and identity after the bootstrap,
        // including a write that must survive reconciliation in both directions.
        let joined = peer.into_rumors();
        assert_eq!(joined.snapshot(), provider.snapshot());
        joined.send(7).unwrap();
        let (ours, theirs) = tokio::join!(
            joined.gossip_once(&mut near),
            provider.gossip_once(&mut far)
        );
        assert!(ours.is_ok(), "the joined peer must complete gossip");
        theirs.unwrap();
        assert_eq!(sessions.0.load(Ordering::Relaxed), fail.len() + 2);
        assert_eq!(joined.snapshot(), provider.snapshot());
        assert_eq!(provider.snapshot().len(), 2);
    })
    .expect("bootstrap retries and later gossip must make progress");
}

proptest! {
    /// Both forms retain every setting, observer and storage across mixed retry sequences.
    #[test]
    fn retries_preserve_configuration_and_storage(
        budget in 0usize..1024 * 1024,
        target in 0usize..512,
        depth in 1u64..32,
        attempts in prop::collection::vec(any::<bool>(), 0..6),
        configure_after_bookmark in any::<bool>(),
    ) {
        let depth = crate::PayloadDepthLimit::new(depth);
        let expected = (budget, RunBudget::from_bytes(target), depth);
        let config = Peer::<u64>::bootstrap()
            .sync_memory_budget(budget)
            .target_message_size(target)
            .payload_depth_limit(depth);
        let plain_sessions = Arc::new(Sessions::default());
        retry_then_join(
            config.clone().observe(plain_sessions.clone()).clone(),
            &attempts, &plain_sessions, None, expected,
        );

        let stored = Arc::new(Stored::default());
        let bookmarked_sessions = Arc::new(Sessions::default());
        let config = if configure_after_bookmark {
            Peer::<u64>::bootstrap().bookmark(Storage(stored.clone()))
                .sync_memory_budget(budget)
                .target_message_size(target)
                .payload_depth_limit(depth)
                .observe(bookmarked_sessions.clone())
        } else {
            config.observe(bookmarked_sessions.clone()).bookmark(Storage(stored.clone()))
        };
        retry_then_join(
            config,
            &attempts, &bookmarked_sessions, Some(&stored), expected,
        );
    }
}

/// Default joins inherit the seed's defaults, and oversized targets saturate.
#[test]
fn configuration_defaults_and_saturation() {
    let config = Peer::<u64>::bootstrap();
    assert_eq!(budget_bytes(config.window), DEFAULT_SYNC_MEMORY_BUDGET);
    assert_eq!(config.run_budget, RunBudget::default());
    assert_eq!(
        config.payload_depth_limit,
        crate::PayloadDepthLimit::default()
    );
    let sessions = Arc::new(Sessions::default());
    retry_then_join(
        config.observe(sessions.clone()),
        &[],
        &sessions,
        None,
        (
            DEFAULT_SYNC_MEMORY_BUDGET,
            RunBudget::default(),
            crate::PayloadDepthLimit::default(),
        ),
    );
    let saturated = Peer::<u64>::bootstrap().target_message_size(usize::MAX);
    assert_eq!(saturated.run_budget, RunBudget::from_bytes(usize::MAX));
    assert_ne!(saturated.run_budget.bytes(), usize::MAX);
}
