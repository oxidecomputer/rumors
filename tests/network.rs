//! The per-universe [`rumors::Network`] guard: combining operations must
//! refuse peers from a different seed, even when their parties happen to
//! look disjoint.
//!
//! Covers handle inheritance, bootstrap propagation, gossip, and retirement.

use rumors_testkit::common;

use rumors::error::Mismatch;
use rumors::testing::{IoPlan, IoSide, wrap_io};
use rumors::{Error, Peer, Retire};

use crate::common::wire::{assert_connection_drained, block_on};
use rumors_testkit::common::property::{LOCAL_DIVISOR, SESSION_DIVISOR};
use rumors_testkit::common::seed::{NetworkSeed, distinct_network_seeds, network_seed};
use rumors_testkit::divided_proptest;

/// A peer founding the network `network` names. The seed's stream supplies
/// the network's secret, and with it its identifier: distinct seeds give
/// distinct networks, and the tests rely on nothing else about the
/// identifiers.
fn seeded<T: serde::Serialize + serde::de::DeserializeOwned + Eq + Send + Sync + 'static>(
    network: NetworkSeed,
) -> Peer<T> {
    network.peer().sync_memory_budget(0)
}

divided_proptest! {
    #![divisor(LOCAL_DIVISOR)]

    /// Every handle on one rumor set belongs to the same universe.
    ///
    /// A [`Rumors`](rumors::Rumors) (and its clones) inherits the originating
    /// [`Peer`]'s [`Network`](rumors::Network) unchanged, and the reclaimed
    /// `Peer` carries it back out.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn rumors_preserves_network(network in network_seed()) {
        let parent = network.peer::<u64>().sync_memory_budget(0);
        let network = parent.network();

        let rumors = parent.into_rumors();
        assert_eq!(rumors.network(), network);
        assert_eq!(rumors.clone().network(), network);
        assert_eq!(
            rumors.snapshot().network(),
            network,
            "a snapshot carries its set's universe"
        );

        let parent = block_on(rumors.try_into_peer()).expect("the sole reuniter reclaims the Peer");
        assert_eq!(parent.network(), network);
    }

    /// Independent [`seed`](Peer::seed)s create distinct networks — the positive
    /// signal that they share no causal history.
    ///
    /// The claim holds for every pair of distinct network seeds.
    #[test]
    fn independent_seeds_differ(
        (network, other_network) in distinct_network_seeds(),
    ) {
        let a = seeded::<u64>(network);
        let b = seeded::<u64>(other_network);
        assert_ne!(a.network(), b.network());
    }
}

/// Items each end flushes before its first frame: the preamble and the
/// greeting, one flush each.
const HANDSHAKE_FLUSHES: usize = 2;

divided_proptest! {
    #![divisor(SESSION_DIVISOR)]

    /// Gossip rejects a foreign network before sending a reconciliation frame.
    ///
    /// Both populated replicas remain unchanged, so the test distinguishes an
    /// early handshake rejection from reconciliation followed by an error.
    ///
    /// The claim holds for every pair of distinct network seeds.
    #[test]
    fn gossip_rejects_foreign_network(
        (network, other_network) in distinct_network_seeds(),
    ) {
        let alice = seeded::<u64>(network).into_rumors();
        let bob = seeded::<u64>(other_network).into_rumors();
        alice.send(1).unwrap();
        bob.send(2).unwrap();
        let alice_hash = alice.snapshot().hash();
        let bob_hash = bob.snapshot().hash();

        let (alice_out, bob_out, alice_report, bob_report) = block_on(async {
            let (a_end, b_end) = rumors::testing::memory_connection(rumors::testing::MEMORY_CAPACITY);
            let (a_read, a_write, alice_report) = wrap_io(
                IoSide::Left,
                IoPlan::default(),
                a_end.read.into_inner(),
                a_end.write,
            );
            let (b_read, b_write, bob_report) = wrap_io(
                IoSide::Right,
                IoPlan::default(),
                b_end.read.into_inner(),
                b_end.write,
            );
            let (alice_out, bob_out) = tokio::join!(
                alice.gossip_once(a_read, a_write),
                bob.gossip_once(b_read, b_write)
            );
            (alice_out, bob_out, alice_report, bob_report)
        });

        for (outcome, ours, theirs) in [
            (alice_out, alice.network(), bob.network()),
            (bob_out, bob.network(), alice.network()),
        ] {
            let Err(Error::Mismatch(Mismatch::Network {
                local_network,
                remote_network,
                ..
            })) = outcome
            else {
                panic!("expected a network mismatch, got {outcome:?}");
            };
            assert_eq!(local_network, ours);
            assert_eq!(remote_network, theirs);
        }
        assert_eq!(alice.snapshot().hash(), alice_hash);
        assert_eq!(bob.snapshot().hash(), bob_hash);
        for report in [alice_report.snapshot(), bob_report.snapshot()] {
            assert!(
                report.flushes <= HANDSHAKE_FLUSHES,
                "a network mismatch must fail before reconciliation flushes a frame: {} flushes",
                report.flushes,
            );
        }
    }

    /// Foreign-network retirement returns the donor with its content and party intact.
    ///
    /// The claim holds for every pair of distinct network seeds.
    #[test]
    fn retire_rejects_foreign_network_without_consuming_the_peer(
        (network, other_network) in distinct_network_seeds(),
    ) {
        let donor = seeded::<u64>(network).into_rumors();
        donor.send(1).unwrap();
        let donor_hash = donor.snapshot().hash();
        let donor_party = donor.dangerously_alias_party();
        let donor = block_on(donor.try_into_peer()).expect("the sole handle reclaims the peer");

        let absorber = seeded::<u64>(other_network).into_rumors();
        absorber.send(2).unwrap();
        let absorber_hash = absorber.snapshot().hash();

        let (retired, served) = block_on(async {
            let (donor_end, absorber_end) =
                rumors::testing::memory_connection(rumors::testing::MEMORY_CAPACITY);
            tokio::join!(
                donor.retire(donor_end.read, donor_end.write),
                absorber.gossip_once(absorber_end.read, absorber_end.write),
            )
        });

        let Retire::Recovered { peer, error } = retired else {
            panic!("a network mismatch must return the donor, got {retired:?}");
        };
        assert!(matches!(error, Error::Mismatch(Mismatch::Network { .. })));
        assert!(matches!(
            served,
            Err(Error::Mismatch(Mismatch::Network { .. }))
        ));

        let donor = peer.into_rumors();
        assert_eq!(donor.snapshot().hash(), donor_hash);
        assert_eq!(donor.dangerously_alias_party(), donor_party);
        assert_eq!(absorber.snapshot().hash(), absorber_hash);
    }

    /// A bootstrapped peer adopts the provider's network, so it lands in exactly
    /// the universe it was served from and can subsequently combine with it.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn bootstrap_adopts_provider_network(network in network_seed()) {
        let provider = network.peer::<u64>().sync_memory_budget(0).into_rumors();
        provider.send_all([1, 2, 3]).unwrap();
        let provider_network = provider.network();

        let bootstrapped = block_on(async move {
            let (mut a_end, mut b_end) =
                rumors::testing::memory_connection(rumors::testing::MEMORY_CAPACITY);
            let (provider_out, bootstrap_out) = tokio::join!(
                provider.gossip_once(&mut a_end.read, &mut a_end.write),
                Peer::<u64>::bootstrap().join(&mut b_end.read, &mut b_end.write),
            );
            provider_out.expect("provider gossip");
            let joined = match bootstrap_out {
                rumors::Joined::Joined { peer, connection: rumors::Connection::Reusable { .. } } => peer,
                _ => panic!("provider served the bootstrap"),
            };
            assert_connection_drained(a_end, b_end);
            joined
        });

        assert_eq!(
            bootstrapped.network(),
            provider_network,
            "bootstrapped peer must join the provider's network",
        );
    }
}
