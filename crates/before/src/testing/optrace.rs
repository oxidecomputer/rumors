//! Stateful differential traces over clocks.
//!
//! The strategy generates sequences of clock operations. [`replay`] applies
//! one sequence to production and to each independent model through
//! [`TraceModel`], so scheduling details cannot drift between copied drivers.
//! The models are the recursive tree oracle, the function-space oracle, and
//! the flat model's parties ([`FlatParties`]), which follow each step's
//! effect on identity exactly but hold no versions. Every trace begins with
//! one seed and creates clocks only by splitting or recombining it, so
//! independently seeded universes are unrepresentable.
//!
//! [`run`] is the narrower adapter for tests that need recursive-oracle
//! populations, and [`step`] advances any model by one operation for tests
//! that assert after every step or compare models in lockstep.
//!
//! # Production-side traces
//!
//! The oracles apply `ticks(n)` as `n` literal ticks, so they cannot follow a
//! tick count wider than a few dozen. [`production_strategy_up_to`] draws
//! traces of [`ProductionOp`]s, which add such counts to the shared steps,
//! for drivers that step only production-backed models ([`ProductionModel`])
//! through [`step_production`] or [`step_impl`].
//!
//! # Rule-respecting use never fails
//!
//! Every trace obeys the crate page's two safety rules. It has one seed, and
//! it uses each clock linearly: a byte round trip replaces the clock it
//! encodes, which is the move the crate page asks of a transmission. The
//! crate page promises that if an overlap error ever arises from
//! [`join`](crate::Clock::join) or [`sync`](crate::Clock::sync), "the
//! diagnosis is always definitive: the programmer violated one of the above
//! rules", and [`join_all`](crate::Clock::join_all) and
//! [`sync_all`](crate::Clock::sync_all) state the same of their errors:
//! "[`Clock`]s used linearly from one [`Clock::seed`] remain pairwise
//! disjoint." So no join, `join_all`, sync, or `sync_all` in a trace may
//! fail. The crate page's section on replicating clocks adds that
//! "everything crosses process boundaries as canonical bytes:
//! [`encode`](crate::Clock::encode)/[`encode_to`](crate::Clock::encode_to)
//! emit them, and [`decode`](crate::Clock::decode) validates strictly", and
//! each decoder accepts exactly canonical input, so no clock, party, or
//! version a trace encodes may fail to decode. [`step`] asserts both of
//! every step it applies, and every trace path applies its steps through
//! it, so every consumer of a trace checks the promise on the histories it
//! draws.
//!
//! # Populations and depth
//!
//! A `forks` step or array split never grows the population past
//! [`MAX_POPULATION`]: near it, `forks` takes fewer children and an array
//! split deals fewer shares, and at or past it they add no clock. A single
//! [`Op::Fork`] always adds its child, so a trace with no `forks` step or
//! array split is never clamped.
//!
//! Each step can deepen the parties it splits by a bounded number of tree
//! levels, [`Op::levels`], and a trace from [`world_strategy_up_to`]`(n)`
//! keeps the sum of its steps' levels below `n`. Joins and normalization
//! never deepen a party, and a tick or an absorb deepens a version no
//! further than the parties that have ticked it, so every party and version
//! in such a trace is fewer than `n` levels deep. The function-space
//! oracle's comparison grid ([`GRID_N`](super::oracles::function::GRID_N))
//! derives from that bound for [`world_strategy`]'s cap.

use proptest::prelude::*;
use proptest::sample::Index;
use rand_chacha::ChaChaRng;

use num_bigint::BigUint;

use crate::testing::oracles::function::{self, Event, FunctionClock};
use crate::testing::oracles::{flat, tree};

use crate::{Clock, Count, Party, Version};

/// One implementation of the stateful ITC operations, shared by production
/// and every model a trace can drive.
///
/// The trace driver is generic over this trait. Consequently, every operation
/// it can generate has one implementation per model, and adding a method
/// fails to compile until every implementation supplies it. The trait
/// deliberately contains only the models' common semantic vocabulary.
pub(crate) trait TraceModel {
    /// This model's clock value.
    type Clock;
    /// The version-like message transmitted by `send`.
    type Message;
    /// The randomness this model's free choices draw on: `()` for a model
    /// whose every step is determined by the trace alone.
    type Rng;

    /// Create the initial clock.
    fn seed() -> Self::Clock;
    /// Register one event.
    fn tick(clock: &mut Self::Clock, rng: &mut Self::Rng);
    /// Register `count` events through the model's native spelling.
    fn ticks(clock: &mut Self::Clock, count: u8, rng: &mut Self::Rng);
    /// Split off a child clock.
    fn fork(clock: &mut Self::Clock, rng: &mut Self::Rng) -> Self::Clock;
    /// Return the first `taken` of the `count` children that
    /// [`Clock::forks`] yields, leaving `clock` the residual; `taken` is at
    /// most `count`.
    fn forks(
        clock: &mut Self::Clock,
        count: u8,
        taken: usize,
        rng: &mut Self::Rng,
    ) -> Vec<Self::Clock>;
    /// Split a clock into `arity` balanced shares, in the order the array
    /// conversion `<[Clock; N]>::from` deals them.
    fn split(clock: Self::Clock, arity: Arity, rng: &mut Self::Rng) -> Vec<Self::Clock>;
    /// Advance and snapshot a message.
    fn send(clock: &mut Self::Clock, rng: &mut Self::Rng) -> Self::Message;
    /// Merge a message and advance.
    fn receive(clock: &mut Self::Clock, message: Self::Message, rng: &mut Self::Rng);
    /// Snapshot a clock's message without advancing it.
    fn snapshot(clock: &Self::Clock) -> Self::Message;
    /// Merge a message without advancing.
    fn absorb(clock: &mut Self::Clock, message: Self::Message);
    /// Merge every message without advancing.
    fn absorb_all(clock: &mut Self::Clock, messages: Vec<Self::Message>);
    /// Reconcile two clocks, reporting whether synchronization succeeded.
    fn sync(left: &mut Self::Clock, right: &mut Self::Clock, rng: &mut Self::Rng) -> bool;
    /// Reconcile a clock with every other participant, in order, reporting
    /// whether synchronization succeeded.
    fn sync_all(
        clock: &mut Self::Clock,
        others: Vec<&mut Self::Clock>,
        rng: &mut Self::Rng,
    ) -> bool;
    /// Absorb a clock, returning it unchanged when its party overlaps.
    fn join(
        clock: &mut Self::Clock,
        other: Self::Clock,
        rng: &mut Self::Rng,
    ) -> Result<(), Self::Clock>;
    /// Absorb every input, returning the clocks not absorbed when any party
    /// overlaps.
    fn join_all(
        clock: &mut Self::Clock,
        others: Vec<Self::Clock>,
        rng: &mut Self::Rng,
    ) -> Result<(), Vec<Self::Clock>>;
    /// Move a clock through its byte encoding, reporting whether its bytes
    /// decoded; a model without a codec moves it unchanged.
    fn transmit(clock: &mut Self::Clock) -> bool;
    /// Move a clock's party and version through their separate byte
    /// encodings, reporting whether both decoded; a model without a codec
    /// moves it unchanged.
    fn transmit_parts(clock: &mut Self::Clock) -> bool;
}

/// Production's implementation of [`TraceModel`].
pub(crate) struct Production;

impl TraceModel for Production {
    type Clock = Clock;
    type Message = Version;
    type Rng = ();

    fn seed() -> Self::Clock {
        Clock::seed()
    }

    fn tick(clock: &mut Self::Clock, _rng: &mut ()) {
        clock.tick();
    }

    fn ticks(clock: &mut Self::Clock, count: u8, _rng: &mut ()) {
        clock.ticks(u64::from(count));
    }

    fn fork(clock: &mut Self::Clock, _rng: &mut ()) -> Self::Clock {
        clock.fork()
    }

    fn forks(clock: &mut Self::Clock, count: u8, taken: usize, _rng: &mut ()) -> Vec<Self::Clock> {
        clock.forks(count).take(taken).collect()
    }

    fn split(clock: Self::Clock, arity: Arity, _rng: &mut ()) -> Vec<Self::Clock> {
        match arity {
            Arity::One => Vec::from(<[Clock; 1]>::from(clock)),
            Arity::Two => Vec::from(<[Clock; 2]>::from(clock)),
            Arity::Three => Vec::from(<[Clock; 3]>::from(clock)),
            Arity::Five => Vec::from(<[Clock; 5]>::from(clock)),
            Arity::Eight => Vec::from(<[Clock; 8]>::from(clock)),
        }
    }

    fn send(clock: &mut Self::Clock, _rng: &mut ()) -> Self::Message {
        clock.send().clone()
    }

    fn receive(clock: &mut Self::Clock, message: Self::Message, _rng: &mut ()) {
        clock.recv(&message);
    }

    fn snapshot(clock: &Self::Clock) -> Self::Message {
        clock.version().clone()
    }

    fn absorb(clock: &mut Self::Clock, message: Self::Message) {
        clock.absorb(&message);
    }

    fn absorb_all(clock: &mut Self::Clock, messages: Vec<Self::Message>) {
        clock.absorb_all(messages);
    }

    fn sync(left: &mut Self::Clock, right: &mut Self::Clock, _rng: &mut ()) -> bool {
        left.sync(right).is_ok()
    }

    fn sync_all(clock: &mut Self::Clock, others: Vec<&mut Self::Clock>, _rng: &mut ()) -> bool {
        clock.sync_all(others).is_ok()
    }

    fn join(clock: &mut Self::Clock, other: Self::Clock, _rng: &mut ()) -> Result<(), Self::Clock> {
        clock.join(other).map(|_| ())
    }

    fn join_all(
        clock: &mut Self::Clock,
        others: Vec<Self::Clock>,
        _rng: &mut (),
    ) -> Result<(), Vec<Self::Clock>> {
        clock.join_all(others).map(|_| ())
    }

    fn transmit(clock: &mut Self::Clock) -> bool {
        match Clock::decode(&clock.encode()[..]) {
            Ok(moved) => {
                *clock = moved;
                true
            }
            Err(_) => false,
        }
    }

    fn transmit_parts(clock: &mut Self::Clock) -> bool {
        let party = Party::decode(&clock.party().encode()[..]);
        let version = Version::decode(&clock.version().encode()[..]);
        match (party, version) {
            (Ok(party), Ok(version)) => {
                *clock = Clock::from_parts(party, version);
                true
            }
            _ => false,
        }
    }
}

/// The recursive tree implementation of [`TraceModel`].
pub(crate) struct Recursive;

impl TraceModel for Recursive {
    type Clock = tree::Clock;
    type Message = tree::Version;
    type Rng = ();

    fn seed() -> Self::Clock {
        tree::Clock::seed()
    }

    fn tick(clock: &mut Self::Clock, _rng: &mut ()) {
        clock.tick();
    }

    fn ticks(clock: &mut Self::Clock, count: u8, _rng: &mut ()) {
        for _ in 0..count {
            clock.tick();
        }
    }

    fn fork(clock: &mut Self::Clock, _rng: &mut ()) -> Self::Clock {
        clock.fork()
    }

    fn forks(clock: &mut Self::Clock, count: u8, taken: usize, _rng: &mut ()) -> Vec<Self::Clock> {
        clock.forks(usize::from(count), taken)
    }

    fn split(clock: Self::Clock, arity: Arity, _rng: &mut ()) -> Vec<Self::Clock> {
        clock.split(arity.shares())
    }

    fn send(clock: &mut Self::Clock, _rng: &mut ()) -> Self::Message {
        clock.send()
    }

    fn receive(clock: &mut Self::Clock, message: Self::Message, _rng: &mut ()) {
        clock.receive(message);
    }

    fn snapshot(clock: &Self::Clock) -> Self::Message {
        clock.version()
    }

    fn absorb(clock: &mut Self::Clock, message: Self::Message) {
        *clock |= message;
    }

    fn absorb_all(clock: &mut Self::Clock, messages: Vec<Self::Message>) {
        for message in messages {
            *clock |= message;
        }
    }

    fn sync(left: &mut Self::Clock, right: &mut Self::Clock, _rng: &mut ()) -> bool {
        left.sync(right).is_ok()
    }

    fn sync_all(clock: &mut Self::Clock, others: Vec<&mut Self::Clock>, _rng: &mut ()) -> bool {
        clock.sync_all(others).is_ok()
    }

    fn join(clock: &mut Self::Clock, other: Self::Clock, _rng: &mut ()) -> Result<(), Self::Clock> {
        clock.join(other)
    }

    fn join_all(
        clock: &mut Self::Clock,
        others: Vec<Self::Clock>,
        _rng: &mut (),
    ) -> Result<(), Vec<Self::Clock>> {
        clock.join_all(others)
    }

    fn transmit(_clock: &mut Self::Clock) -> bool {
        true
    }

    fn transmit_parts(_clock: &mut Self::Clock) -> bool {
        true
    }
}

/// The function-space implementation of [`TraceModel`].
pub(crate) struct FunctionSpace;

impl TraceModel for FunctionSpace {
    type Clock = FunctionClock;
    type Message = Event;
    type Rng = ChaChaRng;

    fn seed() -> Self::Clock {
        FunctionClock::seed()
    }

    fn tick(clock: &mut Self::Clock, rng: &mut ChaChaRng) {
        clock.tick(rng);
    }

    fn ticks(clock: &mut Self::Clock, count: u8, rng: &mut ChaChaRng) {
        for _ in 0..count {
            clock.tick(rng);
        }
    }

    fn fork(clock: &mut Self::Clock, rng: &mut ChaChaRng) -> Self::Clock {
        clock.fork(rng)
    }

    fn forks(
        clock: &mut Self::Clock,
        count: u8,
        taken: usize,
        rng: &mut ChaChaRng,
    ) -> Vec<Self::Clock> {
        clock.forks(usize::from(count), taken, rng)
    }

    fn split(clock: Self::Clock, arity: Arity, rng: &mut ChaChaRng) -> Vec<Self::Clock> {
        clock.split(arity.shares(), rng)
    }

    fn send(clock: &mut Self::Clock, rng: &mut ChaChaRng) -> Self::Message {
        clock.send(rng)
    }

    fn receive(clock: &mut Self::Clock, message: Self::Message, rng: &mut ChaChaRng) {
        clock.receive(message, rng);
    }

    fn snapshot(clock: &Self::Clock) -> Self::Message {
        clock.ev.clone()
    }

    fn absorb(clock: &mut Self::Clock, message: Self::Message) {
        clock.ev = function::join(clock.ev.clone(), message);
    }

    fn absorb_all(clock: &mut Self::Clock, messages: Vec<Self::Message>) {
        clock.ev = function::join_all(core::iter::once(clock.ev.clone()).chain(messages));
    }

    fn sync(left: &mut Self::Clock, right: &mut Self::Clock, rng: &mut ChaChaRng) -> bool {
        left.sync(right, rng).is_ok()
    }

    fn sync_all(
        clock: &mut Self::Clock,
        others: Vec<&mut Self::Clock>,
        rng: &mut ChaChaRng,
    ) -> bool {
        clock.sync_all(others, rng).is_ok()
    }

    fn join(
        clock: &mut Self::Clock,
        other: Self::Clock,
        _rng: &mut ChaChaRng,
    ) -> Result<(), Self::Clock> {
        clock.join(other)
    }

    fn join_all(
        clock: &mut Self::Clock,
        others: Vec<Self::Clock>,
        _rng: &mut ChaChaRng,
    ) -> Result<(), Vec<Self::Clock>> {
        clock.join_all(others)
    }

    fn transmit(_clock: &mut Self::Clock) -> bool {
        true
    }

    fn transmit_parts(_clock: &mut Self::Clock) -> bool {
        true
    }
}

/// The flat model's implementation of [`TraceModel`], over parties alone.
///
/// No step's effect on a party depends on any version, and ticks and
/// messages change no party, so following only the parties computes the
/// party of every clock a trace produces exactly. The flat model has no
/// ticks, so versions are not followed, and its message type is `()`. Its
/// split is computed on interval sets, without trees, so it judges
/// production's identity operations independently of the tree recursion.
pub(crate) struct FlatParties;

impl FlatParties {
    /// Joins `other` into `party` when they are disjoint, reporting whether
    /// it did.
    fn join_disjoint(party: &mut flat::Party, other: &flat::Party) -> bool {
        let disjoint = party.is_disjoint(other);
        if disjoint {
            *party = party.union(other);
        }
        disjoint
    }
}

impl TraceModel for FlatParties {
    type Clock = flat::Party;
    type Message = ();
    type Rng = ();

    fn seed() -> Self::Clock {
        flat::Party::seed()
    }

    fn tick(_clock: &mut Self::Clock, _rng: &mut ()) {}

    fn ticks(_clock: &mut Self::Clock, _count: u8, _rng: &mut ()) {}

    fn fork(clock: &mut Self::Clock, _rng: &mut ()) -> Self::Clock {
        let (kept, given) = clock.fork();
        *clock = kept;
        given
    }

    fn forks(clock: &mut Self::Clock, count: u8, taken: usize, _rng: &mut ()) -> Vec<Self::Clock> {
        let mut shares = clock.shares(usize::from(count) + 1);
        let untaken = shares.split_off(taken + 1);
        let children = shares.split_off(1);
        *clock = untaken
            .iter()
            .fold(shares.remove(0), |residual, share| residual.union(share));
        children
    }

    fn split(clock: Self::Clock, arity: Arity, _rng: &mut ()) -> Vec<Self::Clock> {
        clock.shares(arity.shares())
    }

    fn send(_clock: &mut Self::Clock, _rng: &mut ()) -> Self::Message {}

    fn receive(_clock: &mut Self::Clock, _message: Self::Message, _rng: &mut ()) {}

    fn snapshot(_clock: &Self::Clock) -> Self::Message {}

    fn absorb(_clock: &mut Self::Clock, _message: Self::Message) {}

    fn absorb_all(_clock: &mut Self::Clock, _messages: Vec<Self::Message>) {}

    fn sync(left: &mut Self::Clock, right: &mut Self::Clock, _rng: &mut ()) -> bool {
        let mut whole = left.clone();
        if !Self::join_disjoint(&mut whole, right) {
            return false;
        }
        (*left, *right) = whole.fork();
        true
    }

    fn sync_all(clock: &mut Self::Clock, others: Vec<&mut Self::Clock>, _rng: &mut ()) -> bool {
        let mut whole = clock.clone();
        if !others
            .iter()
            .all(|other| Self::join_disjoint(&mut whole, other))
        {
            return false;
        }
        let mut shares = whole.shares(others.len() + 1).into_iter();
        for participant in core::iter::once(clock).chain(others) {
            *participant = shares.next().expect("one share per participant");
        }
        true
    }

    fn join(clock: &mut Self::Clock, other: Self::Clock, _rng: &mut ()) -> Result<(), Self::Clock> {
        if Self::join_disjoint(clock, &other) {
            Ok(())
        } else {
            Err(other)
        }
    }

    fn join_all(
        clock: &mut Self::Clock,
        others: Vec<Self::Clock>,
        _rng: &mut (),
    ) -> Result<(), Vec<Self::Clock>> {
        let rejected: Vec<flat::Party> = others
            .into_iter()
            .filter(|other| !Self::join_disjoint(clock, other))
            .collect();
        if rejected.is_empty() {
            Ok(())
        } else {
            Err(rejected)
        }
    }

    fn transmit(_clock: &mut Self::Clock) -> bool {
        true
    }

    fn transmit_parts(_clock: &mut Self::Clock) -> bool {
        true
    }
}

/// The share counts an array split may draw, as the arity of the array
/// conversion `<[Clock; N]>::from`.
///
/// A trace draws two, three, five, or eight shares; one share, the identity
/// split, stands in when the population has no room for more.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Arity {
    /// One share: the identity split.
    One,
    /// Two shares, the split a fork deals.
    Two,
    /// Three shares, the smallest count that is not a power of two.
    Three,
    /// Five shares, whose first half of three is itself uneven, so the
    /// balanced descent splits unevenly at two levels.
    Five,
    /// Eight shares, a full balanced tree three levels deep.
    Eight,
}

impl Arity {
    /// Every arity, fewest shares first.
    const ALL: [Arity; 5] = [
        Arity::One,
        Arity::Two,
        Arity::Three,
        Arity::Five,
        Arity::Eight,
    ];

    /// Returns the number of shares.
    pub(crate) fn shares(self) -> usize {
        match self {
            Arity::One => 1,
            Arity::Two => 2,
            Arity::Three => 3,
            Arity::Five => 5,
            Arity::Eight => 8,
        }
    }

    /// Returns the largest arity no larger than `self` whose split adds at
    /// most `room` clocks.
    fn within(self, room: usize) -> Arity {
        Arity::ALL
            .into_iter()
            .filter(|arity| *arity <= self && arity.shares() - 1 <= room)
            .max()
            .expect("the identity split adds no clock")
    }
}

/// One step of a seed-derived execution.
///
/// [`Index`] maps each operand across the entire live population at the moment
/// the step runs. Thus clocks appended by earlier forks remain eligible for
/// later operations. Every member descends from one seed through these
/// operations, keeping all live parties pairwise disjoint.
#[derive(Clone, Debug)]
pub(crate) enum Op {
    /// Advance member `i`.
    Tick(Index),
    /// Advance member `i` by `n` events in one fused call (small `n`:
    /// the oracle applier iterates it literally).
    Ticks(Index, u8),
    /// Split member `i`, appending the child.
    Fork(Index),
    /// Collect `forks(k).take(t)` from member `i`, appending the children;
    /// the population cap may take fewer.
    Forks(Index, u8, u8),
    /// Split member `i` into balanced shares by the array conversion: the
    /// first share replaces it and the rest are appended; the population cap
    /// may deal fewer.
    Split(Index, Arity),
    /// `i` sends (ticks, emits its version); `j` receives it.
    Send(Index, Index),
    /// Merge member `j`'s version into member `i`'s without ticking either;
    /// `j` may be `i`.
    Absorb(Index, Index),
    /// Merge every picked member's version into member `i`'s without
    /// ticking, repeated picks and picks of `i` included.
    AbsorbAll(Index, Vec<Index>),
    /// Reconcile `i` and `j` (join then re-split).
    Sync(Index, Index),
    /// Reconcile member `i` with every other member picked, in pick order;
    /// a repeated pick counts once, and a pick of `i` is dropped.
    SyncAll(Index, Vec<Index>),
    /// Join `j` into `i`, removing `j`.
    Join(Index, Index),
    /// Join every member picked into member `i`, in pick order, removing
    /// them; a repeated pick counts once, and a pick of `i` is dropped.
    JoinAll(Index, Vec<Index>),
    /// Move member `i` through its byte encoding.
    Transmit(Index),
    /// Move member `i`'s party and version through their separate byte
    /// encodings.
    TransmitParts(Index),
}

/// The most members a `sync_all`, `join_all`, or `absorb_all` step picks.
///
/// `sync_all` and `join_all` drop a pick of the receiver, so at most this
/// many others take part besides it; `absorb_all` keeps such a pick.
const MAX_PICKS: usize = 5;

/// The largest count a `forks` step draws: nine shares, the residual's and
/// eight children's, the most any step deals.
pub(crate) const MAX_FORKS_COUNT: u8 = 8;

/// The population size at or past which no `forks` step or array split adds
/// clocks.
///
/// Consumers compare a population's members pairwise, and the function-space
/// replay scans every pair, so this cap bounds their per-trace work while
/// leaving room for several `forks` steps and array splits in one trace.
pub(crate) const MAX_POPULATION: usize = 24;

/// Counts the balanced splits on the way to the deepest of `shares` shares:
/// `⌈log₂ shares⌉`.
pub(crate) fn split_levels(shares: usize) -> u32 {
    shares.next_power_of_two().trailing_zeros()
}

impl Op {
    /// Returns the most tree levels this step can add below any party's
    /// deepest leaf.
    ///
    /// A balanced split into `n` shares reaches its deepest share through
    /// `⌈log₂ n⌉` of the paper's splits, and each split deepens a party by at
    /// most one level. A fork and a sync deal two shares, a `forks` step one
    /// more than its count, an array split its arity, and a `sync_all` one
    /// more than its picks; every other step deals no shares and adds no
    /// level. The bound reads the drawn counts, so clamping a step to the
    /// population can only lower what it adds.
    pub(crate) fn levels(&self) -> u32 {
        match self {
            Op::Fork(_) | Op::Sync(..) => split_levels(2),
            Op::Forks(_, count, _) => split_levels(usize::from(*count) + 1),
            Op::Split(_, arity) => split_levels(arity.shares()),
            Op::SyncAll(_, picks) => split_levels(picks.len() + 1),
            Op::Tick(_)
            | Op::Ticks(..)
            | Op::Send(..)
            | Op::Absorb(..)
            | Op::AbsorbAll(..)
            | Op::Join(..)
            | Op::JoinAll(..)
            | Op::Transmit(_)
            | Op::TransmitParts(_) => 0,
        }
    }

    /// Resolves the step against a population of `population` clocks, or
    /// returns `None` when it is a self-sync or self-join, which the schedule
    /// skips.
    fn resolve(&self, population: usize) -> Option<Step> {
        let at = |i: &Index| i.index(population);
        let room = MAX_POPULATION.saturating_sub(population);
        let distinct = |i: &Index, j: &Index| {
            let (i, j) = (at(i), at(j));
            (i != j).then_some((i, j))
        };
        let others = |receiver: usize, picks: &[Index]| {
            let mut others: Vec<usize> = Vec::with_capacity(picks.len());
            for pick in picks.iter().map(at) {
                if pick != receiver && !others.contains(&pick) {
                    others.push(pick);
                }
            }
            others
        };
        Some(match self {
            Op::Tick(i) => Step::Tick(at(i)),
            Op::Ticks(i, count) => Step::Ticks(at(i), *count),
            Op::Fork(i) => Step::Fork(at(i)),
            Op::Forks(i, count, take) => Step::Forks {
                at: at(i),
                count: *count,
                taken: usize::from(*take.min(count)).min(room),
            },
            Op::Split(i, arity) => Step::Split {
                at: at(i),
                arity: arity.within(room),
            },
            Op::Send(i, j) => Step::Send(at(i), at(j)),
            Op::Absorb(i, j) => Step::Absorb(at(i), at(j)),
            Op::AbsorbAll(i, picks) => Step::AbsorbAll {
                receiver: at(i),
                sources: picks.iter().map(at).collect(),
            },
            Op::Sync(i, j) => {
                let (i, j) = distinct(i, j)?;
                Step::Sync(i, j)
            }
            Op::SyncAll(i, picks) => {
                let receiver = at(i);
                Step::SyncAll {
                    receiver,
                    others: others(receiver, picks),
                }
            }
            Op::Join(i, j) => {
                let (i, j) = distinct(i, j)?;
                Step::Join(i, j)
            }
            Op::JoinAll(i, picks) => {
                let receiver = at(i);
                Step::JoinAll {
                    receiver,
                    others: others(receiver, picks),
                }
            }
            Op::Transmit(i) => Step::Transmit(at(i)),
            Op::TransmitParts(i) => Step::TransmitParts(at(i)),
        })
    }
}

/// An [`Op`] resolved against the population it runs on: every operand a
/// position in it, the picks of a split or join distinct, and counts clamped
/// to [`MAX_POPULATION`].
///
/// Each variant resolves the [`Op`] of the same name.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Step {
    /// Advance the member at this position.
    Tick(usize),
    /// Advance the member by this many events.
    Ticks(usize, u8),
    /// Fork the member, appending the child.
    Fork(usize),
    /// Collect `taken` of the `count` children `forks(count)` yields;
    /// `taken` is at most `count` and fits under the cap.
    Forks { at: usize, count: u8, taken: usize },
    /// Split the member into `arity` shares, an arity that fits under the
    /// cap.
    Split { at: usize, arity: Arity },
    /// The first member sends; the second receives.
    Send(usize, usize),
    /// The first member absorbs the second's version.
    Absorb(usize, usize),
    /// The receiver absorbs every source's version, as picked.
    AbsorbAll {
        receiver: usize,
        sources: Vec<usize>,
    },
    /// Reconcile two distinct members.
    Sync(usize, usize),
    /// Reconcile the receiver with distinct others, in pick order.
    SyncAll { receiver: usize, others: Vec<usize> },
    /// Join the second member into the first, distinct from it.
    Join(usize, usize),
    /// Join distinct others into the receiver, in pick order.
    JoinAll { receiver: usize, others: Vec<usize> },
    /// Move the member through its byte encoding.
    Transmit(usize),
    /// Move the member's party and version through their byte encodings.
    TransmitParts(usize),
}

/// Draws one step, with every kind of [`Op`] reachable.
///
/// Ticks and sends weigh the most, so a trace carries most of its events in
/// them with the splits and joins mixed in. Each step that leaves its
/// participants holding one shared or dominating version weighs less than
/// its neighbors: sync less than fork and join, and `sync_all` and the
/// absorbs less than `forks`, array splits, and `join_all`, since each thins
/// the concurrent pairs consumers draw. The byte round trips, which change
/// no value, share the lowest weight with them.
fn op_strategy() -> impl Strategy<Value = Op> {
    let picks = || prop::collection::vec(any::<Index>(), 0..=MAX_PICKS);
    let arity = prop_oneof![
        Just(Arity::Two),
        Just(Arity::Three),
        Just(Arity::Five),
        Just(Arity::Eight)
    ];
    prop_oneof![
        4 => any::<Index>().prop_map(Op::Tick),
        4 => (any::<Index>(), 0u8..=6).prop_map(|(i, n)| Op::Ticks(i, n)),
        3 => any::<Index>().prop_map(Op::Fork),
        2 => (any::<Index>(), 0..=MAX_FORKS_COUNT, 0..=MAX_FORKS_COUNT)
            .prop_map(|(i, k, t)| Op::Forks(i, k, t)),
        2 => (any::<Index>(), arity).prop_map(|(i, a)| Op::Split(i, a)),
        5 => (any::<Index>(), any::<Index>()).prop_map(|(a, b)| Op::Send(a, b)),
        1 => (any::<Index>(), any::<Index>()).prop_map(|(a, b)| Op::Absorb(a, b)),
        1 => (any::<Index>(), picks()).prop_map(|(i, p)| Op::AbsorbAll(i, p)),
        2 => (any::<Index>(), any::<Index>()).prop_map(|(a, b)| Op::Sync(a, b)),
        1 => (any::<Index>(), picks()).prop_map(|(i, p)| Op::SyncAll(i, p)),
        3 => (any::<Index>(), any::<Index>()).prop_map(|(a, b)| Op::Join(a, b)),
        2 => (any::<Index>(), picks()).prop_map(|(i, p)| Op::JoinAll(i, p)),
        1 => any::<Index>().prop_map(Op::Transmit),
        1 => any::<Index>().prop_map(Op::TransmitParts),
    ]
}

/// The default trace-length cap: [`world_strategy`] draws strictly fewer ops
/// than this, adding strictly fewer tree levels.
///
/// Consumers that size a resource to the deepest reachable history derive
/// from this constant rather than transcribing it (the semantic oracle
/// derives its comparison-grid ceiling
/// [`super::oracles::function::GRID_N`] from it, so a cap change cannot
/// silently outgrow the grid).
pub(crate) const MAX_TRACE_OPS: usize = 30;

/// Keeps each drawn step whose levels fit in what remains of `budget`,
/// dropping the rest.
fn within_levels<T>(drawn: Vec<T>, budget: u32, levels: impl Fn(&T) -> u32) -> Vec<T> {
    let mut spent = 0;
    drawn
        .into_iter()
        .filter(|step| {
            let fits = spent + levels(step) < budget;
            if fits {
                spent += levels(step);
            }
            fits
        })
        .collect()
}

/// Draws a trace of fewer than [`MAX_TRACE_OPS`] ops, adding fewer than
/// [`MAX_TRACE_OPS`] tree levels, over a population that starts as a single
/// seed clock.
///
/// Every kind of [`Op`] is reachable, and its traces regularly hold a
/// `forks` step that takes children and one that leaves some untaken, an
/// array split, a `join_all` of two or more clocks, a `sync_all` with two or
/// more others, each byte round trip, absorbs from other members, a
/// population past the census's large-population size, and a concurrent
/// pair in the final population; the census
/// `trace_strategies_keep_mass_on_their_regimes` floors each of these.
pub(crate) fn world_strategy() -> impl Strategy<Value = Vec<Op>> {
    world_strategy_up_to(MAX_TRACE_OPS)
}

/// Draws a trace of fewer than `max_ops` ops, adding fewer than `max_ops`
/// tree levels, over a population that starts as a single seed clock, for
/// suites that need histories deeper than [`world_strategy`]'s default cap.
///
/// The strategy drops each drawn step whose [`levels`](Op::levels) would
/// bring the trace's total to `max_ops`, so every party and version the
/// trace produces is fewer than `max_ops` levels deep (see the module doc).
/// Only `forks` steps, array splits, and `sync_all` steps declare more than
/// one level, so a trace without them never loses a step.
pub(crate) fn world_strategy_up_to(max_ops: usize) -> impl Strategy<Value = Vec<Op>> {
    let budget = u32::try_from(max_ops).expect("a trace cap fits in u32");
    prop::collection::vec(op_strategy(), 0..max_ops)
        .prop_map(move |drawn| within_levels(drawn, budget, Op::levels))
}

/// The assertion message for a fallible step that failed in a trace.
const FAILED_STEP: &str = "a step of a rule-respecting trace failed, which the crate page \
    promises cannot happen";

/// Applies one operation to an existing population through any model.
///
/// Population indexing and the population cap live here and in
/// [`Op::resolve`], so this one schedule definition serves every model:
/// [`replay`] and [`run`] apply whole traces through it, and tests that step
/// several models in lockstep, or assert after every step, call it
/// directly. A test can implement [`TraceModel`] for production paired with
/// its own bookkeeping and step both through this one schedule, so the
/// bookkeeping cannot drift from the clocks it describes; implementing
/// [`ProductionModel`] as well lets [`step_production`] carry the pair
/// through wide tick counts.
///
/// # Panics
///
/// Panics if a join, `join_all`, sync, `sync_all`, or byte round trip
/// fails, which the crate page promises no step of a rule-respecting trace
/// does (see the module doc). The message names the model and the step.
pub(crate) fn step<M: TraceModel>(clocks: &mut Vec<M::Clock>, op: &Op, rng: &mut M::Rng) {
    let Some(resolved) = op.resolve(clocks.len()) else {
        return;
    };
    let succeeded = match resolved {
        Step::Tick(i) => {
            M::tick(&mut clocks[i], rng);
            true
        }
        Step::Ticks(i, count) => {
            M::ticks(&mut clocks[i], count, rng);
            true
        }
        Step::Fork(i) => {
            let child = M::fork(&mut clocks[i], rng);
            clocks.push(child);
            true
        }
        Step::Forks { at, count, taken } => {
            let children = M::forks(&mut clocks[at], count, taken, rng);
            clocks.extend(children);
            true
        }
        Step::Split { at, arity } => {
            let mut shares = M::split(clocks.remove(at), arity, rng).into_iter();
            clocks.insert(at, shares.next().expect("a split deals at least one share"));
            clocks.extend(shares);
            true
        }
        Step::Send(i, j) => {
            let message = M::send(&mut clocks[i], rng);
            M::receive(&mut clocks[j], message, rng);
            true
        }
        Step::Absorb(i, j) => {
            let message = M::snapshot(&clocks[j]);
            M::absorb(&mut clocks[i], message);
            true
        }
        Step::AbsorbAll { receiver, sources } => {
            let messages = sources.iter().map(|&k| M::snapshot(&clocks[k])).collect();
            M::absorb_all(&mut clocks[receiver], messages);
            true
        }
        Step::Sync(i, j) => {
            let (lo, hi) = (i.min(j), i.max(j));
            let (left, right) = clocks.split_at_mut(hi);
            M::sync(&mut left[lo], &mut right[0], rng)
        }
        Step::SyncAll { receiver, others } => {
            let mut slots: Vec<Option<&mut M::Clock>> = clocks.iter_mut().map(Some).collect();
            let mut take = |k: usize| {
                slots[k]
                    .take()
                    .expect("participants are distinct from each other and from the receiver")
            };
            let receiver = take(receiver);
            let others = others.into_iter().map(&mut take).collect();
            M::sync_all(receiver, others, rng)
        }
        Step::Join(i, j) => {
            let other = clocks.remove(j);
            let receiver = if j < i { i - 1 } else { i };
            M::join(&mut clocks[receiver], other, rng).is_ok()
        }
        Step::JoinAll { receiver, others } => {
            let mut slots: Vec<Option<M::Clock>> = clocks.drain(..).map(Some).collect();
            let mut take = |k: usize| {
                slots[k]
                    .take()
                    .expect("participants are distinct from each other and from the receiver")
            };
            let mut absorbing = take(receiver);
            let inputs = others.into_iter().map(&mut take).collect();
            let joined = M::join_all(&mut absorbing, inputs, rng).is_ok();
            slots[receiver] = Some(absorbing);
            clocks.extend(slots.into_iter().flatten());
            joined
        }
        Step::Transmit(i) => M::transmit(&mut clocks[i]),
        Step::TransmitParts(i) => M::transmit_parts(&mut clocks[i]),
    };
    assert!(
        succeeded,
        "{FAILED_STEP}: {} failed {op:?}",
        core::any::type_name::<M>()
    );
}

/// Applies a trace to one seed-derived universe through a model's complete
/// state-transition interface, returning the final live population in trace
/// index order.
///
/// Every step goes through [`step`], the sole spelling of the trace
/// schedule, so index handling and the definition of a skipped
/// self-operation cannot drift among the implementations. The driver
/// creates exactly one seed so independently seeded universes are
/// unrepresentable here.
///
/// # Panics
///
/// Panics if any step fails, as [`step`] does.
pub(crate) fn replay<M: TraceModel>(ops: &[Op], rng: &mut M::Rng) -> Vec<M::Clock> {
    let mut clocks = vec![M::seed()];
    for op in ops {
        step::<M>(&mut clocks, op, rng);
    }
    clocks
}

/// Applies a trace to a fresh recursive-oracle population.
///
/// # Panics
///
/// Panics if any step fails, as [`step`] does.
pub(crate) fn run(ops: &[Op]) -> Vec<tree::Clock> {
    replay::<Recursive>(ops, &mut ())
}

/// A [`TraceModel`] backed by production clocks, which can also apply tick
/// counts too wide for the oracles to iterate.
pub(crate) trait ProductionModel: TraceModel {
    /// Registers `count` events through production's `ticks`, whatever the
    /// width of `count`.
    fn wide_ticks(clock: &mut Self::Clock, count: &Count);
}

impl ProductionModel for Production {
    fn wide_ticks(clock: &mut Self::Clock, count: &Count) {
        clock.ticks(count.clone());
    }
}

/// The largest exponent `s` of a wide count `2^s + l`: past both the `u64`
/// and the `u128` width.
const MAX_WIDE_EXPONENT: u32 = 130;

/// The largest offset `l` of a wide count `2^s + l`.
const MAX_WIDE_OFFSET: u8 = 3;

/// One step of a production-side trace: any [`Op`], or a tick count too wide
/// for the oracles.
///
/// The recursive and function-space oracles apply `ticks(n)` as `n` literal
/// ticks, so a count past a few dozen is out of their reach, while
/// production's `ticks` costs time logarithmic in the count. Only
/// production-side drivers draw this type, through
/// [`production_strategy_up_to`], and only a [`ProductionModel`] applies it,
/// through [`step_production`].
#[derive(Clone, Debug)]
pub(crate) enum ProductionOp {
    /// A step every model can apply.
    Shared(Op),
    /// Advance member `i` by `count` events, a count `2^s + l` with `s` up to
    /// [`MAX_WIDE_EXPONENT`] and `l` up to [`MAX_WIDE_OFFSET`].
    WideTicks(Index, Count),
}

impl ProductionOp {
    /// Returns the most tree levels this step can add: a shared step's
    /// [`levels`](Op::levels), and none for a tick count, which deepens a
    /// version no further than its clock's party.
    fn levels(&self) -> u32 {
        match self {
            ProductionOp::Shared(op) => op.levels(),
            ProductionOp::WideTicks(..) => 0,
        }
    }
}

/// Draws a production-side trace of fewer than [`MAX_TRACE_OPS`] ops, as
/// [`production_strategy_up_to`] does.
pub(crate) fn production_strategy() -> impl Strategy<Value = Vec<ProductionOp>> {
    production_strategy_up_to(MAX_TRACE_OPS)
}

/// Draws a production-side trace: the steps [`world_strategy_up_to`] draws,
/// with wide tick counts mixed in, under the same op cap and level budget.
///
/// Its traces regularly hold a tick count wider than `u64`; the census
/// `trace_strategies_keep_mass_on_their_regimes` floors that regime.
pub(crate) fn production_strategy_up_to(
    max_ops: usize,
) -> impl Strategy<Value = Vec<ProductionOp>> {
    let budget = u32::try_from(max_ops).expect("a trace cap fits in u32");
    let wide = (any::<Index>(), 0..=MAX_WIDE_EXPONENT, 0..=MAX_WIDE_OFFSET).prop_map(
        |(i, exponent, offset)| {
            let count = (BigUint::from(1u8) << exponent) + offset;
            ProductionOp::WideTicks(i, Count(count))
        },
    );
    let step = prop_oneof![
        14 => op_strategy().prop_map(ProductionOp::Shared),
        1 => wide,
    ];
    prop::collection::vec(step, 0..max_ops)
        .prop_map(move |drawn| within_levels(drawn, budget, ProductionOp::levels))
}

/// Applies one production-side step to an existing population through a
/// production-backed model.
///
/// # Panics
///
/// Panics if a shared step fails, as [`step`] does.
pub(crate) fn step_production<M: ProductionModel>(
    clocks: &mut Vec<M::Clock>,
    op: &ProductionOp,
    rng: &mut M::Rng,
) {
    match op {
        ProductionOp::Shared(op) => step::<M>(clocks, op, rng),
        ProductionOp::WideTicks(i, count) => {
            let at = i.index(clocks.len());
            M::wide_ticks(&mut clocks[at], count);
        }
    }
}

/// Applies one production-side step to an existing production population.
///
/// # Panics
///
/// Panics if a shared step fails, as [`step`] does.
pub(crate) fn step_impl(imp: &mut Vec<Clock>, op: &ProductionOp) {
    step_production::<Production>(imp, op, &mut ());
}

/// Every live clock's current version.
pub(crate) fn versions(cs: &[tree::Clock]) -> Vec<tree::Version> {
    cs.iter().map(|c| c.version()).collect()
}

/// `a <= b` under the oracle causal order (treating concurrency as not-`<=`).
pub(crate) fn leq(a: &tree::Version, b: &tree::Version) -> bool {
    a <= b
}

#[cfg(test)]
mod tests;
