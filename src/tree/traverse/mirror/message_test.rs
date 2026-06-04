//! Borsh round-trip property tests for the five mirror message types. Each
//! test constructs a representative population of fields, serializes via
//! `borsh::to_vec`, deserializes, and asserts structural equality. The exact
//! on-wire bytes are pinned by `mirror::wire_snapshot`; this file pins
//! semantic round-trip correctness across the full state space of
//! `arb_root_tree`.

use std::collections::{BTreeMap, BTreeSet};

use borsh::BorshDeserialize;
use proptest::collection::vec;
use proptest::prelude::*;

use crate::message::Message;
use crate::tree::arb::{arb_root_node, arb_version};
use crate::tree::typed::height::{Height, Root, S, Z};
use crate::tree::typed::{Hash, Node, Prefix};

use super::message;

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

/// `Node<(), S<Z>>` wrapping a leaf with a singleton path-
/// compression byte. Covers the path-compressed branch case at the
/// lowest interesting typed height.
fn arb_s_z_node() -> BoxedStrategy<Node<(), S<Z>>> {
    (arb_leaf(), any::<u8>())
        .prop_map(|(leaf, byte)| Node::beneath(leaf, byte))
        .boxed()
}

proptest! {
    /// `Initiate` is `uncertain: BTreeMap<Prefix<Root>, Hash>`.
    #[test]
    fn initiate_borsh_round_trip(
        entries in vec((arb_prefix::<Root>(), arb_hash()), 0..=4),
    ) {
        let m = message::Initiate {
            uncertain: entries.into_iter().collect(),
        };
        let bytes = borsh::to_vec(&m).unwrap();
        let decoded = message::Initiate::try_from_slice(&bytes).unwrap();
        prop_assert_eq!(m.uncertain, decoded.uncertain);
    }

    /// `Opening` is `uncertain: BTreeMap<Prefix<UnderRoot>, Hash>`.
    #[test]
    fn opening_borsh_round_trip(
        entries in vec((arb_prefix::<message::UnderRoot>(), arb_hash()), 0..=4),
    ) {
        let m = message::Opening {
            uncertain: entries.into_iter().collect(),
        };
        let bytes = borsh::to_vec(&m).unwrap();
        let decoded = message::Opening::try_from_slice(&bytes).unwrap();
        prop_assert_eq!(m.uncertain, decoded.uncertain);
    }

    /// `Exchange<T, UnderRoot>` carries all three channels: providing
    /// subtrees at `Root` height (populated from `arb_root_tree`),
    /// requested prefixes at `Root`, and uncertain hashes at `UnderRoot`.
    #[test]
    fn exchange_borsh_round_trip(
        providing_entries in vec(
            (arb_prefix::<Root>(), arb_root_node(0, 1..=4).prop_filter("non-empty", |n| n.is_some())),
            0..=2,
        ),
        requested in vec(arb_prefix::<Root>(), 0..=4),
        uncertain in vec((arb_prefix::<message::UnderRoot>(), arb_hash()), 0..=4),
    ) {
        let providing: BTreeMap<Prefix<Root>, Node<(), Root>> = providing_entries
            .into_iter()
            .map(|(p, n)| (p, n.expect("filtered non-None")))
            .collect();
        let requested: BTreeSet<Prefix<Root>> = requested.into_iter().collect();
        let uncertain: BTreeMap<Prefix<message::UnderRoot>, Hash> =
            uncertain.into_iter().collect();
        let m: message::Exchange<(), message::UnderRoot> = message::Exchange {
            providing: providing.clone(),
            requested: requested.clone(),
            uncertain: uncertain.clone(),
        };
        let bytes = borsh::to_vec(&m).unwrap();
        let decoded =
            message::Exchange::<(), message::UnderRoot>::try_from_slice(&bytes)
                .unwrap();
        prop_assert_eq!(decoded.providing, providing);
        prop_assert_eq!(decoded.requested, requested);
        prop_assert_eq!(decoded.uncertain, uncertain);
    }

    /// `Closing<T>` carries `providing: BTreeMap<Prefix<S<Z>>, Node<S<Z>>>`
    /// and `requested: BTreeSet<Prefix<S<Z>>>`.
    #[test]
    fn closing_borsh_round_trip(
        providing_entries in vec((arb_prefix::<S<Z>>(), arb_s_z_node()), 0..=4),
        requested in vec(arb_prefix::<S<Z>>(), 0..=4),
    ) {
        let providing: BTreeMap<Prefix<S<Z>>, Node<(), S<Z>>> =
            providing_entries.into_iter().collect();
        let requested: BTreeSet<Prefix<S<Z>>> = requested.into_iter().collect();
        let m: message::Closing<()> = message::Closing {
            providing: providing.clone(),
            requested: requested.clone(),
        };
        let bytes = borsh::to_vec(&m).unwrap();
        let decoded = message::Closing::<()>::try_from_slice(&bytes).unwrap();
        prop_assert_eq!(decoded.providing, providing);
        prop_assert_eq!(decoded.requested, requested);
    }

    /// `Complete<T>` carries `providing: BTreeMap<Prefix<Z>, Node<Z>>`.
    /// At `Z` heights, a `Node` is exactly a leaf.
    #[test]
    fn complete_borsh_round_trip(
        providing_entries in vec((arb_prefix::<Z>(), arb_leaf()), 0..=4),
    ) {
        let providing: BTreeMap<Prefix<Z>, Node<(), Z>> =
            providing_entries.into_iter().collect();
        let m: message::Complete<()> = message::Complete {
            providing: providing.clone(),
        };
        let bytes = borsh::to_vec(&m).unwrap();
        let decoded = message::Complete::<()>::try_from_slice(&bytes).unwrap();
        prop_assert_eq!(decoded.providing, providing);
    }
}
