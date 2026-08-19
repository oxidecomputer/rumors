//! Ingress validation of the control-stream greeting decoder.
//!
//! The greeting is one peer-controlled item arriving on the control
//! stream — the embedded-item tag wrapping a byte string of the greeting
//! map — whose structural validation lives in [`receive`]: deterministic
//! heads, the exact key roster, and the same canonical-order rule the
//! frame codec applies to a wire query, applied at the greeting ingress.
//!
//! The scripted-fault harness wraps only data streams, so this ingress is
//! exercised here directly: crafted control-stream bytes must surface the
//! typed greeting errors ([`Error::HandshakeRead`] for truncation and
//! length lies, [`Error::HandshakeListing`] for canonical-order violations,
//! [`Error::HandshakeDecode`] for malformed items), never a panic, and a
//! canonical greeting must decode intact.

use std::convert::Infallible;

use proptest::collection::vec;
use proptest::prelude::*;

use super::{Error, Greeting, receive};
use crate::Version;
use crate::observe::SessionHandle;
use crate::tree::arb::nth_party;
use crate::tree::mirror::cbor::{self, MAJOR_BSTR, TAG_EMBEDDED_ITEM};
use crate::tree::mirror::streaming::remote::codec::QueryOrderError;
use crate::tree::mirror::streaming::remote::codec::greeting::encode_greeting;
use crate::tree::typed::Hash;

/// Wrap raw content exactly as the greeting item does: the embedded-item
/// tag, then a byte string of the content.
fn raw_item(content: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    cbor::write_tag(&mut bytes, TAG_EMBEDDED_ITEM);
    cbor::write_head(&mut bytes, MAJOR_BSTR, content.len() as u64);
    bytes.extend_from_slice(content);
    bytes
}

/// A greeting whose sizes are zero and whose listing is caller-selected;
/// the encoder trusts its caller, so a non-canonical listing synthesizes
/// wire violations directly.
fn greeting(listing: Vec<(u8, Hash)>) -> Vec<u8> {
    encode_greeting(&Greeting {
        version: Version::new(),
        set_len: 0,
        max_version_bytes: 0,
        target_message_size: 0,
        listing,
    })
}

/// Decode crafted greeting bytes through the production ingress.
async fn receive_greeting(bytes: &[u8]) -> Result<Greeting, Error<Infallible>> {
    receive(&mut &bytes[..], &SessionHandle::default()).await
}

/// The map content behind a greeting item's heads.
fn content_of(item: &[u8]) -> Vec<u8> {
    let mut input = item;
    cbor::read_head(&mut input).expect("the item's tag head");
    cbor::read_head(&mut input).expect("the item's string head");
    input.to_vec()
}

/// A greeting cut inside the item's heads fails as a typed read error.
///
/// The tag and byte-string heads are the first peer-controlled bytes of
/// the greeting; a peer that closes mid-head must surface
/// [`Error::HandshakeRead`] with `UnexpectedEof` — never a hang waiting on
/// bytes that cannot arrive.
#[pollster::test]
async fn truncated_version_header_is_a_typed_read_error() {
    let result = receive_greeting(&[0xd8]).await.map(|_| ());
    match result {
        Err(Error::HandshakeRead(error)) => {
            assert_eq!(error.kind(), std::io::ErrorKind::UnexpectedEof)
        }
        other => panic!("expected the truncated head's typed rejection, got {other:?}"),
    }
}

/// A greeting item declaring more bytes than the stream carries fails as a
/// typed read error.
///
/// An over-declared byte-string head makes the item's exact read run off
/// the end of the peer's bytes; the lie must surface
/// [`Error::HandshakeRead`] with `UnexpectedEof`, never a partially filled
/// item handed to the decoder.
#[pollster::test]
async fn over_declared_version_frame_is_a_typed_read_error() {
    let mut bytes = Vec::new();
    cbor::write_tag(&mut bytes, TAG_EMBEDDED_ITEM);
    cbor::write_head(&mut bytes, MAJOR_BSTR, 8);
    bytes.extend_from_slice(&[1, 2, 3]);

    let result = receive_greeting(&bytes).await.map(|_| ());
    match result {
        Err(Error::HandshakeRead(error)) => {
            assert_eq!(error.kind(), std::io::ErrorKind::UnexpectedEof)
        }
        other => panic!("expected the over-declared item's typed rejection, got {other:?}"),
    }
}

/// An empty greeting item fails as a typed decode error.
///
/// An item whose byte string is empty carries no map at all; it must
/// surface [`Error::HandshakeDecode`] — the under-declared degenerate
/// case, distinct from the transport-level truncations above.
#[pollster::test]
async fn empty_version_frame_is_a_typed_decode_error() {
    let result = receive_greeting(&raw_item(&[])).await.map(|_| ());
    assert!(
        matches!(result, Err(Error::HandshakeDecode(_))),
        "expected the empty item's typed rejection, got {result:?}",
    );
}

/// A control stream opening with anything but the embedded-item tag fails
/// as a typed decode error.
///
/// The tag is the greeting's identity on the wire: a bare map (however
/// well-formed inside) is not the greeting's one spelling.
#[pollster::test]
async fn untagged_greeting_is_a_typed_decode_error() {
    let item = greeting(Vec::new());
    let content = content_of(&item);

    let result = receive_greeting(&content).await.map(|_| ());
    assert!(
        matches!(result, Err(Error::HandshakeDecode(_))),
        "expected the untagged item's typed rejection, got {result:?}",
    );
}

/// A greeting item with bytes after its map fails as a typed decode error.
///
/// The greeting decode is canonical: the item must contain exactly one
/// map, so trailing bytes surface [`Error::HandshakeDecode`] rather than
/// being silently dropped (which would let two encodings name one
/// greeting).
#[pollster::test]
async fn trailing_version_bytes_are_rejected() {
    let item = greeting(Vec::new());
    let mut content = content_of(&item);
    content.push(0xFF);

    let result = receive_greeting(&raw_item(&content)).await.map(|_| ());
    assert!(
        matches!(result, Err(Error::HandshakeDecode(_))),
        "expected the trailing bytes' typed rejection, got {result:?}",
    );
}

/// A greeting whose stream ends inside the item's content fails as a
/// typed read error.
///
/// The byte-string head promised more content than arrived: the exact
/// read runs off the stream's end, a transport-level truncation.
#[pollster::test]
async fn missing_listing_frame_is_a_typed_read_error() {
    let item = greeting(Vec::new());
    let bytes = &item[..item.len() - 1];

    let result = receive_greeting(bytes).await.map(|_| ());
    match result {
        Err(Error::HandshakeRead(error)) => {
            assert_eq!(error.kind(), std::io::ErrorKind::UnexpectedEof)
        }
        other => panic!("expected the cut content's typed rejection, got {other:?}"),
    }
}

proptest! {
    /// Arbitrary greeting item contents decode to a greeting or a typed
    /// error, never a panic.
    ///
    /// The item is honestly sized around arbitrary content, so the fuzz
    /// lands on the map decoder (heads, key roster, version atom, listing
    /// shape and order) rather than on the allocator via a lied length —
    /// the head lies are pinned deterministically above. Every outcome
    /// must be `Ok` or one of the three typed greeting errors.
    #[test]
    fn arbitrary_greeting_bodies_never_panic(
        content in vec(any::<u8>(), 0..96),
    ) {
        let bytes = raw_item(&content);
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
    let item = greeting(vec![(2_u8, Hash::default()), (1_u8, Hash::default())]);

    let result = receive_greeting(&item).await.map(|_| ());
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
    let item = greeting(vec![(3_u8, Hash::default()), (3_u8, Hash::default())]);

    let result = receive_greeting(&item).await.map(|_| ());
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

/// A listing map whose content is truncated fails as a typed decode error.
///
/// The greeting's map declared more listing entries than its content
/// carries; the cut must surface [`Error::HandshakeDecode`] — a typed
/// greeting failure, never a panic and never a partial listing.
#[pollster::test]
async fn truncated_listing_body_is_rejected() {
    let item = greeting(vec![(0_u8, Hash::default()), (1_u8, Hash::default())]);
    let mut content = content_of(&item);
    content.truncate(content.len() - 1);

    let result = receive_greeting(&raw_item(&content)).await.map(|_| ());
    assert!(
        matches!(result, Err(Error::HandshakeDecode(_))),
        "expected the truncated listing's typed rejection, got {result:?}",
    );
}

/// A canonical greeting with an empty listing decodes intact.
///
/// The empty listing is a legal greeting — an empty tree's root fan — and
/// the validation path must pass it through: the decoded handshake carries
/// the sent fields and the empty listing, exercising the success arm of the
/// same ingress the rejection tests pin.
#[pollster::test]
async fn empty_listing_greeting_decodes() {
    let mut version = Version::new();
    version.tick(&nth_party(0));
    let item = encode_greeting(&Greeting {
        version: version.clone(),
        set_len: 7,
        max_version_bytes: 512,
        target_message_size: 1 << 16,
        listing: Vec::new(),
    });

    let handshake = receive_greeting(&item)
        .await
        .expect("a canonical empty-listing greeting decodes");
    assert_eq!(handshake.version, version);
    assert_eq!(handshake.set_len, 7);
    assert_eq!(handshake.max_version_bytes, 512);
    assert_eq!(handshake.target_message_size, 1 << 16);
    assert!(handshake.listing.is_empty());
}
