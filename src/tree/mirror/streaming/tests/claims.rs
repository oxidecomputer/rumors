//! The invitation share's claims over whole wire sessions: every depth's
//! outstanding claims return to zero once a session completes.
//!
//! A question the share admits against the budget claims its reply's
//! predicted need at its depth, and the stage that processes the reply
//! withdraws exactly that need ([`Ledger::processed`]). A claim that was
//! never withdrawn would stay in its depth's sum and hold later questions
//! back for room nothing occupies; a debug assertion catches only the
//! opposite mistake, a withdrawal of more than was claimed. These sessions
//! read each ledger's claims once the session has completed, where every
//! claim must have been withdrawn.
//!
//! [`Ledger::processed`]: crate::tree::mirror::streaming::ledger::Ledger::processed

use proptest::prelude::*;

use super::grid::forks;
use super::join_oracle;
use crate::testing::property::SESSION_DIVISOR;
use crate::tree::mirror::streaming::testing::session::{AMPLE, over_pipe_keeping_ledgers};
use crate::tree::typed::hash::PATH_LEN;

/// An unlimited budget.
const UNLIMITED: usize = usize::MAX;

/// The most messages the two sides of a case share.
const MAX_SHARED: usize = 2_000;

/// The most messages each side of a case holds that the other lacks.
const MAX_OWN: usize = 300;

/// The ledger budgets a side of a case draws.
///
/// A tight one, from 1 KiB up to 256 KiB, makes questions wait for room and
/// for the share and be reconsidered as replies finish; with no limit,
/// every check the share makes admits its question, which claims.
fn budget() -> impl Strategy<Value = usize> {
    prop_oneof![
        2 => (1usize << 10)..=(256 << 10),
        1 => Just(UNLIMITED),
    ]
}

crate::divided_proptest! {
    #![divisor(SESSION_DIVISOR)]

    /// Once a session completes, every depth's outstanding claims are zero
    /// on both sides: every question the share admitted against the budget
    /// withdrew its claim when its reply was processed.
    ///
    /// The pairs are forks sharing up to [`MAX_SHARED`] messages, each with
    /// up to [`MAX_OWN`] of its own, so they dispute at several depths; each
    /// side draws its budget ([`budget`]), and the pipe is one byte wide, a
    /// few frame headers wide, or ample, which reorders when replies arrive
    /// and so when claims are withdrawn. Each session must complete with
    /// both sides holding the join.
    ///
    /// The zeros must judge claims that existed, so each case requires
    /// some share check to have weighed a question on one side or the
    /// other: a share peak above zero. A question that passes the check
    /// claims, as every checked question at an unlimited budget does, and
    /// [`budget`] often draws that budget.
    #[test]
    fn every_depths_claims_return_to_zero(
        shared in 0..=MAX_SHARED,
        own in 1..=MAX_OWN,
        budgets in [budget(), budget()],
        pipe in prop::sample::select(vec![1usize, 64, AMPLE]),
    ) {
        let pair = forks(shared, own);
        let expected = join_oracle(pair.0.clone(), pair.1.clone());
        let (outcome, ledgers) = over_pipe_keeping_ledgers::<()>(pair, budgets, pipe);
        let (a, b) = outcome.unwrap_or_else(|stopped| {
            panic!("the session at budgets {budgets:?} over a {pipe}-byte pipe stopped: {stopped:?}")
        });
        prop_assert_eq!(a.expect("the first endpoint's session completes"), expected.clone());
        prop_assert_eq!(b.expect("the second endpoint's session completes"), expected);
        prop_assert!(
            ledgers.iter().any(|ledger| ledger.share_peak() > 0),
            "no share check weighed a question at budgets {budgets:?}, so the zeros below \
             judge no claim",
        );
        for (side, ledger) in ledgers.iter().enumerate() {
            for depth in 0..=PATH_LEN {
                prop_assert_eq!(
                    ledger.claimed(depth),
                    0,
                    "side {}'s claims at depth {} outlive the session at budgets {:?}",
                    side,
                    depth,
                    budgets,
                );
            }
        }
    }
}
