//! Wire round-trip property tests for the five mirror message types, plus the
//! canonical-order rejection each channel enforces on deserialize.
//!
//! Every channel is a length-prefixed `Vec` that must arrive in strictly
//! ascending, duplicate-free order; the tests feed each one pre-sorted (via
//! [`canonical_pairs`] / [`canonical_keys`] / [`canonical_providing`]) to
//! satisfy that check, and separately pin that a non-canonical frame is
//! rejected. `providing` carries whole `(prefix, node)` pairs, so its tests
//! build nodes via [`arb_root_node`] / [`arb_s_z_node`] / [`arb_leaf`]. The
//! exact on-wire bytes are pinned by `mirror::alternating::wire_snapshot`.

use std::collections::{BTreeMap, BTreeSet};

use proptest::collection::vec;
use proptest::prelude::*;

use crate::Version;
use crate::message::Message;
use crate::tree::arb::{arb_root_node, arb_version, nth_party};
use crate::tree::typed::height::{Height, Root, S, Z};
use crate::tree::typed::{Hash, Node, Prefix, hash::MERKLE_HASH_LEN};
use crate::tree::wire;

use super as message;

/// Build a `Prefix<H>` from a raw byte slice (length `32 - H::HEIGHT`).
fn prefix_from_bytes<H: Height>(bytes: &[u8]) -> Prefix<H> {
    assert_eq!(bytes.len(), 32 - H::HEIGHT);
    wire::from_slice(bytes).expect("known-valid prefix bytes")
}

fn arb_prefix<H: Height + 'static>() -> BoxedStrategy<Prefix<H>> {
    vec(any::<u8>(), 32 - H::HEIGHT)
        .prop_map(|bytes| prefix_from_bytes::<H>(&bytes))
        .boxed()
}

fn arb_hash() -> BoxedStrategy<Hash> {
    any::<[u8; MERKLE_HASH_LEN]>().prop_map(Hash).boxed()
}

fn arb_leaf() -> BoxedStrategy<Node<(), Z>> {
    arb_version()
        .prop_map(|version| Node::leaf(version, Message::new(())))
        .boxed()
}

/// Sort and deduplicate `(prefix, node)` entries into the canonical ascending
/// `Vec` the `providing` channel expects.
fn canonical_providing<H: Height>(
    entries: Vec<(Prefix<H>, Node<(), H>)>,
) -> Vec<(Prefix<H>, Node<(), H>)> {
    entries
        .into_iter()
        .collect::<BTreeMap<_, _>>()
        .into_iter()
        .collect()
}

/// Sort and deduplicate `(prefix, hash)` entries into the canonical ascending
/// `Vec` the wire expects.
fn canonical_pairs<H: Height>(entries: Vec<(Prefix<H>, Hash)>) -> Vec<(Prefix<H>, Hash)> {
    entries
        .into_iter()
        .collect::<BTreeMap<_, _>>()
        .into_iter()
        .collect()
}

/// Sort and deduplicate prefixes into the canonical ascending `Vec`.
fn canonical_keys<H: Height>(keys: Vec<Prefix<H>>) -> Vec<Prefix<H>> {
    keys.into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

proptest! {
    /// `Initiate.uncertain` round-trips, fed in canonical ascending order.
    #[test]
    fn initiate_wire_round_trip(
        entries in vec((arb_prefix::<Root>(), arb_hash()), 0..=4),
    ) {
        let uncertain = canonical_pairs(entries);
        let m = message::Initiate { uncertain: uncertain.clone() };
        let bytes = wire::to_vec(&m).unwrap();
        let decoded = wire::from_slice::<message::Initiate>(&bytes).unwrap();
        prop_assert_eq!(decoded.uncertain, uncertain);
    }

    /// `Opening.uncertain` round-trips, fed in canonical ascending order.
    #[test]
    fn opening_wire_round_trip(
        entries in vec((arb_prefix::<message::UnderRoot>(), arb_hash()), 0..=4),
    ) {
        let uncertain = canonical_pairs(entries);
        let m = message::Opening { uncertain: uncertain.clone() };
        let bytes = wire::to_vec(&m).unwrap();
        let decoded = wire::from_slice::<message::Opening>(&bytes).unwrap();
        prop_assert_eq!(decoded.uncertain, uncertain);
    }

    /// `Exchange` carries all three channels: `providing` subtrees at `Root`
    /// height (populated from `arb_root_node`), an ascending `requested` at
    /// `Root`, and ascending `uncertain` hashes at `UnderRoot`.
    #[test]
    fn exchange_wire_round_trip(
        providing_entries in vec(
            (arb_prefix::<Root>(), arb_root_node(0, 1..=4).prop_filter("non-empty", |n| n.is_some())),
            0..=2,
        ),
        requested in vec(arb_prefix::<Root>(), 0..=4),
        uncertain in vec((arb_prefix::<message::UnderRoot>(), arb_hash()), 0..=4),
    ) {
        let providing = canonical_providing(
            providing_entries
                .into_iter()
                .map(|(p, n)| (p, n.expect("filtered non-None")))
                .collect(),
        );
        let requested = canonical_keys(requested);
        let uncertain = canonical_pairs(uncertain);
        let m: message::Exchange<(), message::UnderRoot> = message::Exchange {
            providing: providing.clone(),
            requested: requested.clone(),
            uncertain: uncertain.clone(),
        };
        let bytes = wire::to_vec(&m).unwrap();
        let decoded =
            wire::from_slice::<message::Exchange<(), message::UnderRoot>>(&bytes).unwrap();
        prop_assert_eq!(decoded.providing, providing);
        prop_assert_eq!(decoded.requested, requested);
        prop_assert_eq!(decoded.uncertain, uncertain);
    }

    /// `Closing` carries leaf-height `providing` and an ascending
    /// `requested`, both at `Z`.
    #[test]
    fn closing_wire_round_trip(
        providing_entries in vec((arb_prefix::<Z>(), arb_leaf()), 0..=4),
        requested in vec(arb_prefix::<Z>(), 0..=4),
    ) {
        let providing = canonical_providing(providing_entries);
        let requested = canonical_keys(requested);
        let m: message::Closing<()> = message::Closing {
            providing: providing.clone(),
            requested: requested.clone(),
        };
        let bytes = wire::to_vec(&m).unwrap();
        let decoded = wire::from_slice::<message::Closing<()>>(&bytes).unwrap();
        prop_assert_eq!(decoded.providing, providing);
        prop_assert_eq!(decoded.requested, requested);
    }

    /// `Complete` carries only `providing`, at leaf (`Z`) height where a `Node`
    /// is exactly a leaf.
    #[test]
    fn complete_wire_round_trip(
        providing_entries in vec((arb_prefix::<Z>(), arb_leaf()), 0..=4),
    ) {
        let providing = canonical_providing(providing_entries);
        let m: message::Complete<()> = message::Complete { providing: providing.clone() };
        let bytes = wire::to_vec(&m).unwrap();
        let decoded = wire::from_slice::<message::Complete<()>>(&bytes).unwrap();
        prop_assert_eq!(decoded.providing, providing);
    }

    /// Any non-canonical permutation of a `providing` list is rejected on
    /// deserialize: only the unique strictly-ascending-by-prefix order decodes.
    /// (Two or more entries are needed for an order to be wrong.)
    #[test]
    fn providing_rejects_non_canonical_order(
        providing_entries in vec((arb_prefix::<Z>(), arb_leaf()), 2..=6),
        rotate in 1usize..6,
    ) {
        let canonical = canonical_providing(providing_entries);
        prop_assume!(canonical.len() >= 2);
        // Rotate the canonical order so the list is no longer ascending; any
        // rotation by a nonzero amount less than the length breaks the order.
        let mut permuted = canonical.clone();
        permuted.rotate_left(rotate % canonical.len());
        prop_assume!(permuted != canonical);
        let m = message::Complete::<()> { providing: permuted };
        let bytes = wire::to_vec(&m).unwrap();
        prop_assert!(wire::from_slice::<message::Complete<()>>(&bytes).is_err());
    }
}

/// A single version, ticked once on a fixed party — enough to place one leaf.
fn one_version() -> Version {
    let p = nth_party(0);
    let mut v = Version::new();
    v.tick(&p);
    v
}

/// A `providing` frame with two entries at the same prefix is rejected: the
/// canonical encoding admits no duplicate keys.
#[test]
fn providing_rejects_duplicate_prefix() {
    let prefix = prefix_from_bytes::<Z>(&[7u8; 32]);
    let leaf = Node::leaf(one_version(), Message::new(()));
    let m = message::Complete::<()> {
        providing: vec![(prefix, leaf.clone()), (prefix, leaf)],
    };
    let bytes = wire::to_vec(&m).unwrap();
    assert!(wire::from_slice::<message::Complete<()>>(&bytes).is_err());
}

/// A `requested` frame whose prefixes descend is rejected.
#[test]
fn requested_rejects_descending_order() {
    let m = message::Closing::<()> {
        providing: Vec::new(),
        requested: vec![
            prefix_from_bytes::<Z>(&[2u8; 32]),
            prefix_from_bytes::<Z>(&[1u8; 32]),
        ],
    };
    let bytes = wire::to_vec(&m).unwrap();
    assert!(wire::from_slice::<message::Closing<()>>(&bytes).is_err());
}

/// An `uncertain` frame with a duplicate prefix is rejected.
#[test]
fn uncertain_rejects_duplicate_prefix() {
    let m = message::Initiate {
        uncertain: vec![
            (prefix_from_bytes::<Root>(&[]), Hash([0; MERKLE_HASH_LEN])),
            (prefix_from_bytes::<Root>(&[]), Hash([1; MERKLE_HASH_LEN])),
        ],
    };
    let bytes = wire::to_vec(&m).unwrap();
    assert!(wire::from_slice::<message::Initiate>(&bytes).is_err());
}

// The `providing` channels carry whole wire-encoded nodes, so the node
// decoder's structural checks (`src/tree/typed/node.rs`) are part of this
// ingress. The three lies a wire node can tell about its own shape are
// pinned here byte-by-byte; the elements of a valid encoding are pinned in
// `mirror::alternating::wire_snapshot`.

/// A node whose declared prefix length exceeds its typed height is rejected.
///
/// The first body byte is the path-compression count; at height 1 any value
/// above 1 promises more singleton levels than the type admits, and the
/// decoder must reject the byte itself (`InvalidData`) rather than recurse
/// past the leaf floor.
#[test]
fn node_prefix_exceeding_height_is_rejected() {
    let error = wire::from_slice::<Node<(), S<Z>>>(&[2]).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
}

/// A branch declaring 257 children is rejected before any child is read.
///
/// The count byte stores `count - 2`, so its maximum (255) declares 257
/// children — one more than the 256-ary radix alphabet admits. The decoder
/// must reject the count (`InvalidData`) instead of reading children that
/// cannot all be placed.
#[test]
fn node_child_count_overflow_is_rejected() {
    let error = wire::from_slice::<Node<(), S<Z>>>(&[0, 255]).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
}

/// Branch radices that fail to strictly ascend are rejected at the radix.
///
/// A two-child branch whose second radix repeats the first violates the
/// canonical ascending order; the decoder must reject the second radix byte
/// (`InvalidData`) rather than let one branch have two encodings.
#[test]
fn node_descending_radices_are_rejected() {
    let leaf = wire::to_vec(&Node::<(), Z>::leaf(one_version(), Message::new(()))).unwrap();
    // prefix_len 0, count_minus_two 0 (two children), radix 5, its leaf,
    // then a second radix that does not ascend.
    let mut bytes = vec![0, 0, 5];
    bytes.extend_from_slice(&leaf);
    bytes.push(5);
    bytes.extend_from_slice(&leaf);

    let error = wire::from_slice::<Node<(), S<Z>>>(&bytes).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
}
