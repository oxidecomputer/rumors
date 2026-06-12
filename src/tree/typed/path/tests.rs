use bytes::Bytes;
use proptest::prelude::*;

use crate::tree::arb::arb_version;

use super::*;

proptest! {
    /// `for_leaf` commits to its `(version, value)` through *full-width*
    /// component hashes: the path is `blake3(blake3(version) ‖
    /// blake3(value))`, 32 bytes wide at every stage. Full width at every
    /// component is what keeps a path collision at 2^128 birthday strength;
    /// a truncated Merkle-width hash anywhere in the construction would cap
    /// the whole path at 2^64, and this pin fails under that wrong reading.
    #[test]
    fn for_leaf_components_are_full_width(
        version in arb_version(),
        value in any::<Vec<u8>>(),
    ) {
        let value = Bytes::from(value);
        let expected: [u8; 32] = {
            let mut buf = Vec::with_capacity(64);
            buf.extend_from_slice(blake3::hash(version.as_bytes()).as_bytes());
            buf.extend_from_slice(blake3::hash(value.as_ref()).as_bytes());
            *blake3::hash(&buf).as_bytes()
        };
        let path = Path::for_leaf(&version, &value);
        prop_assert_eq!(<[u8; 32]>::from(path), expected);
    }

    /// The first byte popped from a root-height path equals byte 0 of
    /// the underlying hash.
    #[test]
    fn path_pop_yields_first_byte(raw in any::<[u8; 32]>()) {
        let path = Path::<Root>::from(raw);
        let (byte, _) = path.pop();
        prop_assert_eq!(byte, raw[0]);
    }

    /// At root height, path equality is equivalent to full byte equality.
    #[test]
    fn path_eq_at_root_compares_all_bytes(
        a in any::<[u8; 32]>(),
        b in any::<[u8; 32]>(),
    ) {
        let pa = Path::<Root>::from(a);
        let pb = Path::<Root>::from(b);
        prop_assert_eq!(pa == pb, a == b);
    }

    /// After one pop, path equality ignores the consumed first byte.
    #[test]
    fn path_eq_after_pop_ignores_consumed_byte(
        a in any::<[u8; 32]>(),
        b in any::<[u8; 32]>(),
    ) {
        let (_, ra) = Path::<Root>::from(a).pop();
        let (_, rb) = Path::<Root>::from(b).pop();
        prop_assert_eq!(ra == rb, a[1..] == b[1..]);
    }

    /// Path ordering at root height matches byte-slice lexicographic ordering.
    #[test]
    fn path_ord_matches_byte_ordering(
        a in any::<[u8; 32]>(),
        b in any::<[u8; 32]>(),
    ) {
        let pa = Path::<Root>::from(a);
        let pb = Path::<Root>::from(b);
        prop_assert_eq!(pa.cmp(&pb), a.cmp(&b));
    }

    /// After one pop, path ordering ignores the consumed first byte.
    #[test]
    fn path_ord_after_pop_ignores_consumed_byte(
        a in any::<[u8; 32]>(),
        b in any::<[u8; 32]>(),
    ) {
        let (_, ra) = Path::<Root>::from(a).pop();
        let (_, rb) = Path::<Root>::from(b).pop();
        prop_assert_eq!(ra.cmp(&rb), a[1..].cmp(&b[1..]));
    }
}
