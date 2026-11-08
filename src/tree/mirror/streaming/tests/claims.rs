//! The invitation share's claims over whole wire sessions: every depth's
//! outstanding claims return to zero once a session completes.
//!
//! A question the share admits against the budget claims its reply's predicted
//! need at its depth, and the stage that processes the reply withdraws exactly
//! that need ([`Ledger::processed`]). A claim that was never withdrawn would
//! stay in its depth's sum and hold later questions back for room nothing
//! occupies; a debug assertion catches only the opposite mistake, a withdrawal
//! of more than was claimed. These sessions read each ledger's claims once the
//! session has completed, where every claim must have been withdrawn.
//!
//! [`Ledger::processed`]: crate::tree::mirror::streaming::ledger::Ledger::processed

use proptest::prelude::*;

use super::grid::forks;
use super::join_oracle;
use crate::testing::LARGEST_GRANT;
use crate::testing::property::SESSION_DIVISOR;
use crate::tree::Root;
use crate::tree::mirror::streaming::ledger::Ledger;
use crate::tree::mirror::streaming::testing::session::{
    AMPLE, SessionLedger, over_pipe_keeping_ledgers,
};
use crate::tree::typed::hash::PATH_LEN;

/// An unlimited budget.
const UNLIMITED: usize = usize::MAX;

/// The most messages the two sides of a case share.
const MAX_SHARED: usize = 2_000;

/// The most messages each side of a case holds that the other lacks.
const MAX_OWN: usize = 300;

/// The ledger budgets a side of a case draws.
///
/// A tight one, from 1 KiB up to 256 KiB, makes questions wait for room and for
/// the share and be reconsidered as replies finish; with no limit, every check
/// the share makes admits its question, which claims.
fn budget() -> impl Strategy<Value = usize> {
    prop_oneof![
        2 => (1usize << 10)..=(256 << 10),
        1 => Just(UNLIMITED),
    ]
}

/// Reconcile `pair` at `budgets` over a `pipe`-byte pipe, check its join, share
/// and claims, and return both sides' ledgers, in the pair's order.
///
/// Both sides must hold the join; each side that admitted some question under
/// the share without exemption must show a share peak above zero, since the
/// share weighed every such question before granting it; and every depth's
/// claims must be zero on both sides once the session completes.
///
/// The converse does not hold: a question the share weighed and refused raises
/// the peak, and may later be admitted exempt, claiming nothing
/// ([`claims_return_to_zero_where_the_share_refuses_every_check`]).
///
/// # Panics
///
/// Panics if any of those checks fails.
fn check_session(pair: (Root, Root), budgets: [usize; 2], pipe: usize) -> [Ledger; 2] {
    let expected = join_oracle(pair.0.clone(), pair.1.clone());
    let (outcome, ledgers) =
        over_pipe_keeping_ledgers::<()>(pair, budgets.map(SessionLedger::Budget), pipe);
    let (a, b) = outcome.unwrap_or_else(|stopped| {
        panic!("the session at budgets {budgets:?} over a {pipe}-byte pipe stopped: {stopped:?}")
    });
    assert_eq!(a.expect("the first endpoint's session completes"), expected);
    assert_eq!(
        b.expect("the second endpoint's session completes"),
        expected
    );
    for (side, ledger) in ledgers.iter().enumerate() {
        assert!(
            ledger.invited() == 0 || ledger.share_peak() > 0,
            "side {side} admitted {} questions under the share without exemption, but its share \
             peak is zero, at budgets {budgets:?}",
            ledger.invited(),
        );
        for depth in 0..=PATH_LEN {
            assert_eq!(
                ledger.claimed(depth),
                0,
                "side {side}'s claims at depth {depth} outlive the session at budgets {budgets:?}",
            );
        }
    }
    ledgers
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
    /// A question within its kind and depth's bound is exempt and never
    /// meets the share, so a small pair can complete with no claim made.
    /// Each case therefore checks the share's peak against its claims
    /// instead of requiring a claim: a side that admitted some question
    /// under the share without exemption has a share peak above zero
    /// ([`check_session`]).
    /// [`claims_return_to_zero_where_the_share_is_exercised`] holds the
    /// suite to sessions in which questions claim.
    #[test]
    fn every_depths_claims_return_to_zero(
        shared in 0..=MAX_SHARED,
        own in 1..=MAX_OWN,
        budgets in [budget(), budget()],
        pipe in prop::sample::select(vec![1usize, 64, AMPLE]),
    ) {
        check_session(forks(shared, own), budgets, pipe);
    }
}

/// Messages each side of the floor case holds that the other lacks: enough that
/// some questions overflow their kinds and depths' bounds and meet the share.
const FLOOR_OWN: usize = 10_000;

/// In a disjoint pair of [`FLOOR_OWN`] messages a side, at an unlimited budget
/// and at twice the largest grant, some question meets the share and claims,
/// and every depth's claims still return to zero.
///
/// This is the suite's floor: the property above passes vacuously on sessions
/// in which no question claims, and here some question must be admitted under
/// the share, and so claim, at each budget.
#[test]
fn claims_return_to_zero_where_the_share_is_exercised() {
    for budget in [UNLIMITED, 2 * LARGEST_GRANT] {
        assert!(
            check_session(forks(0, FLOOR_OWN), [budget; 2], AMPLE)
                .iter()
                .any(|ledger| ledger.invited() > 0),
            "no question was admitted under the share at a budget of {budget} B",
        );
    }
}

/// The ledger budget both sides of
/// [`claims_return_to_zero_where_the_share_refuses_every_check`] run at: so
/// small that, in that pair, one side's share refuses every question it weighs.
const REFUSING_BUDGET: usize = 1 << 10;

/// In a disjoint pair of [`FLOOR_OWN`] messages a side at ledger budgets of
/// [`REFUSING_BUDGET`], one side's share weighs questions and admits none of
/// them under the share, and every depth's claims still return to zero.
///
/// A question whose excess fits the budget but whose share check does not
/// waits, and is admitted exempt once its kind and depth hold little enough, so
/// its check raised the share peak while it claimed nothing. The test requires
/// that state on some side, a share peak above zero with no question admitted
/// under the share, so the session judges it.
#[test]
fn claims_return_to_zero_where_the_share_refuses_every_check() {
    let ledgers = check_session(forks(0, FLOOR_OWN), [REFUSING_BUDGET; 2], AMPLE);
    assert!(
        ledgers
            .iter()
            .any(|ledger| ledger.share_peak() > 0 && ledger.invited() == 0),
        "no side's share weighed a question it then admitted only exempt: {:?}",
        ledgers
            .iter()
            .map(|ledger| (ledger.share_peak(), ledger.invited()))
            .collect::<Vec<_>>(),
    );
}
