//! A budget at the session's no-cost point costs it nothing: the session
//! runs exactly as it would with no budget at all.
//!
//! # The claim
//!
//! Each side's ledger admits the work its session builds in units. A unit
//! first takes its depth's floor slot if that slot is free, whatever the
//! budget, and otherwise is admitted against the budget if the bytes the
//! ledger holds outside its floor slots, with the unit's grant added, fit
//! within it. A question under the invitation share must also keep its
//! depth's claims within the budget: the room the depth's questions hold,
//! with its own grant, beside the predicted needs of their unfinished
//! replies and its own, scaled by the depth's pace,
//! `I_d + c + α · (N_d + n) ≤ B`.
//!
//! Let `Λ` be the most bytes either side's ledger holds outside its floor
//! slots when the budget is unlimited, and the *share peak* the largest sum
//! any share check of that session weighs. The session's *no-cost point* is
//! the larger of the two. At a budget at least that, every admission the
//! unlimited session makes against the budget fits within it, every share
//! check it makes passes, and the floor slots' use does not depend on the
//! budget, so no unit waits that did not wait there. Every admission
//! decision, and with them the session, is then the unlimited session's,
//! event for event: an induction along the unlimited session's schedule.
//! The share peak exceeds `Λ` wherever a depth asks more questions at once
//! than the budget could also process their replies with; between the two,
//! the share holds questions back, and identity is promised only at the
//! no-cost point.
//!
//! The budget here is the ledger's: a peer's memory budget less the floor
//! allowance it sets aside for the floor slots and the fixed allowance it
//! sets aside for the session's fixed state, so the suite sets each budget
//! it states as the ledger's directly ([`BudgetChoice::Ledger`]), reaching
//! budgets below those the public setter allows.
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

/// Prints one pair's oracle beside its session at the no-cost point.
fn report(oracle: &Session, at_peak: &Session) {
    let Case { shape, link, .. } = oracle.case;
    let messages = shape.shared + shape.a;
    let peak = oracle.admitted.budget_peak();
    let no_cost = oracle.admitted.no_cost_budget();
    println!(
        "{messages} messages a side, {} shared, BDP {} B: Λ = {peak} B ({:.1} B per \
         message), no-cost point {no_cost} B ({:.3} Λ), {} units waited there; {} \
         byte-times unlimited, {} there",
        shape.shared,
        link.bdp(),
        peak as f64 / messages as f64,
        no_cost as f64 / peak as f64,
        at_peak.admitted.waited(),
        oracle.duration(),
        at_peak.duration(),
    );
}

/// A budget at the session's no-cost point leaves every session
/// unchanged, event for event.
///
/// Across sizes, disjoint and mostly shared pairs, and a LAN and a
/// long-haul link, a session with both budgets at the larger of `Λ` and the
/// share peak carries the same segments at the same instants as the same
/// session with an unlimited budget, and like it keeps every unit from
/// waiting. Each
/// oracle must have admitted work against the budget and made a share
/// check, so that neither term of the no-cost point reads zero because its
/// counter stopped counting.
#[test]
fn a_budget_of_the_no_cost_point_is_identical_to_no_budget() {
    let oracles = fixture::run_all(cases(|_| BudgetChoice::Bytes(ORACLE_BUDGET)));
    let peaks = oracles
        .each_ref()
        .map(|oracle| oracle.admitted.no_cost_budget());
    let at_peaks = fixture::run_all(cases(|index| BudgetChoice::Ledger(peaks[index])));
    for (oracle, at_peak) in oracles.iter().zip(&at_peaks) {
        report(oracle, at_peak);
    }
    for (oracle, at_peak) in oracles.iter().zip(&at_peaks) {
        let peak = oracle.admitted.budget_peak();
        assert_eq!(
            oracle.admitted.waited(),
            0,
            "no unit waits under the oracle's budget, so it runs as an unlimited \
             session: {:?}",
            oracle.case,
        );
        assert!(
            peak > 0,
            "the ledgers must admit some work against the budget, or Λ reads \
             nothing: {:?}",
            oracle.case,
        );
        assert!(
            oracle.admitted.share_peak() > 0,
            "some question must be checked against its depth's claims, or the \
             share peak reads nothing: {:?}",
            oracle.case,
        );
        assert_eq!(
            at_peak.admitted.waited(),
            0,
            "a unit waited under a budget admitting the no-cost point {}: {:?}",
            oracle.admitted.no_cost_budget(),
            oracle.case,
        );
        assert_eq!(
            at_peak.report,
            oracle.report,
            "a budget at the no-cost point {} (Λ = {peak}) changes the session: {:?}",
            oracle.admitted.no_cost_budget(),
            oracle.case,
        );
    }
}
