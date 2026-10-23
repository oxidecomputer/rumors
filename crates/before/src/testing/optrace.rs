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
//! [`run`] and [`step_impl`] are the narrower adapters used by tests that need
//! recursive-oracle populations or production state after each step, and
//! [`step`] advances any model by one operation for tests that compare models
//! in lockstep.
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
//! fail, and no clock, party, or version a trace encodes may fail to
//! decode. [`run`] and [`step_impl`] assert this of every step they apply,
//! so every consumer of a trace checks the promise on the histories it
//! draws.
//!
//! # Populations and depth
//!
//! A `forks` step or array split never grows the population past
//! [`MAX_POPULATION`]: near it, `forks` takes fewer children and an array
//! split deals fewer shares, and at or past it they add no clock. A single
//! [`Op::Fork`] always adds its child, so a trace of single-party steps is
//! never clamped.
//!
//! Each step can deepen the parties it splits by a bounded number of tree
//! levels, [`Op::levels`], and a trace from [`world_strategy_up_to`]`(n)`
//! keeps the sum of its steps' levels below `n`. Joins and normalization
//! never deepen a party, and a tick deepens a version no further than its
//! clock's party, so every party and version in such a trace is fewer than
//! `n` levels deep. The function-space oracle's comparison grid
//! ([`GRID_N`](super::oracles::function::GRID_N)) derives from that bound
//! for [`world_strategy`]'s cap.

use proptest::prelude::*;
use proptest::sample::Index;
use rand::SeedableRng;
use rand_chacha::ChaChaRng;

use crate::testing::oracles::function::{Event, FunctionClock};
use crate::testing::oracles::{flat, tree};
use crate::{Clock, Party, Version};

/// One implementation of the stateful ITC operations, shared by production
/// and every model a trace can drive.
///
/// The trace driver is generic over this trait. Consequently, every operation
/// it can generate has one implementation for production, the recursive tree
/// model, the function-space model, and the flat model's parties; adding a
/// method fails to compile until all of them are supplied. The trait
/// deliberately contains only their common semantic vocabulary.
pub(crate) trait TraceModel {
    /// This model's clock value.
    type Clock;
    /// The version-like message transmitted by `send`.
    type Message;

    /// Create the initial clock.
    fn seed() -> Self::Clock;
    /// Register one event.
    fn tick(clock: &mut Self::Clock, rng: &mut ChaChaRng);
    /// Register `count` events through the model's native spelling.
    fn ticks(clock: &mut Self::Clock, count: u8, rng: &mut ChaChaRng);
    /// Split off a child clock.
    fn fork(clock: &mut Self::Clock, rng: &mut ChaChaRng) -> Self::Clock;
    /// Return the first `taken` of the `count` children that
    /// [`Clock::forks`] yields, leaving `clock` the residual; `taken` is at
    /// most `count`.
    fn forks(
        clock: &mut Self::Clock,
        count: u8,
        taken: usize,
        rng: &mut ChaChaRng,
    ) -> Vec<Self::Clock>;
    /// Split a clock into `arity` balanced shares, in the order the array
    /// conversion `<[Clock; N]>::from` deals them.
    fn split(clock: Self::Clock, arity: Arity, rng: &mut ChaChaRng) -> Vec<Self::Clock>;
    /// Advance and snapshot a message.
    fn send(clock: &mut Self::Clock, rng: &mut ChaChaRng) -> Self::Message;
    /// Merge a message and advance.
    fn receive(clock: &mut Self::Clock, message: Self::Message, rng: &mut ChaChaRng);
    /// Reconcile two clocks, reporting whether synchronization succeeded.
    fn sync(left: &mut Self::Clock, right: &mut Self::Clock, rng: &mut ChaChaRng) -> bool;
    /// Reconcile a clock with every other participant, in order, reporting
    /// whether synchronization succeeded.
    fn sync_all(
        clock: &mut Self::Clock,
        others: Vec<&mut Self::Clock>,
        rng: &mut ChaChaRng,
    ) -> bool;
    /// Absorb a clock, returning it unchanged when its party overlaps.
    fn join(
        clock: &mut Self::Clock,
        other: Self::Clock,
        rng: &mut ChaChaRng,
    ) -> Result<(), Self::Clock>;
    /// Absorb every input, returning the clocks not absorbed when any party
    /// overlaps.
    fn join_all(
        clock: &mut Self::Clock,
        others: Vec<Self::Clock>,
        rng: &mut ChaChaRng,
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

    fn seed() -> Self::Clock {
        Clock::seed()
    }

    fn tick(clock: &mut Self::Clock, _rng: &mut ChaChaRng) {
        clock.tick();
    }

    fn ticks(clock: &mut Self::Clock, count: u8, _rng: &mut ChaChaRng) {
        clock.ticks(u64::from(count));
    }

    fn fork(clock: &mut Self::Clock, _rng: &mut ChaChaRng) -> Self::Clock {
        clock.fork()
    }

    fn forks(
        clock: &mut Self::Clock,
        count: u8,
        taken: usize,
        _rng: &mut ChaChaRng,
    ) -> Vec<Self::Clock> {
        clock.forks(count).take(taken).collect()
    }

    fn split(clock: Self::Clock, arity: Arity, _rng: &mut ChaChaRng) -> Vec<Self::Clock> {
        match arity {
            Arity::One => Vec::from(<[Clock; 1]>::from(clock)),
            Arity::Two => Vec::from(<[Clock; 2]>::from(clock)),
            Arity::Three => Vec::from(<[Clock; 3]>::from(clock)),
            Arity::Five => Vec::from(<[Clock; 5]>::from(clock)),
            Arity::Eight => Vec::from(<[Clock; 8]>::from(clock)),
        }
    }

    fn send(clock: &mut Self::Clock, _rng: &mut ChaChaRng) -> Self::Message {
        clock.send().clone()
    }

    fn receive(clock: &mut Self::Clock, message: Self::Message, _rng: &mut ChaChaRng) {
        clock.recv(&message);
    }

    fn sync(left: &mut Self::Clock, right: &mut Self::Clock, _rng: &mut ChaChaRng) -> bool {
        left.sync(right).is_ok()
    }

    fn sync_all(
        clock: &mut Self::Clock,
        others: Vec<&mut Self::Clock>,
        _rng: &mut ChaChaRng,
    ) -> bool {
        clock.sync_all(others).is_ok()
    }

    fn join(
        clock: &mut Self::Clock,
        other: Self::Clock,
        _rng: &mut ChaChaRng,
    ) -> Result<(), Self::Clock> {
        clock.join(other).map(|_| ())
    }

    fn join_all(
        clock: &mut Self::Clock,
        others: Vec<Self::Clock>,
        _rng: &mut ChaChaRng,
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

    fn seed() -> Self::Clock {
        tree::Clock::seed()
    }

    fn tick(clock: &mut Self::Clock, _rng: &mut ChaChaRng) {
        clock.tick();
    }

    fn ticks(clock: &mut Self::Clock, count: u8, _rng: &mut ChaChaRng) {
        for _ in 0..count {
            clock.tick();
        }
    }

    fn fork(clock: &mut Self::Clock, _rng: &mut ChaChaRng) -> Self::Clock {
        clock.fork()
    }

    fn forks(
        clock: &mut Self::Clock,
        count: u8,
        taken: usize,
        _rng: &mut ChaChaRng,
    ) -> Vec<Self::Clock> {
        clock.forks(usize::from(count), taken)
    }

    fn split(clock: Self::Clock, arity: Arity, _rng: &mut ChaChaRng) -> Vec<Self::Clock> {
        clock.split(arity.shares())
    }

    fn send(clock: &mut Self::Clock, _rng: &mut ChaChaRng) -> Self::Message {
        clock.send()
    }

    fn receive(clock: &mut Self::Clock, message: Self::Message, _rng: &mut ChaChaRng) {
        clock.receive(message);
    }

    fn sync(left: &mut Self::Clock, right: &mut Self::Clock, _rng: &mut ChaChaRng) -> bool {
        left.sync(right).is_ok()
    }

    fn sync_all(
        clock: &mut Self::Clock,
        others: Vec<&mut Self::Clock>,
        _rng: &mut ChaChaRng,
    ) -> bool {
        clock.sync_all(others).is_ok()
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

/// The function-space implementation of [`TraceModel`].
pub(crate) struct FunctionSpace;

impl TraceModel for FunctionSpace {
    type Clock = FunctionClock;
    type Message = Event;

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
    /// Join `other` into `party` when they are disjoint, reporting whether
    /// it did.
    fn absorb(party: &mut flat::Party, other: &flat::Party) -> bool {
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

    fn seed() -> Self::Clock {
        flat::Party::seed()
    }

    fn tick(_clock: &mut Self::Clock, _rng: &mut ChaChaRng) {}

    fn ticks(_clock: &mut Self::Clock, _count: u8, _rng: &mut ChaChaRng) {}

    fn fork(clock: &mut Self::Clock, _rng: &mut ChaChaRng) -> Self::Clock {
        let (kept, given) = clock.fork();
        *clock = kept;
        given
    }

    fn forks(
        clock: &mut Self::Clock,
        count: u8,
        taken: usize,
        _rng: &mut ChaChaRng,
    ) -> Vec<Self::Clock> {
        let mut shares = clock.shares(usize::from(count) + 1);
        let untaken = shares.split_off(taken + 1);
        let children = shares.split_off(1);
        *clock = untaken
            .iter()
            .fold(shares.remove(0), |residual, share| residual.union(share));
        children
    }

    fn split(clock: Self::Clock, arity: Arity, _rng: &mut ChaChaRng) -> Vec<Self::Clock> {
        clock.shares(arity.shares())
    }

    fn send(_clock: &mut Self::Clock, _rng: &mut ChaChaRng) -> Self::Message {}

    fn receive(_clock: &mut Self::Clock, _message: Self::Message, _rng: &mut ChaChaRng) {}

    fn sync(left: &mut Self::Clock, right: &mut Self::Clock, _rng: &mut ChaChaRng) -> bool {
        let mut whole = left.clone();
        if !Self::absorb(&mut whole, right) {
            return false;
        }
        (*left, *right) = whole.fork();
        true
    }

    fn sync_all(
        clock: &mut Self::Clock,
        others: Vec<&mut Self::Clock>,
        _rng: &mut ChaChaRng,
    ) -> bool {
        let mut whole = clock.clone();
        if !others.iter().all(|other| Self::absorb(&mut whole, other)) {
            return false;
        }
        let mut shares = whole.shares(others.len() + 1).into_iter();
        for participant in core::iter::once(clock).chain(others) {
            *participant = shares.next().expect("one share per participant");
        }
        true
    }

    fn join(
        clock: &mut Self::Clock,
        other: Self::Clock,
        _rng: &mut ChaChaRng,
    ) -> Result<(), Self::Clock> {
        if Self::absorb(clock, &other) {
            Ok(())
        } else {
            Err(other)
        }
    }

    fn join_all(
        clock: &mut Self::Clock,
        others: Vec<Self::Clock>,
        _rng: &mut ChaChaRng,
    ) -> Result<(), Vec<Self::Clock>> {
        let rejected: Vec<flat::Party> = others
            .into_iter()
            .filter(|other| !Self::absorb(clock, other))
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
    /// Five shares.
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

    /// The number of shares.
    pub(crate) fn shares(self) -> usize {
        match self {
            Arity::One => 1,
            Arity::Two => 2,
            Arity::Three => 3,
            Arity::Five => 5,
            Arity::Eight => 8,
        }
    }

    /// The largest arity no larger than `self` whose split adds at most
    /// `room` clocks.
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
    /// Reconcile `i` and `j` (join then re-split).
    Sync(Index, Index),
    /// Reconcile member `i` with every other member picked, in pick order;
    /// repeated picks and picks of `i` count once.
    SyncAll(Index, Vec<Index>),
    /// Join `j` into `i`, removing `j`.
    Join(Index, Index),
    /// Join every member picked into member `i`, in pick order, removing
    /// them; repeated picks and picks of `i` count once.
    JoinAll(Index, Vec<Index>),
    /// Move member `i` through its byte encoding.
    Transmit(Index),
    /// Move member `i`'s party and version through their separate byte
    /// encodings.
    TransmitParts(Index),
}

/// The most participants besides the receiver a `sync_all` or `join_all`
/// step picks.
const MAX_PICKS: usize = 5;

/// The largest count a `forks` step draws: nine shares, the residual's and
/// eight children's.
const MAX_FORKS_COUNT: u8 = 8;

/// The population size at or past which no `forks` step or array split adds
/// clocks.
///
/// Consumers compare a population's members pairwise, and the function-space
/// replay scans every pair, so this cap bounds their per-trace work while
/// leaving room for several multi-party splits in one trace.
pub(crate) const MAX_POPULATION: usize = 24;

/// The number of balanced splits on the way to the deepest of `shares`
/// shares: `⌈log₂ shares⌉`.
pub(crate) fn split_levels(shares: usize) -> u32 {
    shares.next_power_of_two().trailing_zeros()
}

impl Op {
    /// The most tree levels this step can add below any party's deepest
    /// leaf.
    ///
    /// A balanced split into `n` shares reaches its deepest share through
    /// `⌈log₂ n⌉` of the paper's splits, and each split deepens a party by at
    /// most one level. A fork and a sync deal two shares; every other step
    /// splits nothing. The bound reads the drawn counts, so clamping a step
    /// to the population can only lower what it adds.
    pub(crate) fn levels(&self) -> u32 {
        match self {
            Op::Fork(_) | Op::Sync(..) => split_levels(2),
            Op::Forks(_, count, _) => split_levels(usize::from(*count) + 1),
            Op::Split(_, arity) => split_levels(arity.shares()),
            Op::SyncAll(_, picks) => split_levels(picks.len() + 1),
            Op::Tick(_)
            | Op::Ticks(..)
            | Op::Send(..)
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
/// position in it, picks distinct, and counts clamped to
/// [`MAX_POPULATION`].
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

/// One step, with every operation kind in the trace vocabulary reachable.
///
/// The weights keep ticks and sends the most frequent steps, so a trace
/// still carries most of the events a trace of single-party steps would,
/// and give sync and `sync_all` less weight than the other multi-party
/// steps, since each leaves every participant with one shared version and
/// so thins the concurrent pairs consumers draw.
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

/// A trace of fewer than [`MAX_TRACE_OPS`] ops, adding fewer than
/// [`MAX_TRACE_OPS`] tree levels, over a population that starts as a single
/// seed clock.
pub(crate) fn world_strategy() -> impl Strategy<Value = Vec<Op>> {
    world_strategy_up_to(MAX_TRACE_OPS)
}

/// A trace of fewer than `max_ops` ops, adding fewer than `max_ops` tree
/// levels, over a population that starts as a single seed clock, for suites
/// that need histories deeper than [`world_strategy`]'s default cap.
///
/// The strategy drops each drawn step whose [`levels`](Op::levels) would
/// bring the trace's total to `max_ops`, so every party and version the
/// trace produces is fewer than `max_ops` levels deep (see the module doc).
/// A trace of single-party steps adds at most one level per step, so none of
/// its steps is ever dropped.
pub(crate) fn world_strategy_up_to(max_ops: usize) -> impl Strategy<Value = Vec<Op>> {
    let budget = u32::try_from(max_ops).expect("a trace cap fits in u32");
    prop::collection::vec(op_strategy(), 0..max_ops).prop_map(move |drawn| {
        let mut levels = 0;
        drawn
            .into_iter()
            .filter(|op| {
                let fits = levels + op.levels() < budget;
                if fits {
                    levels += op.levels();
                }
                fits
            })
            .collect()
    })
}

/// Result of applying a trace to one [`TraceModel`].
pub(crate) struct Replay<C> {
    /// Final live clock population, in trace index order.
    pub(crate) clocks: Vec<C>,
    /// Whether each fallible step succeeded, in operation order: every join,
    /// `join_all`, sync, `sync_all`, and byte round trip.
    pub(crate) accepted: Vec<bool>,
}

/// Apply one operation through a model, appending a verdict to `accepted`
/// for each fallible step.
///
/// Population indexing, the population cap, and the restoration of rejected
/// joins live here and in [`Op::resolve`], so this one schedule definition
/// is used by every model and by tests that inspect a population after each
/// step. A failed `join_all` appends the clocks it hands back to the end of
/// the population.
pub(crate) fn apply<M: TraceModel>(
    clocks: &mut Vec<M::Clock>,
    op: &Op,
    rng: &mut ChaChaRng,
    accepted: &mut Vec<bool>,
) {
    let Some(step) = op.resolve(clocks.len()) else {
        return;
    };
    match step {
        Step::Tick(i) => M::tick(&mut clocks[i], rng),
        Step::Ticks(i, count) => M::ticks(&mut clocks[i], count, rng),
        Step::Fork(i) => {
            let child = M::fork(&mut clocks[i], rng);
            clocks.push(child);
        }
        Step::Forks { at, count, taken } => {
            let children = M::forks(&mut clocks[at], count, taken, rng);
            clocks.extend(children);
        }
        Step::Split { at, arity } => {
            let mut shares = M::split(clocks.remove(at), arity, rng).into_iter();
            clocks.insert(at, shares.next().expect("a split deals at least one share"));
            clocks.extend(shares);
        }
        Step::Send(i, j) => {
            let message = M::send(&mut clocks[i], rng);
            M::receive(&mut clocks[j], message, rng);
        }
        Step::Sync(i, j) => {
            let (lo, hi) = (i.min(j), i.max(j));
            let (left, right) = clocks.split_at_mut(hi);
            accepted.push(M::sync(&mut left[lo], &mut right[0], rng));
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
            accepted.push(M::sync_all(receiver, others, rng));
        }
        Step::Join(i, j) => {
            let other = clocks.remove(j);
            let receiver = if j < i { i - 1 } else { i };
            match M::join(&mut clocks[receiver], other, rng) {
                Ok(()) => accepted.push(true),
                Err(other) => {
                    clocks.insert(j, other);
                    accepted.push(false);
                }
            }
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
            let returned = M::join_all(&mut absorbing, inputs, rng);
            slots[receiver] = Some(absorbing);
            clocks.extend(slots.into_iter().flatten());
            match returned {
                Ok(()) => accepted.push(true),
                Err(returned) => {
                    clocks.extend(returned);
                    accepted.push(false);
                }
            }
        }
        Step::Transmit(i) => accepted.push(M::transmit(&mut clocks[i])),
        Step::TransmitParts(i) => accepted.push(M::transmit_parts(&mut clocks[i])),
    }
}

/// Apply a trace to one seed-derived universe through a model's complete
/// state-transition interface.
///
/// This is the sole spelling of the trace schedule. The model trait supplies
/// only the operations themselves, so index handling, removal and restoration
/// after a rejected join, and the definition of a skipped self-operation
/// cannot drift among the implementations. The driver creates exactly one
/// seed so independently seeded universes are unrepresentable here.
pub(crate) fn replay<M: TraceModel>(ops: &[Op], rng: &mut ChaChaRng) -> Replay<M::Clock> {
    let mut clocks = vec![M::seed()];
    let mut accepted = Vec::new();
    for op in ops {
        apply::<M>(&mut clocks, op, rng, &mut accepted);
    }
    Replay { clocks, accepted }
}

/// The assertion message for a fallible step that failed in a trace.
const FAILED_STEP: &str = "a step of a rule-respecting trace failed, which the crate page's \
    safety rules promise cannot happen";

/// Apply a trace to a fresh oracle population, asserting that no step
/// fails.
pub(crate) fn run(ops: &[Op]) -> Vec<tree::Clock> {
    let replay = replay::<Recursive>(ops, &mut ChaChaRng::seed_from_u64(0));
    assert!(
        replay.accepted.iter().all(|accepted| *accepted),
        "{FAILED_STEP}"
    );
    replay.clocks
}

/// Apply one operation to an existing population through any model.
///
/// This incremental form lets tests assert after every step while delegating to
/// the same scheduler as [`replay`]. A test can implement [`TraceModel`] for
/// production paired with its own bookkeeping and step both through this one
/// schedule, so the bookkeeping cannot drift from the clocks it describes.
///
/// # Panics
///
/// If the step rejects a join or sync, which no single-seed trace does.
pub(crate) fn step<M: TraceModel>(clocks: &mut Vec<M::Clock>, op: &Op, rng: &mut ChaChaRng) {
    let mut accepted = Vec::new();
    apply::<M>(clocks, op, rng, &mut accepted);
    assert!(accepted.iter().all(|accepted| *accepted), "{FAILED_STEP}");
}

/// Apply one operation to an existing production population.
///
/// Production draws no randomness, so each call seeds a fresh generator.
pub(crate) fn step_impl(imp: &mut Vec<Clock>, op: &Op) {
    step::<Production>(imp, op, &mut ChaChaRng::seed_from_u64(0));
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
