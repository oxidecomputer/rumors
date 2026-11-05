//! A budget at least the session's unlimited-budget peak costs it nothing:
//! the session runs exactly as it would with no budget at all.
//!
//! # The claim
//!
//! Each side's ledger admits the work its session builds in units. A unit
//! first takes its depth's floor slot if that slot is free, whatever the
//! budget, and otherwise is admitted against the budget if the bytes the
//! ledger holds outside its floor slots, with the unit's grant added, fit
//! within it. Let `Λ` be the most bytes either side's ledger holds outside
//! its floor slots when the budget is unlimited. With a budget `B ≥ Λ`,
//! every admission the unlimited session makes against the budget fits
//! within `B` as well, so no unit waits that did not wait there. The floor
//! slots' use does not depend on the budget, so every admission decision,
//! and with them the session, is the unlimited session's, event for event.
//!
//! The budget here is the ledger's: a peer's memory budget less the floor
//! allowance it sets aside for the floor slots, so the suite configures
//! each budget it states beside that allowance ([`fixture::admitting`]).
//!
//! # The measurement
//!
//! For each pair, of several sizes and overlaps, on a LAN and on a
//! long-haul link, the suite runs the session twice on the link model,
//! whose clock counts byte-times and advances only when both peers wait,
//! so a session's timeline is an exact function of its pair, link, and
//! budgets:
//!
//! - the oracle, with an unlimited budget, which reads `Λ` from the ledgers
//!   and must have kept every unit from waiting;
//! - the session with both budgets at `Λ`, which must carry the oracle's
//!   every segment at the oracle's every instant.
//!
//! The comparison is an identity, with no tolerance: any admission the
//! budget refused would reorder the session's work and move its segments.

use std::array;

use rumors_testkit::bench::link::LinkConfig;

use rumors_testkit::bench::session::{
    self as fixture, Case, LAN_BDP, LONG_HAUL_BDP, Session, Shape,
};

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
fn cases(budget: impl Fn(usize) -> usize) -> [Case; SIZES.len() * OVERLAPS.len() * LINKS.len()] {
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

/// Prints one pair's oracle beside its session at `Λ`.
fn report(oracle: &Session, at_peak: &Session) {
    let Case { shape, link, .. } = oracle.case;
    let messages = shape.shared + shape.a;
    let peak = oracle.admitted.budget_peak();
    println!(
        "{messages} messages a side, {} shared, BDP {} B: Λ = {peak} B ({:.1} B per \
         message), {} units waited at Λ; {} byte-times unlimited, {} at Λ",
        shape.shared,
        link.bdp(),
        peak as f64 / messages as f64,
        at_peak.admitted.waited(),
        oracle.duration(),
        at_peak.duration(),
    );
}

/// A budget equal to the session's unlimited-budget peak leaves every
/// session unchanged, event for event.
///
/// Across sizes, disjoint and mostly shared pairs, and a LAN and a
/// long-haul link, a session with both budgets at `Λ` carries the same
/// segments at the same instants as the same session with an unlimited
/// budget, under which no unit waited.
#[test]
fn a_budget_of_the_unlimited_peak_is_identical_to_no_budget() {
    let oracles = fixture::run_all(cases(|_| ORACLE_BUDGET));
    let peaks = oracles
        .each_ref()
        .map(|oracle| oracle.admitted.budget_peak());
    let at_peaks = fixture::run_all(cases(|index| fixture::admitting(peaks[index])));
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
        assert_eq!(
            at_peak.report, oracle.report,
            "a budget of Λ = {peak} changes the session: {:?}",
            oracle.case,
        );
    }
}
