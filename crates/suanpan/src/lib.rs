//! Exact signed accumulation with bounded carry and cancellation work.
//!
//! [`Accumulator`] is a signed integer of any width, built for running totals
//! whose updates change sign near carry boundaries: a weighted sum that moves
//! back and forth across `2^k`, for example. A normalized big integer carries
//! through the same high digits on every such update. An accumulator keeps
//! redundant signed digits instead, so each update touches only a few digits,
//! and each read compacts the cancellation it touches, so no later operation
//! pays for it again.
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
//!   either side of `+` and `-`; `5 - total` negates `total`, at the cost of
//!   unary `-`. Shift counts are primitives, owned or borrowed.
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
//! form of the same value. The value never changes, but the *working width*
//! that [`stored_bits`](Accumulator::stored_bits) reports can. Reads go
//! through a [`RefCell`](core::cell::RefCell), so they take `&self`; as a
//! result an accumulator is `Send` but not `Sync`.
//!
//! An accumulator is not `Clone`: a copy would carry the same cancellation as
//! its original, and both would pay to compact it. To copy a value, construct
//! `Accumulator::from(&total)`, which reads, and so compacts, the original
//! first.
//!
//! [`is_known_zero`](Accumulator::is_known_zero) is a constant-time check that
//! reads nothing: `true` proves zero, and `false` proves nothing.
//! [`cmp_zero_stable_under`](Accumulator::cmp_zero_stable_under) proves that
//! no adjustment below a given width can change the sign, which lets a caller
//! skip reading a much narrower operand.
//!
//! # Costs and storage
//!
//! Cancellation can shorten the value while the accumulator still holds
//! digits at higher positions. Costs therefore use the *working width*
//! rather than the value's bit length. Let `W` be the greatest working width
//! the receiver has reached, `A` an accumulator operand's working width, `L`
//! the number of input limbs, `G` the growth of the receiver's working width,
//! and `Q` the width of a read's result. A large shift can make `G` large even
//! for one input limb.
//!
//! Amortized bounds hold over the whole sequence of operations: one call may
//! finish carry or cancellation work that earlier calls prepared, but no work
//! is paid for twice. Additional space excludes inputs and returned output.
//!
//! | Operation | Time | Additional space |
//! |---|---|---|
//! | `+=` / `-=` a primitive | Amortized O(log(`W` + 1)) | O(1) retained |
//! | Add or subtract `L` limbs | Amortized O(`L` log(`W` + 1) + `G`) | O(`L` + `G`) retained |
//! | Add or subtract an accumulator | Amortized O(`A` log(`W` + 1) + `G`) | O(`A` + `G`) retained |
//! | Sign queries, comparison with a primitive, primitive `TryFrom` | Amortized O(log(`W` + 1)) | O(1) |
//! | [`abs_diff`](Accumulator::abs_diff) | Amortized O(`A` log(`W` + 1)), `A` the narrower; plus O(`W`) to negate the result when the wider operand is negative and the smaller | O(1) beyond the wider operand's storage |
//! | Compare two accumulators | Amortized O(`A` log(`A` + 1)), `A` the narrower; O(log(`W` + 1)) when one is far wider | O(`A`) temporary |
//! | Magnitude read or `Hash` | Amortized O(log(`W` + 1) + `Q`) | O(`Q`) temporary |
//! | `Display` | The magnitude read plus O(`Q`²) | O(`Q`) temporary |
//! | Working-width and known-zero queries | O(1) | O(1) |
//! | Unary `-` and [`reset`](Accumulator::reset) | O(`W`) | O(1) |
//!
//! Growing an allocation may briefly keep the old and new buffers alive. A
//! read can extend the working width by at most one 32-bit position, and
//! grows the buffer by exactly that position when it must. An accumulator may retain more allocation than its
//! working width after cancellation or [`reset`](Accumulator::reset);
//! [`reserve_bits`](Accumulator::reserve_bits) avoids repeated growth.
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
//! - `touch-meter` exposes a process-global counter of digit work in the
//!   `touch_meter` module, for regression tests of the arithmetic core. It
//!   does not count allocation or all bookkeeping, so it does not by itself
//!   establish the bounds above. Counts are deterministic for a fixed
//!   implementation and operation sequence, but are no compatibility promise.
//!   Run measured scenarios serially. Without the feature, counting compiles
//!   away.

#![forbid(unsafe_code)]

mod accumulator;
#[cfg(feature = "touch-meter")]
pub mod touch_meter;

#[cfg(feature = "num-bigint")]
pub use accumulator::NegativeValueError;
pub use accumulator::{Accumulator, Limbs};
