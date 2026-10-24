//! Census floors: the shared machinery of the tests that require a strategy
//! to keep generator mass on each regime it claims.
//!
//! A strategy that stops reaching a regime (a dead arm, a normalization that
//! collapses what it builds) still passes every property over it, on easier
//! inputs. A census reads that as a failure instead. It samples the strategy
//! as a fixed corpus under a committed seed, counts the cases in each claimed
//! regime, and requires every count to meet the floor [`regime_floor`]
//! derives from the case count of the properties that draw from the
//! strategy. Each census names its own seed, so one census's corpus never
//! moves when another's does.

use proptest::strategy::{Strategy, ValueTree};
use proptest::test_runner::{Config, TestRunner};

use crate::testing::rng::strategy_rng;

use super::DEFAULT_CASES;

/// The regime cases each default run of a property must expect, under every
/// census floor derived by [`regime_floor`].
///
/// A defect that every regime case exposes then escapes one default run with
/// probability at most `e` to the minus this count: under a quarter of a
/// percent.
pub(crate) const REGIME_CASES_PER_RUN: usize = 6;

/// Returns the census floor for a regime: the fewest regime cases among
/// `census_cases` samples at which each default run of a property expects
/// [`REGIME_CASES_PER_RUN`] of them.
///
/// The property runs the default case count divided by `divisor`. The floor
/// follows from that premise and the property's case count, not
/// from any measured reading, so a strategy whose regime mass falls below
/// what its own property needs fails its census whatever it last measured.
pub(crate) fn regime_floor(census_cases: usize, divisor: u32) -> usize {
    let cases_per_run = usize::try_from(DEFAULT_CASES.div_ceil(divisor))
        .expect("a case count fits in usize on every tested target");
    (REGIME_CASES_PER_RUN * census_cases).div_ceil(cases_per_run)
}

/// Cases [`sample`] draws from each strategy.
pub(crate) const CENSUS_CASES: usize = 400;

/// Asserts the premise every floor rests on: unless `PROPTEST_CASES`
/// overrides it, proptest runs [`DEFAULT_CASES`] cases.
pub(crate) fn assert_default_case_count() {
    if std::env::var_os("PROPTEST_CASES").is_none() {
        assert_eq!(
            Config::default().cases,
            DEFAULT_CASES,
            "the floors assume proptest's default case count"
        );
    }
}

/// Draws [`CENSUS_CASES`] values from `strategy` under the census seed `seed`.
///
/// Each call draws from a fresh runner, so one strategy's draws never shift
/// another's.
pub(crate) fn sample<S: Strategy>(strategy: &S, seed: u64) -> impl Iterator<Item = S::Value> + '_ {
    let mut runner = TestRunner::new_with_rng(Config::default(), strategy_rng(seed));
    (0..CENSUS_CASES).map(move |_| strategy.new_tree(&mut runner).expect("strategy").current())
}

/// One regime's count among a census's [`CENSUS_CASES`] samples, and the
/// floor it must meet.
pub(crate) struct Tally {
    /// The regime, named as the census reports it.
    pub(crate) regime: String,
    /// The sampled cases in the regime.
    pub(crate) count: usize,
    /// The fewest cases the regime may hold.
    pub(crate) floor: usize,
}

impl Tally {
    /// Names `regime`'s `count` and the `floor` it must meet.
    pub(crate) fn new(regime: impl Into<String>, count: usize, floor: usize) -> Tally {
        Tally {
            regime: regime.into(),
            count,
            floor,
        }
    }
}

/// Reports every tally, then asserts that each meets its floor.
///
/// Every count prints before the first assertion, so one failing run shows
/// the whole census. `cause` names what a count below its floor most likely
/// means for the strategies this census samples.
pub(crate) fn assert_floors(tallies: &[Tally], cause: &str) {
    for Tally {
        regime,
        count,
        floor,
    } in tallies
    {
        eprintln!("census: `{regime}` in {count} of {CENSUS_CASES} cases (floor {floor})");
    }
    for Tally {
        regime,
        count,
        floor,
    } in tallies
    {
        assert!(
            count >= floor,
            "generator mass on `{regime}` fell to {count} of {CENSUS_CASES} sampled cases \
             (floor {floor}): {cause}"
        );
    }
}
