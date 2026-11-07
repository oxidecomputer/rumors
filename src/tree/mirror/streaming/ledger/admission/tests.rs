use std::cmp::Reverse;
use std::collections::BTreeSet;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::task::{Context, Poll, Wake, Waker};
use std::thread::{self, Thread};
use std::time::{Duration, Instant};

use proptest::collection::vec;
use proptest::prelude::*;

use super::super::{Account, Charge, Ledger};
use super::{
    Admit, Bounds, DEPTHS, KIND_DEPTHS, Pace, Request, Unit, Waiter, admit_waiting, excess_beyond,
    lock,
};
use crate::counter;
use crate::tree::mirror::streaming::holding::Holding;

/// Poll `admit` once, returning its grant if it has one.
fn poll(admit: &mut Admit) -> Option<Charge> {
    match Pin::new(admit).poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(grant) => Some(grant),
        Poll::Pending => None,
    }
}

/// Either kind of unit, for generators.
fn arb_unit() -> impl Strategy<Value = Unit> {
    prop_oneof![Just(Unit::Answer), Just(Unit::Question)]
}

/// Bounds of zero but for the kinds and depths `entries` names, each
/// `(unit, depth, bound)`.
fn bounds(entries: &[(Unit, usize, usize)]) -> Bounds {
    entries
        .iter()
        .fold(Bounds::ZERO, |bounds, &(unit, depth, bound)| {
            bounds.with(unit, depth, bound)
        })
}

/// A ledger in the serial configuration: a budget of zero with every bound
/// zero.
fn serial() -> Ledger {
    Ledger::new(0, Bounds::ZERO)
}

/// Every kind and depth has its own index within the ledger's counts: no
/// two share one, so a change to one kind and depth's `held` never moves
/// another's.
#[test]
fn every_kind_and_depth_has_its_own_index() {
    let mut indices = BTreeSet::new();
    for unit in [Unit::Answer, Unit::Question] {
        for depth in 0..DEPTHS {
            let index = unit.index(depth);
            assert!(
                index < KIND_DEPTHS,
                "{unit:?} at {depth} indexes past the counts"
            );
            assert!(indices.insert(index), "{unit:?} at {depth} shares an index");
        }
    }
}

/// A unit that fits within its kind and depth's bound, beside what it
/// already holds, is exempt: admitted at a budget of zero, adding nothing
/// to the excess.
///
/// One byte beyond the bound needs room, and the kind and depth admits
/// again once a release makes the unit fit.
///
/// With a bound of 100 B at `(answer, 3)`, a 60 B unit and then a 40 B unit
/// fit; a 1 B unit would hold 101 B and waits; dropping the 40 B unit lets
/// it fit beside the 60 B one.
#[test]
fn a_unit_within_its_bound_asks_nothing_of_the_budget() {
    let ledger = Ledger::new(0, bounds(&[(Unit::Answer, 3, 100)]));
    let first = poll(&mut ledger.admit(Unit::Answer, 3, 60)).expect("an empty kind and depth");
    let second = poll(&mut ledger.admit(Unit::Answer, 3, 40)).expect("60 + 40 fits the bound");
    assert_eq!((ledger.held_at(Unit::Answer, 3), ledger.excess()), (100, 0));
    let mut third = ledger.admit(Unit::Answer, 3, 1);
    assert!(
        poll(&mut third).is_none(),
        "101 B exceed the bound, and the budget of zero has no room for 1 B"
    );
    drop(second);
    let third = poll(&mut third).expect("60 + 1 fits the bound");
    assert_eq!(ledger.excess(), 0);
    drop((first, third));
}

/// A unit beyond its bound asks the budget only for what it adds to its
/// kind and depth's excess, and waits exactly while the excess with it
/// added exceeds the budget.
///
/// With a bound of 100 B at `(question, 5)` and a budget of 50 B: a 60 B
/// unit is exempt; an 80 B unit brings the kind and depth to 140 B, an
/// excess of 40 B, which fits; a 20 B unit would bring the excess to 60 B
/// and waits. Dropping the 60 B unit leaves 80 B, so the 20 B unit then
/// fits within the bound beside them and is admitted, exempt.
#[test]
fn a_unit_beyond_its_bound_needs_room_for_its_excess() {
    let ledger = Ledger::new(50, bounds(&[(Unit::Question, 5, 100)]));
    let first = poll(&mut ledger.admit(Unit::Question, 5, 60)).expect("an empty kind and depth");
    let second = poll(&mut ledger.admit(Unit::Question, 5, 80)).expect("an excess of 40 B fits");
    assert_eq!(ledger.excess(), 40);
    let mut third = ledger.admit(Unit::Question, 5, 20);
    assert!(
        poll(&mut third).is_none(),
        "an excess of 60 B exceeds the 50 B budget"
    );
    drop(first);
    let third = poll(&mut third).expect("80 + 20 fits the bound");
    assert_eq!(
        (ledger.held_at(Unit::Question, 5), ledger.excess()),
        (100, 0)
    );
    drop((second, third));
}

/// In the serial configuration a waiting unit is admitted in the critical
/// section that returns the last byte its kind and depth holds, and not
/// before: a part split off a grant holds the kind and depth like the
/// grant.
#[test]
fn a_waiter_is_admitted_as_its_kind_and_depth_empties() {
    let ledger = serial();
    let mut grant = poll(&mut ledger.admit(Unit::Answer, 7, 50)).expect("an empty kind and depth");
    let part = grant.take(Holding::Resolutions, 20);
    let mut next = ledger.admit(Unit::Answer, 7, 50);
    drop(grant);
    assert!(
        poll(&mut next).is_none(),
        "the split part still holds the kind and depth"
    );
    drop(part);
    assert!(
        poll(&mut next).is_some(),
        "the last charge emptied the kind and depth"
    );
}

/// A unit that would fit within its kind and depth's bound waits behind an
/// earlier waiter of its kind and depth, and is admitted only after it.
///
/// With a bound of 100 B at `(answer, 3)` and a budget of zero: a 60 B unit
/// is exempt; a 70 B unit would bring the kind and depth to 130 B and waits
/// for room; a 40 B unit would fit beside the 60 B one, but asked after the
/// 70 B unit, so it waits too. Dropping the 60 B unit admits the 70 B unit,
/// beside which the 40 B unit does not fit, so it is admitted only once the
/// 70 B unit drops.
#[test]
fn a_unit_that_fits_waits_behind_an_earlier_unit_of_its_kind_and_depth() {
    let ledger = Ledger::new(0, bounds(&[(Unit::Answer, 3, 100)]));
    let first = poll(&mut ledger.admit(Unit::Answer, 3, 60)).expect("an empty kind and depth");
    let mut large = ledger.admit(Unit::Answer, 3, 70);
    assert!(
        poll(&mut large).is_none(),
        "130 B exceed the bound, and the budget of zero has no room for 30 B"
    );
    let mut small = ledger.admit(Unit::Answer, 3, 40);
    assert!(
        poll(&mut small).is_none(),
        "60 + 40 fits the bound, but the 70 B unit asked first"
    );
    drop(first);
    let large = poll(&mut large).expect("its kind and depth emptied");
    assert!(poll(&mut small).is_none(), "70 + 40 exceeds the bound");
    drop(large);
    assert!(
        poll(&mut small).is_some(),
        "its kind and depth emptied, and nothing asked before it waits"
    );
}

/// An item built beyond its grant is charged in full to the grant's kind
/// and depth, raising its excess by what falls beyond the bound, and
/// returns in full.
///
/// With a bound of 10 B at `(answer, 1)` and a budget of 100 B: a 10 B
/// grant builds a 15 B item, so the kind and depth holds 15 B, 5 B beyond
/// its bound, and 5 B fell short. A 30 B grant, admitted against the budget,
/// builds a 50 B item: 65 B held, an excess of 55 B, and 25 B short in all.
#[test]
fn an_item_beyond_its_grant_is_counted_in_full() {
    let ledger = Ledger::new(100, bounds(&[(Unit::Answer, 1, 10)]));
    let mut exempt = poll(&mut ledger.admit(Unit::Answer, 1, 10)).expect("an empty kind and depth");
    let exempt_item = exempt.take(Holding::Resolutions, 15);
    assert_eq!(
        (
            ledger.held_at(Unit::Answer, 1),
            ledger.excess(),
            ledger.short()
        ),
        (15, 5, 5)
    );
    let mut budgeted = poll(&mut ledger.admit(Unit::Answer, 1, 30)).expect("fits the budget");
    let item = budgeted.take(Holding::Resolutions, 50);
    assert_eq!(
        (
            ledger.held_at(Unit::Answer, 1),
            ledger.excess(),
            ledger.short()
        ),
        (65, 55, 25)
    );
    drop((exempt, exempt_item, budgeted, item));
    assert_eq!(
        (
            ledger.held_at(Unit::Answer, 1),
            ledger.excess(),
            ledger.held()
        ),
        (0, 0, 0)
    );
}

/// A deeper unit waiting for room closes the budget to every shallower unit
/// that needs room, but never to an exempt one, and a withdrawal reopens
/// it.
///
/// Bounds of 10 B at `(question, 9)`, `(question, 2)` and `(answer, 2)`,
/// and a budget of 100 B: a second depth-9 question of 500 B waits for room;
/// a second depth-2 question of 10 B would fit the budget but waits behind
/// it; a depth-2 answer at an empty kind and depth is admitted past both.
#[test]
fn deeper_work_comes_first_for_the_budget() {
    let ledger = Ledger::new(
        100,
        bounds(&[
            (Unit::Question, 9, 10),
            (Unit::Question, 2, 10),
            (Unit::Answer, 2, 10),
        ]),
    );
    let deep_first = poll(&mut ledger.admit(Unit::Question, 9, 10)).expect("exempt");
    let mut deep = ledger.admit(Unit::Question, 9, 500);
    assert!(poll(&mut deep).is_none(), "an excess of 500 B");
    let shallow_first = poll(&mut ledger.admit(Unit::Question, 2, 10)).expect("exempt");
    let mut shallow = ledger.admit(Unit::Question, 2, 10);
    assert!(
        poll(&mut shallow).is_none(),
        "the waiting deeper unit closes the budget"
    );
    let mut exempt = ledger.admit(Unit::Answer, 2, 10);
    assert!(
        poll(&mut exempt).is_some(),
        "an exempt unit is admitted whatever waits deeper"
    );
    drop(deep);
    assert!(poll(&mut shallow).is_some(), "the deeper unit withdrew");
    drop((deep_first, shallow_first));
}

/// A deeper unit that waits for room needs none of it: once the earlier
/// units of its own kind and depth release theirs, it is exempt, whatever
/// holds the budget.
///
/// With a bound of 500 B at `(question, 9)` and a budget of 100 B, a second
/// 500 B depth-9 question waits for room and closes the budget to a depth-2
/// question that needs 60 B of it. Releasing the first depth-9 question
/// empties its kind and depth, so the waiting one is admitted at step 1,
/// adding no excess, and the depth-2 question is then admitted against the
/// budget in the same critical section.
#[test]
fn a_deeper_waiter_is_admitted_by_its_own_kind_and_depth() {
    let ledger = Ledger::new(
        100,
        bounds(&[(Unit::Question, 9, 500), (Unit::Question, 2, 50)]),
    );
    let deep_first = poll(&mut ledger.admit(Unit::Question, 9, 500)).expect("exempt");
    let mut deep = ledger.admit(Unit::Question, 9, 500);
    assert!(poll(&mut deep).is_none(), "an excess of 500 B");
    let shallow_first = poll(&mut ledger.admit(Unit::Question, 2, 50)).expect("exempt");
    let mut shallow = ledger.admit(Unit::Question, 2, 60);
    assert!(
        poll(&mut shallow).is_none(),
        "the waiting deeper unit closes the budget"
    );
    drop(deep_first);
    let deep = poll(&mut deep).expect("its kind and depth emptied");
    assert_eq!(ledger.excess(), 60, "the depth-2 question's excess alone");
    assert!(poll(&mut shallow).is_some(), "the budget reopened");
    drop((deep, shallow_first));
}

/// An answer's own bytes never hold back its questions.
///
/// At a budget of zero with zero bounds, a question at an empty kind and
/// depth is admitted beside an answer holding any amount, and a later
/// question waits only for the earlier questions of its kind and depth.
#[test]
fn an_answer_never_holds_back_its_questions() {
    let ledger = serial();
    let answer =
        poll(&mut ledger.admit(Unit::Answer, 1, 100_000)).expect("an empty kind and depth");
    let first = poll(&mut ledger.admit(Unit::Question, 2, 1_000)).expect("an empty kind and depth");
    let mut second = ledger.admit(Unit::Question, 2, 1_000);
    assert!(
        poll(&mut second).is_none(),
        "the first question holds its kind and depth"
    );
    drop(first);
    assert!(
        poll(&mut second).is_some(),
        "the earlier question released, while the answer still holds its bytes"
    );
    drop(answer);
}

/// The invitation share never refuses an exempt question, however its
/// depth's pace prices the claim, and a question it refuses becomes exempt
/// once its depth's earlier questions release theirs.
///
/// An answer at depth 4 keeps 1,000,000 B for a finished depth-3 reply
/// predicted to need 1 B, a pace of a million. With a budget of 100 B and
/// zero bounds for questions, a depth-3 question needing 50 B is admitted at
/// its empty kind and depth; a second one passes the room check but its
/// claim, `10 + 10 + 1,000,000 · 50`, does not fit, so it waits for the
/// depth's replies, until the first question's release makes it exempt.
#[test]
fn a_lone_question_is_never_held_back_by_its_share() {
    let ledger = Ledger::new(100, bounds(&[(Unit::Answer, 4, 1_000_000)]));
    let answer = poll(&mut ledger.admit(Unit::Answer, 4, 1_000_000)).expect("exempt");
    ledger.processed(3, 1, false);
    assert_eq!(
        ledger.pace(3),
        Pace {
            actual: 1_000_000,
            predicted: 1
        }
    );
    let first = poll(&mut ledger.invite(3, 10, 50)).expect("exempt at an empty kind and depth");
    assert!(!first.claimed(), "an exempt question claims nothing");
    let mut second = ledger.invite(3, 10, 50);
    assert!(poll(&mut second).is_none(), "its claim does not fit");
    assert_eq!(ledger.excess(), 10, "only the first question's excess");
    drop(first);
    let second = poll(&mut second).expect("exempt once its kind and depth emptied");
    assert!(!second.claimed());
    assert_eq!(ledger.claimed(3), 0);
    drop((answer, second));
}

/// A question under the invitation share waits while its claim does not
/// fit beside its depth's outstanding claims, though its excess fits the
/// budget.
///
/// Its depth's claims shrink only as its stage finishes their replies, not
/// as their room returns, and a shallower unit still takes the budget past
/// the waiting question.
///
/// With a budget of 1,000 B, bounds of 10 B for depth-3 and depth-1
/// questions and 900 B for depth-4 answers, and the pace at one: beside an
/// exempt 10 B question, a 100 B question whose reply needs 900 B claims the
/// whole budget, `100 + 900 = 1,000`. A second such question would bring the
/// depth's claims to 2,000 B, and still to 1,900 B once the first one's room
/// returns, since the first reply is unfinished. Once it finishes, having
/// built a 900 B answer one level down, the pace is `900 / 900`, one, and
/// the second question's claim fits alone.
#[test]
fn a_claim_holds_until_its_reply_finishes() {
    let ledger = Ledger::new(
        1_000,
        bounds(&[
            (Unit::Question, 3, 10),
            (Unit::Question, 1, 10),
            (Unit::Answer, 4, 900),
        ]),
    );
    let exempt = poll(&mut ledger.admit(Unit::Question, 3, 10)).expect("exempt");
    let first = poll(&mut ledger.invite(3, 100, 900)).expect("its claim fits the budget");
    assert!(first.claimed());
    assert_eq!((ledger.excess(), ledger.claimed(3)), (100, 900));
    let mut second = ledger.invite(3, 100, 900);
    assert!(
        poll(&mut second).is_none(),
        "2,000 B of depth-3 claims exceed the 1,000 B budget"
    );
    let shallow_exempt = poll(&mut ledger.admit(Unit::Question, 1, 10)).expect("exempt");
    let shallow = poll(&mut ledger.admit(Unit::Question, 1, 500));
    assert!(
        shallow.is_some(),
        "a unit the share holds back keeps no shallower unit from the budget"
    );
    drop(first);
    assert!(
        poll(&mut second).is_none(),
        "the first reply is unfinished, so its 900 B need still stands"
    );
    let answer = poll(&mut ledger.admit(Unit::Answer, 4, 900)).expect("exempt");
    ledger.processed(3, 900, true);
    let second = poll(&mut second).expect("the second claim fits alone once the first withdrew");
    assert_eq!(
        (ledger.held_at(Unit::Question, 3), ledger.claimed(3)),
        (110, 900),
        "the exempt question and the second one, and the second one's need alone"
    );
    drop((exempt, second, shallow_exempt, shallow, answer));
}

/// The pace starts at one and can exceed it, and above one it holds back a
/// question whose claim fits at a pace of one.
///
/// Before any depth-3 reply finishes, the pace is one. A finished reply
/// predicted to need 300 B whose processing built a 600 B answer one level
/// down sets it to `600 / 300`, two. A 100 B question needing 500 B then
/// claims `100 + 2 · 500 = 1,100` B, over the 1,000 B budget, though
/// `100 + 500` would fit.
#[test]
fn a_pace_above_one_holds_back_what_one_admits() {
    let ledger = Ledger::new(
        1_000,
        bounds(&[(Unit::Question, 3, 10), (Unit::Answer, 4, 600)]),
    );
    assert_eq!(ledger.pace(3), Pace::PREDICTED);
    let exempt = poll(&mut ledger.admit(Unit::Question, 3, 10)).expect("exempt");
    let answer = poll(&mut ledger.admit(Unit::Answer, 4, 600)).expect("exempt");
    ledger.processed(3, 300, false);
    assert_eq!(
        ledger.pace(3),
        Pace {
            actual: 600,
            predicted: 300
        }
    );
    let mut question = ledger.invite(3, 100, 500);
    assert!(
        poll(&mut question).is_none(),
        "at a pace of two the claim is 1,100 B"
    );
    drop((exempt, answer, question));
}

/// Questions whose replies are predicted to need nothing claim only their
/// excess, which the room check already weighs: the invitation share then
/// admits exactly what the budget alone would.
#[test]
fn a_reply_that_needs_nothing_leaves_the_whole_budget() {
    let ledger = Ledger::new(300, bounds(&[(Unit::Question, 4, 10)]));
    let exempt = poll(&mut ledger.admit(Unit::Question, 4, 10)).expect("exempt");
    let first = poll(&mut ledger.invite(4, 200, 0)).expect("within the budget");
    let mut second = ledger.invite(4, 200, 0);
    assert!(
        poll(&mut second).is_none(),
        "an excess of 400 B exceeds the 300 B budget"
    );
    let third = poll(&mut ledger.invite(4, 100, 0));
    assert!(
        third.is_none(),
        "the older unit waiting for room keeps the budget from it"
    );
    drop((exempt, first, second, third));
}

/// A unit no longer awaited withdraws, and a grant it never took returns.
#[test]
fn a_withdrawn_unit_returns_its_grant() {
    let ledger = serial();
    let held = poll(&mut ledger.admit(Unit::Answer, 1, 40)).expect("an empty kind and depth");
    let waiting = ledger.admit(Unit::Answer, 1, 40);
    drop(held);
    drop(waiting);
    assert_eq!(ledger.held(), 0);
    let mut again = ledger.admit(Unit::Answer, 1, 40);
    assert!(
        poll(&mut again).is_some(),
        "the withdrawn grant emptied the kind and depth"
    );
}

/// The depths the admission proptests ask questions under the invitation
/// share at.
///
/// The walk never mixes the two kinds of question at one depth: the root's
/// question and leaf requests are admitted outside the share, every other
/// question under it. The generators keep that structure, asking questions
/// outside the share only at shallower depths.
const SHARED: std::ops::Range<usize> = 2..5;

/// The depths the admission proptests admit units at.
const TESTED: std::ops::Range<usize> = 0..SHARED.end;

/// Every kind and depth the admission proptests admit units at.
fn tested() -> impl Iterator<Item = (Unit, usize)> {
    [Unit::Answer, Unit::Question]
        .into_iter()
        .flat_map(|unit| TESTED.map(move |depth| (unit, depth)))
}

/// A table of bounds for the kinds and depths the proptests admit at, each
/// below the largest grant they ask for, zero included.
fn arb_bounds() -> impl Strategy<Value = Bounds> {
    vec(0..500usize, 2 * TESTED.end).prop_map(|drawn| {
        tested()
            .zip(drawn)
            .fold(Bounds::ZERO, |bounds, ((unit, depth), bound)| {
                bounds.with(unit, depth, bound)
            })
    })
}

/// One operation on a ledger's admissions.
#[derive(Clone, Debug)]
enum Op {
    /// Ask to admit a unit of the kind, at the depth, of the bytes, outside
    /// the invitation share.
    Admit(Unit, usize, usize),
    /// Ask to admit a question at the depth, of the bytes, under the
    /// invitation share with the need.
    Invite(usize, usize, usize),
    /// Finish the reply of the unfinished invitation at the index.
    Finish(usize),
    /// Split this many bytes off the live grant or part at the index.
    Split(usize, usize),
    /// Drop the live grant or part at the index.
    Drop(usize),
    /// Withdraw the waiting unit at the index.
    Withdraw(usize),
}

/// Admissions outside the invitation share of `unit` kind, at `depths`.
fn arb_admit(unit: Unit, depths: std::ops::Range<usize>) -> impl Strategy<Value = Op> {
    (depths, 0..400usize).prop_map(move |(depth, bytes)| Op::Admit(unit, depth, bytes))
}

/// Operations on a ledger's admissions: answers at every tested depth,
/// questions outside the share below [`SHARED`], and questions under it at
/// [`SHARED`].
fn arb_op() -> impl Strategy<Value = Op> {
    prop_oneof![
        arb_admit(Unit::Answer, TESTED),
        arb_admit(Unit::Question, 0..SHARED.start),
        (SHARED, 1..400usize, 0..2_000usize)
            .prop_map(|(depth, bytes, need)| Op::Invite(depth, bytes, need)),
        any::<usize>().prop_map(Op::Finish),
        (any::<usize>(), 0..200usize).prop_map(|(at, bytes)| Op::Split(at, bytes)),
        any::<usize>().prop_map(Op::Drop),
        any::<usize>().prop_map(Op::Withdraw),
    ]
}

/// The bytes a unit of `unit` kind at `depth` asks for: `bytes`, or at most
/// its bound when grants keep `within` their bounds, as the walk's do
/// (premise (P1)).
fn asked(bounds: &Bounds, within: bool, unit: Unit, depth: usize, bytes: usize) -> usize {
    if within {
        bytes.min(bounds.of(unit, depth))
    } else {
        bytes
    }
}

/// A waiting unit, with what the checks need to know of it.
struct Waiting {
    /// Its admission, polled after every operation.
    admit: Admit,
    /// Its kind.
    unit: Unit,
    /// The depth of the scope it concerns.
    depth: usize,
    /// Its grant's bytes.
    bytes: usize,
    /// The need of a question under the invitation share.
    need: Option<usize>,
    /// Its place among the units asked for, in the order asked.
    order: usize,
}

/// An admitted question under the invitation share whose reply is
/// unfinished.
#[derive(Clone, Copy)]
struct Invitation {
    /// The depth of the scope it asks about.
    depth: usize,
    /// Its need, which it claimed if `claimed`.
    need: usize,
    /// Whether it claimed its need: granted against the budget, not
    /// exempt.
    claimed: bool,
}

proptest! {
    /// Through any sequence of admissions, some under the invitation share,
    /// finished replies, splits, drops, and withdrawals, admission keeps its
    /// counts, its bound, and its rule, for any table of bounds.
    ///
    /// An independent tally, from the live charges and the kind and depth
    /// each was granted to, checks after every operation:
    ///
    /// - The ledger's `held[k]` for every kind and depth, its total, and its
    ///   excess, `E = Σ_k max(0, held[k] − F[k])`, equal the tally's.
    /// - Each depth's outstanding claims equal the needs of the unfinished
    ///   invitations whose grants claimed, and only a question under the
    ///   share claims.
    /// - Within each kind and depth, units are granted first come, first
    ///   served: no unit is granted while an earlier unit of its kind and
    ///   depth still waits.
    /// - No unit waits at an empty kind and depth. A unit *first in line*,
    ///   with no earlier unit of its kind and depth waiting, does not fit
    ///   within its bound; and one that would pass the room check, and its
    ///   depth's share check if the share governs it, waits behind a deeper
    ///   or older unit first in line that fails the room check, the only
    ///   kind that closes the budget. A unit behind an earlier one of its
    ///   kind and depth is left undecided, so it may fit.
    ///
    /// When every grant keeps within its bound (premise (P1)), it also
    /// checks the memory bound as two checks, `E ≤ B` and
    /// `L − E ≤ Σ F[k]` over the kinds and depths holding bytes, and the
    /// banker's invariant: at each depth, the excess its questions hold and
    /// its outstanding claims, scaled by the pace its latest claiming
    /// admission read, fit the budget together. Each claiming admission
    /// establishes it; exempt grants add no excess, releases and finished
    /// replies only shrink both sums, and only a finished reply moves the
    /// pace, so it holds after every operation.
    ///
    /// Once everything is dropped, the ledger holds nothing.
    #[test]
    fn admission_keeps_its_counts_its_bound_and_its_rule(
        budget in 0..1_000usize,
        bounds in arb_bounds(),
        within in any::<bool>(),
        ops in vec(arb_op(), 0..80),
    ) {
        let ledger = Ledger::new(budget, bounds);
        // Each live charge with the kind and depth of the unit it was
        // granted to.
        let mut live: Vec<(Unit, usize, Charge)> = Vec::new();
        let mut waiting: Vec<Waiting> = Vec::new();
        let mut unfinished: Vec<Invitation> = Vec::new();
        // The pace each depth's latest claiming admission read.
        let mut admitted_at: [Option<Pace>; DEPTHS] = [None; DEPTHS];
        let mut next = 0;
        for op in ops {
            match op {
                Op::Admit(unit, depth, bytes) => {
                    let bytes = asked(&bounds, within, unit, depth, bytes);
                    waiting.push(Waiting {
                        admit: ledger.admit(unit, depth, bytes),
                        unit,
                        depth,
                        bytes,
                        need: None,
                        order: next,
                    });
                    next += 1;
                }
                Op::Invite(depth, bytes, need) => {
                    let bytes = asked(&bounds, within, Unit::Question, depth, bytes);
                    waiting.push(Waiting {
                        admit: ledger.invite(depth, bytes, need),
                        unit: Unit::Question,
                        depth,
                        bytes,
                        need: Some(need),
                        order: next,
                    });
                    next += 1;
                }
                Op::Finish(at) if !unfinished.is_empty() => {
                    let Invitation { depth, need, claimed } =
                        unfinished.swap_remove(at % unfinished.len());
                    ledger.processed(depth, need, claimed);
                }
                Op::Split(at, bytes) if !live.is_empty() => {
                    let at = at % live.len();
                    let (unit, depth, charge) = &mut live[at];
                    let part = charge.split(Holding::Questions, bytes);
                    let entry = (*unit, *depth, part);
                    live.push(entry);
                }
                Op::Drop(at) if !live.is_empty() => {
                    live.swap_remove(at % live.len());
                }
                Op::Withdraw(at) if !waiting.is_empty() => {
                    waiting.swap_remove(at % waiting.len());
                }
                _ => {}
            }
            let mut still = Vec::new();
            let mut granted = Vec::new();
            for mut unit in waiting.drain(..) {
                match poll(&mut unit.admit) {
                    Some(grant) => {
                        granted.push((unit.unit, unit.depth, unit.order));
                        if let Some(need) = unit.need {
                            unfinished.push(Invitation {
                                depth: unit.depth,
                                need,
                                claimed: grant.claimed(),
                            });
                            if grant.claimed() {
                                admitted_at[unit.depth] = Some(ledger.pace(unit.depth));
                            }
                        } else {
                            prop_assert!(!grant.claimed(), "a unit outside the share claimed");
                        }
                        live.push((unit.unit, unit.depth, grant));
                    }
                    None => still.push(unit),
                }
            }
            waiting = still;
            // Whether a unit of `unit` kind at `depth` asked before `order`
            // still waits.
            let earlier_waits = |unit: Unit, depth: usize, order: usize| -> bool {
                waiting
                    .iter()
                    .any(|other| (other.unit, other.depth) == (unit, depth) && other.order < order)
            };
            for &(unit, depth, order) in &granted {
                prop_assert!(
                    !earlier_waits(unit, depth, order),
                    "a unit was granted while an earlier unit of its kind and depth waits"
                );
            }

            let tally = |unit: Unit, depth: usize| -> usize {
                live.iter()
                    .filter(|(kind, at, _)| (*kind, *at) == (unit, depth))
                    .map(|(.., charge)| charge.bytes())
                    .sum()
            };
            let excess: usize = tested()
                .map(|(unit, depth)| excess_beyond(tally(unit, depth), bounds.of(unit, depth)))
                .sum();
            let within_bounds: usize = tested()
                .map(|(unit, depth)| tally(unit, depth).min(bounds.of(unit, depth)))
                .sum();
            for (unit, depth) in tested() {
                prop_assert_eq!(ledger.held_at(unit, depth), tally(unit, depth));
            }
            let held: usize = live.iter().map(|(.., charge)| charge.bytes()).sum();
            prop_assert_eq!(ledger.held(), held);
            prop_assert_eq!(ledger.excess(), excess);
            prop_assert_eq!(held - excess, within_bounds);
            let claimed = |depth: usize| -> usize {
                unfinished
                    .iter()
                    .filter(|invitation| invitation.depth == depth && invitation.claimed)
                    .map(|invitation| invitation.need)
                    .sum()
            };
            for depth in TESTED {
                prop_assert_eq!(ledger.claimed(depth), claimed(depth));
            }
            if within {
                prop_assert!(excess <= budget, "an excess of {} B over {} B", excess, budget);
                let in_use: usize = tested()
                    .filter(|&(unit, depth)| tally(unit, depth) > 0)
                    .map(|(unit, depth)| bounds.of(unit, depth))
                    .sum();
                prop_assert!(
                    within_bounds <= in_use,
                    "{} B within bounds summing to {} B",
                    within_bounds,
                    in_use
                );
                for depth in TESTED {
                    if let Some(pace) = admitted_at[depth] {
                        let invited = excess_beyond(
                            tally(Unit::Question, depth),
                            bounds.of(Unit::Question, depth),
                        );
                        let sum = invited as u128 + pace.scale(claimed(depth));
                        prop_assert!(
                            sum <= budget as u128,
                            "depth {}'s claims of {} B exceed the {} B budget",
                            depth,
                            sum,
                            budget
                        );
                    }
                }
            }
            // The excess with a waiting unit added, `E′`, which the room
            // check compares with the budget.
            let with_unit = |unit: &Waiting| -> usize {
                let (held, bound) = (tally(unit.unit, unit.depth), bounds.of(unit.unit, unit.depth));
                excess - excess_beyond(held, bound) + excess_beyond(held + unit.bytes, bound)
            };
            let first_in_line =
                |unit: &Waiting| -> bool { !earlier_waits(unit.unit, unit.depth, unit.order) };
            for unit in &waiting {
                let (held, bound) = (tally(unit.unit, unit.depth), bounds.of(unit.unit, unit.depth));
                prop_assert!(held > 0, "a unit waits at an empty kind and depth");
                if !first_in_line(unit) {
                    continue;
                }
                prop_assert!(held + unit.bytes > bound, "a unit first in line within its bound waits");
                let claims_fit = unit.need.is_none_or(|need| {
                    let pace = ledger.pace(unit.depth);
                    excess_beyond(held, bound) as u128
                        + unit.bytes as u128
                        + pace.scale(claimed(unit.depth) + need)
                        <= budget as u128
                });
                if with_unit(unit) <= budget && claims_fit {
                    prop_assert!(
                        waiting.iter().any(|other| first_in_line(other)
                            && (other.depth, Reverse(other.order))
                                > (unit.depth, Reverse(unit.order))
                            && with_unit(other) > budget),
                        "a unit that fits waits behind no deeper or older unit first in line \
                         that does not"
                    );
                }
            }
        }
        drop(waiting);
        drop(live);
        prop_assert_eq!(ledger.held(), 0);
        prop_assert_eq!(ledger.excess(), 0);
    }
}

/// One waiting unit of a generated critical section: its kind, depth,
/// grant, and its need if the invitation share governs it.
type Asking = (Unit, usize, u32, Option<usize>);

/// For each unit of `asking`, in the order a critical section decides them,
/// whether one critical section at `budget` grants it, starting from the
/// state the other arguments give.
///
/// The ledger has `bounds`; each kind and depth in `holding` holds its
/// bytes, and each depth's outstanding claims and pace sums are those
/// `share` gives, as `(claimed, predicted, actual)`. The units of `asking`
/// asked in their order. The critical section is the one every release
/// runs ([`admit_waiting`]).
fn decided(
    budget: usize,
    bounds: Bounds,
    holding: &[(Unit, usize, usize)],
    share: &[(usize, u64, u64)],
    asking: &[Asking],
) -> Vec<bool> {
    let account = Arc::new(Account::new(budget, bounds));
    let mut queue = lock(&account.admission);
    for &(unit, depth, bytes) in holding {
        queue.counts.add(&bounds, Some((unit, depth)), bytes);
    }
    for (depth, &(claimed, predicted, actual)) in share.iter().enumerate() {
        queue.share.claimed[depth] = claimed;
        queue.share.predicted[depth] = predicted;
        queue.share.actual[depth] = actual;
    }
    let waiters: Vec<Arc<Waiter>> = asking
        .iter()
        .zip(0..)
        .map(|(&(unit, depth, bytes, need), order)| {
            Arc::new(Waiter {
                request: Request {
                    unit,
                    depth,
                    bytes,
                    need,
                },
                order,
                state: Mutex::default(),
            })
        })
        .collect();
    queue.waiting.extend(waiters.iter().cloned());
    admit_waiting(&account, queue);
    let mut in_order = waiters;
    in_order.sort_by_key(|waiter| (Reverse(waiter.request.depth), waiter.order));
    let granted = in_order
        .iter()
        .map(|waiter| lock(&waiter.state).grant.is_some())
        .collect();
    // The queue gives up its waiters while this still holds each of them,
    // as an `Admit` does as it withdraws: a waiter dropped inside a later
    // critical section would drop its grant there, under the lock its
    // release takes.
    lock(&account.admission).waiting.clear();
    granted
}

/// A generated unit of a critical section: an answer or a question outside
/// the share at any tested depth, or a question under the share at
/// [`SHARED`].
fn arb_asking() -> impl Strategy<Value = Asking> {
    prop_oneof![
        (TESTED, 0..600u32).prop_map(|(depth, bytes)| (Unit::Answer, depth, bytes, None)),
        (0..SHARED.start, 0..600u32).prop_map(|(depth, bytes)| (
            Unit::Question,
            depth,
            bytes,
            None
        )),
        (SHARED, 0..600u32, 0..2_000usize).prop_map(|(depth, bytes, need)| (
            Unit::Question,
            depth,
            bytes,
            Some(need)
        )),
    ]
}

proptest! {
    /// Admission is monotone in the budget in the sense the budget
    /// criterion uses (claim (M)): a larger budget is the first to grant.
    ///
    /// One critical section, started in one state with the same waiters at
    /// two budgets `B ≤ B′`, makes the same grants up to the first unit, in
    /// the order the section takes them, granted at one budget and not the
    /// other, and that unit is granted at `B′`. Waiters of one kind and
    /// depth exercise the first-come, first-served clause, which leaves the
    /// same units undecided at both budgets until the first difference.
    ///
    /// The state is generated directly: a table of bounds, what each kind
    /// and depth holds, each depth's outstanding claims and pace, and up to
    /// a dozen waiting units of every kind the walk asks, under the share
    /// and outside it. A difference in which `B′` waits for the share where
    /// `B` waits for room changes no grant, so the first difference in
    /// grants is the larger budget's whatever outcomes precede it.
    #[test]
    fn a_larger_budget_first_differs_by_a_grant(
        budget in 0..1_500usize,
        more in 0..1_500usize,
        bounds in arb_bounds(),
        holding in vec((arb_unit(), TESTED, 0..800usize), 0..10),
        share in vec((0..2_000usize, 0..1_000u64, 0..2_000u64), TESTED.end),
        asking in vec(arb_asking(), 0..12),
    ) {
        let smaller = decided(budget, bounds, &holding, &share, &asking);
        let larger = decided(budget + more, bounds, &holding, &share, &asking);
        if let Some(first) = smaller.iter().zip(&larger).position(|(a, b)| a != b) {
            prop_assert!(
                larger[first] && !smaller[first],
                "the first unit granted at one budget alone is granted at the smaller one"
            );
        }
    }
}

/// What one run of a schedule of admissions showed.
#[derive(Debug, PartialEq, Eq)]
struct Run {
    /// Units that waited for admission.
    waited: u64,
    /// For each unit admitted, in order, its place among the units asked
    /// for and whether it claimed under the invitation share.
    grants: Vec<(usize, bool)>,
    /// The ledger's largest excess after any operation: with every grant
    /// within its bound, only grants against the budget raise the excess,
    /// so this is the largest `E′` any room check that admitted weighed.
    excess_peak: usize,
    /// The largest sum any invitation share check weighed.
    share_peak: usize,
}

/// Runs `ops` on a ledger with `budget` and `bounds`, as the admission
/// proptest does with every grant kept within its bound, leaving any unit
/// it does not admit waiting to the end.
fn run(budget: usize, bounds: Bounds, ops: &[Op]) -> Run {
    let ledger = Ledger::new(budget, bounds);
    let mut live: Vec<Charge> = Vec::new();
    let mut waiting: Vec<(usize, Option<(usize, usize)>, Admit)> = Vec::new();
    let mut unfinished: Vec<(usize, usize, bool)> = Vec::new();
    let mut grants = Vec::new();
    let mut excess_peak = 0;
    for (order, op) in ops.iter().cloned().enumerate() {
        match op {
            Op::Admit(unit, depth, bytes) => {
                let bytes = asked(&bounds, true, unit, depth, bytes);
                waiting.push((order, None, ledger.admit(unit, depth, bytes)));
            }
            Op::Invite(depth, bytes, need) => {
                let bytes = asked(&bounds, true, Unit::Question, depth, bytes);
                waiting.push((
                    order,
                    Some((depth, need)),
                    ledger.invite(depth, bytes, need),
                ));
            }
            Op::Finish(at) if !unfinished.is_empty() => {
                let (depth, need, claimed) = unfinished.swap_remove(at % unfinished.len());
                ledger.processed(depth, need, claimed);
            }
            Op::Split(at, bytes) if !live.is_empty() => {
                let at = at % live.len();
                let part = live[at].split(Holding::Questions, bytes);
                live.push(part);
            }
            Op::Drop(at) if !live.is_empty() => {
                live.swap_remove(at % live.len());
            }
            Op::Withdraw(at) if !waiting.is_empty() => {
                waiting.swap_remove(at % waiting.len());
            }
            _ => {}
        }
        let mut still = Vec::new();
        for (order, invitation, mut admit) in waiting.drain(..) {
            match poll(&mut admit) {
                Some(grant) => {
                    grants.push((order, grant.claimed()));
                    if let Some((depth, need)) = invitation {
                        unfinished.push((depth, need, grant.claimed()));
                    }
                    live.push(grant);
                }
                None => still.push((order, invitation, admit)),
            }
        }
        waiting = still;
        excess_peak = excess_peak.max(ledger.excess());
    }
    Run {
        waited: ledger.waited(),
        grants,
        excess_peak,
        share_peak: ledger.share_peak(),
    }
}

proptest! {
    /// A ledger whose budget is a schedule's no-cost point admits every unit
    /// of that schedule exactly as an unlimited ledger does, and one byte
    /// less makes some unit wait.
    ///
    /// The no-cost point is the larger of the unlimited run's peak excess,
    /// `Λ`, and its share peak, the largest sum any of its share checks
    /// weighed. With every grant within its bound, exempt grants add no
    /// excess, so every rise of the excess is a room check's `E′`. At that
    /// budget, every check the unlimited run makes passes in the same state,
    /// so by induction along the schedule no unit waits and every unit is
    /// admitted when, and claims as, it did unlimited. One byte below it, the
    /// first check of the unlimited run that weighed that much fails.
    #[test]
    fn the_no_cost_point_admits_as_an_unlimited_ledger(
        bounds in arb_bounds(),
        ops in vec(arb_op(), 0..80),
    ) {
        let unlimited = run(usize::MAX, bounds, &ops);
        prop_assert_eq!(unlimited.waited, 0);
        let point = unlimited.excess_peak.max(unlimited.share_peak);
        let at_point = run(point, bounds, &ops);
        prop_assert_eq!(&at_point.grants, &unlimited.grants);
        prop_assert_eq!(at_point.waited, 0);
        prop_assert_eq!(at_point.excess_peak, unlimited.excess_peak);
        if point > 0 {
            prop_assert!(
                run(point - 1, bounds, &ops).waited > 0,
                "nothing waits below the no-cost point"
            );
        }
    }
}

/// No unit counts toward two depths' invitation share checks: a question's
/// bytes count toward its own depth's excess, an answer's toward the pace of
/// the depth above it, and nothing else.
///
/// That is what keeps the claims nested. A need prices only the answers one
/// level down, so a question's room and an answer's bytes each enter one
/// depth's sum at most; the root's answers enter none, since no question
/// asks about a scope above the root. Every kind and depth is tried with two
/// units on an unlimited ledger, then a finished reply at every depth, after
/// which what each depth's questions hold and each depth's pace show which
/// depths saw the units.
#[test]
fn no_unit_counts_toward_two_depths_checks() {
    for unit in [Unit::Answer, Unit::Question] {
        for depth in 0..DEPTHS {
            let ledger = Ledger::new(usize::MAX, Bounds::ZERO);
            let first = poll(&mut ledger.admit(unit, depth, 10)).expect("exempt");
            let second = poll(&mut ledger.admit(unit, depth, 100)).expect("unlimited");
            for at in 0..DEPTHS {
                ledger.processed(at, 1, false);
            }
            let seen: Vec<usize> = (0..DEPTHS)
                .filter(|&at| ledger.held_at(Unit::Question, at) > 0 || ledger.pace(at).actual > 0)
                .collect();
            let expected = match unit {
                Unit::Question => vec![depth],
                Unit::Answer => depth.checked_sub(1).into_iter().collect(),
            };
            assert_eq!(
                seen, expected,
                "{unit:?} at depth {depth} counts at depths {seen:?}"
            );
            drop((first, second));
        }
    }
}

/// How long a unit may wait for its kind and depth, which other threads
/// keep emptying, before the handoff test calls the wait lost: far beyond
/// the microseconds each holder keeps it.
const HANDOFF_PATIENCE: Duration = Duration::from_secs(30);

/// A waker that unparks the thread awaiting an admission.
struct Unpark(Thread);

impl Wake for Unpark {
    /// Unpark the awaiting thread.
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
}

/// Await `admit` on this thread, parking between polls, and return its
/// grant.
///
/// # Panics
///
/// Panics if no wake admits the unit within [`HANDOFF_PATIENCE`].
fn await_admission(mut admit: Admit) -> Charge {
    let waker = Waker::from(Arc::new(Unpark(thread::current())));
    let deadline = Instant::now() + HANDOFF_PATIENCE;
    loop {
        if let Poll::Ready(grant) = Pin::new(&mut admit).poll(&mut Context::from_waker(&waker)) {
            return grant;
        }
        let now = Instant::now();
        assert!(
            now < deadline,
            "a unit waited at a kind and depth that kept emptying: a lost wakeup"
        );
        thread::park_timeout(deadline - now);
    }
}

/// In the serial configuration, units of one kind and depth on several
/// threads hold it in turn, without a lost wakeup.
///
/// Each is admitted only in the critical section that returns the last
/// byte its predecessor's charges held.
///
/// Each holder splits its grant, drops one part itself and sends the other
/// to a thread that drops it, so the kind and depth empties on whichever
/// thread releases its last charge. No two holders overlap, and every unit
/// is admitted.
///
/// Each holder finds the ledger holding exactly its own grant. A drop
/// returns its bytes and decides the waiters under the admission queue's
/// lock, and the next grant is made under that lock and reaches its holder
/// through it, so the total the holder reads is ordered by the lock alone:
/// had a unit been admitted outside the critical section that returned its
/// predecessor's last bytes, it could find them still held.
#[test]
fn units_pass_between_threads_and_wait_for_the_last_byte() {
    const THREADS: usize = 8;
    const ROUNDS: usize = 2_000;
    const GRANT: usize = 64;
    const PART: usize = 16;
    let ledger = serial();
    let holders = AtomicUsize::new(0);
    let (parts, dropper) = mpsc::channel::<Charge>();
    thread::scope(|scope| {
        scope.spawn(move || dropper.into_iter().for_each(drop));
        for _ in 0..THREADS {
            let parts = parts.clone();
            let (ledger, holders) = (&ledger, &holders);
            scope.spawn(move || {
                for _ in 0..ROUNDS {
                    let mut grant = await_admission(ledger.admit(Unit::Question, 4, GRANT));
                    assert_eq!(holders.fetch_add(1, Ordering::SeqCst), 0, "two holders");
                    assert_eq!(
                        ledger.held(),
                        GRANT,
                        "the last holder's bytes are still held"
                    );
                    let part = grant.take(Holding::Questions, PART);
                    drop(grant);
                    holders.fetch_sub(1, Ordering::SeqCst);
                    parts.send(part).expect("the dropper outlives every holder");
                }
            });
        }
        drop(parts);
    });
    assert_eq!(ledger.held(), 0);
}

/// One operation on the bytes the pace reads.
#[derive(Clone, Debug)]
enum PaceOp {
    /// Admit a unit of the kind at the depth with a grant of the bytes.
    Admit(Unit, usize, usize),
    /// Build an item of this many bytes out of the live grant at the index.
    Take(usize, usize),
    /// Keep only this many bytes of the live grant at the index, returning
    /// the rest unused.
    Keep(usize, usize),
    /// Drop the live grant at the index, returning what it still holds.
    Drop(usize),
    /// Note that a reply to a question at the depth finished processing,
    /// with the need.
    Processed(usize, usize),
}

/// Operations on the bytes the pace reads, at depths around the ones it
/// compares.
fn arb_pace_op() -> impl Strategy<Value = PaceOp> {
    prop_oneof![
        (arb_unit(), 0..6usize, 0..1_000usize)
            .prop_map(|(unit, depth, bytes)| PaceOp::Admit(unit, depth, bytes)),
        (any::<usize>(), 0..400usize).prop_map(|(at, bytes)| PaceOp::Take(at, bytes)),
        (any::<usize>(), 0..400usize).prop_map(|(at, bytes)| PaceOp::Keep(at, bytes)),
        any::<usize>().prop_map(PaceOp::Drop),
        (0..6usize, 0..5_000usize).prop_map(|(depth, need)| PaceOp::Processed(depth, need)),
    ]
}

proptest! {
    /// The invitation share's pace at a depth is the ratio of what the
    /// answers one level down had kept as its latest reply finished to the
    /// needs of its finished replies, and one before any finished.
    ///
    /// Both sums are read at each finish. An answer's grant counts in full
    /// as it is made, less what it returns unused, whether through
    /// [`Charge::keep`] or by dropping with bytes left; bytes built into
    /// items stay kept after the items drop. A question's grant counts
    /// toward no pace: the questions processing sends two levels down claim
    /// at their own depth.
    #[test]
    fn the_pace_is_the_ratio_of_kept_to_predicted(ops in vec(arb_pace_op(), 0..80)) {
        let ledger = Ledger::new(usize::MAX, Bounds::ZERO);
        let mut grants: Vec<(Unit, usize, Charge)> = Vec::new();
        let mut items = Vec::new();
        let mut kept = [0usize; DEPTHS];
        let mut predicted = [0usize; DEPTHS];
        let mut actual = [0usize; DEPTHS];
        let answers = |unit: Unit, bytes: usize| if unit == Unit::Answer { bytes } else { 0 };
        for op in ops {
            match op {
                PaceOp::Admit(unit, depth, bytes) => {
                    let grant = poll(&mut ledger.admit(unit, depth, bytes))
                        .expect("an unlimited ledger admits at once");
                    kept[depth] += answers(unit, bytes);
                    grants.push((unit, depth, grant));
                }
                PaceOp::Take(at, bytes) if !grants.is_empty() => {
                    let at = at % grants.len();
                    let (.., grant) = &mut grants[at];
                    let bytes = bytes.min(grant.bytes());
                    items.push(grant.take(Holding::Resolutions, bytes));
                }
                PaceOp::Keep(at, bytes) if !grants.is_empty() => {
                    let at = at % grants.len();
                    let (unit, depth, grant) = &mut grants[at];
                    kept[*depth] -= answers(*unit, grant.bytes().saturating_sub(bytes));
                    grant.keep(bytes);
                }
                PaceOp::Drop(at) if !grants.is_empty() => {
                    let (unit, depth, grant) = grants.swap_remove(at % grants.len());
                    kept[depth] -= answers(unit, grant.bytes());
                }
                PaceOp::Processed(depth, need) => {
                    ledger.processed(depth, need, false);
                    predicted[depth] += need;
                    actual[depth] = kept.get(depth + 1).copied().unwrap_or(0);
                }
                _ => {}
            }
            for (depth, &predicted) in predicted.iter().enumerate() {
                let expected = if predicted == 0 {
                    Pace::PREDICTED
                } else {
                    Pace {
                        actual: counter::widen(actual[depth]),
                        predicted: counter::widen(predicted),
                    }
                };
                prop_assert_eq!(ledger.pace(depth), expected, "at depth {}", depth);
            }
        }
        drop(items);
    }
}
