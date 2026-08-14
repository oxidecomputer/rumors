//! Liveness pins for the window-budget sweep dimension
//! (`common::window`): proof that the swept population cannot silently
//! degenerate to floor-everywhere.
//!
//! The sweep's value rests on three facts, each pinned here: the
//! generator actually emits every arm (floor, budget, default, with
//! budgets reaching both endpoints of its exponent range), a non-floor
//! choice at suite-scale content actually reaches the session as a
//! granted window wider than one slot, and an explicit budget actually
//! reaches the session's solve (a `Budget` no-op would leave every
//! suite green while sweeping nothing). Any of these could rot
//! silently — a strategy edit could drop an arm, or a plumbing change
//! could quietly stop applying the choice.

mod common;

use proptest::strategy::{Strategy, ValueTree};
use proptest::test_runner::TestRunner;
use rumors::Gossiped;

use crate::common::window::{
    MAX_BUDGET_EXPONENT, MIN_BUDGET_EXPONENT, WindowChoice, arb_window_choice,
};
use crate::common::wire::{block_on, divergent_pair, gossip_pair_async};

/// Values each endpoint originates in the widening pins' sessions:
/// enough that a wide window's population clamp sits well above one
/// slot, at a size the schedule suites' generators routinely reach.
const DIVERGENT_VALUES_PER_SIDE: u64 = 24;

/// One fully-divergent session between endpoints at the two given window
/// choices, returning each side's granted window width.
fn granted_widths(window_a: WindowChoice, window_b: WindowChoice) -> (u64, u64) {
    block_on(async {
        let (a, b) = divergent_pair(DIVERGENT_VALUES_PER_SIDE, window_a, window_b).await;
        let (a_report, b_report): (Gossiped, Gossiped) = gossip_pair_async(&a, &b).await;
        (a_report.stats.window_granted, b_report.stats.window_granted)
    })
}

/// A non-floor window choice must reach the wire as a granted window
/// wider than one slot, at content sizes the swept suites reach.
///
/// The session is asymmetric (floor against default, the sweep's
/// general case), and each endpoint's grant must stay its own: the
/// floor side at one slot while the default side widens.
#[test]
fn non_floor_choice_widens_the_granted_window() {
    let (floor_granted, default_granted) =
        granted_widths(WindowChoice::Floor, WindowChoice::Default);
    assert_eq!(
        floor_granted, 1,
        "the floor endpoint of an asymmetric session must stay at the \
         one-slot serialization floor"
    );
    assert!(
        default_granted > 1,
        "the default endpoint granted a window of {default_granted} slots: \
         a non-floor choice failed to widen the window at suite-scale \
         content, so the sweep is degenerating to floor-everywhere"
    );
}

/// Explicit budgets reach the session's solve, and the generator's
/// budget range provably straddles the width-granting threshold.
///
/// At the range's bottom endpoint (`2^MIN_BUDGET_EXPONENT` bytes) the
/// granted window is exactly one slot — the budget-resolution path's
/// floor-equivalent regime — while at its top endpoint
/// (`2^MAX_BUDGET_EXPONENT` bytes) it is wider than one. A `Budget`
/// no-op regression would collapse the two readings.
#[test]
fn budget_endpoints_straddle_the_width_threshold() {
    let (tight_granted, wide_granted) = granted_widths(
        WindowChoice::Budget(1 << MIN_BUDGET_EXPONENT),
        WindowChoice::Budget(1 << MAX_BUDGET_EXPONENT),
    );
    assert_eq!(
        tight_granted, 1,
        "the minimum generated budget must resolve to the one-slot floor \
         at suite-scale content"
    );
    assert!(
        wide_granted > 1,
        "the maximum generated budget granted {wide_granted} slot(s): \
         explicit budgets are not reaching the session's window solve"
    );
}

/// The sweep generator's population covers every arm.
///
/// Sampled under proptest's deterministic runner, [`arb_window_choice`]
/// emits the floor, the default, and budgets from both endpoint decades
/// of its exponent range — the regimes whose session effects
/// [`budget_endpoints_straddle_the_width_threshold`] pins.
#[test]
fn window_choice_population_covers_every_regime() {
    let mut runner = TestRunner::deterministic();
    let strategy = arb_window_choice();
    let mut floors = 0usize;
    let mut defaults = 0usize;
    let mut budgets: Vec<usize> = Vec::new();
    for _ in 0..256 {
        let choice = strategy
            .new_tree(&mut runner)
            .expect("window-choice strategy always generates")
            .current();
        match choice {
            WindowChoice::Floor => floors += 1,
            WindowChoice::Default => defaults += 1,
            WindowChoice::Budget(bytes) => budgets.push(bytes),
        }
    }
    assert!(floors > 0, "the population must keep the floor exercised");
    assert!(defaults > 0, "the population must include the default");
    assert!(
        budgets.iter().any(|&b| b < 1 << (MIN_BUDGET_EXPONENT + 1)),
        "the budget arm must reach its bottom decade (floor-equivalent \
         budgets, exercising the budget-resolution path at one slot)"
    );
    assert!(
        budgets.iter().any(|&b| b >= 1 << MAX_BUDGET_EXPONENT),
        "the budget arm must reach its top decade (width-granting budgets)"
    );
}
