//! The per-universe [`rumors::Network`] guard: combining operations must
//! refuse peers from a different seed, even when their parties happen to
//! look disjoint.
//!
//! Covers handle inheritance, bootstrap propagation, gossip, and retirement.

use rumors_testkit::common;

use rand::SeedableRng;
use rand_chacha::ChaChaRng;
use rumors::error::Mismatch;
use rumors::testing::{IoPlan, IoSide, wrap_link};
use rumors::{Error, Peer, Retire};

use crate::common::wire::{assert_control_drained, block_on};

/// A peer seeded deterministically, so two seeds with distinct stream ids get
/// distinct (but reproducible) networks.
fn seeded<T: serde::Serialize + serde::de::DeserializeOwned + Eq + Send + Sync + 'static>(
    stream: u64,
) -> Peer<T> {
    Peer::seed_rng(&mut ChaChaRng::seed_from_u64(stream)).sync_window_floor()
}

/// Every handle on one rumor set belongs to the same universe.
///
/// A [`Rumors`](rumors::Rumors) (and its clones) inherits the originating
/// [`Peer`]'s [`Network`](rumors::Network) unchanged, and the reclaimed
/// `Peer` carries it back out.
#[test]
fn rumors_preserves_network() {
    let parent = Peer::<u64>::seed().sync_window_floor();
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
#[test]
fn independent_seeds_differ() {
    let a = seeded::<u64>(1);
    let b = seeded::<u64>(2);
    assert_ne!(a.network(), b.network());
}

/// Gossip rejects a foreign network before opening a reconciliation stream.
///
/// Both populated replicas remain unchanged, so the test distinguishes an
/// early handshake rejection from reconciliation followed by an error.
#[test]
fn gossip_rejects_foreign_network() {
    let alice = seeded::<u64>(1).into_rumors();
    let bob = seeded::<u64>(2).into_rumors();
    alice.send(1).unwrap();
    bob.send(2).unwrap();
    let alice_hash = alice.snapshot().hash();
    let bob_hash = bob.snapshot().hash();

    let (alice_out, bob_out, alice_report, bob_report) = block_on(async {
        let (a_link, b_link) = rumors::link::memory();
        let (mut a_link, alice_report) = wrap_link(IoSide::Left, IoPlan::default(), a_link);
        let (mut b_link, bob_report) = wrap_link(IoSide::Right, IoPlan::default(), b_link);
        let (alice_out, bob_out) =
            tokio::join!(alice.gossip_once(&mut a_link), bob.gossip_once(&mut b_link));
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
        assert_eq!(
            report.connects + report.accepts,
            0,
            "a network mismatch must fail before reconciliation opens a stream",
        );
    }
}

/// Foreign-network retirement returns the donor with its content and party intact.
#[test]
fn retire_rejects_foreign_network_without_consuming_the_peer() {
    let donor = seeded::<u64>(1).into_rumors();
    donor.send(1).unwrap();
    let donor_hash = donor.snapshot().hash();
    let donor_party = donor.dangerously_alias_party();
    let donor = block_on(donor.try_into_peer()).expect("the sole handle reclaims the peer");

    let absorber = seeded::<u64>(2).into_rumors();
    absorber.send(2).unwrap();
    let absorber_hash = absorber.snapshot().hash();

    let (retired, served) = block_on(async {
        let (mut donor_link, mut absorber_link) = rumors::link::memory();
        tokio::join!(
            donor.retire(&mut donor_link),
            absorber.gossip_once(&mut absorber_link),
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
#[test]
fn bootstrap_adopts_provider_network() {
    let provider = Peer::<u64>::seed().sync_window_floor().into_rumors();
    provider.send_all([1, 2, 3]).unwrap();
    let provider_network = provider.network();

    let bootstrapped = block_on(async move {
        let (mut a_link, mut b_link) = rumors::link::memory();
        let (provider_out, bootstrap_out) = tokio::join!(
            provider.gossip_once(&mut a_link),
            Peer::<u64>::bootstrap().join(&mut b_link),
        );
        provider_out.expect("provider gossip");
        let joined = match bootstrap_out {
            rumors::Joined::Joined { peer } => peer,
            _ => panic!("provider served the bootstrap"),
        };
        assert_control_drained(a_link, b_link);
        joined
    });

    assert_eq!(
        bootstrapped.network(),
        provider_network,
        "bootstrapped peer must join the provider's network",
    );
}
