//! `Local`'s bulk `leaves`/`assemble` overrides against the default chain,
//! and its check of the coordinates erased handles are re-tagged and read
//! at.
//!
//! The overrides skip per-level work along compressed paths. Their required
//! property is observational equivalence: for deep paths, dense branches, and
//! multiple same-height runs, each override must produce the same stream as
//! the level-by-level [`Convert`] default.
//!
//! A handle hides as many of its stored node's path bytes as its height
//! decides, so the coordinate checks are what keep a wrong height from
//! selecting other bytes; which coordinates they accept is a property of
//! the untyped view's placement, tested there. These tests show the backend
//! panics on one it rejects.

use std::collections::BTreeSet;
use std::convert::Infallible;

use futures::{TryStreamExt, stream};
use proptest::prelude::*;

use crate::{
    Version,
    message::Message,
    tree::{
        arb::{arb_leaf_paths, nth_party},
        mirror::streaming::{
            Backend, ErasedNode, Summary, backend::BoxNodeStream, convert::Convert, ops,
        },
        typed::{
            self, ErasedPrefix, HashKey, Path, Prefix,
            height::{self, S, Z},
        },
    },
};

use super::Local;

/// An ordered sequence of prefix-keyed leaves.
type LeafRun = Vec<(Prefix<Z>, typed::View<Z>)>;

/// One distinct-version leaf per path, in the (ascending) order given.
fn leaves_at(paths: impl IntoIterator<Item = [u8; 32]>) -> LeafRun {
    let mut version = Version::new();
    paths
        .into_iter()
        .map(|bytes| {
            version.tick(&nth_party(0));
            (
                Path::from(bytes).into(),
                typed::View::leaf(version.clone(), Message::new(())),
            )
        })
        .collect()
}

/// Convert an owned leaf run into the backend's erased stream type.
fn boxed(run: LeafRun) -> BoxNodeStream<'static, Local, Z> {
    Box::pin(stream::iter(run.into_iter().map(Ok::<_, Infallible>)))
}

/// Assemble through the level-by-level default, bypassing Local's override.
fn assemble_default<H: Convert>(run: LeafRun) -> Vec<(Prefix<H>, typed::View<H>)> {
    pollster::block_on(H::assemble(Local, boxed(run)).try_collect())
        .unwrap_or_else(|error| match error {})
}

/// Assemble through `Local`'s bulk override.
fn assemble_local<H: Convert>(run: LeafRun) -> Vec<(Prefix<H>, typed::View<H>)> {
    pollster::block_on(Local.assemble::<H>(boxed(run)).try_collect())
        .unwrap_or_else(|error| match error {})
}

/// Explode through the level-by-level default, bypassing Local's override.
fn leaves_default<H: Convert>(prefix: Prefix<H>, node: typed::View<H>) -> LeafRun {
    pollster::block_on(
        H::explode::<_>(
            Local,
            Box::pin(stream::once(async move { Ok((prefix, node)) })),
        )
        .try_collect(),
    )
    .unwrap_or_else(|error| match error {})
}

/// Walk leaves through `Local`'s direct override.
fn leaves_local<H: Convert>(prefix: Prefix<H>, node: typed::View<H>) -> LeafRun {
    pollster::block_on(<Local as Backend>::leaves(Local, prefix, node).try_collect())
        .unwrap_or_else(|error| match error {})
}

/// The observable content of a leaf run: full path plus version (bare leaf
/// *hashes* commit nothing but their empty suffix — content is committed by
/// path — so equality must compare versions, not hashes).
fn content(run: &LeafRun) -> Vec<(Prefix<Z>, Version)> {
    run.iter()
        .map(|(prefix, leaf)| (*prefix, leaf.span().hi().clone()))
        .collect()
}

proptest! {
    /// Bulk assembly of one full-height run matches the default fold.
    ///
    /// Same hash (structure), same leaf count, same version bounds, same
    /// `version_bytes` aggregate (not serialized, so this equivalence is
    /// its only bulk-path coverage) — and both walks (Local's direct one
    /// and the default explosion) return exactly the input leaves, in
    /// order.
    #[test]
    fn full_height_roundtrip_matches_default(paths in arb_leaf_paths(24)) {
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
        prop_assert_eq!(ours_node.hash(&HashKey::fixture()), theirs_node.hash(&HashKey::fixture()));
        prop_assert_eq!(ours_node.len(), theirs_node.len());
        prop_assert_eq!(ours_node.span(), theirs_node.span());
        prop_assert_eq!(ours_node.version_bytes(), theirs_node.version_bytes());

        let walked = leaves_local(root, ours_node);
        prop_assert_eq!(content(&walked), expected.clone());
        let exploded = leaves_default(root, theirs_node);
        prop_assert_eq!(content(&exploded), expected);
    }

    /// Bulk assembly cuts multi-run streams at the same boundaries as the default fold.
    ///
    /// Near the leaves, one stream carries *several* runs: Local's
    /// assembly must cut nodes at exactly the same height-two prefix
    /// boundaries, yielding the same node sequence — same hash, leaf
    /// count, and `version_bytes` aggregate per node.
    #[test]
    fn multi_run_grouping_matches_default(
        suffixes in proptest::collection::vec((0u8..4, any::<u8>(), any::<u8>()), 0..=40)
            .prop_map(BTreeSet::from_iter),
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
            prop_assert_eq!(ours_node.hash(&HashKey::fixture()), theirs_node.hash(&HashKey::fixture()));
            prop_assert_eq!(ours_node.len(), theirs_node.len());
            prop_assert_eq!(ours_node.version_bytes(), theirs_node.version_bytes());
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
        suffixes in proptest::collection::vec(any::<u8>(), 0..=8).prop_map(BTreeSet::from_iter),
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

/// A leaf two levels below its top, under the path bytes `[TOP, NEXT]`,
/// listed one level down: the handle at height one, which hides `TOP`,
/// erased, with its coordinate.
///
/// The tree above it is all zero bytes, so its coordinate is zeros ending
/// in `TOP`.
fn erased_inside_its_path() -> (Prefix<S<Z>>, <Local as Backend>::Erased) {
    let leaf = typed::Node::leaf(Version::new(), Message::new(()));
    let node = typed::Node::beneath(typed::Node::beneath(leaf, NEXT), TOP);
    let top = Prefix::<S<S<Z>>>::containing(&Path::from([0; 32]));
    let mut children = typed::View::from(node).children();
    let (radix, child) = children.next().expect("a compressed node lists one child");
    assert_eq!(radix, TOP, "the child sits under the top path byte");
    (top.push(radix), <Local as Backend>::erase(child))
}

/// The top path byte of [`erased_inside_its_path`]'s node.
const TOP: u8 = 0x5a;

/// The path byte beneath [`TOP`].
const NEXT: u8 = 0x7c;

/// Re-tagging an erased handle at its own coordinate restores the handle
/// listing yielded, and at a wrong height panics, even a height within the
/// node's own levels.
///
/// Height zero lies between the leaf's body and its top, so only the
/// coordinate's bytes can tell it wrong: the handle there would hide
/// `NEXT` as well, and the coordinate claims another byte in its place.
///
/// The check runs in debug builds alone, so builds without debug
/// assertions ignore the test.
#[test]
#[cfg_attr(
    not(debug_assertions),
    ignore = "the coordinate check runs in debug builds only"
)]
#[should_panic(expected = "must sit on its own path")]
fn re_tagging_at_a_wrong_height_inside_the_path_panics() {
    let (at, erased) = erased_inside_its_path();
    let restored = <Local as Backend>::assume(&at, erased.clone());
    assert_eq!(
        restored.grandchild_count(),
        1,
        "the restored handle sees NEXT"
    );
    let _wrong = <Local as Backend>::assume(&at.push(NEXT ^ 1), erased);
}

/// Re-tagging an erased handle at a height above its stored node's top
/// panics: the node has no path byte to hide there.
///
/// The check runs in debug builds alone, so builds without debug
/// assertions ignore the test.
#[test]
#[cfg_attr(
    not(debug_assertions),
    ignore = "the coordinate check runs in debug builds only"
)]
#[should_panic(expected = "must sit on its own path")]
fn re_tagging_above_the_nodes_top_panics() {
    let (at, erased) = erased_inside_its_path();
    let (top, _) = at.pop();
    let (above, _) = top.pop();
    let _wrong = <Local as Backend>::assume(&above, erased);
}

/// Reading what an erased handle's children are at a coordinate off its
/// stored node's path panics, as re-tagging there does.
///
/// The check runs in debug builds alone, so builds without debug
/// assertions ignore the test.
#[test]
#[cfg_attr(
    not(debug_assertions),
    ignore = "the coordinate check runs in debug builds only"
)]
#[should_panic(expected = "must sit on its own path")]
fn reading_at_a_coordinate_off_the_nodes_path_panics() {
    let (at, erased) = erased_inside_its_path();
    assert_eq!(
        erased.child_count(&at.erase()),
        1,
        "one path byte is visible"
    );
    let (parent, _) = at.pop();
    let _wrong = erased.hash(&parent.push(TOP ^ 1).erase(), &HashKey::fixture());
}

proptest! {
    /// At every level of every tree, the in-memory backend's one-placement
    /// listing summary agrees with the three queries it stands for: the
    /// digest, the child count and the grandchild count at that coordinate.
    #[test]
    fn the_summary_agrees_with_its_three_queries(
        root in crate::tree::arb::arb_tree_root(0, 1..32),
    ) {
        let key = HashKey::fixture();
        let root: crate::tree::mirror::streaming::Root<Local> = root.into();
        let mut stack: Vec<(ErasedPrefix, <Local as Backend>::Erased)> = root
            .root
            .into_iter()
            .map(|node| (Prefix::<height::Root>::new().erase(), Local::erase(node)))
            .collect();
        while let Some((at, node)) = stack.pop() {
            prop_assert_eq!(
                node.summary(&at, &key),
                Summary {
                    hash: node.hash(&at, &key),
                    child_count: node.child_count(&at),
                    grandchildren: node.grandchild_count(&at),
                }
            );
            if at.height() > 0 {
                let children = pollster::block_on(ops::children_of(&Local, at, node))
                    .expect("the in-memory backend lists exactly the children it reports");
                stack.extend(children.into_iter().map(|(radix, child)| (at.push(radix), child)));
            }
        }
    }
}
