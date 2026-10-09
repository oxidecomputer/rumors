//! Tick inputs whose party and version are built together, region by region.
//!
//! [`arb_oracle_party_nonempty`](super::arb_oracle_party_nonempty) and
//! [`arb_oracle_version`](super::arb_oracle_version) draw a tick's two operands
//! independently, so a party branch seldom sits over a version branch, and
//! almost never several levels running. The tick walk keeps its most intricate
//! state exactly there. A *lookahead site* is a party branch whose left child
//! is wholly owned and whose right child is partially owned, over a version
//! branch. Simplifying the owned left child needs the simplified minimum of its
//! right sibling, which lies later in the stream. So the first site the walk
//! meets outside any pre-scanned range starts a pre-scan, which records that
//! minimum, and the minimum of every site nested in the sibling's range, in a
//! memo of differences between consecutive minima. A defect in that state can
//! stay hidden until one pre-scan covers several sites: nested three or more
//! deep, two or more inside one range, more than one memo block's worth, or
//! several such pre-scans in one walk.
//!
//! Each strategy here describes a case as a tree of regions, each fixing the
//! party and the version together over one interval, and draws version leaf
//! heights from a small per-case *palette* of absolute heights. Small palettes
//! make equal minima common; palette entries just past `2^64` and `2^130` make
//! near-ties common at wide scale.
//!
//! | Strategy | What it reaches |
//! |---|---|
//! | [`arb_spine_case`] | long chains of nested sites, and ranges holding several sites |
//! | [`arb_wide_case`] | one pre-scan that reserves more than one memo block |
//! | [`arb_multi_scan_case`] | several such pre-scans in one walk, reusing memo blocks |
//!
//! Nothing is rejected: every region resolves through the recursive oracle's
//! normalizing constructors, so each case is a normal-form pair by
//! construction. The census in this module's tests holds a liveness floor
//! under each strategy's regime.
//!
//! A wide or multi-scan case costs many spine cases, so their properties run
//! a fraction of the configured case count, set by [`divided_config`].

mod tests;

use core::ops::{Range, RangeInclusive};

use num_bigint::BigUint;
use proptest::prelude::*;

use crate::recurse::descend;
use crate::testing::oracles::tree;

/// Proptest's default case count, at which the gate runs every property.
pub(crate) const DEFAULT_CASES: u32 = 256;

/// The case-count divisor for [`arb_wide_case`]'s property.
///
/// A wide case ticks a tree of hundreds of regions, many times the work of a
/// spine case.
pub(crate) const WIDE_CASE_DIVISOR: u32 = 6;

/// The case-count divisor for [`arb_multi_scan_case`]'s property.
///
/// A multi-scan case joins several wide trees ([`MULTI_SCANS`]), and a failing
/// case's shrink must still finish inside nextest's time limit on a loaded
/// machine.
pub(crate) const MULTI_SCAN_CASE_DIVISOR: u32 = 12;

/// Exclusive bound on the number of levels in [`arb_spine_case`]'s spine.
const SPINE_LEVELS: usize = 48;

/// Probability that a spine level continues into its right child.
///
/// Continuing right puts the level's side region on the left, where an owned
/// side forms a lookahead site whose range holds the rest of the spine.
const SPINE_CONTINUES_RIGHT: f64 = 0.7;

/// Regions under the one pre-scan of [`arb_wide_case`].
///
/// Most draws place more sites under the pre-scan than one memo block holds.
const WIDE_REGIONS: Range<usize> = 40..400;

/// Regions under each pre-scan of [`arb_multi_scan_case`].
const MULTI_SCAN_REGIONS: Range<usize> = 20..160;

/// Pre-scans per walk in [`arb_multi_scan_case`].
const MULTI_SCANS: RangeInclusive<usize> = 2..=4;

/// Heights per case palette.
///
/// A palette holds at least two heights: with one, every leaf has the same
/// height, and normalization collapses the drawn version to a single leaf.
const PALETTE_LEN: RangeInclusive<usize> = 2..=5;

/// Exclusive bound on how far from the last leaf the late perturbation lands.
const LATE_LEAVES: usize = 6;

/// One co-generated tick case: an owning party, a version, and the leaf its
/// late perturbation replaces.
#[derive(Clone, Debug)]
pub(crate) struct TickCase {
    /// The ticking party, in normal form and never empty.
    pub(crate) party: tree::Party,
    /// The drawn version, in normal form.
    pub(crate) version: tree::Version,
    /// Which leaf of the fill-fixed successor the late perturbation replaces,
    /// counted back from the last leaf and taken modulo the leaf count.
    pub(crate) late_leaf: usize,
    /// The absolute height the late perturbation gives that leaf: an entry of
    /// the case's palette.
    pub(crate) late_height: BigUint,
}

impl TickCase {
    /// Returns the three versions this case ticks, in order: the drawn
    /// version, its fill-fixed successor, and a late perturbation of that
    /// successor.
    ///
    /// The fill-fixed successor is the recursive oracle's one-tick successor.
    /// Whichever branch that tick took, its result is a fixed point of
    /// simplification (simplification is idempotent, and the grow branch is
    /// absorbing), so the successor's own tick takes the grow branch over the
    /// whole co-generated structure. The late perturbation sets one of the
    /// successor's last few leaves to a palette height, so the walk over it
    /// matches its input for a long prefix and first diverges near the end.
    pub(crate) fn versions(&self) -> [tree::Version; 3] {
        let mut fill_fixed = self.version.clone();
        fill_fixed.tick(&self.party);
        let late = with_leaf_from_end(&fill_fixed, self.late_leaf, &self.late_height);
        [self.version.clone(), fill_fixed, late]
    }
}

/// A version subtree whose leaves name entries of the case's palette.
#[derive(Clone, Debug)]
enum VersionSkeleton {
    /// A leaf at the palette height this tag selects, modulo the palette's
    /// length.
    Leaf(u8),
    /// A branch; resolution lifts its children's common minimum into its base.
    Node(Box<VersionSkeleton>, Box<VersionSkeleton>),
}

/// The party and the version over one interval of a co-generated case.
#[derive(Clone, Debug)]
enum Region {
    /// The party owns the whole interval, over an arbitrary version subtree.
    Owned(VersionSkeleton),
    /// The party owns none of the interval.
    Unowned(VersionSkeleton),
    /// A party branch over one version leaf at the tagged palette height.
    OverLeaf(tree::Party, tree::Party, u8),
    /// Both trees branch here.
    Aligned(Box<Region>, Box<Region>),
}

/// Joins two regions under one branch of both trees.
fn aligned((left, right): (Region, Region)) -> Region {
    Region::Aligned(Box::new(left), Box::new(right))
}

/// Co-generated cases over a spine of up to [`SPINE_LEVELS`] levels, each
/// placing a side region beside the rest of the spine.
pub(crate) fn arb_spine_case() -> impl Strategy<Value = TickCase> {
    arb_case(arb_spine_region())
}

/// Co-generated cases whose one outermost lookahead site covers a balanced tree
/// of [`WIDE_REGIONS`] regions, most of them sites.
///
/// The tree stays shallow enough for the recursive oracle while its pre-scan
/// reserves more memo slots than one block holds.
pub(crate) fn arb_wide_case() -> impl Strategy<Value = TickCase> {
    arb_case(arb_wide_region(WIDE_REGIONS))
}

/// Co-generated cases joining [`MULTI_SCANS`] wide trees side by side, so one
/// walk runs several pre-scans and reuses the memo's blocks across them.
pub(crate) fn arb_multi_scan_case() -> impl Strategy<Value = TickCase> {
    let scans = proptest::collection::vec(arb_wide_region(MULTI_SCAN_REGIONS), MULTI_SCANS);
    arb_case(scans.prop_map(|scans| {
        scans
            .into_iter()
            .rev()
            .reduce(|later, earlier| aligned((earlier, later)))
            .expect("the scan count is at least two")
    }))
}

/// The proptest configuration for a property whose cases each cost about
/// `divisor` ordinary cases: the configured case count divided by `divisor`,
/// rounded up.
///
/// The property then finishes inside nextest's time limit in a debug build at
/// the default count, and still scales with `PROPTEST_CASES` for longer
/// investigative runs.
pub(crate) fn divided_config(divisor: u32) -> ProptestConfig {
    let configured = ProptestConfig::default();
    ProptestConfig {
        cases: configured.cases.div_ceil(divisor),
        ..configured
    }
}

/// Resolves drawn regions against a drawn palette, and draws the late
/// perturbation.
fn arb_case(regions: impl Strategy<Value = Region>) -> impl Strategy<Value = TickCase> {
    (regions, arb_palette(), 0..LATE_LEAVES, any::<u8>()).prop_map(
        |(region, palette, late_leaf, late_tag)| {
            let (party, version) = resolve(&region, &palette, 0);
            // A region tree with no owned region resolves to the empty party,
            // which cannot tick; the seed stands in for it.
            let party = if party.is_empty() {
                tree::Party::seed()
            } else {
                party
            };
            TickCase {
                party,
                version,
                late_leaf,
                late_height: height(&palette, late_tag),
            }
        },
    )
}

/// A per-case palette of absolute heights.
///
/// The arms mix small heights, which force ties and zero memo differences,
/// with word-range heights, heights at the top of `u64`, heights just past
/// `2^64` and `2^130`, which force near-ties at wide scale, and sparse wide
/// multiples of a power of two.
fn arb_palette() -> impl Strategy<Value = Vec<BigUint>> {
    let height = prop_oneof![
        4 => (0u64..6).prop_map(BigUint::from),
        2 => (0u64..1000).prop_map(BigUint::from),
        1 => any::<u64>().prop_map(BigUint::from),
        1 => (u64::MAX - 3..=u64::MAX).prop_map(BigUint::from),
        2 => (0u64..4).prop_map(|k| (BigUint::from(1u8) << 64u32) + k),
        2 => (0u64..4).prop_map(|k| (BigUint::from(1u8) << 130u32) + k),
        1 => (1u64..4, 60u32..200).prop_map(|(m, s)| BigUint::from(m) << s),
    ];
    proptest::collection::vec(height, PALETTE_LEN)
}

/// A version skeleton of at most `depth` levels.
fn arb_skeleton(depth: u32) -> impl Strategy<Value = VersionSkeleton> {
    any::<u8>()
        .prop_map(VersionSkeleton::Leaf)
        .prop_recursive(depth, 24, 2, |inner| {
            (inner.clone(), inner)
                .prop_map(|(left, right)| VersionSkeleton::Node(Box::new(left), Box::new(right)))
        })
}

/// A normal-form party of at most `depth` levels, possibly empty.
fn arb_small_party(depth: u32) -> impl Strategy<Value = tree::Party> {
    any::<bool>()
        .prop_map(tree::Party::Leaf)
        .prop_recursive(depth, 16, 2, |inner| {
            (inner.clone(), inner).prop_map(|(left, right)| tree::Party::node(left, right))
        })
}

/// A bushy region tree of at most `depth` levels; `regions` is
/// `prop_recursive`'s soft target for its total child count.
///
/// The recursive case chooses among an ordinary branch, a lookahead site (an
/// owned left child beside a recursive right child), and an owned right child
/// beside a recursive left child, whose raise reads the left child's
/// simplified minimum.
fn arb_bushy_region(depth: u32, regions: u32) -> impl Strategy<Value = Region> {
    let leaf = prop_oneof![
        3 => arb_skeleton(2).prop_map(Region::Owned),
        1 => any::<u8>().prop_map(|tag| Region::Owned(VersionSkeleton::Leaf(tag))),
        2 => arb_skeleton(2).prop_map(Region::Unowned),
        1 => (arb_small_party(2), arb_small_party(2), any::<u8>())
            .prop_map(|(left, right, tag)| Region::OverLeaf(left, right, tag)),
    ];
    leaf.prop_recursive(depth, regions, 2, |inner| {
        prop_oneof![
            6 => (inner.clone(), inner.clone()).prop_map(aligned),
            3 => (arb_skeleton(2), inner.clone())
                .prop_map(|(owned, right)| aligned((Region::Owned(owned), right))),
            2 => (inner, arb_skeleton(2))
                .prop_map(|(left, owned)| aligned((left, Region::Owned(owned)))),
        ]
    })
}

/// A spine of co-generated levels folded around a small bushy tip.
///
/// Each level continues into its right child with probability
/// [`SPINE_CONTINUES_RIGHT`] and places a side region on the other side:
/// owned, unowned, a party branch over a leaf, or a small bushy region.
/// Consecutive owned left sides nest lookahead sites along the spine, and a
/// small bushy side beside a site's range puts a second site into that range.
fn arb_spine_region() -> impl Strategy<Value = Region> {
    let side = prop_oneof![
        4 => arb_skeleton(2).prop_map(Region::Owned),
        2 => arb_skeleton(2).prop_map(Region::Unowned),
        1 => (arb_small_party(1), arb_small_party(1), any::<u8>())
            .prop_map(|(left, right, tag)| Region::OverLeaf(left, right, tag)),
        2 => arb_bushy_region(2, 4),
    ];
    let level = (proptest::bool::weighted(SPINE_CONTINUES_RIGHT), side);
    (
        proptest::collection::vec(level, 1..SPINE_LEVELS),
        arb_bushy_region(2, 4),
    )
        .prop_map(|(levels, tip)| {
            levels
                .into_iter()
                .rev()
                .fold(tip, |rest, (continues_right, side)| {
                    if continues_right {
                        aligned((side, rest))
                    } else {
                        aligned((rest, side))
                    }
                })
        })
}

/// One outermost lookahead site over a balanced tree of `regions` regions.
///
/// Each region is a minimal site (an owned leaf beside a party branch over a
/// leaf), a site whose range nests another site, an unowned or owned subtree,
/// or a small bushy region. The outer site's pre-scan reserves one memo slot
/// for itself and one for every site inside its range.
fn arb_wide_region(regions: Range<usize>) -> impl Strategy<Value = Region> {
    use tree::Party as P;
    let owned_leaf = |tag| Region::Owned(VersionSkeleton::Leaf(tag));
    let region = prop_oneof![
        4 => (any::<u8>(), any::<u8>()).prop_map(move |(owned, leaf)| aligned((
            owned_leaf(owned),
            Region::OverLeaf(P::Leaf(true), P::Leaf(false), leaf),
        ))),
        2 => (any::<u8>(), any::<u8>(), any::<u8>()).prop_map(move |(outer, inner, leaf)| {
            aligned((
                owned_leaf(outer),
                aligned((
                    owned_leaf(inner),
                    Region::OverLeaf(P::Leaf(false), P::Leaf(true), leaf),
                )),
            ))
        }),
        2 => arb_skeleton(1).prop_map(Region::Unowned),
        1 => arb_skeleton(1).prop_map(Region::Owned),
        2 => arb_bushy_region(2, 4),
    ];
    (proptest::collection::vec(region, regions), any::<u8>())
        .prop_map(move |(regions, owned)| aligned((owned_leaf(owned), balanced(regions))))
}

/// Joins `regions` into a balanced tree, in order.
///
/// Recursive over a logarithmic depth.
fn balanced(mut regions: Vec<Region>) -> Region {
    if regions.len() == 1 {
        return regions.pop().expect("one region remains");
    }
    let right = regions.split_off(regions.len() / 2);
    aligned((balanced(regions), balanced(right)))
}

/// The palette height `tag` selects.
fn height(palette: &[BigUint], tag: u8) -> BigUint {
    palette[usize::from(tag) % palette.len()].clone()
}

/// Resolves a region into a normal-form party and version.
///
/// Party branches go through [`tree::Party::node`], which collapses `(1, 1)`
/// and `(0, 0)`, and version branches through [`tree::Version::node`] with a
/// zero base, which lifts the children's common minimum and collapses equal
/// leaf pairs; leaf heights are absolute.
fn resolve(region: &Region, palette: &[BigUint], depth: usize) -> (tree::Party, tree::Version) {
    match region {
        Region::Owned(skeleton) => (tree::Party::seed(), heights(skeleton, palette, depth)),
        Region::Unowned(skeleton) => (tree::Party::Leaf(false), heights(skeleton, palette, depth)),
        Region::OverLeaf(left, right, tag) => (
            tree::Party::node(left.clone(), right.clone()),
            tree::Version::leaf(height(palette, *tag)),
        ),
        Region::Aligned(left, right) => {
            let (party_left, version_left) = descend!(depth + 1, resolve(left, palette, depth + 1));
            let (party_right, version_right) =
                descend!(depth + 1, resolve(right, palette, depth + 1));
            (
                tree::Party::node(party_left, party_right),
                tree::Version::node(0u8, version_left, version_right),
            )
        }
    }
}

/// Resolves a version skeleton's leaves to their palette heights.
fn heights(skeleton: &VersionSkeleton, palette: &[BigUint], depth: usize) -> tree::Version {
    match skeleton {
        VersionSkeleton::Leaf(tag) => tree::Version::leaf(height(palette, *tag)),
        VersionSkeleton::Node(left, right) => tree::Version::node(
            0u8,
            descend!(depth + 1, heights(left, palette, depth + 1)),
            descend!(depth + 1, heights(right, palette, depth + 1)),
        ),
    }
}

/// Rebuilds `version` with the leaf `from_end` places before its last leaf
/// (modulo the leaf count) set to the absolute height `height`.
fn with_leaf_from_end(version: &tree::Version, from_end: usize, height: &BigUint) -> tree::Version {
    let leaves = leaf_count(version);
    let target = leaves - 1 - from_end % leaves;
    let mut next = 0;
    with_leaf(version, &BigUint::ZERO, target, &mut next, height, 0)
}

/// Counts a version's leaves.
fn leaf_count(version: &tree::Version) -> usize {
    let mut leaves = 0;
    let mut stack = vec![version];
    while let Some(node) = stack.pop() {
        match node {
            tree::Version::Leaf(_) => leaves += 1,
            tree::Version::Node(_, left, right) => stack.extend([&**left, &**right]),
        }
    }
    leaves
}

/// Rebuilds the subtree under absolute height `offset`, setting the leaf whose
/// preorder index is `target` to `height`; `next` is the preorder index of the
/// subtree's first leaf, and advances past its last.
fn with_leaf(
    version: &tree::Version,
    offset: &BigUint,
    target: usize,
    next: &mut usize,
    height: &BigUint,
    depth: usize,
) -> tree::Version {
    match version {
        tree::Version::Leaf(base) => {
            let leaf = if *next == target {
                height.clone()
            } else {
                offset + base
            };
            *next += 1;
            tree::Version::leaf(leaf)
        }
        tree::Version::Node(base, left, right) => {
            let offset = offset + base;
            let left = descend!(
                depth + 1,
                with_leaf(left, &offset, target, next, height, depth + 1)
            );
            let right = descend!(
                depth + 1,
                with_leaf(right, &offset, target, next, height, depth + 1)
            );
            tree::Version::node(0u8, left, right)
        }
    }
}
