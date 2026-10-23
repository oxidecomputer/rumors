//! Versions drawn as leaf lists with absolute heights, far deeper than the
//! arbitrary generators reach.
//!
//! [`arb_oracle_version`](super::arb_oracle_version) recurses at most four
//! levels, so a property fed only by it never meets a version deeper than
//! four, an encoding wider than a few hundred bytes, or a pair of versions
//! that are equal or tie exactly in rank. The strategies here reach all of
//! these.
//!
//! A [`LeafList`] is a version's leaves in stream order, each a
//! [`Leaf`](crate::testing::shape_rows::Leaf) row with its depth and its
//! *absolute height*: the sum of the bases on its root-to-leaf path. The
//! depths tile `[0, 1)`, and any such list with any heights is a valid
//! version once normalized, so nothing is rejected. A leaf list grows from
//! the single root leaf by repeated splits, and its heights come from a
//! separate *height arm*, so topology and heights vary independently:
//!
//! | Component | Strategy | What it reaches |
//! |---|---|---|
//! | Topology | [`DEEP_SIZES`] | depth to 300 and up to 401 leaves, as bushy trees, left and right spines, zigzags, and combs |
//! | Topology | [`arb_multiscale_topology`] | bushy regions under the mixed rule, a quarter of the time beside a same-side spine 32 to 119 levels deep, so shallow leaves sit far above the common scale |
//! | Heights | [`arb_step_heights`] | heights moved by steps at scales up to 2,200 bits, with plateaus, returns, and near-cancellations |
//! | Heights | [`arb_boundary_heights`] | a few levels whose differences straddle `2^31` and `2^32`, the limits of the writer's one-word codes, and pass `2^64` |
//! | Heights | [`arb_near_equal_wide_heights`] | values one or two apart at widths from 31 to 1,000 bits |
//! | Pairs | [`arb_relation`] | a second leaf list that is independent, perturbed, refined, resplit into the same version, edited at one leaf, mirrored, or rotated |
//!
//! The mirrored and rotated relations keep every leaf's width and height, so
//! they tie exactly in rank while usually differing as versions.
//!
//! Each height arm takes a *widest exponent* and draws its scales below it.
//! A deep leaf list's size sets that exponent, so a list of hundreds of
//! leaves draws narrower heights than one of a few; [`Size::widest_exponent`]
//! gives the budget.
//!
//! The composed strategies [`arb_deep_version`], [`arb_deep_version_pair`],
//! [`arb_deep_version_triple`], and [`arb_deep_version_family`] draw one
//! size, then leaf lists of that size with heights from any arm, for the
//! law drivers and the tree-oracle differentials.
//!
//! [`LeafList::tree`] normalizes through the recursive oracle's constructors,
//! so a leaf list's canonical depth can be less than its listed depth. The
//! census in this module's sibling tests floors the regimes these docs claim,
//! measured on canonical trees.

use core::fmt::Debug;
use core::ops::Range;

use num_bigint::BigUint;
use proptest::prelude::*;
use proptest::sample::Index;
use proptest::strategy::Union;

use crate::testing::oracles::tree;
use crate::testing::shape_rows::{version_from_rows, Leaf};

use super::family_picks;

/// The case-count divisor for a property whose inputs hold one deep version.
///
/// A deep version costs a property many arbitrary cases, up to hundreds of
/// leaves and kilobytes of encoding, and its cost dominates each case. So
/// the divisors grow with the versions a property's inputs hold, and a
/// property over deep parties alone runs the full case count.
pub(crate) const ONE_VERSION_DIVISOR: u32 = 4;

/// The case-count divisor for a property whose inputs hold two deep
/// versions, or a family drawn from a pool of up to three.
pub(crate) const TWO_VERSION_DIVISOR: u32 = 8;

/// The case-count divisor for a property whose inputs hold three deep
/// versions.
pub(crate) const THREE_VERSION_DIVISOR: u32 = 12;

// ─────────────────────────────── leaf lists ───────────────────────────────

/// A version's leaves in stream order, with absolute heights.
///
/// The leaf depths always tile `[0, 1)` exactly: every constructor grows them
/// by splits or keeps another leaf list's depths, so [`LeafList::tree`]
/// closes them into one root.
#[derive(Clone, Debug)]
pub(crate) struct LeafList {
    leaves: Vec<Leaf>,
}

impl LeafList {
    /// Pairs each depth with the height at the same position.
    fn from_parts(depths: &[u64], heights: Vec<BigUint>) -> LeafList {
        debug_assert_eq!(depths.len(), heights.len(), "one height per leaf");
        let leaves = depths
            .iter()
            .zip(heights)
            .map(|(&depth, height)| Leaf { depth, height })
            .collect();
        LeafList { leaves }
    }

    /// Returns the leaves in stream order.
    ///
    /// The rows are not normalized: two adjacent equal leaves may stand where
    /// the canonical tree holds their parent.
    /// [`oracle_plateaus`](crate::testing::shape_rows::oracle_plateaus) of
    /// [`tree`](LeafList::tree) gives the canonical rows.
    pub(crate) fn leaves(&self) -> &[Leaf] {
        &self.leaves
    }

    /// Builds the normal-form version tree with these leaves.
    ///
    /// [`version_from_rows`] closes the rows through the oracle's
    /// normalizing [`tree::Version::node`], which lifts each pair of
    /// siblings' common minimum into their parent's base and collapses equal
    /// leaves.
    pub(crate) fn tree(&self) -> tree::Version {
        version_from_rows(&self.leaves)
    }

    /// Returns the leaf list `relation` derives from this one.
    pub(crate) fn related(&self, relation: &Relation) -> LeafList {
        let mut leaves = self.leaves.clone();
        match relation {
            Relation::Independent(other) => return other.clone(),
            Relation::Perturbed(nudges) => {
                for (leaf, nudge) in leaves.iter_mut().zip(nudges.iter().cycle()) {
                    leaf.height = nudge.apply(&leaf.height, &[]);
                }
            }
            Relation::Refined { splits, nudges } => {
                for (choice, nudge) in splits.iter().zip(nudges.iter().cycle()) {
                    let at = choice.mixed_pick(leaves.len());
                    let depth = leaves[at].depth + 1;
                    let height = nudge.apply(&leaves[at].height, &[]);
                    leaves[at].depth = depth;
                    leaves.insert(at + 1, Leaf { depth, height });
                }
            }
            Relation::Resplit(splits) => {
                for choice in splits {
                    let at = choice.mixed_pick(leaves.len());
                    leaves[at].depth += 1;
                    leaves.insert(at + 1, leaves[at].clone());
                }
            }
            Relation::Edited { leaf, step } => {
                let at = leaf.index(leaves.len());
                leaves[at].height = step.apply(&leaves[at].height, &[]);
            }
            Relation::Mirrored => leaves.reverse(),
            Relation::Rotated { leaf } => {
                let depth = leaves[leaf.index(leaves.len())].depth;
                let at: Vec<usize> = (0..leaves.len())
                    .filter(|&i| leaves[i].depth == depth)
                    .collect();
                let heights: Vec<BigUint> = at.iter().map(|&i| leaves[i].height.clone()).collect();
                for (j, &i) in at.iter().enumerate() {
                    leaves[i].height = heights[(j + 1) % heights.len()].clone();
                }
            }
        }
        LeafList { leaves }
    }
}

/// Draws a leaf list: the topology's leaf depths, with heights from the arm.
pub(crate) fn arb_leaf_list(
    topology: impl Strategy<Value = Topology>,
    heights: impl Strategy<Value = Heights>,
) -> impl Strategy<Value = LeafList> {
    (topology, heights).prop_map(|(topology, heights)| {
        let depths = topology.depths();
        let heights = heights.assign(depths.len());
        LeafList::from_parts(&depths, heights)
    })
}

// ───────────────────────────────── sizes ─────────────────────────────────

/// How large a deep leaf list is drawn: its topology's budget and the widest
/// exponent of its heights.
#[derive(Clone, Copy, Debug)]
enum Size {
    /// Bounds a topology to at most `splits` splits under one of the tier
    /// rules ([`arb_split_rule`]), with no leaf deeper than `depth`.
    Tier {
        /// The most splits a topology draws.
        splits: usize,
        /// The deepest leaf a topology may hold.
        depth: u64,
    },
    /// Draws a multi-scale topology.
    Multiscale,
}

impl Size {
    /// The most leaves a leaf list of this size holds.
    fn max_leaves(self) -> usize {
        match self {
            Size::Tier { splits, .. } => splits + 1,
            // The longest spine, the most later splits, and the root leaf.
            Size::Multiscale => (MULTISCALE_SPINE.end - 1) + (MULTISCALE_SPLITS - 1) + 1,
        }
    }

    /// The widest scale exponent this size's height arms draw.
    ///
    /// Counting each leaf's height code as twice its exponent, the gamma
    /// code's length, a size of at most `n` leaves gets the exponent at which
    /// its heights spend at most [`HEIGHT_BUDGET_BITS`] bits. Without the
    /// bound, a few leaf lists of hundreds of leaves at thousands of bits each
    /// would cost a law driver more than the rest of its run.
    fn widest_exponent(self) -> u32 {
        let leaves = u64::try_from(self.max_leaves()).expect("a leaf count fits in u64");
        u32::try_from(HEIGHT_BUDGET_BITS / (2 * leaves))
            .map_or(STEP_K_MAX, |fit| fit.min(STEP_K_MAX))
    }

    /// Draws a topology of this size.
    fn arb_topology(self) -> BoxedStrategy<Topology> {
        match self {
            Size::Tier { splits, depth } => (
                arb_split_rule(),
                proptest::collection::vec(arb_split_choice(), 0..=splits),
            )
                .prop_map(move |(rule, choices)| Topology {
                    spine: None,
                    rule,
                    choices,
                    max_depth: depth,
                })
                .boxed(),
            Size::Multiscale => arb_multiscale_topology().boxed(),
        }
    }

    /// Draws a leaf list of this size, with heights from any arm.
    fn arb_version(self) -> BoxedStrategy<LeafList> {
        arb_leaf_list(self.arb_topology(), arb_any_heights(self.widest_exponent())).boxed()
    }

    /// Draws a leaf list of this size and a second one related to it.
    fn arb_pair(self) -> BoxedStrategy<(LeafList, LeafList)> {
        (
            self.arb_version(),
            arb_relation(self.arb_version(), self.widest_exponent()),
        )
            .prop_map(|(a, relation)| {
                let b = a.related(&relation);
                (a, b)
            })
            .boxed()
    }

    /// Draws a leaf list of this size and two more, each related to it
    /// independently.
    fn arb_triple(self) -> BoxedStrategy<(LeafList, LeafList, LeafList)> {
        let relation = || arb_relation(self.arb_version(), self.widest_exponent());
        (self.arb_version(), relation(), relation())
            .prop_map(|(a, to_b, to_c)| {
                let (b, c) = (a.related(&to_b), a.related(&to_c));
                (a, b, c)
            })
            .boxed()
    }
}

/// The height bits a deep leaf list may spend, from which
/// [`Size::widest_exponent`] derives each size's widest exponent.
const HEIGHT_BUDGET_BITS: u64 = 100_000;

/// The sizes the deep strategies draw from, with their weights.
///
/// The tiers that reach past depth 12 take three shares each, and the
/// shallower tiers and the multi-scale arm one share each. The deep tiers'
/// weight keeps enough drawn triples past depth 32 for the least-sampled law
/// driver, which the census in this module's sibling tests floors; the
/// shallow tiers keep small structures, and small shrink targets, in every
/// run. Within a tier, the spine and zigzag rules reach the depth limit
/// whenever the drawn split count allows, the uniform rule spends the splits
/// on breadth, and the combs fall between.
const DEEP_SIZES: [(u32, Size); 6] = [
    (
        1,
        Size::Tier {
            splits: 6,
            depth: 4,
        },
    ),
    (
        1,
        Size::Tier {
            splits: 40,
            depth: 12,
        },
    ),
    (
        3,
        Size::Tier {
            splits: 120,
            depth: 140,
        },
    ),
    (
        3,
        Size::Tier {
            splits: 300,
            depth: 70,
        },
    ),
    (
        3,
        Size::Tier {
            splits: 400,
            depth: 300,
        },
    ),
    (1, Size::Multiscale),
];

/// Draws one size from [`DEEP_SIZES`] by weight, then what `draw` draws at
/// that size.
fn arb_sized<T: Debug>(draw: fn(Size) -> BoxedStrategy<T>) -> impl Strategy<Value = T> {
    Union::new_weighted(
        DEEP_SIZES
            .iter()
            .map(|&(weight, size)| (weight, draw(size)))
            .collect(),
    )
}

/// Draws a deep version as a leaf list, with heights from any arm.
pub(crate) fn arb_deep_version() -> impl Strategy<Value = LeafList> {
    arb_sized(Size::arb_version)
}

/// Draws a deep version and a second one of the same size related to it.
pub(crate) fn arb_deep_version_pair() -> impl Strategy<Value = (LeafList, LeafList)> {
    arb_sized(Size::arb_pair)
}

/// Draws a deep version and two more of the same size, each related to it
/// independently.
pub(crate) fn arb_deep_version_triple() -> impl Strategy<Value = (LeafList, LeafList, LeafList)> {
    arb_sized(Size::arb_triple)
}

/// Draws a receiver and items for the variadic version laws, as oracle trees,
/// from a pool of a deep triple and the empty version.
///
/// Drawing from a small pool makes repeated values and receiver aliases
/// common at the arities [`arb_fold_arity`](super::arb_fold_arity) sweeps,
/// as [`arb_version_family`](super::arb_version_family) does for shallow
/// versions.
pub(crate) fn arb_deep_version_family() -> impl Strategy<Value = (tree::Version, Vec<tree::Version>)>
{
    arb_deep_version_triple().prop_flat_map(|(a, b, c)| {
        family_picks(vec![a.tree(), b.tree(), c.tree(), tree::Version::new()])
    })
}

// ──────────────────────────────── topology ────────────────────────────────

/// How a topology chooses the leaf each split divides.
#[derive(Clone, Copy, Debug)]
pub(crate) enum SplitRule {
    /// Divides a uniformly chosen leaf, growing bushy trees.
    Uniform,
    /// Divides the first leaf, growing a left spine.
    First,
    /// Divides the last leaf, growing a right spine.
    Last,
    /// Divides a focus leaf, which moves right by zero or one leaf after each
    /// split, growing a spine that zigzags.
    Wander,
    /// Divides the focus leaf with probability `wander_per_256 / 256`, and
    /// otherwise a uniformly chosen leaf, growing combs of spines.
    Comb {
        /// How often, out of 256, a split follows the focus.
        wander_per_256: u8,
    },
    /// Divides a uniformly chosen leaf half the time, and otherwise the
    /// first, the last, or the second-to-last leaf, growing bushy regions
    /// with spine and comb fragments.
    Mixed,
}

/// The random draws one split reads.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SplitChoice {
    /// The leaf a uniform choice divides.
    leaf: Index,
    /// A byte the rule reads as a coin: the focus's step, the comb's
    /// choice, or the mixed rule's choice.
    coin: u8,
}

impl SplitChoice {
    /// The leaf [`SplitRule::Mixed`] divides among `len` leaves: uniform half
    /// the time, and otherwise the last, the first, or the second-to-last in
    /// equal shares.
    fn mixed_pick(self, len: usize) -> usize {
        match self.coin {
            0..=127 => self.leaf.index(len),
            128..=170 => len - 1,
            171..=212 => 0,
            _ => len.saturating_sub(2),
        }
    }
}

/// A same-side spine split before a topology's other splits.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Spine {
    /// Whether the spine divides the last leaf rather than the first.
    at_last: bool,
    /// How many splits the spine makes.
    splits: usize,
}

/// The splits that grow a leaf list's depths from the root leaf.
#[derive(Clone, Debug)]
pub(crate) struct Topology {
    /// The spine split first, if any.
    spine: Option<Spine>,
    /// The rule choosing the leaf each later split divides.
    rule: SplitRule,
    /// One draw per later split.
    choices: Vec<SplitChoice>,
    /// No split puts a leaf deeper than this; a split that would is skipped.
    max_depth: u64,
}

impl Topology {
    /// Returns the leaf depths, in stream order.
    pub(crate) fn depths(&self) -> Vec<u64> {
        let mut depths = vec![0u64];
        let divide = |depths: &mut Vec<u64>, at: usize| {
            let depth = depths[at] + 1;
            depths[at] = depth;
            depths.insert(at + 1, depth);
        };
        if let Some(spine) = self.spine {
            for _ in 0..spine.splits {
                let at = if spine.at_last { depths.len() - 1 } else { 0 };
                if depths[at] < self.max_depth {
                    divide(&mut depths, at);
                }
            }
        }
        let mut focus = 0usize;
        for choice in &self.choices {
            let len = depths.len();
            let at = match self.rule {
                SplitRule::Uniform => choice.leaf.index(len),
                SplitRule::First => 0,
                SplitRule::Last => len - 1,
                SplitRule::Wander => focus.min(len - 1),
                SplitRule::Comb { wander_per_256 } => {
                    if choice.coin < wander_per_256 {
                        focus.min(len - 1)
                    } else {
                        choice.leaf.index(len)
                    }
                }
                SplitRule::Mixed => choice.mixed_pick(len),
            };
            if depths[at] >= self.max_depth {
                // The chosen leaf is at the depth limit, so the focus jumps to
                // a uniformly chosen leaf rather than stalling there.
                focus = choice.leaf.index(len);
                continue;
            }
            divide(&mut depths, at);
            focus = at + usize::from(choice.coin & 1);
        }
        depths
    }
}

/// The exclusive bound on the splits after the spine in
/// [`arb_multiscale_topology`].
const MULTISCALE_SPLITS: usize = 64;

/// The spine lengths [`arb_multiscale_topology`] draws.
///
/// A spine of `s` splits leaves one leaf at depth `s` beside siblings at
/// every depth from 1 to `s`. With `s` at least 32, the shallowest region is
/// wider than the deepest leaf by a factor of at least `2^31`, so the rank
/// integral weighs regions whose scales differ by nearly a 32-bit digit.
const MULTISCALE_SPINE: core::ops::Range<usize> = 32..120;

/// Draws one split's draws.
fn arb_split_choice() -> impl Strategy<Value = SplitChoice> {
    (any::<Index>(), any::<u8>()).prop_map(|(leaf, coin)| SplitChoice { leaf, coin })
}

/// Draws a [`Size::Tier`]'s split rule, in equal shares: uniform, first,
/// last, wander, and combs that follow the focus half the time and 220 times
/// in 256.
fn arb_split_rule() -> impl Strategy<Value = SplitRule> {
    prop_oneof![
        Just(SplitRule::Uniform),
        Just(SplitRule::First),
        Just(SplitRule::Last),
        Just(SplitRule::Wander),
        Just(SplitRule::Comb {
            wander_per_256: 128
        }),
        Just(SplitRule::Comb {
            wander_per_256: 220
        }),
    ]
}

/// Draws a multi-scale topology: a spine of [`MULTISCALE_SPINE`] levels a
/// quarter of the time, then up to [`MULTISCALE_SPLITS`] splits under the
/// mixed rule.
pub(crate) fn arb_multiscale_topology() -> impl Strategy<Value = Topology> {
    (
        prop_oneof![
            3 => Just(None),
            1 => (any::<bool>(), MULTISCALE_SPINE)
                .prop_map(|(at_last, splits)| Some(Spine { at_last, splits })),
        ],
        proptest::collection::vec(arb_split_choice(), 0..MULTISCALE_SPLITS),
    )
        .prop_map(|(spine, choices)| Topology {
            spine,
            rule: SplitRule::Mixed,
            choices,
            max_depth: u64::MAX,
        })
}

// ───────────────────────────────── heights ─────────────────────────────────

/// How a leaf list's leaves receive their absolute heights.
#[derive(Clone, Debug)]
pub(crate) enum Heights {
    /// Sets the first leaf's height to `first` applied to zero, and each
    /// later leaf's to the next step, cycling through `steps`, applied to the
    /// previous leaf's height.
    Steps {
        /// The step that sets the first leaf's height.
        first: Step,
        /// The steps between consecutive leaves, cycled.
        steps: Vec<Step>,
    },
    /// Gives each leaf the palette value its pick selects, cycling through
    /// `picks`.
    Palette {
        /// The values leaves draw from.
        values: Vec<BigUint>,
        /// One pick per leaf, cycled.
        picks: Vec<Index>,
    },
}

impl Heights {
    /// Returns heights for `count` leaves.
    pub(crate) fn assign(&self, count: usize) -> Vec<BigUint> {
        match self {
            Heights::Steps { first, steps } => {
                let mut heights: Vec<BigUint> = Vec::with_capacity(count);
                let mut height = BigUint::ZERO;
                for i in 0..count {
                    let step = match i {
                        0 => first,
                        _ => steps.get((i - 1) % steps.len().max(1)).unwrap_or(first),
                    };
                    height = step.apply(&height, &heights);
                    heights.push(height.clone());
                }
                heights
            }
            Heights::Palette { values, picks } => picks
                .iter()
                .cycle()
                .take(count)
                .map(|pick| values[pick.index(values.len())].clone())
                .collect(),
        }
    }
}

/// What a [`Step`] does to the running height `h`.
#[derive(Clone, Copy, Debug)]
pub(crate) enum StepKind {
    /// Moves to `h + small`.
    Up,
    /// Moves to `h − small`, or to `h + small` when that would go negative.
    Down,
    /// Moves to `h + 2^k`.
    UpPower,
    /// Moves to `h − 2^k`, or to `h + 2^k` when that would go negative.
    DownPower,
    /// Moves to `2^k − 1 + small`, all ones below bit `k`.
    AllOnes,
    /// Moves to `(2^32 − 1) · 2^(32·⌊k/32⌋)`, one full 32-bit digit at a
    /// digit boundary.
    Digit,
    /// Stays at `h`, extending a plateau.
    Plateau,
    /// Returns to an earlier leaf's height, chosen by `k`, or stays at `h`
    /// on the first leaf.
    Return,
    /// Drops to zero.
    Zero,
    /// Flips bit `k` of `h`.
    FlipBit,
    /// Moves by `2^k − 2^⌊k/2⌋`: up when `small` is even or the drop would
    /// go negative, and down otherwise.
    Gap,
    /// Returns to an earlier leaf's height, chosen by `k`, plus `small`,
    /// nearly cancelling the drift since then.
    NearReturn,
}

/// One height move: a kind, a scale exponent `k`, and a small amount.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Step {
    /// What the step does.
    kind: StepKind,
    /// The step's scale exponent.
    k: u32,
    /// The step's small amount, below 8.
    small: u8,
}

impl Step {
    /// Applies the step to `height`, given the heights of the earlier leaves.
    fn apply(&self, height: &BigUint, earlier: &[BigUint]) -> BigUint {
        let small = BigUint::from(self.small);
        let power = BigUint::from(1u8) << self.k;
        let earlier_pick = || {
            let k = usize::try_from(self.k).expect("a step's scale exponent fits in usize");
            earlier.get(k % earlier.len().max(1))
        };
        match self.kind {
            StepKind::Up => height + small,
            StepKind::Down if *height >= small => height - small,
            StepKind::Down => height + small,
            StepKind::UpPower => height + power,
            StepKind::DownPower if *height >= power => height - power,
            StepKind::DownPower => height + power,
            StepKind::AllOnes => power - 1u8 + small,
            StepKind::Digit => BigUint::from(u32::MAX) << (32 * (self.k / 32)),
            StepKind::Plateau => height.clone(),
            StepKind::Return => earlier_pick().unwrap_or(height).clone(),
            StepKind::Zero => BigUint::ZERO,
            StepKind::FlipBit => height ^ power,
            StepKind::Gap => {
                let gap = power - (BigUint::from(1u8) << (self.k / 2));
                if self.small.is_multiple_of(2) || *height < gap {
                    height + gap
                } else {
                    height - gap
                }
            }
            StepKind::NearReturn => earlier_pick().unwrap_or(height) + small,
        }
    }
}

/// Steps per [`Heights::Steps`] script after the first.
const STEP_SCRIPT: core::ops::Range<usize> = 0..40;

/// Picks per [`Heights::Palette`].
const PALETTE_PICKS: core::ops::RangeInclusive<usize> = 1..=64;

/// The widest scale exponent [`arb_step`] draws.
const STEP_K_MAX: u32 = 2_200;

/// The bands [`arb_step_scale`] draws a step's scale exponent from, with
/// their weights: up to 69 bits, about 300, about 600, about 1,000, and
/// 2,000 to [`STEP_K_MAX`].
///
/// The bands put drifts below, near, and far above the integral's 256-bit
/// freeze allowance.
const STEP_SCALE_BANDS: [(u32, Range<u32>); 5] = [
    (4, 0..70),
    (2, 250..330),
    (2, 500..700),
    (1, 900..1200),
    (1, 2000..STEP_K_MAX),
];

/// Draws a step's scale exponent from [`STEP_SCALE_BANDS`], each band cut
/// off at `widest`.
///
/// The first band lies below every size's widest exponent, so some band is
/// always left.
fn arb_step_scale(widest: u32) -> impl Strategy<Value = u32> {
    Union::new_weighted(
        STEP_SCALE_BANDS
            .iter()
            .filter_map(|(weight, band)| {
                let band = band.start..band.end.min(widest + 1);
                (!band.is_empty()).then(|| (*weight, band.boxed()))
            })
            .collect(),
    )
}

/// Draws any step at scales up to `widest`.
fn arb_step(widest: u32) -> impl Strategy<Value = Step> {
    let kind = prop::sample::select(vec![
        StepKind::Up,
        StepKind::Down,
        StepKind::UpPower,
        StepKind::DownPower,
        StepKind::AllOnes,
        StepKind::Digit,
        StepKind::Plateau,
        StepKind::Return,
        StepKind::Zero,
        StepKind::FlipBit,
        StepKind::Gap,
        StepKind::NearReturn,
    ]);
    (kind, arb_step_scale(widest), 0u8..8).prop_map(|(kind, k, small)| Step { kind, k, small })
}

/// Draws a narrow step: up or down by less than 8.
fn arb_narrow_step() -> impl Strategy<Value = Step> {
    (
        prop_oneof![Just(StepKind::Up), Just(StepKind::Down)],
        0u8..8,
    )
        .prop_map(|(kind, small)| Step { kind, k: 0, small })
}

/// Draws a step script at scales up to `widest`.
pub(crate) fn arb_step_heights(widest: u32) -> impl Strategy<Value = Heights> {
    (
        arb_step(widest),
        proptest::collection::vec(arb_step(widest), STEP_SCRIPT),
    )
        .prop_map(|(first, steps)| Heights::Steps { first, steps })
}

/// Draws one level of [`arb_boundary_heights`]: a base plus an offset.
///
/// Half the bases are zero; the rest sit at `2^31`, `2^32 − 2`, `2^64 − 1`,
/// `2^128` when `widest` exceeds 128, or a power of two below
/// `2^min(700, widest)`. The offsets sit at 0 to 3, on either side of
/// `2^31`, `2^32`, and `2^64`, at `2^63`, or at a power of two below
/// `2^min(300, widest)`. The writer moves a gamma code through one 64-bit
/// word only when it is at most 63 bits: an absolute height through
/// `2^32 − 2` or a signed difference within `±(2^31 − 1)`. Differences
/// between two levels land on both sides of that limit.
fn arb_boundary_level(widest: u32) -> impl Strategy<Value = BigUint> {
    let pow2 = |k: u32| BigUint::from(1u8) << k;
    let fixed = |weight: u32, value: BigUint| (weight, Just(value).boxed());
    let mut bases = vec![
        fixed(5, BigUint::ZERO),
        fixed(1, pow2(31)),
        fixed(1, pow2(32) - 2u8),
        fixed(1, pow2(64) - 1u8),
        (1, (0u32..widest.min(700)).prop_map(pow2).boxed()),
    ];
    if widest > 128 {
        bases.push(fixed(1, pow2(128)));
    }
    let offsets = vec![
        fixed(3, BigUint::ZERO),
        fixed(1, BigUint::from(1u8)),
        fixed(1, BigUint::from(2u8)),
        fixed(1, BigUint::from(3u8)),
        fixed(1, pow2(31) - 1u8),
        fixed(1, pow2(31)),
        fixed(1, pow2(31) + 1u8),
        fixed(1, pow2(32) - 2u8),
        fixed(1, pow2(32) - 1u8),
        fixed(1, pow2(32)),
        fixed(1, pow2(63)),
        fixed(1, pow2(64)),
        fixed(1, pow2(64) + 1u8),
        (1, (0u32..widest.min(300)).prop_map(pow2).boxed()),
    ];
    (Union::new_weighted(bases), Union::new_weighted(offsets))
        .prop_map(|(base, offset)| base + offset)
}

/// Draws a palette of one to four boundary levels, at scales up to
/// `widest`, and zero.
///
/// A small palette shared by every leaf makes equal heights, and so
/// collapses, common.
pub(crate) fn arb_boundary_heights(widest: u32) -> impl Strategy<Value = Heights> {
    (
        proptest::collection::vec(arb_boundary_level(widest), 1..=4),
        proptest::collection::vec(any::<Index>(), PALETTE_PICKS),
    )
        .prop_map(|(mut values, picks)| {
            values.push(BigUint::ZERO);
            Heights::Palette { values, picks }
        })
}

/// The exponents of the wide value `W = 2^e` in
/// [`arb_near_equal_wide_heights`]: on both sides of the 32- and 64-bit
/// words, past two words, and far past them.
const WIDE_EXPONENTS: [u32; 9] = [31, 32, 63, 64, 65, 127, 128, 200, 1000];

/// One of [`arb_near_equal_wide_heights`]' palettes over `W`.
#[derive(Clone, Copy, Debug)]
enum WideAlphabet {
    /// `{W, W + 1, W + 2}`.
    Consecutive,
    /// `{0, 1, W, W + 1}`.
    PairedSteps,
    /// `{0, W, 2W}`.
    Multiples,
    /// `{W − 1, W, W + 1}`.
    Straddle,
    /// `{0, 1, W − 1, W²}`.
    Square,
}

impl WideAlphabet {
    /// The palette over `W = 2^exponent`.
    fn values(self, exponent: u32) -> Vec<BigUint> {
        let w = BigUint::from(1u8) << exponent;
        let one = BigUint::from(1u8);
        match self {
            WideAlphabet::Consecutive => vec![w.clone(), &w + 1u8, &w + 2u8],
            WideAlphabet::PairedSteps => vec![BigUint::ZERO, one, w.clone(), &w + 1u8],
            WideAlphabet::Multiples => vec![BigUint::ZERO, w.clone(), &w * 2u8],
            WideAlphabet::Straddle => vec![&w - 1u8, w.clone(), &w + 1u8],
            WideAlphabet::Square => vec![BigUint::ZERO, one, &w - 1u8, &w * &w],
        }
    }

    /// The width in bits of the palette's widest value, in units of the
    /// exponent: `W²` is twice as wide as `W`.
    fn width_factor(self) -> u32 {
        match self {
            WideAlphabet::Square => 2,
            _ => 1,
        }
    }
}

/// Draws a palette of near-equal wide values no wider than about
/// `2^widest`.
///
/// With `W = 2^e` for `e` in [`WIDE_EXPONENTS`], the palette is a
/// [`WideAlphabet`]: `{W, W + 1, W + 2}`, `{0, 1, W, W + 1}`, `{0, W, 2W}`,
/// or `{W − 1, W, W + 1}` with `e` at most `widest`, or `{0, 1, W − 1, W²}`
/// with `2e` at most `widest`. The consecutive and straddling palettes hold
/// values one or two apart at a width past the word, and the paired palette
/// holds one such pair at 0 and another at `W`. The multiples palette steps
/// by exactly `W`, and the square palette sets `W − 1` far below `W²`. Each
/// leaf takes a palette value, so a walk that compares neighboring heights
/// builds and discards differences of one or two at that width, or of whole
/// multiples of `W`.
///
/// # Panics
///
/// Panics if `widest` is below the smallest of [`WIDE_EXPONENTS`], which
/// leaves no palette to draw. Every [`Size`] gives a widest exponent above
/// it.
pub(crate) fn arb_near_equal_wide_heights(widest: u32) -> impl Strategy<Value = Heights> {
    let alphabets = [
        WideAlphabet::Consecutive,
        WideAlphabet::PairedSteps,
        WideAlphabet::Multiples,
        WideAlphabet::Straddle,
        WideAlphabet::Square,
    ];
    let choices: Vec<(WideAlphabet, u32)> = alphabets
        .into_iter()
        .flat_map(|alphabet| {
            WIDE_EXPONENTS
                .into_iter()
                .filter(move |&exponent| alphabet.width_factor() * exponent <= widest)
                .map(move |exponent| (alphabet, exponent))
        })
        .collect();
    assert!(
        !choices.is_empty(),
        "a widest exponent of {widest} admits no near-equal wide palette"
    );
    (
        prop::sample::select(choices),
        proptest::collection::vec(any::<Index>(), PALETTE_PICKS),
    )
        .prop_map(|((alphabet, exponent), picks)| Heights::Palette {
            values: alphabet.values(exponent),
            picks,
        })
}

/// Draws heights from any arm at scales up to `widest`, in equal shares.
pub(crate) fn arb_any_heights(widest: u32) -> impl Strategy<Value = Heights> {
    prop_oneof![
        arb_step_heights(widest),
        arb_boundary_heights(widest),
        arb_near_equal_wide_heights(widest),
    ]
}

// ───────────────────────────── related versions ─────────────────────────────

/// How a second leaf list derives from a first.
#[derive(Clone, Debug)]
pub(crate) enum Relation {
    /// Replaces the leaf list with an independently drawn one.
    Independent(LeafList),
    /// Moves each leaf's height by the next nudge, cycling.
    Perturbed(Vec<Step>),
    /// Splits leaves further: each split divides the leaf the mixed rule
    /// picks, and the new right child takes its parent's height moved by the
    /// next narrow nudge, cycling.
    Refined {
        /// One draw per split.
        splits: Vec<SplitChoice>,
        /// The narrow nudges for the new children, cycled.
        nudges: Vec<Step>,
    },
    /// Splits leaves further, each new child at its parent's height, giving
    /// a different leaf list for the same version: normalization collapses
    /// two equal siblings back into their parent.
    Resplit(Vec<SplitChoice>),
    /// Moves one leaf's height by one step.
    Edited {
        /// The leaf moved.
        leaf: Index,
        /// The step it moves by.
        step: Step,
    },
    /// Reverses the leaves, giving the mirror image, of equal rank.
    Mirrored,
    /// Rotates the heights of every leaf at one leaf's depth by one place
    /// among those leaves, keeping the rank.
    Rotated {
        /// The leaf whose depth is rotated.
        leaf: Index,
    },
}

/// Draws a relation, with steps at scales up to `widest`.
///
/// Mirroring and resplitting take three shares each, and every other kind
/// one. Mirrored and rotated pairs are the distinct pairs of equal rank, and
/// resplit pairs are equal; the census in this module's sibling tests floors
/// both regimes, and the weights keep each well above its floor on any
/// sample. A
/// perturbation's nudges are narrow half the time and any step otherwise,
/// so the pair's difference is sometimes narrow over wide common heights and
/// sometimes wide.
pub(crate) fn arb_relation(
    independent: impl Strategy<Value = LeafList>,
    widest: u32,
) -> impl Strategy<Value = Relation> {
    let nudge = prop_oneof![arb_narrow_step(), arb_step(widest)];
    prop_oneof![
        1 => independent.prop_map(Relation::Independent),
        1 => proptest::collection::vec(nudge, 1..12).prop_map(Relation::Perturbed),
        1 => (
            proptest::collection::vec(arb_split_choice(), 0..24),
            proptest::collection::vec(arb_narrow_step(), 1..12),
        )
            .prop_map(|(splits, nudges)| Relation::Refined { splits, nudges }),
        3 => proptest::collection::vec(arb_split_choice(), 1..24).prop_map(Relation::Resplit),
        1 => (any::<Index>(), arb_step(widest))
            .prop_map(|(leaf, step)| Relation::Edited { leaf, step }),
        3 => Just(Relation::Mirrored),
        1 => any::<Index>().prop_map(|leaf| Relation::Rotated { leaf }),
    ]
}
