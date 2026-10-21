//! Allocation checks for the composite encoders.
//!
//! A [`Span`](before::Span) encodes as its two endpoints' encodings
//! concatenated, and a [`Clock`] as its party's encoding followed by its
//! version's. Each encoder allocates its output once, at the summed length of
//! its parts, so it copies each part's bytes exactly once and never holds a
//! second buffer.

use super::power_of_two;
use alloc_meter::Stats;
use before::{Clock, Version};
use proptest::collection::vec;
use proptest::prelude::*;
use proptest::sample::Index;

/// One step of a generated history over a pool of clocks forked from one seed.
#[derive(Clone, Debug)]
enum Step {
    /// The selected clock records one event.
    Tick(Index),
    /// The selected clock records `2^k` events at once, so payloads can grow
    /// past the machine word.
    Leap(Index, usize),
    /// The first selected clock receives a send from the second.
    Recv(Index, Index),
}

/// Generates one history step, weighted toward the steps that interleave
/// parties.
fn step() -> impl Strategy<Value = Step> {
    prop_oneof![
        2 => any::<Index>().prop_map(Step::Tick),
        1 => (any::<Index>(), 0usize..=96).prop_map(|(i, k)| Step::Leap(i, k)),
        3 => (any::<Index>(), any::<Index>()).prop_map(|(i, j)| Step::Recv(i, j)),
    ]
}

/// What a generated history leaves behind.
struct History {
    /// The pool of clocks, each holding one fork share of the seed party.
    clocks: Vec<Clock>,
    /// The version each step left at the clock it advanced, in step order.
    versions: Vec<Version>,
}

/// Runs `steps` over `clocks` clocks forked from one seed.
fn history(clocks: usize, steps: &[Step]) -> History {
    let mut seed = Clock::seed();
    let mut pool: Vec<Clock> = (1..clocks).map(|_| seed.fork()).collect();
    pool.push(seed);
    let versions = steps
        .iter()
        .map(|step| match step {
            Step::Tick(i) => pool[i.index(clocks)].tick().clone(),
            Step::Leap(i, k) => pool[i.index(clocks)].ticks(power_of_two(*k)).clone(),
            Step::Recv(i, j) => {
                let sent = pool[j.index(clocks)].send().clone();
                pool[i.index(clocks)].recv(&sent).clone()
            }
        })
        .collect();
    History {
        clocks: pool,
        versions,
    }
}

/// The reading of an encoder that allocates its output once, at exactly
/// `len` bytes, and returns it.
fn one_allocation_of(len: usize) -> Stats {
    Stats {
        allocations: 1,
        reallocations: 0,
        bytes_allocated: len,
        peak_bytes: len,
        net_bytes: isize::try_from(len).expect("an encoding in memory fits isize"),
    }
}

proptest! {
    /// `Span::encode` allocates its output once, at exactly the summed length
    /// of the two endpoints' encodings.
    ///
    /// The reading is compared whole: one allocation, no reallocation, and
    /// requested, peak, and retained bytes all equal to that length. The spans
    /// are hulls of two snapshots drawn from one generated multi-party history,
    /// comparable or concurrent.
    #[test]
    fn span_encode_allocates_once_at_its_length(
        clocks in 1usize..=4,
        steps in vec(step(), 1..48),
        picks in (any::<Index>(), any::<Index>()),
    ) {
        let versions = history(clocks, &steps).versions;
        let span = versions[picks.0.index(versions.len())]
            .span(&versions[picks.1.index(versions.len())]);
        let len = span.lo().as_bytes().len() + span.hi().as_bytes().len();
        let (stats, bytes) = alloc_meter::measure(|| span.encode());
        prop_assert_eq!(bytes.len(), len);
        prop_assert_eq!(stats, one_allocation_of(len));
    }

    /// `Clock::encode` allocates its output once, at exactly the summed length
    /// of its party's and its version's encodings.
    ///
    /// The reading is compared whole, as for spans. The clocks are the pool a
    /// generated multi-party history leaves behind, so their parties are fork
    /// shares of one seed and their versions carry received events.
    #[test]
    fn clock_encode_allocates_once_at_its_length(
        clocks in 1usize..=8,
        steps in vec(step(), 0..48),
        pick in any::<Index>(),
    ) {
        let pool = history(clocks, &steps).clocks;
        let clock = &pool[pick.index(clocks)];
        let len = clock.party().as_bytes().len() + clock.version().as_bytes().len();
        let (stats, bytes) = alloc_meter::measure(|| clock.encode());
        prop_assert_eq!(bytes.len(), len);
        prop_assert_eq!(stats, one_allocation_of(len));
    }
}
