//! Canonical version and party trees, and the non-canonical spellings
//! planted in them.

use std::fmt::Debug;
use std::ops::Range;

use num_bigint::BigInt;
use proptest::prelude::*;
use proptest::sample::Index;

use super::Regime;
use crate::testing::oracles::codec;

/// The deepest level a bushy tree reaches.
pub(super) const BUSHY_DEPTH: u64 = 8;

/// The split decisions a bushy tree draws, which bound its leaves to one
/// more than this.
const BUSHY_SPLITS: usize = 48;

/// The level counts a spine draws, on both sides of
/// [`DEEP_LEVELS`](super::DEEP_LEVELS).
pub(crate) const SPINE_LEVELS: Range<usize> = 30..400;

/// A nonnegative height: small values below 4 that make equal siblings
/// common, any `u64`, values from `2^64 - 3` to `2^64 + 1`, and `2^k + d`
/// for `k` from 30 to 299 and `d` from -2 to 2.
fn arb_height() -> impl Strategy<Value = BigInt> {
    prop_oneof![
        10 => (0u64..4).prop_map(BigInt::from),
        2 => any::<u64>().prop_map(BigInt::from),
        1 => (0u64..=4).prop_map(|k| BigInt::from(u64::MAX - 2) + k),
        2 => (30u32..300, -2i64..=2).prop_map(|(k, d)| (BigInt::from(1u8) << k) + d),
    ]
}

/// The leaf depths of the bushy tree that `splits` describes.
///
/// A preorder traversal starts from one open slot at the root. Each slot
/// shallower than [`BUSHY_DEPTH`] takes the next decision: a split opens two
/// child slots one level deeper, and anything else, including a decision
/// past the end of `splits`, places a leaf. A slot at [`BUSHY_DEPTH`] takes
/// no decision and places a leaf.
fn bushy_depths(splits: &[bool]) -> Vec<u64> {
    let mut decisions = splits.iter().copied();
    let mut slots = vec![0u64];
    let mut depths = Vec::new();
    while let Some(depth) = slots.pop() {
        if depth < BUSHY_DEPTH && decisions.next() == Some(true) {
            slots.extend([depth + 1, depth + 1]);
        } else {
            depths.push(depth);
        }
    }
    depths
}

/// The leaves, left to right, of a spine whose level `k` holds a leaf with
/// `levels[k].1` beside the rest of the spine, on the left when
/// `levels[k].0`, and whose deepest leaf holds `bottom`.
fn spine_leaves<T: Clone>(bottom: T, levels: &[(bool, T)]) -> Vec<(u64, T)> {
    let depth = |level: usize| level as u64 + 1;
    let left = levels
        .iter()
        .enumerate()
        .filter(|(_, (on_left, _))| *on_left)
        .map(|(level, (_, value))| (depth(level), value.clone()));
    let right = levels
        .iter()
        .enumerate()
        .rev()
        .filter(|(_, (on_left, _))| !*on_left)
        .map(|(level, (_, value))| (depth(level), value.clone()));
    left.chain([(levels.len() as u64, bottom)])
        .chain(right)
        .collect()
}

/// A canonical version tree: bushy or a spine, equally often.
///
/// Spines are the only trees deeper than [`DEEP_LEVELS`](super::DEEP_LEVELS),
/// so they get half the draws.
pub(super) fn arb_version_tree() -> impl Strategy<Value = codec::Version> {
    prop_oneof![
        1 => (
            prop::collection::vec(any::<bool>(), 0..=BUSHY_SPLITS),
            prop::collection::vec(arb_height(), BUSHY_SPLITS + 1),
        )
            .prop_map(|(splits, heights)| {
                let leaves = bushy_depths(&splits).into_iter().zip(heights).collect();
                codec::Version::from_leaves(leaves).normalized()
            }),
        1 => (
            arb_height(),
            prop::collection::vec((any::<bool>(), arb_height()), SPINE_LEVELS),
        )
            .prop_map(|(bottom, levels)| {
                codec::Version::from_leaves(spine_leaves(bottom, &levels)).normalized()
            }),
    ]
}

/// A canonical, nonempty party tree: bushy or a spine whose deepest leaf is
/// owned, equally often.
///
/// A bushy tree that owns nothing becomes the seed, which owns everything.
pub(super) fn arb_party_tree() -> impl Strategy<Value = codec::Party> {
    let nonempty = |party: codec::Party| {
        if party.leaves().iter().any(|&(_, owned)| owned) {
            party
        } else {
            codec::Party::from_leaves(vec![(0, true)])
        }
    };
    prop_oneof![
        1 => (
            prop::collection::vec(any::<bool>(), 0..=BUSHY_SPLITS),
            prop::collection::vec(any::<bool>(), BUSHY_SPLITS + 1),
        )
            .prop_map(move |(splits, owned)| {
                let leaves = bushy_depths(&splits).into_iter().zip(owned).collect();
                nonempty(codec::Party::from_leaves(leaves).normalized())
            }),
        1 => prop::collection::vec((any::<bool>(), any::<bool>()), SPINE_LEVELS).prop_map(
            |levels| codec::Party::from_leaves(spine_leaves(true, &levels)).normalized()
        ),
    ]
}

/// The leaf a plant replaces.
#[derive(Clone, Debug)]
pub(super) enum Site {
    /// The leaf an index selects among the candidates.
    Any(Index),
    /// The first of the deepest candidates.
    Deepest,
}

/// A site: any candidate leaf two times in three, otherwise the deepest,
/// so plants deeper than [`DEEP_LEVELS`](super::DEEP_LEVELS) stay common in
/// spines of every length.
fn arb_site() -> impl Strategy<Value = Site> {
    prop_oneof![2 => any::<Index>().prop_map(Site::Any), 1 => Just(Site::Deepest)]
}

impl Site {
    /// The index into `leaves` of the site among the leaves `candidate`
    /// accepts, at least one of which exists.
    fn select<T>(&self, leaves: &[(u64, T)], candidate: impl Fn(&T) -> bool) -> usize {
        let candidates: Vec<usize> = (0..leaves.len())
            .filter(|&i| candidate(&leaves[i].1))
            .collect();
        match self {
            Site::Any(at) => candidates[at.index(candidates.len())],
            Site::Deepest => *candidates
                .iter()
                .max_by_key(|&&i| (leaves[i].0, std::cmp::Reverse(i)))
                .expect("a plant has at least one candidate leaf"),
        }
    }
}

/// A non-canonical spelling planted in a canonical version tree.
#[derive(Clone, Debug)]
pub(super) enum VersionPlant {
    /// The site becomes a branch over two leaves of its height.
    CollapsiblePair(Site),
    /// The site's height becomes this negative amount. The first leaf's
    /// height is spelled unsigned, so a site at the first leaf instead
    /// splits it into its own height beside the negative one.
    NegativeHeight(Site, BigInt),
}

/// Plants either spelling, with negative heights from -1 to -3 or from
/// `-2^30` to `-2^199`.
pub(super) fn arb_version_plant() -> impl Strategy<Value = VersionPlant> {
    let depth_below_zero = prop_oneof![
        (1u64..4).prop_map(BigInt::from),
        (30u32..200).prop_map(|k| BigInt::from(1u8) << k),
    ];
    prop_oneof![
        arb_site().prop_map(VersionPlant::CollapsiblePair),
        (arb_site(), depth_below_zero)
            .prop_map(|(site, below)| VersionPlant::NegativeHeight(site, -below)),
    ]
}

/// A non-canonical spelling planted in a canonical party tree: the site, an
/// owned leaf, becomes a branch over two owned leaves.
#[derive(Clone, Debug)]
pub(super) struct PartyPlant(Site);

/// Plants an owned pair at any owned leaf.
pub(super) fn arb_party_plant() -> impl Strategy<Value = PartyPlant> {
    arb_site().prop_map(PartyPlant)
}

/// A canonical tree that a plant makes non-canonical in exactly one place.
pub(super) trait Plantable: Clone {
    /// The non-canonical spellings this tree type takes.
    type Plant: Clone + Debug;

    /// The depth of the deepest leaf.
    fn depth(&self) -> u64;

    /// The stream's live bits, before sealing.
    fn live_bits(&self) -> Vec<bool>;

    /// The tree with `plant` planted in it, and the depth of the planted
    /// leaves.
    fn planted(&self, plant: &Self::Plant) -> (Self, u64);

    /// The census regime `plant` reaches.
    fn plant_regime(plant: &Self::Plant) -> Regime;
}

/// Planting into a canonical version violates exactly the planted
/// constraint: a pair of equal leaves under one branch, or one negative
/// height, which no sibling of nonnegative height can equal.
impl Plantable for codec::Version {
    type Plant = VersionPlant;

    fn depth(&self) -> u64 {
        codec::Version::depth(self)
    }

    fn live_bits(&self) -> Vec<bool> {
        codec::Version::live_bits(self)
    }

    fn planted(&self, plant: &VersionPlant) -> (Self, u64) {
        let mut leaves = self.leaves().to_vec();
        let depth = match plant {
            VersionPlant::CollapsiblePair(site) => {
                let index = site.select(&leaves, |_| true);
                let (depth, height) = leaves[index].clone();
                leaves.splice(
                    index..=index,
                    [(depth + 1, height.clone()), (depth + 1, height)],
                );
                depth + 1
            }
            VersionPlant::NegativeHeight(site, negative) => match site.select(&leaves, |_| true) {
                0 => {
                    let (depth, height) = leaves[0].clone();
                    leaves.splice(0..=0, [(depth + 1, height), (depth + 1, negative.clone())]);
                    depth + 1
                }
                index => {
                    leaves[index].1 = negative.clone();
                    leaves[index].0
                }
            },
        };
        (codec::Version::from_leaves(leaves), depth)
    }

    fn plant_regime(plant: &VersionPlant) -> Regime {
        match plant {
            VersionPlant::CollapsiblePair(_) => Regime::CollapsiblePair,
            VersionPlant::NegativeHeight(..) => Regime::NegativeHeight,
        }
    }
}

/// Planting into a canonical party violates exactly one constraint: two
/// owned leaves under one branch.
impl Plantable for codec::Party {
    type Plant = PartyPlant;

    fn depth(&self) -> u64 {
        codec::Party::depth(self)
    }

    fn live_bits(&self) -> Vec<bool> {
        codec::Party::live_bits(self)
    }

    fn planted(&self, PartyPlant(site): &PartyPlant) -> (Self, u64) {
        let mut leaves = self.leaves().to_vec();
        let index = site.select(&leaves, |&owned| owned);
        let depth = leaves[index].0 + 1;
        leaves.splice(index..=index, [(depth, true), (depth, true)]);
        (codec::Party::from_leaves(leaves), depth)
    }

    fn plant_regime(_: &PartyPlant) -> Regime {
        Regime::CollapsiblePair
    }
}
