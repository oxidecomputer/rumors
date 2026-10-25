//! Liveness census for the near-valid encoding strategies.
//!
//! Each strategy claims the regimes its doc lists. A strategy that stops
//! reaching one (a dead arm, a normalization that flattens the deep trees, a
//! damage stage that no longer fires alone) still passes every decoder
//! property, on easier inputs; the census reads that as a failure instead.
//! It samples each strategy afresh and counts the inputs in each regime the
//! strategy claims, against floors derived from each regime's designed mass
//! (see [`census`](crate::testing::generators::census)).

use proptest::strategy::Strategy;
use proptest::test_runner::TestRunner;

use crate::testing::generators::census::{assert_floors, runner, sample, Mass, Tally};
#[cfg(all(feature = "serde", feature = "borsh"))]
use crate::testing::oracles::codec::verdict;

use super::{
    arb_clock_encoding, arb_party_encoding, arb_rank_encoding, arb_ranked_encoding,
    arb_span_encoding, arb_version_encoding, NearValid, Regime,
};

/// Inputs sampled from each strategy: enough that the thinnest regime, a deep
/// plant in a party, gets a positive floor from its designed mass of
/// `27/1000`.
const CENSUS_INPUTS: usize = 3000;

/// One class of inputs the census counts: its name, its membership test,
/// and its designed mass.
struct Class {
    /// The name the census reports.
    name: String,
    /// Whether an input belongs to the class.
    holds: Box<dyn Fn(&NearValid) -> bool>,
    /// The class's designed mass.
    mass: Mass,
}

/// The inputs that reach `regime`, of designed mass `mass`.
fn regime(regime: Regime, mass: Mass) -> Class {
    Class {
        name: format!("{regime:?}"),
        holds: Box::new(move |input| input.regimes.contains(&regime)),
        mass,
    }
}

/// The spans whose lower endpoint the specification frames, which the
/// human-readable record leg of the span property needs.
///
/// Designed mass `7/10`: the specification frames a lower endpoint whenever
/// its whole sealed encoding leads the input. The endpoint-pair arm (four in
/// five) leaves the lower endpoint undamaged five times in six and adds no
/// byte edit three times in four, and the structural arm (one in five)
/// always writes a canonical lower endpoint: at least
/// `4/5 · 5/6 · 3/4 + 1/5 = 7/10`.
#[cfg(all(feature = "serde", feature = "borsh"))]
fn framed_lower_endpoint() -> Option<Class> {
    Some(Class {
        name: "framed lower endpoint".into(),
        holds: Box::new(|input| verdict::span_lower_len(&input.bytes).is_some()),
        mass: Mass::new(7, 10),
    })
}

/// No class: without both codec features, the decoder properties and their
/// record leg do not exist.
#[cfg(not(all(feature = "serde", feature = "borsh")))]
fn framed_lower_endpoint() -> Option<Class> {
    None
}

/// Counts the inputs in each of `classes` among [`CENSUS_INPUTS`] samples of
/// `strategy`, as tallies serving a property that draws the strategy at the
/// configured case count.
fn census(
    strategy: impl Strategy<Value = NearValid>,
    runner: &mut TestRunner,
    name: &str,
    classes: Vec<Class>,
) -> Vec<Tally> {
    let mut counts = vec![0usize; classes.len()];
    for input in sample(&strategy, runner, CENSUS_INPUTS) {
        for (class, count) in classes.iter().zip(&mut counts) {
            *count += usize::from((class.holds)(&input));
        }
    }
    classes
        .into_iter()
        .zip(counts)
        .map(|(class, count)| {
            Tally::new(format!("{name} {}", class.name), count, class.mass).serving(1)
        })
        .collect()
}

/// Every near-valid strategy keeps enough mass on each regime its doc claims
/// that each run of the decoder property drawing from it expects
/// [`REGIME_CASES_PER_RUN`] inputs there.
///
/// Each strategy's doc lists the [`Regime`]s it claims, and the census
/// counts exactly those, sampling every strategy that a decoder property
/// draws from, together with, under both codec features, the spans whose
/// lower endpoint the specification frames.
///
/// # Designed masses
///
/// The tree and rank strategies draw structured inputs twelve times in
/// thirteen. Their damage is none three times in ten, a plant alone twice,
/// and a bit edit alone twice, a cut two times in five of those
/// ([`arb_damage`](super::damage::arb_damage)). The composites leave an
/// endpoint undamaged five times in six and add no byte edit three times in
/// four.
///
/// A spine of `L` levels, drawn half the time with `L` from 30 to 399, puts
/// two sibling leaves at depth `L`. Normalization keeps a version spine that
/// deep when those two heights differ, which they do with probability at
/// least `0.83`: the likeliest height has probability `1/6` plus
/// `2/(15 · 2^64)`. So a version tree is deeper than 64 with probability at
/// least `1/2 · 335/370 · 0.83 ≥ 0.3757`. A party spine, whose bottom leaf
/// is owned, keeps a depth past 64 unless the `L − 64` leaves nearest the
/// bottom are all owned, which happens with probability `2^−(L−64)`; so a
/// party tree is deeper than 64 with probability at least `1/2 · 0.9027`.
/// Two independent version trees are equal with probability at most that of
/// the likeliest height, since a tree's rank moves with any one leaf's
/// height, so they differ with probability at least `0.83`.
///
/// | Regime | Mass | Argument |
/// |---|---|---|
/// | version `DeepCanonical` | `1/10` | an undamaged deep tree: `12/13 · 3/10 · 0.3757` |
/// | version `DeepNonCanonical` | `1/20` | a plant alone, at the deepest leaf or at one of the `L + 1` leaves deeper than 64: `12/13 · 2/10 · 0.3757 · (1/3 + 2/3 · 0.654)` |
/// | version `CollapsiblePair`, `NegativeHeight` | `9/100` each | a plant alone of that kind: `12/13 · 2/10 · 1/2` |
/// | version, party `CutInsideTree` | `7/100` | a cut alone: `12/13 · 2/10 · 2/5` |
/// | party `DeepCanonical` | `12/100` | an undamaged deep tree: `12/13 · 3/10 · 1/2 · 0.9027` |
/// | party `DeepNonCanonical` | `27/1000` | a plant alone at the deepest owned leaf of a deep tree: `12/13 · 2/10 · 1/2 · 0.9027 · 1/3` |
/// | party `CollapsiblePair` | `18/100` | a plant alone: `12/13 · 2/10` |
/// | rank `LongRank` | `7/100` | undamaged, with at least 459 fraction digits drawn, of which stripping trailing zeros removes at most ten: `12/13 · 4/9 · 1/5 · 2541/2800 · (1 − 2^−11)` |
/// | rank `TooWideRankHeader`, `ZeroFinalGroup` | `1/10` each | that damage: `12/13 · 1/9` |
/// | rank `DirtyRankPadding` | `5/100` | that damage, setting a padding bit for at least 128 of the 255 bytes: `12/13 · 1/9 · 128/255` |
/// | clock `ClockVersionDefect` | `45/1000` | an undamaged party, a plant alone in the version, no edit: `3/10 · 2/10 · 3/4` |
/// | span `UnorderedSpan` | `8/100` | reversed endpoints of unequal trees, undamaged and unedited: `4/5 · 1/4 · 0.83 · (5/6)² · 3/4` |
/// | span `DeepLowerEndpoint` | `7/100` | ordered or equal endpoints over a deep first tree, undamaged and unedited: `4/5 · 2/4 · 0.3757 · (5/6)² · 3/4` |
/// | span `StructuralOverOrdering` | `1/5` | the structural arm |
/// | ranked `MismatchedRank` | `1/10` | another tree's differing rank, an undamaged version, no edit: `1/5 · 0.83 · 5/6 · 3/4` |
///
/// The `0.654` is the mean over `L` from 65 to 399 of `(L − 63)/(L + 1)`,
/// the share of an undamaged deep spine's leaves deeper than 64. A fraction
/// that keeps 449 digits or more makes a rank wider than the rank decoder's
/// 64-byte buffer, since each group of eight digits takes nine bits.
///
/// [`REGIME_CASES_PER_RUN`]: crate::testing::generators::census::REGIME_CASES_PER_RUN
#[test]
fn near_valid_strategies_keep_mass_on_their_regimes() {
    use Regime::*;
    let mut runner = runner();
    let tallies: Vec<Tally> = [
        census(
            arb_version_encoding(),
            &mut runner,
            "version",
            vec![
                regime(DeepCanonical, Mass::new(1, 10)),
                regime(DeepNonCanonical, Mass::new(1, 20)),
                regime(CollapsiblePair, Mass::new(9, 100)),
                regime(NegativeHeight, Mass::new(9, 100)),
                regime(CutInsideTree, Mass::new(7, 100)),
            ],
        ),
        census(
            arb_party_encoding(),
            &mut runner,
            "party",
            vec![
                regime(DeepCanonical, Mass::new(12, 100)),
                regime(DeepNonCanonical, Mass::new(27, 1000)),
                regime(CollapsiblePair, Mass::new(18, 100)),
                regime(CutInsideTree, Mass::new(7, 100)),
            ],
        ),
        census(
            arb_rank_encoding(),
            &mut runner,
            "rank",
            vec![
                regime(LongRank, Mass::new(7, 100)),
                regime(TooWideRankHeader, Mass::new(1, 10)),
                regime(ZeroFinalGroup, Mass::new(1, 10)),
                regime(DirtyRankPadding, Mass::new(5, 100)),
            ],
        ),
        census(
            arb_clock_encoding(),
            &mut runner,
            "clock",
            vec![regime(ClockVersionDefect, Mass::new(45, 1000))],
        ),
        census(
            arb_span_encoding(),
            &mut runner,
            "span",
            [
                regime(UnorderedSpan, Mass::new(8, 100)),
                regime(DeepLowerEndpoint, Mass::new(7, 100)),
                regime(StructuralOverOrdering, Mass::new(1, 5)),
            ]
            .into_iter()
            .chain(framed_lower_endpoint())
            .collect(),
        ),
        census(
            arb_ranked_encoding(),
            &mut runner,
            "ranked",
            vec![regime(MismatchedRank, Mass::new(1, 10))],
        ),
    ]
    .into_iter()
    .flatten()
    .collect();
    assert_floors(
        &tallies,
        CENSUS_INPUTS,
        "an arm has gone dead or construction no longer reaches the class",
    );
}
