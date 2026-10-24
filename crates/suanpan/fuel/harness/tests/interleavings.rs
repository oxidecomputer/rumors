//! The interleaving-fuel property, and the census showing its skeletons
//! reach the hard regimes at both ends of the scale.
//!
//! The property runs [`GATE_CASES`] cases unless `PROPTEST_CASES` says
//! otherwise; each case runs four guest instances, one of them at values of
//! about two megabytes and one over 4096 repetitions. The census runs the
//! model alone, from a fixed seed.

use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::{Config, RngAlgorithm, TestRng, TestRunner};
use suanpan_fuel_harness::judge::{census, has_fixed_steady_charge, judge, Census, SCALE_STEP};
use suanpan_fuel_harness::skeleton::{arb_skeleton, UNIT_BITS};

/// The property's case count when `PROPTEST_CASES` is unset.
///
/// A case takes about a twentieth of a second on an otherwise idle host, so
/// the default run takes seconds, far inside nextest's time limit even on a
/// shared host; investigative runs raise `PROPTEST_CASES` into the
/// thousands.
const GATE_CASES: u32 = 256;

/// Returns the property's case count: `PROPTEST_CASES` if set, otherwise
/// [`GATE_CASES`].
fn cases() -> u32 {
    std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|cases| cases.parse().ok())
        .unwrap_or(GATE_CASES)
}

proptest! {
    #![proptest_config(Config::with_cases(cases()))]

    /// Over random interleavings of accumulator operations, fuel grows only
    /// as the cost rules allow when sizes and repetitions grow.
    ///
    /// Each case's skeleton runs at units `u`, `64 · u`, and `4096 · u`, and
    /// at `u` over 8 and 4096 repetitions, each repetition ending in a clear
    /// of every member. Every prefix of the first repetition costs at most
    /// the growth bound times its cost at the unit below
    /// (`judge::growth_bound`), and exactly the same where the prefix is the
    /// same program at every unit. Every prefix of the long run costs what
    /// the 8-repetition run costs over their shared prefix, and afterwards
    /// at most its average per repetition. A skeleton whose repetitions
    /// charge no sized term once storage is grown costs, per repetition over
    /// the last four of eight, at most twice as much at `4096 · u` as at
    /// `64 · u`. Every
    /// prefix also spends at least its steps' floors, and every step's
    /// answer matches the exact model's, so the readings count the program
    /// the model charged.
    #[test]
    fn interleavings_cost_within_the_rules(skeleton in arb_skeleton()) {
        match judge(&skeleton) {
            Ok(margins) => eprintln!("margins: {margins:?}"),
            Err(violation) => return Err(TestCaseError::fail(violation)),
        }
    }
}

/// The seed of the census's skeleton stream.
const CENSUS_SEED: [u8; 32] = *b"suanpan-fuel interleaving census";

/// The number of skeletons the census draws.
const CENSUS_SKELETONS: usize = 64;

/// The skeleton strategy realizes every hard regime the property relies on,
/// at the base unit and at the top unit alike.
///
/// The regimes are steps of opposite signs, cancellations of half a unit or
/// more, borrows across runs half a unit long, storage growth, sparse high
/// digits, shifted updates, and skeletons whose steady repetitions charge no
/// sized term.
///
/// The census replays a fixed stream of skeletons in the model alone. Each
/// floor asserts that its regime is alive at both units, not how often it
/// occurs: the floors sit at a quarter of the counts the stream realized
/// when they were set, so they need re-measuring only when the strategy or
/// proptest's draws deliberately change.
#[test]
fn skeletons_reach_every_hard_regime_at_both_scales() {
    let mut runner = TestRunner::new_with_rng(
        Config::default(),
        TestRng::from_seed(RngAlgorithm::ChaCha, &CENSUS_SEED),
    );
    let strategy = arb_skeleton();
    let top = SCALE_STEP * SCALE_STEP * UNIT_BITS;
    let (mut base, mut scaled, mut fixed_steady) = (Census::default(), Census::default(), 0);
    for _ in 0..CENSUS_SKELETONS {
        let skeleton = strategy
            .new_tree(&mut runner)
            .expect("the skeleton strategy generates")
            .current();
        base += census(&skeleton, UNIT_BITS);
        scaled += census(&skeleton, top);
        fixed_steady += usize::from(has_fixed_steady_charge(&skeleton));
    }
    eprintln!(
        "census over {CENSUS_SKELETONS} skeletons: base {base:?}, top {scaled:?}, \
         fixed-charge steady repetitions {fixed_steady}"
    );
    // Each floor is a quarter of the stream's count when it was set.
    let regimes = [
        (
            "steps of opposite signs",
            51,
            base.mixed_signs,
            scaled.mixed_signs,
        ),
        (
            "cancellations",
            21,
            base.cancellations,
            scaled.cancellations,
        ),
        ("borrow runs", 6, base.borrow_runs, scaled.borrow_runs),
        ("storage growths", 23, base.growths, scaled.growths),
        (
            "sparse high digits",
            14,
            base.sparse_values,
            scaled.sparse_values,
        ),
        (
            "shifted updates",
            128,
            base.shifted_updates,
            scaled.shifted_updates,
        ),
    ];
    for (regime, floor, at_base, at_top) in regimes {
        assert!(
            at_base >= floor && at_top >= floor,
            "the census realizes at least {floor} {regime} at each unit, found {at_base} at the \
             base unit and {at_top} at the top unit"
        );
    }
    assert!(
        fixed_steady >= 8,
        "the census realizes at least 8 skeletons whose steady repetition has a fixed charge, \
         found {fixed_steady}"
    );
}
