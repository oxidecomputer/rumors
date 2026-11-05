//! Views against the copies peeling a shared node builds.

use proptest::prelude::*;
use proptest::test_runner::TestCaseError;

use crate::tree::arb::arb_leaf_paths;
use crate::tree::typed::untyped::tests::{
    MAX_TEST_DEPTH, TREE_LEAF_BUDGET, arb_tree, leaves_below,
};

use super::{Node, PATH_LEN, View};

/// A tree as the untyped suites generate them: up to [`MAX_TEST_DEPTH`]
/// levels deep, or built from leaves at whole 32-byte paths, whose
/// compressed paths reach every length up to the whole key.
fn arb_any_tree() -> impl Strategy<Value = Node> {
    prop_oneof![
        (0..=MAX_TEST_DEPTH).prop_flat_map(|depth| arb_tree(depth, TREE_LEAF_BUDGET)),
        arb_leaf_paths(64).prop_map(|paths| {
            let mut entries: Vec<_> = leaves_below([0; 32], 0, paths)
                .into_iter()
                .map(|(path, leaf)| (path, Some(leaf)))
                .collect();
            Node::from_sorted_leaves(0, &mut entries)
        }),
    ]
}

/// Visit every level of `tree`'s listings, root first, each as a view, as
/// the node [`Node::into_children`] builds for it, and with its coordinate,
/// with `visit`.
///
/// The tree's own handles stay held throughout, as the walk holds the
/// stored tree while it lists, so each compressed level of the copies is a
/// copy of a shared node. The two listings must agree on the radices they
/// yield; the pairs beneath are visited in turn. A level's coordinate is
/// the path from the root to it: the root's is zeros, as many as the path
/// bytes above the tree's height, and each child's extends its parent's by
/// its radix. The traversal keeps an explicit stack, so it never recurses
/// on the tree's shape.
fn visit_levels(
    tree: &Node,
    mut visit: impl FnMut(&View, &Node, &[u8]) -> Result<(), TestCaseError>,
) -> Result<(), TestCaseError> {
    let root = vec![0; PATH_LEN - tree.height()];
    let mut stack = vec![(View::from(tree.clone()), tree.clone(), root)];
    while let Some((view, copy, at)) = stack.pop() {
        visit(&view, &copy, &at)?;
        match (view.clone().children(), copy.clone().into_children()) {
            (Err(_), Err(_)) => {}
            (Ok(views), Ok(copies)) => {
                let views: Vec<_> = views.collect();
                let copies: Vec<_> = copies.into_iter().collect();
                prop_assert!(
                    views
                        .iter()
                        .map(|(radix, _)| radix)
                        .eq(copies.iter().map(|(radix, _)| radix)),
                    "{:?}",
                    view
                );
                stack.extend(
                    views
                        .into_iter()
                        .zip(copies)
                        .map(|((radix, view), (_, copy))| {
                            let mut child = at.clone();
                            child.push(radix);
                            (view, copy, child)
                        }),
                );
            }
            _ => prop_assert!(
                false,
                "a view and its copy disagree on being a leaf: {view:?}"
            ),
        }
    }
    Ok(())
}

/// The key bytes of the level `at` and of the levels down through
/// `view`'s body: `at`, then the path bytes the view leaves visible,
/// shallowest first.
///
/// Its first `PATH_LEN - h` bytes are the coordinate of the level at
/// height `h`, for every height from the view's own down to its body's.
fn key_through_body(view: &View, at: &[u8]) -> Vec<u8> {
    let prefix = &view.node.inner.prefix;
    let visible = &prefix[..prefix.len() - view.peeled.bytes()];
    at.iter()
        .copied()
        .chain(visible.iter().rev().copied())
        .collect()
}

proptest! {
    /// A view placed from its stored node and the height or coordinate of
    /// the level it is listed at is the view listing yields there, and
    /// answers each query as the copy peeling builds, at every level of an
    /// arbitrary tree.
    ///
    /// The stored handles keep no peel: the session derives each from the
    /// height its handle is tagged with, or from the coordinate it keeps
    /// beside an erased one. So the derivation must recover listing's own
    /// peel, the same allocation hiding the same bytes, at every peel depth
    /// of every compressed path. The derived view must then answer every
    /// query as the copy: the child count, radices, grandchild count, and
    /// hash, from the visible path, and the leaf count, span and encoded
    /// version size, from the stored node it shares. Building the node it
    /// represents gives that copy's subtree: the same compressed path over
    /// the same body.
    #[test]
    fn a_placed_view_is_the_listed_view_and_answers_as_the_copy(tree in arb_any_tree()) {
        visit_levels(&tree, |listed, copy, at| {
            let height = PATH_LEN - at.len();
            let derived = View::at_height(listed.stored().clone(), height)
                .expect("a listed level is one of its stored node's levels");
            prop_assert!(derived.same(listed), "{:?} at height {}", listed, height);
            let placed = View::placed(listed.stored(), at)
                .expect("a listed level's coordinate lies on its stored node's path");
            prop_assert!(placed.same(listed), "{:?} at {}", listed, hex::encode(at));
            prop_assert_eq!(derived.height(), height);

            // A whole view of the copy reads the copy's own path.
            let whole = View::from(copy.clone());
            prop_assert_eq!(placed.child_count(), whole.child_count());
            prop_assert!(placed.child_radices().eq(whole.child_radices()));
            prop_assert_eq!(placed.grandchild_count(), whole.grandchild_count());
            prop_assert_eq!(placed.hash(), copy.hash());
            prop_assert_eq!(placed.stored().len(), copy.len());
            prop_assert_eq!(placed.stored().span(), copy.span());
            prop_assert_eq!(placed.stored().version_bytes(), copy.version_bytes());
            let built = derived.into_node();
            prop_assert_eq!(built.compressed_prefix_len(), copy.compressed_prefix_len());
            prop_assert_eq!(built.hash(), copy.hash());
            Ok(())
        })?;
    }

    /// Placing a stored node accepts exactly the coordinates on its own
    /// path, at every level of an arbitrary tree.
    ///
    /// For each level, with the stored node's body at height `b` and its
    /// top at `T`, the coordinates of every height from 0 to the whole key
    /// are tried, each spelled by the level's key through the body
    /// (zero-padded below the body): placing accepts exactly the heights
    /// from `b` through `T`, each as the view at that height. Then each
    /// byte of the level's own coordinate is changed in turn: placing still
    /// accepts a change above the node's top, at a position the node does
    /// not store, and rejects every change among the path bytes the view
    /// hides. So no height outside the node's levels, and no coordinate
    /// disagreeing with a hidden path byte, places a view.
    #[test]
    fn placing_accepts_exactly_the_coordinates_on_the_nodes_path(tree in arb_any_tree()) {
        visit_levels(&tree, |listed, _, at| {
            let stored = listed.stored();
            let (body, top) = (stored.height() - stored.compressed_prefix_len(), stored.height());
            let mut key = key_through_body(listed, at);
            key.resize(PATH_LEN, 0);
            for height in 0..=PATH_LEN {
                let placed = View::placed(stored, &key[..PATH_LEN - height]);
                prop_assert_eq!(placed.is_some(), (body..=top).contains(&height),
                    "{:?} at height {}", listed, height);
                if let Some(placed) = placed {
                    let derived = View::at_height(stored, height)
                        .expect("a placed height is one of the node's levels");
                    prop_assert!(placed.same(&derived));
                }
            }
            let mut wrong = at.to_vec();
            for position in 0..at.len() {
                wrong[position] ^= 1;
                let above_top = position < PATH_LEN - top;
                prop_assert_eq!(View::placed(stored, &wrong).is_some(), above_top,
                    "{:?} at {} changed at {}", listed, hex::encode(at), position);
                wrong[position] ^= 1;
            }
            Ok(())
        })?;
    }

    /// Listing a view's children allocates nothing at any level of an
    /// arbitrary tree, though the tree shares every node it lists.
    ///
    /// Building the node a peeled view of a shared node represents does
    /// allocate, which shows the meter counting the copy listing avoids.
    #[test]
    fn listing_a_view_allocates_nothing(tree in arb_any_tree()) {
        visit_levels(&tree, |view, _, _| {
            let (listed, _) = alloc_meter::measure(|| {
                view.clone().children().map_or(0, Iterator::count)
            });
            prop_assert_eq!(listed.bytes_allocated, 0, "{:?}", view);
            if view.peeled != super::Peel::P0 {
                let (built, _) = alloc_meter::measure(|| view.clone().into_node());
                prop_assert!(built.bytes_allocated > 0, "{:?}", view);
            }
            Ok(())
        })?;
    }

    /// Walking a warm tree's leaves as height-zero views allocates as much
    /// as walking a one-leaf tree's: nothing per leaf.
    ///
    /// The walk allocates its own descent buffer alone, though every leaf
    /// of a full-height tree sits under a compressed path the tree keeps.
    ///
    /// Published trees are warm; a cold branch computes its bounds on the
    /// walk's first read of them, which allocates per branch, not per
    /// leaf.
    #[test]
    fn walking_leaves_allocates_nothing_per_leaf(paths in arb_leaf_paths(64)) {
        let build = |paths| {
            let mut entries: Vec<_> = leaves_below([0; 32], 0, paths)
                .into_iter()
                .map(|(path, leaf)| (path, Some(leaf)))
                .collect();
            let tree = Node::from_sorted_leaves(0, &mut entries);
            tree.version_bytes();
            tree
        };
        let walk = |tree: &Node| {
            alloc_meter::measure(|| {
                View::from(tree.clone())
                    .leaves(&[])
                    .fold(0, |walked, (_, leaf)| {
                        // Each leaf becomes its height-zero view, as the
                        // backend yields it.
                        drop(leaf.into_view());
                        walked + 1
                    })
            })
        };
        let lone = build([[0; 32]].into_iter().collect());
        let tree = build(paths.clone());
        let (one, _) = walk(&lone);
        let (all, walked) = walk(&tree);
        prop_assert_eq!(walked, paths.len());
        prop_assert_eq!(all.bytes_allocated, one.bytes_allocated);
    }
}
