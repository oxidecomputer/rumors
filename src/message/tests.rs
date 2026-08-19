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
    /// the value's CBOR encoding.
    #[test]
    fn new_caches_cbor_serialization(p in payload()) {
        let m = Message::new(p.clone());
        let direct = cbor_vec(&p);
        prop_assert_eq!(m.bytes(), direct.as_slice());
        prop_assert_eq!(m.message(), &p);
    }

    /// `from_slice` reconstructs the inner value and stores exactly the input
    /// bytes in the cache, with no reserialization drift.
    #[test]
    fn from_slice_roundtrips(p in payload()) {
        let bytes = cbor_vec(&p);
        let m = Message::<Payload>::from_slice(&bytes).unwrap();
        prop_assert_eq!(m.message(), &p);
        prop_assert_eq!(m.bytes(), bytes.as_slice());
    }

    /// `from_bytes` (zero-copy) and `from_slice` (copying) produce equivalent
    /// `Message`s from the same input.
    #[test]
    fn from_bytes_matches_from_slice(p in payload()) {
        let bytes = cbor_vec(&p);
        let a = Message::<Payload>::from_slice(&bytes).unwrap();
        let b = Message::<Payload>::from_bytes(Bytes::from(bytes.clone())).unwrap();
        prop_assert_eq!(&a, &b);
        prop_assert_eq!(a.bytes(), b.bytes());
    }

    /// A payload followed by trailing bytes is rejected: the cache is
    /// always exactly one CBOR value's encoding, never a value plus noise.
    #[test]
    fn trailing_bytes_are_rejected(p in payload(), trailer in proptest::collection::vec(any::<u8>(), 1..8)) {
        let mut bytes = cbor_vec(&p);
        bytes.extend_from_slice(&trailer);
        prop_assert!(Message::<Payload>::from_slice(&bytes).is_err());
        prop_assert!(Message::<Payload>::from_bytes(Bytes::from(bytes)).is_err());
    }

    /// The serde form of a `Message<T>` is one CBOR byte string wrapping
    /// the cached payload bytes — never a re-encoding of `T` — so nesting
    /// a message in a larger CBOR value costs one length header.
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

    /// A `Message<T>` roundtrips through its serde form: deserializing a
    /// serialized message yields an equal message with equal cached bytes.
    #[test]
    fn serde_roundtrip(p in payload()) {
        let m = Message::new(p);
        let bytes = cbor_vec(&m);
        let back: Message<Payload> = ciborium::de::from_reader(bytes.as_slice()).unwrap();
        prop_assert_eq!(&m, &back);
        prop_assert_eq!(m.bytes(), back.bytes());
    }

    /// `Message<T>` nests correctly inside other CBOR containers: a
    /// `Vec<Message<T>>` roundtrips and preserves each element's cached
    /// bytes.
    #[test]
    fn nested_in_vec_roundtrips(ps in proptest::collection::vec(payload(), 0..8)) {
        let msgs: Vec<Message<Payload>> =
            ps.into_iter().map(Message::new).collect();
        let bytes = cbor_vec(&msgs);
        let back: Vec<Message<Payload>> = ciborium::de::from_reader(bytes.as_slice()).unwrap();
        prop_assert_eq!(&msgs, &back);
        for (a, b) in msgs.iter().zip(back.iter()) {
            prop_assert_eq!(a.bytes(), b.bytes());
        }
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
        let back: Message<Payload> = ciborium::de::from_reader(&mut slice).unwrap();
        prop_assert_eq!(back.bytes(), m.bytes());
        prop_assert_eq!(slice, trailer.as_slice());
        prop_assert_eq!(combined.len() - slice.len(), expected.len());
    }

    /// Equal `Message<T>` values hash identically, so `Hash` agrees with
    /// `PartialEq` as required by the standard library contract.
    #[test]
    fn eq_implies_hash_eq(p in payload()) {
        let a = Message::new(p.clone());
        let b = Message::new(p);
        prop_assert_eq!(&a, &b);
        prop_assert_eq!(hash_of(&a), hash_of(&b));
    }

    /// `into_parts` returns exactly the inner value and cached bytes, matching
    /// what `message()` and `bytes()` would have returned.
    #[test]
    fn into_parts_matches_accessors(p in payload()) {
        let m = Message::new(p.clone());
        let expected_bytes = m.bytes().to_vec();
        let (inner, bytes) = m.into_parts();
        prop_assert_eq!(&*inner, &p);
        prop_assert_eq!(bytes.as_ref(), expected_bytes.as_slice());
    }
}
