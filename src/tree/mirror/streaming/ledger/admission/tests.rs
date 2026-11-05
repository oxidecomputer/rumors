use std::collections::BTreeSet;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, mpsc};
use std::task::{Context, Poll, Wake, Waker};
use std::thread::{self, Thread};
use std::time::{Duration, Instant};

use proptest::collection::vec;
use proptest::prelude::*;

use super::super::{Charge, Ledger};
use super::{Admit, DEPTHS, Pace, SLOTS, Slot, Unit};
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

/// Every kind and depth names its own floor slot, within the ledger's
/// counts: no two slots share a count, so taking one never takes another,
/// and each slot names back the kind and depth it serves.
#[test]
fn every_slot_has_its_own_index() {
    let mut indices = BTreeSet::new();
    for unit in [Unit::Answer, Unit::Question] {
        for depth in 0..DEPTHS {
            let slot = Slot::new(unit, depth);
            let index = slot.index();
            assert!(index < SLOTS, "{unit:?} at {depth} indexes past the counts");
            assert!(indices.insert(index), "{unit:?} at {depth} shares an index");
            assert_eq!(slot.serves(), (unit, depth));
        }
    }
}

/// A unit at a free floor slot takes it, and its bytes join the floor part,
/// even when the budget has room for it.
#[test]
fn a_free_floor_slot_comes_first() {
    let ledger = Ledger::new(usize::MAX);
    let mut admit = ledger.admit(Unit::Answer, 3, 100);
    let grant = poll(&mut admit).expect("a free slot admits at once");
    assert!(grant.is_floor());
    assert_eq!(ledger.floor(), 100);
}

/// With its slot taken, a unit is admitted against the budget exactly when
/// it fits beside what the ledger holds outside its floor slots.
#[test]
fn a_taken_slot_leaves_the_budget() {
    let ledger = Ledger::new(250);
    let floor = poll(&mut ledger.admit(Unit::Question, 5, 1_000)).expect("free slot");
    let first = poll(&mut ledger.admit(Unit::Question, 5, 200)).expect("fits the budget");
    assert!(!first.is_floor());
    let mut second = ledger.admit(Unit::Question, 5, 100);
    assert!(
        poll(&mut second).is_none(),
        "200 + 100 exceeds the 250 B budget"
    );
    drop(first);
    assert!(poll(&mut second).is_some(), "the release makes room");
    drop(floor);
}

/// A slot frees only when the last charge built out of its grant drops,
/// and then admits the unit waiting for it.
#[test]
fn a_slot_frees_with_its_last_charge() {
    let ledger = Ledger::new(0);
    let mut grant = poll(&mut ledger.admit(Unit::Answer, 7, 50)).expect("free slot");
    let part = grant.take(Holding::Resolutions, 20);
    let mut next = ledger.admit(Unit::Answer, 7, 50);
    drop(grant);
    assert!(
        poll(&mut next).is_none(),
        "the split part still holds the slot"
    );
    drop(part);
    assert!(poll(&mut next).is_some(), "the last charge freed the slot");
}

/// A slot that frees while a unit of its kind and depth holds budget
/// carries that unit's bytes, returning the budget's room at once.
///
/// A unit waiting for the room is admitted, and the slot stays taken until
/// as many bytes of its kind and depth have returned.
///
/// With a 100 B budget, one depth-1 answer takes its slot with 10 B and a
/// second is admitted against the budget with 60 B. A depth-2 question of
/// 50 B, its own slot taken, then waits, since `60 + 50` exceeds the budget.
/// The first answer's release frees the depth-1 answer slot, which carries
/// the second answer's 60 B: the budget counts none of them, and the
/// question is admitted against it. A third depth-1 answer finds the slot
/// taken and goes to the budget. Its 10 B return to the carry first, since
/// every byte of the slot's kind and depth does, and the slot frees once the
/// second answer has returned 50 B more, for a fourth answer waiting for it,
/// with nothing of its kind and depth left against the budget.
#[test]
fn a_freed_slot_carries_what_its_kind_holds_against_the_budget() {
    let ledger = Ledger::new(100);
    let first = poll(&mut ledger.admit(Unit::Answer, 1, 10)).expect("free slot");
    let mut second = poll(&mut ledger.admit(Unit::Answer, 1, 60)).expect("fits the budget");
    assert!(!second.is_floor());
    let question_slot = poll(&mut ledger.admit(Unit::Question, 2, 10)).expect("free slot");
    let mut question = ledger.admit(Unit::Question, 2, 50);
    assert!(
        poll(&mut question).is_none(),
        "60 + 50 exceeds the 100 B budget"
    );
    drop(first);
    assert_eq!(ledger.carried(Unit::Answer, 1), 60);
    let question = poll(&mut question).expect("the carry returned the budget's room");
    assert!(!question.is_floor());
    assert_eq!(ledger.normal(), 50);
    let third = poll(&mut ledger.admit(Unit::Answer, 1, 10)).expect("fits the budget");
    assert!(!third.is_floor(), "a carrying slot is taken");
    assert_eq!(ledger.normal(), 60);
    drop(third);
    assert_eq!(
        (ledger.carried(Unit::Answer, 1), ledger.normal()),
        (50, 60),
        "the third answer's bytes returned to the carry"
    );
    let part = second.split(Holding::Resolutions, 20);
    drop(second);
    assert_eq!(ledger.carried(Unit::Answer, 1), 10);
    let mut next = ledger.admit(Unit::Answer, 1, 200);
    assert!(
        poll(&mut next).is_none(),
        "the slot still carries, and 200 B exceed the budget"
    );
    drop(part);
    assert_eq!(ledger.carried(Unit::Answer, 1), 0);
    let next = poll(&mut next).expect("the carry's last bytes freed the slot");
    assert!(next.is_floor());
    assert_eq!(ledger.budgeted(Unit::Answer, 1), 0);
    drop((question_slot, question, next));
    assert_eq!(ledger.held(), 0);
}

/// An item built beyond its grant is charged in full where its grant is
/// counted.
///
/// A floor grant's shortfall joins the floor part, and a budget grant's
/// joins the budget's count and its kind and depth's bytes against the
/// budget, which a slot can then carry and which return in full.
///
/// A depth-1 answer takes its slot with 10 B and builds a 15 B item out of
/// it, so the budget counts nothing. A second one, admitted against a 100 B
/// budget with 30 B, builds a 50 B item: the budget counts 50 B, all of them
/// its kind and depth's, and 25 B fell short in all. The first answer's
/// charges drop, so its slot frees and carries 30 B, the largest grant of
/// its kind and depth, leaving 20 B to the budget. The 50 B item's drop
/// returns 30 B to the carry and the other 20 B to the budget, which then
/// counts nothing, and the slot frees.
#[test]
fn an_item_beyond_its_grant_is_counted_in_full() {
    let ledger = Ledger::new(100);
    let mut floor = poll(&mut ledger.admit(Unit::Answer, 1, 10)).expect("free slot");
    let floor_item = floor.take(Holding::Resolutions, 15);
    assert_eq!((ledger.floor(), ledger.normal()), (15, 0));
    let mut budgeted = poll(&mut ledger.admit(Unit::Answer, 1, 30)).expect("fits the budget");
    assert!(!budgeted.is_floor());
    let item = budgeted.take(Holding::Resolutions, 50);
    assert_eq!(
        (
            ledger.normal(),
            ledger.budgeted(Unit::Answer, 1),
            ledger.short()
        ),
        (50, 50, 25)
    );
    drop((floor, floor_item));
    assert_eq!(
        (ledger.carried(Unit::Answer, 1), ledger.normal()),
        (30, 20),
        "the freed slot carries the largest grant's worth"
    );
    drop((budgeted, item));
    assert_eq!(
        (
            ledger.carried(Unit::Answer, 1),
            ledger.normal(),
            ledger.budgeted(Unit::Answer, 1),
            ledger.held()
        ),
        (0, 0, 0, 0)
    );
    let next = poll(&mut ledger.admit(Unit::Answer, 1, 10)).expect("free slot");
    assert!(next.is_floor(), "the carry's last bytes freed the slot");
}

/// A slot that frees while a unit of its kind and depth waits for it goes
/// to that unit, and carries nothing, whatever its kind and depth hold
/// against the budget.
#[test]
fn a_waiting_unit_takes_a_freed_slot_before_any_carry() {
    let ledger = Ledger::new(100);
    let first = poll(&mut ledger.admit(Unit::Question, 3, 10)).expect("free slot");
    let budgeted = poll(&mut ledger.admit(Unit::Question, 3, 60)).expect("fits the budget");
    let mut waiting = ledger.admit(Unit::Question, 3, 80);
    assert!(
        poll(&mut waiting).is_none(),
        "60 + 80 exceeds the 100 B budget"
    );
    drop(first);
    let waiting = poll(&mut waiting).expect("the freed slot admits the waiting unit");
    assert!(waiting.is_floor());
    assert_eq!(ledger.carried(Unit::Question, 3), 0);
    assert_eq!(ledger.normal(), 60);
    drop((budgeted, waiting));
}

/// A waiting deeper unit closes the budget to shallower ones, but never a
/// shallower unit's own free floor slot.
#[test]
fn deeper_work_comes_first_for_the_budget() {
    let ledger = Ledger::new(100);
    let deep_floor = poll(&mut ledger.admit(Unit::Question, 9, 10)).expect("free slot");
    let mut deep = ledger.admit(Unit::Question, 9, 500);
    assert!(poll(&mut deep).is_none(), "over the budget, its slot taken");
    let shallow_floor = poll(&mut ledger.admit(Unit::Question, 2, 10)).expect("free slot");
    let mut shallow = ledger.admit(Unit::Question, 2, 10);
    assert!(
        poll(&mut shallow).is_none(),
        "the waiting deeper unit holds the budget"
    );
    let mut own_slot = ledger.admit(Unit::Answer, 2, 10);
    assert!(
        poll(&mut own_slot).is_some(),
        "a free floor slot admits whatever waits deeper"
    );
    drop(deep);
    assert!(poll(&mut shallow).is_some(), "the deeper unit withdrew");
    drop((deep_floor, shallow_floor));
}

/// A question under the invitation share waits while its claim does not
/// fit beside its depth's outstanding claims, though its grant fits the
/// budget.
///
/// Its depth's claims shrink only as its stage finishes their replies, not
/// as their room returns, and a shallower unit still takes the budget past
/// the waiting question.
///
/// With a budget of 1,000 B and the pace at one, a 100 B question whose
/// reply needs 900 B claims the whole budget: `100 + 900 = 1,000`. A second
/// such question would bring the depth's claims to 2,000 B, and still to
/// 1,900 B once the first one's room returns, since the first reply is
/// unfinished. Once it finishes, having built a 900 B answer one level
/// down, the pace is `900 / 900`, one, and the second question's claim fits
/// alone.
#[test]
fn a_claim_holds_until_its_reply_finishes() {
    let ledger = Ledger::new(1_000);
    let slot = poll(&mut ledger.admit(Unit::Question, 3, 10)).expect("free slot");
    let first = poll(&mut ledger.invite(3, 100, 900)).expect("its claim fits the budget");
    assert!(!first.is_floor());
    assert_eq!(
        (ledger.budgeted(Unit::Question, 3), ledger.claimed(3)),
        (100, 900)
    );
    let mut second = ledger.invite(3, 100, 900);
    assert!(
        poll(&mut second).is_none(),
        "2,000 B of depth-3 claims exceed the 1,000 B budget"
    );
    let shallow_slot = poll(&mut ledger.admit(Unit::Question, 1, 10)).expect("free slot");
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
    let answer = poll(&mut ledger.admit(Unit::Answer, 4, 900)).expect("free slot");
    ledger.processed(3, 900, false);
    let second = poll(&mut second).expect("the second claim fits alone once the first withdrew");
    assert_eq!(
        (ledger.budgeted(Unit::Question, 3), ledger.claimed(3)),
        (100, 900),
        "the second question's room and need alone"
    );
    drop((slot, second, shallow_slot, shallow, answer));
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
    let ledger = Ledger::new(1_000);
    assert_eq!(ledger.pace(3), Pace::PREDICTED);
    let slot = poll(&mut ledger.admit(Unit::Question, 3, 10)).expect("free slot");
    let answer = poll(&mut ledger.admit(Unit::Answer, 4, 600)).expect("free slot");
    ledger.processed(3, 300, true);
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
    drop((slot, answer, question));
}

/// Questions whose replies are predicted to need nothing claim only their
/// room, which the budget already counts: the invitation share then admits
/// exactly what the budget alone would.
#[test]
fn a_reply_that_needs_nothing_leaves_the_whole_budget() {
    let ledger = Ledger::new(300);
    let slot = poll(&mut ledger.admit(Unit::Question, 4, 10)).expect("free slot");
    let first = poll(&mut ledger.invite(4, 200, 0)).expect("within the budget");
    let mut second = ledger.invite(4, 200, 0);
    assert!(
        poll(&mut second).is_none(),
        "400 B exceeds the 300 B budget"
    );
    let third = poll(&mut ledger.invite(4, 100, 0));
    assert!(
        third.is_none(),
        "the older unit waiting for room keeps the budget from it"
    );
    drop((slot, first, second, third));
}

/// A unit no longer awaited withdraws, and a grant it never took returns.
#[test]
fn a_withdrawn_unit_returns_its_grant() {
    let ledger = Ledger::new(0);
    let held = poll(&mut ledger.admit(Unit::Answer, 1, 40)).expect("free slot");
    let waiting = ledger.admit(Unit::Answer, 1, 40);
    drop(held);
    drop(waiting);
    assert_eq!(ledger.held(), 0);
    let mut again = ledger.admit(Unit::Answer, 1, 40);
    assert!(
        poll(&mut again).is_some(),
        "the withdrawn grant freed the slot"
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
    /// Fold the live part at the second index into the one at the first,
    /// when both are parts of one grant for one place.
    Absorb(usize, usize),
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
        (any::<usize>(), any::<usize>()).prop_map(|(into, from)| Op::Absorb(into, from)),
    ]
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
    /// Its need, which it claims unless it took its floor slot.
    need: usize,
    /// Whether it took its depth's floor slot, and so claims nothing.
    floor: bool,
}

proptest! {
    /// Through any sequence of admissions, some under the invitation share,
    /// finished replies, splits, folds, drops, and withdrawals, admission
    /// keeps its bound and its rule.
    ///
    /// - The ledger holds at most its budget outside its floor slots, the
    ///   bytes its slots carry for others excepted, and each floor slot
    ///   serves at most one unit at a time, its carry counting as one.
    /// - The count the budget is checked against, a location of its own,
    ///   equals the bytes the live charges outside floor slots hold, less
    ///   what the slots carry: the difference of the totals it is part of.
    /// - Each kind and depth's bytes held against the budget agree with the
    ///   live charges, and each depth's outstanding claims with the needs of
    ///   the unfinished invitations admitted against the budget.
    /// - A slot carries no more than its kind and depth hold against the
    ///   budget, nor more than the largest grant admitted to one of their
    ///   units, and a free slot leaves nothing of its kind and depth held
    ///   against the budget: a slot freeing while its kind and depth hold
    ///   budget carries their bytes.
    /// - The banker's invariant: at each depth, the room its questions hold
    ///   against the budget, less what their slot carries, and its
    ///   outstanding claims, scaled by the pace its latest
    ///   admission under the share read, fit the budget together. Each such
    ///   admission establishes it; releases and finished replies only
    ///   shrink both sums, and only a finished reply moves the pace, so it
    ///   holds after every operation.
    /// - No unit waits that the rule would admit: every waiting unit's slot
    ///   is taken or carrying, and one that would fit the budget, and its
    ///   depth's claims
    ///   if the share governs it, waits behind a deeper or older waiting
    ///   unit that does not fit the budget, the only kind that keeps others
    ///   from it.
    ///
    /// Once everything is dropped, the ledger holds nothing.
    #[test]
    fn admission_keeps_its_rule_and_its_bound(
        budget in 0..1_000usize,
        ops in vec(arb_op(), 0..80),
    ) {
        let ledger = Ledger::new(budget);
        // Each live charge with the unit it was granted to, by order, and
        // its place.
        let mut live: Vec<(usize, Unit, usize, Holding, Charge)> = Vec::new();
        let mut waiting: Vec<Waiting> = Vec::new();
        let mut unfinished: Vec<Invitation> = Vec::new();
        // The pace each depth's latest admission under the share read.
        let mut admitted_at: [Option<Pace>; DEPTHS] = [None; DEPTHS];
        // The largest grant admitted to each kind and depth.
        let mut largest = std::collections::BTreeMap::<(Unit, usize), usize>::new();
        let mut next = 0;
        for op in ops {
            match op {
                Op::Admit(unit, depth, bytes) => {
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
                    let Invitation { depth, need, floor } =
                        unfinished.swap_remove(at % unfinished.len());
                    ledger.processed(depth, need, floor);
                }
                Op::Split(at, bytes) if !live.is_empty() => {
                    let at = at % live.len();
                    let (order, unit, depth, _, charge) = &mut live[at];
                    let part = charge.split(Holding::Questions, bytes);
                    let entry = (*order, *unit, *depth, Holding::Questions, part);
                    live.push(entry);
                }
                Op::Absorb(into, from) if live.len() > 1 => {
                    let (into, from) = (into % live.len(), from % live.len());
                    let (same_grant, same_place) =
                        (live[into].0 == live[from].0, live[into].3 == live[from].3);
                    if into != from && same_grant && same_place {
                        let (.., charge) = live.swap_remove(from);
                        let into = if into == live.len() { from } else { into };
                        live[into].4.absorb(charge);
                    }
                }
                Op::Drop(at) if !live.is_empty() => {
                    live.swap_remove(at % live.len());
                }
                Op::Withdraw(at) if !waiting.is_empty() => {
                    waiting.swap_remove(at % waiting.len());
                }
                _ => {}
            }
            // Every grant this operation made is waiting to be polled, and
            // one under the share was decided at the pace it leaves, since
            // only `Finish` moves a pace, before it reconsiders.
            let mut still = Vec::new();
            for mut unit in waiting.drain(..) {
                match poll(&mut unit.admit) {
                    Some(grant) => {
                        if let Some(need) = unit.need {
                            let floor = grant.is_floor();
                            unfinished.push(Invitation { depth: unit.depth, need, floor });
                            if !floor {
                                admitted_at[unit.depth] = Some(ledger.pace(unit.depth));
                            }
                        }
                        let most = largest.entry((unit.unit, unit.depth)).or_default();
                        *most = (*most).max(grant.bytes());
                        live.push((unit.order, unit.unit, unit.depth, unit.unit.place(), grant));
                    }
                    None => still.push(unit),
                }
            }
            waiting = still;

            let normal: usize = live
                .iter()
                .filter(|(.., charge)| !charge.is_floor())
                .map(|(.., charge)| charge.bytes())
                .sum();
            prop_assert_eq!(ledger.held() - ledger.floor(), normal);
            let kinds = || [Unit::Answer, Unit::Question].into_iter().flat_map(|unit| TESTED.map(move |depth| (unit, depth)));
            let carried: usize = kinds().map(|(unit, depth)| ledger.carried(unit, depth)).sum();
            prop_assert_eq!(ledger.normal(), normal - carried);
            prop_assert!(
                normal - carried <= budget,
                "{} B outside the floor over {} B",
                normal - carried,
                budget
            );
            let budgeted = |unit: Unit, depth: usize| -> usize {
                live.iter()
                    .filter(|(_, kind, at, _, charge)| {
                        *kind == unit && *at == depth && !charge.is_floor()
                    })
                    .map(|(.., charge)| charge.bytes())
                    .sum()
            };
            let mut serving = std::collections::BTreeMap::new();
            for (order, unit, depth, _, charge) in &live {
                if charge.is_floor() {
                    let holder = serving.entry((*unit, *depth)).or_insert(*order);
                    prop_assert_eq!(*holder, *order, "two units share one floor slot");
                }
            }
            for (unit, depth) in kinds() {
                let (held, carry) = (budgeted(unit, depth), ledger.carried(unit, depth));
                prop_assert_eq!(ledger.budgeted(unit, depth), held);
                prop_assert!(carry <= held, "a slot carries {} B of {} B held", carry, held);
                let most = largest.get(&(unit, depth)).copied().unwrap_or(0);
                prop_assert_eq!(ledger.largest(unit, depth), most);
                prop_assert!(carry <= most, "a slot carries {} B, over the largest grant {} B", carry, most);
                let taken = serving.contains_key(&(unit, depth));
                prop_assert!(!(taken && carry > 0), "a slot serves a unit while it carries");
                prop_assert!(
                    taken || carry > 0 || held == 0,
                    "{:?} at depth {} holds {} B against the budget beside its free slot",
                    unit,
                    depth,
                    held
                );
            }
            let invited = |depth: usize| -> usize {
                budgeted(Unit::Question, depth) - ledger.carried(Unit::Question, depth)
            };
            let claimed = |depth: usize| -> usize {
                unfinished
                    .iter()
                    .filter(|invitation| invitation.depth == depth && !invitation.floor)
                    .map(|invitation| invitation.need)
                    .sum()
            };
            for depth in TESTED {
                prop_assert_eq!(ledger.claimed(depth), claimed(depth));
                if let Some(pace) = admitted_at[depth] {
                    let sum = invited(depth) as u128 + pace.scale(claimed(depth));
                    prop_assert!(
                        sum <= budget as u128,
                        "depth {}'s claims of {} B exceed the {} B budget",
                        depth,
                        sum,
                        budget
                    );
                }
            }
            let normal = normal - carried;
            for unit in &waiting {
                prop_assert!(
                    serving.contains_key(&(unit.unit, unit.depth))
                        || ledger.carried(unit.unit, unit.depth) > 0,
                    "a unit waits at a free floor slot"
                );
                let claims_fit = unit.need.is_none_or(|need| {
                    let pace = ledger.pace(unit.depth);
                    invited(unit.depth) as u128
                        + unit.bytes as u128
                        + pace.scale(claimed(unit.depth) + need)
                        <= budget as u128
                });
                if normal + unit.bytes <= budget && claims_fit {
                    prop_assert!(
                        waiting.iter().any(|other| (other.depth, std::cmp::Reverse(other.order))
                            > (unit.depth, std::cmp::Reverse(unit.order))
                            && normal + other.bytes > budget),
                        "a unit that fits waits behind no deeper or older unit that does not"
                    );
                }
            }
        }
        drop(waiting);
        drop(live);
        prop_assert_eq!(ledger.held(), 0);
    }
}

/// What one run of a schedule of admissions showed.
#[derive(Debug, PartialEq, Eq)]
struct Run {
    /// Units that waited for admission.
    waited: u64,
    /// For each unit admitted, in order, whether it took its floor slot.
    floors: Vec<bool>,
    /// The most the ledger held outside its floor slots after any
    /// operation: every rise is one admission, so this is the ledger's peak
    /// there.
    normal_peak: usize,
    /// The largest sum any invitation share check weighed.
    share_peak: usize,
}

/// Runs `ops` on a ledger with `budget`, as the admission proptest does,
/// leaving any unit it does not admit waiting to the end.
fn run(budget: usize, ops: &[Op]) -> Run {
    let ledger = Ledger::new(budget);
    let mut live: Vec<(usize, Charge)> = Vec::new();
    let mut waiting: Vec<(usize, Option<(usize, usize)>, Admit)> = Vec::new();
    let mut unfinished: Vec<(usize, usize, bool)> = Vec::new();
    let mut floors = Vec::new();
    let mut normal_peak = 0;
    for (order, op) in ops.iter().cloned().enumerate() {
        match op {
            Op::Admit(unit, depth, bytes) => {
                waiting.push((order, None, ledger.admit(unit, depth, bytes)));
            }
            Op::Invite(depth, bytes, need) => {
                waiting.push((
                    order,
                    Some((depth, need)),
                    ledger.invite(depth, bytes, need),
                ));
            }
            Op::Finish(at) if !unfinished.is_empty() => {
                let (depth, need, floor) = unfinished.swap_remove(at % unfinished.len());
                ledger.processed(depth, need, floor);
            }
            Op::Split(at, bytes) if !live.is_empty() => {
                let at = at % live.len();
                let part = live[at].1.split(Holding::Questions, bytes);
                live.push((live[at].0, part));
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
                    floors.push(grant.is_floor());
                    if let Some((depth, need)) = invitation {
                        unfinished.push((depth, need, grant.is_floor()));
                    }
                    live.push((order, grant));
                }
                None => still.push((order, invitation, admit)),
            }
        }
        waiting = still;
        normal_peak = normal_peak.max(ledger.normal());
    }
    Run {
        waited: ledger.waited(),
        floors,
        normal_peak,
        share_peak: ledger.share_peak(),
    }
}

proptest! {
    /// A ledger whose budget is a schedule's no-cost point admits every unit
    /// of that schedule exactly as an unlimited ledger does, and one byte
    /// less makes some unit wait.
    ///
    /// The no-cost point is the larger of the unlimited run's peak outside
    /// its floor slots, `Λ`, and its share peak, the largest sum any of its
    /// share checks weighed. At that budget, every check the unlimited run
    /// makes passes in the same state, so by induction along the schedule
    /// no unit waits and every unit takes the floor slot or the budget it
    /// took unlimited. One byte below it, the first check of the unlimited
    /// run that weighed that much fails.
    #[test]
    fn the_no_cost_point_admits_as_an_unlimited_ledger(ops in vec(arb_op(), 0..80)) {
        let unlimited = run(usize::MAX, &ops);
        prop_assert_eq!(unlimited.waited, 0);
        let point = unlimited.normal_peak.max(unlimited.share_peak);
        let at_point = run(point, &ops);
        prop_assert_eq!(&at_point.floors, &unlimited.floors);
        prop_assert_eq!(at_point.waited, 0);
        prop_assert_eq!(at_point.normal_peak, unlimited.normal_peak);
        if point > 0 {
            prop_assert!(run(point - 1, &ops).waited > 0, "nothing waits below the no-cost point");
        }
    }
}

/// No unit counts toward two depths' invitation share checks: a question's
/// bytes count toward its own depth's invited room, an answer's toward the
/// pace of the depth above it, and nothing else.
///
/// That is what keeps the claims nested. A need prices only the answers one
/// level down, so a question's room and an answer's bytes each enter one
/// depth's sum at most; the root's answers enter none, since no question
/// asks about a scope above the root. Every kind and depth is tried: a unit
/// admitted through its floor slot and one against the budget, then a
/// finished reply at every depth, after which each depth's invited room and
/// pace show which depths saw the unit.
#[test]
fn no_unit_counts_toward_two_depths_checks() {
    for unit in [Unit::Answer, Unit::Question] {
        for depth in 0..DEPTHS {
            let ledger = Ledger::new(usize::MAX);
            let floor = poll(&mut ledger.admit(unit, depth, 10)).expect("free slot");
            let budget = poll(&mut ledger.admit(unit, depth, 100)).expect("unlimited");
            for at in 0..DEPTHS {
                ledger.processed(at, 1, true);
            }
            let seen: Vec<usize> = (0..DEPTHS)
                .filter(|&at| ledger.budgeted(Unit::Question, at) > 0 || ledger.pace(at).actual > 0)
                .collect();
            let expected = match unit {
                Unit::Question => vec![depth],
                Unit::Answer => depth.checked_sub(1).into_iter().collect(),
            };
            assert_eq!(
                seen, expected,
                "{unit:?} at depth {depth} counts at depths {seen:?}"
            );
            drop((floor, budget));
        }
    }
}

/// How long a unit may wait for a slot that other threads keep freeing
/// before the handoff test calls the wait lost: far beyond the microseconds
/// each holder keeps the slot.
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
            "a unit waited at a slot that kept freeing: a lost wakeup"
        );
        thread::park_timeout(deadline - now);
    }
}

/// A floor slot passes between threads without a lost wakeup, and frees
/// only in the critical section that returns its last charge's bytes.
///
/// At a zero budget every unit is admitted through its slot, so units of
/// one kind and depth hold it in turn. Each holder splits its grant, drops
/// one part itself and sends the other to a thread that drops it, so the
/// slot frees on whichever thread releases its last charge. No two holders
/// overlap, and every unit is admitted.
///
/// Each holder finds the ledger holding exactly its own grant. A drop
/// returns its bytes and leaves its slot under the admission queue's lock,
/// and the next grant is made under that lock and reaches its holder
/// through it, so the total the holder reads is ordered by the lock alone:
/// had a slot freed outside the critical section that returned its last
/// charge's bytes, the next holder could find them still held.
#[test]
fn a_slot_passes_between_threads_and_frees_after_its_bytes() {
    const THREADS: usize = 8;
    const ROUNDS: usize = 2_000;
    const GRANT: usize = 64;
    const PART: usize = 16;
    let ledger = Ledger::new(0);
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
        let ledger = Ledger::new(usize::MAX);
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
                    ledger.processed(depth, need, true);
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
