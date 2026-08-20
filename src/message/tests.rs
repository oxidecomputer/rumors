use crate::message::{PayloadCodec, PayloadDepthLimit};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use bytes::Bytes;
use proptest::prelude::*;
use serde::{Deserialize, Serialize};

use super::Message;

use serde::Serializer;
/// A small serde payload with varied field types, so proptests exercise
/// nontrivial serialization structure (nested containers, strings) rather
/// than only fixed-width primitives.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct Payload {
    id: u64,
    tag: String,
    data: Vec<u8>,
}

fn payload() -> impl Strategy<Value = Payload> {
    (any::<u64>(), any::<String>(), any::<Vec<u8>>()).prop_map(|(id, tag, data)| Payload {
        id,
        tag,
        data,
    })
}

fn hash_of<T: Hash>(value: &T) -> u64 {
    let mut h = DefaultHasher::new();
    value.hash(&mut h);
    h.finish()
}

/// Encode a value as one CBOR value, as `Message::new` does internally.
fn cbor_vec<T: Serialize>(value: &T) -> Vec<u8> {
    let mut buf = Vec::new();
    ciborium::ser::into_writer(value, &mut buf).unwrap();
    buf
}

proptest! {
    /// After construction via `new`, the cached serialized bytes are exactly
    /// the value's CBOR encoding, and the typed read recovers the value.
    #[test]
    fn new_caches_cbor_serialization(p in payload()) {
        let m = Message::new(p.clone());
        let direct = cbor_vec(&p);
        prop_assert_eq!(m.bytes(), direct.as_slice());
        prop_assert_eq!(&*m.arc::<Payload>(), &p);
    }

    /// `from_slice` reconstructs the inner value and stores exactly the input
    /// bytes in the cache, with no reserialization drift.
    #[test]
    fn from_slice_roundtrips(p in payload()) {
        let bytes = cbor_vec(&p);
        let m = Message::from_slice::<Payload>(&bytes, PayloadDepthLimit::default()).unwrap();
        prop_assert_eq!(&*m.arc::<Payload>(), &p);
        prop_assert_eq!(m.bytes(), bytes.as_slice());
    }

    /// `from_bytes` (zero-copy) and `from_slice` (copying) produce equivalent
    /// `Message`s from the same input.
    #[test]
    fn from_bytes_matches_from_slice(p in payload()) {
        let bytes = cbor_vec(&p);
        let a = Message::from_slice::<Payload>(&bytes, PayloadDepthLimit::default()).unwrap();
        let b = Message::from_bytes::<Payload>(Bytes::from(bytes.clone()), PayloadDepthLimit::default()).unwrap();
        prop_assert_eq!(&a, &b);
        prop_assert_eq!(a.bytes(), b.bytes());
    }

    /// A payload followed by trailing bytes is rejected: the cache is
    /// always exactly one CBOR value's encoding, never a value plus noise.
    #[test]
    fn trailing_bytes_are_rejected(p in payload(), trailer in proptest::collection::vec(any::<u8>(), 1..8)) {
        let mut bytes = cbor_vec(&p);
        bytes.extend_from_slice(&trailer);
        prop_assert!(Message::from_slice::<Payload>(&bytes, PayloadDepthLimit::default()).is_err());
        prop_assert!(Message::from_bytes::<Payload>(Bytes::from(bytes), PayloadDepthLimit::default()).is_err());
    }

    /// The serde form of a `Message` is one CBOR byte string wrapping
    /// the cached payload bytes — never a re-encoding of the payload — so
    /// nesting a message in a larger CBOR value costs one length header.
    #[test]
    fn serde_form_wraps_cached_bytes(p in payload()) {
        struct Bstr<'a>(&'a [u8]);
        impl Serialize for Bstr<'_> {
            fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.serialize_bytes(self.0)
            }
        }
        let m = Message::new(p);
        let wrapped = cbor_vec(&m);
        let direct = cbor_vec(&Bstr(m.bytes()));
        prop_assert_eq!(wrapped, direct);
    }

    /// A `Message` roundtrips through its serde form: `from_reader` on a
    /// serialized message yields an equal message with equal cached bytes.
    #[test]
    fn serde_roundtrip(p in payload()) {
        let m = Message::new(p);
        let bytes = cbor_vec(&m);
        let back = Message::from_reader(bytes.as_slice(), PayloadCodec::mint::<Payload>(PayloadDepthLimit::default())).unwrap();
        prop_assert_eq!(&m, &back);
        prop_assert_eq!(m.bytes(), back.bytes());
    }

    /// Reading a message off a stream consumes exactly the message's own
    /// bytes: trailing data after the CBOR value survives for the next
    /// field (the property the wire codec's mid-stream decodes rest on).
    #[test]
    fn stream_decode_consumes_only_message_bytes(p in payload(), trailer in any::<Vec<u8>>()) {
        let m = Message::new(p);
        let mut combined = cbor_vec(&m);
        let expected = combined.clone();
        combined.extend_from_slice(&trailer);

        let mut slice: &[u8] = &combined;
        let back = Message::from_reader(&mut slice, PayloadCodec::mint::<Payload>(PayloadDepthLimit::default())).unwrap();
        prop_assert_eq!(back.bytes(), m.bytes());
        prop_assert_eq!(slice, trailer.as_slice());
        prop_assert_eq!(combined.len() - slice.len(), expected.len());
    }

    /// Equal `Message`s hash identically, so `Hash` agrees with
    /// `PartialEq` as required by the standard library contract.
    #[test]
    fn eq_implies_hash_eq(p in payload()) {
        let a = Message::new(p.clone());
        let b = Message::new(p);
        prop_assert_eq!(&a, &b);
        prop_assert_eq!(hash_of(&a), hash_of(&b));
    }

    /// `arc` hands out the same shared allocation `new` stored, not a
    /// copy: unsizing erased the type, never the identity.
    #[test]
    fn arc_shares_the_stored_allocation(p in payload()) {
        let stored = std::sync::Arc::new(p);
        let m = Message::from_arc(stored.clone());
        prop_assert!(std::sync::Arc::ptr_eq(&stored, &m.arc::<Payload>()));
    }
}

/// A typed read with the wrong payload type panics: the mispairing is a
/// crate bug, and the downcast is the tripwire that catches it.
#[test]
#[should_panic(expected = "payload type matches")]
fn mismatched_downcast_panics() {
    let m = Message::new(0u64);
    let _ = m.arc::<String>();
}

/// Nested-array CBOR bytes at exactly `depth` scopes: `depth` array heads
/// around one integer, the minimal encoding whose nesting depth is chosen
/// freely by the test.
fn nested_arrays(depth: usize) -> Vec<u8> {
    let mut bytes = vec![0x81; depth];
    bytes.push(0x00);
    bytes
}

/// The rehydration constructors take the limit explicitly, so an
/// application on a raised fleet limit can rehydrate its own stored
/// deep messages.
///
/// A payload past the default depth fails `from_slice` and `from_bytes`
/// at the default limit (as invalid data) and succeeds at a raised one:
/// both directions, so the parameter is proven live in each.
#[test]
fn rehydration_honors_the_explicit_limit() {
    let default = PayloadDepthLimit::default();
    let deep = nested_arrays((default.get() + 1) as usize);

    let rejected = Message::from_slice::<ciborium::Value>(&deep, default);
    assert_eq!(
        rejected.unwrap_err().kind(),
        std::io::ErrorKind::InvalidData,
        "the default limit must reject a payload one scope past it"
    );
    let rejected = Message::from_bytes::<ciborium::Value>(Bytes::from(deep.clone()), default);
    assert_eq!(
        rejected.unwrap_err().kind(),
        std::io::ErrorKind::InvalidData
    );

    let raised = PayloadDepthLimit::new(default.get() + 1);
    let m = Message::from_slice::<ciborium::Value>(&deep, raised)
        .expect("a raised limit must rehydrate the deep message");
    assert_eq!(m.as_slice(), deep.as_slice());
    let m = Message::from_bytes::<ciborium::Value>(Bytes::from(deep.clone()), raised)
        .expect("a raised limit must rehydrate the deep message");
    assert_eq!(m.as_slice(), deep.as_slice());
}

/// One wrapper scope for the depth differential's generated spines.
#[derive(Debug, Clone, Copy)]
enum Wrap {
    Array,
    Map,
    Tag(u64),
}

/// Arrays, maps, and tags mixed, with tag numbers straddling the
/// decoder's big-integer special case (tags 2 and 3) and ordinary tags.
fn wrap_kind() -> impl Strategy<Value = Wrap> {
    prop_oneof![
        Just(Wrap::Array),
        Just(Wrap::Map),
        (0u64..=4).prop_map(Wrap::Tag),
        Just(Wrap::Tag(24)),
    ]
}

/// A leaf the decoder consumes without opening a scope — scalars of every
/// major type — plus the tag-2-over-bytes big-integer form, whose byte
/// lengths straddle the decoder's 16-byte scalar cutoff.
fn depth_leaf() -> impl Strategy<Value = ciborium::Value> {
    use ciborium::Value;
    prop_oneof![
        any::<i64>().prop_map(|n| Value::Integer(n.into())),
        any::<f64>().prop_map(Value::Float),
        any::<bool>().prop_map(Value::Bool),
        Just(Value::Null),
        "[a-z]{0,8}".prop_map(Value::Text),
        proptest::collection::vec(any::<u8>(), 0..8).prop_map(Value::Bytes),
        proptest::collection::vec(any::<u8>(), 0..=20)
            .prop_map(|b| Value::Tag(2, Box::new(Value::Bytes(b)))),
    ]
}

/// Wrap `leaf` in the given scopes, outermost first.
fn wrapped(kinds: &[Wrap], leaf: ciborium::Value) -> ciborium::Value {
    use ciborium::Value;
    let mut value = leaf;
    for kind in kinds.iter().rev() {
        value = match kind {
            Wrap::Array => Value::Array(vec![value]),
            Wrap::Map => Value::Map(vec![(Value::Integer(0.into()), value)]),
            Wrap::Tag(tag) => Value::Tag(*tag, Box::new(value)),
        };
    }
    value
}

proptest! {
    /// The send-side depth scanner and the decoder's recursion limit
    /// agree on accept/reject for values nested to depths around the
    /// limit (limit − 1 through limit + 1 wrapper scopes).
    ///
    /// Arrays, maps, and tags mixed over scalar leaves: the committed
    /// instrument that keeps the symmetric bound true against either
    /// side drifting.
    #[test]
    fn depth_scanner_agrees_with_the_decoder(
        (limit, kinds) in (2u64..=16).prop_flat_map(|limit| {
            let lo = (limit - 1) as usize;
            (
                Just(limit),
                proptest::collection::vec(wrap_kind(), lo..=lo + 2),
            )
        }),
        leaf in depth_leaf(),
    ) {
        let value = wrapped(&kinds, leaf);
        let bytes = cbor_vec(&value);
        let limit = super::PayloadDepthLimit::new(limit);
        let scanner = super::depth_within(&bytes, limit).is_ok();
        let decoder = match ciborium::de::from_reader_with_recursion_limit::<ciborium::Value, _>(
            bytes.as_slice(),
            limit.recursion_limit(),
        ) {
            Ok(_) => true,
            Err(ciborium::de::Error::RecursionLimitExceeded) => false,
            Err(e) => {
                return Err(TestCaseError::fail(format!(
                    "the decoder failed outside the depth domain: {e}"
                )));
            }
        };
        prop_assert_eq!(
            scanner,
            decoder,
            "scanner and decoder disagree at {:?} over {:?}",
            limit,
            kinds
        );
    }
}

/// The admission boundary is exact and typed: a value at the configured
/// limit constructs, one scope past it is `PayloadDepthError` carrying
/// the configured limit, and the decoder draws the same line.
#[test]
fn try_new_admits_exactly_the_limit() {
    let limit = super::PayloadDepthLimit::new(8);
    let at = wrapped(&[Wrap::Array; 8], ciborium::Value::Integer(0.into()));
    let m = Message::try_new(at.clone(), limit).expect("at the limit is admitted");
    assert_eq!(m.bytes(), cbor_vec(&at).as_slice());

    let over = ciborium::Value::Array(vec![at]);
    let error = Message::try_new(over.clone(), limit).unwrap_err();
    assert_eq!(error.limit, limit);

    // The decoder agrees on both sides of the line.
    assert!(
        Message::from_slice::<ciborium::Value>(&cbor_vec(&over), limit).is_err(),
        "the decoder rejects what the scanner rejects"
    );
    let raised = super::PayloadDepthLimit::new(9);
    Message::try_new(over, raised).expect("one more scope of limit admits it");
}

/// The minted codec's serializing half applies the carried limit and
/// reuses the caller's allocation.
///
/// The codec is `Message::try_new` with the peer's configured limit
/// riding along.
#[test]
fn codec_serializes_through_the_carried_limit() {
    use std::sync::Arc;
    let limit = super::PayloadDepthLimit::new(4);
    let codec = super::PayloadCodec::mint::<ciborium::Value>(limit);

    let deep = wrapped(&[Wrap::Map; 5], ciborium::Value::Bool(true));
    let error = codec.message(Arc::new(deep)).unwrap_err();
    assert_eq!(error.limit, limit);

    let shallow = wrapped(&[Wrap::Tag(24); 4], ciborium::Value::Null);
    let stored: Arc<ciborium::Value> = Arc::new(shallow.clone());
    let m = codec.message(stored.clone()).expect("within the limit");
    assert_eq!(m.bytes(), cbor_vec(&shallow).as_slice());
    assert!(
        std::sync::Arc::ptr_eq(&stored, &m.arc::<ciborium::Value>()),
        "the codec stores the caller's own allocation"
    );
}
