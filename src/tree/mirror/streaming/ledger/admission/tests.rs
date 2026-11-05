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
use super::{Admit, Unit};
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

/// The depths the admission proptest admits units at.
const TESTED: std::ops::Range<usize> = 0..4;

/// One operation on a ledger's admissions.
#[derive(Clone, Debug)]
enum Op {
    /// Ask to admit a unit of the kind, at the depth, of the bytes.
    Admit(Unit, usize, usize),
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

/// Operations on a ledger's admissions: units of either kind at every
/// tested depth, and splits, drops, withdrawals and folds of what they hold.
fn arb_op() -> impl Strategy<Value = Op> {
    prop_oneof![
        (arb_unit(), TESTED, 0..400usize)
            .prop_map(|(unit, depth, bytes)| Op::Admit(unit, depth, bytes)),
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
    /// Its place among the units asked for, in the order asked.
    order: usize,
}

proptest! {
    /// Through any sequence of admissions, splits, folds, drops, and
    /// withdrawals, admission keeps its bound and its rule.
    ///
    /// - The ledger holds at most its budget outside its floor slots, the
    ///   bytes its slots carry for others excepted, and each floor slot
    ///   serves at most one unit at a time, its carry counting as one.
    /// - The count the budget is checked against, a count of its own,
    ///   equals the bytes the live charges outside floor slots hold, less
    ///   what the slots carry: the difference of the totals it is part of.
    /// - Each kind and depth's bytes held against the budget agree with the
    ///   live charges.
    /// - A slot carries no more than its kind and depth hold against the
    ///   budget, nor more than the largest grant admitted to one of their
    ///   units, and a free slot leaves nothing of its kind and depth held
    ///   against the budget: a slot freeing while its kind and depth hold
    ///   budget carries their bytes.
    /// - No unit waits that the rule would admit: every waiting unit's slot
    ///   is taken or carrying, and one that would fit the budget waits
    ///   behind a deeper or older waiting unit that does not, the only kind
    ///   that keeps others from it.
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
                        order: next,
                    });
                    next += 1;
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
            let mut still = Vec::new();
            for mut unit in waiting.drain(..) {
                match poll(&mut unit.admit) {
                    Some(grant) => {
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
            let kinds = || {
                [Unit::Answer, Unit::Question]
                    .into_iter()
                    .flat_map(|unit| TESTED.map(move |depth| (unit, depth)))
            };
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
                prop_assert!(
                    carry <= most,
                    "a slot carries {} B, over the largest grant {} B",
                    carry,
                    most
                );
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
            let normal = normal - carried;
            for unit in &waiting {
                prop_assert!(
                    serving.contains_key(&(unit.unit, unit.depth))
                        || ledger.carried(unit.unit, unit.depth) > 0,
                    "a unit waits at a free floor slot"
                );
                if normal + unit.bytes <= budget {
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
