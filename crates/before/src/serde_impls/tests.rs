//! The serde suite for the rank and span surfaces.
//!
//! Round-trips through self-describing and binary formats, the canonical-bytes
//! payload pin, strict rejection, and composition inside a larger serde value.
//! The party/version/clock legs live beside the world fixture in
//! `clock/tests.rs`.

use proptest::prelude::*;

use crate::span::Span;
use crate::testing::bridge::from_oracle_version;
use crate::testing::generators::arb_oracle_version;
use crate::{Clock, Rank, Ranked, Version};

/// Two strictly ordered versions (`older < newer`) from one history.
fn ordered_pair() -> (Version, Version) {
    let mut clock = Clock::seed();
    let older = clock.tick().clone();
    let newer = clock.tick().clone();
    (older, newer)
}

proptest! {
    /// [`Rank`], [`Ranked`], and [`Span`] round-trip through serde.
    ///
    /// Both deserialization paths are driven: the self-describing number-array
    /// (`serde_json`), the non-self-describing length-prefixed bytes
    /// (`postcard`), and CBOR's *typed* byte string (`ciborium`, major type 2)
    /// — each serialized as the canonical encoding and deserialized back
    /// through the strict decode.
    #[test]
    fn serde_roundtrip_rank_and_span(
        oa in arb_oracle_version(),
        ob in arb_oracle_version(),
    ) {
        let a = from_oracle_version(&oa);
        let b = from_oracle_version(&ob);
        let rank = a.rank();
        let ranked = Ranked::from(a.clone());
        let span = a.span(&b);

        let r2: Rank = serde_json::from_slice(&serde_json::to_vec(&rank).unwrap()).unwrap();
        let k2: Ranked = serde_json::from_slice(&serde_json::to_vec(&ranked).unwrap()).unwrap();
        let s2: Span = serde_json::from_slice(&serde_json::to_vec(&span).unwrap()).unwrap();
        prop_assert_eq!(&r2, &rank);
        prop_assert_eq!(&k2, &ranked);
        prop_assert_eq!(&s2, &span);

        let r2: Rank = postcard::from_bytes(&postcard::to_allocvec(&rank).unwrap()).unwrap();
        let k2: Ranked = postcard::from_bytes(&postcard::to_allocvec(&ranked).unwrap()).unwrap();
        let s2: Span = postcard::from_bytes(&postcard::to_allocvec(&span).unwrap()).unwrap();
        prop_assert_eq!(&r2, &rank);
        prop_assert_eq!(&k2, &ranked);
        prop_assert_eq!(&s2, &span);

        // ciborium: each value must serialize as a CBOR byte string
        // (major type 2), the typed-bytes path `serde_json` never takes.
        let cbor = |bytes: &[u8]| -> u8 { bytes[0] >> 5 };
        let mut buf = Vec::new();
        ciborium::ser::into_writer(&rank, &mut buf).unwrap();
        prop_assert_eq!(cbor(&buf), 2, "Rank did not serialize as a CBOR byte string");
        let r3: Rank = ciborium::de::from_reader(&buf[..]).unwrap();
        prop_assert_eq!(&r3, &rank);
        let mut buf = Vec::new();
        ciborium::ser::into_writer(&ranked, &mut buf).unwrap();
        prop_assert_eq!(cbor(&buf), 2, "Ranked did not serialize as a CBOR byte string");
        let k3: Ranked = ciborium::de::from_reader(&buf[..]).unwrap();
        prop_assert_eq!(&k3, &ranked);
        let mut buf = Vec::new();
        ciborium::ser::into_writer(&span, &mut buf).unwrap();
        prop_assert_eq!(cbor(&buf), 2, "Span did not serialize as a CBOR byte string");
        let s3: Span = ciborium::de::from_reader(&buf[..]).unwrap();
        prop_assert_eq!(&s3, &span);
    }

    /// The serde byte payload is exactly the canonical encoding.
    ///
    /// Each type serializes to the same stream as its own `encode()` bytes
    /// handed to the format as a plain byte sequence — the wire form is
    /// `encode()` with nothing added, reordered, or wrapped.
    #[test]
    fn serde_bytes_pin_the_canonical_encoding_rank_and_span(
        oa in arb_oracle_version(),
        ob in arb_oracle_version(),
    ) {
        let a = from_oracle_version(&oa);
        let b = from_oracle_version(&ob);
        let rank = a.rank();
        let ranked = Ranked::from(a.clone());
        let span = a.span(&b);

        prop_assert_eq!(
            postcard::to_allocvec(&rank).unwrap(),
            postcard::to_allocvec(&rank.encode()).unwrap(),
        );
        prop_assert_eq!(
            postcard::to_allocvec(&ranked).unwrap(),
            postcard::to_allocvec(&ranked.encode()).unwrap(),
        );
        prop_assert_eq!(
            postcard::to_allocvec(&span).unwrap(),
            postcard::to_allocvec(&span.encode()).unwrap(),
        );
    }
}

/// Serde deserialization runs the strict decoders.
///
/// A defective payload is rejected through both the binary (typed-bytes) and
/// the self-describing (number-array) paths, for every rejection genre the raw
/// decodes mint — trailing bytes on each type, the rank-mismatch composite
/// [`Ranked::decode`] rejects, and the crossed pair [`Span::decode`] rejects.
#[test]
fn serde_rejects_defective_rank_and_span_payloads() {
    let (older, newer) = ordered_pair();
    let span = Span::new(&older, &newer).unwrap();

    let mut rank_trailing = older.rank().encode();
    rank_trailing.push(0x00);
    assert!(Rank::decode(&rank_trailing[..]).is_err());

    let mut ranked_trailing = Ranked::from(older.clone()).encode();
    ranked_trailing.push(0x00);
    assert!(Ranked::decode(&ranked_trailing[..]).is_err());

    let mut span_trailing = span.encode();
    span_trailing.push(0x00);
    assert!(Span::decode(&span_trailing[..]).is_err());

    // The composite genres: a rank prefix the version does not
    // measure, and a crossed span pair.
    let mismatched = [newer.rank().encode(), older.encode()].concat();
    assert!(Ranked::decode(&mismatched[..]).is_err());
    let crossed = [newer.encode(), older.encode()].concat();
    assert!(Span::decode(&crossed[..]).is_err());

    let postcard_frame = |body: &[u8]| postcard::to_allocvec(&body.to_vec()).unwrap();
    let json_frame = |body: &[u8]| serde_json::to_vec(&body.to_vec()).unwrap();
    for body in [&rank_trailing, &mismatched] {
        assert!(postcard::from_bytes::<Rank>(&postcard_frame(body)).is_err());
        assert!(serde_json::from_slice::<Rank>(&json_frame(body)).is_err());
    }
    for body in [&ranked_trailing, &mismatched] {
        assert!(postcard::from_bytes::<Ranked>(&postcard_frame(body)).is_err());
        assert!(serde_json::from_slice::<Ranked>(&json_frame(body)).is_err());
    }
    for body in [&span_trailing, &crossed] {
        assert!(postcard::from_bytes::<Span>(&postcard_frame(body)).is_err());
        assert!(serde_json::from_slice::<Span>(&json_frame(body)).is_err());
    }
}

/// The new impls compose inside a larger serde value: a `(Span, Rank, Ranked)`
/// tuple round-trips through postcard, each field framed by the format itself.
#[test]
fn serde_composes_rank_and_span_in_larger_values() {
    let (older, newer) = ordered_pair();
    let tuple = (
        Span::new(&older, &newer).unwrap(),
        older.rank(),
        Ranked::from(newer.clone()),
    );
    let bytes = postcard::to_allocvec(&tuple).unwrap();
    let back: (Span, Rank, Ranked) = postcard::from_bytes(&bytes).unwrap();
    assert_eq!(back, tuple);
}
