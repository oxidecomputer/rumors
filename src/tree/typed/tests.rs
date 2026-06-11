use borsh::BorshDeserialize;
use proptest::prelude::*;

use crate::tree::typed::height::{Height, Root, S, Z};
use crate::tree::typed::{Hash, Prefix};

proptest! {
    /// A `Hash` borsh round-trips losslessly as exactly its 32 raw bytes.
    /// The trivial fixed-width case, pinned so a future encoding change to
    /// the helper trait surfaces here first.
    #[test]
    fn hash_borsh_round_trip(bytes in any::<[u8; 32]>()) {
        let original = Hash(bytes);
        let serialized = borsh::to_vec(&original).unwrap();
        prop_assert_eq!(serialized.len(), 32);
        let deserialized = Hash::try_from_slice(&serialized).unwrap();
        prop_assert_eq!(original, deserialized);
    }
}

/// Test helper: construct a `Prefix<H>` directly from a byte buffer of the
/// exact length `H` demands. Mirrors the wire-format invariant; tests use
/// it instead of the public push/pop API so we can sweep all heights.
fn prefix_from_bytes<H: Height>(bytes: &[u8]) -> Prefix<H> {
    let expected_len = 32 - H::HEIGHT;
    assert_eq!(bytes.len(), expected_len);
    let serialized = bytes.to_vec();
    Prefix::<H>::try_from_slice(&serialized).expect("known-valid prefix bytes")
}

/// `Prefix<H>` is encoded as exactly `32 - H::HEIGHT` raw bytes with no
/// length prefix. The wire length must match the type's height and round-
/// trips must be byte-identical.
macro_rules! prefix_roundtrip_test {
    ($name:ident, $height:ty) => {
        proptest! {
            #[test]
            fn $name(bytes in proptest::collection::vec(any::<u8>(), 32 - <$height>::HEIGHT)) {
                let prefix: Prefix<$height> = prefix_from_bytes(&bytes);
                let serialized = borsh::to_vec(&prefix).unwrap();
                prop_assert_eq!(serialized.len(), 32 - <$height>::HEIGHT);
                prop_assert_eq!(serialized.as_slice(), bytes.as_slice());
                let deserialized = Prefix::<$height>::try_from_slice(&serialized).unwrap();
                prop_assert_eq!(prefix, deserialized);
            }
        }
    };
}

prefix_roundtrip_test!(prefix_borsh_round_trip_z, Z);
prefix_roundtrip_test!(prefix_borsh_round_trip_s_z, S<Z>);
prefix_roundtrip_test!(prefix_borsh_round_trip_root, Root);

/// A `Prefix<Root>` is exactly zero bytes on the wire (the root has no
/// prefix). Pin the empty serialization so a future change to the encoding
/// surfaces here.
#[test]
fn prefix_root_serializes_to_empty() {
    let prefix = Prefix::<Root>::new();
    let serialized = borsh::to_vec(&prefix).unwrap();
    assert!(serialized.is_empty());
}
