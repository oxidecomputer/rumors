//! Liveness pins for the window-budget sweep dimension
//! (`common::window`): proof that the swept population cannot silently
//! degenerate to floor-everywhere.
//!
//! The sweep's value rests on two facts, each pinned here: the generator
//! actually emits every regime (floor, tight budget, default — and
//! budgets on both sides of the width-granting threshold), and a
//! non-floor choice at suite-scale content actually reaches the session
//! as a granted window wider than one slot. Either fact could rot
//! silently — a strategy edit could drop an arm, or a plumbing change
//! could quietly stop applying the choice — and every swept suite would
//! keep passing while testing the floor alone.

mod common;

use proptest::strategy::{Strategy, ValueTree};
use proptest::test_runner::TestRunner;
use rumors::Peer;

use crate::common::window::{WindowChoice, arb_window_choice};
use crate::common::wire::{LINK_BUF, block_on, bootstrap_fork_with_window_async};

/// Values each endpoint originates in the widening pin's session: enough
/// that the default budget's population clamp sits well above one slot,
/// at a size the schedule suites' generators routinely reach.
const DIVERGENT_VALUES_PER_SIDE: u64 = 24;

/// A non-floor window choice must reach the wire as a granted window
/// wider than one slot, at content sizes the swept suites reach.
///
/// The session is asymmetric (floor against default, the sweep's
/// general case), and each endpoint's grant must stay its own: the
/// floor side at one slot while the default side widens.
#[test]
fn non_floor_choice_widens_the_granted_window() {
    let (floor_granted, default_granted) = block_on(async {
        let seed = WindowChoice::Default
            .apply(Peer::<u64>::seed())
            .into_rumors();
        let floor_side = bootstrap_fork_with_window_async(&seed, WindowChoice::Floor).await;
        let default_side = bootstrap_fork_with_window_async(&seed, WindowChoice::Default).await;
        {
            let mut batch = floor_side.batch();
            for v in 0..DIVERGENT_VALUES_PER_SIDE {
                batch.send(v);
            }
        }
        {
            let mut batch = default_side.batch();
            for v in 0..DIVERGENT_VALUES_PER_SIDE {
                batch.send(1_000_000 + v);
            }
        }
        let (mut link_f, mut link_d) = rumors::link::memory_with_capacity(LINK_BUF);
        let (out_f, out_d) = tokio::join!(
            floor_side.gossip(&mut link_f),
            default_side.gossip(&mut link_d),
        );
        (
            out_f.expect("floor endpoint session").stats.window_granted,
            out_d
                .expect("default endpoint session")
                .stats
                .window_granted,
        )
    });
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

/// The sweep generator's population covers every regime.
///
/// Sampled under proptest's deterministic runner, [`arb_window_choice`]
/// emits the floor, the default, and budgets spanning both sides of the
/// width-granting threshold (budgets at or below 64 KiB resolve back to
/// one slot at suite scale; budgets at or above 1 MiB grant the
/// population clamp).
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
        budgets.iter().any(|&b| b <= 64 << 10),
        "the budget arm must reach floor-equivalent budgets, exercising \
         the budget resolution path at one slot"
    );
    assert!(
        budgets.iter().any(|&b| b >= 1 << 20),
        "the budget arm must reach width-granting budgets"
    );
}
