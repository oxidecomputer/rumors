//! Wire⇄node conversion for the `providing` channel, and order-enforcement
//! for every channel.
//!
//! On the wire, `providing` is a flat `Vec<(Version, Message<T>)>` in ascending
//! path order: just the leaves of the subtrees being provided, with every
//! prefix and structural byte elided. This module turns that list back into the
//! `BTreeMap<Prefix<H>, Node<T, H>>` the protocol consumes
//! ([`reassemble_providing`]) and, on the send side, flattens such a map into
//! the wire list ([`flatten_providing`]).
//!
//! Re-materialization is a *verification*, not a trust: each leaf's position is
//! recomputed from its own `(version, value)` via [`Path::for_leaf`], so a peer
//! cannot place a leaf anywhere its content does not hash to. Because the
//! canonical compressed trie is uniquely determined by its leaf set
//! ([`Node::branch`]/[`Node::beneath`] enforce one shape), the rebuilt nodes are
//! structurally and hash-identical to the originals.
//!
//! The [`verify_*`](verify_providing_canonical) helpers re-impose the canonical
//! ordering the old `de_strict_order` `BTreeMap`/`BTreeSet` encodings gave for
//! free: a frame whose entries are out of order or duplicated is rejected at
//! deserialize time.

use std::collections::BTreeMap;

use borsh::BorshDeserialize;
use itertools::Itertools;

use crate::tree::typed::height::{Height, Root, S, Z};
use crate::tree::typed::{Children, Node, Path, Prefix};
use crate::{message::Message, version::Version};

/// Build a single [`Node<T, Self>`] from the leaves beneath it, each carrying
/// the remaining path from this node's height down to the leaf.
///
/// A synchronous mirror of the recursion in
/// [`act`](crate::tree::traverse::act): group by the next radix and recurse,
/// reassembling with [`Node::branch`] (which path-compresses single-child
/// branches via [`Node::beneath`]). Every group is non-empty by construction.
pub(crate) trait BuildNode: Height {
    fn build<T>(leaves: Vec<(Path<Self>, Version, Message<T>)>) -> Node<T, Self>;
}

impl BuildNode for Z {
    fn build<T>(leaves: Vec<(Path<Z>, Version, Message<T>)>) -> Node<T, Z> {
        // Paths are content-addressed and unique, and the canonical-order check
        // rejects duplicates upstream, so a leaf-height group holds exactly one
        // entry. Take the last defensively (last-writer-wins, as `act` does).
        let (_, version, message) = leaves
            .into_iter()
            .next_back()
            .expect("a reassembled group is never empty");
        Node::leaf(version, message)
    }
}

impl<H: BuildNode> BuildNode for S<H>
where
    S<H>: Height,
{
    fn build<T>(leaves: Vec<(Path<S<H>>, Version, Message<T>)>) -> Node<T, S<H>> {
        // Peel the next radix off each leaf's path and group siblings together,
        // collecting eagerly (as `act` does) so the lazy `ChunkBy` state isn't
        // held across the recursion.
        #[allow(clippy::type_complexity)]
        let by_radix: Vec<(u8, Vec<(Path<H>, Version, Message<T>)>)> = leaves
            .into_iter()
            .map(|(path, version, message)| {
                let (radix, path) = path.pop();
                (radix, path, version, message)
            })
            .sorted_by_key(|(radix, ..)| *radix)
            .chunk_by(|(radix, ..)| *radix)
            .into_iter()
            .map(|(radix, group)| (radix, group.map(|(_, path, v, m)| (path, v, m)).collect()))
            .collect();

        let children: Children<T, H> = by_radix
            .into_iter()
            .map(|(radix, group)| (radix, H::build(group)))
            .collect();

        Node::branch(children).expect("a reassembled group is never empty")
    }
}

/// Re-materialize a flat wire leaf list into the `providing` map at height `H`,
/// recomputing every leaf's content-addressed path so its placement is verified
/// rather than trusted. Inverse of [`flatten_providing`].
pub(crate) fn reassemble_providing<T, H: BuildNode>(
    leaves: Vec<(Version, Message<T>)>,
) -> BTreeMap<Prefix<H>, Node<T, H>> {
    let prefix_len = 32 - H::HEIGHT;
    #[allow(clippy::type_complexity)]
    let mut groups: BTreeMap<Prefix<H>, Vec<(Path<H>, Version, Message<T>)>> = BTreeMap::new();
    for (version, message) in leaves {
        let full: [u8; 32] = Path::<Root>::for_leaf(&version, message.bytes()).into();
        // The node sits at `Prefix<H>` (the leading `32 - H::HEIGHT` bytes); the
        // remaining bytes are descended inside `H::build`.
        let prefix = Prefix::<H>::try_from_slice(&full[..prefix_len])
            .expect("a path prefix is exactly 32 - H::HEIGHT bytes");
        groups
            .entry(prefix)
            .or_default()
            .push((Path::<H>::at_height(full), version, message));
    }
    groups
        .into_iter()
        .map(|(prefix, group)| (prefix, H::build(group)))
        .collect()
}

/// Flatten a `providing` map into the wire leaf list, in ascending path order
/// (disjoint sorted prefixes, each node's leaves ascending). Inverse of
/// [`reassemble_providing`]; the result satisfies [`verify_providing_canonical`].
pub(crate) fn flatten_providing<T, H: Height>(
    map: BTreeMap<Prefix<H>, Node<T, H>>,
) -> Vec<(Version, Message<T>)> {
    let mut leaves = Vec::new();
    for (prefix, node) in map {
        for (_key, version, message) in node.leaves(prefix) {
            leaves.push((version.clone(), message.clone()));
        }
    }
    leaves
}

/// An out-of-order or duplicated wire channel: the canonical encoding admits
/// exactly one byte sequence per value, so a peer that reorders or pads is
/// rejected before its content is acted on.
fn not_canonical(what: &'static str) -> borsh::io::Error {
    borsh::io::Error::new(
        borsh::io::ErrorKind::InvalidData,
        format!("{what} not in strictly ascending order"),
    )
}

/// Require a `providing` leaf list to be in strictly ascending recomputed-path
/// order (which also rejects duplicate paths).
pub(crate) fn verify_providing_canonical<T>(
    leaves: &[(Version, Message<T>)],
) -> borsh::io::Result<()> {
    let mut prev: Option<[u8; 32]> = None;
    for (version, message) in leaves {
        let path: [u8; 32] = Path::<Root>::for_leaf(version, message.bytes()).into();
        if prev.is_some_and(|p| path <= p) {
            return Err(not_canonical("providing leaves"));
        }
        prev = Some(path);
    }
    Ok(())
}

/// Require key→value pairs to be in strictly ascending key order (rejecting
/// duplicate keys): the `uncertain` channel.
pub(crate) fn verify_pairs_canonical<K: Ord, V>(
    pairs: &[(K, V)],
    what: &'static str,
) -> borsh::io::Result<()> {
    if pairs.windows(2).any(|w| w[0].0 >= w[1].0) {
        return Err(not_canonical(what));
    }
    Ok(())
}

/// Require keys to be in strictly ascending order (rejecting duplicates): the
/// `requested` channel.
pub(crate) fn verify_keys_canonical<K: Ord>(
    keys: &[K],
    what: &'static str,
) -> borsh::io::Result<()> {
    if keys.windows(2).any(|w| w[0] >= w[1]) {
        return Err(not_canonical(what));
    }
    Ok(())
}
