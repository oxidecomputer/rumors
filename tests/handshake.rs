//! Protocol preamble exchange (`mirror::remote::preamble`).
//!
//! Drives [`rumors::Rumors::gossip`] against a counterparty whose control
//! halves are driven by hand over an in-memory [`rumors::link`] pair,
//! asserting that a mismatched magic, version, or intent surfaces as the
//! typed error variant rather than corrupting the local rumor set. The V2
//! preamble is one self-described CBOR item of exactly 30 bytes with no
//! redundant length:
//! `55799(["rumors", version: uint, network: bstr(16), intent: uint])`.
//! The layout is transcribed here by hand, deliberately: this suite is an
//! independent oracle of the documented wire spelling, so it must not
//! derive the bytes from the code under test.
//! Network mismatch rejection rides the same preamble but needs
//! a real peer in a different universe, so it is exercised separately in
//! `tests/network.rs`.

mod common;

use rumors::{Error, Peer, Protocol, Rumors};
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

/// Assemble a V2 preamble item by hand, matching the layout in the module
/// doc, with caller-selected opening bytes.
///
/// The network bytes are arbitrary: every scenario below fails (or
/// completes) before the network would be consulted. `version` and
/// `intent` must be below 24 so each spells as a one-byte uint item.
fn preamble(opening: [u8; 11], version: u8, intent: u8) -> [u8; PREAMBLE_LEN] {
    assert!(version < 24 && intent < 24, "one-byte uint items only");
    let mut p = [0u8; PREAMBLE_LEN];
    p[..11].copy_from_slice(&opening);
    p[11] = version;
    p[12] = 0x50;
    p[13..29].copy_from_slice(&[0xAB; 16]);
    p[29] = intent;
    p
}

/// The fixed markers and selectable versions match the hand-encoded
/// layouts: the legacy magic opens V1 preambles, the self-described CBOR
/// opening starts V2 ones.
#[test]
fn protocol_constants_match_spec() {
    #[cfg(feature = "protocol-v1")]
    {
        assert_eq!(rumors::PROTOCOL_MAGIC, *b"RUMORS");
        assert_eq!(Protocol::V1 as u16, 1);
    }
    assert_eq!(Protocol::V2 as u16, 2);
    assert_eq!(&V2_OPENING[..3], &[0xd9, 0xd9, 0xf7]);
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

    let (alice_out, bob_out) = tokio::join!(alice.gossip(&mut a_link), bob.gossip(&mut b_link));

    alice_out.expect("alice gossip");
    bob_out.expect("bob gossip");
    assert_control_drained(a_link, b_link);
}

/// A peer that opens with the wrong bytes is rejected with
/// [`Error::MagicMismatch`] before any framed traffic.
#[pollster::test]
async fn magic_mismatch_surfaces_error() {
    let (mut a_link, b) = rumors::link::memory();
    let b = b.into_parts();
    let mut b_r = b.control_read;
    let mut b_w = b.control_write;

    let bad_opening = *b"NOPENOPENOP";
    let fake_peer = async move {
        // Drain alice's preamble (so her write_all completes) and reply with a
        // non-rumors one.
        let mut got = [0u8; PREAMBLE_LEN];
        b_r.read_exact(&mut got).await.expect("fake peer read");
        let reply = preamble(bad_opening, Protocol::V2 as u8, INTENT_REMAIN);
        b_w.write_all(&reply).await.expect("fake peer write");
    };

    let alice: Rumors<String> = Peer::seed().sync_window_floor().into_rumors();
    let alice_fut = alice.gossip(&mut a_link);

    let (alice_result, ()) = tokio::join!(alice_fut, fake_peer);
    match alice_result {
        Err(Error::MagicMismatch { remote_magic }) => {
            assert_eq!(remote_magic, *b"NOPENO");
        }
        other => panic!("expected MagicMismatch, got {other:?}"),
    }
}

/// A peer with the correct opening but an unsupported version is rejected
/// with [`Error::VersionMismatch`].
#[pollster::test]
async fn version_mismatch_surfaces_error() {
    let (mut a_link, b) = rumors::link::memory();
    let b = b.into_parts();
    let mut b_r = b.control_read;
    let mut b_w = b.control_write;

    // Pick a version we definitely don't speak yet (kept below 24 so the
    // item's width matches the fixed layout).
    let bogus_version: u8 = 7;
    let fake_peer = async move {
        let mut got = [0u8; PREAMBLE_LEN];
        b_r.read_exact(&mut got).await.expect("fake peer read");
        // Correct opening, bogus version: the version check fires on the
        // preamble item before the network or intent are interpreted.
        let reply = preamble(V2_OPENING, bogus_version, INTENT_REMAIN);
        b_w.write_all(&reply).await.expect("fake peer write");
    };

    let alice: Rumors<String> = Peer::seed().sync_window_floor().into_rumors();
    let alice_fut = alice.gossip(&mut a_link);

    let (alice_result, ()) = tokio::join!(alice_fut, fake_peer);
    match alice_result {
        Err(Error::VersionMismatch {
            local_protocol,
            remote_version,
        }) => {
            assert_eq!(local_protocol, Protocol::V2);
            assert_eq!(remote_version, u64::from(bogus_version));
        }
        other => panic!("expected VersionMismatch, got {other:?}"),
    }
}

/// Selecting V1 changes the preamble dialect itself; a V2 counterparty is
/// diagnosed as a version mismatch before either implementation consumes
/// protocol-specific bytes, in both directions of the skew.
#[cfg(feature = "protocol-v1")]
#[pollster::test]
async fn selected_protocols_must_match() {
    let (mut a_link, b) = rumors::link::memory();
    let b = b.into_parts();
    let mut b_r = b.control_read;
    let mut b_w = b.control_write;

    const LEGACY_PREAMBLE_LEN: usize = 25;
    let fake_v2 = async move {
        let mut got = [0u8; LEGACY_PREAMBLE_LEN];
        b_r.read_exact(&mut got).await.expect("fake peer read");
        let reply = preamble(V2_OPENING, Protocol::V2 as u8, INTENT_REMAIN);
        b_w.write_all(&reply).await.expect("fake peer write");
    };
    let v1 = Peer::<String>::seed()
        .sync_window_floor()
        .protocol(Protocol::V1)
        .into_rumors();

    let (result, ()) = tokio::join!(v1.gossip(&mut a_link), fake_v2);
    assert!(matches!(
        result,
        Err(Error::VersionMismatch {
            local_protocol: Protocol::V1,
            remote_version,
        }) if remote_version == Protocol::V2 as u64
    ));
}

/// A peer whose intent is neither 0 (remain) nor 1 (retire) is rejected
/// with [`Error::IntentInvalid`]: the intent is peer-supplied and must be
/// validated rather than assumed.
#[pollster::test]
async fn invalid_intent_surfaces_error() {
    let (mut a_link, b) = rumors::link::memory();
    let b = b.into_parts();
    let mut b_r = b.control_read;
    let mut b_w = b.control_write;

    let bogus_intent: u8 = 2;
    let fake_peer = async move {
        let mut got = [0u8; PREAMBLE_LEN];
        b_r.read_exact(&mut got).await.expect("fake peer read");
        let reply = preamble(V2_OPENING, Protocol::V2 as u8, bogus_intent);
        b_w.write_all(&reply).await.expect("fake peer write");
    };

    let alice: Rumors<String> = Peer::seed().sync_window_floor().into_rumors();
    let alice_fut = alice.gossip(&mut a_link);

    let (alice_result, ()) = tokio::join!(alice_fut, fake_peer);
    match alice_result {
        Err(Error::IntentInvalid { byte }) => {
            assert_eq!(byte, bogus_intent);
        }
        other => panic!("expected IntentInvalid, got {other:?}"),
    }
}

/// A peer that closes the connection mid-preamble surfaces as
/// [`Error::PreambleTruncated`] carrying the exact byte counts of the
/// cut, not a bare I/O error and not a malformed-preamble error.
#[pollster::test]
async fn truncated_handshake_surfaces_typed_truncation() {
    let (mut a_link, b) = rumors::link::memory();
    let b = b.into_parts();
    let mut b_r = b.control_read;
    let mut b_w = b.control_write;

    let fake_peer = async move {
        let mut got = [0u8; PREAMBLE_LEN];
        b_r.read_exact(&mut got).await.expect("fake peer read");
        // Write only the first six bytes, then drop the write half to signal
        // EOF before the fixed preamble is complete.
        let partial = preamble(V2_OPENING, Protocol::V2 as u8, INTENT_REMAIN);
        b_w.write_all(&partial[..6]).await.expect("partial write");
        drop(b_w);
    };

    let alice: Rumors<String> = Peer::seed().sync_window_floor().into_rumors();
    let alice_fut = alice.gossip(&mut a_link);

    let (alice_result, ()) = tokio::join!(alice_fut, fake_peer);
    match alice_result {
        Err(Error::PreambleTruncated { received, expected }) => {
            assert_eq!(received, 6, "the six delivered bytes are counted");
            assert_eq!(expected, PREAMBLE_LEN, "the full dialect width is named");
        }
        other => panic!("expected PreambleTruncated, got {other:?}"),
    }
}

/// A peer whose preamble opens correctly but spells a field wrong is
/// rejected as [`Error::PreambleMalformed`] with the defect naming the
/// field.
///
/// Never accepted, and never blamed on the transport.
#[pollster::test]
async fn malformed_preamble_surfaces_typed_defect() {
    let (mut a_link, b) = rumors::link::memory();
    let b = b.into_parts();
    let mut b_r = b.control_read;
    let mut b_w = b.control_write;

    let fake_peer = async move {
        let mut got = [0u8; PREAMBLE_LEN];
        b_r.read_exact(&mut got).await.expect("fake peer read");
        // Correct opening and version, but the network item's head spells
        // a 16-byte *text* string (0x70) where the wire demands a 16-byte
        // byte string (0x50).
        let mut reply = preamble(V2_OPENING, Protocol::V2 as u8, INTENT_REMAIN);
        reply[12] = 0x70;
        b_w.write_all(&reply).await.expect("fake peer write");
    };

    let alice: Rumors<String> = Peer::seed().sync_window_floor().into_rumors();
    let alice_fut = alice.gossip(&mut a_link);

    let (alice_result, ()) = tokio::join!(alice_fut, fake_peer);
    match alice_result {
        Err(Error::PreambleMalformed { defect }) => {
            assert_eq!(defect, rumors::error::PreambleDefect::Network);
        }
        other => panic!("expected PreambleMalformed, got {other:?}"),
    }
}

/// The preamble must be the connection's first bytes: a peer that skips it and
/// goes straight to protocol traffic is rejected as a magic mismatch before
/// any peer-declared protocol frame length can be read or trusted.
#[pollster::test]
async fn handshake_precedes_protocol_traffic() {
    let (mut a_link, b) = rumors::link::memory();
    let b = b.into_parts();
    let mut b_r = b.control_read;
    let mut b_w = b.control_write;

    let fake_peer = async move {
        let mut got = [0u8; PREAMBLE_LEN];
        b_r.read_exact(&mut got).await.expect("fake peer read");
        // Arbitrary protocol-looking bytes whose opening is definitely not
        // a rumors preamble.
        let reply = [b'X'; PREAMBLE_LEN];
        b_w.write_all(&reply).await.expect("fake peer write");
    };

    let alice: Rumors<String> = Peer::seed().sync_window_floor().into_rumors();
    let alice_fut = alice.gossip(&mut a_link);

    let (alice_result, ()) = tokio::join!(alice_fut, fake_peer);
    match alice_result {
        Err(Error::MagicMismatch { remote_magic }) => {
            assert_eq!(remote_magic, *b"XXXXXX");
        }
        other => panic!("expected MagicMismatch, got {other:?}"),
    }
}
