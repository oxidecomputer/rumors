//! A session swept across a ladder of budgets, against the properties that
//! hold exactly at every budget.
//!
//! # The claims
//!
//! For one pair on one link, write `T∞` for the session's duration with an
//! unlimited budget, `F` for the floor allowance every budget sets aside
//! for the floor slots, which a ledger admits work beyond against the
//! rest, `N*` for the session's no-cost point, the ledger budget defined
//! in [`no_slowdown`](super::no_slowdown), `W` for the bytes the busier
//! direction carries, `δ` for the link's one-way delay, and `h` for the
//! unlimited session's critical path in one-way delays: its duration over
//! the same delay with no serialization time, the longest chain of messages
//! each of which waits for the one before. At every budget `B`:
//!
//! 1. If `B ≥ F + N*`, the session is the unlimited session, event for
//!    event: every admission and share check the unlimited session makes
//!    passes against `B − F`, by the induction along its schedule that
//!    [`no_slowdown`](super::no_slowdown) gives for a ledger budget of
//!    `N*`. So no unit waits, as none waits in the unlimited session.
//! 2. Each direction carries exactly the unlimited session's bytes: the
//!    budget decides when the session admits its work, never what it sends.
//! 3. `T(B) ≥ max(W + δ, h · δ)`. The busier direction's last byte
//!    finishes serializing no earlier than `W`, and arrives `δ` later; and
//!    the session's longest chain of messages takes `δ` per message on any
//!    link, which waiting for admission only lengthens.
//! 4. Each side's ledger is exact: no item needs more than its unit's
//!    grant, which a reply's parked chunks would if their room were priced
//!    below what the reply delivers; the ledger holds at most `B − F`
//!    outside its floor slots, and its floor slots at most `F`, so it holds
//!    at most the larger of `B` and `F` in all; and it asks questions only
//!    about scopes at depths of one parity and answers only at the other,
//!    the premise `F` is derived on.
//! 5. A budget at or below `F` runs the session on its floor slots alone,
//!    as a budget of zero does: the session at `F / 2` is the session at
//!    `F`, event for event.
//!
//! # What it does not claim
//!
//! Below `F + N*`, a larger budget can make a session slower, so the test
//! states no tolerance there, and no monotonicity. Admission is greedy: it
//! admits whatever fits when room returns, deeper units first, each stage
//! in arrival order. A different budget admits a different set of units at
//! some moment, which reorders the work after it, and moves the stretches
//! where one direction's wire idles, waiting for room, against the other
//! direction's traffic. The duration is set by the busier wire's bytes,
//! which do not move, and by its idle time, which moves either way. Such an
//! inversion is an anomaly of the kind Graham showed for greedy list
//! scheduling, where more resources can lengthen a schedule, and the
//! admission rule yields no bound on its size.
//!
//! # The measurement
//!
//! Each pair runs on the link model, whose clock counts byte-times, so a
//! session's timeline is an exact function of its pair, link, and budget.
//! For each pair and link, the test runs the unlimited session on the link,
//! which reads `N*`, `W` and `T∞`, and on its latency-only form, which
//! reads `h`. Then it runs the session at each budget of a fixed ladder:
//! `F` beside fractions of `N*`, below and above it, and half of `F`; and
//! it checks the claims at each.

use rumors::testing::floor_allowance;
use rumors_testkit::bench::link::{LinkConfig, Side};
use rumors_testkit::bench::session::{
    self as fixture, Case, DEPTHS, Grants, LAN_BDP, LONG_HAUL_BDP, METRO_BDP, Session, Shape,
    admitting,
};

/// The unlimited budget the reference sessions run at.
const UNLIMITED: usize = usize::MAX;

/// The pairs the sweep runs, each with the bandwidth-delay product of its
/// link: a disjoint pair on each link, a mostly shared pair, and a small
/// replica against a large one.
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
    (Shape::overlapping(1_000, 0), LONG_HAUL_BDP),
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

/// The budgets each pair runs at beside the floor allowance, in sixteenths
/// of its `N*`: from none, where only the floor slots admit work, through
/// `N*` itself, to twice it.
const LADDER: [usize; 9] = [0, 1, 4, 8, 12, 15, 16, 20, 32];

/// The rung below the ladder, half the floor allowance: a budget the
/// allowance exceeds, which admits work through the floor slots alone.
fn below_floor() -> usize {
    floor_allowance() / 2
}

/// The budget at the ladder's rung `sixteenths`, for a pair whose no-cost
/// point is `no_cost`.
///
/// That is the floor allowance and the rung's fraction of the no-cost
/// point beside it, at least [`admitting`] the no-cost point exactly when
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
        budget: UNLIMITED,
        link: on(link(index)),
    })
}

/// The rungs each pair runs at: the ladder's, then the one below it.
const RUNGS: usize = LADDER.len() + 1;

/// Every pair at every rung, pair-major: the ladder's, each a fraction of
/// the pair's no-cost point in `no_cost` beside the floor allowance, then
/// [`below_floor`].
fn ladder(no_cost: &[usize; PAIRS.len()]) -> [Case; PAIRS.len() * RUNGS] {
    std::array::from_fn(|index| {
        let pair = index / RUNGS;
        let budget = LADDER
            .get(index % RUNGS)
            .map_or_else(below_floor, |&sixteenths| budget(no_cost[pair], sixteenths));
        Case {
            shape: PAIRS[pair].0,
            budget,
            link: link(pair),
        }
    })
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
    let admitted = case.budget.saturating_sub(allowance);
    println!(
        "{:?} on BDP {}: B = {} (F = {allowance} + {:.4} N*), {} waited, T = {} \
         byte-times, T∞ = {}, floor {floor}",
        case.shape,
        case.link.bdp(),
        case.budget,
        admitted as f64 / no_cost as f64,
        session.admitted.waited(),
        session.duration(),
        reference.duration(),
    );
    if case.budget >= admitting(no_cost) {
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
        "the session lasts {} byte-times, below max(W + δ, h · δ) = {floor}: {case:?}",
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
             admits beside the floor allowance: {case:?}",
            ledger.budget_peak,
        );
        assert!(
            ledger.floor_peak <= allowance,
            "a ledger's floor slots held {} B, over the floor allowance of \
             {allowance} B: {case:?}",
            ledger.floor_peak,
        );
        assert!(
            ledger.held_peak <= case.budget.max(allowance),
            "a ledger held {} B, over its budget and the floor allowance: {case:?}",
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
/// point beside the floor allowance, identity with the floor-only session
/// below the allowance, and the bytes, the lower bound on duration, and
/// ledger exactness at every budget.
///
/// Every session whose budget admits at least its pair's no-cost point
/// beside the floor allowance is the unlimited session, event for event,
/// with no unit waiting; the session at half the allowance is the session
/// at the allowance; every session carries the unlimited session's bytes
/// in each direction, lasts at least its busier direction's bytes and one
/// delay, and at least the unlimited session's critical path; and every ledger is exact, holds
/// at most its budget (or the allowance, below it), and asks and answers
/// at depths of opposite parities.
///
/// Below the no-cost point the duration is held to the lower bound alone:
/// greedy admission makes it non-monotone there, with no derivable bound on
/// the inversions.
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
        let floor = (reference.busier_direction_bytes() + delay).max(critical.hops() * delay);
        let sessions = &swept[pair * RUNGS..][..RUNGS];
        for session in sessions {
            check_rung(session, reference, no_cost[pair], floor);
        }
        let (at_floor, below_floor) = (&sessions[0], &sessions[LADDER.len()]);
        assert_eq!(
            below_floor.report, at_floor.report,
            "a budget below the floor allowance runs otherwise than one at it: {:?}",
            below_floor.case,
        );
        assert!(
            sessions.iter().any(|session| session.admitted.waited() > 0),
            "some rung must constrain the session, or the ladder judges only \
             unconstrained ones: {:?}",
            reference.case,
        );
    }
}
