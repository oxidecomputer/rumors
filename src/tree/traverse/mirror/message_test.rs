//! Borsh round-trip property tests for the five mirror message types, plus the
//! `providing` reassemble⇄flatten round-trip and the canonical-order rejection
//! each channel enforces on deserialize.
//!
//! The `providing` channel carries only leaves on the wire and re-derives each
//! leaf's position from its `(version, value)`, so its tests build leaves at
//! their true content-addressed paths (via [`Path::for_leaf`]) rather than at
//! arbitrary prefixes. The `uncertain` / `requested` channels still carry
//! arbitrary prefixes, fed pre-sorted to satisfy the canonical-order check.
//! The exact on-wire bytes are pinned by `mirror::wire_snapshot`.

use std::collections::{BTreeMap, BTreeSet};

use borsh::BorshDeserialize;
use proptest::collection::vec;
use proptest::prelude::*;

use crate::message::Message;
use crate::tree::arb::{arb_version, nth_party};
use crate::tree::typed::height::{Height, Root, S, Z};
use crate::tree::typed::{Hash, Path, Prefix};
use crate::version::Version;

use super::message;
use super::reassemble::{flatten_providing, reassemble_providing};

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

/// A `providing` leaf list in canonical wire form: each leaf placed at its true
/// content-addressed path, deduplicated, in strictly ascending path order. The
/// value is `()` so the path is determined by the version alone.
fn canonical_leaves(versions: Vec<Version>) -> Vec<(Version, Message<()>)> {
    let mut by_path: BTreeMap<[u8; 32], (Version, Message<()>)> = BTreeMap::new();
    for version in versions {
        let message = Message::new(());
        let path: [u8; 32] = Path::<Root>::for_leaf(&version, message.bytes()).into();
        by_path.insert(path, (version, message));
    }
    by_path.into_values().collect()
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

    /// `Exchange` carries all three channels: a `providing` leaf list, an
    /// ascending `requested`, and ascending `uncertain` hashes.
    #[test]
    fn exchange_borsh_round_trip(
        versions in vec(arb_version(), 0..=6),
        requested in vec(arb_prefix::<Root>(), 0..=4),
        uncertain in vec((arb_prefix::<message::UnderRoot>(), arb_hash()), 0..=4),
    ) {
        let providing = canonical_leaves(versions);
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

    /// `Closing` carries a `providing` leaf list and an ascending `requested`.
    #[test]
    fn closing_borsh_round_trip(
        versions in vec(arb_version(), 0..=6),
        requested in vec(arb_prefix::<S<Z>>(), 0..=4),
    ) {
        let providing = canonical_leaves(versions);
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

    /// `Complete` carries only a `providing` leaf list.
    #[test]
    fn complete_borsh_round_trip(
        versions in vec(arb_version(), 0..=6),
    ) {
        let providing = canonical_leaves(versions);
        let m: message::Complete<()> = message::Complete { providing: providing.clone() };
        let bytes = borsh::to_vec(&m).unwrap();
        let decoded = message::Complete::<()>::try_from_slice(&bytes).unwrap();
        prop_assert_eq!(decoded.providing, providing);
    }

    /// Any non-canonical permutation of a `providing` leaf list is rejected on
    /// deserialize: only the unique strictly-ascending-by-path order decodes.
    /// (Two or more leaves are needed for an order to be wrong.)
    #[test]
    fn providing_rejects_non_canonical_order(
        versions in vec(arb_version(), 2..=6),
        rotate in 1usize..6,
    ) {
        let canonical = canonical_leaves(versions);
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

    /// Reassembling a leaf list into the `providing` map at a given height and
    /// flattening it back is the identity: placement is content-derived, so the
    /// rebuilt subtrees yield exactly the original leaves in the original order.
    /// Exercised across heights from leaf (`Z`) up near the root.
    #[test]
    fn reassemble_flatten_identity(versions in vec(arb_version(), 0..=6)) {
        let leaves = canonical_leaves(versions);
        prop_assert_eq!(
            flatten_providing(reassemble_providing::<_, Z>(leaves.clone())),
            leaves.clone()
        );
        prop_assert_eq!(
            flatten_providing(reassemble_providing::<_, S<Z>>(leaves.clone())),
            leaves.clone()
        );
        prop_assert_eq!(
            flatten_providing(reassemble_providing::<_, S<S<Z>>>(leaves.clone())),
            leaves.clone()
        );
        prop_assert_eq!(
            flatten_providing(reassemble_providing::<_, message::UnderRoot>(leaves.clone())),
            leaves
        );
    }
}

/// A single version, ticked once on a fixed party — enough to place one leaf.
fn one_version() -> Version {
    let p = nth_party(0);
    let mut v = Version::new();
    v.tick(&p);
    v
}

/// A `providing` frame with two identical leaves (hence identical recomputed
/// paths) is rejected: the canonical encoding admits no duplicates.
#[test]
fn providing_rejects_duplicate_paths() {
    let version = one_version();
    let m = message::Complete::<()> {
        providing: vec![
            (version.clone(), Message::new(())),
            (version, Message::new(())),
        ],
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
