//! The closed-world liveness grid: whole wire sessions across replica
//! sizes, overlaps, pipe widths, and pairs of budgets.
//!
//! Each session runs under the closed-world poller, which reports a stall
//! whenever every task waits with no wake arranged.
//!
//! Admission's liveness argument uses no budget above zero and never
//! relates the two sides' budgets, so the grid crosses zero, tiny,
//! mismatched, minimum, and unlimited budgets with every pipe width, from a
//! pipe that hands over one byte at a time to one that holds whole frames.
//! Every session must complete with both sides holding the join.

use super::join_oracle;
use super::pipe::placed_over_pipe;
use crate::message::Message;
use crate::testing::LARGEST_GRANT;
use crate::tree::arb::{nth_party, roots_at_depth};
use crate::tree::mirror::streaming::testing::session::{SessionLedger, completed, over_pipe};
use crate::tree::{Action, Root, Tree};

/// An unlimited budget.
const UNLIMITED: SessionLedger = SessionLedger::Budget(usize::MAX);

/// The serial configuration: one unit of each kind and depth at a time.
const SERIAL: SessionLedger = SessionLedger::Serial;

/// A budget of `bytes` beside the walk's floor slots.
const fn budget(bytes: usize) -> SessionLedger {
    SessionLedger::Budget(bytes)
}

/// The ledger pairs every grid cell runs.
///
/// Both sides serial, where every unit takes a floor slot; one side serial
/// against an unlimited peer, in both orders; tiny and mismatched budgets,
/// which admit a few units beside the floor; both at the largest grant, the
/// least ledger budget the public API leaves a session, which disjoint
/// forks of 30,000 messages a side outgrow; and both unlimited.
const BUDGETS: [[SessionLedger; 2]; 7] = [
    [SERIAL, SERIAL],
    [SERIAL, UNLIMITED],
    [UNLIMITED, SERIAL],
    [budget(1 << 10), budget(1 << 10)],
    [budget(256 << 10), budget(64 << 10)],
    [budget(LARGEST_GRANT), budget(LARGEST_GRANT)],
    [UNLIMITED, UNLIMITED],
];

/// Bytes each direction of a cell's pipe buffers: one, so every byte
/// crosses in its own handoff; a few frame headers' worth; and several
/// whole frames.
const PIPES: [usize; 3] = [1, 64, 8 << 10];

/// Two forks of one network: both hold `shared` messages, and each then
/// sends `own` messages the other lacks.
pub(super) fn forks(shared: usize, own: usize) -> (Root, Root) {
    let mut base = Tree::<()>::default();
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

/// Two forks of one network that both hold `shared` messages, where the
/// first then redacts every shared message and each sends `own` messages
/// the other lacks.
///
/// The second's subtrees then mix leaves the first's version knows, which
/// it must not resend, with leaves the first lacks. Where the first holds
/// nothing, the second supplies a subtree through the deletion filter,
/// which drops every leaf the first knows.
fn redacting_forks(shared: usize, own: usize) -> (Root, Root) {
    let mut base = Tree::<()>::default();
    base.act(
        &nth_party(0),
        (0..shared).map(|_| Action::Insert(Message::new(()))),
    );
    let versions: Vec<_> = base.iter().map(|(version, _)| version.clone()).collect();
    let mut redacting = base.clone();
    redacting.act(&nth_party(1), versions.into_iter().map(Action::Forget));
    redacting.act(
        &nth_party(1),
        (0..own).map(|_| Action::Insert(Message::new(()))),
    );
    let mut keeping = base;
    keeping.act(
        &nth_party(2),
        (0..own).map(|_| Action::Insert(Message::new(()))),
    );
    (redacting.root, keeping.root)
}

/// Run one cell, `pair` at `budgets` over a pipe buffering `pipe` bytes,
/// and require both sides to complete holding the join.
fn assert_cell(
    pair: (Root, Root),
    budgets: [SessionLedger; 2],
    pipe: usize,
    placed: bool,
    cell: &str,
) {
    let expected = join_oracle(pair.0.clone(), pair.1.clone());
    let outcome = if placed {
        placed_over_pipe(pair, budgets, pipe)
    } else {
        over_pipe::<()>(pair, budgets, pipe)
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
/// are placed at, so units at many depths hold their kinds and depths at
/// once.
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

/// Forks where one side redacted every shared message complete at every
/// budget pair over every pipe, both sides holding the join.
///
/// The forks hold 300 shared messages beside 30 and 300 of each side's own,
/// and 3,000 shared beside 300 over the two wider pipes. The keeping side's
/// supplies pass through the deletion filter over mixed subtrees at every
/// budget and pipe width.
#[test]
fn redacting_forks_complete_at_every_budget_over_every_pipe() {
    for (shared, own, pipes) in [
        (300, 30, &PIPES[..]),
        (300, 300, &PIPES[..]),
        (3_000, 300, &PIPES[1..]),
    ] {
        let pair = redacting_forks(shared, own);
        for budgets in BUDGETS {
            for &pipe in pipes {
                assert_cell(
                    pair.clone(),
                    budgets,
                    pipe,
                    false,
                    &format!("{shared} redacted+{own}"),
                );
            }
        }
    }
}
