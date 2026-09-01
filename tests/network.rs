//! The per-universe [`rumors::Network`] guard: combining operations must
//! refuse peers from a different seed, even when their parties happen to
//! look disjoint.
//!
//! Covers handle inheritance, remote `gossip`, and bootstrap
//! propagation.

mod common;

use rand::SeedableRng;
use rand::rngs::SmallRng;
use rumors::{Error, Peer};

use crate::common::wire::{assert_control_drained, block_on};

/// A peer seeded deterministically, so two seeds with distinct stream ids get
/// distinct (but reproducible) networks.
fn seeded<T: serde::Serialize + serde::de::DeserializeOwned + Eq + Send + Sync + 'static>(
    stream: u64,
) -> Peer<T> {
    Peer::seed_rng(&mut SmallRng::seed_from_u64(stream)).sync_window_floor()
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

/// Two peers from different seeds that try to [`gossip`](rumors::Rumors::gossip)
/// are both rejected with [`Error::NetworkMismatch`] at the handshake, before
/// any content crosses the wire.
#[test]
fn gossip_rejects_foreign_network() {
    let alice = seeded::<u64>(1).into_rumors();
    let bob = seeded::<u64>(2).into_rumors();

    let (alice_out, bob_out) = block_on(async {
        let (mut a_link, mut b_link) = rumors::link::memory();
        tokio::join!(alice.gossip(&mut a_link), bob.gossip(&mut b_link))
    });

    assert!(
        matches!(alice_out, Err(Error::NetworkMismatch { .. })),
        "expected NetworkMismatch, got {alice_out:?}",
    );
    assert!(
        matches!(bob_out, Err(Error::NetworkMismatch { .. })),
        "expected NetworkMismatch, got {bob_out:?}",
    );
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
            provider.gossip(&mut a_link),
            Peer::<u64>::bootstrap().join(&mut b_link),
        );
        provider_out.expect("provider gossip");
        let joined = bootstrap_out
            .expect("bootstrap handshake")
            .expect("provider served the bootstrap");
        assert_control_drained(a_link, b_link);
        joined
    });

    assert_eq!(
        bootstrapped.network(),
        provider_network,
        "bootstrapped peer must join the provider's network",
    );
}
