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
//!   `64 · u`, and `4096 · u`, the last reaching values of a few
//!   megabytes.
//! - [`model`] replays a program on exact big integers, independently of
//!   `suanpan`, and prices each step under the rules.
//! - [`guest`] runs a program in the compiled guest (`suanpan-fuel-guest`)
//!   under the fuel board's operator prices.
//! - [`judge`] runs every variant of a skeleton and compares the readings.
//! - [`claims`] runs fixed programs for the crate's focused cost claims, each
//!   a sequence and a parameter chosen by hand, and states the forms the
//!   claims in `tests/claims.rs` take.
//!
//! The suites run from `just suanpan-fuel`, which builds the guest first.
//!
//! # Testing
//!
//! Each program is its own control: every reading is compared with a reading
//! of the same skeleton at another scale or repetition count, or of the same
//! fixed program at another value of its parameter, and no fuel number is
//! committed anywhere. The [`judge`] module states each bound of the
//! interleaving property and what it rests on; [`claims`] states the forms
//! of the focused claims.
//!
//! What the interleaving property samples that no other instrument does is
//! *order*. The focused claims and `suanpan`'s touch tests measure chosen
//! operation sequences, and the amplification board measures each operation
//! on chosen worst-case families. All fix the sequence. Here the sequence is
//! drawn, over every regular operation (updates by primitives, limbs, and
//! accumulators in every owned and borrowed spelling, shifted updates and
//! shifts, negation, clear, sign queries, comparisons, conversions,
//! magnitude reads and hashes, clones, `clone_from`, `abs_diff`, and sums),
//! on operands shaped to cancel, borrow across long runs, and leave sparse
//! high digits, so a cost that depends on which operation follows which is
//! reachable. Two spellings are left to the focused claims: conversions to
//! types other than `i64`, and a primitive on the left of `+` or `-`.
//!
//! What the checks see, by the shape of the excess work (the [`judge`]
//! module states each bound and what it rests on):
//!
//! - *Work that grows faster than the sizes* (quadratic work, a logarithm
//!   multiplying a length): the scaled comparisons at the end of each
//!   repetition, where every deferral of the repetition has come due, in any
//!   interleaving that reaches it.
//! - *Work that grows with the length of the history at a fixed size*
//!   (positions never reclaimed, a scan that revisits what earlier
//!   repetitions wrote): the long-repetition comparison.
//! - *Work linear in a size, paid again in every repetition whose steps
//!   charge no size* (growth paid again after a clear, a gap crossed at a
//!   cost linear in its width): the steady-repetition comparison at two
//!   scales.
//! - *An update, sign query, comparison with a primitive, conversion that
//!   refuses, or owned negation whose work is linear in the width of a
//!   member it touches, inside a program that charges sizes elsewhere* (an
//!   update that pays its receiver's width, a sign query that rescans a wide
//!   value): the block comparison. A value-neutral block of such steps runs
//!   repeatedly after the context; once each call costs exactly what the
//!   last did, the state is taken to repeat, and the potential with it, so
//!   the cost of a call is its own, and at two scales it may differ only by
//!   the logarithm.
//! - *Work linear in a size the rules do not charge to a step whose charge
//!   has a sized term of its own* (an operand addition that also pays its
//!   receiver's width): nothing here, since that term scales by the same
//!   factor. Nor can a reading separate work the context defers and a later
//!   step pays once, which is what amortization allows, from the same work
//!   paid by a defect once.

pub mod claims;
pub mod guest;
pub mod judge;
pub mod model;
pub mod skeleton;
