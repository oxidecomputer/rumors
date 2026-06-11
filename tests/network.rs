//! The per-universe [`rumors::Network`] guard: combining operations must
//! refuse peers from a different seed, even when their parties happen to
//! look disjoint. Covers handle inheritance, remote `gossip`, and bootstrap
//! propagation.

mod common;

use rand::SeedableRng;
use rand::rngs::SmallRng;
use rumors::{Error, Peer};

use crate::common::wire::block_on;

/// Capacity for the in-memory duplex pipe.
const DUPLEX_BUF: usize = 64 * 1024;

/// A peer seeded deterministically, so two seeds with distinct stream ids get
/// distinct (but reproducible) networks.
fn seeded<T>(stream: u64) -> Peer<T> {
    Peer::seed_rng(&mut SmallRng::seed_from_u64(stream))
}

/// Every handle on one rumor set belongs to the same universe: a
/// [`Rumors`](rumors::Rumors) (and its clones) inherits the originating
/// [`Peer`]'s [`Network`](rumors::Network) unchanged, and the reclaimed
/// `Peer` carries it back out.
#[test]
fn rumors_preserves_network() {
    let parent = Peer::<u64>::seed();
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

/// Independent [`seed`](Peer::seed)s mint distinct networks — the positive
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
        let (a_side, b_side) = tokio::io::duplex(DUPLEX_BUF);
        let (mut a_r, mut a_w) = tokio::io::split(a_side);
        let (mut b_r, mut b_w) = tokio::io::split(b_side);
        tokio::join!(
            alice.gossip(&mut a_r, &mut a_w),
            bob.gossip(&mut b_r, &mut b_w),
        )
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
    let provider = Peer::<u64>::seed().into_rumors();
    provider.batch().send(1).send(2).send(3);
    let provider_network = provider.network();

    let bootstrapped = block_on(async move {
        let (a_side, b_side) = tokio::io::duplex(DUPLEX_BUF);
        let (mut a_r, mut a_w) = tokio::io::split(a_side);
        let (mut b_r, mut b_w) = tokio::io::split(b_side);
        let (provider_out, bootstrap_out) = tokio::join!(
            provider.gossip(&mut a_r, &mut a_w),
            Peer::<u64>::bootstrap(&mut b_r, &mut b_w),
        );
        provider_out.expect("provider gossip");
        bootstrap_out
            .expect("bootstrap handshake")
            .expect("provider served the bootstrap")
    });

    assert_eq!(
        bootstrapped.network(),
        provider_network,
        "bootstrapped peer must join the provider's network",
    );
}
