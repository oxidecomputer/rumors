//! Raw protocol preamble exchange (`mirror::remote::preamble`).
//!
//! Drives [`rumors::Known::gossip`] against a hand-crafted peer over a
//! [`tokio::io::duplex`] pipe, asserting that a mismatched magic or version
//! surfaces as the typed error variant rather than corrupting the local rumor
//! set. The preamble is the *raw* (non-length-delimited) prefix validated before
//! any framed traffic; the [`Network`](rumors::Network) now rides the framed
//! greeting that follows, so network-mismatch rejection is exercised separately
//! in `tests/network.rs`.

use rumors::{Error, Known, PROTOCOL_MAGIC, PROTOCOL_VERSION};
use tokio::io::{AsyncReadExt, AsyncWriteExt, duplex};

/// Length of the raw preamble: magic(6) + version(BE u16). The network is no
/// longer part of the preamble; it travels in the framed greeting that follows.
const PREAMBLE_LEN: usize = 8;

/// Assemble a raw preamble by hand, matching the layout the protocol encodes:
/// `magic(6) | version(BE u16)`.
fn preamble(magic: [u8; 6], version: u16) -> [u8; PREAMBLE_LEN] {
    let mut p = [0u8; PREAMBLE_LEN];
    p[..6].copy_from_slice(&magic);
    p[6..8].copy_from_slice(&version.to_be_bytes());
    p
}

/// The compile-time constants match the layout this test crate
/// encodes by hand.
#[test]
fn protocol_constants_match_spec() {
    assert_eq!(PROTOCOL_MAGIC, *b"RUMORS");
    assert_eq!(PROTOCOL_VERSION, 1);
}

/// Two well-behaved peers in the same universe complete the preamble and
/// proceed to a (trivially empty) gossip session.
#[tokio::test(flavor = "current_thread")]
async fn handshake_roundtrip_succeeds() {
    let (a, b) = duplex(1024);
    let (mut a_r, mut a_w) = tokio::io::split(a);
    let (mut b_r, mut b_w) = tokio::io::split(b);

    // Same universe: `bob` is a fork of `alice`, so their networks match.
    let alice: Known<String> = Known::seed();
    let bob = alice.rumors();

    let (alice_out, bob_out) = tokio::join!(
        alice.gossip(&mut a_r, &mut a_w),
        bob.gossip(&mut b_r, &mut b_w),
    );

    alice_out.expect("alice gossip");
    bob_out.expect("bob gossip");
}

/// A peer that opens with the wrong magic is rejected with
/// [`Error::MagicMismatch`] before any framed traffic.
#[tokio::test(flavor = "current_thread")]
async fn magic_mismatch_surfaces_error() {
    let (a, b) = duplex(1024);
    let (mut a_r, mut a_w) = tokio::io::split(a);
    let (mut b_r, mut b_w) = tokio::io::split(b);

    let bad_magic = *b"NOPENO";
    let fake_peer = async move {
        // Drain alice's preamble (so her write_all completes) and reply with a
        // non-rumors one.
        let mut got = [0u8; PREAMBLE_LEN];
        b_r.read_exact(&mut got).await.expect("fake peer read");
        let reply = preamble(bad_magic, PROTOCOL_VERSION);
        b_w.write_all(&reply).await.expect("fake peer write");
    };

    let alice: Known<String> = Known::seed();
    let alice_fut = alice.gossip(&mut a_r, &mut a_w);

    let (alice_result, ()) = tokio::join!(alice_fut, fake_peer);
    match alice_result {
        Err(Error::MagicMismatch { remote_magic }) => {
            assert_eq!(remote_magic, bad_magic);
        }
        other => panic!("expected MagicMismatch, got {other:?}"),
    }
}

/// A peer with correct magic but an unsupported version is rejected
/// with [`Error::VersionMismatch`].
#[tokio::test(flavor = "current_thread")]
async fn version_mismatch_surfaces_error() {
    let (a, b) = duplex(1024);
    let (mut a_r, mut a_w) = tokio::io::split(a);
    let (mut b_r, mut b_w) = tokio::io::split(b);

    // Pick a version we definitely don't speak yet.
    let bogus_version: u16 = PROTOCOL_VERSION.wrapping_add(0xFFFE);
    let fake_peer = async move {
        let mut got = [0u8; PREAMBLE_LEN];
        b_r.read_exact(&mut got).await.expect("fake peer read");
        // Correct magic, bogus version: the version check fires on the raw
        // prefix, before any framed greeting is read.
        let reply = preamble(PROTOCOL_MAGIC, bogus_version);
        b_w.write_all(&reply).await.expect("fake peer write");
    };

    let alice: Known<String> = Known::seed();
    let alice_fut = alice.gossip(&mut a_r, &mut a_w);

    let (alice_result, ()) = tokio::join!(alice_fut, fake_peer);
    match alice_result {
        Err(Error::VersionMismatch { remote_version }) => {
            assert_eq!(remote_version, bogus_version);
        }
        other => panic!("expected VersionMismatch, got {other:?}"),
    }
}

/// A peer that closes the connection mid-preamble surfaces as an
/// I/O error (specifically `UnexpectedEof`), not a malformed-
/// preamble error.
#[tokio::test(flavor = "current_thread")]
async fn truncated_handshake_io_error() {
    let (a, b) = duplex(1024);
    let (mut a_r, mut a_w) = tokio::io::split(a);
    let (mut b_r, mut b_w) = tokio::io::split(b);

    let fake_peer = async move {
        let mut got = [0u8; PREAMBLE_LEN];
        b_r.read_exact(&mut got).await.expect("fake peer read");
        // Write only the six magic bytes (short of the full preamble), then
        // drop the write half to signal EOF mid-preamble.
        b_w.write_all(b"RUMORS").await.expect("partial write");
        drop(b_w);
    };

    let alice: Known<String> = Known::seed();
    let alice_fut = alice.gossip(&mut a_r, &mut a_w);

    let (alice_result, ()) = tokio::join!(alice_fut, fake_peer);
    match alice_result {
        Err(Error::Io(e)) => {
            assert_eq!(
                e.kind(),
                std::io::ErrorKind::UnexpectedEof,
                "expected UnexpectedEof, got {e:?}",
            );
        }
        other => panic!("expected Io(UnexpectedEof), got {other:?}"),
    }
}

/// The preamble bytes appear *before* any length-delimited frame: a peer that
/// skips the preamble and goes straight to framed traffic must be rejected as a
/// magic mismatch (the framing's 4-byte length prefix will not parse as the
/// magic).
#[tokio::test(flavor = "current_thread")]
async fn handshake_precedes_framed_traffic() {
    let (a, b) = duplex(1024);
    let (mut a_r, mut a_w) = tokio::io::split(a);
    let (mut b_r, mut b_w) = tokio::io::split(b);

    let fake_peer = async move {
        let mut got = [0u8; PREAMBLE_LEN];
        b_r.read_exact(&mut got).await.expect("fake peer read");
        // A frame-style length prefix (4-byte big-endian) for a 64-byte
        // payload, then junk to fill the preamble width. The leading six bytes
        // `[0, 0, 0, 64, X, X]` are definitely not `RUMORS`.
        let mut reply = [b'X'; PREAMBLE_LEN];
        reply[..4].copy_from_slice(&[0, 0, 0, 64]);
        b_w.write_all(&reply).await.expect("fake peer write");
    };

    let alice: Known<String> = Known::seed();
    let alice_fut = alice.gossip(&mut a_r, &mut a_w);

    let (alice_result, ()) = tokio::join!(alice_fut, fake_peer);
    assert!(
        matches!(alice_result, Err(Error::MagicMismatch { .. })),
        "expected MagicMismatch, got {alice_result:?}",
    );
}
