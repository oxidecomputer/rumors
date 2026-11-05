//! The closed-world liveness grid: whole wire sessions across replica
//! sizes, overlaps, pipe widths, and pairs of budgets.
//!
//! Each session runs under the closed-world poller, which reports a stall
//! whenever every task waits with no wake arranged.
//!
//! Admission's liveness argument uses no budget above zero and never
//! relates the two sides' budgets, so the grid crosses zero, tiny,
//! mismatched, and unlimited budgets with every pipe width, from a pipe that
//! hands over one byte at a time to one that holds whole frames. Every
//! session must complete with both sides holding the join.

use super::join_oracle;
use super::pipe::placed_over_pipe;
use crate::message::Message;
use crate::tree::arb::{nth_party, roots_at_depth};
use crate::tree::mirror::streaming::testing::session::{completed, over_pipe};
use crate::tree::mirror::streaming::window::WindowConfig;
use crate::tree::mirror::streaming::{Backend, Local, materialized::floor_allowance};
use crate::tree::{Action, Root, Tree};

/// An unlimited budget.
const UNLIMITED: usize = usize::MAX;

/// The budget pairs every grid cell runs.
///
/// Both sides at zero, where every unit takes a floor slot; one side at
/// zero against an unlimited peer, in both orders; tiny and mismatched
/// budgets, which admit a few units beside the floor; and both unlimited.
const BUDGETS: [[usize; 2]; 6] = [
    [0, 0],
    [0, UNLIMITED],
    [UNLIMITED, 0],
    [1 << 10, 1 << 10],
    [256 << 10, 64 << 10],
    [UNLIMITED, UNLIMITED],
];

/// Bytes each direction of a cell's pipe buffers: one, so every byte
/// crosses in its own handoff; a few frame headers' worth; and several
/// whole frames.
const PIPES: [usize; 3] = [1, 64, 8 << 10];

/// Two forks of one network: both hold `shared` messages, and each then
/// sends `own` messages the other lacks.
fn forks(shared: usize, own: usize) -> (Root, Root) {
    let mut base = Tree::<()>::new();
    base.act(
        &nth_party(0),
        (0..shared).map(|_| Action::Insert(Message::new(()))),
    );
    let fork = |index: usize| {
        let mut tree = base.clone();
        tree.act(
            &nth_party(index),
            (0..own).map(|_| Action::Insert(Message::new(()))),
        );
        tree.root
    };
    (fork(1), fork(2))
}

/// Run one cell, `pair` at `budgets` over a pipe buffering `pipe` bytes,
/// and require both sides to complete holding the join.
///
/// Each budget is what that side's ledger admits against beside its floor
/// slots: the session's memory budget is the budget and the floor
/// allowance together, since a session sets the allowance aside first.
fn assert_cell(pair: (Root, Root), budgets: [usize; 2], pipe: usize, placed: bool, cell: &str) {
    let expected = join_oracle(pair.0.clone(), pair.1.clone());
    let allowance = floor_allowance::<<Local as Backend>::Erased, <Local as Backend>::Error>();
    let windows = budgets.map(|budget| WindowConfig::Budget(budget.saturating_add(allowance)));
    let outcome = if placed {
        placed_over_pipe(pair, windows, pipe)
    } else {
        over_pipe::<()>(pair, windows, pipe)
    };
    let (a, b) = completed(outcome.unwrap_or_else(|stopped| {
        panic!("{cell} at budgets {budgets:?} over a {pipe}-byte pipe stopped: {stopped:?}")
    }));
    assert_eq!(
        a, expected,
        "{cell} at budgets {budgets:?} over a {pipe}-byte pipe"
    );
    assert_eq!(
        b, expected,
        "{cell} at budgets {budgets:?} over a {pipe}-byte pipe"
    );
}

/// Disjoint forks of 300 messages a side, and forks sharing 90% of 3,000,
/// complete over every pipe at every budget pair.
#[test]
fn small_forks_complete_at_every_budget_over_every_pipe() {
    for (shared, own) in [(0, 300), (2_700, 300)] {
        let pair = forks(shared, own);
        for budgets in BUDGETS {
            for pipe in PIPES {
                assert_cell(
                    pair.clone(),
                    budgets,
                    pipe,
                    false,
                    &format!("{shared}+{own}"),
                );
            }
        }
    }
}

/// Disjoint forks of 3,000 messages a side, and forks sharing 90% of
/// 30,000, complete at every budget pair over the two wider pipes.
#[test]
fn larger_forks_complete_at_every_budget() {
    for (shared, own) in [(0, 3_000), (27_000, 3_000)] {
        let pair = forks(shared, own);
        for budgets in BUDGETS {
            for pipe in [PIPES[1], PIPES[2]] {
                assert_cell(
                    pair.clone(),
                    budgets,
                    pipe,
                    false,
                    &format!("{shared}+{own}"),
                );
            }
        }
    }
}

/// Disjoint forks of 30,000 messages a side complete at every budget pair
/// over a pipe of whole frames.
#[test]
fn large_forks_complete_at_every_budget() {
    let pair = forks(0, 30_000);
    for budgets in BUDGETS {
        assert_cell(pair.clone(), budgets, PIPES[2], false, "0+30000");
    }
}

/// Forks placed below a shared prefix of every depth complete at every
/// budget pair over a one-byte pipe: their disputes reach the depth they
/// are placed at, so the floor slots of many depths are in use at once.
#[test]
fn placed_forks_complete_at_every_depth_and_budget() {
    let (a, b) = forks(40, 40);
    for depth in [0, 1, 2, 7, 15, 30] {
        let [a, b] = roots_at_depth([a.clone(), b.clone()], depth);
        for budgets in BUDGETS {
            assert_cell(
                (a.clone(), b.clone()),
                budgets,
                PIPES[0],
                true,
                &format!("depth {depth}"),
            );
        }
    }
}
