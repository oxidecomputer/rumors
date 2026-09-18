//! Protocol preamble exchange.
//!
//! Drives [`rumors::Rumors::gossip_once`] against a counterparty whose control
//! stream is driven by hand over an in-memory [`rumors::link`] pair. Each
//! rejection test checks the outbound wire spelling, the reported error, and
//! that the local set remains unchanged. The preamble is one self-described
//! CBOR item of exactly 30 bytes with no redundant length:
//! `55799(["rumors", version: uint, network: bstr(16), intent: uint])`.
//! The layout is transcribed here so the test remains independent of the
//! encoder and decoder it checks.
//! Network mismatch rejection rides the same preamble but needs
//! a real peer in a different universe, so it is exercised separately in
//! `tests/network.rs`.

use rumors_testkit::common;

use rumors::error::Mismatch;
use rumors::{Error, Gossiped, Peer, Protocol, Rumors};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::common::wire::{assert_control_drained, bootstrap_fork_async};

/// Length of the complete V2 preamble item: the self-described tag (3),
/// the four-item array head (1), the text `"rumors"` (7), the version
/// uint (1), the network byte string (1 + 16), and the intent uint (1).
const PREAMBLE_LEN: usize = 30;

/// The V2 preamble's leading bytes: tag 55799, array(4), text "rumors".
const V2_OPENING: [u8; 11] = [
    0xd9, 0xd9, 0xf7, 0x84, 0x66, b'r', b'u', b'm', b'o', b'r', b's',
];

/// Intent value for a peer that participates and remains.
const INTENT_REMAIN: u8 = 0;

/// Intent value for a peer that donates its identity after reconciliation.
const INTENT_RETIRE: u8 = 1;

/// The wire version carried by this dialect's preamble.
const WIRE_VERSION: u8 = 2;

/// Assemble a preamble item by hand, matching the layout in the module doc.
///
/// `version` and `intent` must be below 24 so each spells as a one-byte uint.
fn preamble(opening: [u8; 11], version: u8, network: [u8; 16], intent: u8) -> [u8; PREAMBLE_LEN] {
    assert!(version < 24 && intent < 24, "one-byte uint items only");
    let mut p = [0u8; PREAMBLE_LEN];
    p[..11].copy_from_slice(&opening);
    p[11] = version;
    p[12] = 0x50;
    p[13..29].copy_from_slice(&network);
    p[29] = intent;
    p
}

/// Run one gossip attempt against a hand-written preamble reply.
///
/// The fake peer checks every stable outbound byte: the opening, version,
/// network byte-string head, and intent. Its network value varies per seed,
/// so only that field is omitted. A rejected reply must leave the local set
/// byte-for-byte unchanged.
async fn gossip_against(reply: &[u8]) -> Result<Gossiped, Error> {
    let alice: Rumors<String> = Peer::seed().sync_window_floor().into_rumors();
    let before = alice.snapshot();
    let (mut a_link, b) = rumors::link::memory();
    let mut b_read = b.control_read;
    let mut b_write = b.control_write;

    let fake_peer = async move {
        let mut got = [0u8; PREAMBLE_LEN];
        b_read.read_exact(&mut got).await.expect("fake peer read");
        let expected = preamble(V2_OPENING, WIRE_VERSION, [0; 16], INTENT_REMAIN);
        assert_eq!(&got[..13], &expected[..13], "outbound preamble head");
        assert_eq!(got[29], INTENT_REMAIN, "outbound gossip intent");
        b_write.write_all(reply).await.expect("fake peer write");
    };

    let (result, ()) = tokio::join!(alice.gossip_once(&mut a_link), fake_peer);
    assert_eq!(alice.snapshot(), before, "rejection changed the local set");
    result
}

/// Two well-behaved peers in the same universe complete the preamble and
/// proceed to a (trivially empty) gossip session.
#[pollster::test]
async fn handshake_roundtrip_succeeds() {
    // Same universe: `bob` is a party-disjoint fork of `alice`, so their
    // networks match.
    let alice: Rumors<String> = Peer::seed().sync_window_floor().into_rumors();
    let bob = bootstrap_fork_async(&alice).await;

    let (mut a_link, mut b_link) = rumors::link::memory();

    let (alice_out, bob_out) =
        tokio::join!(alice.gossip_once(&mut a_link), bob.gossip_once(&mut b_link));

    alice_out.expect("alice gossip");
    bob_out.expect("bob gossip");
    assert_control_drained(a_link, b_link);
}

/// A peer that opens with the wrong bytes is rejected with
/// [`Error::Protocol`] before reconciliation.
#[pollster::test]
async fn unrecognized_preamble_is_a_violation() {
    let bad_opening = *b"NOPENOPENOP";
    let reply = preamble(bad_opening, WIRE_VERSION, [0xAB; 16], INTENT_REMAIN);
    match gossip_against(&reply).await {
        Err(Error::Protocol(error)) => {
            assert_eq!(error.context.phase, rumors::error::Phase::Preamble);
        }
        other => panic!("expected an unrecognized-preamble violation, got {other:?}"),
    }
}

/// A peer with the correct opening but an unsupported version is rejected
/// with [`Mismatch::Protocol`].
#[pollster::test]
async fn version_mismatch_surfaces_error() {
    // Pick a version we definitely don't speak yet (kept below 24 so the
    // item's width matches the fixed layout).
    let bogus_version: u8 = 7;
    let reply = preamble(V2_OPENING, bogus_version, [0xAB; 16], INTENT_REMAIN);
    match gossip_against(&reply).await {
        Err(Error::Mismatch(Mismatch::Protocol {
            local_protocol,
            remote_version,
            ..
        })) => {
            assert_eq!(local_protocol, Protocol::V2);
            assert_eq!(remote_version, u64::from(bogus_version));
        }
        other => panic!("expected protocol mismatch, got {other:?}"),
    }
}

/// A peer whose intent is neither 0 (remain) nor 1 (retire) is rejected
/// with [`Error::Protocol`]: the intent is peer-supplied and must be
/// validated rather than assumed.
#[pollster::test]
async fn invalid_intent_surfaces_error() {
    let bogus_intent: u8 = 2;
    let reply = preamble(V2_OPENING, WIRE_VERSION, [0xAB; 16], bogus_intent);
    match gossip_against(&reply).await {
        Err(Error::Protocol(error)) => {
            assert_eq!(error.context.phase, rumors::error::Phase::Preamble);
        }
        other => panic!("expected invalid-intent violation, got {other:?}"),
    }
}

/// Closing mid-preamble reports a transport EOF in the preamble phase.
#[pollster::test]
async fn truncated_handshake_surfaces_typed_truncation() {
    let reply = preamble(V2_OPENING, WIRE_VERSION, [0xAB; 16], INTENT_REMAIN);
    match gossip_against(&reply[..6]).await {
        Err(Error::Transport(error)) => {
            assert_eq!(error.context.phase, rumors::error::Phase::Preamble);
            assert_eq!(error.source.kind(), std::io::ErrorKind::UnexpectedEof);
        }
        other => panic!("expected preamble transport failure, got {other:?}"),
    }
}

/// A peer whose preamble opens correctly but spells a field wrong is
/// rejected as [`Error::Protocol`] in the preamble phase.
///
/// Never accepted, and never blamed on the transport.
#[pollster::test]
async fn malformed_preamble_surfaces_typed_defect() {
    // The network item's head spells a 16-byte text string (0x70) where
    // the wire requires a 16-byte byte string (0x50).
    let mut reply = preamble(V2_OPENING, WIRE_VERSION, [0xAB; 16], INTENT_REMAIN);
    reply[12] = 0x70;
    match gossip_against(&reply).await {
        Err(Error::Protocol(error)) => {
            assert_eq!(error.context.phase, rumors::error::Phase::Preamble);
        }
        other => panic!("expected preamble violation, got {other:?}"),
    }
}

/// A bootstrap network paired with a retiring intent is rejected because a
/// peer cannot receive and donate an identity in the same session.
#[pollster::test]
async fn bootstrap_retire_conflict_is_a_violation() {
    let reply = preamble(V2_OPENING, WIRE_VERSION, [0; 16], INTENT_RETIRE);
    match gossip_against(&reply).await {
        Err(Error::Protocol(error)) => {
            assert_eq!(error.context.phase, rumors::error::Phase::Preamble);
        }
        other => panic!("expected a bootstrap-retire violation, got {other:?}"),
    }
}

/// Check ownership of the first supply reply after the greeting.
#[path = "handshake/opening_supply.rs"]
mod opening_supply;

/// Check that gossip preserves messages sent after a bootstrap snapshot.
#[path = "handshake/stale_floor.rs"]
mod stale_floor;
