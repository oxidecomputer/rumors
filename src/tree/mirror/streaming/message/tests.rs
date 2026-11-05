//! Listing entries against the counts a backend reports: every count a node
//! can have lists, and every count no node can have is an error naming it.

use before::Span;
use proptest::prelude::*;

use super::{CountError, Listed};
use crate::Version;
use crate::network::NetworkKeys;
use crate::tree::mirror::streaming::{ErasedNode, byte_set::ByteSet};
use crate::tree::typed::{
    ErasedPrefix, Hash, HashKey, Prefix,
    untyped::memo_census::{self, Work},
};

/// A node whose child and grandchild counts the test chooses, as a backend
/// reading corrupted storage could report them.
struct Counted {
    /// The child count reported.
    children: usize,
    /// The grandchild count reported.
    grandchildren: usize,
}

impl ErasedNode for Counted {
    /// An empty span: listing reads no version.
    fn span(&self) -> Span<'_> {
        Span::at(Version::new())
    }

    /// The default digest: listing copies it unread.
    fn hash(&self, _: &ErasedPrefix, _: &HashKey) -> Hash {
        Hash::default()
    }

    /// One leaf: listing reads no leaf count.
    fn len(&self) -> usize {
        1
    }

    /// The chosen grandchild count, wherever the node sits.
    fn grandchild_count(&self, _: &ErasedPrefix) -> usize {
        self.grandchildren
    }

    /// The chosen child count, wherever the node sits.
    fn child_count(&self, _: &ErasedPrefix) -> usize {
        self.children
    }

    /// None: listing reads no radix.
    fn child_radices(&self, _: &ErasedPrefix) -> ByteSet {
        ByteSet::EMPTY
    }
}

/// Generates a count around `limit`, the largest any node can have: every
/// count up to a little past it, and counts far past it up to `usize::MAX`.
fn arb_count(limit: usize) -> impl Strategy<Value = usize> {
    prop_oneof![0..=limit + 2, limit + 1..=usize::MAX, Just(usize::MAX),]
}

proptest! {
    /// A node lists exactly when its counts are ones a node can have: at
    /// most 256 children and 65,536 grandchildren.
    ///
    /// The entry then carries both counts, a count of zero reading as one.
    /// Otherwise listing fails with the out-of-range count, the child
    /// count's first when both are.
    #[test]
    fn listing_accepts_exactly_the_counts_a_node_can_have(
        children in arb_count(256),
        grandchildren in arb_count(65_536),
        radix in any::<u8>(),
    ) {
        let listed = Listed::of(
            Prefix::new().erase(),
            radix,
            &Counted { children, grandchildren },
            NetworkKeys::fixture().hash(),
        );
        if children > 256 {
            prop_assert_eq!(listed, Err(CountError::Children(children)));
        } else if grandchildren > 65_536 {
            prop_assert_eq!(listed, Err(CountError::Grandchildren(grandchildren)));
        } else {
            let listed = listed.expect("both counts are in range");
            prop_assert_eq!(listed.radix, radix);
            prop_assert_eq!(listed.grandchildren.get(), grandchildren.max(1));
            prop_assert_eq!(usize::from(listed.children_less_one), children.max(1) - 1);
        }
    }
}

/// The root fan of `root` as its greeting lists it, through the in-memory
/// backend.
fn root_fan(
    root: crate::tree::Root,
) -> Vec<(
    u8,
    <crate::tree::mirror::streaming::Local as crate::tree::mirror::streaming::Backend>::Erased,
)> {
    use crate::tree::mirror::streaming::{Local, Root, materialized};
    let root: Root<Local> = root.into();
    pollster::block_on(materialized::greeting_fan(&Local, root.root))
        .expect("the in-memory backend lists exactly the children it reports")
}

proptest! {
    /// A listing carries each listed node's own digest at its coordinate,
    /// under the tree's hash key, and listing a fan whose memos are warm
    /// computes a digest only for a node viewed from inside its compressed
    /// path.
    ///
    /// Publication warms every node's memo, so a greeting's root fan of
    /// stored nodes lists with no hashing. A root with a compressed path has
    /// one child, the root itself one level down, whose digest at that level
    /// no memo holds.
    #[test]
    fn a_listing_carries_each_node_s_cached_digest(
        root in crate::tree::arb::arb_tree_root(0, 2..64),
    ) {
        let keys = NetworkKeys::fixture();
        let peeled = usize::from(root.node.as_ref().is_some_and(|node| {
            node.clone().into_untyped().compressed_prefix_len() > 0
        }));
        let fan = root_fan(root);
        let at = |radix: u8| Prefix::new().erase().push(radix);
        // Reading every digest warms the memos, as publication does.
        let expected: Vec<Hash> = fan
            .iter()
            .map(|(radix, node)| node.hash(&at(*radix), keys.hash()))
            .collect();

        let before = memo_census::read(Work::Hash);
        let listing = crate::tree::mirror::streaming::materialized::fan_listing(
            Prefix::new().erase(),
            &fan,
            Some(&keys),
        )
            .expect("the in-memory backend keeps every count in range");
        let computed = memo_census::read(Work::Hash) - before;

        prop_assert!(!listing.is_empty(), "a tree of two or more leaves lists its root fan");
        prop_assert_eq!(listing.iter().map(|entry| entry.hash).collect::<Vec<_>>(), expected);
        prop_assert_eq!(computed, peeled as u64);
    }
}
