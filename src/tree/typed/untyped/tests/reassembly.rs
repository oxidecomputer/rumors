//! Reassembling a node from its listed children, given the node they came
//! from ([`Node::rebuild`]).

use before::Party;
use proptest::collection::vec;
use proptest::prelude::*;
use proptest::sample::Index;

use crate::Version;
use crate::message::Message;
use crate::tree::arb::{arb_version, nth_party};
use crate::tree::typed::untyped::{
    Node,
    fan::Fan,
    memo_census::{self, Work},
};

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

/// One edit to a listing for the seeding properties: a removal, or a leaf
/// at a drawn version replacing the child at a drawn position or placed at
/// a drawn radix.
#[derive(Clone, Debug)]
enum Edit {
    /// Remove the child at the drawn position, if any child is left.
    Remove(Index),
    /// Replace the child at the drawn position with a leaf.
    Replace(Index, Version),
    /// Place a leaf at the drawn radix, replacing any child there.
    Place(u8, Version),
}

/// Any [`Edit`], its versions drawn from several parties, so a new leaf
/// can be older, newer, or concurrent with those it joins.
fn arb_edit() -> impl Strategy<Value = Edit> {
    prop_oneof![
        any::<Index>().prop_map(Edit::Remove),
        (any::<Index>(), arb_version()).prop_map(|(at, version)| Edit::Replace(at, version)),
        (any::<u8>(), arb_version()).prop_map(|(radix, version)| Edit::Place(radix, version)),
    ]
}

/// `fan` with `edits` applied in turn; a removal from an empty fan does
/// nothing.
fn edited(fan: &Fan, edits: &[Edit]) -> Fan {
    let mut entries: Vec<(u8, Node)> = fan
        .iter()
        .map(|(radix, child)| (radix, child.clone()))
        .collect();
    for edit in edits {
        match edit {
            Edit::Remove(at) if !entries.is_empty() => {
                entries.remove(at.index(entries.len()));
            }
            Edit::Remove(_) => {}
            Edit::Replace(at, version) if !entries.is_empty() => {
                let at = at.index(entries.len());
                entries[at].1 = Node::leaf(version.clone(), Message::new(()));
            }
            Edit::Replace(..) => {}
            Edit::Place(radix, version) => {
                entries.retain(|(at, _)| at != radix);
                entries.push((*radix, Node::leaf(version.clone(), Message::new(()))));
            }
        }
    }
    entries.into_iter().collect()
}

proptest! {
    /// Reassembling an edited listing of a warm tree leaves the new node's
    /// bounds and encoded version size equal to a direct walk's.
    ///
    /// That holds however they were reached: seeded whole, seeded at one
    /// endpoint, or left for the full computation. The node's subtree is
    /// the one `branch` builds.
    ///
    /// Edits remove children, and add or replace them with leaves whose
    /// versions come from several parties, so the changed and removed
    /// children's spans grow, shrink, and fall outside every certificate.
    #[test]
    fn a_rebuilt_node_holds_exact_memos(
        tree in (1..=MAX_TEST_DEPTH).prop_flat_map(|d| arb_tree(d, TREE_LEAF_BUDGET)),
        pick in any::<Index>(),
        edits in vec(arb_edit(), 1..4),
    ) {
        // Published trees are warm: what seeding reads of the original side.
        tree.hash();
        tree.version_bytes();
        let listings = listings(tree);
        let (node, fan) = &listings[pick.index(listings.len())];
        let fan = edited(fan, &edits);
        let rebuilt = Node::rebuild(Some(node.clone()), fan.clone());
        let built = Node::branch(fan);
        prop_assert_eq!(rebuilt.as_ref().map(Node::hash), built.as_ref().map(Node::hash));
        if let Some(rebuilt) = rebuilt {
            prop_assert!(rebuilt.mismemoized().is_none(), "memos drifted: {rebuilt:?}");
        }
    }
}

/// Successive versions of one party, each after the last.
fn chain(count: usize) -> Vec<Version> {
    let party = Party::seed();
    let mut version = Version::new();
    (0..count)
        .map(|_| {
            version.tick(&party);
            version.clone()
        })
        .collect()
}

/// One version each from `count` parties forked from one seed, pairwise
/// concurrent.
fn antichain(count: usize) -> Vec<Version> {
    let mut seed = Party::seed();
    (0..count)
        .map(|_| {
            let mut version = Version::new();
            version.tick(&seed.fork());
            version
        })
        .collect()
}

/// A fan of leaves at `versions`, the `i`th at radix `i`.
fn leaves(versions: &[Version]) -> Fan {
    versions
        .iter()
        .enumerate()
        .map(|(radix, version)| {
            let radix = u8::try_from(radix).expect("a fan holds at most 256 children");
            (radix, Node::leaf(version.clone(), Message::new(())))
        })
        .collect()
}

/// The fan `original` stores, its children shared, for editing.
fn listed(original: &Node) -> Fan {
    original
        .stored_fan()
        .expect("a branch stores a fan")
        .clone()
}

/// Rebuild `original`, warmed, from `children`, asserting the rebuilt
/// node's memos equal a direct walk's.
///
/// Returns the counts of each seeding outcome the rebuild recorded, in the
/// order seeded, seeded at one endpoint, seeded with an unchanged child's
/// help, and declined.
fn seeding(original: &Node, children: Fan) -> [u64; 4] {
    original.span();
    let outcomes = [
        Work::BoundsSeeded,
        Work::BoundsHalf,
        Work::BoundsBySibling,
        Work::BoundsDeclined,
    ];
    let before = outcomes.map(memo_census::read);
    let rebuilt = Node::rebuild(Some(original.clone()), children).expect("children remain");
    let after = outcomes.map(memo_census::read);
    assert!(
        rebuilt.mismemoized().is_none(),
        "memos drifted: {rebuilt:?}"
    );
    std::array::from_fn(|at| after[at] - before[at])
}

/// Removing the middle of three successive leaves seeds both endpoints from
/// the unchanged leaves: the oldest certifies the floor and the newest the
/// ceiling, though no changed child is left to certify either.
#[test]
fn a_removal_covered_by_unchanged_siblings_seeds_by_their_certificates() {
    let versions = chain(3);
    let original = Node::branch(leaves(&versions)).expect("three leaves");
    let mut children = listed(&original);
    children.remove(1);
    assert_eq!(seeding(&original, children), [1, 0, 1, 0]);
}

/// Removing the newest of three successive leaves lowers the ceiling, which
/// nothing certifies: the floor is seeded from the oldest leaf, and the
/// ceiling folded from the children left.
#[test]
fn a_removal_covered_by_nothing_folds_its_endpoint() {
    let versions = chain(3);
    let original = Node::branch(leaves(&versions)).expect("three leaves");
    let mut children = listed(&original);
    children.remove(2);
    assert_eq!(seeding(&original, children), [0, 1, 1, 0]);
}

/// Removing one of three concurrent leaves certifies neither endpoint, so
/// the bounds are left for the full computation.
#[test]
fn a_removal_from_an_antichain_declines_to_seed() {
    let versions = antichain(3);
    let original = Node::branch(leaves(&versions)).expect("three leaves");
    let mut children = listed(&original);
    children.remove(1);
    assert_eq!(seeding(&original, children), [0, 0, 0, 1]);
}

/// Removing a leaf while adding an older and a newer one seeds both
/// endpoints from the added leaves alone: their union contains the removed
/// leaf, and the one unchanged leaf, concurrent with all of them, certifies
/// nothing.
#[test]
fn a_removal_covered_by_the_changed_children_seeds_without_siblings() {
    let mut seed = Party::seed();
    let (writer, other) = (seed.fork(), seed.fork());
    let mut version = Version::new();
    let [older, removed, newer] = [(); 3].map(|()| {
        version.tick(&writer);
        version.clone()
    });
    let mut concurrent = Version::new();
    concurrent.tick(&other);
    let leaf = |version: Version| Node::leaf(version, Message::new(()));
    let kept = leaf(concurrent);
    let original = Node::branch(
        [(0, kept.clone()), (1, leaf(removed))]
            .into_iter()
            .collect(),
    )
    .expect("two leaves");
    let children = [(0, kept), (2, leaf(older)), (3, leaf(newer))]
        .into_iter()
        .collect();
    assert_eq!(seeding(&original, children), [1, 0, 0, 0]);
}
