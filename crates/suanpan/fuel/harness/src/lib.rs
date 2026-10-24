//! Judges `suanpan`'s cost rules in Wasmtime fuel over random interleavings
//! of accumulator operations.
//!
//! `suanpan`'s crate documentation states four amortized cost rules (an
//! update costs what it adds, a read costs what it returns, growing storage
//! to a width costs once, and a shifted update adds a logarithm) and a rule
//! for ownership. An amortization argument can hold for every operation on
//! its own and still fail for some order of operations: deferred work that a
//! later step pays twice, storage paid for again after a clear, a crossing
//! that the next read repeats. This crate generates programs over a pool of
//! accumulators, runs each in a wasm guest that reads fuel after every step,
//! and checks that the readings grow only as the rules allow when the
//! program's sizes and its repetition count grow.
//!
//! - [`skeleton`] generates programs whose lengths are measured against a
//!   unit `u`, so one skeleton yields the same program at units `u`,
//!   `64 · u`, and `4096 · u`, the last reaching values of about two
//!   megabytes.
//! - [`model`] replays a program on exact big integers, independently of
//!   `suanpan`, and prices each step under the rules.
//! - [`guest`] runs a program in the compiled guest (`suanpan-fuel-guest`)
//!   under the fuel board's operator prices.
//! - [`judge`] runs every variant of a skeleton and compares the readings.
//!
//! The suite runs from `just suanpan-fuel`, which builds the guest first.
//!
//! # Testing
//!
//! Each program is its own control: every reading is compared with a reading
//! of the same skeleton at another scale or repetition count, and no fuel
//! number is committed anywhere. The [`judge`] module states each bound and
//! derives it from the rules.
//!
//! What this samples that no other instrument does is *order*. `suanpan`'s
//! touch tests measure chosen operation sequences at fixed widths, and the
//! amplification board measures each operation on chosen worst-case
//! families. Both fix the sequence. Here the sequence is drawn, over every
//! regular operation (updates by primitives, limbs, and accumulators in every
//! owned and borrowed spelling, shifted updates and shifts, negation, clear,
//! sign queries, comparisons, conversions, magnitude reads and hashes,
//! clones, `clone_from`, `abs_diff`, and sums), on operands shaped to cancel,
//! borrow across long runs, and leave sparse high digits, so a cost that
//! depends on which operation follows which is reachable.
//!
//! What the checks see, by the shape of the excess work:
//!
//! - *Work that grows faster than the sizes* (quadratic work, a logarithm
//!   multiplying a length, a buffer copied at every step): the per-prefix
//!   comparisons across scales see it in any interleaving that reaches it.
//! - *Work that grows with the length of the history at a fixed size*
//!   (positions that are never reclaimed, a scan that revisits everything an
//!   earlier repetition wrote): the long-repetition comparison sees it.
//! - *Work linear in a size, paid again in every repetition whose steps
//!   charge no size at all* (growth paid again after a clear, a gap crossed
//!   at a cost linear in its width): the steady-repetition comparison at two
//!   scales sees it.
//! - *Work linear in a size the rules do not charge to the step doing it,
//!   inside a program that also charges sizes* (an update that pays its
//!   receiver's width among reads of wide values): nothing here sees it.
//!   Such work scales exactly as charged work does. Learning a constant per
//!   operation class does not separate the two either: correct interleavings
//!   already contain steps charged one word whose fuel grows with the unit,
//!   such as a single-limb write that converts the written set to a bitset,
//!   or a clear that passes the positions a read left. Those steps spend
//!   potential that earlier steps deposited, so any sound per-class bound
//!   must let a step draw on a potential whose rate it learns, and that rate
//!   then covers any excess proportional to the same deposits. Separating
//!   them would take the potential itself, which only the implementation
//!   holds.

pub mod guest;
pub mod judge;
pub mod model;
pub mod skeleton;
