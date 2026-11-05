//! Admission: when a walk may build its next unit of work, and against
//! which allowance it is charged.
//!
//! A *unit* is the work the walk builds in one step of an answer. Each
//! answer is one [`Unit::Answer`], its own items: its resolution and the
//! reply's reactions that ask nothing. Each question the answer asks is one [`Unit::Question`]: the
//! question's built items (its query, its chunk, and its record) and the
//! reservation for the peer's reply. The reservation's largest part is the
//! room the reply's chunks park in as they arrive, *the reply's room*; its
//! other parts hold the records the reply's questions derive and the
//! parent resolution that reassembles the scope asked about. A unit is
//! admitted with a *grant*, a charge of the most its items can hold; the
//! walk builds the items out of the grant
//! ([`Charge::take`](super::Charge::take)) and drops what is left.
//!
//! # The model
//!
//! - **The floor is a liveness guarantee for sending.** Each depth of the
//!   comparison can always build and send one answer and one question,
//!   whatever the budget, so the budget can never starve a depth. Each
//!   depth has one *floor slot* per kind of unit, and the memory a unit
//!   takes through its floor slot is an allowance outside the budget.
//! - **A floor slot frees once everything its unit was granted has been
//!   released.** For a question, that includes the reply's room, released
//!   as the reply is processed, and the parent resolution's part, released
//!   once the scope asked about is assembled. That keeps the floor to one
//!   unit of each kind per depth at a time.
//! - **The *budget* is how much the session pipelines beyond that
//!   minimum:** it bounds the memory of every unit not on its floor slot.
//!
//! Processing a reply does two things:
//!
//! 1. **The reply's own memory comes back.** The reply's room is released
//!    chunk by chunk as this side's stage uses the chunks, and whatever
//!    exceeds what the reply can still deliver as soon as the reply shows
//!    it: to the budget if its question was admitted against the budget,
//!    or toward freeing its floor slot if it took the slot. Receiving
//!    charges nothing new.
//! 2. **Processing the reply makes this side send new things.** The peer's
//!    reply carries its questions to this side, and answering them means
//!    building and sending this side's answers and its own next questions.
//!    Every unit needs permission to build and send: its depth's floor slot
//!    if free, otherwise room in the budget.
//!
//! Each send is admitted on its own: a unit built while the stage processes
//! a chunk waits, if it must, while the chunk still holds its part of the
//! reply's room, which returns once the stage has used the chunk.
//!
//! # The rule
//!
//! The rule lives in [`decide`] and nowhere else. A unit concerns a scope
//! at some depth, the root's 0 through the leaves' 32: an answer the scope
//! it answers, a question the scope it asks about. Each unit gets
//! permission to build and send this way:
//!
//! 1. **If its depth's floor slot for its kind is free, it takes the
//!    slot.** Its bytes count toward the ledger's floor part, `F`, not
//!    against the budget. The floor part also counts what slots carry for
//!    units admitted against the budget ([below](#carrying)).
//! 2. **Otherwise, it is admitted against the budget** if no deeper unit is
//!    waiting for room and its grant fits beside everything else the ledger
//!    holds outside its floor slots, `L − F + c ≤ B`.
//! 3. **A question under the invitation share is admitted only while its
//!    depth's claims, with its own, fit the budget:**
//!    `I_d + c + α · (N_d + n) ≤ B` (see [below](#the-invitation-share)).
//!
//! A unit refused at step 2 or 3 waits, reconsidered whenever the ledger
//! releases bytes, a slot comes free, a slot starts carrying, or a reply
//! finishes. One refused at
//! step 2 waits for room, and keeps every shallower unit from the budget
//! meanwhile, so deeper work comes first; one refused at step 3 waits for
//! its depth's replies to be processed, not for room, and keeps no other
//! unit waiting.
//!
//! So every depth always has room for one unit of each kind, and the budget
//! adds pipelining on top; a larger budget only ever adds capacity, since
//! the floor's use does not depend on it, and a check of step 2 or 3 that
//! passes at one budget passes, in the same state, at every larger one.
//! The ledger then holds at most the budget plus one grant per slot:
//! `L ≤ B + Σ_d (F_d^answer + F_d^question)`, where each `F` is the largest
//! grant admitted to a unit of that slot's kind and depth. The invitation
//! share takes nothing from this bound: it governs no floor slot, and it
//! only refuses admissions that step 2 would allow.
//!
//! A session that may hold `M` in all, as a peer's memory budget sets it,
//! gives its ledger the budget `B = M − A − X`. Here `A` is the walk's
//! *floor allowance*, the most its floor slots can hold at once, derived
//! from the largest grant each slot can admit
//! ([`floor_allowance`](crate::tree::mirror::streaming::materialized::floor_allowance)),
//! and `X` is the session's *fixed allowance*, the most the state it holds
//! outside the ledger whatever its budget can hold
//! ([`fixed_allowance`](crate::tree::mirror::streaming::materialized::fixed_allowance)).
//! Each slot holds at most one grant of its kind and depth, its own unit's
//! or what it carries, so the slots hold at most `A` together, the ledger
//! at most `M − X`, and the session at most `M` with its fixed state.
//! An `M` at or below `A + X` gives the ledger a budget of zero: the
//! session runs on its floor slots alone.
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
//!   names its slot in a byte and has no room to name its unit, so what a
//!   slot carries is kept as a count. Each charge of its kind and depth
//!   admitted against the budget returns its bytes to that count first, as
//!   it drops ([`release`]), and the slot frees once the count reaches zero:
//!   the budget has then had back as many bytes as the slot took from it.
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
//! Carrying reads the schedule and never the budget, so the states every
//! check reads along a schedule in which nothing waits are the same at
//! every budget: the [no-cost point](#the-no-cost-point) holds with `Λ` and
//! the share peak read from a session carrying as every session does. Nor
//! does it change [why every unit is admitted](#why-every-unit-is-admitted).
//! At a zero budget nothing is held against the budget, so nothing is
//! carried. At any budget, a slot starts carrying only while no unit of its
//! kind and depth waits, and carries no more than the units of its kind and
//! depth then hold against the budget, all admitted before any unit that
//! waits for the slot later; and every byte of its kind and depth that
//! returns goes to the carry first. So the slot frees no later than those
//! units have released their charges, which needs nothing of a unit admitted
//! after them, as a slot held by its own unit frees once that unit has.
//!
//! The units of an invitation the slot carries keep their claims: a claim
//! withdraws as its reply finishes, and the share's room `I_d` counts only
//! what the budget still holds of the depth's questions.
//!
//! # The invitation share
//!
//! A question *invites* its reply: it holds the reply's room from the
//! moment it is admitted until this side's stage has processed the reply.
//! A stage processes its replies one at a time, in the order its questions
//! were asked, and processing a reply admits units of its own: an answer
//! one level down for each child the reply disputes, and a question two
//! levels down for each grandchild it lists that this side does not match.
//! Room held by a reply that waits its turn overlaps no round trip, and
//! takes the budget from the processing that would let the stage reach it.
//! Without a limit, the answers at the top of a session ask their questions
//! all at once, before any deeper work exists; with a budget smaller than
//! the session's whole need, their replies fill it and then wait behind
//! one stage, while the rest of the session runs in what is left. So a
//! depth should invite no more replies than the budget would also let it
//! process.
//!
//! Each question the share admits against the budget therefore carries a
//! *claim*: what it holds of its grant, `c`, and `α · n`, where `n` is the
//! bytes processing its reply is predicted to admit and `α` the depth's
//! *pace* (both below). A depth admits a question only while its
//! outstanding claims, the new one included, fit in the budget together:
//!
//! > `I_d + c + α · (N_d + n) ≤ B`
//!
//! where `I_d` is the bytes the depth's questions hold against the budget
//! and `N_d` the predicted needs of those of them whose replies this side's
//! stage has not finished, the depth's *outstanding claims*. The check
//! follows Dijkstra's banker in its simplest form, lending only while every
//! customer's declared credit fits in the capital at once: a question is
//! admitted only if its depth could also afford to process every reply it
//! has invited. A
//! question's room returns as its reply is processed, and its need
//! withdraws as the stage finishes the reply
//! ([`Ledger::processed`](super::Ledger::processed)), so the depth's claims
//! shrink as its stage reaches them.
//!
//! No term depends on round trips, delays, or scheduling. A claim states
//! what an invitation could cost the budget, never how long it holds it, so
//! the check reads the same on every link.
//!
//! - **The predicted need, `n`, is a lower bound.** The answers a reply's
//!   processing builds one level down are sized by this side's children of
//!   the node's children, which the walk reads only as it builds the
//!   question, after admitting it, and by the
//!   reply's entries, which only the reply carries. So the counts known at
//!   admission bound the need only at the type maximum. The
//!   walk predicts the least it can be instead: the answers that a reply
//!   listing every grandchild its bound allows must make processing build,
//!   as few as can hold the entries, each at the least it can cost (the
//!   walk's `charges::processing_need`, which the walk passes to
//!   [`Ledger::invite`](super::Ledger::invite)).
//!   A reply that can list nothing, such as one answering a request, is
//!   predicted to need nothing, which leaves its depth the budget's whole
//!   room.
//! - **The pace, `α`, corrects the prediction from what processing admits**
//!   ([`Pace`], [`Share::pace`]): per depth, the bytes the answers one
//!   level down have kept out of their grants over the needs predicted for the
//!   replies that built them, both read as each reply finishes. It is a
//!   ratio of sums, one until the first reply finishes. It falls below one
//!   where replies list less than their bounds allow, or where the answers
//!   return room they did not use, and rises above one where this side
//!   holds children the peer lacks. Since `n` bounds below, `α = 1` takes the
//!   prediction as it stands and bounds nothing.
//! - **The claims nest.** A depth-`d` claim prices only the answers at
//!   `d + 1` its reply's processing builds. The questions at `d + 2` that
//!   processing sends claim their own room, and their own replies'
//!   processing, at their own depth when they are asked. So each byte
//!   counts toward one depth's sum at most: a question's toward its own
//!   depth's, an answer's toward the depth above it. Pooling the depths
//!   into one check fails by construction rather than by degree: a claim
//!   would have to cover the whole subtree its reply leads to, since the
//!   questions it covers carry claims of their own two levels further down,
//!   so the pooled sum outgrows the budget, refuses every later question at
//!   every depth, and leaves the session to its floor slots. The per-depth
//!   check is the structure the claims have.
//! - **It applies at every budget.** The share governs every question an
//!   internal answer asks ([`Ledger::invite`](super::Ledger::invite)).
//!   Each depth's check stops binding by its own form, at the depth's
//!   *endpoint*: the sum, over every question the depth would have
//!   outstanding at once, of `c + α · n`, its questions beside the
//!   processing of all their replies. A budget at or above it admits
//!   every question at that depth that fits the budget. For the root's
//!   questions, which a session asks together, the endpoint is about a fan
//!   of them, each beside its reply's predicted processing.
//!
//! # The no-cost point
//!
//! A budget large enough never meets the share. Let `Λ` be the most the
//! ledger holds outside its floor slots in a session with no budget, and
//! the *share peak* the largest `I_d + c + α · (N_d + n)` any share check
//! of that session weighs. The session's *no-cost point* is the larger of
//! the two. At any budget at or above it, every check that session makes
//! would pass in the same state, so no unit waits and every admission
//! decision is the same, and by induction along its schedule the session
//! is the one with no budget, event for event. Below it, the first check
//! along that schedule that fails refuses a unit, so some unit waits.
//!
//! Identity is promised at the no-cost point, not at `Λ`. A rule that holds
//! back the first round's questions, the burst it exists to stop, refuses
//! some question that fits; to stay identical at every budget of at least
//! `Λ` it would have to know, at the first round trip, that the budget lies
//! below `Λ`, which the session cannot know until it has run.
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
//!   reservation are released: its chunk once the multiplexer has written
//!   the chunk's last frame (a wait on the pipe alone, which drains
//!   whatever the walk waits for), its record when the reply's last chunk
//!   is taken, the reply's room chunk by chunk as this
//!   side's stage uses the reply's chunks and the rest as the reply shows
//!   it cannot be used, and the reservation's other parts as the stage
//!   finishes the reply and assembles the scope asked about.
//!   That takes the peer's answer and questions about the scope, at depth
//!   `d` and below on the peer's side, this side's answers to those
//!   questions, at `d + 1`, and the subtree's assembly: work at depths the
//!   induction already grants, apart from the peer's answer at `d`, whose
//!   own slot frees without anything at `d` on this side.
//! - An answer slot at depth `d` frees once the answer's scope is
//!   assembled, which takes its questions, at question depth `d + 1`, and
//!   their subtrees, and once its chunks' last frames are written, which
//!   waits on the pipe alone.
//! - A stage takes its units in order, so a unit waiting at depth `d` waits
//!   only for units ahead of it at `d`, whose slots free by the two cases
//!   above, and for deeper units, which waiters at greater depths precede
//!   for the budget but never for the floor.
//!
//! The invitation share only refuses admissions against the budget, never
//! a floor slot, and a unit it refuses keeps no other unit from the budget
//! or from a slot, so the argument, which uses no budget above zero, covers
//! it unchanged; and every grant it admits against the budget still fits
//! beside what the ledger holds, so the bound above holds with it.
//!
//! Every part of a question's reservation returns no later than the scope
//! it asks about is assembled, when its parent resolution's part returns,
//! so a slot frees at that assembly however early the parked room returns.
//! Returning bytes earlier only adds room against the budget: a grant
//! admitted against the budget fit beside everything the ledger held, and
//! the charges built out of it hold no more than it did, so the bound
//! above holds however early its parts return.
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
use std::num::NonZeroU8;
use std::pin::Pin;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::task::{Context, Poll, Waker};

use super::{Account, Charge, widen};
use crate::counter;
use crate::tree::mirror::streaming::holding::{Holding, held_by};

/// The depths a scope can sit at, the root's 0 through the leaves' 32,
/// each with one floor slot per kind of unit.
pub(super) const DEPTHS: usize = 33;

/// The floor slots of one ledger: one per kind of unit at each depth.
pub(super) const SLOTS: usize = 2 * DEPTHS;

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
    fn place(self) -> Holding {
        match self {
            Self::Answer => Holding::Resolutions,
            Self::Question => Holding::Questions,
        }
    }
}

/// The units waiting for one ledger's admission, the bytes the budget
/// counts, what each kind and depth of unit holds against the budget, what
/// its floor slots carry, and the invitation share's claims and pace.
///
/// Its lock also orders the taking and freeing of floor slots: a slot's
/// count of sharers moves to or from zero only under it ([`Slot`]).
pub(super) struct Queue {
    /// The units waiting, in no particular order.
    waiting: Vec<Arc<Waiter>>,
    /// The order number the next waiting unit takes.
    next: u64,
    /// The bytes the budget counts, and the state of each floor slot's kind
    /// and depth, which only admission's critical sections touch.
    slots: Slots,
    /// The invitation share's state, which only admission's critical
    /// sections touch.
    share: Share,
}

impl Default for Queue {
    /// Returns a queue with no unit waiting, no slot carrying, and no
    /// claim or finished reply.
    fn default() -> Self {
        Self {
            waiting: Vec::new(),
            next: 0,
            slots: Slots {
                normal: 0,
                budgeted: [0; SLOTS],
                carried: [0; SLOTS],
                largest: [0; SLOTS],
            },
            share: Share {
                claimed: [0; DEPTHS],
                predicted: [0; DEPTHS],
                actual: [0; DEPTHS],
            },
        }
    }
}

/// The bytes the budget counts, and for each floor slot's kind and depth,
/// what its units hold against the budget, what the slot carries of it,
/// and the most it may carry ([Carrying](self#carrying)).
///
/// The per-slot counts are indexed by [`Slot::index`].
///
/// It lives in the queue, so every change and every read holds the queue's
/// lock: a function that changes it takes it, which only the lock reaches
/// ([`Account::add`], [`Account::sub`]). [`Slot`]'s methods take it as
/// proof of the lock where they test whether a slot is taken or move its
/// count of sharers to or from zero.
pub(super) struct Slots {
    /// Bytes held outside floor slots, neither through a slot's grant nor
    /// carried by a slot: what the budget counts, `L − F`, which the rule's
    /// step 2 reads ([`decide`]).
    ///
    /// It is a count of its own, changed by the same operations that change
    /// the totals it is part of, rather than the difference of the total
    /// held, the floor part, and what slots carry: only builds that observe
    /// ledgers keep those totals.
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
    /// Bytes held by charges built out of grants admitted against the
    /// budget, by the kind and depth of their unit, the bytes floor slots
    /// carry for them included.
    ///
    /// Less what its slot carries, the questions' count at a depth is the
    /// room that depth's invitations hold against the budget, `I_d` in the
    /// invitation share's check; each count is what a freed slot of its kind
    /// and depth can carry. A charge admitted against the budget is built
    /// under the lock ([`decide`]), an item's shortfall joins under it
    /// ([`shortfall`]), and a charge's drop returns its bytes under it
    /// ([`release`]), so each check and carry reads exactly what the budget
    /// holds of each kind and depth.
    budgeted: [usize; SLOTS],
    /// The bytes each slot carries for charges of its kind and depth
    /// admitted against the budget; the account's `carried` is their sum.
    carried: [usize; SLOTS],
    /// The largest grant admitted to a unit of each slot's kind and depth,
    /// through the slot or against the budget: the most the slot carries.
    largest: [u32; SLOTS],
}

impl Slots {
    /// The bytes charges of `unit` kind at `depth` hold against the budget,
    /// for the account to change as they are charged and returned.
    pub(super) fn budgeted(&mut self, unit: Unit, depth: usize) -> &mut usize {
        &mut self.budgeted[Slot::new(unit, depth).index()]
    }

    /// Note a grant of `bytes` admitted to a unit of `slot`'s kind and
    /// depth.
    fn admitted(&mut self, slot: Slot, bytes: u32) {
        let largest = &mut self.largest[slot.index()];
        *largest = (*largest).max(bytes);
    }
}

/// One unit asking for admission: what the rule decides it by.
#[derive(Clone, Copy)]
struct Request {
    /// The unit's kind.
    unit: Unit,
    /// The depth of the scope it concerns.
    depth: usize,
    /// The most its items can hold.
    bytes: u32,
    /// For a question under the invitation share, its need: the bytes
    /// processing its reply is predicted to admit one level down, which its
    /// claim weighs ([`decide`]); `None` for every other unit.
    need: Option<usize>,
}

/// What the rule decides for one unit.
enum Decision {
    /// The unit is admitted with this grant.
    Grant(Charge),
    /// The unit waits for room in the budget, and keeps every shallower
    /// unit waiting for it too.
    Room,
    /// The unit's grant fits the budget, but its claim does not fit beside
    /// its depth's outstanding claims.
    ///
    /// The invitation share holds it back until the depth's stage processes
    /// more of its replies; shallower units may still take the budget.
    Share,
}

/// The pace, `α`, of one depth's replies: the factor that scales each
/// predicted need in the depth's claims, as a ratio of two byte totals
/// ([`Share::pace`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Pace {
    /// The bytes the answers built by processing the depth's finished
    /// replies have kept.
    pub(super) actual: u64,
    /// The needs predicted for those replies; never zero.
    pub(super) predicted: u64,
}

impl Pace {
    /// α of one: the prediction taken as it stands.
    pub(super) const PREDICTED: Self = Self {
        actual: 1,
        predicted: 1,
    };

    /// `need` scaled by α, rounded down.
    pub(super) fn scale(self, need: usize) -> u128 {
        need as u128 * u128::from(self.actual) / u128::from(self.predicted)
    }
}

/// The invitation share's state for each depth: its outstanding claims and
/// the two sums of its pace
/// ([The invitation share](self#the-invitation-share)).
///
/// It lives in the queue, so every change and every check holds the queue's
/// lock: a check reads a depth's claims and pace as whole admissions and
/// whole finished replies left them, never a finish half applied.
struct Share {
    /// The needs of each depth's questions the invitation share admitted
    /// against the budget, whose replies this side's stage has not
    /// finished: each depth's outstanding claims, `N_d` in the share's check.
    ///
    /// A need joins as its question is admitted and withdraws as the stage
    /// finishes the reply ([`processed`]). A question admitted but never
    /// built, which happens only as its session ends, leaves its need behind
    /// in a ledger nothing admits against again.
    claimed: [usize; DEPTHS],
    /// The needs predicted for the replies to questions about scopes at
    /// each depth, over the replies this side's stage has finished: the
    /// pace's predicted bytes for each depth.
    predicted: [u64; DEPTHS],
    /// Bytes the answers one level below each depth had kept as its latest
    /// reply finished: what processing its finished replies built, the
    /// pace's actual bytes for each depth.
    actual: [u64; DEPTHS],
}

impl Share {
    /// Add a question's `need` to the outstanding claims at `depth`, as the
    /// invitation share admits it against the budget.
    ///
    /// The sum saturates rather than wraps: reaching `usize::MAX` takes more
    /// live questions than an address space holds, and a saturated sum only
    /// holds every later question at the depth to its floor slot.
    fn claim(&mut self, depth: usize, need: usize) {
        self.claimed[depth] = self.claimed[depth].saturating_add(need);
    }

    /// Note that this side's stage has finished a reply to a question about
    /// a scope at `depth`, whose need was `need`.
    ///
    /// The question took its floor slot if `floor`, and the answers one
    /// level down have kept `answers` bytes
    /// ([`Ledger::processed`](super::Ledger::processed)).
    ///
    /// A question on its floor slot claimed nothing; any other withdraws its
    /// need. A need withdraws only after it was claimed, both under the
    /// queue's lock, so the claims never fall below it; the subtraction
    /// saturates so that a saturated sum ([`claim`](Self::claim)) cannot
    /// wrap on its way down. The predicted sum saturates too, for the same
    /// reason.
    fn finish(&mut self, depth: usize, need: usize, floor: bool, answers: u64) {
        if !floor {
            let claimed = &mut self.claimed[depth];
            debug_assert!(
                *claimed >= need,
                "a finished reply withdraws only a need its question claimed",
            );
            *claimed = claimed.saturating_sub(need);
        }
        self.predicted[depth] = self.predicted[depth].saturating_add(counter::widen(need));
        self.actual[depth] = answers;
    }

    /// The pace of the replies to questions about scopes at `depth`: the one
    /// place `α` is chosen ([`Pace`]).
    ///
    /// A need predicts the answers one level down that processing a reply
    /// builds, and nothing but that processing builds answers there. So `α`
    /// is the bytes those answers had kept out of their grants as the latest
    /// reply finished, over the needs predicted for the replies finished so
    /// far ([`Ledger::processed`](super::Ledger::processed)): a ratio of
    /// sums, the mean and not a typical reply, since memory adds. Reading
    /// both at a reply's finish keeps a reply in progress out of both sums,
    /// so its prediction never stands against answers it has yet to build.
    /// `α` is one until a reply has finished, the prediction taken as it
    /// stands, and may exceed one: the prediction is a lower bound only for
    /// replies that list every grandchild their bounds allow, and answers
    /// can keep more than their least.
    ///
    /// It follows only what this side's own processing admits, so it is
    /// deterministic wherever the session is, and it paces admissions only:
    /// the bound and the liveness argument never read it.
    fn pace(&self, depth: usize) -> Pace {
        let predicted = self.predicted[depth];
        if predicted == 0 {
            return Pace::PREDICTED;
        }
        Pace {
            actual: self.actual[depth],
            predicted,
        }
    }
}

/// One unit waiting for admission.
struct Waiter {
    /// The unit asking.
    request: Request,
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

/// A floor slot, named by its kind and depth in one byte.
///
/// Every charge built out of a floor grant names its slot, and a charge's
/// size prices every item that carries one, so the name takes one byte and
/// the slot's state lives in the ledger: a count of the live charges
/// sharing it (the account's `sharing`). Admission takes a slot by raising
/// its count from zero to one, for the grant; each part split off a charge
/// adds one; each charge that drops, or is folded into another, takes one
/// away once its bytes have returned. So a slot is taken exactly while its
/// count is above zero, and frees when the last charge built out of its
/// unit's grant is released, as the rule requires.
///
/// A slot that carries bytes for units admitted against the budget
/// ([Carrying](self#carrying)) counts its carry as its one sharer, which
/// leaves once every carried byte has returned.
///
/// Every change that can move a count to or from zero is made under the
/// queue's lock: only [`decide`] and [`carry`] take a slot
/// ([`Slot::take`]), and every count falls in [`release`] or [`fold`]
/// ([`Slot::leave`]). Each of these, and the test of whether a slot is
/// taken ([`Slot::taken`]), takes the queue's [`Slots`], which only the lock
/// reaches, so the compiler holds them to it. The one change made outside
/// the lock is a split's rise ([`Slot::share`]), made through a charge that
/// shares the slot before and after it. So while the lock is held, a slot is
/// taken or free as the critical section's own steps leave it, and every
/// count is one location, read and written on its own.
///
/// A charge returns its bytes and leaves its slot in one critical section
/// ([`release`]), which admits the waiting units before it ends
/// ([`admit_waiting`]): a unit waiting for the slot takes it there, and a
/// unit that takes it in a later critical section finds the bytes returned.
/// A unit queued after that section is decided under the lock as it queues
/// ([`Admit::new`]), so no unit is left waiting at a free slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Slot(NonZeroU8);

/// Every slot's index, and one more, fits the byte that names a slot.
const _: () = assert!(SLOTS < u8::MAX as usize);

impl Slot {
    /// The slot of `unit` kind at `depth`, which must be below [`DEPTHS`].
    fn new(unit: Unit, depth: usize) -> Self {
        let index = u8::try_from(unit.row() * DEPTHS + depth).ok();
        Self(
            index
                .and_then(|index| NonZeroU8::MIN.checked_add(index))
                .expect("every slot's index, and one more, fits a byte"),
        )
    }

    /// The slot's index among the ledger's [`SLOTS`].
    pub(super) fn index(self) -> usize {
        usize::from(self.0.get() - 1)
    }

    /// The kind of unit the slot serves and the depth of the scopes they
    /// concern: inverse to [`Slot::new`].
    fn serves(self) -> (Unit, usize) {
        let unit = if self.index() < DEPTHS {
            Unit::Answer
        } else {
            Unit::Question
        };
        (unit, self.index() % DEPTHS)
    }

    /// Whether this slot is taken: whether any charge shares it, or it
    /// carries bytes.
    ///
    /// The caller holds the queue's lock, which `slots` proves, so the
    /// answer stands until the caller's own critical section changes it.
    fn taken(self, account: &Account, _slots: &Slots) -> bool {
        account.sharing[self.index()].load(Ordering::Relaxed) != 0
    }

    /// Take this free slot, for a grant or a carry, as its one sharer.
    ///
    /// The caller holds the queue's lock, which `slots` proves, and has
    /// found the slot free under it.
    fn take(self, account: &Account, _slots: &Slots) {
        account.sharing[self.index()].store(1, Ordering::Relaxed);
    }

    /// Count one more charge sharing this slot: a part split off a charge
    /// that shares it already, and still does.
    ///
    /// The count rises from above zero, so this takes no lock: nothing that
    /// tests or takes a slot under the lock reads a different answer.
    pub(super) fn share(self, account: &Account) {
        account.sharing[self.index()].fetch_add(1, Ordering::Relaxed);
    }

    /// Count one fewer charge sharing this slot, one whose bytes have
    /// returned, and return whether that freed the slot.
    ///
    /// The caller holds the queue's lock, which `slots` proves, and admits
    /// the waiting units in the same critical section if this frees the
    /// slot ([`admit_waiting`]).
    fn leave(self, account: &Account, _slots: &Slots) -> bool {
        account.sharing[self.index()].fetch_sub(1, Ordering::Relaxed) == 1
    }
}

/// Lock `mutex`, whose data no panic can leave inconsistent: each critical
/// section here only flips flags and moves values.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The admission rule: return the grant that gives a unit permission to
/// build and send, or `None` to keep it waiting.
///
/// The unit `request` names is of its kind, concerns a scope at its depth,
/// and holds at most its bytes. The steps are the module's rule;
/// `budget_open` is false once a deeper unit must keep waiting.
///
/// The caller holds the queue's lock, which makes this one of the two
/// places a free slot is taken ([`Slot`]), and passes the slots' and the
/// share's state it keeps. Every value the steps read is one the ledger has
/// held: whether the slot is taken changes only under the lock, and every
/// other value, the bytes the budget counts included, lives in the state
/// the lock guards.
fn decide(
    account: &Arc<Account>,
    slots: &mut Slots,
    share: &mut Share,
    budget_open: bool,
    request: Request,
) -> Decision {
    let Request {
        unit,
        depth,
        bytes,
        need,
    } = request;
    let slot = Slot::new(unit, depth);
    // 1. If the unit's floor slot is free, it takes the slot.
    if !slot.taken(account, slots) {
        slot.take(account, slots);
        slots.admitted(slot, bytes);
        #[cfg(any(test, feature = "test-internals"))]
        account.slot_peaks[unit.row()][depth].fetch_max(widen(bytes), Ordering::Relaxed);
        return Decision::Grant(Charge::granted(
            account,
            slots,
            unit.place(),
            bytes,
            Some(slot),
            Some((unit, depth)),
        ));
    }
    // 2. Otherwise it is admitted against the budget if no deeper unit
    // waits for room and its grant fits beside what the ledger holds
    // outside its floor slots, or it waits.
    if !budget_open || slots.normal.saturating_add(widen(bytes)) > account.budget {
        return Decision::Room;
    }
    // 3. A question under the invitation share is admitted only while its
    // depth's claims, with its own, fit the budget: the room its depth's
    // questions hold and the needs of their unfinished replies, scaled by
    // the depth's pace, beside its own grant and need.
    if let Some(need) = need {
        let invited = slots.budgeted[slot.index()] - slots.carried[slot.index()];
        let claimed = share.claimed[depth];
        let pace = share.pace(depth);
        let sum = invited as u128 + widen(bytes) as u128 + pace.scale(claimed.saturating_add(need));
        #[cfg(any(test, feature = "test-internals"))]
        account.weigh(super::SharePeak {
            sum: usize::try_from(sum).unwrap_or(usize::MAX),
            depth,
            invited,
            grant: widen(bytes),
            claimed,
            need,
            pace_actual: pace.actual,
            pace_predicted: pace.predicted,
            normal: slots.normal,
        });
        if sum > account.budget as u128 {
            return Decision::Share;
        }
        share.claim(depth, need);
    }
    slots.admitted(slot, bytes);
    Decision::Grant(Charge::granted(
        account,
        slots,
        unit.place(),
        bytes,
        None,
        Some((unit, depth)),
    ))
}

/// Admit every waiting unit the ledger now has room for, deepest first.
///
/// A unit that must keep waiting closes the budget to every shallower one,
/// so deeper work is admitted first; a shallower unit may still take its
/// own free floor slot.
///
/// For the session-budget meter, everything this allocates is admission's
/// own ([`Holding::Admission`]), whichever place's release called it.
fn reconsider(account: &Arc<Account>) {
    let queue = lock(&account.admission);
    held_by(Holding::Admission, || admit_waiting(account, queue, None));
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
    slot: Option<Slot>,
    unit: Option<(Unit, usize)>,
) {
    let mut queue = lock(&account.admission);
    let freed = match (slot, unit) {
        (Some(slot), unit) => {
            account.sub(&mut queue.slots, place, bytes, true, unit, 0);
            slot.leave(account, &queue.slots).then_some(slot)
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
            let freed = emptied && slot.leave(account, &queue.slots);
            debug_assert_eq!(
                emptied, freed,
                "a slot carrying bytes is shared by its carry alone"
            );
            freed.then_some(slot)
        }
        (None, None) => {
            account.sub(&mut queue.slots, place, bytes, false, None, 0);
            None
        }
    };
    held_by(Holding::Admission, || admit_waiting(account, queue, freed));
}

/// Count one fewer charge sharing `slot`: one folded into another charge
/// that shares it, whose bytes stay held ([`Charge::absorb`]).
///
/// The count falls under the queue's lock, as every count does ([`Slot`]).
/// The charge folded into still shares the slot, so this cannot free it;
/// were it to, admission would reconsider as a release does.
pub(super) fn fold(account: &Arc<Account>, slot: Slot) {
    let queue = lock(&account.admission);
    if slot.leave(account, &queue.slots) {
        held_by(Holding::Admission, || {
            admit_waiting(account, queue, Some(slot));
        });
    }
}

/// Note that this side's stage has finished a reply to a question about a
/// scope at `depth`, then admit every waiting unit the ledger has room for.
///
/// The question's need was `need`, and it took its floor slot if `floor`
/// ([`Ledger::processed`](super::Ledger::processed)). The claims and the pace change under the queue's lock ([`Share`]), the
/// pace's actual bytes read as the reply finishes: what the answers one
/// level down have kept.
pub(super) fn processed(account: &Arc<Account>, depth: usize, need: usize, floor: bool) {
    let mut queue = lock(&account.admission);
    let answers = account
        .kept
        .get(depth + 1)
        .map_or(0, |kept| kept.load(Ordering::Relaxed));
    queue.share.finish(depth, need, floor, answers);
    held_by(Holding::Admission, || admit_waiting(account, queue, None));
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
pub(super) fn charge(account: &Arc<Account>, place: Holding, bytes: u32) -> Charge {
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

/// The outstanding claims of the questions about scopes at `depth`.
#[cfg(test)]
pub(super) fn claimed(account: &Account, depth: usize) -> usize {
    lock(&account.admission).share.claimed[depth]
}

/// The invitation share's pace at `depth` ([`Share::pace`]).
#[cfg(test)]
pub(super) fn pace(account: &Account, depth: usize) -> Pace {
    lock(&account.admission).share.pace(depth)
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
    widen(lock(&account.admission).slots.largest[Slot::new(unit, depth).index()])
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
        !slot.taken(account, &queue.slots),
        "a slot freed in this critical section stays free until a step in it takes the slot",
    );
    let (unit, depth) = slot.serves();
    if queue
        .waiting
        .iter()
        .any(|waiter| (waiter.request.unit, waiter.request.depth) == (unit, depth))
    {
        return;
    }
    // A free slot carries nothing, so every byte of its kind and depth held
    // against the budget is the budget's.
    let held = queue.slots.budgeted[slot.index()];
    let bytes = held.min(widen(queue.slots.largest[slot.index()]));
    if bytes == 0 {
        return;
    }
    slot.take(account, &queue.slots);
    queue.slots.carried[slot.index()] = bytes;
    account.carry(&mut queue.slots, bytes, unit, depth);
}

/// Let the slot `freed` names carry ([`carry`]), then admit every waiting
/// unit the ledger has room for ([`reconsider`]), under the lock `queue`
/// holds.
fn admit_waiting(account: &Arc<Account>, mut queue: MutexGuard<'_, Queue>, freed: Option<Slot>) {
    if let Some(slot) = freed {
        carry(account, &mut queue, slot);
    }
    let mut woken = Vec::new();
    let Queue {
        waiting,
        slots,
        share,
        ..
    } = &mut *queue;
    if !waiting.is_empty() {
        waiting.sort_by_key(|waiter| (Reverse(waiter.request.depth), waiter.order));
        let mut budget_open = true;
        waiting.retain(
            |waiter| match decide(account, slots, share, budget_open, waiter.request) {
                Decision::Grant(grant) => {
                    let mut state = lock(&waiter.state);
                    state.grant = Some(grant);
                    woken.extend(state.waker.take());
                    false
                }
                Decision::Room => {
                    budget_open = false;
                    true
                }
                Decision::Share => true,
            },
        );
    }
    // Whether the freed slot is taken changes under this lock only by this
    // critical section's own steps, so a slot that reads free here is one
    // its carry and its waiters left free.
    if let Some(slot) = freed.filter(|&slot| !slot.taken(account, &queue.slots)) {
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

/// One unit's admission, as the process's admission hook sees it: the unit
/// starting to wait, taking its grant, or its grant returning what the unit
/// did not use.
#[cfg(any(test, feature = "test-internals"))]
#[derive(Clone, Copy, Debug)]
pub struct Admission {
    /// The ledger that admitted the unit, identified as in
    /// [`LedgerChange::ledger`](super::LedgerChange::ledger).
    pub ledger: usize,
    /// The unit's order among the units its ledger queued, which names it
    /// for as long as the ledger lives.
    ///
    /// It is `None` for a unit admitted at once, which was never queued and
    /// never waits, and in a report of a grant's return, which names no
    /// unit.
    pub order: Option<u64>,
    /// Whether this reports the unit starting to wait, because the ledger
    /// did not admit it as it was queued; its admission is reported later,
    /// with `waited` set.
    pub waiting: bool,
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
    /// at once or as it was queued.
    pub waited: bool,
    /// Whether this reports the grant's unused bytes returning once its
    /// unit was built, rather than the grant itself: `bytes` then counts
    /// what returned, and `waited` is false.
    pub leftover: bool,
    /// Whether this leftover is a part the grant returned before its unit
    /// was built, as soon as the unit was known not to need it; the rest
    /// returns, in a report without this flag, once the unit is built.
    pub partial: bool,
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

/// Report a unit starting to wait to the installed hook.
#[cfg(any(test, feature = "test-internals"))]
fn waiting(account: &Account, waiter: &Waiter) {
    if let Some(hook) = ADMISSION_HOOK.get() {
        let Request {
            unit, depth, bytes, ..
        } = waiter.request;
        hook(Admission {
            ledger: std::ptr::from_ref(account) as usize,
            order: Some(waiter.order),
            waiting: true,
            question: unit == Unit::Question,
            depth,
            bytes: widen(bytes),
            floor: false,
            waited: false,
            leftover: false,
            partial: false,
        });
    }
}

/// Report a unit starting to wait: nothing outside test builds.
#[cfg(not(any(test, feature = "test-internals")))]
#[inline(always)]
fn waiting(_: &Account, _: &Waiter) {}

/// Report the unit `request` names taking `grant` to the installed hook,
/// with its queue order if it was queued.
#[cfg(any(test, feature = "test-internals"))]
fn admitted(account: &Account, request: Request, order: Option<u64>, grant: &Charge, waited: bool) {
    if let Some(hook) = ADMISSION_HOOK.get() {
        hook(Admission {
            ledger: std::ptr::from_ref(account) as usize,
            order,
            waiting: false,
            question: request.unit == Unit::Question,
            depth: request.depth,
            bytes: widen(request.bytes),
            floor: grant.is_floor(),
            waited,
            leftover: false,
            partial: false,
        });
    }
}

/// Report a grant of `unit` kind at `depth` returning the `bytes` its unit
/// did not use, through a floor slot if `floor`, to the installed hook,
/// before its unit is built if `partial`.
#[cfg(any(test, feature = "test-internals"))]
pub(super) fn settled(
    account: &Account,
    unit: Unit,
    depth: usize,
    bytes: usize,
    floor: bool,
    partial: bool,
) {
    if let Some(hook) = ADMISSION_HOOK.get() {
        hook(Admission {
            ledger: std::ptr::from_ref(account) as usize,
            order: None,
            waiting: false,
            question: unit == Unit::Question,
            depth,
            bytes,
            floor,
            waited: false,
            leftover: true,
            partial,
        });
    }
}

/// Report a unit taking its grant: nothing outside test builds.
#[cfg(not(any(test, feature = "test-internals")))]
#[inline(always)]
fn admitted(_: &Account, _: Request, _: Option<u64>, _: &Charge, _: bool) {}

/// Wait for a unit's admission ([`Ledger::admit`](super::Ledger::admit)).
pub(crate) struct Admit {
    /// The ledger admitting the unit.
    account: Arc<Account>,
    /// The unit's grant or its place among the waiting units, until its
    /// task takes the grant.
    state: Option<Admitting>,
    /// Whether the unit was not admitted as it was queued.
    waited: bool,
}

/// Where an [`Admit`] stands before its task takes the grant.
enum Admitting {
    /// Admitted at once, with no unit waiting: the grant, which no other
    /// task can reach, and the unit it admits.
    Granted(Request, Charge),
    /// Queued among the waiting units, where [`reconsider`] leaves the grant.
    Queued(Arc<Waiter>),
}

impl Admit {
    /// Admit a unit of `unit` kind concerning a scope at `depth`, whose
    /// items take at most `bytes`, at once if nothing waits and the rule
    /// allows, or queue it and admit what the ledger has room for.
    ///
    /// For the session-budget meter, a waiting unit is admission's own
    /// ([`Holding::Admission`]); a unit admitted at once allocates nothing.
    ///
    /// A question whose `need` is given is governed by the invitation share,
    /// its reply predicted to need that many bytes processed.
    pub(super) fn new(
        account: Arc<Account>,
        unit: Unit,
        depth: usize,
        bytes: u32,
        need: Option<usize>,
    ) -> Self {
        assert!(depth < DEPTHS, "a scope sits at most 32 bytes deep");
        let request = Request {
            unit,
            depth,
            bytes,
            need,
        };
        held_by(Holding::Admission, || Self::queue(account, request))
    }

    /// Admit a unit at once or queue it ([`Admit::new`]).
    fn queue(account: Arc<Account>, request: Request) -> Self {
        let waiter = {
            let mut queue = lock(&account.admission);
            // With no unit waiting, this unit is the only one `reconsider`
            // would consider, with the budget open, so deciding it here
            // admits exactly what queueing it would, without building a
            // waiter. The lock is held from the decision to the queueing,
            // so a unit this refuses queues as if it had never been tried.
            let Queue {
                waiting,
                slots,
                share,
                ..
            } = &mut *queue;
            if waiting.is_empty()
                && let Decision::Grant(grant) = decide(&account, slots, share, true, request)
            {
                drop(queue);
                return Self {
                    account,
                    state: Some(Admitting::Granted(request, grant)),
                    waited: false,
                };
            }
            let order = queue.next;
            queue.next += 1;
            let waiter = Arc::new(Waiter {
                request,
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
            waiting(&account, &waiter);
        }
        Self {
            account,
            state: Some(Admitting::Queued(waiter)),
            waited,
        }
    }
}

impl Future for Admit {
    type Output = Charge;

    /// Return the grant once the unit is admitted.
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Charge> {
        let (request, order, grant) = match self
            .state
            .take()
            .expect("an admission is not polled after it returns its grant")
        {
            Admitting::Granted(request, grant) => (request, None, grant),
            Admitting::Queued(waiter) => {
                let mut state = lock(&waiter.state);
                let Some(grant) = state.grant.take() else {
                    state.waker = Some(held_by(Holding::Admission, || cx.waker().clone()));
                    drop(state);
                    self.state = Some(Admitting::Queued(waiter));
                    return Poll::Pending;
                };
                (waiter.request, Some(waiter.order), grant)
            }
        };
        admitted(&self.account, request, order, &grant, self.waited);
        Poll::Ready(grant)
    }
}

impl Drop for Admit {
    /// Withdraw a unit no longer awaited, returning a grant it never took.
    fn drop(&mut self) {
        let Some(Admitting::Queued(waiter)) = self.state.take() else {
            // A grant made at once returns as it drops here, which
            // reconsiders the waiting units itself.
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
