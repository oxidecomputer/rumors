//! Admission: when a stage may build its next answer or question, and what
//! of it the ledger charges against the memory budget.
//!
//! On each side of a session, one task per tree depth, a *stage*, reads the
//! peer's replies about scopes at that depth. For each disputed scope it builds
//! an answer, and for each child it disputes, a question for the peer. Before
//! building either, the stage asks its side's ledger for a *grant*: a
//! [`Charge`] of bytes covering everything that answer or question will hold,
//! which the code calls a *unit*. The ledger sorts units by *kind and depth*
//! (answer or question, and the depth of the scope it concerns) and counts the
//! bytes each kind and depth's grants hold. It lets each kind and depth hold
//! one largest unit's worth of bytes without charging the budget; a unit
//! granted that way is *exempt*. It charges the budget for whatever a kind and
//! depth holds beyond that, and makes a stage wait when a grant would take that
//! charge past the budget, or, for a question that invites a reply, past its
//! depth's share of the budget. Within a kind and depth, the ledger grants in
//! the order the stages asked. It decides deeper kinds and depths first, but
//! never makes an exempt unit wait on another kind and depth. So a side never
//! holds more than its budget plus one largest unit per kind and depth. And no
//! stage waits forever: each grant gives its bytes back once the deeper stages
//! finish the subtree it concerns (and, for a question, once the peer has
//! answered it), so each kind and depth in turn holds nothing, the ledger
//! grants its first waiting unit as exempt, and the session completes from the
//! leaves up.
//!
//! An answer's unit ([`Unit::Answer`]) holds its resolution and its reply's
//! reactions that ask nothing. A question's unit ([`Unit::Question`]) holds
//! its query, its chunk, its record, and a reservation for the peer's reply:
//! mostly *the reply's room*, where the reply's chunks park as they arrive,
//! and also the records the reply's questions derive and the parent
//! resolution that reassembles the scope asked about. A stage builds each item
//! as a part split off the unit's grant ([`Charge::take`](super::Charge::take))
//! and drops what is left; [the ledger's docs](super) say when each part
//! returns.
//!
//! # Kinds and depths, bounds, and the excess
//!
//! - `k = (kind, d)` is a kind and depth. `d` runs from the root's 0 to the
//!   leaves' 32: the depth of the scope a question asks about, or an answer
//!   answers.
//! - `held[k]` is the bytes held by the live charges from grants to units of
//!   `k`, parts split off them included.
//! - `F[k]`, the *bound*, is the most one grant to a unit of `k` can hold
//!   ([`Bounds`]).
//! - The *excess* `E = Σ_k max(0, held[k] − F[k])` is all the ledger charges
//!   against its *budget* `B`. `L = Σ_k held[k]` is all it holds.
//! - `A`, the *floor allowance*, is the larger, over a session's two sides, of
//!   `Σ F[k]` over the kinds and depths that side uses.
//! - A *waiter* has asked for a grant and not received one; it is *first in
//!   line* when no earlier unit of its kind and depth waits. A *critical
//!   section* is one hold of the admission queue's lock.
//!
//! # The rule
//!
//! Under the queue's lock, [`decide`] judges a unit of `k = (kind, d)` with
//! grant `c` against [`Counts`], given a flag `open` that is false once an
//! earlier unit in the same critical section has waited for room:
//!
//! 1. **Exempt.** If `held[k] = 0` or `held[k] + c ≤ F[k]`, `decide` grants it.
//! 2. **Room.** Otherwise, if `open` is false or
//!    `E′ = E − max(0, held[k] − F[k]) + max(0, held[k] + c − F[k]) > B`,
//!    the unit waits *for room*, and `open` becomes false.
//! 3. **Share.** Otherwise, a question under the invitation share waits *for
//!    its depth's replies* unless `I_d + c + α · (N_d + n) ≤ B`
//!    ([below](#the-invitation-share)), leaving `open` as it was.
//! 4. **Grant.** Otherwise `decide` grants it against the budget, and a
//!    question under the share adds its need `n` to `N_d`.
//!
//! [`Admit::new`] decides a unit at once if nothing waits. Otherwise the unit
//! joins the waiters, and the ledger runs a *reconsideration*: in one critical
//! section, with `open` starting true, [`admit_waiting`] decides every waiter,
//! deepest first and then in the order they asked. The ledger reconsiders
//! whenever a unit joins the waiters, a charge returns bytes, a stage finishes
//! a reply, or a waiter withdraws.
//!
//! **First come, first served, within a kind and depth only.** Once
//! `admit_waiting` refuses a unit of `k`, it skips `k`'s later waiters for the
//! rest of the critical section, leaving `open` as it was, so the ledger grants
//! no unit of `k` while an earlier one waits. Exemption depends only on a
//! unit's own grant, so without the skip a stream of small units could starve
//! a larger earlier one. One order across all kinds and depths would make an
//! exempt unit wait behind units whose release needs it.
//!
//! # Premises
//!
//! - **(P1) Every grant is within its bound,** `c ≤ F[k]`: [`grant_bound`]
//!   evaluates each grant formula, which grows with its arguments, at the
//!   largest arguments a stage can pass. The test kit checks every admission.
//! - **(P2) One ledger holds one side's kinds and depths:** one side asks each
//!   question and the other answers it, from the initiator's opening question
//!   at depth 0, and each ledger is fresh per session. The test kit checks it.
//! - **(P3) A charge's bytes keep one kind and depth for life:** only grants
//!   and splits create charges, a split keeps its origin, no charge merges
//!   into another, and the opening question takes exactly one question's.
//! - **(P4) The ledger changes and reads its counts only under the queue's
//!   lock** ([`Counts`]). Charges with no unit, which only tests make, count
//!   in a class of their own whose bound is zero.
//! - **(P5) No item needs more than its grant has left** (no *shortfall*):
//!   the ledger counts any shortfall in `short`, and the session-budget suite
//!   fails on it.
//! - **(P6) A stage asks for an answer's questions before its next answer:**
//!   `Work::answering_walk` awaits each admission in turn. A second producer
//!   at a kind and depth would cost parallelism, never liveness.
//! - **(P7) A unit's return needs only deeper work:** provided the pipe
//!   drains, a unit at depth `d` returns its grant once units deeper than `d`,
//!   on either side, and for a question the peer's answer to it, have been
//!   admitted and returned theirs. A deeper stage asks for each, or the unit's
//!   own stage before its next answer; none is a later unit of `k`.
//! - **(P8) The pipe drains,** so only the ledger can hold a stage (the pipe
//!   module's wait table). (P6) to (P8) are *conforming traffic*.
//!
//! [`grant_bound`]: crate::tree::mirror::streaming::materialized::grant_bound
//!
//! # The memory bound
//!
//! **Claim.** Under (P1) to (P5), outside critical sections, `E ≤ B`, so
//! `L = Σ_k min(held[k], F[k]) + E ≤ A + B`.
//!
//! **Argument.** A new ledger has `E = 0`. By (P3) and (P4), `E` changes only
//! in a critical section: an exempt grant leaves `k`'s excess at 0 (by (P1)
//! when `held[k] = 0`), a grant at step 4 passed step 2, a return or split
//! cannot raise it, and (P5) rules out shortfalls. By (P2), only this side's
//! kinds and depths hold bytes, so `Σ_k min(held[k], F[k]) ≤ A`. The ledger's
//! budget is a session's memory budget less `A` and the session allowance
//! ([`session_allowance`]), its state outside the ledger.
//!
//! [`session_allowance`]: crate::tree::mirror::streaming::materialized::session_allowance
//!
//! # Every unit is admitted
//!
//! **Claim.** Under conforming traffic and (P3) to (P5), the two ledgers
//! eventually grant every unit, at every pair of budgets and every table of
//! bounds, zeros included. The argument uses only step 1.
//!
//! **Argument.** By induction on depth, deepest first, over both sides at
//! once, and within a depth over answers before questions and along the order
//! units of `k` asked. Take a waiter `W` of `k`, with every deeper unit and
//! every earlier unit of `k` eventually granted and returning its grant; so
//! `W` becomes first in line. The ledger then grants no unit of `k` before `W`
//! (first come, first served), and no shortfall happens (P5), so `held[k]`
//! only falls. Each unit holding `k` returns its grant after deeper work and,
//! for a question, the peer's answer at depth `d` (P7); that work includes its
//! own questions, which its stage asks for without waiting on `k` (P6). When a
//! return empties `k`, or `W` joins an empty `k`, the reconsideration in that
//! critical section (P4) decides `W`, first in line, and grants it at step 1.
//! Only the ledger can hold a stage (P8), so every session completes.
//!
//! Step 1 rescues three schedules that would otherwise deadlock:
//!
//! - **The deeper-first closure:** a deeper unit waiting for room closes the
//!   budget, but needs only its own kind and depth to empty.
//! - **Hold and wait:** an answer's resolution waits for its later questions,
//!   which wait only on earlier questions of their own kind and depth.
//! - **The share's lone question:** a claim larger than `B` has no replies to
//!   wait for, but its question becomes exempt once its depth empties.
//!
//! # Monotone in the budget
//!
//! **Claim (M).** Start a critical section in one state, with the same
//! waiters, at budgets `B ≤ B′`. Up to the first decision whose outcome
//! differs, both budgets decide and skip alike; at it, `B′` grants where `B`
//! waits, or waits for the share where `B` waits for room. The ledger's state
//! depends on the schedule, never on the budget, and the two waits change no
//! state, so the first unit one session grants and the other does not is
//! granted at `B′`. The argument: only steps 2 and 3 read the budget; a unit
//! refused at step 2 at `B′` is refused at step 2 at `B`, and one refused at
//! step 3 at `B′` is refused at step 2 or 3 at `B`; and equal outcomes leave
//! equal states. (M) does not promise that every unit
//! is admitted at least as soon at `B′`: what follows a first difference is
//! greedy repacking, as in Graham's anomalies of list scheduling.
//!
//! ## The no-cost point
//!
//! In the session with no budget, `Λ` is the largest `E′` that `decide` weighs
//! at step 2, and the *share peak* the largest sum it weighs at step 3. At or
//! above their maximum, the session's *no-cost point*, no unit waits and the
//! session is the unlimited one, event for event; below it, some unit waits.
//! Identity holds from the no-cost point, not from `Λ`: to stop the first
//! round's burst, the share must hold back some question that fits.
//!
//! # What must stay
//!
//! - **Step 1 comes first, reads no budget, and grants at an empty kind and
//!   depth at least**, past a closed budget, the share, and other kinds and
//!   depths' waiters.
//! - **First come, first served within each kind and depth, and only there.**
//! - **(P6), and a reconsideration on every return** (P4).
//! - **Answers and questions are separate kinds:** with zero bounds, an answer
//!   sharing its questions' kind and depth would block them, and it returns
//!   only after them.
//! - **Step 2 precedes step 3**, so a refusal at `B′` that closes the budget
//!   implies one at `B`.
//! - **A unit refused for room closes the budget, whatever its size.** If a
//!   unit too large for `B` waited without closing, a larger budget for which
//!   it is not too large would close where `B` stays open, breaking (M).
//!
//! # Boundary cases
//!
//! - **A budget of zero:** only exempt units are granted, and sessions still
//!   pipeline, within `A`.
//! - **Zero bounds with a budget of zero**, the *serial configuration* tests
//!   use ([`Bounds::ZERO`]): one unit per kind and depth at a time. (P1) fails,
//!   so the memory bound does not apply.
//! - **A grant above its bound** ((P1) broken): step 1's `held[k] = 0` clause
//!   still grants it, and its overshoot may take `E` past `B`; at most one
//!   uncounted overshoot per kind and depth is outstanding.
//! - **A shortfall** ((P5) broken) may take `E` past `B` by its size.
//! - **An unlimited budget:** nothing waits.
//!
//! # The invitation share
//!
//! A question *invites* its reply: its grant holds the reply's room until this
//! side's stage processes the reply, and stages process replies one at a time.
//! Unchecked, the top of a session asks all its questions at once, and their
//! replies fill the budget while they wait. So the ledger lets a depth invite
//! only as many replies as the budget would also let its stage process.
//!
//! Each question asked through [`Ledger::invite`](super::Ledger::invite) and
//! granted at step 4 carries a *claim*, `c + α · n`
//! ([`Charge::claimed`](super::Charge::claimed)). `decide` grants it only
//! while its depth's claims fit the budget together, as Dijkstra's banker
//! lends:
//!
//! > `I_d + c + α · (N_d + n) ≤ B`,
//!
//! where `I_d` is the excess its depth's questions hold and `N_d` the needs of
//! those that claimed and whose replies the stage has not finished
//! ([`Ledger::processed`](super::Ledger::processed)). An exempt question
//! claims nothing.
//!
//! - **The need `n`** is the least the stage can build while processing the
//!   reply: the answers one level down for a reply that lists every grandchild
//!   its bound allows, at their least cost (`charges::processing_need`).
//! - **The pace `α`** ([`Share::pace`]) corrects that lower bound: the bytes
//!   those answers have kept, over the needs predicted, both read as each reply
//!   finishes; it is one until a reply finishes.
//! - **The claims nest:** a depth-`d` claim covers only the answers at `d + 1`,
//!   while the questions at `d + 2` claim at their own depth, so each byte
//!   counts toward one depth's sum. One pooled check would have to cover whole
//!   subtrees and would refuse every later question.
//!
//! The memory bound and the liveness argument never read the share.

use std::cmp::Reverse;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::task::{Context, Poll, Waker};

use super::{Account, Charge, widen};
use crate::counter;
use crate::tree::mirror::streaming::holding::Holding;

/// The depths a scope can sit at, the root's 0 through the leaves' 32.
pub(super) const DEPTHS: usize = 33;

/// The kinds and depths of unit one ledger counts: one per kind of unit at
/// each depth.
const KIND_DEPTHS: usize = 2 * DEPTHS;

/// The two kinds of unit, each counted apart at every depth.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Unit {
    /// An answer's own items: everything it builds but its questions.
    Answer,
    /// One question's built items and the reservation for its reply.
    Question,
}

impl Unit {
    /// The index of this kind at `depth` among a ledger's
    /// [`KIND_DEPTHS`], which `depth` must be below [`DEPTHS`] to fit.
    const fn index(self, depth: usize) -> usize {
        let row = match self {
            Self::Answer => 0,
            Self::Question => 1,
        };
        row * DEPTHS + depth
    }

    /// The place a grant of this kind is charged to until a stage builds
    /// its items from it.
    fn place(self) -> Holding {
        match self {
            Self::Answer => Holding::Resolutions,
            Self::Question => Holding::Questions,
        }
    }
}

/// Every kind and depth's bound `F[k]`, the most one grant to a unit of `k`
/// can hold ([Kinds and depths](self#kinds-and-depths-bounds-and-the-excess)).
///
/// A session's ledger takes the bounds computed from the types
/// ([`grant_bounds`](crate::tree::mirror::streaming::materialized::grant_bounds));
/// a test may pass [`Bounds::ZERO`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Bounds([usize; KIND_DEPTHS]);

impl Bounds {
    /// Every bound zero: with a budget of zero, the *serial configuration*,
    /// which grants only at an empty kind and depth
    /// ([Boundary cases](self#boundary-cases)).
    pub(crate) const ZERO: Self = Self([0; KIND_DEPTHS]);

    /// These bounds, with that of `unit` kind at `depth` set to `bound`.
    ///
    /// # Panics
    ///
    /// Panics if `depth` exceeds 32, the depth of a leaf: a programmer
    /// error, since every scope is a prefix of a 32-byte path.
    pub(crate) const fn with(mut self, unit: Unit, depth: usize, bound: usize) -> Self {
        assert!(depth < DEPTHS, "a scope sits at most 32 bytes deep");
        self.0[unit.index(depth)] = bound;
        self
    }

    /// The bound of `unit` kind at `depth`, which must be below [`DEPTHS`].
    fn of(&self, unit: Unit, depth: usize) -> usize {
        self.0[unit.index(depth)]
    }
}

/// One ledger's waiters, what each kind and depth holds, the excess, and
/// the invitation share's claims and pace.
pub(super) struct Queue {
    /// The units waiting, in no particular order.
    waiting: Vec<Arc<Waiter>>,
    /// The order number the next waiting unit takes.
    next: u64,
    /// What each kind and depth holds, and the ledger's excess.
    counts: Counts,
    /// The invitation share's claims and pace.
    share: Share,
}

impl Default for Queue {
    /// Returns a queue with no unit waiting, nothing held, and no claim or
    /// finished reply.
    fn default() -> Self {
        Self {
            waiting: Vec::new(),
            next: 0,
            counts: Counts {
                held: [0; KIND_DEPTHS],
                excess: 0,
            },
            share: Share {
                claimed: [0; DEPTHS],
                predicted: [0; DEPTHS],
                actual: [0; DEPTHS],
            },
        }
    }
}

/// What each kind and depth holds, `held[k]`, and the ledger's excess `E`
/// ([Kinds and depths](self#kinds-and-depths-bounds-and-the-excess)).
///
/// It lives in the queue, so only a holder of the queue's lock can reach it
/// (premise (P4)), and [`Account::add`] and [`Account::sub`] take it by
/// reference.
///
/// `E` is a running total: each change moves it by the change in its own
/// kind and depth's excess, so no decision sums the table. A charge with no
/// unit, which only tests make, adds all its bytes to it.
pub(super) struct Counts {
    /// The bytes every live charge built out of grants to units of each kind
    /// and depth holds, indexed by [`Unit::index`].
    held: [usize; KIND_DEPTHS],
    /// The excess, `E`, which [`decide`] weighs at step 2.
    excess: usize,
}

/// The excess of a kind and depth that holds `held` under the bound
/// `bound`: what it holds beyond the bound.
fn excess_beyond(held: usize, bound: usize) -> usize {
    held.saturating_sub(bound)
}

impl Counts {
    /// Count `bytes` more held by a charge of `unit`, under `bounds`: a
    /// grant, or an item's shortfall.
    pub(super) fn add(&mut self, bounds: &Bounds, unit: Option<(Unit, usize)>, bytes: usize) {
        let Some((unit, depth)) = unit else {
            self.excess += bytes;
            return;
        };
        let (held, bound) = (&mut self.held[unit.index(depth)], bounds.of(unit, depth));
        let before = excess_beyond(*held, bound);
        *held += bytes;
        self.excess += excess_beyond(*held, bound) - before;
    }

    /// Count `bytes` fewer held by a charge of `unit`, under `bounds`: a
    /// charge returning bytes it holds.
    pub(super) fn sub(&mut self, bounds: &Bounds, unit: Option<(Unit, usize)>, bytes: usize) {
        let Some((unit, depth)) = unit else {
            self.excess -= bytes;
            return;
        };
        let (held, bound) = (&mut self.held[unit.index(depth)], bounds.of(unit, depth));
        let before = excess_beyond(*held, bound);
        *held -= bytes;
        self.excess -= before - excess_beyond(*held, bound);
    }

    /// The ledger's excess, `E`, for the observation hook to copy.
    #[cfg(any(test, feature = "test-internals"))]
    pub(super) fn excess(&self) -> usize {
        self.excess
    }
}

/// One unit asking for a grant: what [`decide`] judges it by.
#[derive(Clone, Copy)]
struct Request {
    /// The unit's kind.
    unit: Unit,
    /// The depth of the scope it concerns.
    depth: usize,
    /// The most its items can hold.
    bytes: u32,
    /// For a question under the invitation share, its predicted need,
    /// which [`decide`] weighs at step 3; `None` for every other unit.
    need: Option<usize>,
}

/// A unit's grant, and whether [`decide`] made it at step 1.
struct Granted {
    /// The grant.
    charge: Charge,
    /// Whether the unit was exempt: admitted at step 1, asking nothing of
    /// the budget.
    exempt: bool,
}

/// What [`decide`] returns for one unit.
enum Decision {
    /// The unit is admitted with this grant.
    Grant(Granted),
    /// The unit waits for room in the budget, which closes the budget to
    /// every unit decided after it in the same critical section.
    Room,
    /// The unit's grant fits the budget, but its claim does not fit beside
    /// its depth's outstanding claims; the budget stays open.
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
/// It lives in the queue, so every check reads a depth's claims and pace
/// under the queue's lock, as whole admissions and finished replies left
/// them, never a finish half applied.
struct Share {
    /// The needs of each depth's questions that claimed (at step 4)
    /// and whose replies this side's stage has not finished: each depth's
    /// outstanding claims, `N_d` in the share's check.
    ///
    /// A need joins as its question is granted and withdraws as the stage
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
    /// Add a question's `need` to the outstanding claims at `depth`, as
    /// [`decide`] grants it against the budget at step 4.
    ///
    /// The sum saturates rather than wraps: reaching `usize::MAX` takes more
    /// live questions than an address space holds, and a saturated sum only
    /// holds every later question at the depth until it is exempt.
    fn claim(&mut self, depth: usize, need: usize) {
        self.claimed[depth] = self.claimed[depth].saturating_add(need);
    }

    /// Note that this side's stage has finished a reply to a question about
    /// a scope at `depth`.
    ///
    /// The question's need was `need`, claimed if `claimed`, and the answers
    /// one level down have kept `answers` bytes
    /// ([`Ledger::processed`](super::Ledger::processed)).
    ///
    /// A need withdraws only after its question claimed it, both under the
    /// queue's lock, so the claims never fall below it. Both sums saturate,
    /// as in [`claim`](Self::claim), so that a saturated sum cannot wrap.
    fn finish(&mut self, depth: usize, need: usize, claimed: bool, answers: u64) {
        if claimed {
            let outstanding = &mut self.claimed[depth];
            debug_assert!(
                *outstanding >= need,
                "a finished reply withdraws only a need its question claimed",
            );
            *outstanding = outstanding.saturating_sub(need);
        }
        self.predicted[depth] = self.predicted[depth].saturating_add(counter::widen(need));
        self.actual[depth] = answers;
    }

    /// The pace `α` of the replies to questions about scopes at `depth`:
    /// the one place `α` is chosen ([`Pace`]).
    ///
    /// Only processing these replies builds answers one level down, so `α`
    /// is the bytes those answers had kept when the latest reply finished,
    /// over the needs predicted for every reply finished so far: a ratio of
    /// sums, since memory adds. Reading both at a reply's finish keeps a
    /// reply in progress out of both. `α` is one until a reply finishes, and
    /// may exceed one, since answers can keep more than their least. It
    /// reads only this side's own processing, so it is deterministic.
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
    /// Its place in the order its ledger's units queued, by which
    /// [`admit_waiting`] decides each depth's waiters.
    order: u64,
    /// The grant once admitted, and the task to wake then.
    state: Mutex<State>,
}

/// What a waiting unit has received.
#[derive(Default)]
struct State {
    /// The grant, once the unit is admitted and until its task takes it.
    grant: Option<Granted>,
    /// The task awaiting the grant.
    waker: Option<Waker>,
}

/// Lock `mutex`, whose data no panic can leave inconsistent: each critical
/// section here only flips flags and moves values.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Judge the unit `request` names by the module's [rule](self#the-rule):
/// return its grant, or why it waits.
///
/// `budget_open` is false once an earlier unit in this critical section
/// has waited for room. The caller holds the queue's lock and passes the
/// counts and share state it guards, so a grant counts its bytes before
/// the lock is released.
fn decide(
    account: &Arc<Account>,
    counts: &mut Counts,
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
    let grant = widen(bytes);
    let held = counts.held[unit.index(depth)];
    let bound = account.bounds.of(unit, depth);
    // Step 1: exempt.
    if held == 0 || held + grant <= bound {
        let charge = Charge::granted(account, counts, unit.place(), bytes, (unit, depth), false);
        return Decision::Grant(Granted {
            charge,
            exempt: true,
        });
    }
    // Step 2: room.
    let with_unit = counts.excess - excess_beyond(held, bound) + excess_beyond(held + grant, bound);
    if !budget_open || with_unit > account.budget {
        return Decision::Room;
    }
    // Step 3: the invitation share.
    if let Some(need) = need {
        let invited = excess_beyond(held, bound);
        let claimed = share.claimed[depth];
        let pace = share.pace(depth);
        let sum = invited as u128 + grant as u128 + pace.scale(claimed.saturating_add(need));
        #[cfg(any(test, feature = "test-internals"))]
        account.weigh(super::SharePeak {
            sum: usize::try_from(sum).unwrap_or(usize::MAX),
            depth,
            invited,
            grant,
            claimed,
            need,
            pace_actual: pace.actual,
            pace_predicted: pace.predicted,
            excess: counts.excess,
        });
        if sum > account.budget as u128 {
            return Decision::Share;
        }
        share.claim(depth, need);
    }
    // Step 4: grant against the budget.
    let charge = Charge::granted(
        account,
        counts,
        unit.place(),
        bytes,
        (unit, depth),
        need.is_some(),
    );
    Decision::Grant(Granted {
        charge,
        exempt: false,
    })
}

/// Run a reconsideration ([`admit_waiting`]).
///
/// The session-budget meter counts what it allocates as admission's own
/// ([`Holding::Admission`]).
fn reconsider(account: &Arc<Account>) {
    let queue = lock(&account.admission);
    Holding::Admission.create(|| admit_waiting(account, queue));
}

/// Return the `bytes` a dropped charge for `place` holds, from the kind and
/// depth `unit` names if the ledger granted it, then run a reconsideration
/// ([`Charge`]'s drop).
///
/// Both happen in one critical section, so no decision sees the bytes
/// counted twice or not at all, and a return that empties a kind and depth
/// decides its first waiter in that same section, as the liveness argument
/// requires.
pub(super) fn release(
    account: &Arc<Account>,
    place: Holding,
    bytes: usize,
    unit: Option<(Unit, usize)>,
) {
    let mut queue = lock(&account.admission);
    account.sub(&mut queue.counts, place, bytes, unit);
    Holding::Admission.create(|| admit_waiting(account, queue));
}

/// Note that this side's stage has finished a reply to a question about a
/// scope at `depth`, whose need was `need`, claimed if `claimed`, then run
/// a reconsideration ([`Ledger::processed`](super::Ledger::processed)).
///
/// It reads what the answers one level down have kept, the pace's actual
/// bytes, under the queue's lock as the reply finishes ([`Share::pace`]).
pub(super) fn processed(account: &Arc<Account>, depth: usize, need: usize, claimed: bool) {
    let mut queue = lock(&account.admission);
    let answers = account
        .kept
        .get(depth + 1)
        .map_or(0, |kept| kept.load(Ordering::Relaxed));
    queue.share.finish(depth, need, claimed, answers);
    Holding::Admission.create(|| admit_waiting(account, queue));
}

/// Add the `bytes` an item for `place` needed beyond the grant it was built
/// out of, for a charge of `unit` ([`Charge::take`](super::Charge::take)).
///
/// It takes the queue's lock, as every change to the counts does
/// ([`Counts`]), so the caller must not hold it. Only a grant that fails to
/// bound its unit, a conformance bug (premise (P5)), lets an item fall
/// short, so the lock costs a conforming session nothing.
pub(super) fn shortfall(
    account: &Account,
    place: Holding,
    bytes: usize,
    unit: Option<(Unit, usize)>,
) {
    let mut queue = lock(&account.admission);
    account.add(&mut queue.counts, place, bytes, unit);
}

/// Charge `bytes` for `place` with no unit, under the queue's lock
/// ([`Ledger::charge`](super::Ledger::charge)): every byte of it is excess.
#[cfg(test)]
pub(super) fn charge(account: &Arc<Account>, place: Holding, bytes: u32) -> Charge {
    let mut queue = lock(&account.admission);
    Charge::loose(account, &mut queue.counts, place, bytes)
}

/// The ledger's excess, `E` ([`Counts`]).
#[cfg(test)]
pub(super) fn excess(account: &Account) -> usize {
    lock(&account.admission).counts.excess
}

/// The bytes charges built out of grants to units of `unit` kind at
/// `depth` hold, `held[k]`.
#[cfg(test)]
pub(super) fn held(account: &Account, unit: Unit, depth: usize) -> usize {
    lock(&account.admission).counts.held[unit.index(depth)]
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

/// Run a reconsideration under the lock `queue` holds, then release the lock
/// and wake the units granted.
///
/// It decides every waiter, deepest first and then in the order they asked,
/// by the module's [rule](self#the-rule) and its first-come, first-served
/// clause.
fn admit_waiting(account: &Arc<Account>, mut queue: MutexGuard<'_, Queue>) {
    let mut woken = Vec::new();
    let Queue {
        waiting,
        counts,
        share,
        ..
    } = &mut *queue;
    if !waiting.is_empty() {
        waiting.sort_by_key(|waiter| (Reverse(waiter.request.depth), waiter.order));
        let mut budget_open = true;
        // The kinds and depths at which this reconsideration has refused a
        // unit; it skips their later waiters. Without the skip, a later unit
        // that fits beside what its kind and depth holds would pass an
        // earlier waiter that does not, and a second producer could starve
        // that waiter; the liveness argument rests on the skip. A skipped
        // unit waits for the earlier one, not for room, so it leaves the
        // budget open.
        let mut refused = [false; KIND_DEPTHS];
        waiting.retain(|waiter| {
            let Request { unit, depth, .. } = waiter.request;
            let kind_and_depth = unit.index(depth);
            if refused[kind_and_depth] {
                return true;
            }
            match decide(account, counts, share, budget_open, waiter.request) {
                Decision::Grant(grant) => {
                    let mut state = lock(&waiter.state);
                    state.grant = Some(grant);
                    woken.extend(state.waker.take());
                    false
                }
                Decision::Room => {
                    budget_open = false;
                    refused[kind_and_depth] = true;
                    true
                }
                Decision::Share => {
                    refused[kind_and_depth] = true;
                    true
                }
            }
        });
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
    /// Whether the unit was exempt: admitted because its kind and depth held
    /// nothing or it fit within their bound, asking nothing of the budget.
    ///
    /// It is false in a report of a unit starting to wait and in a report
    /// of a grant's return.
    pub exempt: bool,
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
            exempt: false,
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

/// Report the unit `request` names taking its grant to the installed hook,
/// with its queue order if it was queued, exempt if `exempt`, and count it
/// among the account's invited questions if it is one.
#[cfg(any(test, feature = "test-internals"))]
fn admitted(account: &Account, request: Request, order: Option<u64>, exempt: bool, waited: bool) {
    // The share governs exactly the questions at depths 1 to 31: the opening
    // (depth 0) and leaf requests (depth 32) are admitted outside it.
    if request.unit == Unit::Question && !exempt && (1..DEPTHS - 1).contains(&request.depth) {
        account.invited.fetch_add(1, Ordering::Relaxed);
    }
    if let Some(hook) = ADMISSION_HOOK.get() {
        hook(Admission {
            ledger: std::ptr::from_ref(account) as usize,
            order,
            waiting: false,
            question: request.unit == Unit::Question,
            depth: request.depth,
            bytes: widen(request.bytes),
            exempt,
            waited,
            leftover: false,
            partial: false,
        });
    }
}

/// Report a grant of `unit` kind at `depth` returning the `bytes` its unit
/// did not use to the installed hook, before its unit is built if
/// `partial`.
#[cfg(any(test, feature = "test-internals"))]
pub(super) fn settled(account: &Account, unit: Unit, depth: usize, bytes: usize, partial: bool) {
    if let Some(hook) = ADMISSION_HOOK.get() {
        hook(Admission {
            ledger: std::ptr::from_ref(account) as usize,
            order: None,
            waiting: false,
            question: unit == Unit::Question,
            depth,
            bytes,
            exempt: false,
            waited: false,
            leftover: true,
            partial,
        });
    }
}

/// Report a unit taking its grant: nothing outside test builds.
#[cfg(not(any(test, feature = "test-internals")))]
#[inline(always)]
fn admitted(_: &Account, _: Request, _: Option<u64>, _: bool, _: bool) {}

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
    Granted(Request, Granted),
    /// Queued among the waiting units, where [`admit_waiting`] leaves the
    /// grant.
    Queued(Arc<Waiter>),
}

impl Admit {
    /// Admit a unit of `unit` kind concerning a scope at `depth`, whose
    /// items take at most `bytes`: at once if nothing waits and [`decide`]
    /// grants it, or else queue it and run a reconsideration.
    ///
    /// For the session-budget meter, a waiting unit is admission's own
    /// ([`Holding::Admission`]); a unit admitted at once allocates nothing.
    ///
    /// A question with a `need` is under the invitation share, its reply
    /// predicted to need that many bytes processed.
    ///
    /// # Panics
    ///
    /// Panics if `depth` exceeds 32, the depth of a leaf: a programmer
    /// error, since every scope is a prefix of a 32-byte path.
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
        Holding::Admission.create(|| Self::queue(account, request))
    }

    /// Admit a unit at once or queue it ([`Admit::new`]).
    fn queue(account: Arc<Account>, request: Request) -> Self {
        let waiter = {
            let mut queue = lock(&account.admission);
            // With nothing waiting, a reconsideration would decide only this
            // unit, with the budget open, so deciding it here grants exactly
            // what queueing would, without building a waiter; holding the
            // lock through the queueing makes a refused unit queue as if
            // never tried. With anything waiting, the unit queues, so it is
            // never decided past an earlier unit of its kind and depth.
            let Queue {
                waiting,
                counts,
                share,
                ..
            } = &mut *queue;
            if waiting.is_empty()
                && let Decision::Grant(grant) = decide(&account, counts, share, true, request)
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
                    state.waker = Some(Holding::Admission.create(|| cx.waker().clone()));
                    drop(state);
                    self.state = Some(Admitting::Queued(waiter));
                    return Poll::Pending;
                };
                (waiter.request, Some(waiter.order), grant)
            }
        };
        let Granted { charge, exempt } = grant;
        admitted(&self.account, request, order, exempt, self.waited);
        Poll::Ready(charge)
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
        // The withdrawn unit may have closed the budget, or stood ahead of
        // later units of its kind and depth.
        reconsider(&self.account);
    }
}

#[cfg(test)]
mod tests;
