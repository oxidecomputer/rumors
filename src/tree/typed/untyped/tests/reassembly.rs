//! Reassembling a view from its listed children, given the view they came
//! from ([`View::rebuild`]).

use before::Party;
use proptest::collection::vec;
use proptest::prelude::*;
use proptest::sample::Index;

use crate::Version;
use crate::message::Message;
use crate::tree::arb::{arb_version, nth_party};
use crate::tree::typed::untyped::{
    Node, View,
    fan::Fan,
    memo_census::{self, Work},
};

use super::{MAX_TEST_DEPTH, TREE_LEAF_BUDGET, arb_tree, key};

/// Children as reassembly takes them: views at ascending radices.
type Listing = Vec<(u8, View)>;

/// Every level of `tree` that lists children, each as a view beside the
/// children [`View::children`] lists for it.
///
/// The tree's own handles stay held, as the walk holds the stored tree, so
/// every view shares its stored node.
fn listings(tree: Node) -> Vec<(View, Listing)> {
    let mut found = Vec::new();
    let mut stack = vec![View::from(tree)];
    while let Some(view) = stack.pop() {
        let Ok(children) = view.clone().children() else {
            continue;
        };
        let listing: Listing = children.collect();
        stack.extend(listing.iter().map(|(_, child)| child.clone()));
        found.push((view, listing));
    }
    found
}

/// The fan of the nodes `listing`'s views represent, as `branch` takes it.
fn nodes(listing: &Listing) -> Fan {
    listing
        .iter()
        .map(|(radix, child)| (*radix, child.clone().into_node()))
        .collect()
}

/// The hash of the subtree reassembly returned, if any, under the fixture
/// key.
fn hash_of(view: Option<&View>) -> Option<crate::tree::typed::Hash> {
    view.map(|view| view.hash(&key()))
}

/// The hash of a node, if any, under the fixture key.
fn node_hash(node: Option<&Node>) -> Option<crate::tree::typed::Hash> {
    node.map(|node| node.hash(&key()))
}

/// A leaf at `version`, under a path of zero bytes reaching `height`: a
/// node that can stand among children at that height.
fn leaf_at(version: Version, height: usize) -> Node {
    let leaf = Node::leaf(version, Message::new(()));
    (0..height).fold(leaf, |node, _| node.beneath(0))
}

/// A leaf whose version no generated tree holds, reaching `height`
/// ([`leaf_at`]): the generators tick the first few parties of the fork
/// chain, and this one ticks a later one.
fn foreign_leaf(height: usize) -> Node {
    let mut version = Version::new();
    version.tick(&nth_party(64));
    leaf_at(version, height)
}

/// The height of the children `view` lists: one level below it.
fn child_height(view: &View) -> usize {
    view.height() - 1
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
    /// Move the child at the drawn position to another radix ([`moved`]).
    Move(Index, u8),
}

/// Any [`Change`].
fn arb_change() -> impl Strategy<Value = Change> {
    prop_oneof![
        any::<Index>().prop_map(Change::Remove),
        any::<Index>().prop_map(Change::Replace),
        any::<u8>().prop_map(Change::Add),
        (any::<Index>(), any::<u8>()).prop_map(|(at, radix)| Change::Move(at, radix)),
    ]
}

/// `entries` with the child at `at` moved to `radix`, or to the radix after
/// it where the child already sits at `radix`, replacing any child there.
///
/// The child is the same handle under another path, so the listing
/// describes another subtree.
fn moved(entries: &mut Listing, at: Index, radix: u8) {
    let (from, child) = entries.remove(at.index(entries.len()));
    let to = if radix == from {
        radix.wrapping_add(1)
    } else {
        radix
    };
    entries.retain(|(at, _)| *at != to);
    entries.push((to, child));
    entries.sort_by_key(|(at, _)| *at);
}

/// The subtree `view` represents, in allocations of its own: equal to it,
/// yet no view of any stored node a listing yields, with its hash cold, and
/// a bare branch's bounds cold too.
///
/// It is the child a session hands reassembly when it rebuilds a child and
/// arrives back at the same contents.
fn copy_of(view: &View) -> View {
    let copy = match view.clone().into_node().into_children() {
        Ok(fan) => Node::branch(fan).expect("a node's listing is never empty"),
        Err(leaf) => Node::leaf(
            leaf.floor().clone(),
            leaf.as_leaf()
                .expect("a node without children is a leaf")
                .clone(),
        ),
    };
    View::from(copy)
}

/// `listing`, which is never empty and holds children at `height`, with
/// `change` applied.
fn changed(listing: &Listing, height: usize, change: Change) -> Listing {
    let mut entries = listing.clone();
    match change {
        Change::Remove(at) => {
            entries.remove(at.index(entries.len()));
        }
        Change::Replace(at) => {
            let at = at.index(entries.len());
            entries[at].1 = View::from(foreign_leaf(height));
        }
        Change::Add(radix) => {
            entries.retain(|(at, _)| *at != radix);
            entries.push((radix, View::from(foreign_leaf(height))));
            entries.sort_by_key(|(at, _)| *at);
        }
        Change::Move(at, radix) => moved(&mut entries, at, radix),
    }
    entries
}

proptest! {
    /// Reassembling a view from exactly the children its listing yields
    /// returns the view itself, at every level of a tree, peeled levels of
    /// compressed paths included.
    ///
    /// A stored child comes back as a whole view of the same allocation; a
    /// compressed level's child as the same stored node with one more byte
    /// hidden.
    #[test]
    fn reassembling_a_listing_returns_its_node(
        tree in (0..=MAX_TEST_DEPTH).prop_flat_map(|d| arb_tree(d, TREE_LEAF_BUDGET)),
    ) {
        for (view, listing) in listings(tree) {
            let rebuilt = View::rebuild(Some(view.clone()), listing)
                .expect("a listing is never empty");
            prop_assert!(rebuilt.same(&view), "a listed view was rebuilt: {view:?}");
        }
    }

    /// Reassembling a listing with one child removed, replaced, added, or
    /// moved to another radix builds the subtree `branch` builds from the
    /// nodes the same children represent, and never returns the original
    /// view.
    ///
    /// A moved child is the very handle the listing yielded, so only its
    /// radix tells the listing apart from the original's. Removing a node's
    /// only child deletes it, as `branch` of nothing does. The trees are at least one level deep, so each lists children.
    #[test]
    fn reassembling_a_changed_listing_builds_its_children(
        tree in (1..=MAX_TEST_DEPTH).prop_flat_map(|d| arb_tree(d, TREE_LEAF_BUDGET)),
        pick in any::<Index>(),
        change in arb_change(),
    ) {
        let listings = listings(tree);
        let (view, listing) = &listings[pick.index(listings.len())];
        let listing = changed(listing, child_height(view), change);
        let built = Node::branch(nodes(&listing));
        let rebuilt = View::rebuild(Some(view.clone()), listing);
        prop_assert_eq!(hash_of(rebuilt.as_ref()), node_hash(built.as_ref()));
        prop_assert!(rebuilt.is_none_or(|rebuilt| !rebuilt.same(view)));
    }
}

/// One edit to a listing for the seeding properties.
///
/// That is a removal, a leaf at a drawn version, reaching the listing's
/// height ([`leaf_at`]), replacing the child at a drawn position or placed
/// at a drawn radix, a child moved to another radix, or a child replaced
/// by an equal copy.
#[derive(Clone, Debug)]
enum Edit {
    /// Remove the child at the drawn position, if any child is left.
    Remove(Index),
    /// Replace the child at the drawn position with a leaf.
    Replace(Index, Version),
    /// Place a leaf at the drawn radix, replacing any child there.
    Place(u8, Version),
    /// Move the child at the drawn position to another radix ([`moved`]),
    /// if any child is left.
    Move(Index, u8),
    /// Replace the child at the drawn position with an equal subtree in
    /// allocations of its own ([`copy_of`]), if any child is left.
    Copy(Index),
}

/// Any [`Edit`], its versions drawn from several parties, so a new leaf
/// can be older, newer, or concurrent with those it joins.
fn arb_edit() -> impl Strategy<Value = Edit> {
    prop_oneof![
        any::<Index>().prop_map(Edit::Remove),
        (any::<Index>(), arb_version()).prop_map(|(at, version)| Edit::Replace(at, version)),
        (any::<u8>(), arb_version()).prop_map(|(radix, version)| Edit::Place(radix, version)),
        (any::<Index>(), any::<u8>()).prop_map(|(at, radix)| Edit::Move(at, radix)),
        any::<Index>().prop_map(Edit::Copy),
    ]
}

/// `listing`, which holds children at `height`, with `edits` applied in
/// turn; a removal from an empty listing does nothing.
fn edited(listing: &Listing, height: usize, edits: &[Edit]) -> Listing {
    let mut entries = listing.clone();
    let leaf = |version: &Version| View::from(leaf_at(version.clone(), height));
    for edit in edits {
        match edit {
            Edit::Remove(at) if !entries.is_empty() => {
                entries.remove(at.index(entries.len()));
            }
            Edit::Remove(_) => {}
            Edit::Replace(at, version) if !entries.is_empty() => {
                let at = at.index(entries.len());
                entries[at].1 = leaf(version);
            }
            Edit::Replace(..) => {}
            Edit::Place(radix, version) => {
                entries.retain(|(at, _)| at != radix);
                entries.push((*radix, leaf(version)));
                entries.sort_by_key(|(at, _)| *at);
            }
            Edit::Move(at, radix) if !entries.is_empty() => moved(&mut entries, *at, *radix),
            Edit::Move(..) => {}
            Edit::Copy(at) if !entries.is_empty() => {
                let at = at.index(entries.len());
                entries[at].1 = copy_of(&entries[at].1);
            }
            Edit::Copy(_) => {}
        }
    }
    entries
}

proptest! {
    /// Reassembling an edited listing of a warm tree leaves the new node's
    /// bounds and encoded version size equal to a direct walk's.
    ///
    /// That holds however they were reached: seeded whole, seeded at one
    /// endpoint, or left for the full computation. The node's subtree is
    /// the one `branch` builds from the nodes the children represent.
    ///
    /// Edits remove children, and add or replace them with leaves whose
    /// versions come from several parties, so the changed and removed
    /// children's spans grow, shrink, and fall outside every certificate.
    /// They also move children to other radices, and replace them with
    /// equal copies, whose spans equal those they replace and whose cold
    /// bounds seeding must compute.
    #[test]
    fn a_rebuilt_node_holds_exact_memos(
        tree in (1..=MAX_TEST_DEPTH).prop_flat_map(|d| arb_tree(d, TREE_LEAF_BUDGET)),
        pick in any::<Index>(),
        edits in vec(arb_edit(), 1..4),
    ) {
        // Published trees are warm: what seeding reads of the original side.
        tree.hash(&key());
        tree.version_bytes();
        let listings = listings(tree);
        let (view, listing) = &listings[pick.index(listings.len())];
        let listing = edited(listing, child_height(view), &edits);
        let built = Node::branch(nodes(&listing));
        let rebuilt = View::rebuild(Some(view.clone()), listing).map(View::into_node);
        prop_assert_eq!(node_hash(rebuilt.as_ref()), node_hash(built.as_ref()));
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

/// The listing of the fan `original` stores, its children shared, for
/// editing.
fn listed(original: &Node) -> Listing {
    View::from(original.clone())
        .children()
        .expect("a branch lists its fan")
        .collect()
}

/// Rebuild `original`, warmed, from `children`, asserting the rebuilt
/// node's memos equal a direct walk's.
///
/// Returns the counts of each seeding outcome the rebuild recorded, in the
/// order seeded, seeded at one endpoint, seeded with an unchanged child's
/// help, and declined.
fn seeding(original: &Node, children: Listing) -> [u64; 4] {
    original.span();
    let outcomes = [
        Work::BoundsSeeded,
        Work::BoundsHalf,
        Work::BoundsBySibling,
        Work::BoundsDeclined,
    ];
    let before = outcomes.map(memo_census::read);
    let rebuilt = View::rebuild(Some(View::from(original.clone())), children)
        .expect("children remain")
        .into_node();
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
        .map(|(radix, node)| (radix, View::from(node)))
        .collect();
    assert_eq!(seeding(&original, children), [1, 0, 0, 0]);
}
