use proptest::prelude::*;

use crate::tree::arb::arb_version;

use super::*;

proptest! {
    /// `for_leaf` is exactly the *full-width* hash of the version's
    /// canonical bytes: `blake3(version)`, 32 bytes, no other input.
    ///
    /// Full width is what keeps a path collision at 2^128 birthday
    /// strength (a truncated Merkle-width hash would cap it lower), and
    /// the version's canonical bytes are the whole preimage: no message
    /// byte can steer where a leaf lands. This pin fails under either
    /// wrong reading.
    #[test]
    fn for_leaf_is_the_full_width_version_hash(version in arb_version()) {
        let expected: [u8; 32] = *blake3::hash(version.as_bytes()).as_bytes();
        let path = Path::for_leaf(&version);
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
