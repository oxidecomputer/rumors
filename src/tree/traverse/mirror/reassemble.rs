//! Wire⇄node conversion for the `providing` channel, and order-enforcement
//! for every channel.
//!
//! On the wire, `providing` is a flat `Vec<(Key, Version, Message<T>)>` in
//! ascending key order: just the leaves of the subtrees being provided, with
//! every prefix and structural byte elided. This module turns that list back
//! into the `BTreeMap<Prefix<H>, Node<T, H>>` the protocol consumes
//! ([`reassemble_providing`]) and, on the send side, flattens such a map into
//! the wire list ([`flatten_providing`]).
//!
//! Each leaf carries its [`Key`], which *is* its content-addressed path
//! `blake3(blake3(version) ‖ blake3(value))` ([`Path::for_leaf`]). The provider
//! already holds that hash, so it ships it and the receiver places the leaf
//! directly — no re-hash of the `(version, value)`, which is otherwise the
//! dominant cost of reassembly (up to ~4×). Release builds trust the
//! transmitted key; debug builds recompute the path and `debug_assert!` it
//! matches, catching a misbehaving peer or our own protocol drift. Because the
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

use crate::tree::key::Key;
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
/// placing each leaf at the path named by its transmitted [`Key`]. The key *is*
/// the leaf's content-addressed path, so release builds trust it directly and
/// skip the [`Path::for_leaf`] re-hash; debug builds recompute the path and
/// `debug_assert!` it matches the key, catching a misbehaving peer or protocol
/// drift. Inverse of [`flatten_providing`].
pub(crate) fn reassemble_providing<T, H: BuildNode>(
    leaves: Vec<(Key, Version, Message<T>)>,
) -> BTreeMap<Prefix<H>, Node<T, H>> {
    let prefix_len = 32 - H::HEIGHT;
    #[allow(clippy::type_complexity)]
    let mut groups: BTreeMap<Prefix<H>, Vec<(Path<H>, Version, Message<T>)>> = BTreeMap::new();
    for (key, version, message) in leaves {
        let full: [u8; 32] = key.0;
        // The provider already holds the leaf's hash; we trust the transmitted
        // key rather than re-deriving it. In debug builds we still recompute and
        // assert, so a key that does not match its content is caught in test and
        // dev runs (release skips it for the performance win).
        debug_assert_eq!(
            full,
            <[u8; 32]>::from(Path::<Root>::for_leaf(&version, message.bytes())),
            "provided leaf key does not match its content-addressed path",
        );
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

/// Flatten a `providing` map into the wire leaf list, in ascending key order
/// (disjoint sorted prefixes, each node's leaves ascending). Each leaf's [`Key`]
/// — already memoized as its content-addressed path — travels with it so the
/// receiver need not recompute. Inverse of [`reassemble_providing`]; the result
/// satisfies [`verify_providing_canonical`].
pub(crate) fn flatten_providing<T, H: Height>(
    map: BTreeMap<Prefix<H>, Node<T, H>>,
) -> Vec<(Key, Version, Message<T>)> {
    let mut leaves = Vec::new();
    for (prefix, node) in map {
        for (key, version, message) in node.leaves(prefix) {
            leaves.push((key, version.clone(), message.clone()));
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

/// Require a `providing` leaf list to be in strictly ascending transmitted-key
/// order (which also rejects duplicate keys). The key is the leaf's
/// content-addressed path, so this is the same ordering the receiver places by;
/// it costs only key comparisons, not a per-leaf re-hash. (Whether each key
/// *matches* its content is a separate, debug-only check in
/// [`reassemble_providing`].)
pub(crate) fn verify_providing_canonical<T>(
    leaves: &[(Key, Version, Message<T>)],
) -> borsh::io::Result<()> {
    if leaves.windows(2).any(|w| w[0].0 >= w[1].0) {
        return Err(not_canonical("providing leaves"));
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
