//! Ingress validation of the control-stream greeting decoder.
//!
//! The greeting's root-fan listing is peer-controlled bytes arriving on the
//! control stream, and its structural validation lives in [`receive`] — not
//! in the data-stream query decode, where it lived before the listing rode
//! the greeting. The scripted-fault harness wraps only data streams, so this
//! ingress is exercised here directly: crafted control-stream bytes must
//! surface the typed greeting errors ([`Error::HandshakeListing`] for
//! canonical-order violations, [`Error::HandshakeDecode`] for malformed
//! bodies), never a panic, and a canonical greeting must decode intact.

use std::convert::Infallible;

use super::{Error, Handshake, receive};
use crate::Version;
use crate::tree::mirror::streaming::remote::codec::QueryOrderError;
use crate::tree::typed::Hash;

/// Length-delimit one frame body exactly as [`super::send`] does.
fn frame(body: &[u8]) -> Vec<u8> {
    let len = u32::try_from(body.len()).expect("test frame bodies fit in u32");
    let mut bytes = len.to_be_bytes().to_vec();
    bytes.extend_from_slice(body);
    bytes
}

/// A full greeting: the identity-version frame, then `listing_body` framed.
fn greeting(listing_body: &[u8]) -> Vec<u8> {
    let mut bytes = frame(Version::new().as_bytes());
    bytes.extend_from_slice(&frame(listing_body));
    bytes
}

/// Decode crafted greeting bytes through the production ingress.
async fn receive_greeting(bytes: &[u8]) -> Result<Handshake, Error<Infallible>> {
    receive(&mut &bytes[..]).await
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
