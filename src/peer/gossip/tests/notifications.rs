//! Identity transfers stay quiet; reconciled tree changes notify observers.

use proptest::prelude::*;

use super::party_of;
use crate::peer::Inner;
use crate::peer::gossip::ForkGuard;
use crate::testing::property::network_seed;
use crate::testing::run_to_quiescence;
use crate::testing::{MEMORY_CAPACITY, memory_connection};
use crate::{Connection, Peer, Retire};

proptest! {
    /// Bootstrap and retirement notify the surviving replica only when its
    /// tree changes, while conserving the identity donated and returned.
    #[test]
    fn lifecycle_notifies_only_for_tree_changes(
        initial in 0u64..8,
        added in 0u64..8,
        redact in any::<bool>(),
        network in network_seed(),
    ) {
        let provider = network.peer::<u64>();
        provider.send_all(0..initial).unwrap();
        let original_party = party_of(&provider);
        let mut changes = provider.inner.subscribe();
        let (served, joined) = run_to_quiescence(async {
            let (serving, joining) = memory_connection(MEMORY_CAPACITY);
            tokio::join!(
                provider.gossip_once(serving.read, serving.write),
                Peer::<u64>::bootstrap().join(joining.read, joining.write),
            )
        }).expect("bootstrap must complete");
        served.unwrap();
        let crate::Joined::Joined { peer: newcomer, connection: crate::Connection::Reusable { .. } } = joined else {
            panic!("bootstrap must succeed");
        };
        prop_assert!(!changes.has_changed().unwrap(), "donating a fork changes no content");

        newcomer.send_all(100..100 + added).unwrap();
        if redact {
            let versions: Vec<_> = provider.snapshot().versions().cloned().collect();
            newcomer.redact_all(&versions);
        }
        let before = provider.snapshot();
        changes.borrow_and_update();
        let (served, retired) = run_to_quiescence(async {
            let (serving, retiring) = memory_connection(MEMORY_CAPACITY);
            tokio::join!(provider.gossip_once(serving.read, serving.write), newcomer.retire(retiring.read, retiring.write))
        }).expect("retirement must complete");
        served.unwrap();
        prop_assert!(matches!(retired, Retire::Retired { connection: Connection::Reusable { .. } }), "retirement completes");
        prop_assert_eq!(party_of(&provider), original_party);
        prop_assert_eq!(changes.has_changed().unwrap(), provider.snapshot() != before);
    }

    /// An abandoned bootstrap fork rejoins its owner without changing the tree
    /// or waking content observers.
    #[test]
    fn abandoned_donations_restore_identity_without_notification(
        initial in 0u64..8,
        network in network_seed(),
    ) {
        let provider = network.peer::<u64>();
        provider.send_all(0..initial).unwrap();
        let original_party = party_of(&provider);
        let before = provider.snapshot();
        let changes = provider.inner.subscribe();
        let mut donation = None;
        Inner::update_party(&provider.inner, |inner| {
            donation = Some(inner.reserve());
        });
        drop(ForkGuard { fork: donation, recover: &provider.inner });
        prop_assert_eq!(party_of(&provider), original_party);
        prop_assert!(provider.snapshot() == before);
        prop_assert!(!changes.has_changed().unwrap());
    }
}
