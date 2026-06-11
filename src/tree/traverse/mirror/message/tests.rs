//! Borsh round-trip property tests for the five mirror message types, plus the
//! canonical-order rejection each channel enforces on deserialize.
//!
//! Every channel is a length-prefixed `Vec` that must arrive in strictly
//! ascending, duplicate-free order; the tests feed each one pre-sorted (via
//! [`canonical_pairs`] / [`canonical_keys`] / [`canonical_providing`]) to
//! satisfy that check, and separately pin that a non-canonical frame is
//! rejected. `providing` carries whole `(prefix, node)` pairs, so its tests
//! build nodes via [`arb_root_node`] / [`arb_s_z_node`] / [`arb_leaf`]. The
//! exact on-wire bytes are pinned by `mirror::wire_snapshot`.

use std::collections::{BTreeMap, BTreeSet};

use borsh::BorshDeserialize;
use proptest::collection::vec;
use proptest::prelude::*;

use crate::Version;
use crate::message::Message;
use crate::tree::arb::{arb_root_node, arb_version, nth_party};
use crate::tree::typed::height::{Height, Root, S, Z};
use crate::tree::typed::{Hash, Node, Prefix};

use crate::tree::traverse::mirror::message;

/// Build a `Prefix<H>` from a raw byte slice (length `32 - H::HEIGHT`).
fn prefix_from_bytes<H: Height>(bytes: &[u8]) -> Prefix<H> {
    assert_eq!(bytes.len(), 32 - H::HEIGHT);
    Prefix::<H>::try_from_slice(bytes).expect("known-valid prefix bytes")
}

fn arb_prefix<H: Height + 'static>() -> BoxedStrategy<Prefix<H>> {
    vec(any::<u8>(), 32 - H::HEIGHT)
        .prop_map(|bytes| prefix_from_bytes::<H>(&bytes))
        .boxed()
}

fn arb_hash() -> BoxedStrategy<Hash> {
    any::<[u8; 32]>().prop_map(Hash).boxed()
}

fn arb_leaf() -> BoxedStrategy<Node<(), Z>> {
    arb_version()
        .prop_map(|version| Node::leaf(version, Message::new(())))
        .boxed()
}

/// `Node<(), S<Z>>` wrapping a leaf with a singleton path-compression byte.
/// Covers the path-compressed branch case at the lowest interesting typed
/// height.
fn arb_s_z_node() -> BoxedStrategy<Node<(), S<Z>>> {
    (arb_leaf(), any::<u8>())
        .prop_map(|(leaf, byte)| Node::beneath(leaf, byte))
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
    fn initiate_borsh_round_trip(
        entries in vec((arb_prefix::<Root>(), arb_hash()), 0..=4),
    ) {
        let uncertain = canonical_pairs(entries);
        let m = message::Initiate { uncertain: uncertain.clone() };
        let bytes = borsh::to_vec(&m).unwrap();
        let decoded = message::Initiate::try_from_slice(&bytes).unwrap();
        prop_assert_eq!(decoded.uncertain, uncertain);
    }

    /// `Opening.uncertain` round-trips, fed in canonical ascending order.
    #[test]
    fn opening_borsh_round_trip(
        entries in vec((arb_prefix::<message::UnderRoot>(), arb_hash()), 0..=4),
    ) {
        let uncertain = canonical_pairs(entries);
        let m = message::Opening { uncertain: uncertain.clone() };
        let bytes = borsh::to_vec(&m).unwrap();
        let decoded = message::Opening::try_from_slice(&bytes).unwrap();
        prop_assert_eq!(decoded.uncertain, uncertain);
    }

    /// `Exchange` carries all three channels: `providing` subtrees at `Root`
    /// height (populated from `arb_root_node`), an ascending `requested` at
    /// `Root`, and ascending `uncertain` hashes at `UnderRoot`.
    #[test]
    fn exchange_borsh_round_trip(
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
        let bytes = borsh::to_vec(&m).unwrap();
        let decoded =
            message::Exchange::<(), message::UnderRoot>::try_from_slice(&bytes).unwrap();
        prop_assert_eq!(decoded.providing, providing);
        prop_assert_eq!(decoded.requested, requested);
        prop_assert_eq!(decoded.uncertain, uncertain);
    }

    /// `Closing` carries `providing` subtrees at `S<Z>` and an ascending
    /// `requested` at `S<Z>`.
    #[test]
    fn closing_borsh_round_trip(
        providing_entries in vec((arb_prefix::<S<Z>>(), arb_s_z_node()), 0..=4),
        requested in vec(arb_prefix::<S<Z>>(), 0..=4),
    ) {
        let providing = canonical_providing(providing_entries);
        let requested = canonical_keys(requested);
        let m: message::Closing<()> = message::Closing {
            providing: providing.clone(),
            requested: requested.clone(),
        };
        let bytes = borsh::to_vec(&m).unwrap();
        let decoded = message::Closing::<()>::try_from_slice(&bytes).unwrap();
        prop_assert_eq!(decoded.providing, providing);
        prop_assert_eq!(decoded.requested, requested);
    }

    /// `Complete` carries only `providing`, at leaf (`Z`) height where a `Node`
    /// is exactly a leaf.
    #[test]
    fn complete_borsh_round_trip(
        providing_entries in vec((arb_prefix::<Z>(), arb_leaf()), 0..=4),
    ) {
        let providing = canonical_providing(providing_entries);
        let m: message::Complete<()> = message::Complete { providing: providing.clone() };
        let bytes = borsh::to_vec(&m).unwrap();
        let decoded = message::Complete::<()>::try_from_slice(&bytes).unwrap();
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
        let bytes = borsh::to_vec(&m).unwrap();
        prop_assert!(message::Complete::<()>::try_from_slice(&bytes).is_err());
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
    let bytes = borsh::to_vec(&m).unwrap();
    assert!(message::Complete::<()>::try_from_slice(&bytes).is_err());
}

/// A `requested` frame whose prefixes descend is rejected.
#[test]
fn requested_rejects_descending_order() {
    let m = message::Closing::<()> {
        providing: Vec::new(),
        requested: vec![
            prefix_from_bytes::<S<Z>>(&[2u8; 31]),
            prefix_from_bytes::<S<Z>>(&[1u8; 31]),
        ],
    };
    let bytes = borsh::to_vec(&m).unwrap();
    assert!(message::Closing::<()>::try_from_slice(&bytes).is_err());
}

/// An `uncertain` frame with a duplicate prefix is rejected.
#[test]
fn uncertain_rejects_duplicate_prefix() {
    let m = message::Initiate {
        uncertain: vec![
            (prefix_from_bytes::<Root>(&[]), Hash([0; 32])),
            (prefix_from_bytes::<Root>(&[]), Hash([1; 32])),
        ],
    };
    let bytes = borsh::to_vec(&m).unwrap();
    assert!(message::Initiate::try_from_slice(&bytes).is_err());
}
