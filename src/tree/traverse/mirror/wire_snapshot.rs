//! Wire-format snapshot tests.
//!
//! Each type that crosses the protocol boundary is pinned here against an
//! `insta` snapshot of its borsh encoding. A drift means an interop break;
//! re-accept a snapshot only after a deliberate format change.

use borsh::BorshDeserialize;
use std::collections::BTreeMap;

use super::message;
use crate::tree::arb::nth_party;
use crate::tree::key::Key;
use crate::tree::typed::height::{Height, Root, S, Z};
use crate::tree::typed::{Hash, Path, Prefix};
use crate::{message::Message, version::Version};

/// Map a single-letter party label to its disjoint-party index (see
/// [`nth_party`]): `"a"` → 0, `"b"` → 1, and so on.
fn party_index(label: &str) -> usize {
    (label.bytes().next().unwrap_or(b'a').to_ascii_lowercase() - b'a') as usize
}

/// The [`Version`] reached by ticking `label`'s disjoint party `ticks` times.
/// Replaces the old `Version::from((party, scalar))` vector constructor.
fn ticked(label: &str, ticks: u64) -> Version {
    let p = nth_party(party_index(label));
    let mut v = Version::new();
    for _ in 0..ticks {
        v.tick(&p);
    }
    v
}

fn hex_dump(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::from("(empty)");
    }
    let mut s = String::with_capacity(bytes.len() * 3);
    for (i, chunk) in bytes.chunks(16).enumerate() {
        if i > 0 {
            s.push('\n');
        }
        s.push_str(&format!("{:04x}:", i * 16));
        for byte in chunk {
            s.push_str(&format!(" {:02x}", byte));
        }
    }
    s
}

fn snap<T: borsh::BorshSerialize>(value: &T) -> String {
    hex_dump(&borsh::to_vec(value).unwrap())
}

fn prefix_from_bytes<H: Height>(bytes: &[u8]) -> Prefix<H> {
    assert_eq!(bytes.len(), 32 - H::HEIGHT);
    Prefix::<H>::try_from_slice(bytes).expect("known-valid prefix bytes")
}

/// A canonical `providing` leaf list: `(key, version, ())` triples placed at
/// their content-addressed paths, in strictly ascending key order. The key is
/// the leaf's path, transmitted so the receiver need not re-hash.
fn providing_leaves(versions: &[Version]) -> Vec<(Key, Version, Message<()>)> {
    let mut by_path: BTreeMap<[u8; 32], (Key, Version, Message<()>)> = BTreeMap::new();
    for version in versions {
        let message = Message::new(());
        let path: [u8; 32] = Path::<Root>::for_leaf(version, message.bytes()).into();
        by_path.insert(
            path,
            (
                Key::from(Path::<Root>::from(path)),
                version.clone(),
                message,
            ),
        );
    }
    by_path.into_values().collect()
}

// ---------- Hash ----------

#[test]
fn hash_zeros() {
    insta::assert_snapshot!(snap(&Hash([0u8; 32])));
}

#[test]
fn hash_ones() {
    insta::assert_snapshot!(snap(&Hash([0xffu8; 32])));
}

#[test]
fn hash_sequential() {
    let bytes: [u8; 32] = std::array::from_fn(|i| i as u8);
    insta::assert_snapshot!(snap(&Hash(bytes)));
}

// ---------- Prefix ----------

#[test]
fn prefix_root_empty() {
    insta::assert_snapshot!(snap(&Prefix::<Root>::new()));
}

#[test]
fn prefix_under_root_single_byte() {
    insta::assert_snapshot!(snap(&prefix_from_bytes::<message::UnderRoot>(&[0x42])));
}

#[test]
fn prefix_s_s_z() {
    let bytes: Vec<u8> = (0u8..30).collect();
    insta::assert_snapshot!(snap(&prefix_from_bytes::<S<S<Z>>>(&bytes)));
}

#[test]
fn prefix_s_z_max_byte() {
    let mut bytes = vec![0u8; 31];
    *bytes.last_mut().unwrap() = 0xff;
    insta::assert_snapshot!(snap(&prefix_from_bytes::<S<Z>>(&bytes)));
}

#[test]
fn prefix_z_full_32_bytes() {
    let bytes: [u8; 32] = std::array::from_fn(|i| i as u8);
    insta::assert_snapshot!(snap(&prefix_from_bytes::<Z>(&bytes)));
}

// ---------- Version ----------

#[test]
fn version_empty() {
    insta::assert_snapshot!(snap(&Version::new()));
}

#[test]
fn version_two_parties_ascending() {
    let v: Version = ticked("a", 1) | ticked("b", 2);
    insta::assert_snapshot!(snap(&v));
}

// ---------- Messages ----------

#[test]
fn message_initiate_empty() {
    insta::assert_snapshot!(snap(&message::Initiate::default()));
}

#[test]
fn message_initiate_one_entry() {
    let uncertain = vec![(Prefix::<Root>::new(), Hash([1u8; 32]))];
    insta::assert_snapshot!(snap(&message::Initiate { uncertain }));
}

#[test]
fn message_opening_empty() {
    insta::assert_snapshot!(snap(&message::Opening::default()));
}

#[test]
fn message_opening_one_entry() {
    let uncertain = vec![(
        prefix_from_bytes::<message::UnderRoot>(&[0x42]),
        Hash([2u8; 32]),
    )];
    insta::assert_snapshot!(snap(&message::Opening { uncertain }));
}

#[test]
fn message_exchange_empty() {
    let m: message::Exchange<(), message::UnderRoot> = message::Exchange::default();
    insta::assert_snapshot!(snap(&m));
}

#[test]
fn message_exchange_populated() {
    // `providing` is a leaf list (paths elided), plus an ascending `requested`
    // and `uncertain`.
    let providing = providing_leaves(&[ticked("a", 1), ticked("a", 2)]);
    let requested = vec![Prefix::<Root>::new()];
    let uncertain = vec![(
        prefix_from_bytes::<message::UnderRoot>(&[0xcc]),
        Hash([3u8; 32]),
    )];

    let m: message::Exchange<(), message::UnderRoot> = message::Exchange {
        providing,
        requested,
        uncertain,
    };
    insta::assert_snapshot!(snap(&m));
}

#[test]
fn message_closing_empty() {
    let m: message::Closing<()> = message::Closing::default();
    insta::assert_snapshot!(snap(&m));
}

#[test]
fn message_closing_populated() {
    let providing = providing_leaves(&[ticked("a", 1)]);
    let requested = vec![prefix_from_bytes::<S<Z>>(&[0xffu8; 31])];
    let m: message::Closing<()> = message::Closing {
        providing,
        requested,
    };
    insta::assert_snapshot!(snap(&m));
}

#[test]
fn message_complete_empty() {
    let m: message::Complete<()> = message::Complete::default();
    insta::assert_snapshot!(snap(&m));
}

#[test]
fn message_complete_populated() {
    let providing = providing_leaves(&[ticked("a", 1)]);
    let m: message::Complete<()> = message::Complete { providing };
    insta::assert_snapshot!(snap(&m));
}
