//! A session swept across a ladder of budgets, against the properties that
//! hold exactly at every budget.
//!
//! # The claims
//!
//! For one pair on one link, write `T∞` for the session's duration with an
//! unlimited budget, `F` for the floor allowance every budget sets aside
//! for the floor slots, `X` for the fixed allowance it sets aside for the
//! session's fixed state, beyond both of which a ledger admits work against
//! the rest, `M` for [`MIN_SYNC_MEMORY_BUDGET`], the least budget the public
//! setter lets a session run on, `N*` for the session's no-cost point, the ledger budget defined
//! in [`no_slowdown`](super::no_slowdown), `W` for the bytes the busier
//! direction carries, `δ` for the link's one-way delay, and `h` for the
//! unlimited session's critical path in one-way delays: its duration over
//! the same delay with no serialization time, the longest chain of messages
//! each of which waits for the one before. At every budget `B` the setter
//! is given, the session runs on `B' = max(B, M)`, and:
//!
//! 1. If `B' ≥ F + X + N*`, the session is the unlimited session, event for
//!    event: every admission and share check the unlimited session makes
//!    passes against `B' − F − X`, by the induction along its schedule that
//!    [`no_slowdown`](super::no_slowdown) gives for a ledger budget of
//!    `N*`. So no unit waits, as none waits in the unlimited session.
//! 2. Each direction carries exactly the unlimited session's bytes: the
//!    budget decides when the session admits its work, never what it sends.
//! 3. `T(B) ≥ max(W + 2δ, h · δ)`. Each side writes its preamble while it
//!    reads its peer's, and writes nothing more until it has read the whole
//!    of it; both preambles take the same bytes, and every pair here
//!    carries far more than a preamble each way. So the busier direction's
//!    bytes after its preamble start serializing no earlier than its peer's
//!    preamble arrives, `δ` after it could have finished: its wire idles for
//!    at least `δ`, its last byte finishes serializing no earlier than
//!    `W + δ`, and arrives `δ` later. And the session's longest chain of
//!    messages takes `δ` per message on any link, which waiting for
//!    admission only lengthens.
//! 4. Each side's ledger is exact: no item needs more than its unit's
//!    grant, which a reply's parked chunks would if their room were priced
//!    below what the reply delivers; the ledger holds at most `B' − F − X`
//!    outside its floor slots, and its floor slots at most `F`, so it holds
//!    at most `B' − X` in all; and it asks questions only about scopes at
//!    depths of one parity and answers only at the other, the premise `F`
//!    is derived on.
//! 5. A budget below `M` runs the session as `M` does, event for event:
//!    the sessions at `F / 2`, at `F + X`, and at every rung below `M` are
//!    the session at `M`.
//!
//! # What it does not claim
//!
//! Below `F + X + N*`, a larger budget can make a session slower, and this test
//! states no tolerance there. Admission is greedy: it admits whatever fits
//! when room returns, deeper units first, each stage in arrival order. A
//! different budget admits a different set of units at some moment, which
//! reorders the work after it, and moves the stretches where one
//! direction's wire idles, waiting for room, against the other direction's
//! traffic. The duration is set by the busier wire's bytes, which do not
//! move, and by its idle time, which moves either way. Such an inversion is
//! an anomaly of the kind Graham showed for greedy list scheduling, where
//! more resources can lengthen a schedule.
//!
//! Those inversions are bounded above the largest grant any unit can take,
//! `c_max`, the least ledger budget which may be set by the public API. For
//! every pair of ledger budgets `c_max ≤ B ≤ B′`, with inversions counted
//! cumulatively, so that every pair is judged and not only adjacent budgets:
//!
//! - **(a)** on every link, no inversion exceeds one round trip,
//!   `T(B′) ≤ T(B) + 2δ`;
//! - **(b)** on the latency-only link, none occur once `B` reaches the
//!   session's depth-1 claims' total.
//!
//! On a link with bandwidth, a larger budget can still reorder a
//! direction's writes above that total, at budgets up to just below `N*`,
//! and lengthen the session by about one message's serialization. Those
//! residual inversions are observed, not asserted.
//!
//! [`budget_criterion`](super::budget_criterion) states the criterion in full
//! and asserts (a) on every link and (b) on the latency-only link, whose
//! threshold is the session's depth-1 claims' total. On a link with bandwidth, a
//! larger budget can reorder a direction's writes and lengthen the session by a
//! message's serialization or a few segments' at budgets up to just below `N*`,
//! so (b) fails there.
//!
//! # The measurement
//!
//! Each pair runs on the link model, whose clock counts byte-times, so a
//! session's timeline is an exact function of its pair, link, and budget.
//! For each pair and link, the test runs the unlimited session on the link,
//! which reads `N*`, `W` and `T∞`, and on its latency-only form, which
//! reads `h`. Then it runs the session at each budget of a fixed ladder,
//! every one through the public setter: `F + X` beside fractions of `N*`,
//! below and above it, `M`, and half of `F`; and it checks the claims at
//! each.

use rumors::MIN_SYNC_MEMORY_BUDGET;
use rumors::testing::{FIXED_ALLOWANCE, floor_allowance, ledger_budget_within};
use rumors_testkit::bench::link::{LinkConfig, Side};
use rumors_testkit::bench::session::{
    self as fixture, Case, DEPTHS, Grants, LAN_BDP, LONG_HAUL_BDP, METRO_BDP, Session, Shape,
    admitting,
};
use rumors_testkit::common::budget::BudgetChoice;

/// The unlimited budget the reference sessions run at.
const UNLIMITED: usize = usize::MAX;

/// The pairs the sweep runs, each with the bandwidth-delay product of its
/// link: a disjoint pair on each link, a mostly shared pair, and a small
/// replica against a large one.
///
/// Each pair's no-cost point exceeds the largest grant, so the ladder's
/// rungs above [`MIN_SYNC_MEMORY_BUDGET`] constrain it: a pair whose
/// no-cost point fits beside the minimum runs unconstrained at every
/// budget the public setter allows.
///
/// Beneath a child of the root, the small replica holds about one child,
/// where the large one holds about 100,000 / 256 ≈ 390 leaves, nearly all
/// under distinct grandchildren: more than a fan of grandchildren for the
/// one child listed. A reply about such a node carries at most a fan of
/// listing entries per question, which prices its parked room below the
/// peer's count.
const PAIRS: [(Shape, u64); 5] = [
    (Shape::overlapping(10_000, 0), LAN_BDP),
    (Shape::overlapping(10_000, 0), METRO_BDP),
    (Shape::overlapping(10_000, 0), LONG_HAUL_BDP),
    (Shape::overlapping(10_000, 90), LAN_BDP),
    (
        Shape {
            a: 100,
            ..Shape::symmetric(100_000)
        },
        LAN_BDP,
    ),
];

/// The denominator of the ladder's fractions of `N*`.
const SIXTEENTHS: usize = 16;

/// The budgets each pair runs at beside the floor and fixed allowances, in
/// sixteenths of its `N*`: from none, where only the floor slots admit
/// work, through `N*` itself, to twice it.
const LADDER: [usize; 9] = [0, 1, 4, 8, 12, 15, 16, 20, 32];

/// The rung below the ladder, half the floor allowance: a budget a
/// session raises to the minimum.
fn below_floor() -> usize {
    floor_allowance() / 2
}

/// The budget at the ladder's rung `sixteenths`, for a pair whose no-cost
/// point is `no_cost`.
///
/// That is the floor and fixed allowances and the rung's fraction of the
/// no-cost point beside them, at least [`admitting`] the no-cost point exactly when
/// the rung is at least `N*`.
fn budget(no_cost: usize, sixteenths: usize) -> usize {
    admitting(no_cost * sixteenths / SIXTEENTHS)
}

/// Every pair's link.
fn link(index: usize) -> LinkConfig {
    LinkConfig::with_bdp(PAIRS[index].1)
}

/// Every pair on its link at an unlimited budget, with its link changed by
/// `on`.
fn unlimited(on: impl Fn(LinkConfig) -> LinkConfig) -> [Case; PAIRS.len()] {
    std::array::from_fn(|index| Case {
        shape: PAIRS[index].0,
        budget: BudgetChoice::Bytes(UNLIMITED),
        link: on(link(index)),
    })
}

/// The rungs each pair runs at: the ladder's, then the minimum, then the
/// one below the ladder.
const RUNGS: usize = LADDER.len() + 2;

/// The index among a pair's rungs of the minimum's rung.
const MINIMUM_RUNG: usize = LADDER.len();

/// Every pair at every rung, pair-major: the ladder's, each a fraction of
/// the pair's no-cost point in `no_cost` beside the floor and fixed
/// allowances, then [`MIN_SYNC_MEMORY_BUDGET`], then [`below_floor`].
fn ladder(no_cost: &[usize; PAIRS.len()]) -> [Case; PAIRS.len() * RUNGS] {
    std::array::from_fn(|index| {
        let pair = index / RUNGS;
        let rung = index % RUNGS;
        let budget = match LADDER.get(rung) {
            Some(&sixteenths) => budget(no_cost[pair], sixteenths),
            None if rung == MINIMUM_RUNG => MIN_SYNC_MEMORY_BUDGET,
            None => below_floor(),
        };
        Case {
            shape: PAIRS[pair].0,
            budget: BudgetChoice::Bytes(budget),
            link: link(pair),
        }
    })
}

/// The memory budget `case` gives the public setter.
///
/// # Panics
///
/// If `case` sets its budget otherwise: every session here runs through
/// the setter.
fn requested(case: &Case) -> usize {
    match case.budget {
        BudgetChoice::Bytes(budget) => budget,
        other => panic!("the sweep sets every budget through the public setter, not {other:?}"),
    }
}

/// The bytes each direction of `session` carried, in [`Side::BOTH`] order.
fn bytes(session: &Session) -> [u64; 2] {
    Side::BOTH.map(|side| session.report.sent_by(side).bytes())
}

/// The parity of the depths of the scopes the units in `grants`, indexed
/// by depth, concern: `Some` of it if they all share one, `Some(None)` if
/// there are none, and `None` if they mix parities.
fn parity(grants: &[Grants; DEPTHS]) -> Option<Option<usize>> {
    let mut depths = grants
        .iter()
        .enumerate()
        .filter(|(_, grants)| grants.units > 0)
        .map(|(depth, _)| depth % 2);
    let first = depths.next();
    depths.all(|parity| Some(parity) == first).then_some(first)
}

/// Checks the claims at one rung: `session`, at its case's budget, against
/// `reference`, the same pair and link at an unlimited budget.
///
/// `no_cost` is the reference's no-cost point, and `floor` the lower bound
/// on every session's duration.
fn check_rung(session: &Session, reference: &Session, no_cost: usize, floor: u64) {
    let case = &session.case;
    let allowance = floor_allowance();
    let runs_on = requested(case).max(MIN_SYNC_MEMORY_BUDGET);
    let admitted = ledger_budget_within(runs_on);
    println!(
        "{:?} on BDP {}: B = {}, B' = {runs_on} (F + X = {allowance} + {FIXED_ALLOWANCE} \
         + {:.4} N*), {} waited, T = {} byte-times, T∞ = {}, floor {floor}",
        case.shape,
        case.link.bdp(),
        requested(case),
        admitted as f64 / no_cost as f64,
        session.admitted.waited(),
        session.duration(),
        reference.duration(),
    );
    if runs_on >= admitting(no_cost) {
        assert_eq!(
            session.admitted.waited(),
            0,
            "a unit waited under a budget admitting at least the no-cost point {no_cost}: \
             {case:?}",
        );
        assert_eq!(
            session.report, reference.report,
            "a budget admitting at least the no-cost point {no_cost} changes the session: \
             {case:?}",
        );
    }
    assert_eq!(
        bytes(session),
        bytes(reference),
        "the budget changes the bytes a direction carries: {case:?}",
    );
    assert!(
        session.duration() >= floor,
        "the session lasts {} byte-times, below max(W + 2δ, h · δ) = {floor}: {case:?}",
        session.duration(),
    );
    for ledger in &session.admitted.ledgers {
        assert!(ledger.charged > 0, "a ledger stopped counting: {case:?}");
        assert_eq!(
            ledger.short, 0,
            "items needed bytes beyond their units' grants: {case:?}"
        );
        assert!(
            ledger.budget_peak <= admitted,
            "a ledger held {} B outside its floor slots, over the {admitted} B the budget \
             admits beside the floor and fixed allowances: {case:?}",
            ledger.budget_peak,
        );
        assert!(
            ledger.floor_peak <= allowance,
            "a ledger's floor slots held {} B, over the floor allowance of \
             {allowance} B: {case:?}",
            ledger.floor_peak,
        );
        assert!(
            ledger.held_peak <= runs_on,
            "a ledger held {} B, over the {runs_on} B the session runs on: {case:?}",
            ledger.held_peak,
        );
        let (questions, answers) = (parity(&ledger.questions), parity(&ledger.answers));
        assert!(
            matches!((questions, answers), (Some(Some(asked)), Some(Some(answered))) if asked != answered),
            "a ledger asked about depths of parity {questions:?} and answered at {answers:?}, \
             not one parity each: {case:?}",
        );
    }
}

/// Across a ladder of budgets, every session keeps the exact properties.
///
/// They are identity with the unlimited session at or above the no-cost
/// point beside the floor and fixed allowances, identity with the session at
/// [`MIN_SYNC_MEMORY_BUDGET`] below it, and the bytes, the lower bound on
/// duration, and ledger exactness at every budget.
///
/// Every session whose budget, raised to the minimum, admits at least its
/// pair's no-cost point beside the floor and fixed allowances is the
/// unlimited session, event for event, with no unit waiting; every session
/// below the minimum, at the two allowances and at half the floor
/// allowance included, is the session at the minimum; every session
/// carries the unlimited session's bytes in each direction, lasts at least
/// its busier direction's bytes and two delays, and at least the unlimited
/// session's critical path; and every ledger is exact, holds at most its
/// raised budget, and asks and answers at depths of opposite parities.
///
/// Below the no-cost point this test holds the duration to the lower bound
/// alone: greedy admission makes it non-monotone there, and the owner's
/// criterion for its inversions is asserted by
/// [`budget_criterion`](super::budget_criterion).
///
/// Each unlimited session must have kept every unit from waiting and
/// admitted work against the budget, and each pair must have some rung
/// where units waited, so that the ladder judges sessions the budget
/// constrains and `N*` reads a counter that counted.
#[test]
fn every_budget_keeps_the_exact_properties() {
    let references = fixture::run_all(unlimited(|link| link));
    let critical = fixture::run_all(unlimited(LinkConfig::latency_only));
    let no_cost = references
        .each_ref()
        .map(|reference| reference.admitted.no_cost_budget());
    let swept = fixture::run_all(ladder(&no_cost));

    for (pair, (reference, critical)) in references.iter().zip(&critical).enumerate() {
        assert_eq!(
            reference.admitted.waited(),
            0,
            "no unit waits at an unlimited budget: {:?}",
            reference.case,
        );
        assert!(
            reference.admitted.budget_peak() > 0,
            "the ledgers must admit some work against the budget, or N* reads \
             nothing: {:?}",
            reference.case,
        );
        let delay = reference.case.link.one_way_delay();
        let floor = (reference.busier_direction_bytes() + 2 * delay).max(critical.hops() * delay);
        let sessions = &swept[pair * RUNGS..][..RUNGS];
        for session in sessions {
            check_rung(session, reference, no_cost[pair], floor);
        }
        let at_minimum = &sessions[MINIMUM_RUNG];
        let below_minimum: Vec<_> = sessions
            .iter()
            .filter(|session| requested(&session.case) < MIN_SYNC_MEMORY_BUDGET)
            .collect();
        assert!(
            below_minimum.len() >= 2,
            "the rungs at F + X and at F / 2 lie below the minimum: {:?}",
            reference.case,
        );
        for session in below_minimum {
            assert_eq!(
                session.report, at_minimum.report,
                "a budget below the minimum runs otherwise than the minimum: {:?}",
                session.case,
            );
        }
        assert!(
            sessions.iter().any(|session| session.admitted.waited() > 0),
            "some rung must constrain the session, or the ladder judges only \
             unconstrained ones: {:?}",
            reference.case,
        );
    }
}
