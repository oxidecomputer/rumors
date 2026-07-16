//! Golden byte-level snapshots of a single bootstrap session: a stateless
//! newcomer obtaining a fully-formed [`rumors::Peer`] from an established
//! provider that drives `gossip` concurrently.
//!
//! The companion to `gossip_snapshot.rs` for the *bootstrap* leg of the
//! protocol. Each test stages a provider, drives one bootstrap through the
//! recording duplex in [`common::gossip_snapshot::capture_session`], and pins
//! every wire byte. V2 traffic is grouped by logical stream while preserving
//! its exact per-stream order; a representative V1 case pins its strictly
//! alternating timeline. Drift in the preamble, reconciliation, or trailing
//! party hand-off shows up as a diff.
//!
//! Party convention: **A is the provider** — the established peer serving its
//! state through `gossip` — and **B is the bootstrapping newcomer**, running
//! [`Peer::bootstrap`]. The lone exception is [`mutual_bootstrap_bails`], where
//! both sides bootstrap and neither has state to give.
//!
//! As in `gossip_snapshot.rs` the payload is `u64` (a fixed 8 bytes, easy to
//! spot in the hex), except [`string_payload`], which pins how a
//! variable-length value is framed inside the served whole-tree transfer.

mod common;

use borsh::{BorshDeserialize, BorshSerialize};
use rand::SeedableRng;
use rand::rngs::SmallRng;
#[cfg(feature = "protocol-v1")]
use rumors::Protocol;
use rumors::{Peer, Rumors};

use crate::common::gossip_snapshot::capture_session;
#[cfg(feature = "protocol-v1")]
use crate::common::gossip_snapshot::capture_session_v1;

/// A provider seeded from a fixed RNG, so the [`rumors::Network`] id carried in
/// the preamble — and the party region it forks off for the newcomer — are
/// deterministic and these captures stay reproducible. Mirrors
/// `gossip_snapshot::seeded`.
fn seeded<T>() -> Rumors<T> {
    Peer::seed_rng(&mut SmallRng::seed_from_u64(0)).into_rumors()
}

/// Capture one successful bootstrap: `provider` serves its state via `gossip`
/// (party A) while a fresh newcomer runs [`Peer::bootstrap`] (party B) and is
/// expected to be served a successor.
fn capture_bootstrap<T>(provider: Rumors<T>) -> String
where
    T: BorshSerialize + BorshDeserialize + Send + Sync + 'static,
{
    capture_session(
        move |mut r, mut w| async move {
            provider
                .gossip(&mut r, &mut w)
                .await
                .expect("provider gossip");
        },
        move |mut r, mut w| async move {
            Peer::<T>::bootstrap(&mut r, &mut w)
                .await
                .expect("bootstrap handshake")
                .expect("provider served the bootstrap");
        },
    )
}

/// Bootstrap from an *empty* provider: the minimal bootstrap session. After the
/// preamble the provider serves an empty tree and hands off a freshly-forked
/// party — the shortest path by which a newcomer joins a universe.
#[test]
fn empty_provider() {
    let provider: Rumors<u64> = seeded();
    insta::assert_snapshot!(capture_bootstrap(provider));
}

/// Bootstrap from a populated provider. The provider holds three distinct
/// messages (`1`, `2`, `3`); the newcomer receives the whole tree in one
/// descent, so this pins the content-bearing whole-tree frame that the empty
/// case never exercises.
#[test]
fn populated_provider() {
    let provider: Rumors<u64> = seeded();
    provider.batch().send(1).send(2).send(3);
    insta::assert_snapshot!(capture_bootstrap(provider));
}

/// V1 bootstrap retains its original preamble, alternating descent, and
/// trailing party hand-off through the public compatibility entry point.
#[cfg(feature = "protocol-v1")]
#[test]
fn v1_populated_provider() {
    let provider: Rumors<u64> = Peer::seed_rng(&mut SmallRng::seed_from_u64(0))
        .protocol(Protocol::V1)
        .into_rumors();
    provider.batch().send(1).send(2).send(3);
    let capture = capture_session_v1(
        move |mut r, mut w| async move {
            provider
                .gossip(&mut r, &mut w)
                .await
                .expect("V1 provider gossip");
        },
        move |mut r, mut w| async move {
            Peer::<u64>::bootstrap_with_protocol(Protocol::V1, &mut r, &mut w)
                .await
                .expect("V1 bootstrap handshake")
                .expect("V1 provider served the bootstrap");
        },
    );
    insta::assert_snapshot!(capture);
}

/// Bootstrap of a non-primitive, variable-length payload. `u64` borsh-encodes
/// to a fixed 8 bytes; `String` encodes as a length prefix followed by its
/// UTF-8 bytes, so this is the only bootstrap scenario that pins how a
/// variable-length value is framed inside a served leaf.
#[test]
fn string_payload() {
    let provider: Rumors<String> = seeded();
    provider
        .batch()
        .send("hello".to_string())
        .send("world".to_string());
    insta::assert_snapshot!(capture_bootstrap(provider));
}

/// Both sides declare bootstrapping: neither has state to give, so each reads
/// the other's bootstrap intent from the preamble and bails. The capture pins
/// the bytes of that mutual stand-down — the preamble exchange and nothing
/// after it.
#[test]
fn mutual_bootstrap_bails() {
    let capture = capture_session(
        |mut r, mut w| async move {
            let out = Peer::<u64>::bootstrap(&mut r, &mut w)
                .await
                .expect("handshake A");
            assert!(
                out.is_none(),
                "a mutually-bootstrapping peer must bail with None"
            );
        },
        |mut r, mut w| async move {
            let out = Peer::<u64>::bootstrap(&mut r, &mut w)
                .await
                .expect("handshake B");
            assert!(
                out.is_none(),
                "a mutually-bootstrapping peer must bail with None"
            );
        },
    );
    insta::assert_snapshot!(capture);
}
