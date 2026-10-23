//! Tests for population-relative trace operands, the population cap, the
//! depth budget, and the generator's reach.

use proptest::prelude::*;
use proptest::sample::Index;
use proptest::strategy::ValueTree;
use proptest::test_runner::{Config, TestRunner};
use rand::SeedableRng;
use rand_chacha::ChaChaRng;

use crate::testing::generators::{regime_floor, DEFAULT_CASES};
use crate::testing::oracles::function::{ev_res, id_res, FunctionClock};
use crate::testing::oracles::{flat, function, tree};
use crate::testing::rng::strategy_rng;
use crate::{Clock, Count, Version};

use super::{
    production_strategy, step, world_strategy, world_strategy_up_to, FlatParties, FunctionSpace,
    Op, Production, ProductionOp, Recursive, Step, MAX_PICKS, MAX_POPULATION, MAX_TRACE_OPS,
};

proptest! {
    /// A trace operand can select and mutate any clock appended beyond the
    /// generator's initially small population.
    #[test]
    fn population_selectors_reach_appended_members(
        (population, selector) in (9usize..MAX_TRACE_OPS, any::<Index>())
            .prop_filter("selector addresses an appended member", |(n, i)| i.index(*n) >= 8)
    ) {
        let mut clocks = vec![Clock::seed()];
        while clocks.len() < population {
            let child = clocks[0].fork();
            clocks.push(child);
        }

        let selected = selector.index(population);
        let before: Vec<Version> = clocks.iter().map(|clock| clock.version().clone()).collect();
        step::<Production>(&mut clocks, &Op::Tick(selector), &mut ChaChaRng::seed_from_u64(0));

        for (index, (old, clock)) in before.iter().zip(&clocks).enumerate() {
            if index == selected {
                prop_assert_ne!(old, clock.version());
            } else {
                prop_assert_eq!(old, clock.version());
            }
        }
    }

    /// A `forks` step or array split never grows a population past
    /// [`MAX_POPULATION`], nor at all once it is there, and a single fork
    /// always adds its child.
    ///
    /// Each step's population change is read from the flat model's
    /// replay, which keeps one party per live clock. The traces run to
    /// [`CAP_REACHING_OPS`] steps, so many of them reach the cap and clamp
    /// steps there; [`world_strategy`]'s shorter traces seldom do.
    #[test]
    fn multi_party_splits_respect_the_population_cap(ops in world_strategy_up_to(CAP_REACHING_OPS)) {
        let mut parties = vec![flat::Party::seed()];
        let mut rng = ChaChaRng::seed_from_u64(0);
        for op in &ops {
            let before = parties.len();
            step::<FlatParties>(&mut parties, op, &mut rng);
            match op {
                Op::Fork(_) => prop_assert_eq!(parties.len(), before + 1, "a fork adds one clock"),
                Op::Forks(..) | Op::Split(..) => prop_assert!(
                    parties.len() <= MAX_POPULATION.max(before),
                    "{:?} grew {} clocks to {}", op, before, parties.len()
                ),
                _ => {}
            }
        }
    }

    /// Every party and version a trace produces is at most as many levels
    /// deep as the levels its steps declare, summed over the steps so far,
    /// in the recursive oracle's trees and in the function space's
    /// resolution alike.
    ///
    /// This is the premise of the depth bound in the module doc, and so of
    /// the function-space grid derived from it, checked after every step.
    #[test]
    fn traces_deepen_no_more_than_their_levels(ops in world_strategy(), seed in any::<u64>()) {
        let mut trees = vec![tree::Clock::seed()];
        let mut functions = vec![FunctionClock::seed()];
        let mut rng = ChaChaRng::seed_from_u64(seed);
        let mut levels = 0;
        for op in &ops {
            levels += op.levels();
            step::<Recursive>(&mut trees, op, &mut rng);
            step::<FunctionSpace>(&mut functions, op, &mut rng);
            for clock in &trees {
                let (party, version) = clock.trees();
                let depth = function::id_depth(party).max(function::ev_depth(version));
                prop_assert!(
                    depth <= levels,
                    "tree depth {} after {} levels at {:?}", depth, levels, op
                );
            }
            for clock in &functions {
                let resolution = id_res(&clock.id).max(ev_res(&clock.ev));
                prop_assert!(
                    resolution <= levels,
                    "function-space resolution {} after {} levels at {:?}", resolution, levels, op
                );
            }
        }
        let cap = u32::try_from(MAX_TRACE_OPS).expect("the trace cap is small");
        prop_assert!(levels < cap, "the trace's levels stay under its cap");
    }
}

/// The op cap for traces that test the population cap, four times
/// [`MAX_TRACE_OPS`], long enough that many traces reach [`MAX_POPULATION`].
const CAP_REACHING_OPS: usize = 4 * MAX_TRACE_OPS;

/// The population size past which a `sync_all` or `join_all` with the most
/// picks leaves at least three clocks outside it, so the census's
/// large-population regime holds steps that act on part of a population.
const LARGE_POPULATION: usize = MAX_PICKS + 3;

/// The number of traces the census samples.
const CENSUS_CASES: usize = 2048;

/// The census's fixed strategy seed, so its counts are reproducible.
const CENSUS_SEED: u64 = 19;

/// Counts the production-side traces, of [`CENSUS_CASES`] sampled, holding a
/// tick count wider than `u64`.
fn wide_tick_traces() -> usize {
    let mut runner = TestRunner::new_with_rng(Config::default(), strategy_rng(CENSUS_SEED));
    let strategy = production_strategy();
    let past_u64 = Count::from(u64::MAX);
    (0..CENSUS_CASES)
        .filter(|_| {
            let trace = strategy.new_tree(&mut runner).expect("strategy").current();
            trace
                .iter()
                .any(|op| matches!(op, ProductionOp::WideTicks(_, count) if *count > past_u64))
        })
        .count()
}

/// What one sampled trace executes, read from its steps as resolved against
/// the live population.
#[derive(Default)]
struct Reach {
    /// A `forks` step took at least one child.
    forks_taking: bool,
    /// A `forks` step left at least one child untaken, for the residual to
    /// keep.
    forks_leaving: bool,
    /// An array split dealt two or more shares.
    split: bool,
    /// A `join_all` step absorbed two or more clocks.
    join_all: bool,
    /// A `sync_all` step reconciled the receiver with two or more clocks.
    sync_all: bool,
    /// A clock moved through its byte encoding.
    transmit: bool,
    /// A clock's party and version moved through their byte encodings.
    transmit_parts: bool,
    /// A clock absorbed another clock's version.
    absorb: bool,
    /// A clock absorbed two or more versions in one step.
    absorb_all: bool,
    /// The population exceeded [`LARGE_POPULATION`] after some step.
    large_population: bool,
}

impl Reach {
    /// Replays `ops` against the flat model's parties, recording what each
    /// resolved step executes and how large the population grows.
    fn of(ops: &[Op]) -> Reach {
        let mut reach = Reach::default();
        let mut parties = vec![flat::Party::seed()];
        let mut rng = ChaChaRng::seed_from_u64(0);
        for op in ops {
            match op.resolve(parties.len()) {
                Some(Step::Forks { count, taken, .. }) => {
                    reach.forks_taking |= taken >= 1;
                    reach.forks_leaving |= taken < usize::from(count);
                }
                Some(Step::Split { arity, .. }) => reach.split |= arity.shares() >= 2,
                Some(Step::JoinAll { others, .. }) => reach.join_all |= others.len() >= 2,
                Some(Step::SyncAll { others, .. }) => reach.sync_all |= others.len() >= 2,
                Some(Step::Transmit(_)) => reach.transmit = true,
                Some(Step::TransmitParts(_)) => reach.transmit_parts = true,
                Some(Step::Absorb(..)) => reach.absorb = true,
                Some(Step::AbsorbAll { sources, .. }) => reach.absorb_all |= sources.len() >= 2,
                _ => {}
            }
            step::<FlatParties>(&mut parties, op, &mut rng);
            reach.large_population |= parties.len() > LARGE_POPULATION;
        }
        reach
    }
}

/// The trace strategies keep enough generator mass on each regime their docs
/// claim for every default run of a trace consumer.
///
/// Each default run of a consumer of [`world_strategy`] or
/// [`production_strategy`] expects
/// [`REGIME_CASES_PER_RUN`](crate::testing::generators::REGIME_CASES_PER_RUN)
/// traces reaching each regime. The regimes are a `forks` step that takes children and one that leaves
/// some for the residual, an array split, a `join_all` of two or more
/// clocks, a `sync_all` with two or more others, each byte round trip, an
/// absorb and an `absorb_all` of two or more versions, a population past
/// [`LARGE_POPULATION`], and, in production-side traces, a tick count wider
/// than `u64`. The consumers run the default case count, so each floor
/// follows from that premise alone, and a regime whose mass falls below it
/// fails here whatever it last measured.
#[test]
fn world_strategy_keeps_mass_on_its_regimes() {
    if std::env::var_os("PROPTEST_CASES").is_none() {
        assert_eq!(
            Config::default().cases,
            DEFAULT_CASES,
            "the floors assume proptest's default case count"
        );
    }
    let mut runner = TestRunner::new_with_rng(Config::default(), strategy_rng(CENSUS_SEED));
    let strategy = world_strategy();
    let reaches: Vec<Reach> = (0..CENSUS_CASES)
        .map(|_| Reach::of(&strategy.new_tree(&mut runner).expect("strategy").current()))
        .collect();
    let count = |regime: fn(&Reach) -> bool| reaches.iter().filter(|reach| regime(reach)).count();
    let regimes = [
        ("a forks step taking children", count(|r| r.forks_taking)),
        (
            "a forks step leaving children untaken",
            count(|r| r.forks_leaving),
        ),
        ("an array split into two or more shares", count(|r| r.split)),
        ("a join_all of two or more clocks", count(|r| r.join_all)),
        ("a sync_all with two or more others", count(|r| r.sync_all)),
        ("a clock's byte round trip", count(|r| r.transmit)),
        (
            "a party and version's byte round trips",
            count(|r| r.transmit_parts),
        ),
        ("an absorb", count(|r| r.absorb)),
        (
            "an absorb_all of two or more versions",
            count(|r| r.absorb_all),
        ),
        (
            "a population past the large-population size",
            count(|r| r.large_population),
        ),
        ("a tick count wider than u64", wide_tick_traces()),
    ];
    let floor = regime_floor(CENSUS_CASES, 1);
    for (regime, count) in &regimes {
        eprintln!("census: `{regime}` in {count} of {CENSUS_CASES} traces (floor {floor})");
    }
    for (regime, count) in regimes {
        assert!(
            count >= floor,
            "generator mass on `{regime}` fell to {count} of {CENSUS_CASES} sampled traces \
             (floor {floor}): an op kind has gone dead or the population cap or depth budget \
             starves it"
        );
    }
}
