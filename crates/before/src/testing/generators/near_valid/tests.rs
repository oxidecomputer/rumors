//! Liveness census for the near-valid encoding strategies.
//!
//! Each strategy claims the regimes its doc lists. A strategy that stops
//! reaching one (a dead arm, a normalization that flattens the deep trees, a
//! damage stage that no longer fires alone) still passes every decoder
//! property, on easier inputs; the census reads that as a failure instead.
//! It samples each strategy under a committed seed and counts the inputs in
//! each regime the strategy claims.

use proptest::strategy::Strategy;

use crate::testing::generators::census::{
    assert_default_case_count, assert_floors, regime_floor, sample, Tally, CENSUS_CASES,
};
#[cfg(all(feature = "serde", feature = "borsh"))]
use crate::testing::oracles::codec::verdict;

use super::{
    arb_clock_encoding, arb_party_encoding, arb_rank_encoding, arb_ranked_encoding,
    arb_span_encoding, arb_version_encoding, NearValid, Regime,
};

/// The census seed: names the sampled corpus (see
/// [`crate::testing::rng::strategy_rng`]).
const CENSUS_SEED: u64 = 13;

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

/// Counts the inputs in each of `classes` among the census's samples of
/// `strategy`, as tallies against the floor of a property drawing the
/// strategy at the default case count.
fn census(
    strategy: impl Strategy<Value = NearValid>,
    name: &str,
    classes: Vec<Class>,
) -> Vec<Tally> {
    let mut counts = vec![0usize; classes.len()];
    for input in sample(&strategy, CENSUS_SEED) {
        for (class, count) in classes.iter().zip(&mut counts) {
            *count += usize::from((class.holds)(&input));
        }
    }
    let floor = regime_floor(CENSUS_CASES, 1);
    classes
        .into_iter()
        .zip(counts)
        .map(|(class, count)| Tally::new(format!("{name} {}", class.name), count, floor))
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
///
/// [`REGIME_CASES_PER_RUN`]: crate::testing::generators::census::REGIME_CASES_PER_RUN
#[test]
fn near_valid_strategies_keep_mass_on_their_regimes() {
    assert_default_case_count();
    use Regime::*;
    let tallies: Vec<Tally> = [
        census(
            arb_version_encoding(),
            "version",
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
        census(
            arb_party_encoding(),
            "party",
            [
                DeepCanonical,
                DeepNonCanonical,
                CollapsiblePair,
                CutInsideTree,
            ]
            .map(regime)
            .into(),
        ),
        census(
            arb_rank_encoding(),
            "rank",
            [
                LongRank,
                TooWideRankHeader,
                ZeroFinalGroup,
                DirtyRankPadding,
            ]
            .map(regime)
            .into(),
        ),
        census(
            arb_clock_encoding(),
            "clock",
            vec![regime(ClockVersionDefect)],
        ),
        census(
            arb_span_encoding(),
            "span",
            [
                regime(UnorderedSpan),
                regime(DeepLowerEndpoint),
                regime(StructuralOverOrdering),
            ]
            .into_iter()
            .chain(framed_lower_endpoint())
            .collect(),
        ),
        census(
            arb_ranked_encoding(),
            "ranked",
            vec![regime(MismatchedRank)],
        ),
    ]
    .into_iter()
    .flatten()
    .collect();
    assert_floors(
        &tallies,
        "an arm has gone dead or construction no longer reaches the class",
    );
}
