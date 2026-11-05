//! Views against the copies peeling a shared node builds.

use proptest::prelude::*;
use proptest::test_runner::TestCaseError;

use crate::tree::arb::arb_leaf_paths;
use crate::tree::typed::untyped::tests::{
    MAX_TEST_DEPTH, TREE_LEAF_BUDGET, arb_tree, leaves_below,
};

use super::{Node, View};

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

/// Visit every level of `tree`'s listings, root first, each as a view and
/// as the node [`Node::into_children`] builds for it, with `visit`.
///
/// The tree's own handles stay held throughout, as the walk holds the
/// stored tree while it lists, so each compressed level of the copies is a
/// copy of a shared node. The two listings must agree on the radices they
/// yield; the pairs beneath are visited in turn. The traversal keeps an
/// explicit stack, so it never recurses on the tree's shape.
fn visit_levels(
    tree: &Node,
    mut visit: impl FnMut(&View, &Node) -> Result<(), TestCaseError>,
) -> Result<(), TestCaseError> {
    let mut stack = vec![(View::from(tree.clone()), tree.clone())];
    while let Some((view, copy)) = stack.pop() {
        visit(&view, &copy)?;
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
                        .map(|((_, view), (_, copy))| (view, copy)),
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

proptest! {
    /// A view answers each query as the copy peeling builds, at every level
    /// of an arbitrary tree.
    ///
    /// That covers every peel depth of every compressed path, and every
    /// query: the child count, radices, grandchild count, leaf count, span,
    /// encoded version size and hash. Building the node the view represents gives that copy's subtree:
    /// the same compressed path over the same body.
    #[test]
    fn a_view_answers_as_the_copy_peeling_builds(tree in arb_any_tree()) {
        visit_levels(&tree, |view, copy| {
            // A whole view of the copy reads the copy's own path.
            let whole = View::from(copy.clone());
            prop_assert_eq!(view.child_count(), whole.child_count());
            prop_assert!(view.child_radices().eq(whole.child_radices()));
            prop_assert_eq!(view.grandchild_count(), whole.grandchild_count());
            prop_assert_eq!(view.len(), copy.len());
            prop_assert_eq!(view.span(), copy.span());
            prop_assert_eq!(view.version_bytes(), copy.version_bytes());
            prop_assert_eq!(view.hash(), copy.hash());
            let built = view.clone().into_node();
            prop_assert_eq!(built.compressed_prefix_len(), copy.compressed_prefix_len());
            prop_assert_eq!(built.hash(), copy.hash());
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
        visit_levels(&tree, |view, _| {
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
