//! A budget at the session's no-cost point costs it nothing: the session
//! runs exactly as it would with no budget at all.
//!
//! # The claim
//!
//! Each side's ledger admits the work its session builds in units. A unit
//! is exempt, admitted whatever the budget, when its kind and depth (its
//! kind of unit and the depth of the scope it concerns) hold nothing, or
//! when it fits beside what they hold within their bound. Otherwise it is
//! admitted against the budget if the ledger's *excess*, the bytes its
//! kinds and depths hold beyond their bounds, with the unit's added, fits
//! within it. A question under the invitation share must also keep its
//! depth's claims within the budget: the excess the depth's questions
//! hold, with its own grant, beside the predicted needs of their unfinished
//! replies and its own, scaled by the depth's pace,
//! `I_d + c + α · (N_d + n) ≤ B`. Here `B` is the ledger's budget: the
//! peer's memory budget less the floor allowance it sets aside for what
//! the kinds and depths hold within their bounds and the session allowance
//! it sets aside for the session's fixed state and wire pools, so the
//! session at the no-cost point runs on that point beside the two
//! allowances.
//!
//! Let `Λ` be the largest excess either side's ledger holds when the budget
//! is unlimited, and the *share peak* the largest sum any share check of
//! that session weighs. The session's *no-cost point* is the larger of the
//! two. At a budget at least that, every admission the unlimited session
//! makes against the budget fits within it, every share check it makes
//! passes, and no exemption depends on the budget, so no unit waits that
//! did not wait there. Every admission
//! decision, and with them the session, is then the unlimited session's,
//! event for event: an induction along the unlimited session's schedule.
//! The share peak exceeds `Λ` wherever a depth asks more questions at once
//! than the budget could also process their replies with; between the two,
//! the share holds questions back, and identity is promised only at the
//! no-cost point.
//!
//! # The measurement
//!
//! For each pair, of several sizes and overlaps, on a LAN and on a
//! long-haul link, the suite runs the session twice on the link model,
//! whose clock counts byte-times and advances only when both peers wait,
//! so a session's timeline is an exact function of its pair, link, and
//! budgets:
//!
//! - the oracle, with an unlimited budget, which reads the no-cost point
//!   from the ledgers and must have kept every unit from waiting;
//! - the session with both budgets at the no-cost point, which must carry
//!   the oracle's every segment at the oracle's every instant, and keep
//!   every unit from waiting, as the oracle does.
//!
//! The comparison is an identity, with no tolerance. The segments alone
//! would not show every admission the budget refused: computation takes no
//! link time, so a unit that waits and is admitted within the instant it
//! asked moves no segment. The count of units that waited shows it.

use std::array;

use rumors_testkit::bench::link::LinkConfig;

use rumors_testkit::bench::session::{
    self as fixture, Case, LAN_BDP, LONG_HAUL_BDP, Session, Shape,
};
use rumors_testkit::common::budget::BudgetChoice;

/// The oracle's budget: unlimited.
const ORACLE_BUDGET: usize = usize::MAX;

/// Messages a side in each pair the suite runs.
const SIZES: [u64; 2] = [1_000, 10_000];

/// The share of each side's messages the other side also holds, in percent.
const OVERLAPS: [u64; 2] = [0, 90];

/// The links each pair runs on, by bandwidth-delay product.
const LINKS: [u64; 2] = [LAN_BDP, LONG_HAUL_BDP];

/// Every pair and link the suite runs, the pair at `index` on a budget of
/// `budget(index)`.
fn cases(
    budget: impl Fn(usize) -> BudgetChoice,
) -> [Case; SIZES.len() * OVERLAPS.len() * LINKS.len()] {
    array::from_fn(|index| {
        let size = SIZES[index / (OVERLAPS.len() * LINKS.len())];
        let overlap = OVERLAPS[index / LINKS.len() % OVERLAPS.len()];
        let bdp = LINKS[index % LINKS.len()];
        Case {
            shape: Shape::overlapping(size, overlap),
            budget: budget(index),
            link: LinkConfig::with_bdp(bdp),
        }
    })
}

/// The questions under the invitation share: those about scopes below the
/// root and above the leaves. The opening question, about the root, and
/// the leaf requests are admitted outside it.
const SHARED_DEPTHS: std::ops::Range<usize> = 1..32;

/// Whether some unit of `session` was admitted against the budget, not
/// exempt, as the ledgers' admission counts report.
fn admitted_against_the_budget(session: &Session) -> bool {
    session.admitted.ledgers.iter().any(|ledger| {
        let mut grants = ledger.questions.iter().chain(&ledger.answers);
        grants.any(|grants| grants.units > grants.exempt)
    })
}

/// Whether some question under the invitation share in `session` was not
/// exempt: at an unlimited budget, each such question is weighed by its
/// depth's share check.
fn invited_against_the_budget(session: &Session) -> bool {
    session.admitted.ledgers.iter().any(|ledger| {
        ledger.questions[SHARED_DEPTHS]
            .iter()
            .any(|grants| grants.units > grants.exempt)
    })
}

/// Prints one pair's oracle beside its session at the no-cost point.
fn report(oracle: &Session, at_peak: &Session) {
    let Case { shape, link, .. } = oracle.case;
    let messages = shape.shared + shape.a;
    let peak = oracle.admitted.budget_peak();
    let no_cost = oracle.admitted.no_cost_point();
    println!(
        "{messages} messages a side, {} shared, BDP {} B: Λ = {peak} B ({:.1} B per \
         message), no-cost point {no_cost} B beside the allowances, {} \
         units waited there; {} byte-times unlimited, {} there",
        shape.shared,
        link.bdp(),
        peak as f64 / messages as f64,
        at_peak.admitted.waited(),
        oracle.duration(),
        at_peak.duration(),
    );
}

/// A budget at the session's no-cost point leaves every session
/// unchanged, event for event.
///
/// Across sizes, disjoint and mostly shared pairs, and a LAN and a
/// long-haul link, a session with both ledgers admitting work against the
/// larger of `Λ` and the share peak, beside the floor and session allowances
/// every budget sets aside, carries the same segments at the same instants
/// as the same session with an unlimited budget, and like it keeps every
/// unit from waiting.
///
/// Neither term of the no-cost point may read zero because its counter
/// stopped counting. Each is zero exactly when every unit it covers was
/// exempt, which a small, mostly shared pair can be: `Λ` is positive
/// exactly when some unit was admitted against the budget, and the share
/// peak exactly when some question under the share was not exempt, as the
/// ledgers' admission counts report. And some pair must have both
/// positive, so that the suite judges sessions the no-cost point
/// constrains.
#[test]
fn a_budget_of_the_no_cost_point_is_identical_to_no_budget() {
    let oracles = fixture::run_all(cases(|_| BudgetChoice::Bytes(ORACLE_BUDGET)));
    let peaks = oracles
        .each_ref()
        .map(|oracle| oracle.admitted.no_cost_point());
    let at_peaks = fixture::run_all(cases(|index| BudgetChoice::Ledger(peaks[index])));
    for (oracle, at_peak) in oracles.iter().zip(&at_peaks) {
        report(oracle, at_peak);
    }
    assert!(
        oracles
            .iter()
            .any(|oracle| oracle.admitted.budget_peak() > 0 && oracle.admitted.share_peak() > 0),
        "some pair must admit work against the budget and check a question against its \
         depth's claims, or the suite judges no session the no-cost point constrains",
    );
    for (oracle, at_peak) in oracles.iter().zip(&at_peaks) {
        let peak = oracle.admitted.budget_peak();
        assert_eq!(
            oracle.admitted.waited(),
            0,
            "no unit waits under the oracle's budget, so it runs as an unlimited \
             session: {:?}",
            oracle.case,
        );
        assert_eq!(
            peak > 0,
            admitted_against_the_budget(oracle),
            "Λ must be positive exactly when some unit was admitted against the budget \
             rather than exempt: {:?}",
            oracle.case,
        );
        assert_eq!(
            oracle.admitted.share_peak() > 0,
            invited_against_the_budget(oracle),
            "the share peak must be positive exactly when some question under the share \
             was not exempt: {:?}",
            oracle.case,
        );
        assert_eq!(
            at_peak.admitted.waited(),
            0,
            "a unit waited under a budget admitting the no-cost point {}: {:?}",
            oracle.admitted.no_cost_point(),
            oracle.case,
        );
        assert_eq!(
            at_peak.report,
            oracle.report,
            "a budget admitting the no-cost point {} (Λ = {peak}) changes the session: {:?}",
            oracle.admitted.no_cost_point(),
            oracle.case,
        );
    }
}
