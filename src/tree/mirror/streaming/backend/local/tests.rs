//! `Local`'s bulk `leaves`/`assemble` overrides against the default chain.
//!
//! The overrides exist purely as a cost optimization (skip the per-virtual-
//! level unwrapping of path-compressed spines), so the property that keeps
//! them honest is *observational equivalence*: over arbitrary leaf runs —
//! deep compressed spines, dense branch points, multiple same-height runs —
//! the override and the level-by-level [`Convert`] default must produce
//! identical streams.

use std::convert::Infallible;

use futures::{TryStreamExt, stream};
use proptest::prelude::*;

use crate::{
    Version,
    message::Message,
    tree::{
        arb::nth_party,
        mirror::streaming::{Backend, backend::BoxNodeStream, convert::Convert},
        typed::{
            self, Path, Prefix,
            height::{self, S, Z},
        },
    },
};

use super::Local;

type LeafRun = Vec<(Prefix<Z>, typed::Node<(), Z>)>;

/// One distinct-version leaf per path, in the (ascending) order given.
fn leaves_at(paths: impl IntoIterator<Item = [u8; 32]>) -> LeafRun {
    let mut version = Version::new();
    paths
        .into_iter()
        .map(|bytes| {
            version.tick(&nth_party(0));
            (
                Path::from(bytes).into(),
                typed::Node::leaf(version.clone(), Message::new(())),
            )
        })
        .collect()
}

fn boxed(run: LeafRun) -> BoxNodeStream<'static, Local, (), Z> {
    Box::pin(stream::iter(run.into_iter().map(Ok::<_, Infallible>)))
}

/// Assemble through the level-by-level default, bypassing Local's override.
fn assemble_default<H: Convert>(run: LeafRun) -> Vec<(Prefix<H>, typed::Node<(), H>)> {
    pollster::block_on(H::assemble(Local, boxed(run)).try_collect())
        .unwrap_or_else(|error| match error {})
}

/// Assemble through the backend seam, which Local overrides in bulk.
fn assemble_local<H: Convert>(run: LeafRun) -> Vec<(Prefix<H>, typed::Node<(), H>)> {
    pollster::block_on(Local.assemble::<H>(boxed(run)).try_collect())
        .unwrap_or_else(|error| match error {})
}

/// Explode through the level-by-level default, bypassing Local's override.
fn leaves_default<H: Convert>(prefix: Prefix<H>, node: typed::Node<(), H>) -> LeafRun {
    pollster::block_on(
        H::explode(
            Local,
            Box::pin(stream::once(async move { Ok((prefix, node)) })),
        )
        .try_collect(),
    )
    .unwrap_or_else(|error| match error {})
}

/// Walk leaves through the backend seam, which Local overrides directly.
fn leaves_local<H: Convert>(prefix: Prefix<H>, node: typed::Node<(), H>) -> LeafRun {
    pollster::block_on(Local.leaves(prefix, node).try_collect())
        .unwrap_or_else(|error| match error {})
}

/// The observable content of a leaf run: full path plus version (bare leaf
/// *hashes* commit nothing but their empty suffix — content is committed by
/// path — so equality must compare versions, not hashes).
fn content(run: &LeafRun) -> Vec<(Prefix<Z>, Version)> {
    run.iter()
        .map(|(prefix, leaf)| (*prefix, leaf.ceiling().clone()))
        .collect()
}

/// Paths over a tiny alphabet share long prefixes, forcing deep compressed
/// spines and branch points at many depths; paths over the full alphabet
/// mostly diverge at the top, forcing wide fans. Both shapes matter.
fn paths() -> impl Strategy<Value = std::collections::BTreeSet<[u8; 32]>> {
    prop_oneof![
        proptest::collection::btree_set(proptest::array::uniform32(0u8..3), 1..=24),
        proptest::collection::btree_set(proptest::array::uniform32(any::<u8>()), 1..=24),
    ]
}

proptest! {
    /// Bulk assembly of one full-height run matches the default fold.
    ///
    /// Same hash (structure), same leaf count, same version bounds — and
    /// both walks (Local's direct one and the default explosion) return
    /// exactly the input leaves, in order.
    #[test]
    fn full_height_roundtrip_matches_default(paths in paths()) {
        let run = leaves_at(paths);
        let expected = content(&run);
        let root = Prefix::<height::Root>::new();

        let ours = assemble_local::<height::Root>(run.clone());
        let theirs = assemble_default::<height::Root>(run);
        prop_assert_eq!(ours.len(), 1);
        prop_assert_eq!(theirs.len(), 1);
        let (ours_prefix, ours_node) = ours.into_iter().next().expect("asserted nonempty");
        let (theirs_prefix, theirs_node) = theirs.into_iter().next().expect("asserted nonempty");
        prop_assert_eq!(ours_prefix, root);
        prop_assert_eq!(theirs_prefix, root);
        prop_assert_eq!(ours_node.hash(), theirs_node.hash());
        prop_assert_eq!(ours_node.len(), theirs_node.len());
        prop_assert_eq!(ours_node.ceiling(), theirs_node.ceiling());
        prop_assert_eq!(ours_node.floor(), theirs_node.floor());

        let walked = leaves_local(root, ours_node);
        prop_assert_eq!(content(&walked), expected.clone());
        let exploded = leaves_default(root, theirs_node);
        prop_assert_eq!(content(&exploded), expected);
    }

    /// Bulk assembly cuts multi-run streams at the same boundaries as the default fold.
    ///
    /// Near the leaves, one stream carries *several* runs: Local's
    /// assembly must cut nodes at exactly the same height-two prefix
    /// boundaries, yielding the same node sequence.
    #[test]
    fn multi_run_grouping_matches_default(
        suffixes in proptest::collection::btree_set(
            (0u8..4, any::<u8>(), any::<u8>()),
            0..=40,
        ),
    ) {
        let run = leaves_at(suffixes.into_iter().map(|(run_byte, mid, low)| {
            let mut bytes = [0u8; 32];
            bytes[29] = run_byte;
            bytes[30] = mid;
            bytes[31] = low;
            bytes
        }));

        let ours = assemble_local::<S<S<Z>>>(run.clone());
        let theirs = assemble_default::<S<S<Z>>>(run);
        prop_assert_eq!(ours.len(), theirs.len());
        for ((ours_prefix, ours_node), (theirs_prefix, theirs_node)) in
            ours.into_iter().zip(theirs)
        {
            prop_assert_eq!(ours_prefix, theirs_prefix);
            prop_assert_eq!(ours_node.hash(), theirs_node.hash());
            prop_assert_eq!(ours_node.len(), theirs_node.len());
            let walked = leaves_local(ours_prefix, ours_node);
            let exploded = leaves_default(theirs_prefix, theirs_node);
            prop_assert_eq!(content(&walked), content(&exploded));
        }
    }

    /// At height zero, bulk assembly is the identity.
    ///
    /// Each leaf is its own maximal run, so assembly returns the input
    /// leaves unchanged, one node per leaf.
    #[test]
    fn leaf_height_assembly_is_identity(
        suffixes in proptest::collection::btree_set(any::<u8>(), 0..=8),
    ) {
        let run = leaves_at(suffixes.into_iter().map(|low| {
            let mut bytes = [0u8; 32];
            bytes[31] = low;
            bytes
        }));
        let expected = content(&run);
        let ours = assemble_local::<Z>(run);
        prop_assert_eq!(content(&ours), expected);
    }
}
