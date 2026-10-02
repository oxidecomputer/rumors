//! Stateful differential traces over clocks.
//!
//! The strategy generates sequences of clock operations. [`replay`] applies
//! the same sequence to production and both independent models through
//! [`TraceModel`], so scheduling details cannot drift between three copied
//! drivers. Every trace begins with one seed and creates clocks only by
//! splitting or recombining it; independently seeded universes are therefore
//! unrepresentable.
//!
//! [`run`] and [`step_impl`] are the narrower adapters used by tests that need
//! recursive-oracle populations or production state after each step.

use proptest::prelude::*;
use proptest::sample::Index;
use rand::SeedableRng;
use rand_chacha::ChaChaRng;

use crate::testing::oracles::function::{Event, FunctionClock};
use crate::testing::oracles::tree;
use crate::{Clock, Version};

/// One implementation of the stateful ITC operations shared by all three
/// semantic models.
///
/// The trace driver is generic over this trait. Consequently, every operation
/// it can generate has one implementation for production, the recursive tree
/// model, and the function-space model; adding a method fails to compile until
/// all three are supplied. The trait deliberately contains only their common
/// semantic vocabulary.
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
    /// Advance and snapshot a message.
    fn send(clock: &mut Self::Clock, rng: &mut ChaChaRng) -> Self::Message;
    /// Merge a message and advance.
    fn receive(clock: &mut Self::Clock, message: Self::Message, rng: &mut ChaChaRng);
    /// Reconcile two clocks, reporting whether synchronization succeeded.
    fn sync(left: &mut Self::Clock, right: &mut Self::Clock, rng: &mut ChaChaRng) -> bool;
    /// Absorb a clock, returning it unchanged when its party overlaps.
    fn join(
        clock: &mut Self::Clock,
        other: Self::Clock,
        rng: &mut ChaChaRng,
    ) -> Result<(), Self::Clock>;
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

    fn send(clock: &mut Self::Clock, _rng: &mut ChaChaRng) -> Self::Message {
        clock.send().clone()
    }

    fn receive(clock: &mut Self::Clock, message: Self::Message, _rng: &mut ChaChaRng) {
        clock.recv(&message);
    }

    fn sync(left: &mut Self::Clock, right: &mut Self::Clock, _rng: &mut ChaChaRng) -> bool {
        left.sync(right).is_ok()
    }

    fn join(
        clock: &mut Self::Clock,
        other: Self::Clock,
        _rng: &mut ChaChaRng,
    ) -> Result<(), Self::Clock> {
        clock.join(other).map(|_| ())
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

    fn send(clock: &mut Self::Clock, _rng: &mut ChaChaRng) -> Self::Message {
        clock.send()
    }

    fn receive(clock: &mut Self::Clock, message: Self::Message, _rng: &mut ChaChaRng) {
        clock.receive(message);
    }

    fn sync(left: &mut Self::Clock, right: &mut Self::Clock, _rng: &mut ChaChaRng) -> bool {
        left.sync(right).is_ok()
    }

    fn join(
        clock: &mut Self::Clock,
        other: Self::Clock,
        _rng: &mut ChaChaRng,
    ) -> Result<(), Self::Clock> {
        clock.join(other)
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

    fn send(clock: &mut Self::Clock, rng: &mut ChaChaRng) -> Self::Message {
        clock.send(rng)
    }

    fn receive(clock: &mut Self::Clock, message: Self::Message, rng: &mut ChaChaRng) {
        clock.receive(message, rng);
    }

    fn sync(left: &mut Self::Clock, right: &mut Self::Clock, rng: &mut ChaChaRng) -> bool {
        left.sync(right, rng).is_ok()
    }

    fn join(
        clock: &mut Self::Clock,
        other: Self::Clock,
        _rng: &mut ChaChaRng,
    ) -> Result<(), Self::Clock> {
        clock.join(other)
    }
}

/// One step of a seed-derived execution.
///
/// [`Index`] maps each operand across the entire live population at the moment
/// the step runs. Thus clocks appended by earlier forks remain eligible for
/// later operations. Every member descends from one seed through fork, join,
/// and sync, keeping all live parties pairwise disjoint.
#[derive(Clone, Debug)]
pub(crate) enum Op {
    /// Advance member `i`.
    Tick(Index),
    /// Advance member `i` by `n` events in one fused call (small `n`:
    /// the oracle applier iterates it literally).
    Ticks(Index, u8),
    /// Split member `i`, appending the child.
    Fork(Index),
    /// `i` sends (ticks, emits its version); `j` receives it.
    Send(Index, Index),
    /// Reconcile `i` and `j` (join then re-split).
    Sync(Index, Index),
    /// Join `j` into `i`, removing `j`.
    Join(Index, Index),
}

fn op_strategy() -> impl Strategy<Value = Op> {
    prop_oneof![
        any::<Index>().prop_map(Op::Tick),
        (any::<Index>(), 0u8..=6).prop_map(|(i, n)| Op::Ticks(i, n)),
        any::<Index>().prop_map(Op::Fork),
        (any::<Index>(), any::<Index>()).prop_map(|(a, b)| Op::Send(a, b)),
        (any::<Index>(), any::<Index>()).prop_map(|(a, b)| Op::Sync(a, b)),
        (any::<Index>(), any::<Index>()).prop_map(|(a, b)| Op::Join(a, b)),
    ]
}

/// The default trace-length cap: [`world_strategy`] draws strictly fewer ops
/// than this.
///
/// Consumers that size a resource to the deepest reachable history derive
/// from this constant rather than transcribing it (the semantic oracle
/// derives its comparison-grid ceiling
/// [`super::oracles::function::GRID_N`] from it, so a cap change cannot
/// silently outgrow the grid).
pub(crate) const MAX_TRACE_OPS: usize = 30;

/// A trace of fewer than [`MAX_TRACE_OPS`] ops over a population that starts
/// as a single seed clock.
pub(crate) fn world_strategy() -> impl Strategy<Value = Vec<Op>> {
    world_strategy_up_to(MAX_TRACE_OPS)
}

/// A trace of fewer than `max_ops` ops over a population that starts as a
/// single seed clock, for suites that need histories deeper than
/// [`world_strategy`]'s default cap.
pub(crate) fn world_strategy_up_to(max_ops: usize) -> impl Strategy<Value = Vec<Op>> {
    prop::collection::vec(op_strategy(), 0..max_ops)
}

/// Result of applying a trace to one [`TraceModel`].
pub(crate) struct Replay<C> {
    /// Final live clock population, in trace index order.
    pub(crate) clocks: Vec<C>,
    /// Whether each attempted join or sync succeeded, in operation order.
    pub(crate) accepted: Vec<bool>,
}

/// Apply one operation through a model.
///
/// Population indexing and rejected-join restoration live here so the
/// operation enum's exhaustive match is the one schedule definition used by
/// every model and by tests that inspect production after each step.
fn apply<M: TraceModel>(
    clocks: &mut Vec<M::Clock>,
    op: &Op,
    rng: &mut ChaChaRng,
    accepted: &mut Vec<bool>,
) {
    let n = clocks.len();
    match *op {
        Op::Tick(i) => M::tick(&mut clocks[i.index(n)], rng),
        Op::Ticks(i, count) => M::ticks(&mut clocks[i.index(n)], count, rng),
        Op::Fork(i) => {
            let child = M::fork(&mut clocks[i.index(n)], rng);
            clocks.push(child);
        }
        Op::Send(i, j) => {
            let (i, j) = (i.index(n), j.index(n));
            let message = M::send(&mut clocks[i], rng);
            M::receive(&mut clocks[j], message, rng);
        }
        Op::Sync(i, j) => {
            let (i, j) = (i.index(n), j.index(n));
            if i != j {
                let (lo, hi) = (i.min(j), i.max(j));
                let (left, right) = clocks.split_at_mut(hi);
                accepted.push(M::sync(&mut left[lo], &mut right[0], rng));
            }
        }
        Op::Join(i, j) => {
            if n > 1 {
                let (i, j) = (i.index(n), j.index(n));
                if i != j {
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
            }
        }
    }
}

/// Apply a trace to one seed-derived universe through a model's complete
/// state-transition interface.
///
/// This is the sole spelling of the trace schedule. The model trait supplies
/// only the operations themselves, so index handling, removal and restoration
/// after a rejected join, and the definition of a skipped self-operation
/// cannot drift among the three implementations. The driver creates exactly
/// one seed so independently seeded universes are unrepresentable here.
pub(crate) fn replay<M: TraceModel>(ops: &[Op], rng: &mut ChaChaRng) -> Replay<M::Clock> {
    let mut clocks = vec![M::seed()];
    let mut accepted = Vec::new();
    for op in ops {
        apply::<M>(&mut clocks, op, rng, &mut accepted);
    }
    Replay { clocks, accepted }
}

/// Apply a trace to a fresh oracle population.
pub(crate) fn run(ops: &[Op]) -> Vec<tree::Clock> {
    let replay = replay::<Recursive>(ops, &mut ChaChaRng::seed_from_u64(0));
    assert!(
        replay.accepted.iter().all(|accepted| *accepted),
        "single-seed trace produced overlapping live parties"
    );
    replay.clocks
}

/// Apply one operation to an existing production population.
///
/// This incremental form lets tests assert after every step while delegating to
/// the same scheduler as [`replay`].
pub(crate) fn step_impl(imp: &mut Vec<Clock>, op: &Op) {
    let mut accepted = Vec::new();
    apply::<Production>(imp, op, &mut ChaChaRng::seed_from_u64(0), &mut accepted);
    assert!(
        accepted.iter().all(|accepted| *accepted),
        "single-seed trace produced overlapping live parties"
    );
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
