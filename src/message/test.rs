use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use borsh::{BorshDeserialize, BorshSerialize};
use bytes::Bytes;
use proptest::prelude::*;

use super::Message;

/// A small borsh-serializable payload with varied field types, so proptests
/// exercise nontrivial serialization structure (length prefixes, nested
/// vectors) rather than only fixed-width primitives.
#[derive(Clone, Debug, PartialEq, Eq, Hash, BorshSerialize, BorshDeserialize)]
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

proptest! {
    /// After construction via `new`, the cached serialized bytes are exactly
    /// what borsh would produce for the inner value.
    #[test]
    fn new_caches_borsh_serialization(p in payload()) {
        let m = Message::new(p.clone());
        let direct = borsh::to_vec(&p).unwrap();
        prop_assert_eq!(m.bytes(), direct.as_slice());
        prop_assert_eq!(m.message(), &p);
    }

    /// `from_slice` reconstructs the inner value and stores exactly the input
    /// bytes in the cache, with no reserialization drift.
    #[test]
    fn from_slice_roundtrips(p in payload()) {
        let bytes = borsh::to_vec(&p).unwrap();
        let m = Message::<Payload>::from_slice(&bytes).unwrap();
        prop_assert_eq!(m.message(), &p);
        prop_assert_eq!(m.bytes(), bytes.as_slice());
    }

    /// `from_bytes` (zero-copy) and `from_slice` (copying) produce equivalent
    /// `Message`s from the same input.
    #[test]
    fn from_bytes_matches_from_slice(p in payload()) {
        let bytes = borsh::to_vec(&p).unwrap();
        let a = Message::<Payload>::from_slice(&bytes).unwrap();
        let b = Message::<Payload>::from_bytes(Bytes::from(bytes.clone())).unwrap();
        prop_assert_eq!(&a, &b);
        prop_assert_eq!(a.bytes(), b.bytes());
    }

    /// `BorshSerialize` on a `Message<T>` writes exactly the cached bytes, so
    /// serializing a `Message<T>` is indistinguishable from serializing `T`.
    #[test]
    fn serialize_writes_cached_bytes(p in payload()) {
        let m = Message::new(p.clone());
        let reserialized = borsh::to_vec(&m).unwrap();
        prop_assert_eq!(reserialized.as_slice(), m.bytes());
        prop_assert_eq!(reserialized, borsh::to_vec(&p).unwrap());
    }

    /// A `Message<T>` roundtrips through borsh: deserializing a serialized
    /// message yields an equal message with equal cached bytes.
    #[test]
    fn borsh_roundtrip(p in payload()) {
        let m = Message::new(p);
        let bytes = borsh::to_vec(&m).unwrap();
        let back: Message<Payload> = borsh::from_slice(&bytes).unwrap();
        prop_assert_eq!(&m, &back);
        prop_assert_eq!(m.bytes(), back.bytes());
    }

    /// `BorshDeserialize` captures only the bytes actually consumed by `T`:
    /// when a `Message<T>` is embedded alongside trailing data, the cached
    /// bytes match `T`'s serialization and the trailing data survives.
    #[test]
    fn deserialize_captures_only_message_bytes(p in payload(), trailer in any::<Vec<u8>>()) {
        let expected = borsh::to_vec(&p).unwrap();
        let mut combined = expected.clone();
        combined.extend_from_slice(&trailer);

        let mut slice: &[u8] = &combined;
        let m = Message::<Payload>::deserialize_reader(&mut slice).unwrap();
        prop_assert_eq!(m.bytes(), expected.as_slice());
        prop_assert_eq!(slice, trailer.as_slice());
    }

    /// `Message<T>` nests correctly inside other borsh types: a `Vec<Message<T>>`
    /// roundtrips and preserves each element's cached bytes.
    #[test]
    fn nested_in_vec_roundtrips(ps in proptest::collection::vec(payload(), 0..8)) {
        let msgs: Vec<Message<Payload>> =
            ps.into_iter().map(|p| Message::new(p)).collect();
        let bytes = borsh::to_vec(&msgs).unwrap();
        let back: Vec<Message<Payload>> = borsh::from_slice(&bytes).unwrap();
        prop_assert_eq!(&msgs, &back);
        for (a, b) in msgs.iter().zip(back.iter()) {
            prop_assert_eq!(a.bytes(), b.bytes());
        }
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
