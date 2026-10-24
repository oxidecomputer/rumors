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
//! What it cannot see follows from comparing readings without a committed
//! constant. Work linear in a size that the rules do not charge to the step
//! doing it scales exactly as charged work does, so scaling alone cannot
//! separate the two; the one such case it isolates is a repetition whose
//! steps charge no size at all once storage is grown, compared at two
//! scales. The comparisons in units of charge, and their limits, are
//! derived in [`judge`].

pub mod guest;
pub mod judge;
pub mod model;
pub mod skeleton;
