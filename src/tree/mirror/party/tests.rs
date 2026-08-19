//! Ingress validation of the trailing party-donation frame.
//!
//! The donated identity is the last peer-controlled payload of a bootstrap
//! or retire session: one party-atom-tagged byte string whose content must
//! be exactly one canonical party encoding. This suite feeds [`receive`]
//! crafted items — truncations at each structural boundary, length lies in
//! both directions, wrong tags, trailing and arbitrary bodies — and pins
//! that each surfaces as the typed [`Error::Io`], never a panic, never a
//! hang, and never a partial identity; and that a clean receive leaves the
//! next session's bytes untouched in the transport.

use before::Party;
use proptest::collection::vec;
use proptest::prelude::*;

use super::{receive, send};
use crate::Error;
use crate::Protocol;
use crate::observe::SessionHandle;
use crate::tree::arb::nth_party;
use crate::tree::mirror::cbor::{self, MAJOR_BSTR};

/// Wrap one item content exactly as [`send`] does: the party-atom tag,
/// then a byte string of the content.
fn frame(body: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    cbor::write_tag(&mut bytes, crate::tags::PARTY_TAG);
    cbor::write_head(&mut bytes, MAJOR_BSTR, body.len() as u64);
    bytes.extend_from_slice(body);
    bytes
}

/// Receive a donation from crafted wire bytes through the production ingress.
fn receive_party(bytes: &[u8]) -> Result<Party, Error> {
    pollster::block_on(async {
        receive(Protocol::V2, &mut &bytes[..], &SessionHandle::default()).await
    })
}

/// Unwrap the sole error variant this ingress can produce.
fn io_error(result: Result<Party, Error>) -> std::io::Error {
    match result {
        Err(Error::Io(error)) => error,
        Ok(_) => panic!("a malformed donation must not decode"),
        Err(other) => panic!("the donation ingress fails as Error::Io, got {other:?}"),
    }
}

/// A donated party survives its wire trip intact.
///
/// [`send`] and [`receive`] are each other's inverses: the received party
/// equals the donated one (compared against an identically constructed
/// value, since a live [`Party`] is deliberately `!Clone`), pinning the
/// success arm of the same ingress the rejection tests below feed.
#[test]
fn a_donated_party_round_trips() {
    pollster::block_on(async {
        let mut wire = Vec::new();
        send(
            Protocol::V2,
            nth_party(3),
            &mut wire,
            &SessionHandle::default(),
        )
        .await
        .expect("donation sends");
        let received = receive(Protocol::V2, &mut &wire[..], &SessionHandle::default())
            .await
            .expect("a canonical donation decodes");
        assert_eq!(received, nth_party(3));
    });
}

/// A peer that closes before or inside the item's heads is a typed EOF.
///
/// Every strict prefix of the tag and byte-string heads — the close at
/// each boundary included — must resolve to [`Error::Io`] with
/// `UnexpectedEof`, never a hang on bytes that cannot arrive.
#[test]
fn truncated_frame_header_is_a_typed_eof() {
    let heads = frame(&[0]);
    let heads = &heads[..heads.len() - 1];
    for cut in 0..heads.len() {
        let error = io_error(receive_party(&heads[..cut]));
        assert_eq!(
            error.kind(),
            std::io::ErrorKind::UnexpectedEof,
            "cut after {cut} head bytes must be an unexpected EOF",
        );
    }
}

/// An item that is not the party-atom tag is a typed protocol violation.
///
/// The tag is the hand-off's identity on the wire; a different tag (or a
/// bare byte string) must surface as `InvalidData`, never decode.
#[test]
fn wrong_tag_is_a_typed_error() {
    let mut wrong = Vec::new();
    cbor::write_tag(&mut wrong, crate::tags::VERSION_TAG);
    cbor::write_head(&mut wrong, MAJOR_BSTR, 0);
    let error = io_error(receive_party(&wrong));
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);

    let mut bare = Vec::new();
    cbor::write_head(&mut bare, MAJOR_BSTR, 0);
    let error = io_error(receive_party(&bare));
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
}

/// A frame declaring more bytes than the peer sends is a typed EOF.
///
/// The over-declared length makes the exact body read run off the end of
/// the stream; the lie must surface as [`Error::Io`] with `UnexpectedEof`,
/// never as a partially filled body handed to the party decoder.
#[test]
fn over_declared_frame_is_a_typed_eof() {
    let mut bytes = Vec::new();
    cbor::write_tag(&mut bytes, crate::tags::PARTY_TAG);
    cbor::write_head(&mut bytes, MAJOR_BSTR, 16);
    bytes.extend_from_slice(&[1, 2, 3, 4]);

    let error = io_error(receive_party(&bytes));
    assert_eq!(error.kind(), std::io::ErrorKind::UnexpectedEof);
}

/// A zero-length frame body cannot carry an identity and is a typed error.
///
/// The anonymous (empty) id has no reader-path encoding — every canonical
/// party carries at least one tag — so an empty body fails the decoder's
/// first bit read: [`Error::Io`] with `UnexpectedEof`, the under-declared
/// degenerate case.
#[test]
fn empty_frame_body_is_a_typed_error() {
    let error = io_error(receive_party(&frame(&[])));
    assert_eq!(error.kind(), std::io::ErrorKind::UnexpectedEof);
}

/// A party encoding cut short inside an honestly sized frame is rejected.
///
/// The frame's header matches its body, but the body is a strict prefix of
/// a canonical party encoding; the decoder runs out of bits and must
/// surface [`Error::Io`], never accept a smaller identity (which would
/// break party linearity).
#[test]
fn under_declared_frame_is_a_typed_error() {
    let mut body = nth_party(3).as_bytes().to_vec();
    body.truncate(body.len() - 1);

    let error = io_error(receive_party(&frame(&body)));
    assert_eq!(error.kind(), std::io::ErrorKind::UnexpectedEof);
}

/// A frame with bytes after the party is rejected as non-canonical.
///
/// The party encoding is prefix-free and [`receive`] decodes the frame body
/// exactly: one identity, no remainder. Trailing garbage must surface as
/// typed `InvalidData` rather than being silently dropped.
#[test]
fn trailing_frame_bytes_are_rejected() {
    let mut body = nth_party(3).as_bytes().to_vec();
    body.push(0xFF);

    let error = io_error(receive_party(&frame(&body)));
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
}

/// Receiving one donation consumes exactly its frame, leaving later bytes
/// untouched.
///
/// The donation is followed on the same control stream by the epilogue
/// marker (and possibly a next session's preamble); the exact-read framing
/// contract requires those bytes to remain unread after a clean receive.
#[test]
fn bytes_after_the_frame_stay_untouched() {
    pollster::block_on(async {
        let mut wire = Vec::new();
        send(
            Protocol::V2,
            nth_party(3),
            &mut wire,
            &SessionHandle::default(),
        )
        .await
        .expect("donation sends");
        wire.extend_from_slice(b".RUMORS");

        let mut cursor = &wire[..];
        receive(Protocol::V2, &mut cursor, &SessionHandle::default())
            .await
            .expect("a canonical donation decodes");
        assert_eq!(cursor, b".RUMORS", "bytes after the donation were consumed");
    });
}

proptest! {
    /// Arbitrary frame bodies decode to a party or the typed [`Error::Io`] —
    /// never a panic — and anything accepted is canonical.
    ///
    /// The frame is honestly sized around an arbitrary body, so the fuzz
    /// lands on the party bit codec rather than on the allocator via a lied
    /// length header (the header lies are pinned deterministically above).
    /// The canonicality arm mirrors `before`'s decode fuzz target: an
    /// accepted body re-encodes byte-for-byte, so no two frames name one
    /// identity. Decoding a slice always terminates, so a completed run is
    /// also the no-hang witness.
    #[test]
    fn arbitrary_frame_bodies_never_panic(body in vec(any::<u8>(), 0..64)) {
        match receive_party(&frame(&body)) {
            Ok(party) => {
                let reencoded = party.as_bytes().to_vec();
                prop_assert_eq!(reencoded, body, "accepted donation was not canonical");
            }
            Err(Error::Io(_)) => {}
            Err(other) => prop_assert!(false, "expected a typed I/O error, got {other:?}"),
        }
    }
}
