//! Ingress validation of the control-stream greeting decoder.
//!
//! The greeting's frames are peer-controlled bytes arriving on the control
//! stream — first the causal-version frame, then the root-fan listing frame,
//! whose structural validation lives in [`receive`] rather than in the
//! data-stream query decode (where the listing lived before it rode the
//! greeting). The scripted-fault harness wraps only data streams, so this
//! ingress is exercised here directly: crafted control-stream bytes must
//! surface the typed greeting errors ([`Error::HandshakeRead`] for
//! truncation and length lies, [`Error::HandshakeListing`] for
//! canonical-order violations, [`Error::HandshakeDecode`] for malformed
//! bodies), never a panic, and a canonical greeting must decode intact.

use std::convert::Infallible;

use proptest::collection::vec;
use proptest::prelude::*;

use super::{Error, Handshake, receive};
use crate::Version;
use crate::tree::arb::nth_party;
use crate::tree::mirror::streaming::remote::codec::QueryOrderError;
use crate::tree::typed::Hash;

/// Length-delimit one frame body exactly as [`super::send`] does.
fn frame(body: &[u8]) -> Vec<u8> {
    let len = u32::try_from(body.len()).expect("test frame bodies fit in u32");
    let mut bytes = len.to_be_bytes().to_vec();
    bytes.extend_from_slice(body);
    bytes
}

/// A version frame's body: the sender's set-size prefix, then the version.
fn version_body(version: &Version) -> Vec<u8> {
    let mut body = 0_u64.to_le_bytes().to_vec();
    body.extend_from_slice(version.as_bytes());
    body
}

/// A full greeting: the identity-version frame, then `listing_body` framed.
fn greeting(listing_body: &[u8]) -> Vec<u8> {
    let mut bytes = frame(&version_body(&Version::new()));
    bytes.extend_from_slice(&frame(listing_body));
    bytes
}

/// Decode crafted greeting bytes through the production ingress.
async fn receive_greeting(bytes: &[u8]) -> Result<Handshake, Error<Infallible>> {
    receive(&mut &bytes[..]).await
}

/// A nonempty causal version, so truncating its encoding leaves bytes to cut.
fn ticked_version() -> Version {
    let party = nth_party(0);
    let mut version = Version::new();
    version.tick(&party);
    version
}

/// A greeting cut inside the version frame's length header fails as a typed
/// read error.
///
/// The four header bytes are the first peer-controlled bytes of the
/// greeting; a peer that closes mid-header must surface
/// [`Error::HandshakeRead`] with `UnexpectedEof` — never a hang waiting on
/// bytes that cannot arrive.
#[pollster::test]
async fn truncated_version_header_is_a_typed_read_error() {
    let result = receive_greeting(&[0, 0]).await.map(|_| ());
    match result {
        Err(Error::HandshakeRead(error)) => {
            assert_eq!(error.kind(), std::io::ErrorKind::UnexpectedEof)
        }
        other => panic!("expected the truncated header's typed rejection, got {other:?}"),
    }
}

/// A version frame declaring more bytes than the stream carries fails as a
/// typed read error.
///
/// An over-declared length header makes the frame's exact read run off the
/// end of the peer's bytes; the lie must surface [`Error::HandshakeRead`]
/// with `UnexpectedEof`, never a partially filled frame handed to the
/// decoder.
#[pollster::test]
async fn over_declared_version_frame_is_a_typed_read_error() {
    let mut bytes = 8_u32.to_be_bytes().to_vec();
    bytes.extend_from_slice(&[1, 2, 3]);

    let result = receive_greeting(&bytes).await.map(|_| ());
    match result {
        Err(Error::HandshakeRead(error)) => {
            assert_eq!(error.kind(), std::io::ErrorKind::UnexpectedEof)
        }
        other => panic!("expected the over-declared frame's typed rejection, got {other:?}"),
    }
}

/// A zero-length version frame fails as a typed decode error.
///
/// A frame whose declared length is zero carries neither the set-size
/// prefix nor a version; the empty body must surface
/// [`Error::HandshakeDecode`] — the under-declared degenerate case,
/// distinct from the transport-level truncations above.
#[pollster::test]
async fn empty_version_frame_is_a_typed_decode_error() {
    let result = receive_greeting(&frame(&[])).await.map(|_| ());
    assert!(
        matches!(result, Err(Error::HandshakeDecode(_))),
        "expected the empty version body's typed rejection, got {result:?}",
    );
}

/// A version body truncated inside an honestly sized frame fails as a typed
/// decode error.
///
/// The frame is well-formed — its header matches its body — but the body is
/// a strict prefix of a canonical version encoding, so the decoder runs out
/// of bits: [`Error::HandshakeDecode`], never a panic and never a shorter
/// version silently accepted.
#[pollster::test]
async fn truncated_version_body_is_a_typed_decode_error() {
    let mut body = version_body(&ticked_version());
    body.truncate(body.len() - 1);

    let result = receive_greeting(&frame(&body)).await.map(|_| ());
    assert!(
        matches!(result, Err(Error::HandshakeDecode(_))),
        "expected the truncated version body's typed rejection, got {result:?}",
    );
}

/// A version frame with bytes after the version fails as a typed decode
/// error.
///
/// The version encoding is prefix-free and the greeting decode is
/// canonical: the frame must contain exactly one version, so trailing bytes
/// surface [`Error::HandshakeDecode`] rather than being silently dropped
/// (which would let two encodings name one greeting).
#[pollster::test]
async fn trailing_version_bytes_are_rejected() {
    let mut body = version_body(&ticked_version());
    body.push(0xFF);

    let result = receive_greeting(&frame(&body)).await.map(|_| ());
    assert!(
        matches!(result, Err(Error::HandshakeDecode(_))),
        "expected the trailing bytes' typed rejection, got {result:?}",
    );
}

/// A greeting that ends after the version frame fails as a typed read error.
///
/// The listing frame is not optional: a peer that sends its version and
/// closes must surface [`Error::HandshakeRead`] with `UnexpectedEof` on the
/// missing listing, never a greeting with a defaulted listing.
#[pollster::test]
async fn missing_listing_frame_is_a_typed_read_error() {
    let bytes = frame(&version_body(&Version::new()));

    let result = receive_greeting(&bytes).await.map(|_| ());
    match result {
        Err(Error::HandshakeRead(error)) => {
            assert_eq!(error.kind(), std::io::ErrorKind::UnexpectedEof)
        }
        other => panic!("expected the missing listing's typed rejection, got {other:?}"),
    }
}

proptest! {
    /// Arbitrary greeting bodies decode to a greeting or a typed error,
    /// never a panic.
    ///
    /// Both frames are honestly sized around arbitrary bodies, so the fuzz
    /// lands on the body decoders (the version's bit codec, the listing's
    /// borsh shape and order check) rather than on the allocator via a lied
    /// length header — the header lies are pinned deterministically above.
    /// Every outcome must be `Ok` or one of the three typed greeting
    /// errors.
    #[test]
    fn arbitrary_greeting_bodies_never_panic(
        version_body in vec(any::<u8>(), 0..64),
        listing_body in vec(any::<u8>(), 0..64),
    ) {
        let mut bytes = frame(&version_body);
        bytes.extend_from_slice(&frame(&listing_body));

        let result = pollster::block_on(receive_greeting(&bytes)).map(|_| ());
        prop_assert!(matches!(
            result,
            Ok(())
                | Err(Error::HandshakeRead(_)
                    | Error::HandshakeDecode(_)
                    | Error::HandshakeListing(_)),
        ));
    }
}

/// A listing whose radixes descend is rejected as [`Error::HandshakeListing`].
///
/// The canonical strictly-ascending radix order is the rule positional
/// pairing rests on; an out-of-order peer listing must die at the greeting
/// with the exact violating pair, before any scope is built from it.
#[pollster::test]
async fn unordered_listing_is_rejected() {
    let listing = vec![(2_u8, Hash::default()), (1_u8, Hash::default())];
    let body = borsh::to_vec(&listing).expect("test listings encode");

    let result = receive_greeting(&greeting(&body)).await.map(|_| ());
    assert!(
        matches!(
            result,
            Err(Error::HandshakeListing(QueryOrderError {
                previous: 2,
                radix: 1,
            })),
        ),
        "expected the unordered pair's typed rejection, got {result:?}",
    );
}

/// A listing repeating a radix is rejected as [`Error::HandshakeListing`].
///
/// Strictly ascending means duplicates are non-canonical too: equal adjacent
/// radixes trip the same greeting-time order check as a descending pair,
/// with both offending radixes reported.
#[pollster::test]
async fn duplicate_listing_radix_is_rejected() {
    let listing = vec![(3_u8, Hash::default()), (3_u8, Hash::default())];
    let body = borsh::to_vec(&listing).expect("test listings encode");

    let result = receive_greeting(&greeting(&body)).await.map(|_| ());
    assert!(
        matches!(
            result,
            Err(Error::HandshakeListing(QueryOrderError {
                previous: 3,
                radix: 3,
            })),
        ),
        "expected the duplicate radix's typed rejection, got {result:?}",
    );
}

/// A listing frame whose borsh body is truncated fails as a typed decode
/// error.
///
/// A frame declaring more listing entries than its body carries must surface
/// [`Error::HandshakeDecode`] — a typed greeting failure, never a panic and
/// never a partial listing.
#[pollster::test]
async fn truncated_listing_body_is_rejected() {
    let listing = vec![(0_u8, Hash::default()), (1_u8, Hash::default())];
    let mut body = borsh::to_vec(&listing).expect("test listings encode");
    body.truncate(body.len() - 1);

    let result = receive_greeting(&greeting(&body)).await.map(|_| ());
    assert!(
        matches!(result, Err(Error::HandshakeDecode(_))),
        "expected the truncated body's typed rejection, got {result:?}",
    );
}

/// A listing frame with bytes after the listing fails as a typed decode
/// error.
///
/// The greeting decode is canonical: the frame must contain exactly one
/// borsh listing, so trailing garbage surfaces [`Error::HandshakeDecode`]
/// rather than being silently ignored (which would let two encodings name
/// one greeting).
#[pollster::test]
async fn trailing_listing_bytes_are_rejected() {
    let listing: Vec<(u8, Hash)> = Vec::new();
    let mut body = borsh::to_vec(&listing).expect("test listings encode");
    body.push(0xFF);

    let result = receive_greeting(&greeting(&body)).await.map(|_| ());
    assert!(
        matches!(result, Err(Error::HandshakeDecode(_))),
        "expected the trailing bytes' typed rejection, got {result:?}",
    );
}

/// A canonical greeting with an empty listing decodes intact.
///
/// The empty listing is a legal greeting — an empty tree's root fan — and
/// the validation path must pass it through: the decoded handshake carries
/// the sent version and the empty listing, exercising the success arm of the
/// same ingress the rejection tests pin.
#[pollster::test]
async fn empty_listing_greeting_decodes() {
    let listing: Vec<(u8, Hash)> = Vec::new();
    let body = borsh::to_vec(&listing).expect("test listings encode");

    let handshake = receive_greeting(&greeting(&body))
        .await
        .expect("a canonical empty-listing greeting decodes");
    assert_eq!(handshake.version, Version::new());
    assert!(handshake.listing.is_empty());
}
