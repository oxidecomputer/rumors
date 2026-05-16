use borsh::BorshDeserialize;
use proptest::prelude::*;

use crate::tree::arb::arb_root_tree;
use crate::tree::typed::height::{Height, Root, S, Z};
use crate::tree::typed::{Hash, Node, Prefix};

// `Hash` is a fixed-width newtype; its borsh round-trip is the trivial
// case but worth pinning so a future encoding change to the helper trait
// surfaces here first.
proptest! {
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

// Arbitrary trees round-trip through borsh: deserialize-then-serialize
// returns to the original `Option<Node<...>>`, with identical `hash()`
// and structural `Eq`. Also pin the in-memory path-compression layout:
// the underlying prefix-byte count must survive the round-trip so a
// decoder that mis-distributes prefix bytes between parent and child
// (producing a logically-equivalent but structurally-different node)
// would fail here.
proptest! {
    #[test]
    fn typed_root_node_borsh_round_trip(node in arb_root_tree("p", 0..=8)) {
        let serialized = borsh::to_vec(&node).unwrap();
        let deserialized: Option<Node<String, (), Root>> =
            Option::try_from_slice(&serialized).unwrap();
        let hash_before = Node::root_hash(&node);
        let hash_after = Node::root_hash(&deserialized);
        prop_assert_eq!(hash_before, hash_after);
        prop_assert_eq!(
            node.as_ref().map(|n| n.compressed_prefix_len()),
            deserialized.as_ref().map(|n| n.compressed_prefix_len()),
        );
        prop_assert_eq!(node, deserialized);
    }
}

// ---- Negative tests: each crafts a wire payload that the decoder must
// reject, and asserts the specific error path. These pin the rejection
// points so a refactor that silently accepts malformed wires would fail
// here. ----

/// A `Node<P, T, Z>` with `prefix_len > 0` is structurally impossible
/// (leaves never carry a prefix); the decoder must reject it rather
/// than absorb the bytes as a leaf body.
#[test]
fn node_z_rejects_nonzero_prefix_len() {
    let mut wire = vec![0x01]; // prefix_len = 1
    wire.push(0xab); // pretend head byte
    wire.extend_from_slice(&[0u8; 4]); // empty version map (u32 len = 0)
    let err = Node::<String, (), Z>::try_from_slice(&wire).unwrap_err();
    assert!(
        err.to_string()
            .contains("leaf height cannot carry a prefix"),
        "unexpected error: {err}",
    );
}

/// A `Node<P, T, S<Z>>` with `prefix_len > S<Z>::HEIGHT` is past the
/// height-derived maximum and must be rejected before any further
/// reads.
#[test]
fn node_s_z_rejects_oversized_prefix_len() {
    // S<Z>::HEIGHT == 1, so prefix_len = 2 is out of range.
    let wire = vec![0x02];
    let err = Node::<String, (), S<Z>>::try_from_slice(&wire).unwrap_err();
    assert!(
        err.to_string()
            .contains("prefix length exceeds typed height"),
        "unexpected error: {err}",
    );
}

/// Branch children must appear in strictly-ascending radix order.
#[test]
fn node_s_z_rejects_non_ascending_radices() {
    // S<Z> branch, prefix_len=0, count_minus_two=0 (count=2), radices 5 then 3.
    let mut wire = vec![0x00, 0x00, 0x05];
    // First child (Z leaf): prefix_len=0, empty version, empty message.
    wire.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00]);
    wire.push(0x03);
    wire.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00]);
    let err = Node::<String, (), S<Z>>::try_from_slice(&wire).unwrap_err();
    assert!(
        err.to_string().contains("strictly ascending"),
        "unexpected error: {err}",
    );
}

/// Two children at the same radix is also rejected (the strict-ascending
/// rule subsumes the no-duplicates rule).
#[test]
fn node_s_z_rejects_duplicate_radix() {
    let mut wire = vec![0x00, 0x00, 0x05];
    wire.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00]);
    wire.push(0x05);
    wire.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00]);
    let err = Node::<String, (), S<Z>>::try_from_slice(&wire).unwrap_err();
    assert!(
        err.to_string().contains("strictly ascending"),
        "unexpected error: {err}",
    );
}

/// `count_minus_two = 0xff` overflows the 256-child maximum (`count = 257`).
#[test]
fn node_s_z_rejects_overflow_count() {
    let wire = vec![0x00, 0xff];
    let err = Node::<String, (), S<Z>>::try_from_slice(&wire).unwrap_err();
    assert!(
        err.to_string().contains("exceeds 256"),
        "unexpected error: {err}",
    );
}
