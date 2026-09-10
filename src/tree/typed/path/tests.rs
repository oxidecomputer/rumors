use proptest::prelude::*;

use crate::tree::arb::arb_version;

use super::*;
use sha3::Digest;

proptest! {
    /// A normal leaf address is SHA3-256 of its canonical version bytes.
    #[test]
    fn for_leaf_is_the_full_width_version_hash(version in arb_version()) {
        let expected: [u8; 32] = sha3::Sha3_256::digest(version.as_bytes()).into();
        let path = Path::for_leaf(&version);
        prop_assert_eq!(<[u8; 32]>::from(path), expected);
    }

    /// Nested fixtures replace only their thread's mapping and restore it
    /// afterward; ordinary hashing resumes when the outer fixture ends.
    #[test]
    fn scoped_paths_restore_the_enclosing_mapping(
        version in arb_version(),
        outer in any::<[u8; 32]>(),
        inner in any::<[u8; 32]>(),
    ) {
        let hashed = Path::for_leaf(&version);
        Path::with_leaf_paths([(version.clone(), outer.into())], || {
            assert_eq!(Path::for_leaf(&version), outer.into());
            std::thread::scope(|scope| {
                scope.spawn(|| assert_eq!(Path::for_leaf(&version), hashed));
            });
            Path::with_leaf_paths([(version.clone(), inner.into())], || {
                assert_eq!(Path::for_leaf(&version), inner.into());
            });
            assert_eq!(Path::for_leaf(&version), outer.into());
        });
        prop_assert_eq!(Path::for_leaf(&version), hashed);
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

/// An incomplete fixture fails at lookup and restores normal hashing on unwind.
#[test]
fn missing_fixture_version_restores_hashing_after_panic() {
    let version = Version::new();
    let hashed = Path::for_leaf(&version);
    let result = std::panic::catch_unwind(|| {
        Path::with_leaf_paths([], || Path::for_leaf(&version));
    });
    assert!(result.is_err());
    assert_eq!(Path::for_leaf(&version), hashed);
}

/// A fixture cannot merge distinct leaf identities or assign one version twice.
#[test]
fn fixture_paths_and_versions_must_be_distinct() {
    let first = Version::new();
    let mut second = first.clone();
    second.tick(&crate::tree::arb::nth_party(0));
    let path = Path::from([0; 32]);
    for mapping in [
        [(first.clone(), path), (second, path)],
        [(first.clone(), path), (first, [1; 32].into())],
    ] {
        assert!(std::panic::catch_unwind(|| Path::with_leaf_paths(mapping, || ())).is_err());
    }
}
