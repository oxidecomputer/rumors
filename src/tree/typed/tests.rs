use proptest::prelude::*;

use crate::tree::typed::height::{Height, Root, S, Z};
use crate::tree::typed::{Hash, Prefix, hash::MERKLE_HASH_LEN};
use crate::tree::wire;

proptest! {
    /// A `Hash` wire round-trips losslessly as exactly its
    /// `MERKLE_HASH_LEN` raw bytes.
    /// The trivial fixed-width case, pinned so a future encoding change to
    /// the wire codec surfaces here first.
    #[test]
    fn hash_wire_round_trip(bytes in any::<[u8; MERKLE_HASH_LEN]>()) {
        let original = Hash(bytes);
        let serialized = wire::to_vec(&original).unwrap();
        prop_assert_eq!(serialized.len(), MERKLE_HASH_LEN);
        let deserialized: Hash = wire::from_slice(&serialized).unwrap();
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
    wire::from_slice(&serialized).expect("known-valid prefix bytes")
}

/// `Prefix<H>` is encoded as exactly `32 - H::HEIGHT` raw bytes with no
/// length prefix. The wire length must match the type's height and round-
/// trips must be byte-identical.
macro_rules! prefix_roundtrip_test {
    ($name:ident, $height:ty) => {
        proptest! {
            /// The prefix's fixed-width wire form round-trips exactly at this height.
            #[test]
            fn $name(bytes in proptest::collection::vec(any::<u8>(), 32 - <$height>::HEIGHT)) {
                let prefix: Prefix<$height> = prefix_from_bytes(&bytes);
                let serialized = wire::to_vec(&prefix).unwrap();
                prop_assert_eq!(serialized.len(), 32 - <$height>::HEIGHT);
                prop_assert_eq!(serialized.as_slice(), bytes.as_slice());
                let deserialized: Prefix<$height> = wire::from_slice(&serialized).unwrap();
                prop_assert_eq!(prefix, deserialized);
            }
        }
    };
}

prefix_roundtrip_test!(prefix_wire_round_trip_z, Z);
prefix_roundtrip_test!(prefix_wire_round_trip_s_z, S<Z>);
prefix_roundtrip_test!(prefix_wire_round_trip_root, Root);

/// A `Prefix<Root>` is exactly zero bytes on the wire (the root has no
/// prefix). Pin the empty serialization so a future change to the encoding
/// surfaces here.
#[test]
fn prefix_root_serializes_to_empty() {
    let prefix = Prefix::<Root>::new();
    let serialized = wire::to_vec(&prefix).unwrap();
    assert!(serialized.is_empty());
}
