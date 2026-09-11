use crate::message::PayloadDepthLimit;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io;
use std::sync::Arc;

use bytes::Bytes;
use proptest::prelude::*;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use super::{Message, PayloadCodec, PayloadDecodeError, decode_exact, encode};

use serde::Serializer;

/// Build tree and capture fixtures without requiring a peer or admission checks.
impl Message {
    /// Cache a fixture payload's encoding, panicking if serialization fails.
    pub(crate) fn new<T: Serialize + Send + Sync + 'static>(message: T) -> Self {
        Self {
            serialized: encode(&message),
            message: Arc::new(message),
        }
    }

    /// Decode one fixture payload and copy its exact bytes into the cache.
    pub(crate) fn from_slice<T: DeserializeOwned + Send + Sync + 'static>(
        bytes: &[u8],
        limit: PayloadDepthLimit,
    ) -> io::Result<Self> {
        let message: T = decode_exact(bytes, limit).map_err(PayloadDecodeError::into_io)?;
        Ok(Self {
            message: Arc::new(message),
            serialized: Bytes::copy_from_slice(bytes),
        })
    }
}

/// Check the cache's original allocation without copying or shrinking it.
fn assert_exact_cache(message: Message) {
    let bytes = message.serialized;
    let pointer = bytes.as_ptr();
    let bytes = bytes.try_into_mut().expect("this cache has one owner");
    assert_eq!(bytes.as_ptr(), pointer);
    assert_eq!(
        bytes.capacity(),
        bytes.len(),
        "stored encoding has spare capacity"
    );
}

/// A small serde payload with varied field types, so proptests exercise
/// nontrivial serialization structure (nested containers, strings) rather
/// than only fixed-width primitives.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct Payload {
    id: u64,
    tag: String,
    data: Vec<u8>,
}

/// Generate payloads with varied scalar, string, and array encodings.
fn payload() -> impl Strategy<Value = Payload> {
    (any::<u64>(), any::<String>(), any::<Vec<u8>>()).prop_map(|(id, tag, data)| Payload {
        id,
        tag,
        data,
    })
}

/// Hash a value for the equality/hash consistency property.
fn hash_of<T: Hash>(value: &T) -> u64 {
    let mut h = DefaultHasher::new();
    value.hash(&mut h);
    h.finish()
}

/// Encode a payload independently for comparison with its stored bytes.
fn cbor_vec<T: Serialize>(value: &T) -> Vec<u8> {
    let mut buf = Vec::new();
    ciborium::ser::into_writer(value, &mut buf).unwrap();
    buf
}

proptest! {
    /// Locally allocated caches retain only the encoding's length, including
    /// payload sizes on either side of buffer growth boundaries.
    #[test]
    fn stored_encodings_have_no_spare_capacity(size in 0usize..4096, value in any::<u8>()) {
        let codec = PayloadCodec::new::<Vec<u8>>(PayloadDepthLimit::default());
        assert_exact_cache(codec.message(Arc::new(vec![value; size])).unwrap());
    }

    /// Admission caches the payload's CBOR encoding and preserves its value.
    #[test]
    fn admission_caches_cbor_serialization(p in payload()) {
        let codec = PayloadCodec::new::<Payload>(PayloadDepthLimit::default());
        let m = codec.message(Arc::new(p.clone())).unwrap();
        let direct = cbor_vec(&p);
        prop_assert_eq!(m.as_slice(), direct.as_slice());
        prop_assert_eq!(&*m.arc::<Payload>(), &p);
    }

    /// Wire ingress recovers the value and retains its exact encoding without
    /// copying the received bytes.
    #[test]
    fn wire_ingress_preserves_value_and_encoding(p in payload()) {
        let bytes = Bytes::from(cbor_vec(&p));
        let codec = PayloadCodec::new::<Payload>(PayloadDepthLimit::default());
        let m = Message::from_wire(bytes.clone(), codec).unwrap();
        prop_assert_eq!(&*m.arc::<Payload>(), &p);
        prop_assert_eq!(m.as_slice(), bytes.as_ref());
        prop_assert_eq!(m.as_slice().as_ptr(), bytes.as_ptr());
    }

    /// A payload followed by trailing bytes is rejected: the cache is
    /// always exactly one CBOR value's encoding, never a value plus noise.
    #[test]
    fn trailing_bytes_are_rejected(p in payload(), trailer in proptest::collection::vec(any::<u8>(), 1..8)) {
        let mut bytes = cbor_vec(&p);
        bytes.extend_from_slice(&trailer);
        let codec = PayloadCodec::new::<Payload>(PayloadDepthLimit::default());
        prop_assert!(Message::from_wire(Bytes::from(bytes), codec).is_err());
    }

    /// The serde form of a `Message` is one CBOR byte string wrapping
    /// the cached payload bytes — never a re-encoding of the payload — so
    /// nesting a message in a larger CBOR value costs one length header.
    #[test]
    fn serde_form_wraps_cached_bytes(p in payload()) {
        /// Encode borrowed bytes as a CBOR byte string.
        struct Bstr<'a>(&'a [u8]);
        /// Match the wire wrapper independently of `Message`'s serialization.
        impl Serialize for Bstr<'_> {
            /// Write the borrowed bytes as one byte string.
            fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.serialize_bytes(self.0)
            }
        }
        let m = Message::new(p);
        let wrapped = cbor_vec(&m);
        let direct = cbor_vec(&Bstr(m.as_slice()));
        prop_assert_eq!(wrapped, direct);
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

    /// Admission and typed reads share the caller's payload allocation.
    #[test]
    fn arc_shares_the_stored_allocation(p in payload()) {
        let stored = Arc::new(p);
        let codec = PayloadCodec::new::<Payload>(PayloadDepthLimit::default());
        let m = codec.message(stored.clone()).unwrap();
        prop_assert!(Arc::ptr_eq(&stored, &m.arc::<Payload>()));
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

/// Pure CBOR array nesting from a type satisfying the payload contract:
/// each layer serializes as a one-element array, the innermost empty.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Arr(Vec<Arr>);

/// A value of exactly `depth` nested array scopes (`depth` >= 1).
fn nested_arr(depth: u64) -> Arr {
    (1..depth).fold(Arr(vec![]), |a, _| Arr(vec![a]))
}

proptest! {
    /// Admission and wire ingress accept arrays at the configured depth and
    /// reject the same payload one step below it, including a zero limit.
    #[test]
    fn admission_and_ingress_share_depth_limit(depth in prop_oneof![
        1u64..33,
        Just(PayloadDepthLimit::default().get()),
        Just(PayloadDepthLimit::default().get() + 1),
    ]) {
        let value = Arc::new(nested_arr(depth));
        let bytes = Bytes::from(cbor_vec(&*value));
        let codec = PayloadCodec::new::<Arr>(PayloadDepthLimit::new(depth));
        let lower_limit = PayloadDepthLimit::new(depth - 1);
        let lower = codec.with_limit(lower_limit);
        let error = lower.message(value.clone()).unwrap_err();
        prop_assert!(matches!(error, super::EncodeError::Depth { limit } if limit == lower_limit), "unexpected admission error: {:?}", error);
        prop_assert_eq!(
            Message::from_wire(bytes.clone(), lower).unwrap_err().kind(),
            io::ErrorKind::InvalidData,
        );
        let admitted = codec.message(value.clone()).unwrap();
        let received = Message::from_wire(bytes.clone(), codec).unwrap();
        prop_assert_eq!(admitted.as_slice(), bytes.as_ref());
        prop_assert_eq!(&*received.arc::<Arr>(), &*value);
    }
}

/// A recursive enum whose decoder counts one step per map wrapper plus
/// one for the innermost unit variant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
enum E {
    /// The innermost unit variant.
    A,
    /// One more map wrapper.
    N(Box<E>),
}

/// `E::A` under `wrappers` layers of `E::N`.
fn nested_enum(wrappers: u64) -> E {
    (0..wrappers).fold(E::A, |e, _| E::N(Box::new(e)))
}

proptest! {
    /// Enum admission counts the decoder's unit-variant step as well as each
    /// map wrapper, and admitted bytes decode at the same receiver limit.
    #[test]
    fn enum_admission_counts_the_payloads_decode(wrappers in 0u64..16) {
        let limit = PayloadDepthLimit::new(wrappers + 1);
        let codec = PayloadCodec::new::<E>(limit);
        let value = Arc::new(nested_enum(wrappers));
        let admitted = codec.message(value.clone()).unwrap();
        let received = Message::from_wire(Bytes::copy_from_slice(admitted.as_slice()), codec).unwrap();
        prop_assert_eq!(&*received.arc::<E>(), &*value);
        let error = codec.message(Arc::new(nested_enum(wrappers + 1))).unwrap_err();
        prop_assert!(matches!(error, super::EncodeError::Depth { limit: actual } if actual == limit), "unexpected admission error: {:?}", error);
    }
}

/// A payload type violating the round-trip obligation: it serializes as
/// an integer but deserializes expecting text.
#[derive(Debug, PartialEq, Eq)]
struct Lopsided;

/// Encode a value in a form that its own decoder rejects.
impl Serialize for Lopsided {
    /// Write an integer instead of the text expected by the decoder.
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u64(0)
    }
}

/// Require text, contradicting this fixture's integer encoding.
impl<'de> serde::Deserialize<'de> for Lopsided {
    /// Accept only a text payload.
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(|_| Lopsided)
    }
}

/// A payload type whose `Deserialize` rejects its own `Serialize` output
/// is the typed `EncodeError::Roundtrip` at the author — the value would
/// have failed at every receiver, and admission is that decode.
#[test]
fn a_type_that_cannot_read_its_own_output_fails_admission() {
    let codec = PayloadCodec::new::<Lopsided>(PayloadDepthLimit::default());
    let error = codec.message(Arc::new(Lopsided)).unwrap_err();
    assert!(
        matches!(error, super::EncodeError::Roundtrip(_)),
        "the round-trip violation is its own typed case: {error:?}"
    );
}
