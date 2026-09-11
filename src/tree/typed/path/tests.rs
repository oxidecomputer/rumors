use proptest::prelude::*;

use crate::tree::arb::arb_version;

use super::super::{Prefix, height};
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

    /// At every height, comparisons and debug output use only the active
    /// suffix or prefix; typed and erased descent move the same bytes.
    #[test]
    fn path_and_prefix_geometry_agree_at_every_height(
        a in any::<[u8; PATH_LEN]>(),
        b in any::<[u8; PATH_LEN]>(),
    ) {
        seq_macro::seq!(N in 0..=32 {
            #(check_geometry::<height::H~N>(a, b)?;)*
        });
        seq_macro::seq!(N in 0..32 {
            #(check_step::<height::H~N>(a, b)?;)*
        });
        let prefix = Prefix::from(a);
        prop_assert_eq!(<[u8; PATH_LEN]>::from(prefix), a);
        prop_assert_eq!(<[u8; PATH_LEN]>::from(Path::from(prefix)), a);
        prop_assert_eq!(Prefix::from(Path::from(a)), prefix);
    }

    /// Restoring an erased prefix at a different height panics in every build profile.
    #[test]
    fn erased_prefix_rejects_a_different_height(
        actual in 0usize..=PATH_LEN,
        offset in 1usize..=PATH_LEN,
    ) {
        let mut prefix = Prefix::from([0; PATH_LEN]).erase();
        for _ in 0..actual {
            prefix = prefix.pop().0;
        }
        let claimed = (actual + offset) % (PATH_LEN + 1);
        seq_macro::seq!(N in 0..=32 {
            match claimed {
                #(N => prop_assert!(std::panic::catch_unwind(
                    || prefix.assume::<height::H~N>()
                ).is_err()),)*
                _ => unreachable!("the claimed height is within the path length"),
            }
        });
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

/// Compare typed views against slices, including differences outside each view.
fn check_geometry<H: Height>(
    a: [u8; PATH_LEN],
    b: [u8; PATH_LEN],
) -> proptest::test_runner::TestCaseResult {
    let depth = PATH_LEN - H::HEIGHT;
    let path_a = Path::<H> {
        height: PhantomData,
        hash: a,
    };
    let path_b = Path::<H> {
        height: PhantomData,
        hash: b,
    };
    prop_assert_eq!(path_a == path_b, a[depth..] == b[depth..]);
    prop_assert_eq!(path_a.cmp(&path_b), a[depth..].cmp(&b[depth..]));
    prop_assert_eq!(format!("{path_a:?}"), format!("{:?}", &a[depth..]));

    // Random full addresses seldom have equal suffixes. Force equality
    // after the consumed prefix to exercise that side of the contract.
    let mut same_suffix = b;
    same_suffix[depth..].copy_from_slice(&a[depth..]);
    let same_suffix = Path::<H> {
        height: PhantomData,
        hash: same_suffix,
    };
    prop_assert_eq!(path_a, same_suffix);
    prop_assert_eq!(path_a.cmp(&same_suffix), std::cmp::Ordering::Equal);
    prop_assert_eq!(format!("{path_a:?}"), format!("{same_suffix:?}"));

    let prefix_a = Prefix::<H>::containing(&Path::from(a));
    let prefix_b = Prefix::<H>::containing(&Path::from(b));
    prop_assert_eq!(prefix_a.as_bytes(), &a[..depth]);
    prop_assert_eq!(prefix_a == prefix_b, a[..depth] == b[..depth]);
    prop_assert_eq!(prefix_a.cmp(&prefix_b), a[..depth].cmp(&b[..depth]));
    prop_assert_eq!(format!("{prefix_a:?}"), format!("{:?}", &a[..depth]));

    // containing() can keep unused array bytes behind its logical length.
    // They must not influence equality, ordering, or the displayed prefix.
    let mut same_prefix = b;
    same_prefix[..depth].copy_from_slice(&a[..depth]);
    let same_prefix = Prefix::<H>::containing(&Path::from(same_prefix));
    prop_assert_eq!(prefix_a, same_prefix);
    prop_assert_eq!(prefix_a.cmp(&same_prefix), std::cmp::Ordering::Equal);
    prop_assert_eq!(format!("{prefix_a:?}"), format!("{same_prefix:?}"));

    let erased = prefix_a.erase();
    prop_assert_eq!(erased.height(), H::HEIGHT);
    prop_assert_eq!(erased.as_bytes(), prefix_a.as_bytes());
    prop_assert_eq!(erased.assume::<H>(), prefix_a);
    prop_assert_eq!(format!("{erased:?}"), format!("{prefix_a:?}"));
    Ok(())
}

/// Pop a path byte and round-trip typed and erased prefixes at one level.
fn check_step<H: Height>(
    raw: [u8; PATH_LEN],
    other: [u8; PATH_LEN],
) -> proptest::test_runner::TestCaseResult
where
    S<H>: Height,
{
    let depth = PATH_LEN - S::<H>::HEIGHT;
    let path = Path::<S<H>> {
        height: PhantomData,
        hash: raw,
    };
    let (byte, rest) = path.pop();
    prop_assert_eq!(byte, raw[depth]);
    prop_assert_eq!(rest.as_bytes(), &raw[depth + 1..]);

    // Push a byte independent of the stored array's next byte, so a
    // constructor that retains unused bytes must still overwrite that slot.
    let byte = other[depth];
    let prefix = Prefix::<S<H>>::containing(&Path::from(raw));
    let mut extended = raw;
    extended[depth] = byte;
    let child = prefix.push(byte);
    prop_assert_eq!(child, Prefix::<H>::containing(&Path::from(extended)));
    prop_assert_eq!(child.pop(), (prefix, byte));
    let erased = prefix.erase().push(byte);
    prop_assert_eq!(erased, child.erase());
    prop_assert_eq!(erased.pop(), (prefix.erase(), byte));
    Ok(())
}
