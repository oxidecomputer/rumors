//! The per-universe [`rumors::Network`] guard: combining operations must
//! refuse peers from a different seed, even when their parties happen to
//! look disjoint. Covers handle inheritance, remote `gossip`, and bootstrap
//! propagation.

mod common;

use rand::SeedableRng;
use rand::rngs::SmallRng;
use rumors::{Error, Known};

use crate::common::wire::block_on;

/// Capacity for the in-memory duplex pipe.
const DUPLEX_BUF: usize = 64 * 1024;

/// A peer seeded deterministically, so two seeds with distinct stream ids get
/// distinct (but reproducible) networks.
fn seeded<T>(stream: u64) -> Known<T> {
    Known::seed_rng(&mut SmallRng::seed_from_u64(stream))
}

/// Every handle on one rumor set belongs to the same universe: a
/// [`Broadcast`](rumors::Broadcast) (and its clones) inherits the
/// originating `Known`'s [`Network`](rumors::Network) unchanged, and the
/// reunited `Known` carries it back out.
#[test]
fn broadcast_preserves_network() {
    let parent = Known::<u64>::seed();
    let network = parent.network();

    let broadcast = parent.broadcast();
    assert_eq!(broadcast.network(), network);
    assert_eq!(broadcast.clone().network(), network);

    let parent = block_on(broadcast.reunite()).expect("the sole reuniter reclaims the Known");
    assert_eq!(parent.network(), network);
}

/// Independent [`seed`](Known::seed)s mint distinct networks — the positive
/// signal that they share no causal history.
#[test]
fn independent_seeds_differ() {
    let a = seeded::<u64>(1);
    let b = seeded::<u64>(2);
    assert_ne!(a.network(), b.network());
}

/// Two peers from different seeds that try to [`gossip`](Known::gossip) are
/// both rejected with [`Error::NetworkMismatch`] at the handshake, before any
/// content crosses the wire.
#[test]
fn gossip_rejects_foreign_network() {
    let mut alice = seeded::<u64>(1);
    let mut bob = seeded::<u64>(2);

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
    let mut provider = Known::<u64>::seed();
    provider.batch().send(1).send(2).send(3);
    let provider_network = provider.network();

    let bootstrapped = block_on(async move {
        let (a_side, b_side) = tokio::io::duplex(DUPLEX_BUF);
        let (mut a_r, mut a_w) = tokio::io::split(a_side);
        let (mut b_r, mut b_w) = tokio::io::split(b_side);
        let (provider_out, bootstrap_out) = tokio::join!(
            provider.gossip(&mut a_r, &mut a_w),
            Known::<u64>::bootstrap(&mut b_r, &mut b_w),
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
