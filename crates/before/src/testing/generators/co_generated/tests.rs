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

use crate::recurse::descend;
use crate::testing::oracles::tree;
use crate::testing::rng::strategy_rng;

use crate::version::tick::BLOCK_SLOTS;

use super::{
    arb_multi_scan_case, arb_spine_case, arb_wide_case, TickCase, DEFAULT_CASES,
    MULTI_SCAN_CASE_DIVISOR, WIDE_CASE_DIVISOR,
};

/// The census seed: names the sampled corpus (see
/// [`crate::testing::rng::strategy_rng`]).
const CENSUS_SEED: u64 = 11;

/// Sampled cases per strategy.
const CENSUS_CASES: usize = 400;

/// The regime cases each default run of a strategy's property must expect.
///
/// A defect that every regime case exposes then escapes one default run with
/// probability at most `e` to the minus this count: under a quarter of a
/// percent.
const REGIME_CASES_PER_RUN: usize = 6;

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
        sites.visit(&case.party, &case.version, 0, 0);
        sites
    }

    /// Visit the region where `party` meets `version`, `nesting` sites deep,
    /// and return its site count at this nesting level and at every level.
    ///
    /// A site is a party branch with an owned left child and a nonempty right
    /// child over a version branch, which is where the tick walk and its
    /// pre-scan reserve a memo slot. The walk reaches no site under an owned
    /// or unowned party leaf, nor under a version leaf.
    fn visit(
        &mut self,
        party: &tree::Party,
        version: &tree::Version,
        nesting: usize,
        depth: usize,
    ) -> (usize, usize) {
        let (tree::Party::Node(party_left, party_right), tree::Version::Node(_, left, right)) =
            (party, version)
        else {
            return (0, 0);
        };
        if matches!(**party_left, tree::Party::Leaf(true)) && !party_right.is_empty() {
            self.max_nesting = self.max_nesting.max(nesting + 1);
            let (next_level, inside) = descend!(
                depth + 1,
                self.visit(party_right, right, nesting + 1, depth + 1)
            );
            self.shared_range |= next_level >= 2;
            if nesting == 0 {
                self.scan_slots.push(1 + inside);
            }
            return (1, 1 + inside);
        }
        let (left_level, left_all) =
            descend!(depth + 1, self.visit(party_left, left, nesting, depth + 1));
        let (right_level, right_all) = descend!(
            depth + 1,
            self.visit(party_right, right, nesting, depth + 1)
        );
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

/// The census floor for a property that runs the default case count divided
/// by `divisor`: the fewest regime cases among [`CENSUS_CASES`] samples at
/// which each default run expects [`REGIME_CASES_PER_RUN`] of them.
fn floor(divisor: u32) -> usize {
    let cases_per_run = usize::try_from(DEFAULT_CASES.div_ceil(divisor))
        .expect("a case count fits in usize on every tested target");
    (REGIME_CASES_PER_RUN * CENSUS_CASES).div_ceil(cases_per_run)
}

/// Every co-generated strategy keeps enough generator mass on its pre-scan
/// regime that each default run of its property expects
/// [`REGIME_CASES_PER_RUN`] cases there.
///
/// The regimes: sites nested three deep and a range holding two sites side by
/// side (spine), a pre-scan reserving more slots than one memo block holds
/// (wide), and two such pre-scans in one walk (multi-scan). Each floor follows
/// from that premise and its property's case count, so a strategy whose
/// regime mass falls below what its own runs need reads as a failure here,
/// whatever its count was when last measured.
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
            floor(1),
        ),
        (
            "spine: two sites in one range",
            census(arb_spine_case(), |s| s.shared_range),
            floor(1),
        ),
        (
            "wide: a pre-scan past one block",
            census(arb_wide_case(), |s| s.scans_past_one_block() >= 1),
            floor(WIDE_CASE_DIVISOR),
        ),
        (
            "multi-scan: two such pre-scans",
            census(arb_multi_scan_case(), |s| s.scans_past_one_block() >= 2),
            floor(MULTI_SCAN_CASE_DIVISOR),
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
