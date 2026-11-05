use proptest::prelude::*;

use crate::tree::arb::arb_version;

use super::super::{HMAC_KEY_LEN, Prefix, height};
use super::*;
use crate::network::NetworkSecret;
use hmac::{Hmac, KeyInit, Mac};

proptest! {
    /// A normal leaf address is HMAC-SHA-256 of its canonical version bytes
    /// under the path key.
    ///
    /// The expected value is recomputed through the `hmac` crate's one-shot
    /// interface rather than the pre-keyed state the tree clones. The
    /// network's tests check that the key schedule derives the key itself.
    #[test]
    fn for_leaf_is_the_keyed_version_hash(
        version in arb_version(),
        raw_key in any::<[u8; HMAC_KEY_LEN]>(),
    ) {
        let expected: [u8; PATH_LEN] = <Hmac<sha2::Sha256> as KeyInit>::new_from_slice(&raw_key)
            .expect("HMAC accepts a key of any length")
            .chain_update(version.as_bytes())
            .finalize()
            .into_bytes()
            .into();
        let key = PathKey::new(&raw_key);
        prop_assert_eq!(<[u8; PATH_LEN]>::from(Path::for_leaf(&key, &version)), expected);
        // A cloned key derives the same path: the tree clones the pre-keyed
        // state for every path it derives.
        let cloned = key.clone();
        prop_assert_eq!(Path::for_leaf(&cloned, &version), Path::for_leaf(&key, &version));
    }

    /// Two different secrets place the same version at different paths, so
    /// independently seeded networks share no placement.
    ///
    /// The second secret is the first XOR a nonzero difference: a random
    /// mask whose byte at `at` is forced nonzero, so every case compares two
    /// distinct secrets, differing anywhere from one byte to all sixteen.
    #[test]
    fn different_secrets_place_a_version_differently(
        version in arb_version(),
        a in any::<[u8; 16]>(),
        mut difference in any::<[u8; 16]>(),
        at in 0..16usize,
        forced in 1..=u8::MAX,
    ) {
        difference[at] = forced;
        let b: [u8; 16] = std::array::from_fn(|i| a[i] ^ difference[i]);
        let path = |secret| Path::for_leaf(&NetworkSecret::from_bytes(secret).path_key(), &version);
        prop_assert_ne!(path(a), path(b));
    }

    /// Nested fixtures replace only their thread's mapping and restore it
    /// afterward; ordinary hashing resumes when the outer fixture ends.
    #[test]
    fn scoped_paths_restore_the_enclosing_mapping(
        version in arb_version(),
        outer in any::<[u8; 32]>(),
        inner in any::<[u8; 32]>(),
    ) {
        let hashed = Path::for_fixture_leaf(&version);
        Path::with_leaf_paths([(version.clone(), outer.into())], || {
            assert_eq!(Path::for_fixture_leaf(&version), outer.into());
            std::thread::scope(|scope| {
                scope.spawn(|| assert_eq!(Path::for_fixture_leaf(&version), hashed));
            });
            Path::with_leaf_paths([(version.clone(), inner.into())], || {
                assert_eq!(Path::for_fixture_leaf(&version), inner.into());
            });
            assert_eq!(Path::for_fixture_leaf(&version), outer.into());
        });
        prop_assert_eq!(Path::for_fixture_leaf(&version), hashed);
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
    let hashed = Path::for_fixture_leaf(&version);
    let result = std::panic::catch_unwind(|| {
        Path::with_leaf_paths([], || Path::for_fixture_leaf(&version));
    });
    assert!(result.is_err());
    assert_eq!(Path::for_fixture_leaf(&version), hashed);
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
