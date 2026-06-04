//! Pre-handshake protocol-version exchange (`mirror::remote::handshake`).
//!
//! Drives [`rumors::Known::gossip`] against a hand-crafted peer over a
//! [`tokio::io::duplex`] pipe, asserting that mismatched magic or
//! version surfaces as the typed error variant rather than corrupting
//! the local rumor set.

use rumors::{Error, Known, PROTOCOL_MAGIC, PROTOCOL_VERSION};
use tokio::io::{AsyncReadExt, AsyncWriteExt, duplex};

/// The compile-time constants match the layout this test crate
/// encodes by hand.
#[test]
fn protocol_constants_match_spec() {
    assert_eq!(PROTOCOL_MAGIC, *b"RUMORS");
    assert_eq!(PROTOCOL_VERSION, 1);
}

/// Two well-behaved peers complete the handshake and proceed to a
/// (trivially empty) gossip session.
#[tokio::test(flavor = "current_thread")]
async fn handshake_roundtrip_succeeds() {
    let (a, b) = duplex(1024);
    let (mut a_r, mut a_w) = tokio::io::split(a);
    let (mut b_r, mut b_w) = tokio::io::split(b);

    let alice: Known<String> = Known::seed();
    let bob: Known<String> = Known::seed();

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
        // Drain alice's handshake (so her write_all completes) and
        // reply with a non-rumors preamble.
        let mut got = [0u8; 8];
        b_r.read_exact(&mut got).await.expect("fake peer read");
        let preamble = [
            bad_magic[0],
            bad_magic[1],
            bad_magic[2],
            bad_magic[3],
            bad_magic[4],
            bad_magic[5],
            0,
            1,
        ];
        b_w.write_all(&preamble).await.expect("fake peer write");
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
        let mut got = [0u8; 8];
        b_r.read_exact(&mut got).await.expect("fake peer read");
        let v = bogus_version.to_be_bytes();
        let preamble = [
            PROTOCOL_MAGIC[0],
            PROTOCOL_MAGIC[1],
            PROTOCOL_MAGIC[2],
            PROTOCOL_MAGIC[3],
            PROTOCOL_MAGIC[4],
            PROTOCOL_MAGIC[5],
            v[0],
            v[1],
        ];
        b_w.write_all(&preamble).await.expect("fake peer write");
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

/// A peer that closes the connection mid-handshake surfaces as an
/// I/O error (specifically `UnexpectedEof`), not a malformed-
/// preamble error.
#[tokio::test(flavor = "current_thread")]
async fn truncated_handshake_io_error() {
    let (a, b) = duplex(1024);
    let (mut a_r, mut a_w) = tokio::io::split(a);
    let (mut b_r, mut b_w) = tokio::io::split(b);

    let fake_peer = async move {
        let mut got = [0u8; 8];
        b_r.read_exact(&mut got).await.expect("fake peer read");
        // Write only the six magic bytes (short of the 8-byte preamble),
        // then drop the write half to signal EOF mid-handshake.
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

/// The handshake bytes appear *before* the existing mirror version
/// vector exchange: a peer that skips the preamble and goes straight
/// to length-delimited frames must be rejected as a magic mismatch
/// (the framing's 4-byte length prefix will not parse as `RUMR`).
#[tokio::test(flavor = "current_thread")]
async fn handshake_precedes_framed_traffic() {
    let (a, b) = duplex(1024);
    let (mut a_r, mut a_w) = tokio::io::split(a);
    let (mut b_r, mut b_w) = tokio::io::split(b);

    let fake_peer = async move {
        let mut got = [0u8; 8];
        b_r.read_exact(&mut got).await.expect("fake peer read");
        // A frame-style length prefix (4-byte big-endian) for a
        // 64-byte payload, then 4 bytes of arbitrary junk. The first
        // 8 bytes are `[0, 0, 0, 64, junk*4]` which definitely is not
        // `RUMORS`.
        b_w.write_all(&[0, 0, 0, 64, b'X', b'X', b'X', b'X'])
            .await
            .expect("fake peer write");
    };

    let alice: Known<String> = Known::seed();
    let alice_fut = alice.gossip(&mut a_r, &mut a_w);

    let (alice_result, ()) = tokio::join!(alice_fut, fake_peer);
    assert!(
        matches!(alice_result, Err(Error::MagicMismatch { .. })),
        "expected MagicMismatch, got {alice_result:?}",
    );
}
