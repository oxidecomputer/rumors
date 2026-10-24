//! Liveness census for the arbitrary-input strategies.
//!
//! The exhaustive corpus has a totality pin (`corpus_counts_are_exact`); this
//! is its analog for the sampled tier. Every bound proved over generated
//! inputs is a bound over the universe the generators actually reach, so a
//! silently dead arm or depth regression shrinks that universe with no red
//! anywhere — every differential keeps passing, on less. The census samples
//! the strategies under a committed seed and holds a positive floor for each
//! named input class the suites rely on.

use proptest::strategy::{Strategy, ValueTree};
use proptest::test_runner::{Config, TestRunner};

use crate::testing::oracles::function::{ev_depth, id_depth};
use crate::testing::oracles::tree;
use crate::testing::rng::strategy_rng;
use num_bigint::BigUint;

use super::{arb_dyadic, arb_oracle_party, arb_oracle_version, arb_related_dyadic, ARB_DEPTH};

/// The census seed: names the sampled corpus (see
/// [`crate::testing::rng::strategy_rng`]).
const CENSUS_SEED: u64 = 7;

/// Sampled trees per strategy.
const CENSUS_TREES: usize = 3000;

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
    /// Some base is a small nonzero multiple of `2^64` (at least 64
    /// trailing zero bits, at most 67 bits wide): the `2^64`-aligned arm's
    /// class, whose low limb is exactly zero.
    aligned: bool,
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
        aligned: false,
        depth: ev_depth(v),
    };
    let mut consider = |b: &BigUint| {
        c.wide |= *b > two64;
        c.three_limb |= b.bits() > 128;
        c.beyond_narrow |= b.bits() > 129;
        c.aligned |= b.trailing_zeros().is_some_and(|z| z >= 64) && b.bits() <= 67;
    };
    let mut stack = vec![v];
    while let Some(node) = stack.pop() {
        match node {
            V::Leaf(n) => consider(n),
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
/// strategies under the committed seed meets a positive floor per class, so a
/// dead arm or depth regression reads red here instead of nowhere.
///
/// The sampled analog of the exhaustive corpus's totality pin
/// (`corpus_counts_are_exact`): the differential suites' guarantees are
/// denominated in the universe [`arb_oracle_version`]/[`arb_oracle_party`]
/// actually reach, and nothing else measures that universe. The classes:
/// wide (beyond-`2^64`) bases, three-limb bases, bases beyond every narrow
/// arm's ceiling (the genuinely-wide arm's mass), small nonzero multiples of
/// `2^64` (the aligned arm's zero-low-limb class), full-depth trees, and the
/// full-depth-and-wide conjunction — for parties, full-depth reach. Floors
/// are liveness floors set well below the seed's measured counts (a quarter
/// to a half): they assert each class is alive, not its distribution, so
/// re-measuring is needed only when the strategies or the proptest draw
/// pattern deliberately change.
#[test]
fn generator_classes_stay_under_mass() {
    let mut runner = TestRunner::new_with_rng(Config::default(), strategy_rng(CENSUS_SEED));

    let versions = arb_oracle_version();
    let (mut wide, mut three_limb, mut beyond_narrow, mut aligned) = (0usize, 0, 0, 0);
    let (mut deep, mut deep_and_wide) = (0usize, 0);
    for _ in 0..CENSUS_TREES {
        let v = versions.new_tree(&mut runner).expect("strategy").current();
        let c = census_of(&v);
        wide += usize::from(c.wide);
        three_limb += usize::from(c.three_limb);
        beyond_narrow += usize::from(c.beyond_narrow);
        aligned += usize::from(c.aligned);
        deep += usize::from(c.depth == ARB_DEPTH);
        deep_and_wide += usize::from(c.depth == ARB_DEPTH && c.wide);
    }

    let parties = arb_oracle_party();
    let mut party_deep = 0usize;
    for _ in 0..CENSUS_TREES {
        let p = parties.new_tree(&mut runner).expect("strategy").current();
        party_deep += usize::from(id_depth(&p) == ARB_DEPTH);
    }

    eprintln!(
        "census over {CENSUS_TREES} trees per strategy: wide {wide}, three_limb {three_limb}, \
         beyond_narrow {beyond_narrow}, aligned {aligned}, deep {deep}, deep_and_wide \
         {deep_and_wide}, party_deep {party_deep}"
    );

    let floors = [
        ("a base beyond 2^64", wide, 650),
        ("a base needing three limbs", three_limb, 300),
        ("a base beyond every narrow arm", beyond_narrow, 300),
        // Half of measured, not a quarter: the genuinely-wide arm's small
        // shift range leaks a trickle of identical values into this class,
        // and the floor must sit well above that trickle to read red when
        // the aligned arm alone dies.
        ("a small nonzero multiple of 2^64", aligned, 90),
        ("a version tree at full depth", deep, 70),
        ("a full-depth tree with a wide base", deep_and_wide, 70),
        ("a party tree at full depth", party_deep, 35),
    ];
    for (class, count, floor) in floors {
        assert!(
            count >= floor,
            "generator mass on `{class}` fell to {count} of {CENSUS_TREES} sampled trees \
             (liveness floor {floor}): an arm or the depth recursion has gone dead"
        );
    }
}

/// Dyadic rationals sampled for the census.
const CENSUS_DYADICS: usize = 1000;

/// [`arb_dyadic`] and [`arb_related_dyadic`] keep their claimed regimes
/// alive: zero ranks, nonzero integral ranks, and pairs that spell one value
/// two ways.
///
/// Each floor is a quarter of the least count the strategies' weights imply
/// over [`CENSUS_DYADICS`] draws. The zero arm has weight 1 in 13, so zero is
/// at least 1 in 13 of draws. Two arms never draw zero: a power of two or its
/// successor, at least 2 in 13, and a small value from 1 to 15, at least 15 in
/// 104; with an exponent of zero, weight 3 in 11, nonzero integral values are
/// at least 93 in 1144. The respelling mode has weight 1 in 7 and shifts by a
/// nonzero amount in 69 of 70 draws, so a second spelling of the first value
/// arrives in at least 69 in 490 pairs.
#[test]
fn dyadic_generators_reach_their_claimed_regimes() {
    let mut runner = TestRunner::new_with_rng(Config::default(), strategy_rng(CENSUS_SEED));
    let pairs = arb_dyadic().prop_flat_map(arb_related_dyadic);
    let (mut zero, mut integral, mut respelled) = (0usize, 0, 0);
    for _ in 0..CENSUS_DYADICS {
        let (first, second) = pairs.new_tree(&mut runner).expect("strategy").current();
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
    eprintln!(
        "census over {CENSUS_DYADICS} dyadic pairs: zero {zero}, nonzero integral {integral}, \
         respelled {respelled}"
    );

    let floor = |numerator: usize, denominator: usize| CENSUS_DYADICS * numerator / denominator / 4;
    let floors = [
        ("a zero value", zero, floor(1, 13)),
        ("a nonzero integral value", integral, floor(93, 1144)),
        ("one value in two spellings", respelled, floor(69, 490)),
    ];
    for (class, count, floor) in floors {
        assert!(
            count >= floor,
            "generator mass on `{class}` fell to {count} of {CENSUS_DYADICS} sampled pairs \
             (liveness floor {floor}): an arm has gone dead"
        );
    }
}
