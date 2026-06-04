//! Wire-format snapshot tests.
//!
//! Each type that crosses the protocol boundary is pinned here against an
//! `insta` snapshot of its borsh encoding. A drift means an interop break;
//! re-accept a snapshot only after a deliberate format change.

use borsh::BorshDeserialize;
use imbl::{OrdMap, OrdSet};

use super::message;
use crate::tree::arb::nth_party;
use crate::tree::typed::height::{Height, Root, S, Z};
use crate::tree::typed::{Hash, Node, Prefix};
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

fn leaf(party: &str, version: u64) -> Node<(), Z> {
    Node::leaf(ticked(party, version), Message::new(()))
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

// ---------- Node<T, Z>: leaf ----------

#[test]
fn node_z_leaf() {
    insta::assert_snapshot!(snap(&leaf("a", 1)));
}

#[test]
fn node_z_leaf_empty_version() {
    let l: Node<(), Z> = Node::leaf(Version::default(), Message::new(()));
    insta::assert_snapshot!(snap(&l));
}

// ---------- Node<T, S<Z>> ----------

#[test]
fn node_s_z_singleton_path_compressed_leaf() {
    let n: Node<(), S<Z>> = Node::beneath(leaf("a", 1), 0xab);
    insta::assert_snapshot!(snap(&n));
}

#[test]
fn node_s_z_two_child_branch() {
    let mut children: OrdMap<u8, Node<(), Z>> = OrdMap::new();
    children.insert(0x00, leaf("a", 1));
    children.insert(0xff, leaf("a", 2));
    let n = Node::<(), S<Z>>::branch(children).unwrap();
    insta::assert_snapshot!(snap(&n));
}

#[test]
fn node_s_z_full_256_child_branch() {
    let mut children: OrdMap<u8, Node<(), Z>> = OrdMap::new();
    for i in 0u16..=255 {
        children.insert(i as u8, leaf("a", i as u64 + 1));
    }
    let n = Node::<(), S<Z>>::branch(children).unwrap();
    insta::assert_snapshot!(snap(&n));
}

// ---------- Node<T, Root> ----------

#[test]
fn node_root_none() {
    let n: Option<Node<(), Root>> = None;
    insta::assert_snapshot!(snap(&n));
}

#[test]
fn node_root_single_leaf_full_compression() {
    let n = leaf("a", 1);
    seq_macro::seq!(I in 0..32 {
        let n = Node::beneath(n, I);
    });
    let n: Node<(), Root> = n;
    insta::assert_snapshot!(snap(&n));
}

#[test]
fn node_root_two_leaves_branched_at_root() {
    let n = {
        let l0 = leaf("a", 1);
        let l1 = leaf("a", 2);
        let n0 = {
            let n = l0;
            seq_macro::seq!(I in 0..31 {
                let n = Node::beneath(n, I);
            });
            n
        };
        let n1 = {
            let n = l1;
            seq_macro::seq!(I in 0..31 {
                let n = Node::beneath(n, I);
            });
            n
        };
        let mut children: OrdMap<u8, Node<(), _>> = OrdMap::new();
        children.insert(0x01, n0);
        children.insert(0x02, n1);
        Node::<(), Root>::branch(children).unwrap()
    };
    insta::assert_snapshot!(snap(&n));
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
    let mut uncertain = OrdMap::new();
    uncertain.insert(Prefix::<Root>::new(), Hash([1u8; 32]));
    insta::assert_snapshot!(snap(&message::Initiate { uncertain }));
}

#[test]
fn message_opening_empty() {
    insta::assert_snapshot!(snap(&message::Opening::default()));
}

#[test]
fn message_opening_one_entry() {
    let mut uncertain = OrdMap::new();
    uncertain.insert(
        prefix_from_bytes::<message::UnderRoot>(&[0x42]),
        Hash([2u8; 32]),
    );
    insta::assert_snapshot!(snap(&message::Opening { uncertain }));
}

#[test]
fn message_exchange_empty() {
    let m: message::Exchange<(), message::UnderRoot> = message::Exchange::default();
    insta::assert_snapshot!(snap(&m));
}

#[test]
fn message_exchange_populated() {
    let leaf_z: Node<(), Z> = leaf("a", 1);
    let inner: Node<(), S<Z>> = Node::beneath(leaf_z, 0xab);
    let mut other_children: OrdMap<u8, Node<(), S<Z>>> = OrdMap::new();
    other_children.insert(0x01, inner.clone());
    other_children.insert(0x02, inner.clone());
    let s_s_z = Node::<(), S<S<Z>>>::branch(other_children).unwrap();
    let n_root: Node<(), Root> = {
        let n = s_s_z;
        seq_macro::seq!(I in 0..30 {
            let n = Node::beneath(n, I);
        });
        n
    };

    let mut providing: OrdMap<Prefix<Root>, Node<(), Root>> = OrdMap::new();
    providing.insert(Prefix::<Root>::new(), n_root);

    let mut requested: OrdSet<Prefix<Root>> = OrdSet::new();
    requested.insert(Prefix::<Root>::new());

    let mut uncertain: OrdMap<Prefix<message::UnderRoot>, Hash> = OrdMap::new();
    uncertain.insert(
        prefix_from_bytes::<message::UnderRoot>(&[0xcc]),
        Hash([3u8; 32]),
    );

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
    let n_s_z: Node<(), S<Z>> = Node::beneath(leaf("a", 1), 0xab);
    let mut providing: OrdMap<Prefix<S<Z>>, Node<(), S<Z>>> = OrdMap::new();
    providing.insert(prefix_from_bytes::<S<Z>>(&[0u8; 31]), n_s_z);
    let mut requested: OrdSet<Prefix<S<Z>>> = OrdSet::new();
    requested.insert(prefix_from_bytes::<S<Z>>(&[0xffu8; 31]));
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
    let mut providing: OrdMap<Prefix<Z>, Node<(), Z>> = OrdMap::new();
    providing.insert(prefix_from_bytes::<Z>(&[0u8; 32]), leaf("a", 1));
    let m: message::Complete<()> = message::Complete { providing };
    insta::assert_snapshot!(snap(&m));
}
