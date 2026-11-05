//! The budget criterion: how a session's duration may move as its budget
//! grows below the no-cost point, on generated pairs and links.
//!
//! # The criterion
//!
//! Write `T(B)` for a session's duration at ledger budget `B` (the budget
//! its ledgers admit work against, beside the floor and session allowances),
//! `N*` for the session's no-cost point, and `δ` for the link's one-way
//! delay, so that one round trip is `2δ` on every link. The owner's ruling
//! states what holds for every pair of ledger budgets `0 ≤ B ≤ B′`, zero
//! included: below as well as above the largest grant
//! ([`LARGEST_GRANT`](rumors::testing::LARGEST_GRANT)), the least ledger
//! budget a public memory budget leaves, which only the test path's
//! budgets reach below:
//!
//! - **(a)** On every link, no inversion exceeds one round trip:
//!   `T(B′) ≤ T(B) + 2δ`. Inversions are cumulative, so the claim covers
//!   every pair, not only adjacent budgets.
//! - **(b)** On the latency-only link, none occur above a threshold the
//!   session sets: `T(B′) ≤ T(B)` whenever `B ≥ B*`.
//!
//! On links with bandwidth, a larger budget can still reorder the bytes a
//! direction writes above `B*`, which shifts its serialization by up to
//! about a message, so (b) is not asserted there.
//!
//! At `B ≥ N*` the session is the unlimited session, event for event
//! ([`no_slowdown`](super::no_slowdown)), so the pairs that matter have `B <
//! N*`; `B′` ranges up to the unlimited budget.
//!
//! # The threshold
//!
//! `B*` is `D₁`, the session's *depth-1 claims' total*: the largest sum the
//! invitation share weighs for the depth-1 questions the responder's root
//! answer asks, in the unlimited session on the latency-only link. On that link
//! the root answer asks all of them in one instant, before any depth-1 reply
//! can arrive, so their checks are its ledger's first, the pace is one, and
//! nothing but those questions is held against its budget. At every budget of
//! at least `D₁` each of them is admitted as it is asked, and the session runs
//! as the unlimited one until the root answer has asked its last; at every
//! budget below `D₁` one of them waits, and the root answer's later questions
//! wait behind it for depth-1 replies to be processed. `D₁` is the budget above
//! which that first burst no longer depends on the budget.
//!
//! # The measurement
//!
//! Each case draws a pair (its size, the share of messages both sides hold, and
//! its network's seed) and a link with bandwidth. It runs the unlimited session
//! on the latency-only link, which reads `N*` and `D₁`, and on the drawn link,
//! which reads that link's `N*`. Then it runs the pair at [`BUDGETS`] budgets
//! drawn below each link's `N*`: on the latency-only link the first below `D₁`
//! and the rest at or above it, so every case judges pairs on both sides of the
//! threshold; on the drawn link all across `[0, N*)`. It checks (a) on every
//! pair of each link's budgets, the unlimited session among them, and (b) on
//! every latency-only pair whose smaller budget is at least `D₁`. Over the run,
//! it requires some drawn budget to have slowed its session beyond the
//! unlimited one, and some pair of two drawn budgets, both below `N*`, so that
//! the criterion is judged on sessions the budget constrains.
//!
//! A second test checks the threshold's own property on the same pairs
//! ([`the_first_burst_saturates_at_the_depth1_claims_total`]): it runs each
//! pair on the latency-only link at `D₁`, at `D₁ − 1`, and at one budget drawn
//! on each side of them, and compares each session's first burst with the
//! unlimited session's.

use proptest::prelude::*;
use rumors_testkit::bench::link::{LinkConfig, Side};
use rumors_testkit::bench::session::{
    self as fixture, Case, DatedAdmission, LAN_BDP, LONG_HAUL_BDP, METRO_BDP, Session, Shape,
};
use rumors_testkit::common::budget::BudgetChoice;
use rumors_testkit::common::property::{FLEET_DIVISOR, run};
use rumors_testkit::common::seed::{NetworkSeed, network_seed};

/// The unlimited budget the reference sessions run at.
const UNLIMITED: usize = usize::MAX;

/// The links with bandwidth a case draws from, by bandwidth-delay product.
const LINKS: [u64; 3] = [LAN_BDP, METRO_BDP, LONG_HAUL_BDP];

/// The budgets a case runs on each link below its no-cost point.
const BUDGETS: usize = 3;

/// The denominator of the fractions that place a budget in its range.
const PARTS: u32 = 1 << 16;

/// The pairs a case draws: messages on each side, and the percent of them
/// both sides hold.
///
/// The sizes give a no-cost point of about 5 to 6.5 MB on the
/// latency-only link and a depth-1 claims' total of about 4 to 5.3 MB,
/// while keeping a case's sessions inside the suite's time.
///
/// The sizes start high and sharing is capped so that `D₁` stays below the
/// no-cost point. On the latency-only link that point is then the
/// initiator's depth-2 share peak: the depth-2 questions its answers ask at
/// once, about as many as the messages each side holds alone, with their
/// rooms and their replies' needs. `D₁` weighs a fixed fan of questions,
/// one per child of the root, beyond the first of them, which fit within
/// their kind and depth's bound and take no share check; their rooms and
/// needs follow every child the peer lists, shared or not. So sharing
/// removes depth-2 questions without shrinking `D₁`, and fewer messages
/// shrink the depth-2 peak toward `D₁`'s fixed fan. From 10,000 messages
/// and up to a twentieth shared, `D₁` stays below about 0.86 of the no-cost
/// point; at 10,000 messages it reaches it at about a fifth shared, leaving
/// nothing above `D₁` to judge.
fn shapes() -> impl Strategy<Value = Shape> {
    (10_000_u64..=12_000, 0_u64..=5)
        .prop_map(|(messages, shared)| Shape::overlapping(messages, shared))
}

/// The ledger budget `parts / PARTS` of the way from `low` to `high`.
fn between(low: usize, high: usize, parts: u32) -> usize {
    low + (high - low) * parts as usize / PARTS as usize
}

/// The case of `shape` on `link` at ledger budget `budget`, or at an
/// unlimited budget for [`UNLIMITED`].
fn case(shape: Shape, link: LinkConfig, budget: usize) -> Case {
    Case {
        shape,
        budget: if budget == UNLIMITED {
            BudgetChoice::Bytes(UNLIMITED)
        } else {
            BudgetChoice::Ledger(budget)
        },
        link,
    }
}

/// The depth-1 claims' total of the unlimited session `reference` on the
/// latency-only link: the largest depth-1 share check either ledger weighed.
///
/// Those checks are the responder's first, so each one that outweighed
/// the others raised its ledger's share peak
/// ([`LedgerReading::share_peaks`](rumors_testkit::bench::session::LedgerReading::share_peaks)).
fn depth1_claims(reference: &Session) -> usize {
    reference
        .admitted
        .ledgers
        .iter()
        .map(|ledger| ledger.share_peaks[1])
        .max()
        .unwrap_or(0)
}

/// What one case judged, for the run's liveness checks.
#[derive(Clone, Copy, Default)]
struct Judged {
    /// Pairs of two drawn budgets `B < B′`, both below the no-cost point.
    constrained: usize,
    /// Drawn budgets at which the session lasted longer than with no budget.
    slowed: usize,
}

impl Judged {
    /// Both cases' counts, added.
    fn add(self, other: Self) -> Self {
        Self {
            constrained: self.constrained + other.constrained,
            slowed: self.slowed + other.slowed,
        }
    }
}

/// Checks (a) on every pair of one link's sessions, and (b) on every pair
/// whose smaller budget is at least `threshold`, if given, returning what
/// it judged.
///
/// `sessions` are the link's sessions at drawn budgets below its no-cost
/// point, sorted by ledger budget, and `reference` its unlimited session,
/// which joins every pair as the larger budget.
fn check_pairs(
    sessions: &[(usize, &Session)],
    reference: &Session,
    threshold: Option<usize>,
) -> Judged {
    let unlimited = [(UNLIMITED, reference)];
    let mut judged = Judged::default();
    for (index, &(b, smaller)) in sessions.iter().enumerate() {
        judged.slowed += usize::from(smaller.duration() > reference.duration());
        for &(b_prime, larger) in sessions[index..].iter().chain(&unlimited) {
            judged.constrained += usize::from(b < b_prime && b_prime != UNLIMITED);
            let delay = smaller.case.link.one_way_delay();
            let (t, t_prime) = (smaller.duration(), larger.duration());
            assert!(
                t_prime <= t + 2 * delay,
                "(a): the session lasts {t_prime} byte-times at ledger budget {b_prime}, more \
                 than one round trip ({}) beyond {t} at {b}: {:?}",
                2 * delay,
                smaller.case,
            );
            if threshold.is_some_and(|threshold| b >= threshold) {
                assert!(
                    t_prime <= t,
                    "(b): the session lasts {t_prime} byte-times at ledger budget {b_prime}, \
                     longer than {t} at {b}, at or above the threshold {threshold:?}: {:?}",
                    smaller.case,
                );
            }
        }
    }
    judged
}

/// Runs one case: `shape` on `network`, on the latency-only link and on the
/// link of product `bdp`, at budgets placed by `latency_parts` and
/// `link_parts`; checks its premise and the criterion, and returns what it
/// judged.
fn check_case(
    shape: Shape,
    bdp: u64,
    latency_parts: [u32; BUDGETS],
    link_parts: [u32; BUDGETS],
    network: NetworkSeed,
) -> Judged {
    let shape = shape.with_network(network);
    let latency_only = LinkConfig::with_bdp(bdp).latency_only();
    let link = LinkConfig::with_bdp(bdp);
    let references = fixture::run_all([
        case(shape, latency_only, UNLIMITED),
        case(shape, link, UNLIMITED),
    ]);
    let no_cost = references
        .each_ref()
        .map(|reference| reference.admitted.no_cost_point());
    let threshold = depth1_claims(&references[0]);
    assert!(
        0 < threshold && threshold < no_cost[0],
        "the premise fails: the depth-1 claims' total {threshold} must lie strictly between \
         zero and the no-cost point {}: {shape:?}",
        no_cost[0],
    );
    assert!(
        no_cost[1] > 0,
        "the premise fails: the no-cost point on the drawn link must exceed zero: {shape:?}",
    );
    let latency_budgets: [usize; BUDGETS] = std::array::from_fn(|index| match index {
        0 => between(0, threshold, latency_parts[index]),
        _ => between(threshold, no_cost[0], latency_parts[index]),
    });
    let link_budgets = link_parts.map(|parts| between(0, no_cost[1], parts));
    let swept = fixture::run_all(std::array::from_fn::<_, { 2 * BUDGETS }, _>(|index| {
        if index < BUDGETS {
            case(shape, latency_only, latency_budgets[index])
        } else {
            case(shape, link, link_budgets[index - BUDGETS])
        }
    }));
    let mut judged = Judged::default();
    for (on_link, (reference, budgets)) in [
        (&references[0], latency_budgets),
        (&references[1], link_budgets),
    ]
    .into_iter()
    .enumerate()
    {
        let sessions = &swept[on_link * BUDGETS..][..BUDGETS];
        let mut pairs: Vec<(usize, &Session)> = budgets.into_iter().zip(sessions).collect();
        for (budget, session) in &pairs {
            assert!(
                session.admitted.waited() > 0,
                "no unit waited at ledger budget {budget}, below the no-cost point {}, so the \
                 case judges an unconstrained session: {:?}",
                no_cost[on_link],
                session.case,
            );
        }
        pairs.sort_by_key(|&(budget, _)| budget);
        judged = judged.add(check_pairs(
            &pairs,
            reference,
            (on_link == 0).then_some(threshold),
        ));
    }
    judged
}

/// At every budget, zero included, a larger budget never costs a session
/// more than one round trip, on any link, and never costs it anything at
/// or above its depth-1 claims' total on the latency-only link.
///
/// This is the criterion the module docs state, measured rather than proven:
/// (a) on every link, and (b) on the latency-only link alone, since on a link
/// with bandwidth a larger budget can reorder a direction's writes and
/// lengthen the session by about one message's serialization.
///
/// Neither half may pass vacuously. Each case premises a constrained
/// session: its no-cost point exceeds zero on both links, its depth-1
/// claims' total lies strictly between zero and the no-cost point, so it
/// judges pairs both below and above the threshold, and units wait at every
/// budget it draws. And over the run, some drawn budget must have slowed
/// its session beyond the unlimited one, and some pair must hold two drawn
/// budgets `B < B′`, both below the no-cost point.
#[test]
fn a_larger_budget_costs_at_most_a_round_trip() {
    let judged = std::cell::Cell::new(Judged::default());
    run(
        None,
        FLEET_DIVISOR,
        concat!(
            module_path!(),
            "::a_larger_budget_costs_at_most_a_round_trip"
        ),
        file!(),
        (
            shapes(),
            proptest::sample::select(LINKS.to_vec()),
            proptest::array::uniform::<_, BUDGETS>(0..PARTS),
            proptest::array::uniform::<_, BUDGETS>(0..PARTS),
            network_seed(),
        ),
        |(shape, bdp, latency_parts, link_parts, network)| {
            judged.set(judged.get().add(check_case(
                shape,
                bdp,
                latency_parts,
                link_parts,
                network,
            )));
            Ok(())
        },
    );
    let Judged {
        constrained,
        slowed,
    } = judged.get();
    assert!(
        slowed > 0,
        "no drawn budget slowed its session beyond the unlimited one, so the run judged no \
         session the budget constrained",
    );
    assert!(
        constrained > 0,
        "no pair held two drawn budgets below the no-cost point, so (a) judged no pair of \
         constrained sessions",
    );
}

/// The first burst of `session` on the latency-only link: the admissions
/// the responder's ledger made within one round trip of `start`, the time
/// of its first depth-1 question in the unlimited session.
///
/// The responder's ledger is the one whose session reported from `side`.
/// Its depth-1 questions travel one way to the initiator, which answers at
/// once, since the link model computes in no time, so no depth-1 reply can
/// reach the responder before one round trip has passed: until then,
/// nothing the budget holds can return to admit a depth-1 question that
/// waited for one.
fn first_burst(session: &Session, side: Option<Side>, start: u64) -> Vec<DatedAdmission> {
    let round_trip = 2 * session.case.link.one_way_delay();
    session
        .admitted
        .ledgers
        .iter()
        .find(|ledger| ledger.side == side)
        .unwrap_or_else(|| panic!("no ledger reported from the responder's side, {side:?}"))
        .dated
        .iter()
        .copied()
        .filter(|admission| admission.at < start + round_trip)
        .collect()
}

/// On the latency-only link, a session's first burst is the unlimited
/// session's at every ledger budget of at least `D₁`, and differs from it
/// at every budget below `D₁`.
///
/// This is the threshold's own property, which the module docs argue under "The
/// threshold": at a budget of at least `D₁` every depth-1 question the
/// responder's root answer asks passes its checks as it is asked, and below
/// `D₁` the check that weighed `D₁` refuses its question. The first burst is
/// every admission the responder's ledger makes before a depth-1 reply can
/// reach it ([`first_burst`]), and two bursts are compared admission by
/// admission, each one's time, unit, grant, and whether it waited or was
/// exempt.
///
/// Each case runs the pair at `D₁` and `D₁ − 1`, which the claim must
/// separate exactly, and at one budget drawn in `[0, D₁ − 1)` and one
/// drawn in `(D₁, N*)`. Neither half may pass vacuously: the unlimited
/// burst must hold depth-1 questions, and `D₁` must lie strictly between
/// zero and the no-cost point, so that every budget drawn lies below the
/// point where nothing waits.
#[test]
fn the_first_burst_saturates_at_the_depth1_claims_total() {
    run(
        None,
        FLEET_DIVISOR,
        concat!(
            module_path!(),
            "::the_first_burst_saturates_at_the_depth1_claims_total"
        ),
        file!(),
        (
            shapes(),
            proptest::sample::select(LINKS.to_vec()),
            proptest::array::uniform::<_, 2>(0..PARTS),
            network_seed(),
        ),
        |(shape, bdp, parts, network): (Shape, u64, [u32; 2], NetworkSeed)| {
            let shape = shape.with_network(network);
            let latency_only = LinkConfig::with_bdp(bdp).latency_only();
            let [reference] = fixture::run_all([case(shape, latency_only, UNLIMITED)]);
            let threshold = depth1_claims(&reference);
            let no_cost = reference.admitted.no_cost_point();
            assert!(
                0 < threshold && threshold < no_cost,
                "the premise fails: the depth-1 claims' total {threshold} must lie strictly \
                 between zero and the no-cost point {no_cost}: {shape:?}",
            );
            let responder = reference
                .admitted
                .ledgers
                .iter()
                .find(|ledger| {
                    ledger
                        .dated
                        .iter()
                        .any(|admission| admission.question && admission.depth == 1)
                })
                .expect("the responder's root answer asks depth-1 questions");
            let start = responder
                .dated
                .iter()
                .find(|admission| admission.question && admission.depth == 1)
                .map(|admission| admission.at)
                .expect("the responder admitted a depth-1 question");
            let unlimited = first_burst(&reference, responder.side, start);
            assert!(
                unlimited
                    .iter()
                    .filter(|admission| admission.question && admission.depth == 1)
                    .count()
                    > 1,
                "the unlimited first burst holds several depth-1 questions: {unlimited:?}",
            );
            let budgets = [
                (between(0, threshold - 1, parts[0]), false),
                (threshold - 1, false),
                (threshold, true),
                (between(threshold + 1, no_cost, parts[1]), true),
            ];
            let sessions =
                fixture::run_all(budgets.map(|(budget, _)| case(shape, latency_only, budget)));
            for ((budget, saturated), session) in budgets.into_iter().zip(&sessions) {
                let burst = first_burst(session, responder.side, start);
                if saturated {
                    assert_eq!(
                        burst, unlimited,
                        "at ledger budget {budget}, at least the depth-1 claims' total \
                         {threshold}, the first burst is the unlimited session's: {shape:?}",
                    );
                } else {
                    assert_ne!(
                        burst, unlimited,
                        "at ledger budget {budget}, below the depth-1 claims' total \
                         {threshold}, the first burst differs from the unlimited session's: \
                         {shape:?}",
                    );
                }
            }
            Ok(())
        },
    );
}
