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

use crate::codec::Base;
use crate::oracle;
use crate::testing::rng::strategy_rng;
use crate::testing::semantic_oracle::{ev_depth, id_depth};

use super::{arb_oracle_party, arb_oracle_version, ARB_DEPTH};

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
fn census_of(v: &oracle::Version) -> VersionCensus {
    use oracle::Version as V;
    let two64 = Base::from(1u8) << 64u32;
    let mut c = VersionCensus {
        wide: false,
        three_limb: false,
        beyond_narrow: false,
        aligned: false,
        depth: ev_depth(v),
    };
    let mut consider = |b: &Base| {
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
