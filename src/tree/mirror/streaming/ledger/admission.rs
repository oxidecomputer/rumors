//! Admission: when a walk may build its next unit of work, and against
//! which allowance it is charged.
//!
//! A *unit* is the work the walk builds in one step of an answer. Each
//! answer is one [`Unit::Answer`], its own items: its resolution, the node
//! copies exploding its scope made, and the reply's reactions that ask
//! nothing. Each question the answer asks is one [`Unit::Question`]: the
//! question's built items (its query, its chunk, and its record) and the
//! reservation for the peer's reply. A unit is admitted with a *grant*, a
//! charge of the most its items can hold; the walk builds the items out of
//! the grant ([`Charge::take`](super::Charge::take)) and drops what is left.
//!
//! # The rule
//!
//! The rule lives in [`decide`] and nowhere else. A waiting unit concerns a
//! scope at some depth, the root's 0 through the leaves' 32: an answer the
//! scope it answers, a question the scope it asks about.
//!
//! 1. If its depth's *floor slot* for its kind is free, it takes the slot.
//!    The slot stays taken until every byte the unit's grant charged has
//!    been released, and those bytes count toward the ledger's floor part,
//!    `F`, not against the budget. The floor part also counts what slots
//!    carry for units admitted against the budget ([below](#carrying)).
//! 2. Otherwise, if no deeper unit is waiting and the grant fits beside
//!    everything else the ledger holds outside its floor slots,
//!    `L − F + c ≤ B`, it is admitted against the budget.
//! 3. Otherwise it waits, and is reconsidered whenever the ledger releases
//!    bytes, a slot comes free, or a slot starts carrying.
//!
//! So every depth always has room for one unit of each kind, and the budget
//! adds pipelining on top; a larger budget only ever adds capacity, since
//! the floor's use does not depend on it, and a check of step 2 that passes
//! at one budget passes, in the same state, at every larger one. The ledger
//! then holds at most the budget plus one grant per slot:
//! `L ≤ B + Σ_d (F_d^answer + F_d^question)`, where each `F` is the largest
//! grant admitted to a unit of that slot's kind and depth.
//!
//! A session that may hold `M` in all, as a peer's memory budget sets it,
//! gives its ledger the budget `B = M − A`, where `A` is the walk's *floor
//! allowance*, the most its floor slots can hold at once, derived from the
//! largest grant each slot can admit
//! ([`floor_allowance`](crate::tree::mirror::streaming::materialized::floor_allowance)).
//! Each slot holds at most one grant of its kind and depth, its own unit's
//! or what it carries, so the slots hold at most `A` together and the
//! ledger at most `M`. An `M` at or below `A` gives the ledger a budget of
//! zero: the session runs on its floor slots alone and holds at most `A`.
//!
//! # Carrying
//!
//! A unit whose floor slot is taken is admitted against the budget, and
//! the slot can free moments later, even within the same instant, while
//! that unit still holds its grant. Its bytes would then hold budget beside
//! a free slot that could hold them, and keep that budget from other work
//! for no work of their own: the slot, not the budget, would have admitted
//! the unit, had it been asked a moment later. A larger budget makes this
//! more likely, since a unit that does not fit the budget waits for its
//! slot instead.
//!
//! So when a slot frees and no unit of its kind and depth waits to take it,
//! the slot *carries* the bytes that units of its kind and depth hold
//! against the budget ([`carry`]), up to the largest grant admitted to one
//! of them. The budget stops counting those bytes and the floor part counts
//! them instead, which may admit waiting units at once, and the slot stays
//! taken while it carries anything.
//!
//! - **A slot carries a count of bytes, not particular charges.** A charge
//!   knows the kind and depth of its unit, not which of their units it was
//!   built out of, so what a slot carries is kept as a count. Each charge
//!   of its kind and depth admitted against the budget returns its bytes to
//!   that count first, as it drops ([`release`]), and the slot frees once
//!   the count reaches zero: the budget has then had back as many bytes as
//!   the slot took from it.
//! - **The floor's sizing holds.** A slot carries only while no unit of its
//!   own holds it, and at most the largest grant of its kind and depth, so
//!   it holds at most one such grant at a time, as the bound above requires.
//! - **The budget's bound holds.** Carrying moves bytes from the budget's
//!   count to the floor part and admits nothing by itself; a charge
//!   returning bytes lowers the budget's count by those its slot did not
//!   carry.
//! - **Exactness holds.** Carrying moves no bytes between charges, so every
//!   item is still built out of its own unit's grant.
//! - **What remains true** at every moment outside admission's critical
//!   sections: a free slot leaves no bytes of its kind and depth held
//!   against the budget. A slot frees only as bytes return, under the
//!   queue's lock ([`release`]), and the critical section that frees it
//!   either hands it to a unit waiting for it or lets it carry what the
//!   budget holds of its kind and depth; while it is free, nothing of its
//!   kind and depth is admitted against the budget, since step 1 takes it
//!   first.
//!
//! Carrying reads the schedule and never the budget, so along a schedule in
//! which no unit waits, every check reads the same state at every budget: a
//! budget at least the most an unlimited ledger holds outside its floor
//! slots along that schedule admits every unit as the unlimited one does.
//! Nor does carrying change
//! [why every unit is admitted](#why-every-unit-is-admitted). At a zero
//! budget nothing is held against the budget, so nothing is carried. At any
//! budget, a slot starts carrying only while no unit of its kind and depth
//! waits, and carries no more than the units of its kind and depth then
//! hold against the budget, all admitted before any unit that waits for the
//! slot later; and every byte of its kind and depth that returns goes to
//! the carry first. So the slot frees no later than those units have
//! released their charges, which needs nothing of a unit admitted after
//! them, as a slot held by its own unit frees once that unit has.
//!
//! # One lock for every decision
//!
//! Every value a decision reads lives in the queue's locked state
//! ([`Slots`]): whether each slot is taken, the bytes the budget counts,
//! what each kind and depth holds against the budget, and what each slot
//! carries. Every change to them is made under the same lock: a charge is
//! built there ([`decide`]), an item's shortfall joins there
//! ([`shortfall`]), and a charge's drop returns its bytes, leaves its slot,
//! and reconsiders the waiting units in one critical section ([`release`]).
//! So no decision reads a total beside a part of it that a concurrent drop
//! has changed and the total has not yet: the budget's count is a count of
//! its own, never the difference of counts that change apart.
//!
//! # Why every unit is admitted
//!
//! The receive path never waits on the walk, and the send path ends because
//! the peer's receive path drains the pipe (see the pipe module's wait
//! table), so only admission can hold the walk. The argument is an
//! induction from the leaves upward, over both sides at once; it uses no
//! budget above zero, so it covers every budget on either side.
//!
//! - A question slot at depth `d` frees once the question's items and
//!   reservation are released: its chunk when encoded, its record when the
//!   reply's last chunk is taken, its reservation as this side's stage
//!   finishes the reply and assembles the scope asked about. That takes the
//!   peer's answer and questions about the scope, at depth `d` and below on
//!   the peer's side, this side's answers to those questions, at `d + 1`,
//!   and the subtree's assembly: work at depths the induction already
//!   grants, apart from the peer's answer at `d`, whose own slot frees
//!   without anything at `d` on this side.
//! - An answer slot at depth `d` frees once the answer's scope is
//!   assembled, which takes its questions, at question depth `d + 1`, and
//!   their subtrees.
//! - A stage takes its units in order, so a unit waiting at depth `d` waits
//!   only for units ahead of it at `d`, whose slots free by the two cases
//!   above, and for deeper units, which waiters at greater depths precede
//!   for the budget but never for the floor.
//!
//! An answer's own items cannot share a slot with its questions: at a zero
//! budget the answer's resolution would hold the slot until its scope is
//! assembled, which waits on the answer's later questions, which wait for
//! that slot. Nor can a question take the slot of the depth it asks about
//! where answers there take it too: this side's question would wait on the
//! peer's answer at that depth, held behind the peer's question waiting on
//! this side's answer there.

use std::cmp::Reverse;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::task::{Context, Poll, Waker};

use super::{Account, Charge};
use crate::tree::mirror::streaming::holding::Holding;

/// The depths a scope can sit at, the root's 0 through the leaves' 32,
/// each with one floor slot per kind of unit.
pub(super) const DEPTHS: usize = 33;

/// The floor slots of one ledger: one per kind of unit at each depth.
const SLOTS: usize = 2 * DEPTHS;

/// The two kinds of unit, each with its own floor slots.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Unit {
    /// An answer's own items: everything it builds but its questions.
    Answer,
    /// One question's built items and the reservation for its reply.
    Question,
}

impl Unit {
    /// The kind's row of floor slots.
    pub(super) fn row(self) -> usize {
        match self {
            Self::Answer => 0,
            Self::Question => 1,
        }
    }

    /// The place a grant of this kind is charged to until the walk builds
    /// its items out of it.
    pub(super) fn place(self) -> Holding {
        match self {
            Self::Answer => Holding::Resolutions,
            Self::Question => Holding::Questions,
        }
    }
}

/// The units waiting for one ledger's admission, and the state of its
/// floor slots and its budget, which every decision reads.
pub(super) struct Queue {
    /// The units waiting, in no particular order.
    waiting: Vec<Arc<Waiter>>,
    /// The order number the next waiting unit takes.
    next: u64,
    /// The bytes the budget counts, and the state of each floor slot's kind
    /// and depth, which only admission's critical sections touch.
    slots: Slots,
}

impl Default for Queue {
    /// Returns a queue with no unit waiting, every slot free, and nothing
    /// held against the budget.
    fn default() -> Self {
        Self {
            waiting: Vec::new(),
            next: 0,
            slots: Slots {
                normal: 0,
                taken: [false; SLOTS],
                budgeted: [0; SLOTS],
                carried: [0; SLOTS],
                largest: [0; SLOTS],
            },
        }
    }
}

/// The bytes the budget counts, and for each floor slot, whether it is
/// taken, what units of its kind and depth hold against the budget, what
/// the slot carries of it, and the most it may carry
/// ([Carrying](self#carrying)).
///
/// The per-slot counts are indexed by [`Slot::index`].
///
/// It lives in the queue, so every change and every read holds the queue's
/// lock: a function that changes it takes it, which only the lock reaches
/// ([`Account::add`], [`Account::sub`], [`Account::carry`]).
pub(super) struct Slots {
    /// Bytes held outside floor slots, neither through a slot's grant nor
    /// carried by a slot: what the budget counts, `L − F`, which the rule's
    /// step 2 reads ([`decide`]).
    ///
    /// It is a count of its own, changed by the same operations that change
    /// the totals it is part of, rather than the difference of the total
    /// held, the floor part, and what slots carry, which change apart.
    ///
    /// It is written only under the queue's lock, like every count here,
    /// and is never below zero:
    ///
    /// - A charge admitted against the budget adds its bytes as it is
    ///   built and subtracts them as it drops. A floor charge never changes
    ///   the count.
    /// - A slot that starts carrying subtracts at most what charges of its
    ///   kind and depth hold against the budget (`budgeted`), all of which
    ///   the count includes ([Carrying](self#carrying)).
    /// - A charge whose bytes return to its slot's carry subtracts only the
    ///   rest ([`Account::sub`]), so no byte the carry took leaves twice.
    ///
    /// So it counts the bytes of charges built and not yet dropped, less
    /// what slots carry of them.
    pub(super) normal: usize,
    /// Whether each slot is taken: by a unit, until the last charge built
    /// out of the unit's grant drops, or by its carry, until every carried
    /// byte has returned.
    taken: [bool; SLOTS],
    /// Bytes held by charges built out of grants admitted against the
    /// budget, by the kind and depth of their unit, the bytes floor slots
    /// carry for them included.
    ///
    /// Each count is what a freed slot of its kind and depth can carry. A
    /// charge admitted against the budget is built under the lock
    /// ([`decide`]), an item's shortfall joins under it ([`shortfall`]),
    /// and a charge's drop returns its bytes under it ([`release`]), so each
    /// carry reads exactly what the budget holds of its kind and depth.
    budgeted: [usize; SLOTS],
    /// The bytes each slot carries for charges of its kind and depth
    /// admitted against the budget; the account's `carried` is their sum.
    carried: [usize; SLOTS],
    /// The largest grant admitted to a unit of each slot's kind and depth,
    /// through the slot or against the budget: the most the slot carries.
    largest: [usize; SLOTS],
}

impl Slots {
    /// The bytes charges of `unit` kind at `depth` hold against the budget,
    /// for the account to change as they are charged and returned.
    pub(super) fn budgeted(&mut self, unit: Unit, depth: usize) -> &mut usize {
        &mut self.budgeted[Slot::new(unit, depth).index()]
    }

    /// Note a grant of `bytes` admitted to a unit of `slot`'s kind and
    /// depth.
    fn admitted(&mut self, slot: Slot, bytes: usize) {
        let largest = &mut self.largest[slot.index()];
        *largest = (*largest).max(bytes);
    }
}

/// One unit waiting for admission.
struct Waiter {
    /// The unit's kind.
    unit: Unit,
    /// The depth of the scope it concerns.
    depth: usize,
    /// The most its items can hold.
    bytes: usize,
    /// Its order among the units its ledger queued, which breaks ties
    /// between waiting units at one depth.
    order: u64,
    /// The grant once admitted, and the task to wake then.
    state: Mutex<State>,
}

/// What a waiting unit has received.
#[derive(Default)]
struct State {
    /// The grant, once the unit is admitted and until its task takes it.
    grant: Option<Charge>,
    /// The task awaiting the grant.
    waker: Option<Waker>,
}

/// A floor slot: the one for its kind of unit at its depth.
///
/// Every charge built out of a floor grant holds its slot through one
/// shared pointer, and each part split off such a charge holds another. So
/// the pointer's count is the number of live charges sharing the slot, and
/// the slot frees when the last of them is released, as the rule requires.
///
/// Whether a slot is taken lives in the queue's locked state ([`Slots`])
/// and changes only under the lock: only [`decide`] and [`carry`] take a
/// slot, and a slot frees only in [`release`] or [`fold`], which take the
/// last pointer to it ([`Arc::into_inner`]) under the lock. The one change
/// to a slot's sharers made outside the lock is a split's, made through a
/// charge that shares the slot before and after it. So while the lock is
/// held, a slot is taken or free as the critical section's own steps leave
/// it.
///
/// A charge returns its bytes and leaves its slot in one critical section
/// ([`release`]), which admits the waiting units before it ends
/// ([`admit_waiting`]): a unit waiting for the slot takes it there, and a
/// unit that takes it in a later critical section finds the bytes returned.
/// A unit queued after that section is decided under the lock as it queues
/// ([`Admit::new`]), so no unit is left waiting at a free slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Slot {
    /// The kind of unit the slot serves.
    unit: Unit,
    /// The depth of the scopes those units concern.
    depth: usize,
}

impl Slot {
    /// The slot of `unit` kind at `depth`, which must be below [`DEPTHS`].
    fn new(unit: Unit, depth: usize) -> Self {
        Self { unit, depth }
    }

    /// The slot's index among the ledger's slots.
    fn index(self) -> usize {
        self.unit.row() * DEPTHS + self.depth
    }
}

/// Lock `mutex`, whose data no panic can leave inconsistent: each critical
/// section here only flips flags and moves values.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The admission rule: return the grant admitting a unit, or `None` to keep
/// it waiting.
///
/// The unit, of `unit` kind concerning a scope at `depth` and holding at
/// most `bytes`, is admitted through its floor slot if free, or against the
/// budget if `budget_open` and it fits. The steps are the module's rule.
///
/// The caller holds the queue's lock, which makes this one of the two
/// places a free slot is taken ([`Slot`]), and passes the slots' state it
/// keeps. Every value the steps read lives in that state, the bytes the
/// budget counts included, so each is one the ledger has held.
fn decide(
    account: &Arc<Account>,
    slots: &mut Slots,
    budget_open: bool,
    unit: Unit,
    depth: usize,
    bytes: usize,
) -> Option<Charge> {
    let slot = Slot::new(unit, depth);
    // 1. If the unit's floor slot is free, it takes the slot.
    if !slots.taken[slot.index()] {
        slots.taken[slot.index()] = true;
        slots.admitted(slot, bytes);
        #[cfg(any(test, feature = "test-internals"))]
        account.slot_peaks[unit.row()][depth].fetch_max(bytes, Ordering::Relaxed);
        return Some(
            Charge::granted(
                account,
                slots,
                unit.place(),
                bytes,
                Some(Arc::new(slot)),
                Some((unit, depth)),
            )
            .marked_as_grant(),
        );
    }
    // 2. Otherwise it is admitted against the budget if no deeper unit
    // waits and its grant fits beside what the ledger holds outside its
    // floor slots, or it waits.
    if !budget_open || slots.normal.saturating_add(bytes) > account.budget {
        return None;
    }
    slots.admitted(slot, bytes);
    Some(
        Charge::granted(
            account,
            slots,
            unit.place(),
            bytes,
            None,
            Some((unit, depth)),
        )
        .marked_as_grant(),
    )
}

/// Admit every waiting unit the ledger now has room for, deepest first
/// ([`admit_waiting`]).
fn reconsider(account: &Arc<Account>) {
    let queue = lock(&account.admission);
    admit_waiting(account, queue, None);
}

/// Return the `bytes` a dropped charge for `place` holds, leave the floor
/// slot it shares if any, and admit every waiting unit the ledger then has
/// room for ([`Charge`]'s drop).
///
/// The charge shares `slot` if given, and was built out of a grant to a
/// unit of the kind and depth `unit` gives, if admission granted it.
///
/// A charge admitted against the budget returns its bytes first to what its
/// unit's floor slot carries for its kind and depth: those are the bytes the
/// slot stopped the budget from counting, whichever of their charges drops
/// first. A slot whose carried bytes have all returned frees, as a slot
/// does when its own unit's last charge drops.
///
/// It all happens in one critical section of the queue's lock: no decision
/// sees the bytes counted twice or not at all, no slot carries more of its
/// kind's bytes than the budget holds for it, and a slot frees only with its
/// bytes returned and the waiting units reconsidered.
pub(super) fn release(
    account: &Arc<Account>,
    place: Holding,
    bytes: usize,
    slot: Option<Arc<Slot>>,
    unit: Option<(Unit, usize)>,
) {
    let mut queue = lock(&account.admission);
    let freed = match (slot, unit) {
        (Some(slot), unit) => {
            account.sub(&mut queue.slots, place, bytes, true, unit, 0);
            let freed = Arc::into_inner(slot);
            if let Some(slot) = freed {
                queue.slots.taken[slot.index()] = false;
            }
            freed
        }
        (None, Some((unit, depth))) => {
            let slot = Slot::new(unit, depth);
            let carried = &mut queue.slots.carried[slot.index()];
            let returned = bytes.min(*carried);
            *carried -= returned;
            let emptied = returned > 0 && *carried == 0;
            // The bytes that returned to the carry left the budget's count
            // when the slot took them, so only the rest leave it now, in one
            // change.
            account.sub(
                &mut queue.slots,
                place,
                bytes,
                false,
                Some((unit, depth)),
                returned,
            );
            if emptied {
                queue.slots.taken[slot.index()] = false;
            }
            emptied.then_some(slot)
        }
        (None, None) => {
            account.sub(&mut queue.slots, place, bytes, false, None, 0);
            None
        }
    };
    admit_waiting(account, queue, freed);
}

/// Leave `slot` for a charge folded into another charge that shares it,
/// whose bytes stay held ([`Charge::absorb`]).
///
/// The slot frees only under the queue's lock, as every slot does
/// ([`Slot`]). The charge folded into still shares the slot, so this cannot
/// free it; were it to, admission would reconsider as a release does.
pub(super) fn fold(account: &Arc<Account>, slot: Arc<Slot>) {
    let mut queue = lock(&account.admission);
    if let Some(slot) = Arc::into_inner(slot) {
        queue.slots.taken[slot.index()] = false;
        admit_waiting(account, queue, Some(slot));
    }
}

/// Add the `bytes` an item for `place` needed beyond the grant it was built
/// out of, through a floor slot if `floor`, for a charge of `unit`
/// ([`Charge::take`](super::Charge::take)).
///
/// It holds the queue's lock as it adds, as every byte admitted against the
/// budget joins, so that a slot's carry, made under the lock, never
/// subtracts bytes the budget's count has yet to receive
/// ([`Slots::normal`]). An item falls short only where a grant fails to bound its
/// unit, which admission requires it to, so the lock costs a conforming
/// session nothing. The caller holds no admission lock.
pub(super) fn shortfall(
    account: &Account,
    place: Holding,
    bytes: usize,
    floor: bool,
    unit: Option<(Unit, usize)>,
) {
    let mut queue = lock(&account.admission);
    account.add(&mut queue.slots, place, bytes, floor, unit);
}

/// Charge `bytes` for `place` outside admission
/// ([`Ledger::charge`](super::Ledger::charge)), under the queue's lock, as
/// the account takes every charge.
#[cfg(test)]
pub(super) fn charge(account: &Arc<Account>, place: Holding, bytes: usize) -> Charge {
    let mut queue = lock(&account.admission);
    Charge::granted(account, &mut queue.slots, place, bytes, None, None)
}

/// Bytes held outside floor slots: what the budget counts
/// ([`Slots::normal`]).
#[cfg(test)]
pub(super) fn normal(account: &Account) -> usize {
    lock(&account.admission).slots.normal
}

/// Bytes held by charges built out of grants of `unit` kind admitted
/// against the budget, concerning scopes at `depth`, the bytes their slot
/// carries for them included.
#[cfg(test)]
pub(super) fn budgeted(account: &Account, unit: Unit, depth: usize) -> usize {
    lock(&account.admission).slots.budgeted[Slot::new(unit, depth).index()]
}

/// The bytes the floor slot of `unit` kind at `depth` carries
/// ([Carrying](self#carrying)).
#[cfg(test)]
pub(super) fn carried(account: &Account, unit: Unit, depth: usize) -> usize {
    lock(&account.admission).slots.carried[Slot::new(unit, depth).index()]
}

/// The largest grant admitted to a unit of `unit` kind at `depth`.
#[cfg(test)]
pub(super) fn largest(account: &Account, unit: Unit, depth: usize) -> usize {
    lock(&account.admission).slots.largest[Slot::new(unit, depth).index()]
}

/// Let the free `slot` carry what its kind and depth hold against the
/// budget, unless one of them waits for it.
///
/// The slot carries at most the largest grant of one of their units
/// ([Carrying](self#carrying)). A slot left free because a unit of its kind
/// and depth waits is taken by that unit's decision before the lock is
/// released ([`admit_waiting`]).
///
/// The caller holds the queue's lock, which makes this, with [`decide`],
/// one of the two places a free slot is taken, and freed the slot in the
/// same critical section ([`release`], [`fold`]). Only a step under the
/// lock takes a free slot, so the slot is still free here.
fn carry(account: &Account, queue: &mut Queue, slot: Slot) {
    debug_assert!(
        !queue.slots.taken[slot.index()],
        "a slot freed in this critical section stays free until a step in it takes the slot",
    );
    let Slot { unit, depth } = slot;
    if queue
        .waiting
        .iter()
        .any(|waiter| (waiter.unit, waiter.depth) == (unit, depth))
    {
        return;
    }
    // A free slot carries nothing, so every byte of its kind and depth held
    // against the budget is the budget's.
    let held = queue.slots.budgeted[slot.index()];
    let bytes = held.min(queue.slots.largest[slot.index()]);
    if bytes == 0 {
        return;
    }
    queue.slots.taken[slot.index()] = true;
    queue.slots.carried[slot.index()] = bytes;
    account.carry(&mut queue.slots, bytes, unit, depth);
}

/// Let the slot `freed` names carry ([`carry`]), then admit every waiting
/// unit the ledger has room for, under the lock `queue` holds.
///
/// Units are considered deepest first. A unit that must keep waiting closes
/// the budget to every shallower one, so deeper work is admitted first; a
/// shallower unit may still take its own free floor slot.
fn admit_waiting(account: &Arc<Account>, mut queue: MutexGuard<'_, Queue>, freed: Option<Slot>) {
    if let Some(slot) = freed {
        carry(account, &mut queue, slot);
    }
    let mut woken = Vec::new();
    let Queue { waiting, slots, .. } = &mut *queue;
    if !waiting.is_empty() {
        waiting.sort_by_key(|waiter| (Reverse(waiter.depth), waiter.order));
        let mut budget_open = true;
        waiting.retain(|waiter| {
            match decide(
                account,
                slots,
                budget_open,
                waiter.unit,
                waiter.depth,
                waiter.bytes,
            ) {
                Some(grant) => {
                    let mut state = lock(&waiter.state);
                    state.grant = Some(grant);
                    woken.extend(state.waker.take());
                    false
                }
                None => {
                    budget_open = false;
                    true
                }
            }
        });
    }
    // Whether the freed slot is taken changes under this lock only by this
    // critical section's own steps, so a slot that reads free here is one
    // its carry and its waiters left free.
    if let Some(slot) = freed.filter(|slot| !queue.slots.taken[slot.index()]) {
        debug_assert!(
            queue.slots.budgeted[slot.index()] == 0,
            "a slot left free leaves nothing of its kind and depth held against the budget",
        );
    }
    drop(queue);
    for waker in woken {
        waker.wake();
    }
}

/// One admitted unit, as the process's admission hook sees it.
#[cfg(any(test, feature = "test-internals"))]
#[derive(Clone, Copy, Debug)]
pub struct Admission {
    /// The ledger that admitted the unit, identified as in
    /// [`LedgerChange::ledger`](super::LedgerChange::ledger).
    pub ledger: usize,
    /// Whether the unit is a question; otherwise it is an answer's own
    /// items.
    pub question: bool,
    /// The depth of the scope the unit concerns: the root's 0 through the
    /// leaves' 32.
    pub depth: usize,
    /// The grant's bytes: the most the unit's items can hold.
    pub bytes: usize,
    /// Whether the unit took its depth's floor slot rather than budget.
    pub floor: bool,
    /// Whether the unit waited for admission rather than being admitted
    /// as it was queued.
    pub waited: bool,
    /// Whether this reports the grant's unused bytes returning once its
    /// unit was built, rather than the grant itself: `bytes` then counts
    /// what returned, and `waited` is false.
    pub leftover: bool,
}

/// The process's admission hook, called as each unit takes its grant and
/// as each grant returns what its unit did not use.
#[cfg(any(test, feature = "test-internals"))]
pub type AdmissionHook = fn(Admission);

/// The admission hook every ledger in the process reports through, once
/// installed.
#[cfg(any(test, feature = "test-internals"))]
static ADMISSION_HOOK: std::sync::OnceLock<AdmissionHook> = std::sync::OnceLock::new();

/// Install `hook` as the process's admission hook, returning whether it is
/// now the installed one; the first installation wins.
#[cfg(any(test, feature = "test-internals"))]
pub fn observe_admissions(hook: AdmissionHook) -> bool {
    std::ptr::fn_addr_eq(*ADMISSION_HOOK.get_or_init(|| hook), hook)
}

/// Report a unit taking `grant` to the installed hook.
#[cfg(any(test, feature = "test-internals"))]
fn admitted(account: &Account, waiter: &Waiter, grant: &Charge, waited: bool) {
    if let Some(hook) = ADMISSION_HOOK.get() {
        hook(Admission {
            ledger: std::ptr::from_ref(account) as usize,
            question: waiter.unit == Unit::Question,
            depth: waiter.depth,
            bytes: waiter.bytes,
            floor: grant.is_floor(),
            waited,
            leftover: false,
        });
    }
}

/// Report a grant of `unit` kind at `depth` returning the `bytes` its unit
/// did not use, through a floor slot if `floor`, to the installed hook.
#[cfg(any(test, feature = "test-internals"))]
pub(super) fn settled(account: &Account, unit: Unit, depth: usize, bytes: usize, floor: bool) {
    if let Some(hook) = ADMISSION_HOOK.get() {
        hook(Admission {
            ledger: std::ptr::from_ref(account) as usize,
            question: unit == Unit::Question,
            depth,
            bytes,
            floor,
            waited: false,
            leftover: true,
        });
    }
}

/// Report a unit taking its grant: nothing outside test builds.
#[cfg(not(any(test, feature = "test-internals")))]
#[inline(always)]
fn admitted(_: &Account, _: &Waiter, _: &Charge, _: bool) {}

/// Wait for a unit's admission ([`Ledger::admit`](super::Ledger::admit)).
pub(crate) struct Admit {
    /// The ledger admitting the unit.
    account: Arc<Account>,
    /// The unit, waiting until its grant is taken.
    waiter: Option<Arc<Waiter>>,
    /// Whether the unit was not admitted as it was queued.
    waited: bool,
}

impl Admit {
    /// Queue a unit of `unit` kind concerning a scope at `depth`, whose
    /// items take at most `bytes`, and admit what the ledger has room for.
    pub(super) fn new(account: Arc<Account>, unit: Unit, depth: usize, bytes: usize) -> Self {
        assert!(depth < DEPTHS, "a scope sits at most 32 bytes deep");
        let waiter = {
            let mut queue = lock(&account.admission);
            let order = queue.next;
            queue.next += 1;
            let waiter = Arc::new(Waiter {
                unit,
                depth,
                bytes,
                order,
                state: Mutex::default(),
            });
            queue.waiting.push(waiter.clone());
            waiter
        };
        reconsider(&account);
        let waited = lock(&waiter.state).grant.is_none();
        if waited {
            account.waited.fetch_add(1, Ordering::Relaxed);
        }
        Self {
            account,
            waiter: Some(waiter),
            waited,
        }
    }
}

impl Future for Admit {
    type Output = Charge;

    /// Return the grant once the unit is admitted.
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Charge> {
        let waiter = self
            .waiter
            .as_ref()
            .expect("an admission is not polled after it returns its grant");
        let mut state = lock(&waiter.state);
        match state.grant.take() {
            Some(grant) => {
                drop(state);
                admitted(&self.account, waiter, &grant, self.waited);
                self.waiter = None;
                Poll::Ready(grant)
            }
            None => {
                state.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

impl Drop for Admit {
    /// Withdraw a unit no longer awaited, returning a grant it never took.
    fn drop(&mut self) {
        let Some(waiter) = self.waiter.take() else {
            return;
        };
        lock(&self.account.admission)
            .waiting
            .retain(|queued| !Arc::ptr_eq(queued, &waiter));
        let unclaimed = lock(&waiter.state).grant.take();
        drop(unclaimed);
        // A withdrawn unit may have held the budget for shallower ones.
        reconsider(&self.account);
    }
}

#[cfg(test)]
mod tests;
