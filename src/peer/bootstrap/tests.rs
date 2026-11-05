//! Bootstrap configuration survives retries and reaches the joined peer.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use proptest::prelude::*;

use super::{Bootstrap, Joined};
use crate::DEFAULT_SYNC_MEMORY_BUDGET;
use crate::Peer;
use crate::bookmark::Bookmark;
use crate::observe::{Observer, SessionInfo, SessionObserver};
use crate::testing::property::{NetworkSeed, SESSION_DIVISOR, divided_proptest, network_seed};
use crate::testing::run_to_quiescence;
use crate::tree::mirror::streaming::remote::RunBudget;

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

/// Model atomic replacement while allowing the test to inspect committed bytes.
impl Bookmark for Storage {
    /// Replacing an owned in-memory buffer cannot fail.
    type Error = std::convert::Infallible;
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

    /// Replace the complete record and count the successful store.
    async fn store(&self, bytes: Vec<u8>) -> Result<(), Self::Error> {
        *self.0.bytes.lock().unwrap() = Some(bytes);
        self.0.stores.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}

/// Retry failed and mutually bootstrapping sessions, then join the network
/// `network` founds and gossip.
fn retry_then_join<B: Bookmark>(
    network: NetworkSeed,
    mut bootstrap: Bootstrap<u64, B>,
    fail: &[bool],
    sessions: &Sessions,
    stored: Option<&Stored>,
    expected: (usize, RunBudget, crate::message::PayloadDepthLimit),
) {
    let (budget, target, depth) = expected;
    assert_eq!(bootstrap.sync_memory_budget, budget);
    assert_eq!(bootstrap.run_budget, target);
    assert_eq!(bootstrap.payload_depth_limit, depth);
    run_to_quiescence(async {
        for (attempt, &fail) in fail.iter().enumerate() {
            let (mut near, mut far) =
                crate::testing::memory_connection(crate::testing::MEMORY_CAPACITY);
            bootstrap = if fail {
                drop(far);
                let Joined::Failed { bootstrap, .. } =
                    bootstrap.join(&mut near.read, &mut near.write).await
                else {
                    panic!("a closed provider must fail the attempt");
                };
                bootstrap
            } else {
                let (ours, theirs) = tokio::join!(
                    bootstrap.join(&mut near.read, &mut near.write),
                    Peer::<u64>::bootstrap()
                        .payload_depth_limit(depth.get())
                        .join(&mut far.read, &mut far.write),
                );
                assert!(matches!(theirs, Joined::Bailed { .. }));
                let Joined::Bailed { bootstrap, .. } = ours else {
                    panic!("two bootstrappers must both return their builders");
                };
                bootstrap
            };
            assert_eq!(bootstrap.sync_memory_budget, budget);
            assert_eq!(bootstrap.run_budget, target);
            assert_eq!(bootstrap.payload_depth_limit, depth);
            assert_eq!(sessions.0.load(Ordering::Relaxed), attempt + 1);
            if let Some(stored) = stored {
                assert_eq!(stored.loads.load(Ordering::Relaxed), 0);
                assert_eq!(stored.stores.load(Ordering::Relaxed), 0);
            }
        }

        let provider = network
            .peer::<u64>()
            .payload_depth_limit(depth.get())
            .into_rumors();
        provider.send(42).unwrap();
        let (mut near, mut far) =
            crate::testing::memory_connection(crate::testing::MEMORY_CAPACITY);
        let (joined, served) = tokio::join!(
            bootstrap.join(&mut near.read, &mut near.write),
            provider.gossip_once(&mut far.read, &mut far.write)
        );
        served.unwrap();
        let Joined::Joined { peer, .. } = joined else {
            panic!("the returned builder must remain usable with an established provider");
        };
        assert_eq!(peer.sync_memory_budget, budget);
        assert_eq!(peer.run_budget, target);
        assert_eq!(peer.codec.limit(), depth);
        assert_eq!(sessions.0.load(Ordering::Relaxed), fail.len() + 1);
        if let Some(stored) = stored {
            assert_eq!(stored.loads.load(Ordering::Relaxed), 1);
            assert_eq!(stored.stores.load(Ordering::Relaxed), 1);
            let bytes = stored.bytes.lock().unwrap();
            let record = crate::bookmark::format::decode(bytes.as_ref().unwrap()).unwrap();
            assert!(record.contains_network(provider.network()));
        }

        // Exercise the retained observer and identity after the bootstrap,
        // including a write that must survive reconciliation in both directions.
        let joined = peer.into_rumors();
        assert_eq!(joined.snapshot(), provider.snapshot());
        joined.send(7).unwrap();
        let (ours, theirs) = tokio::join!(
            joined.gossip_once(&mut near.read, &mut near.write),
            provider.gossip_once(&mut far.read, &mut far.write)
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
    ///
    /// Budgets are drawn on both sides of the minimum, so the retained
    /// budget is the given one raised to
    /// [`MIN_SYNC_MEMORY_BUDGET`](crate::MIN_SYNC_MEMORY_BUDGET).
    #[test]
    fn retries_preserve_configuration_and_storage(
        budget in 0usize..2 * crate::MIN_SYNC_MEMORY_BUDGET,
        target in 0usize..512,
        depth in 1u64..32,
        attempts in prop::collection::vec(any::<bool>(), 0..6),
        configure_after_bookmark in any::<bool>(),
        network in network_seed(),
    ) {
        let expected = (
            budget.max(crate::MIN_SYNC_MEMORY_BUDGET),
            RunBudget::from_bytes(target),
            crate::message::PayloadDepthLimit::new(depth),
        );
        let config = Peer::<u64>::bootstrap()
            .sync_memory_budget(budget)
            .target_message_size(target)
            .payload_depth_limit(depth);
        let plain_sessions = Arc::new(Sessions::default());
        retry_then_join(
            network,
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
            network,
            config,
            &attempts, &bookmarked_sessions, Some(&stored), expected,
        );
    }
}

divided_proptest! {
    #![divisor(SESSION_DIVISOR)]

    /// Default joins inherit the seed's defaults, and oversized targets saturate.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn configuration_defaults_and_saturation(network in network_seed()) {
        let config = Peer::<u64>::bootstrap();
        assert_eq!(config.sync_memory_budget, DEFAULT_SYNC_MEMORY_BUDGET);
        assert_eq!(config.run_budget, RunBudget::default());
        assert_eq!(
            config.payload_depth_limit,
            crate::message::PayloadDepthLimit::default()
        );
        let sessions = Arc::new(Sessions::default());
        retry_then_join(
            network,
            config.observe(sessions.clone()),
            &[],
            &sessions,
            None,
            (
                DEFAULT_SYNC_MEMORY_BUDGET,
                RunBudget::default(),
                crate::message::PayloadDepthLimit::default(),
            ),
        );
        let saturated = Peer::<u64>::bootstrap().target_message_size(usize::MAX);
        assert_eq!(saturated.run_budget, RunBudget::from_bytes(usize::MAX));
        assert_ne!(saturated.run_budget.bytes(), usize::MAX);
    }
}
