//! Wire-format snapshot tests.
//!
//! Each type that crosses the protocol boundary is pinned here against an
//! `insta` snapshot of its borsh encoding. A drift means an interop break;
//! re-accept a snapshot only after a deliberate format change.

use borsh::BorshDeserialize;

use super::message;
use crate::tree::arb::nth_party;
use crate::tree::typed::height::{Height, Root, S, UnderRoot, Z};
use crate::tree::typed::{Children, Hash, Node, Prefix, hash::MERKLE_HASH_LEN};
use crate::{Version, message::Message};

/// Map a single-letter party label to its disjoint-party index (see
/// [`nth_party`]): `"a"` → 0, `"b"` → 1, and so on.
fn party_index(label: &str) -> usize {
    (label.bytes().next().unwrap_or(b'a').to_ascii_lowercase() - b'a') as usize
}

/// The [`Version`] reached by ticking `label`'s disjoint party `ticks`
/// times.
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

/// A `Hash`'s encoding is its 16 raw bytes, no length prefix: all-zero case.
#[test]
fn hash_zeros() {
    insta::assert_snapshot!(snap(&Hash([0u8; MERKLE_HASH_LEN])));
}

/// All-ones `Hash`: every byte value survives encoding unmangled.
#[test]
fn hash_ones() {
    insta::assert_snapshot!(snap(&Hash([0xffu8; MERKLE_HASH_LEN])));
}

/// Sequential-byte `Hash`: byte order on the wire is array order.
#[test]
fn hash_sequential() {
    let bytes: [u8; MERKLE_HASH_LEN] = std::array::from_fn(|i| i as u8);
    insta::assert_snapshot!(snap(&Hash(bytes)));
}

// ---------- Prefix ----------

/// The root `Prefix` (zero bytes consumed) encodes as the empty string.
#[test]
fn prefix_root_empty() {
    insta::assert_snapshot!(snap(&Prefix::<Root>::new()));
}

/// A one-byte `Prefix` (height `UnderRoot`): the length is implied by the
/// height, so exactly one raw byte crosses the wire.
#[test]
fn prefix_under_root_single_byte() {
    insta::assert_snapshot!(snap(&prefix_from_bytes::<UnderRoot>(&[0x42])));
}

/// A 30-byte `Prefix` (height `S<S<Z>>`): the height-implied length scales
/// to deep prefixes.
#[test]
fn prefix_s_s_z() {
    let bytes: Vec<u8> = (0u8..30).collect();
    insta::assert_snapshot!(snap(&prefix_from_bytes::<S<S<Z>>>(&bytes)));
}

/// A 31-byte `Prefix` ending in `0xff`: the high byte value is preserved
/// at the boundary position.
#[test]
fn prefix_s_z_max_byte() {
    let mut bytes = vec![0u8; 31];
    *bytes.last_mut().unwrap() = 0xff;
    insta::assert_snapshot!(snap(&prefix_from_bytes::<S<Z>>(&bytes)));
}

/// The full 32-byte `Prefix` (height `Z`): a complete leaf path.
#[test]
fn prefix_z_full_32_bytes() {
    let bytes: [u8; 32] = std::array::from_fn(|i| i as u8);
    insta::assert_snapshot!(snap(&prefix_from_bytes::<Z>(&bytes)));
}

// ---------- Node<T, Z>: leaf ----------

/// A bare leaf node: version then message payload.
#[test]
fn node_z_leaf() {
    insta::assert_snapshot!(snap(&leaf("a", 1)));
}

/// A leaf at the empty `Version`: the degenerate-timestamp encoding.
#[test]
fn node_z_leaf_empty_version() {
    let l: Node<(), Z> = Node::leaf(Version::default(), Message::new(()));
    insta::assert_snapshot!(snap(&l));
}

// ---------- Node<T, S<Z>> ----------

/// A path-compressed single child: the compressed prefix byte rides the
/// node, not a materialized intermediate level.
#[test]
fn node_s_z_singleton_path_compressed_leaf() {
    let n: Node<(), S<Z>> = Node::beneath(leaf("a", 1), 0xab);
    insta::assert_snapshot!(snap(&n));
}

/// A two-child branch at the radix extremes (`0x00`, `0xff`), in
/// ascending-radix order.
#[test]
fn node_s_z_two_child_branch() {
    let children: Children<(), Z> = [(0x00, leaf("a", 1)), (0xff, leaf("a", 2))]
        .into_iter()
        .collect();
    let n = Node::<(), S<Z>>::branch(children).unwrap();
    insta::assert_snapshot!(snap(&n));
}

/// The saturated 256-child branch: the maximum fan-out boundary.
#[test]
fn node_s_z_full_256_child_branch() {
    let children: Children<(), Z> = (0u16..=255)
        .map(|i| (i as u8, leaf("a", i as u64 + 1)))
        .collect();
    let n = Node::<(), S<Z>>::branch(children).unwrap();
    insta::assert_snapshot!(snap(&n));
}

// ---------- Node<T, Root> ----------

/// The empty tree (`None` root): the smallest possible encoding.
#[test]
fn node_root_none() {
    let n: Option<Node<(), Root>> = None;
    insta::assert_snapshot!(snap(&n));
}

/// One leaf under the root: all 32 levels collapse into a single
/// compressed prefix on one node.
#[test]
fn node_root_single_leaf_full_compression() {
    let n = leaf("a", 1);
    seq_macro::seq!(I in 0..32 {
        let n = Node::beneath(n, I);
    });
    let n: Node<(), Root> = n;
    insta::assert_snapshot!(snap(&n));
}

/// Two leaves diverging at the very first byte: a root branch over two
/// 31-level compressed spines.
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
        let children: Children<(), _> = [(0x01, n0), (0x02, n1)].into_iter().collect();
        Node::<(), Root>::branch(children).unwrap()
    };
    insta::assert_snapshot!(snap(&n));
}

// ---------- Version ----------

/// The empty `Version` (no events anywhere): the canonical zero encoding.
#[test]
fn version_empty() {
    insta::assert_snapshot!(snap(&Version::new()));
}

/// A `Version` joining ticks from two disjoint parties: a non-trivial
/// event tree crosses the wire canonically.
#[test]
fn version_two_parties_ascending() {
    let v: Version = ticked("a", 1) | ticked("b", 2);
    insta::assert_snapshot!(snap(&v));
}

// ---------- Messages ----------

/// An `Initiate` with nothing uncertain: the convergent-open greeting.
#[test]
fn message_initiate_empty() {
    insta::assert_snapshot!(snap(&message::Initiate::default()));
}

/// An `Initiate` carrying one uncertain root hash: the ordinary opening.
#[test]
fn message_initiate_one_entry() {
    let uncertain = vec![(Prefix::<Root>::new(), Hash([1u8; MERKLE_HASH_LEN]))];
    insta::assert_snapshot!(snap(&message::Initiate { uncertain }));
}

/// An empty `Opening` reply: the responder with nothing in dispute.
#[test]
fn message_opening_empty() {
    insta::assert_snapshot!(snap(&message::Opening::default()));
}

/// An `Opening` disputing one child hash one level down.
#[test]
fn message_opening_one_entry() {
    let uncertain = vec![(
        prefix_from_bytes::<UnderRoot>(&[0x42]),
        Hash([2u8; MERKLE_HASH_LEN]),
    )];
    insta::assert_snapshot!(snap(&message::Opening { uncertain }));
}

/// An `Exchange` with all three sets empty: the in-band termination shape.
#[test]
fn message_exchange_empty() {
    let m: message::Exchange<(), UnderRoot> = message::Exchange::default();
    insta::assert_snapshot!(snap(&m));
}

/// An `Exchange` with all three sets populated — `providing` (a subtree),
/// `requested`, and `uncertain` — the full steady-state round shape.
#[test]
fn message_exchange_populated() {
    let leaf_z: Node<(), Z> = leaf("a", 1);
    let inner: Node<(), S<Z>> = Node::beneath(leaf_z, 0xab);
    let other_children: Children<(), S<Z>> =
        [(0x01, inner.clone()), (0x02, inner)].into_iter().collect();
    let s_s_z = Node::<(), S<S<Z>>>::branch(other_children).unwrap();
    let n_root: Node<(), Root> = {
        let n = s_s_z;
        seq_macro::seq!(I in 0..30 {
            let n = Node::beneath(n, I);
        });
        n
    };

    // `providing` is an ascending `(prefix, node)` list, plus an ascending
    // `requested` and `uncertain`.
    let providing = vec![(Prefix::<Root>::new(), n_root)];
    let requested = vec![Prefix::<Root>::new()];
    let uncertain = vec![(
        prefix_from_bytes::<UnderRoot>(&[0xcc]),
        Hash([3u8; MERKLE_HASH_LEN]),
    )];

    let m: message::Exchange<(), UnderRoot> = message::Exchange {
        providing,
        requested,
        uncertain,
    };
    insta::assert_snapshot!(snap(&m));
}

/// An empty `Closing`: the responder's closing round with nothing left.
#[test]
fn message_closing_empty() {
    let m: message::Closing<()> = message::Closing::default();
    insta::assert_snapshot!(snap(&m));
}

/// A `Closing` still providing and requesting leaves.
#[test]
fn message_closing_populated() {
    let providing = vec![(prefix_from_bytes::<Z>(&[0u8; 32]), leaf("a", 1))];
    let requested = vec![prefix_from_bytes::<Z>(&[0xffu8; 32])];
    let m: message::Closing<()> = message::Closing {
        providing,
        requested,
    };
    insta::assert_snapshot!(snap(&m));
}

/// An empty `Complete`: the initiator's sign-off with nothing owed.
#[test]
fn message_complete_empty() {
    let m: message::Complete<()> = message::Complete::default();
    insta::assert_snapshot!(snap(&m));
}

/// A `Complete` shipping one final leaf: the initiator's last `providing`.
#[test]
fn message_complete_populated() {
    let providing = vec![(prefix_from_bytes::<Z>(&[0u8; 32]), leaf("a", 1))];
    let m: message::Complete<()> = message::Complete { providing };
    insta::assert_snapshot!(snap(&m));
}
