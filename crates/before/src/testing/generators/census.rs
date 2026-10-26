//! Census floors: the shared machinery of the tests that require a strategy
//! to keep generator mass on each regime it claims.
//!
//! A strategy that stops reaching a regime (a dead arm, a normalization that
//! collapses what it builds) still passes every property over it, on easier
//! inputs. A census reads that as a failure instead. It samples the strategy
//! afresh on every run, counts the samples in each claimed regime, and
//! requires every count to meet a floor derived from the regime's designed
//! mass. A census with a regime that has no stated designed mass samples a
//! committed corpus instead, against the floors of [`seeded`].
//!
//! # Designed mass
//!
//! A regime's *designed mass* is a lower bound on the probability that one
//! draw of its strategy lands in the regime, read off the strategy's weights
//! by an argument stated beside the regime. A strategy that works as
//! designed meets it; the bound may be loose, and a looser bound only lowers
//! the floor. [`Mass`] holds it as an exact fraction.
//!
//! # Why a working strategy meets its floors
//!
//! A census draws `n` independent samples, and a strategy working as
//! designed lands each in a regime of designed mass `p` with probability at
//! least `p`. The regime's count then dominates a binomial count of mean
//! `μ = n p`, and Chernoff's lower tail bound gives
//! `P(count ≤ μ − t) ≤ exp(−t² / 2μ)`. Taking `t = sqrt(2 μ ln 2^30)` makes
//! the right side `2^−30`, and [`floor`] returns an integer no greater than
//! `μ − t`. That is: a working strategy falls below a floor with probability
//! at most `2^−30` per regime per run, so a census of `r` regimes fails
//! falsely with probability at most `r · 2^−30`.
//!
//! A floor detects a dead regime only if it is positive, which needs
//! `μ − t ≥ 1`, about `μ ≥ 44`. Each census draws enough samples that every
//! regime it counts clears that, and [`assert_floors`] checks it.
//!
//! # Why a regime's mass serves its properties
//!
//! A property drawing a strategy reaches a regime on about `p` of its cases.
//! Where a census guards a regime for a property, it also requires that each
//! run of that property expect [`REGIME_CASES_PER_RUN`] cases in the regime,
//! at the case count the run is configured with: `p · cases ≥`
//! [`REGIME_CASES_PER_RUN`]. The case count is read from proptest's
//! configuration at run time, so `PROPTEST_CASES` moves it. A run configured
//! with too few cases for a regime's designed mass fails here and names the
//! regime: such a run cannot expect the coverage this check guarantees.
//!
//! # Reproducing a failure
//!
//! A census draws its seed fresh on every run and prints it before
//! sampling. Setting `PROPTEST_RNG_SEED` to the printed value draws the same
//! samples again.

use proptest::strategy::{Strategy, ValueTree};
use proptest::test_runner::{Config, RngSeed, TestRunner};

use crate::testing::property::divided_cases;

pub(crate) mod seeded;
mod tests;

/// The regime cases each run of a property must expect, under every
/// requirement [`assert_floors`] checks and every fixed-seed floor
/// [`seeded::regime_floor`] derives.
///
/// A defect that every regime case exposes then escapes one run with
/// probability at most `e` to the minus this count: under a quarter of a
/// percent.
pub(crate) const REGIME_CASES_PER_RUN: u64 = 6;

/// The negated base-2 logarithm of the probability with which a working
/// strategy may fall below one floor in one run.
const FALSE_FAILURE_BITS: u128 = 30;

/// An upper bound on `ln 2` (which is `0.6931471…`), as a fraction over
/// [`LN_2_DENOMINATOR`], so [`floor`] can stay in integer arithmetic.
const LN_2_NUMERATOR: u128 = 693_148;

/// The denominator of [`LN_2_NUMERATOR`].
const LN_2_DENOMINATOR: u128 = 1_000_000;

/// A regime's designed mass: a lower bound on the probability that one draw
/// of its strategy lands in the regime, as an exact fraction.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Mass {
    /// The fraction's numerator.
    numerator: u64,
    /// The fraction's denominator.
    denominator: u64,
}

impl Mass {
    /// The mass `numerator / denominator`.
    ///
    /// # Panics
    ///
    /// Panics unless `0 < numerator <= denominator`: a regime of no designed
    /// mass has no floor, and a probability is at most one.
    pub(crate) const fn new(numerator: u64, denominator: u64) -> Mass {
        assert!(
            0 < numerator && numerator <= denominator,
            "a designed mass lies in (0, 1]"
        );
        Mass {
            numerator,
            denominator,
        }
    }
}

/// Returns the census floor for a regime of designed mass `mass` among
/// `samples` draws: an integer no greater than `μ − t`, where `μ` is
/// `samples · mass` and `t = sqrt(2 μ ln 2^30)` (see the module doc).
///
/// It returns `⌊μ⌋ − ⌈t'⌉`, saturating at zero, where `t'` is `t` computed
/// with [`LN_2_NUMERATOR`]'s upper bound in place of `ln 2`. Both roundings
/// and the bound move the result down, so it never exceeds `μ − t`. The
/// arithmetic is exact: `⌈t'⌉` is the least `m` with
/// `m² · denominator · 10⁶ ≥ 2 · 30 · 693148 · samples · numerator`.
pub(crate) fn floor(samples: usize, mass: Mass) -> u64 {
    let samples = u128::try_from(samples).expect("a sample count fits in u128");
    let (numerator, denominator) = (u128::from(mass.numerator), u128::from(mass.denominator));
    let mean_floor = samples * numerator / denominator;
    let target = 2 * FALSE_FAILURE_BITS * LN_2_NUMERATOR * samples * numerator;
    let scale = denominator * LN_2_DENOMINATOR;
    let covers = |m: u128| m * m * scale >= target;
    // Start from the floating-point root, then step to the exact least `m`.
    let mut deviation = ((target as f64) / (scale as f64)).sqrt() as u128;
    while !covers(deviation) {
        deviation += 1;
    }
    while deviation > 0 && covers(deviation - 1) {
        deviation -= 1;
    }
    u64::try_from(mean_floor.saturating_sub(deviation)).expect("a floor fits in u64")
}

/// Returns a runner whose seed is fresh on every run, or fixed by
/// `PROPTEST_RNG_SEED`, and prints that seed so a failure can be replayed.
pub(crate) fn runner() -> TestRunner {
    let config = Config::default();
    let seed = match config.rng_seed {
        RngSeed::Fixed(seed) => seed,
        RngSeed::Random => rand::random(),
    };
    eprintln!("census seed {seed}: rerun with PROPTEST_RNG_SEED={seed} to draw the same samples");
    TestRunner::new(Config {
        rng_seed: RngSeed::Fixed(seed),
        ..config
    })
}

/// Draws `samples` values from `strategy` with `runner`.
pub(crate) fn sample<'a, S: Strategy>(
    strategy: &'a S,
    runner: &'a mut TestRunner,
    samples: usize,
) -> impl Iterator<Item = S::Value> + 'a {
    (0..samples).map(move |_| strategy.new_tree(runner).expect("strategy").current())
}

/// One regime's count among a census's samples, the designed mass its floor
/// derives from, and the case divisor of the property it serves, if any.
pub(crate) struct Tally {
    /// The regime, named as the census reports it.
    regime: String,
    /// The sampled cases in the regime.
    count: usize,
    /// The regime's designed mass.
    mass: Mass,
    /// The case divisor of the most demanding property relying on the
    /// regime, whose runs must expect [`REGIME_CASES_PER_RUN`] cases in it.
    serves: Option<u32>,
}

impl Tally {
    /// Names `regime`'s `count` and its designed `mass`.
    pub(crate) fn new(regime: impl Into<String>, count: usize, mass: Mass) -> Tally {
        Tally {
            regime: regime.into(),
            count,
            mass,
            serves: None,
        }
    }

    /// Requires the regime's mass to give each run of a property at the
    /// configured case count divided by `divisor` (see [`divided_cases`])
    /// [`REGIME_CASES_PER_RUN`] regime cases in expectation.
    pub(crate) fn serving(self, divisor: u32) -> Tally {
        Tally {
            serves: Some(divisor),
            ..self
        }
    }
}

/// Reports every tally against its floor among `samples` draws, then
/// asserts that each floor is positive, that each mass serves its property,
/// and that each count meets its floor.
///
/// Every count prints before the first assertion, so one failing run shows
/// the whole census. `cause` names what a count below its floor most likely
/// means for the strategies this census samples.
pub(crate) fn assert_floors(tallies: &[Tally], samples: usize, cause: &str) {
    for tally in tallies {
        eprintln!(
            "census: `{}` in {} of {samples} cases (floor {})",
            tally.regime,
            tally.count,
            floor(samples, tally.mass)
        );
    }
    for tally in tallies {
        let floor = floor(samples, tally.mass);
        assert!(
            floor > 0,
            "`{}`'s designed mass sets no positive floor among {samples} samples: the census \
             must draw more",
            tally.regime
        );
        if let Some(divisor) = tally.serves {
            let cases = u64::from(divided_cases(Config::default().cases, divisor));
            assert!(
                tally.mass.numerator * cases >= REGIME_CASES_PER_RUN * tally.mass.denominator,
                "`{}`'s designed mass, {}/{}, gives a run of {cases} cases fewer than \
                 {REGIME_CASES_PER_RUN} expected regime cases",
                tally.regime,
                tally.mass.numerator,
                tally.mass.denominator
            );
        }
        let count = u64::try_from(tally.count).expect("a count fits in u64");
        assert!(
            count >= floor,
            "generator mass on `{}` fell to {count} of {samples} sampled cases (floor {floor}): \
             {cause}",
            tally.regime
        );
    }
}
