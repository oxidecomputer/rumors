//! Reassembling a node from its listed children, given the node they came
//! from ([`Node::rebuild`]).

use proptest::prelude::*;
use proptest::sample::Index;

use crate::Version;
use crate::message::Message;
use crate::tree::arb::nth_party;
use crate::tree::typed::untyped::{Node, fan::Fan};

use super::{MAX_TEST_DEPTH, TREE_LEAF_BUDGET, arb_tree};

/// Every node of `tree` that lists children, and every view those listings
/// yield, each beside the children [`Node::child_iter`] lists for it.
///
/// Each node is listed while a second handle to it is held, as the walk
/// holds the node it explodes, so a compressed node's one child is a
/// peeled copy, never the node's own allocation.
fn listings(tree: Node) -> Vec<(Node, Fan)> {
    let mut found = Vec::new();
    let mut stack = vec![tree];
    while let Some(node) = stack.pop() {
        let Ok(children) = node.clone().child_iter() else {
            continue;
        };
        let fan: Fan = children.collect();
        stack.extend(fan.values().cloned());
        found.push((node, fan));
    }
    found
}

/// A leaf whose version no generated tree holds: the generators tick the
/// first few parties of the fork chain, and this one ticks a later one.
fn foreign_leaf() -> Node {
    let mut version = Version::new();
    version.tick(&nth_party(64));
    Node::leaf(version, Message::new(()))
}

/// One change to a listing that changes the subtree it describes.
#[derive(Clone, Copy, Debug)]
enum Change {
    /// Remove the child at the drawn position.
    Remove(Index),
    /// Replace the child at the drawn position with a leaf no tree holds.
    Replace(Index),
    /// Add a leaf no tree holds at the drawn radix, replacing the child
    /// there if there is one.
    Add(u8),
}

/// Any [`Change`].
fn arb_change() -> impl Strategy<Value = Change> {
    prop_oneof![
        any::<Index>().prop_map(Change::Remove),
        any::<Index>().prop_map(Change::Replace),
        any::<u8>().prop_map(Change::Add),
    ]
}

/// `fan`, which is never empty, with `change` applied.
fn changed(fan: &Fan, change: Change) -> Fan {
    let mut entries: Vec<(u8, Node)> = fan
        .iter()
        .map(|(radix, child)| (radix, child.clone()))
        .collect();
    match change {
        Change::Remove(at) => {
            entries.remove(at.index(entries.len()));
        }
        Change::Replace(at) => {
            let at = at.index(entries.len());
            entries[at].1 = foreign_leaf();
        }
        Change::Add(radix) => entries.push((radix, foreign_leaf())),
    }
    // Collecting keeps the last entry at a repeated radix: the added leaf.
    entries.into_iter().collect()
}

proptest! {
    /// Reassembling a node from exactly the children its listing yields
    /// returns the node itself, at every node of a tree and at every peeled
    /// view of a compressed path.
    ///
    /// A stored child comes back as the same allocation; a compressed
    /// node's child is a peeled copy, which reassembly must recognize by
    /// its path and body.
    #[test]
    fn reassembling_a_listing_returns_its_node(
        tree in (0..=MAX_TEST_DEPTH).prop_flat_map(|d| arb_tree(d, TREE_LEAF_BUDGET)),
    ) {
        for (node, fan) in listings(tree) {
            let rebuilt = Node::rebuild(Some(node.clone()), fan)
                .expect("a listing is never empty");
            prop_assert!(rebuilt.ptr_eq(&node), "a listed node was rebuilt: {node:?}");
        }
    }

    /// Reassembling a listing with one child removed, replaced, or added
    /// builds the subtree `branch` builds from the same children, and never
    /// returns the original node.
    ///
    /// Removing a node's only child deletes it, as `branch` of nothing
    /// does. The trees are at least one level deep, so each lists children.
    #[test]
    fn reassembling_a_changed_listing_builds_its_children(
        tree in (1..=MAX_TEST_DEPTH).prop_flat_map(|d| arb_tree(d, TREE_LEAF_BUDGET)),
        pick in any::<Index>(),
        change in arb_change(),
    ) {
        let listings = listings(tree);
        let (node, fan) = &listings[pick.index(listings.len())];
        let fan = changed(fan, change);
        let rebuilt = Node::rebuild(Some(node.clone()), fan.clone());
        let built = Node::branch(fan);
        prop_assert_eq!(rebuilt.as_ref().map(Node::hash), built.as_ref().map(Node::hash));
        prop_assert!(rebuilt.is_none_or(|rebuilt| !rebuilt.ptr_eq(node)));
    }
}
