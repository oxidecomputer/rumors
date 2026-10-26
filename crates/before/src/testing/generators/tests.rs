//! Liveness census for the arbitrary-input and deep strategies.
//!
//! The exhaustive corpus has a totality pin (`corpus_counts_are_exact`); this
//! is its analog for the sampled tier. Every bound proved over generated
//! inputs is a bound over the universe the generators actually reach, so a
//! dead arm or depth regression shrinks that universe while every
//! differential keeps passing, on less. Each census here holds a floor for
//! each named input class the suites rely on. The arbitrary-input and dyadic
//! censuses sample their strategies afresh, against floors derived from each
//! class's designed mass as [`census`](super::census) explains; the deep
//! strategies' census samples a committed corpus
//! ([`census::seeded`](super::census::seeded)). The multi-scale pairs'
//! census lives with the measure tests, beside the deferral counter it
//! reads.

use proptest::prelude::*;

use crate::testing::bridge::{from_oracle_party, from_oracle_version};
use crate::testing::oracles::flat::{self, Collapses};
use crate::testing::oracles::function::{ev_depth, id_depth};
use crate::testing::oracles::tree;
use crate::testing::shape_rows::{oracle_plateaus, version_from_rows, Leaf};
use crate::Clock;
use num_bigint::BigUint;

use super::census::{assert_floors, runner, sample, seeded, Mass, Tally};
use super::leaf_list::{
    arb_deep_or_correlated_pair, arb_deep_version, arb_deep_version_pair, arb_deep_version_triple,
    LeafList, ONE_VERSION_DIVISOR, THREE_VERSION_DIVISOR, TWO_VERSION_DIVISOR,
};
use super::regions::{arb_deep_party, arb_deep_party_family};
use super::{
    arb_clock_family, arb_dyadic, arb_oracle_party, arb_oracle_version, arb_related_dyadic,
    ARB_DEPTH,
};
use crate::testing::property::divided_proptest;

/// Trees sampled from each arbitrary strategy: enough that the thinnest
/// class, a leaf aligned at `2^64`, gets a positive floor from its designed
/// mass of `14/1000`.
const CENSUS_TREES: usize = 6000;

/// Which named base classes a version tree touches, plus its depth.
struct VersionCensus {
    /// Some base exceeds `2^64`: the class every word-scale kernel guard
    /// first activates in.
    wide: bool,
    /// Some base needs more than two 64-bit limbs (`bits > 128`).
    three_limb: bool,
    /// Some base lies beyond every narrow arm's ceiling (`bits > 129`;
    /// the narrow arms top out at `u128 + u64::MAX`): mass here is the
    /// genuinely-wide shifted-odd arm's alone.
    beyond_narrow: bool,
    /// Some leaf is a small nonzero multiple of `2^64` (at least 64
    /// trailing zero bits, at most 67 bits wide): the `2^64`-aligned arm's
    /// class, whose low limb is exactly zero.
    ///
    /// Leaves alone count, because an interior base is a sum of drawn
    /// magnitudes, and sums such as `u64::MAX + 1` land in the class
    /// whatever the aligned arm does. A leaf holds one drawn magnitude less
    /// its sibling's, so with the aligned arm dead the class nearly
    /// vanishes, far below its floor.
    aligned_leaf: bool,
    /// Structural depth (0 for a leaf).
    depth: u32,
}

/// Classify one version tree for the census.
fn census_of(v: &tree::Version) -> VersionCensus {
    use tree::Version as V;
    let two64 = BigUint::from(1u8) << 64u32;
    let mut c = VersionCensus {
        wide: false,
        three_limb: false,
        beyond_narrow: false,
        aligned_leaf: false,
        depth: ev_depth(v),
    };
    let mut consider = |b: &BigUint| {
        c.wide |= *b > two64;
        c.three_limb |= b.bits() > 128;
        c.beyond_narrow |= b.bits() > 129;
    };
    let mut stack = vec![v];
    while let Some(node) = stack.pop() {
        match node {
            V::Leaf(n) => {
                consider(n);
                c.aligned_leaf |= n.trailing_zeros().is_some_and(|z| z >= 64) && n.bits() <= 67;
            }
            V::Node(n, l, r) => {
                consider(n);
                stack.push(l);
                stack.push(r);
            }
        }
    }
    c
}

/// Every named input class stays under generator mass: sampling the arbitrary
/// strategies meets a positive floor per class, derived from its designed
/// mass, so a dead arm or depth regression fails here instead of nowhere.
///
/// The sampled analog of the exhaustive corpus's totality pin
/// (`corpus_counts_are_exact`): the differential suites' guarantees are
/// denominated in the universe [`arb_oracle_version`]/[`arb_oracle_party`]
/// actually reach, and nothing else measures that universe.
///
/// # Designed masses
///
/// A magnitude comes from [`arb_magnitude`](super::arb_magnitude)'s seven
/// arms, of weights 6, 2, 1, 1, 1, 1, 1 in 13. Both strategies recurse
/// through `prop_recursive(4, 16, 2)`, whose four layers, innermost first,
/// take their branch arm with probability `1/16`, `1/4`, and twice `q`,
/// where `q = 1932735283 / 2^31`, proptest's rounding of `0.9`. So the
/// root branches with probability `q`, and a tree reaches structural depth
/// four with probability `d₄ = q d₃ (2 − d₃) ≥ 0.0940`, where
/// `d₃ = q (1 − (993/1024)²)`. A node's base is its drawn magnitude plus its
/// children's common minimum, so the root's base is at least the root's
/// drawn magnitude; a leaf keeps the value its parent's construction gives
/// it, since every later step moves only that parent's base.
///
/// - *A base beyond `2^64`*, `23/100`: the root's magnitude exceeds `2^64`
///   on the `u128` arm whenever its draw is at least 2, on the `2^k + 1` arm
///   for 32 of 96 exponents, on the `k · 2^64` arm for 6 of 7 multipliers,
///   and on the shifted-odd arm for 1796 of 2048 draws: at least `0.2359`.
/// - *A base needing three limbs*, `57/1000`: only the shifted-odd arm
///   reaches `2^128` (the `u128` arm does so with probability `2^−64`), for
///   1541 of its 2048 draws: `1541 / 26624 ≥ 0.0578`.
/// - *A base beyond every narrow arm*, `57/1000`: the shifted-odd arm
///   reaches `2^129` for 1537 of its 2048 draws: `1537 / 26624 ≥ 0.0577`.
/// - *A leaf aligned at `2^64`*, `14/1000`: a node whose children are both
///   leaves keeps leaves `a − min` and `b − min` for drawn magnitudes `a`
///   and `b`. One is aligned when one draw is zero and the other comes from
///   the aligned arm, `2/169`, or both come from the aligned arm with
///   different multipliers, `6/1183`: `20/1183` per such node. A third-layer
///   instance branches with both children leaves with probability
///   `q (45/64)²`, and the root holds two independent instances when it
///   branches and one otherwise, so `r = q (45/64)² · 20/1183 ≥ 0.007522`
///   gives at least `r (1 + q − q r) ≥ 0.01424`.
/// - *A version tree at full depth*, `43/500`: depth four survives
///   normalization unless the two leaves under the deepest node are equal,
///   which happens with probability at most the largest single magnitude's:
///   `1/13 + 1/1248 + 1/26624` for the value 2, 3, or 5, which three arms
///   draw, plus the full-`u64` arm's `2/(13 · 2^64)`, below `0.0778`. So
///   at least `0.0940 · 0.9222 ≥ 0.0866`.
/// - *A full-depth tree with a wide base*, `1/50`: the root's magnitude is
///   drawn independently of the full-depth event: at least
///   `0.0866 · 0.2359 ≥ 0.0204`.
/// - *A party tree at full depth*, `47/1000`: the deepest node's two leaf
///   bits differ with probability one half: at least `0.0940 / 2 = 0.0470`.
#[test]
fn generator_classes_stay_under_mass() {
    let mut runner = runner();
    let versions = arb_oracle_version();
    let (mut wide, mut three_limb, mut beyond_narrow, mut aligned_leaf) = (0usize, 0, 0, 0);
    let (mut deep, mut deep_and_wide) = (0usize, 0);
    for v in sample(&versions, &mut runner, CENSUS_TREES) {
        let c = census_of(&v);
        wide += usize::from(c.wide);
        three_limb += usize::from(c.three_limb);
        beyond_narrow += usize::from(c.beyond_narrow);
        aligned_leaf += usize::from(c.aligned_leaf);
        deep += usize::from(c.depth == ARB_DEPTH);
        deep_and_wide += usize::from(c.depth == ARB_DEPTH && c.wide);
    }
    let party_deep = sample(&arb_oracle_party(), &mut runner, CENSUS_TREES)
        .filter(|p| id_depth(p) == ARB_DEPTH)
        .count();
    let tallies = [
        Tally::new("a base beyond 2^64", wide, Mass::new(23, 100)),
        Tally::new(
            "a base needing three limbs",
            three_limb,
            Mass::new(57, 1000),
        ),
        Tally::new(
            "a base beyond every narrow arm",
            beyond_narrow,
            Mass::new(57, 1000),
        ),
        Tally::new(
            "a leaf that is a small nonzero multiple of 2^64",
            aligned_leaf,
            Mass::new(14, 1000),
        ),
        Tally::new("a version tree at full depth", deep, Mass::new(43, 500)),
        Tally::new(
            "a full-depth tree with a wide base",
            deep_and_wide,
            Mass::new(1, 50),
        ),
        Tally::new(
            "a party tree at full depth",
            party_deep,
            Mass::new(47, 1000),
        ),
    ];
    assert_floors(
        &tallies,
        CENSUS_TREES,
        "an arm or the depth recursion has gone dead",
    );
}

/// Dyadic pairs sampled for the census: enough that the thinnest regime, a
/// zero value, gets a positive floor from its designed mass of `1/13`.
const CENSUS_DYADICS: usize = 1000;

/// [`arb_dyadic`] and [`arb_related_dyadic`] keep their claimed regimes
/// alive: zero ranks, nonzero integral ranks, and pairs that spell one value
/// two ways.
///
/// Each designed mass is the least the strategies' weights imply. The zero
/// arm has weight 1 in 13, so zero is at least 1 in 13 of draws. Two arms
/// never draw zero: a power of two or its successor, at least 2 in 13, and a
/// small value from 1 to 15, at least 15 in 104; with an exponent of zero,
/// weight 3 in 11, nonzero integral values are at least 93 in 1144. The
/// respelling mode has weight 1 in 7 and shifts by a nonzero amount in 69 of
/// 70 draws, so a second spelling of the first value arrives in at least 69
/// in 490 pairs.
#[test]
fn dyadic_generators_reach_their_claimed_regimes() {
    let pairs = arb_dyadic().prop_flat_map(arb_related_dyadic);
    let (mut zero, mut integral, mut respelled) = (0usize, 0, 0);
    for (first, second) in sample(&pairs, &mut runner(), CENSUS_DYADICS) {
        let is_zero = first.num == BigUint::ZERO;
        let is_integral = first
            .num
            .trailing_zeros()
            .is_some_and(|zeros| zeros >= first.exp);
        zero += usize::from(is_zero);
        integral += usize::from(!is_zero && is_integral);
        let same_value = first.cmp_value(&second).is_eq();
        respelled +=
            usize::from(same_value && (first.num != second.num || first.exp != second.exp));
    }
    let tallies = [
        Tally::new("a zero value", zero, Mass::new(1, 13)),
        Tally::new("a nonzero integral value", integral, Mass::new(93, 1144)),
        Tally::new("one value in two spellings", respelled, Mass::new(69, 490)),
    ];
    assert_floors(&tallies, CENSUS_DYADICS, "an arm has gone dead");
}

/// Clock families sampled for the census: enough that the thinner regime, a
/// rejection after a partial join, gets a positive floor from its designed
/// mass of `36/1000`.
const CENSUS_CLOCK_FAMILIES: usize = 3000;

/// [`arb_clock_family`] keeps both cases its dealt families carry under
/// mass, enough for every run of a property drawing it at the configured
/// case count, such as the clock law driver, to expect cases of each.
///
/// The census lowers each family to clocks and calls `Clock::join_all`. The
/// family is *accepted* when the call returns `Ok`. It is *rejected after a
/// partial join* when the call returns `Err` and the receiver's party has
/// grown. That happens exactly when the items fold cleanly into groups, the
/// receiver joins the first group, and a later group overlaps the receiver
/// as it then stands: the path where `join_all` returns a group and every
/// later one after joining the earlier ones, and where `sync_all` must
/// still leave every clock unchanged.
///
/// # Designed masses
///
/// A family is dealt with probability `1/2`, and a dealt family places its
/// overlap with probability `3/4`.
///
/// - *Accepted*, `12/100`: a dealt family without an overlap holds distinct
///   shares, which are pairwise disjoint: `1/2 · 1/4 = 1/8`.
/// - *Rejected after a partial join*, `36/1000`: take a dealt family with `k`
///   items whose item `j` takes another participant's party, and let `2^m`
///   be the largest power of two at most `k`. `join_all` folds the items into
///   groups of the sizes of `k`'s binary digits, largest first, so the first
///   group is the first `2^m` items. Item `j` lies outside it with
///   probability `(k − 2^m)/k`. Its party then comes from the receiver or the
///   first group with probability `(2^m + 1)/(j + 1)`, more than `1/2`
///   because `j < k < 2^(m+1)`. In that case every group folds, since only
///   item `j` repeats a party and its source lies outside its group; the
///   receiver joins the groups before item `j`'s; and item `j`'s group
///   overlaps the receiver, which now holds the source. Over the eighteen
///   equally likely arities, `Σ (k − 2^m)/k` is at least `3.551`, so the
///   mass is at least `1/2 · 3/4 · 1/18 · 1/2 · 3.551 ≥ 0.0369`.
#[test]
fn clock_family_reaches_acceptance_and_rejection_after_a_partial_join() {
    let lower = |(p, v): &(tree::Party, tree::Version)| {
        Clock::from_parts(from_oracle_party(p), from_oracle_version(v))
    };
    let (mut accepted, mut rejected_late) = (0usize, 0);
    for (r, items) in sample(&arb_clock_family(), &mut runner(), CENSUS_CLOCK_FAMILIES) {
        let original = lower(&r);
        let mut receiver = original.dangerously_alias();
        match receiver.join_all(items.iter().map(lower)) {
            Ok(_) => accepted += 1,
            Err(_) => rejected_late += usize::from(receiver.party() != original.party()),
        }
    }
    let tallies = [
        Tally::new("an accepted family", accepted, Mass::new(12, 100)).serving(1),
        Tally::new(
            "a rejection after a partial join",
            rejected_late,
            Mass::new(36, 1000),
        )
        .serving(1),
    ];
    assert_floors(
        &tallies,
        CENSUS_CLOCK_FAMILIES,
        "the dealt arm has gone dead or no longer places its overlap",
    );
}

/// The deep census seed: names the sampled corpus (see
/// [`crate::testing::rng::strategy_rng`]).
///
/// Fixed because the census measures its generators
/// deterministically: a regime it counts has no mass derived from the
/// generators' weights, so its floors hold for this corpus alone.
const DEEP_CENSUS_SEED: u64 = 13;

/// The depth past which the deep strategies claim their reach.
///
/// Past 32 levels a leaf's width, `2^-depth`, is finer than a 32-bit
/// fraction can hold.
const DEEP_DEPTH: u32 = 32;

/// Samples `strategy` [`CENSUS_CASES`](seeded::CENSUS_CASES) times under
/// the deep census seed.
///
/// Every regime over one strategy is judged on one corpus.
fn deep_corpus<T>(strategy: impl Strategy<Value = T>) -> Vec<T> {
    seeded::sample(&strategy, DEEP_CENSUS_SEED).collect()
}

/// Counts the corpus members `in_regime` accepts.
fn regime_count<T>(corpus: &[T], in_regime: impl Fn(&T) -> bool) -> usize {
    corpus.iter().filter(|member| in_regime(member)).count()
}

/// Whether a canonical tree is deeper than [`DEEP_DEPTH`].
fn is_deep(version: &tree::Version) -> bool {
    ev_depth(version) > DEEP_DEPTH
}

/// Whether a family's parties are pairwise disjoint, by the recursive
/// oracle's judgment.
fn pairwise_disjoint((receiver, items): &(tree::Party, Vec<tree::Party>)) -> bool {
    let family: Vec<&tree::Party> = core::iter::once(receiver).chain(items).collect();
    family
        .iter()
        .enumerate()
        .all(|(i, a)| family[i + 1..].iter().all(|b| a.is_disjoint(b)))
}

/// The collapses that canonicalizing a pair's join and its meet takes, in
/// the flat model: each is the pointwise extremum over the canonical
/// operands' common refinement, collapsed bottom up.
fn join_and_meet_collapses((a, b): &(LeafList, LeafList)) -> [Collapses; 2] {
    let (a, b) = (
        flat::Version::from_rows(a.leaves()),
        flat::Version::from_rows(b.leaves()),
    );
    [
        flat::Version::collapses(&a.pointwise_cells(&b, |x, y| x.max(y).clone())),
        flat::Version::collapses(&a.pointwise_cells(&b, |x, y| x.min(y).clone())),
    ]
}

/// Every deep strategy keeps enough generator mass on the regime its docs
/// claim that each default run of the properties relying on it expects
/// [`REGIME_CASES_PER_RUN`](seeded::REGIME_CASES_PER_RUN) cases there.
///
/// The regimes: canonical depth past [`DEEP_DEPTH`] for deep versions,
/// pairs, triples, and parties; equal pairs, and distinct pairs of equal
/// rank, which independent draws never produce; and, for party families,
/// both pairwise disjoint families of two or more parties, which `join_all`
/// must accept, and overlapping ones, which it must reject. The pairs the
/// flat model's differentials draw must reach joins or meets whose
/// canonicalization cascades, a collapsed pair collapsing again with its
/// sibling, and cascades whose surviving leaf's code is wider than the
/// version writer's one-word limit; the lattice differential, which claims
/// them, takes the two-version drivers' case count. Each floor
/// follows from that premise and the case count of the most demanding
/// property that draws the strategy: the clock-family driver holds deep
/// versions and deep parties beside a family, so a deep version, party, or
/// family takes the two-version drivers' case count.
#[test]
fn deep_strategies_keep_mass_on_their_regimes() {
    seeded::assert_default_case_count();
    let versions = deep_corpus(arb_deep_version().prop_map(|v| v.tree()));
    let pairs = deep_corpus(arb_deep_version_pair().prop_map(|(a, b)| (a.tree(), b.tree())));
    let triples =
        deep_corpus(arb_deep_version_triple().prop_map(|(a, b, c)| [a.tree(), b.tree(), c.tree()]));
    let parties = deep_corpus(arb_deep_party());
    let families = deep_corpus(arb_deep_party_family());
    let collapses =
        deep_corpus(arb_deep_or_correlated_pair().prop_map(|pair| join_and_meet_collapses(&pair)));
    let two_version_floor = seeded::regime_floor(seeded::CENSUS_CASES, TWO_VERSION_DIVISOR);
    let tallies = [
        seeded::Tally::new(
            "deep versions: canonical depth past 32",
            regime_count(&versions, is_deep),
            two_version_floor,
        ),
        seeded::Tally::new(
            "deep version pairs: an operand past depth 32",
            regime_count(&pairs, |(a, b)| is_deep(a) || is_deep(b)),
            two_version_floor,
        ),
        seeded::Tally::new(
            "deep version pairs: equal",
            regime_count(&pairs, |(a, b)| a == b),
            two_version_floor,
        ),
        seeded::Tally::new(
            "deep version pairs: distinct with equal ranks",
            regime_count(&pairs, |(a, b)| a != b && a.rank() == b.rank()),
            two_version_floor,
        ),
        seeded::Tally::new(
            "deep version triples: an operand past depth 32",
            regime_count(&triples, |triple| triple.iter().any(is_deep)),
            seeded::regime_floor(seeded::CENSUS_CASES, THREE_VERSION_DIVISOR),
        ),
        seeded::Tally::new(
            "deep parties: canonical depth past 32",
            regime_count(&parties, |p| id_depth(p) > DEEP_DEPTH),
            two_version_floor,
        ),
        seeded::Tally::new(
            "deep party families: pairwise disjoint, two or more parties",
            regime_count(&families, pairwise_disjoint),
            two_version_floor,
        ),
        seeded::Tally::new(
            "deep party families: overlapping",
            regime_count(&families, |family| !pairwise_disjoint(family)),
            two_version_floor,
        ),
        seeded::Tally::new(
            "deep or correlated pairs: a join or meet that cascades",
            regime_count(&collapses, |pair| pair.iter().any(|c| c.cascades > 0)),
            two_version_floor,
        ),
        seeded::Tally::new(
            "deep or correlated pairs: a cascade keeping a wide code",
            regime_count(&collapses, |pair| pair.iter().any(|c| c.wide_cascades > 0)),
            two_version_floor,
        ),
    ];
    seeded::assert_floors(
        &tallies,
        "an arm has gone dead or normalization collapses its depth",
    );
}

/// Checks that every leaf lies inside one plateau, at that plateau's height,
/// and that the plateaus cover exactly the leaves.
///
/// Both listings tile `[0, 1)` in stream order, so one pass traverses them in
/// step. Widths count cells of the finest leaf depth, as `BigUint` because
/// depths pass 128: a leaf at depth `d` spans `2^(finest − d)` cells. A leaf
/// wider than what remains of the current plateau straddles a plateau
/// boundary; a plateau deeper than the finest leaf splits a leaf.
fn assert_leaves_lie_in_plateaus(leaves: &[Leaf], plateaus: &[Leaf]) -> Result<(), TestCaseError> {
    let finest = leaves
        .iter()
        .map(|leaf| leaf.depth)
        .max()
        .expect("a leaf list holds at least the root leaf");
    let cells = |depth: u64| -> Result<BigUint, TestCaseError> {
        let below = finest.checked_sub(depth).ok_or_else(|| {
            TestCaseError::fail(format!(
                "a plateau at depth {depth} is finer than the finest leaf, at {finest}"
            ))
        })?;
        Ok(BigUint::from(1u8) << below)
    };
    let mut plateaus = plateaus.iter();
    let mut current = plateaus.next();
    let mut remaining = BigUint::ZERO;
    if let Some(plateau) = current {
        remaining = cells(plateau.depth)?;
    }
    for leaf in leaves {
        let Some(plateau) = current else {
            return Err(TestCaseError::fail("the plateaus end before the leaves"));
        };
        prop_assert_eq!(
            &leaf.height,
            &plateau.height,
            "a leaf's height differs from its plateau's"
        );
        let width = cells(leaf.depth)?;
        prop_assert!(
            width <= remaining,
            "a leaf at depth {} straddles a plateau boundary",
            leaf.depth
        );
        remaining -= width;
        if remaining == BigUint::ZERO {
            current = plateaus.next();
            if let Some(next) = current {
                remaining = cells(next.depth)?;
            }
        }
    }
    prop_assert!(current.is_none(), "the leaves end before the plateaus");
    Ok(())
}

divided_proptest! {
    #![divisor(ONE_VERSION_DIVISOR)]

    /// A deep leaf list's rows and its closed tree's plateau rows close to
    /// one normal-form tree, and normalization only merges rows: every leaf
    /// lies inside one plateau, at that plateau's height.
    ///
    /// [`version_from_rows`] and [`oracle_plateaus`] are the conversion pair
    /// between row listings and oracle trees that the leaf lists, the
    /// differential table's function-space legs, and any model over rows
    /// rely on; on normal-form trees they are inverse, at every depth the
    /// deep strategies reach.
    #[test]
    fn leaf_list_rows_and_plateau_rows_close_to_one_tree(list in arb_deep_version()) {
        let tree = version_from_rows(list.leaves());
        let plateaus = oracle_plateaus(&tree);
        assert_leaves_lie_in_plateaus(list.leaves(), &plateaus)?;
        prop_assert_eq!(version_from_rows(&plateaus), tree);
    }
}
