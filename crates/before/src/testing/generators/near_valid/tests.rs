//! Liveness census for the near-valid encoding strategies.
//!
//! Each strategy claims the regimes its doc lists. A strategy that stops
//! reaching one (a dead arm, a normalization that flattens the deep trees, a
//! damage stage that no longer fires alone) still passes every decoder
//! property, on easier inputs; the census reads that as a failure instead.
//! It samples each strategy under a committed seed and counts the inputs in
//! each regime the strategy claims.

use proptest::strategy::{Strategy, ValueTree};
use proptest::test_runner::{Config, TestRunner};

use crate::testing::generators::co_generated::DEFAULT_CASES;
#[cfg(all(feature = "serde", feature = "borsh"))]
use crate::testing::oracles::codec::verdict;
use crate::testing::rng::strategy_rng;

use super::{
    arb_clock_encoding, arb_party_encoding, arb_rank_encoding, arb_ranked_encoding,
    arb_span_encoding, arb_version_encoding, NearValid, Regime,
};

/// The census seed: names the sampled corpus (see
/// [`crate::testing::rng::strategy_rng`]).
const CENSUS_SEED: u64 = 13;

/// Sampled inputs per strategy.
const CENSUS_CASES: usize = 400;

/// The regime inputs each default run of a decoder property must expect.
///
/// A defect that every input in a regime exposes then escapes one default
/// run with probability at most `e` to the minus this count: under a quarter
/// of a percent.
const REGIME_CASES_PER_RUN: usize = 6;

/// The fewest regime inputs among [`CENSUS_CASES`] samples at which each
/// default run of a property drawing from the strategy expects
/// [`REGIME_CASES_PER_RUN`] of them.
fn floor() -> usize {
    let cases_per_run =
        usize::try_from(DEFAULT_CASES).expect("a case count fits in usize on every tested target");
    (REGIME_CASES_PER_RUN * CENSUS_CASES).div_ceil(cases_per_run)
}

/// One class of inputs the census counts: its name and its membership test.
struct Class {
    /// The name the census reports.
    name: String,
    /// Whether an input belongs to the class.
    holds: Box<dyn Fn(&NearValid) -> bool>,
}

/// The inputs that reach `regime`.
fn regime(regime: Regime) -> Class {
    Class {
        name: format!("{regime:?}"),
        holds: Box::new(move |input| input.regimes.contains(&regime)),
    }
}

/// The spans whose lower endpoint the specification frames, which the
/// human-readable record leg of the span property needs.
#[cfg(all(feature = "serde", feature = "borsh"))]
fn framed_lower_endpoint() -> Option<Class> {
    Some(Class {
        name: "framed lower endpoint".into(),
        holds: Box::new(|input| verdict::span_lower_len(&input.bytes).is_some()),
    })
}

/// No class: without both codec features, the decoder properties and their
/// record leg do not exist.
#[cfg(not(all(feature = "serde", feature = "borsh")))]
fn framed_lower_endpoint() -> Option<Class> {
    None
}

/// Samples `strategy` [`CENSUS_CASES`] times under the census seed, and
/// counts the inputs in each of `classes`.
///
/// Each strategy draws from a fresh runner, so one strategy's draws never
/// shift another's.
fn census(strategy: impl Strategy<Value = NearValid>, classes: Vec<Class>) -> Vec<(String, usize)> {
    let mut runner = TestRunner::new_with_rng(Config::default(), strategy_rng(CENSUS_SEED));
    let mut counts = vec![0usize; classes.len()];
    for _ in 0..CENSUS_CASES {
        let input = strategy.new_tree(&mut runner).expect("strategy").current();
        for (class, count) in classes.iter().zip(&mut counts) {
            *count += usize::from((class.holds)(&input));
        }
    }
    classes
        .into_iter()
        .map(|class| class.name)
        .zip(counts)
        .collect()
}

/// Every near-valid strategy keeps enough mass on each regime its doc claims
/// that each default run of the decoder property drawing from it expects
/// [`REGIME_CASES_PER_RUN`] inputs there.
///
/// Each strategy's doc lists the [`Regime`]s it claims, and the census
/// counts exactly those, sampling every strategy that a decoder property
/// draws from, together with, under both codec features, the spans whose
/// lower endpoint the specification frames.
///
/// Each floor follows from that premise and the properties' default case
/// count, so a regime whose mass falls below what a run needs reads as a
/// failure here, whatever its count was when last measured.
#[test]
fn near_valid_strategies_keep_mass_on_their_regimes() {
    if std::env::var_os("PROPTEST_CASES").is_none() {
        assert_eq!(
            Config::default().cases,
            DEFAULT_CASES,
            "the floors assume proptest's default case count"
        );
    }
    use Regime::*;
    let tallies = [
        (
            "version",
            census(
                arb_version_encoding(),
                [
                    DeepCanonical,
                    DeepNonCanonical,
                    CollapsiblePair,
                    NegativeHeight,
                    CutInsideTree,
                ]
                .map(regime)
                .into(),
            ),
        ),
        (
            "party",
            census(
                arb_party_encoding(),
                [
                    DeepCanonical,
                    DeepNonCanonical,
                    CollapsiblePair,
                    CutInsideTree,
                ]
                .map(regime)
                .into(),
            ),
        ),
        (
            "rank",
            census(
                arb_rank_encoding(),
                [
                    LongRank,
                    TooWideRankHeader,
                    ZeroFinalGroup,
                    DirtyRankPadding,
                ]
                .map(regime)
                .into(),
            ),
        ),
        (
            "clock",
            census(arb_clock_encoding(), vec![regime(ClockVersionDefect)]),
        ),
        (
            "span",
            census(
                arb_span_encoding(),
                [
                    regime(UnorderedSpan),
                    regime(DeepLowerEndpoint),
                    regime(StructuralOverOrdering),
                ]
                .into_iter()
                .chain(framed_lower_endpoint())
                .collect(),
            ),
        ),
        (
            "ranked",
            census(arb_ranked_encoding(), vec![regime(MismatchedRank)]),
        ),
    ];
    let floor = floor();
    for (strategy, counts) in &tallies {
        for (class, count) in counts {
            eprintln!(
                "census: {strategy} `{class}` in {count} of {CENSUS_CASES} inputs (floor {floor})"
            );
        }
    }
    for (strategy, counts) in tallies {
        for (class, count) in counts {
            assert!(
                count >= floor,
                "{strategy} mass on `{class}` fell to {count} of {CENSUS_CASES} sampled inputs \
                 (floor {floor}): an arm has gone dead or construction no longer reaches the class"
            );
        }
    }
}
