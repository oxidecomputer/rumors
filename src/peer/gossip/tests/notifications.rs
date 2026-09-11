//! Identity transfers stay quiet; reconciled tree changes notify observers.

use proptest::prelude::*;

use super::party_of;
use crate::link::memory;
use crate::peer::gossip::ForkGuard;
use crate::testing::run_to_quiescence;
use crate::{Peer, Retire};

proptest! {
    /// Bootstrap and retirement notify the surviving replica only when its
    /// tree changes, while conserving the identity donated and returned.
    #[test]
    fn lifecycle_notifies_only_for_tree_changes(
        initial in 0u64..8,
        added in 0u64..8,
        redact in any::<bool>(),
    ) {
        let provider = Peer::<u64>::seed();
        provider.send_all(0..initial).unwrap();
        let original_party = party_of(&provider);
        let mut changes = provider.inner.subscribe();
        let (served, joined) = run_to_quiescence(async {
            let (mut serving, mut joining) = memory();
            tokio::join!(
                provider.gossip_once(&mut serving),
                Peer::<u64>::bootstrap().join(&mut joining),
            )
        }).expect("bootstrap must complete");
        served.unwrap();
        let crate::Joined::Joined { peer: newcomer } = joined else {
            panic!("bootstrap must succeed");
        };
        prop_assert!(!changes.has_changed().unwrap(), "donating a fork changes no content");

        newcomer.send_all(100..100 + added).unwrap();
        if redact {
            let versions: Vec<_> = provider.snapshot().iter().map(|(v, _)| v.clone()).collect();
            newcomer.redact_all(&versions);
        }
        let before = provider.snapshot();
        changes.borrow_and_update();
        let (served, retired) = run_to_quiescence(async {
            let (mut serving, mut retiring) = memory();
            tokio::join!(provider.gossip_once(&mut serving), newcomer.retire(&mut retiring))
        }).expect("retirement must complete");
        served.unwrap();
        prop_assert!(matches!(retired, Retire::Retired));
        prop_assert_eq!(party_of(&provider), original_party);
        prop_assert_eq!(changes.has_changed().unwrap(), provider.snapshot() != before);
    }

    /// An abandoned bootstrap fork rejoins its owner without changing the tree
    /// or waking content observers.
    #[test]
    fn abandoned_donations_restore_identity_without_notification(
        initial in 0u64..8,
    ) {
        let provider = Peer::<u64>::seed();
        provider.send_all(0..initial).unwrap();
        let original_party = party_of(&provider);
        let before = provider.snapshot();
        let changes = provider.inner.subscribe();
        let mut donation = None;
        provider.inner.send_if_modified(|inner| {
            donation = Some(inner.party.fork());
            false
        });
        drop(ForkGuard { party: donation, recover: provider.inner.clone() });
        prop_assert_eq!(party_of(&provider), original_party);
        prop_assert!(provider.snapshot() == before);
        prop_assert!(!changes.has_changed().unwrap());
    }
}
