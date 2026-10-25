//! Exact signed accumulation with bounded carry and cancellation work.
//!
//! [`Accumulator`] is a signed integer of any width, built for running totals
//! whose updates change sign near carry boundaries: a weighted sum that moves
//! back and forth across `2^k`, for example. A normalized big integer carries
//! through the same high positions on every such update. An accumulator keeps
//! a redundant representation instead, so each update touches only a few
//! positions, and each read compacts the cancellation it touches, so no later
//! operation pays for it again.
//!
//! ```
//! use suanpan::Accumulator;
//!
//! let mut total = Accumulator::new();
//! total.add_shifted_limbs(512, [1]); // 2^512
//! for _ in 0..1_000 {
//!     total -= 1;
//!     assert!(total.is_positive());
//!     total += 1;
//! }
//! assert!(total > u128::MAX);
//! ```
//!
//! # Operations
//!
//! An accumulator behaves as a standard integer without multiplication,
//! division, or right shift:
//!
//! - [`Accumulator::new`] and `Default` construct zero, and `From` converts any
//!   primitive integer.
//! - `+`, `-`, `+=`, `-=`, unary `-`, `<<`, `<<=`, and `Sum` take owned and
//!   borrowed accumulators and primitive integers. A primitive may stand on
//!   either side of `+` and `-`; `5 - total` negates `total`. Shift counts
//!   are primitives, owned or borrowed.
//! - `==`, `<`, [`cmp`](Ord::cmp), and `Hash` work by value, against another
//!   accumulator or against a primitive integer on the right.
//!   [`abs_diff`](Accumulator::abs_diff) compares two accumulators and
//!   subtracts the smaller from the larger in one pass.
//! - [`sign`](Accumulator::sign), [`is_zero`](Accumulator::is_zero),
//!   [`is_positive`](Accumulator::is_positive), and
//!   [`is_negative`](Accumulator::is_negative) query the sign, and `Display`
//!   writes the value in decimal.
//! - `TryFrom` extracts a primitive integer exactly, and returns the
//!   accumulator if the value does not fit.
//! - [`clear`](Accumulator::clear) sets the value to zero and keeps the
//!   storage, as `Vec::clear` does.
//!
//! Wide values stream in as little-endian `u64` limbs through
//! [`add_shifted_limbs`](Accumulator::add_shifted_limbs) and
//! [`sub_shifted_limbs`](Accumulator::sub_shifted_limbs), scaled by the power of
//! two given first. [`add_shifted`](Accumulator::add_shifted) and
//! [`sub_shifted`](Accumulator::sub_shifted) take another accumulator the same
//! way, and [`signed_magnitude`](Accumulator::signed_magnitude) reads a value
//! out as [`Limbs`], which dereference to a `[u64]` slice and stay inline
//! below `2^128`.
//!
//! # Reads compact what they touch
//!
//! Every read, whether a comparison, a sign query, a conversion, formatting,
//! or use as an operand, rewrites the cancellation it reads into a shorter
//! form of the same value. The value never changes, but the stored form can,
//! and with it the bound on the value's width that
//! [`stored_bits`](Accumulator::stored_bits) reports. Reads go through a
//! [`RefCell`](core::cell::RefCell), so they take `&self`; as a result an
//! accumulator is `Send` but not `Sync`.
//!
//! A [`clone`](Clone::clone) is a read too: it normalizes the original in
//! place, then copies only the value's own width, so the copy starts with no
//! deferred cancellation.
//!
//! [`is_known_zero`](Accumulator::is_known_zero) is a constant-time check that
//! reads nothing: `true` proves zero, and `false` proves nothing.
//! [`cmp_zero_stable_under`](Accumulator::cmp_zero_stable_under) proves that
//! no adjustment below a given width can change the sign, which lets a caller
//! skip reading a much narrower operand.
//!
//! # Costs
//!
//! An accumulator's costs follow from what you pass in and what you read
//! out. An update never pays linearly for the width of the accumulator it
//! changes: beyond a logarithm at a nonzero shift, it pays only to grow
//! storage, once. Four rules give the cost of updates, reads, and storage:
//!
//! - **An update costs what it adds.** Adding or subtracting a primitive
//!   costs O(1), `L` limbs O(`L`), and another accumulator O(1 + `n`) for an
//!   `n`-limb value, however wide the receiver is and however often updates
//!   change its sign.
//! - **A read costs what it returns.** A sign query, a comparison with a
//!   primitive, or a conversion to a primitive returns a constant and costs
//!   O(1). A magnitude read, a hash, or a clone costs the width of the value
//!   it returns.
//! - **Growing storage costs once.** An accumulator stores every bit from 0
//!   up to the highest bit it has written, its *storage width*. Unlike the
//!   value's width, which falls when the value does, the storage width only
//!   rises: writing above it costs the distance, once, and writing that high
//!   again costs nothing more. [`clear`](Accumulator::clear) keeps the
//!   storage width; `clone_from` and a nonzero shift start it again at their
//!   result's width.
//! - **A shifted update adds a logarithm.** An update at a nonzero shift
//!   costs an extra base-64 logarithm of the storage width, at most 11 steps
//!   on a 64-bit target and 6 on a 32-bit one, to record which bits it wrote,
//!   so that later reads can skip the bits no update has written.
//!
//! These costs are amortized, as `Vec::push`'s is. One call can finish
//! carries or cancellation that earlier updates left, and then cost up to the
//! storage width of the accumulators it touches; but the update that left
//! that work paid for it, and no work is done twice. That is: over any
//! sequence of operations, the total time is proportional to the bits passed
//! in, plus the bits read out, plus the width each storage reaches, plus one
//! logarithm per shifted update.
//!
//! Memory follows the widest storage an accumulator has held, as a `Vec`'s
//! allocation follows its capacity: `clear`, `clone_from`, and a shift of a
//! value that has needed no heap allocation since creation or the last clear
//! keep the allocation they find, and
//! [`reserve_bits`](Accumulator::reserve_bits) can enlarge it. An operation's
//! temporary space is at most the width it reads or returns.
//!
//! Note that no rule charges an update linearly for its receiver's width,
//! only rule 4's logarithm at a nonzero shift. A normalized big integer's
//! update can cost its whole width, carrying or borrowing through high bits
//! the update never touched; an accumulator's update cannot.
//!
//! The remaining operations cost as follows. Negation and shifts build a new
//! value, and cost its width, as a clone does. `Display` adds to its read a
//! decimal conversion that costs the square of the value's width. Comparing
//! two accumulators reads only the narrower one, and costs its width.
//! [`clear`](Accumulator::clear) costs amortized O(1).
//!
//! In arithmetic, ownership decides only what the result can reuse. A
//! borrowed accumulator is read in full, and an owned one can lend its
//! storage to the result; of two owned operands, only the narrower is read.
//! So `a + b` reads only the narrower of the two, `a + &b` reads `b`, and
//! `&a + &b` reads both. Subtraction reads its right operand in full, owned
//! or borrowed, because reusing it would mean negating it: `a - b` subtracts
//! `b` into `a`'s storage, `&a - b` subtracts `a` from `b` and negates the
//! result, and `5 - a` negates `a` and adds 5.
//! [`abs_diff`](Accumulator::abs_diff) reads only the narrower of its two
//! owned operands, then negates the result when the wider operand was the
//! smaller value, which costs the result's width. An operator and the method
//! it names cost the same.
//!
//! ```
//! use suanpan::Accumulator;
//!
//! let mut total = Accumulator::new();
//! // One limb: O(1), plus growing storage to 1,000,001 bits, once, plus a
//! // logarithm for the shift.
//! total.add_shifted_limbs(1_000_000, [1]);
//! for _ in 0..1_000 {
//!     total -= 1; // one primitive: O(1)
//!     assert!(total.is_positive()); // one ordering: O(1)
//!     total += 1;
//! }
//! ```
//!
//! The loop costs O(1,000). A normalized big integer would rewrite all 15,625
//! limbs below `2^1,000,000` at every `-= 1`, and again at every `+= 1`.
//!
//! Growing storage once is what keeps a value that rises and falls cheap:
//!
//! ```
//! # use suanpan::Accumulator;
//! # let mut total = Accumulator::new();
//! # total.add_shifted_limbs(1_000_000, [1]);
//! for _ in 0..1_000 {
//!     // The first round grows the storage to 2,000,001 bits. Every later
//!     // round costs O(1) plus a logarithm per update, however far the top
//!     // moves.
//!     total.add_shifted_limbs(2_000_000, [1]);
//!     total.sub_shifted_limbs(2_000_000, [1]);
//! }
//! # assert!(total.is_positive());
//! ```
//!
//! ## The precise statement
//!
//! Write `|x|` for the bit length of an accumulator's value, taken to be at
//! least 1. A *storage* begins when an accumulator is created (by `new`,
//! `Default`, `From`, `clone`, or a borrowed `<<`), receives `clone_from`, or
//! is shifted by a nonzero count with `<<=` or an owned `<<`. Every other
//! operation keeps it, `clear` included, and an owned operator's result
//! keeps the storage of the operand it reuses. Its width `h` is the highest
//! bit an operation has written since it began. For each operation, `in`
//! counts the 64-bit words it reads from operands: 1 for a primitive, `L` for
//! limbs, and `1 + |b| / 64` for an accumulator operand `b`, or for the
//! narrower of two compared or owned-added operands. `out` counts the words
//! it produces: 1 for an ordering or a primitive, the result's width for a
//! magnitude read, hash, clone, negation, or shift, and that width squared
//! for `Display`. An operation built from others satisfies the claim part
//! by part: an operator on a borrowed left operand costs at most a clone
//! and then the owned operator; `a - b` is `a -= &b`, `&a - b` is
//! `-(b - &a)`, and a primitive minus an accumulator is a negation and then
//! an addition; `abs_diff` subtracts the narrower operand from the wider and
//! negates a negative result; and `Sum` is one addition per input.
//!
//! **Claim.** There is a potential `Φ`, a nonnegative function of every
//! accumulator's stored state that is zero for a new accumulator, such that
//! every operation satisfies
//!
//! `actual + ΔΦ ≤ c · (1 + in + out + Δh + σ · log₆₄(h + 1))`,
//!
//! where `Δh` is how far the operation raises its storage's width and `σ` is
//! 1 for an update at a nonzero shift and 0 otherwise. Summing gives the
//! total over a sequence: at most `c` times the inputs, outputs, and
//! logarithms, plus each storage's final width. One call's actual cost is at
//! most O(`in` + `out` + `Δh` + log₆₄(`h` + 1)) plus the storage widths of
//! the accumulators it touches.
//!
//! ## Updates of one sign
//!
//! A *stretch* is the sequence of operations from the start of a storage
//! until the first operation other than an update by a primitive or limbs,
//! [`stored_bits`](Accumulator::stored_bits), or
//! [`is_known_zero`](Accumulator::is_known_zero). Suppose every update in a
//! stretch has contributions of one sign, and the value at the stretch's
//! start is zero or has that sign. Then those updates pay no logarithm, at
//! any shift, and the first comparison with zero after the stretch, which
//! every read, comparison, and nonzero shift begins with, pays one logarithm
//! of the storage width, once. A stretch cannot begin at
//! [`clear`](Accumulator::clear) or at a read, because neither begins a
//! storage: updates after them can pay a logarithm each, refilling
//! bookkeeping across the width the storage has reached.
//!
//! A sum of terms of one sign can therefore shift its total to a finer scale
//! between stretches, since each nonzero shift begins a storage, and pay each
//! shift only its result's width.
//!
//! # Scope
//!
//! This is an accumulator, not a general-purpose integer: it has no
//! multiplication, division, or right shift. If the value fits a fixed-width
//! integer, that is usually simpler; if you need those operations, use a
//! general big-integer crate such as `num-bigint`. An accumulator pays off when
//! updates change sign near carry boundaries and reads interleave with
//! cancellation. The crate requires `std`.
//!
//! # Features
//!
//! - `num-bigint` converts accumulators to and from `num-bigint`'s `BigInt`
//!   and `BigUint`, and accepts both as `+=` and `-=` operands. Converting a
//!   negative value to `BigUint` fails with `NegativeValueError`.
//! - `touch-meter` exposes a process-global counter of implementation steps in
//!   the `touch_meter` module, for regression tests of the arithmetic core. It
//!   does not count allocation or all bookkeeping, so it does not by itself
//!   establish the costs above. Counts are deterministic for a fixed
//!   implementation and operation sequence, but are no compatibility promise.
//!   Run measured scenarios serially. Without the feature, counting compiles
//!   away.

#![forbid(unsafe_code)]

mod accumulator;
#[cfg(feature = "num-bigint")]
mod interop;
#[cfg(feature = "touch-meter")]
pub mod touch_meter;

pub use accumulator::{Accumulator, Limbs};
#[cfg(feature = "num-bigint")]
pub use interop::NegativeValueError;
