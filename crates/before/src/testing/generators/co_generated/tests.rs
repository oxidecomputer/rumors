//! Liveness census for the co-generated tick strategies.
//!
//! Each strategy exists to reach one pre-scan regime that independently drawn
//! operands do not. A strategy that stops reaching its regime (a dead arm, a
//! normalization that collapses the sites it builds) still passes every
//! differential, on easier inputs; the census reads that as a failure
//! instead. It samples each strategy under a committed seed and classifies
//! every case by the lookahead sites the tick walk meets in it.

use proptest::strategy::{Strategy, ValueTree};
use proptest::test_runner::{Config, TestRunner};

use crate::testing::oracles::tree;
use crate::testing::rng::strategy_rng;

use crate::version::tick::BLOCK_SLOTS;

use super::super::{regime_floor, DEFAULT_CASES};
use super::{
    arb_deep_spine_case, arb_multi_scan_case, arb_spine_case, arb_wide_case, TickCase,
    DEEP_SPINE_CASE_DIVISOR, MULTI_SCAN_CASE_DIVISOR, WIDE_CASE_DIVISOR,
};

/// The census seed: names the sampled corpus (see
/// [`crate::testing::rng::strategy_rng`]).
const CENSUS_SEED: u64 = 11;

/// Sampled cases per strategy.
const CENSUS_CASES: usize = 400;

/// The lookahead sites the tick walk meets in one case.
#[derive(Default)]
struct Sites {
    /// The deepest chain of sites, each inside the previous one's range.
    max_nesting: usize,
    /// Whether some site's range holds two or more sites that are not inside
    /// one another.
    shared_range: bool,
    /// The memo slots each pre-scan reserves: one per outermost site, counting
    /// that site and every site inside its range.
    scan_slots: Vec<usize>,
}

impl Sites {
    /// Classify the sites of one case's party over its version.
    fn of(case: &TickCase) -> Sites {
        let mut sites = Sites::default();
        sites.visit(&case.party, &case.version, 0);
        sites
    }

    /// Visit the region where `party` meets `version`, `nesting` sites deep,
    /// and return its site count at this nesting level and at every level.
    ///
    /// A site is a party branch with an owned left child and a nonempty right
    /// child over a version branch, which is where the tick walk and its
    /// pre-scan reserve a memo slot. The walk reaches no site under an owned
    /// or unowned party leaf, nor under a version leaf.
    ///
    /// Recurses once per level of the case, whose depth its strategy bounds
    /// by construction (see [`resolve`](super::resolve)).
    fn visit(
        &mut self,
        party: &tree::Party,
        version: &tree::Version,
        nesting: usize,
    ) -> (usize, usize) {
        let (tree::Party::Node(party_left, party_right), tree::Version::Node(_, left, right)) =
            (party, version)
        else {
            return (0, 0);
        };
        if matches!(**party_left, tree::Party::Leaf(true)) && !party_right.is_empty() {
            self.max_nesting = self.max_nesting.max(nesting + 1);
            let (next_level, inside) = self.visit(party_right, right, nesting + 1);
            self.shared_range |= next_level >= 2;
            if nesting == 0 {
                self.scan_slots.push(1 + inside);
            }
            return (1, 1 + inside);
        }
        let (left_level, left_all) = self.visit(party_left, left, nesting);
        let (right_level, right_all) = self.visit(party_right, right, nesting);
        (left_level + right_level, left_all + right_all)
    }

    /// The number of pre-scans that reserve more slots than one memo block
    /// holds.
    fn scans_past_one_block(&self) -> usize {
        self.scan_slots
            .iter()
            .filter(|&&slots| slots > BLOCK_SLOTS)
            .count()
    }
}

/// Sample `strategy` [`CENSUS_CASES`] times under the census seed, and count
/// the cases `in_regime` accepts.
///
/// Each count draws from a fresh runner, so one strategy's draws never shift
/// another's.
fn census(strategy: impl Strategy<Value = TickCase>, in_regime: impl Fn(&Sites) -> bool) -> usize {
    let mut runner = TestRunner::new_with_rng(Config::default(), strategy_rng(CENSUS_SEED));
    (0..CENSUS_CASES)
        .filter(|_| {
            let case = strategy.new_tree(&mut runner).expect("strategy").current();
            in_regime(&Sites::of(&case))
        })
        .count()
}

/// Every co-generated strategy keeps enough generator mass on its pre-scan
/// regime that each default run of every property over it expects
/// [`REGIME_CASES_PER_RUN`](super::super::REGIME_CASES_PER_RUN) cases there.
///
/// The regimes: sites nested three deep and a range holding two sites side by
/// side (spine), sites nested more deeply than one memo block holds slots
/// (deep spine), a pre-scan reserving more slots than one memo block holds
/// (wide), and two such pre-scans in one walk (multi-scan). Every property
/// over a strategy runs the default case count divided by that strategy's
/// divisor, and each floor follows from that premise and that case count, so
/// a strategy whose regime mass falls below what its own runs need reads as a
/// failure here, whatever its count was when last measured.
#[test]
fn co_generated_strategies_keep_mass_on_their_regimes() {
    if std::env::var_os("PROPTEST_CASES").is_none() {
        assert_eq!(
            Config::default().cases,
            DEFAULT_CASES,
            "the floors assume proptest's default case count"
        );
    }
    let regimes = [
        (
            "spine: sites nested three deep",
            census(arb_spine_case(), |s| s.max_nesting >= 3),
            regime_floor(CENSUS_CASES, 1),
        ),
        (
            "spine: two sites in one range",
            census(arb_spine_case(), |s| s.shared_range),
            regime_floor(CENSUS_CASES, 1),
        ),
        (
            "deep spine: sites nested past one block",
            census(arb_deep_spine_case(), |s| s.max_nesting > BLOCK_SLOTS),
            regime_floor(CENSUS_CASES, DEEP_SPINE_CASE_DIVISOR),
        ),
        (
            "wide: a pre-scan past one block",
            census(arb_wide_case(), |s| s.scans_past_one_block() >= 1),
            regime_floor(CENSUS_CASES, WIDE_CASE_DIVISOR),
        ),
        (
            "multi-scan: two such pre-scans",
            census(arb_multi_scan_case(), |s| s.scans_past_one_block() >= 2),
            regime_floor(CENSUS_CASES, MULTI_SCAN_CASE_DIVISOR),
        ),
    ];
    for (regime, count, floor) in &regimes {
        eprintln!("census: `{regime}` in {count} of {CENSUS_CASES} cases (floor {floor})");
    }
    for (regime, count, floor) in regimes {
        assert!(
            count >= floor,
            "generator mass on `{regime}` fell to {count} of {CENSUS_CASES} sampled cases \
             (floor {floor}): an arm has gone dead or normalization collapses its sites"
        );
    }
}
