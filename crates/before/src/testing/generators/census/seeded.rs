//! Fixed-seed census floors, for the censuses with a regime that has no
//! stated designed mass.
//!
//! Each such census samples its strategies as a fixed corpus under a
//! committed seed, counts the cases in each claimed regime, and requires
//! every count to meet the floor [`regime_floor`] derives from the case
//! count of the properties that draw from the strategy. The floor holds for
//! the committed corpus, not for a fresh one. A census moves to the parent
//! module's derived floors when every one of its regimes has a designed
//! mass. Each census names its own seed, so one census's corpus never moves
//! when another's does.

use proptest::strategy::{Strategy, ValueTree};
use proptest::test_runner::{Config, TestRunner};

use crate::testing::rng::strategy_rng;

/// Proptest's default case count, which every floor here assumes.
pub(crate) const DEFAULT_CASES: u32 = 256;

pub(crate) use super::REGIME_CASES_PER_RUN;

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
    let regime_cases = usize::try_from(REGIME_CASES_PER_RUN).expect("a small count fits in usize");
    (regime_cases * census_cases).div_ceil(cases_per_run)
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
