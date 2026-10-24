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
//! # Costs and storage
//!
//! Cancellation can shorten the value while the accumulator still holds
//! storage at higher positions. Costs therefore use the *working width*
//! rather than the value's bit length, through these quantities:
//!
//! - A receiver's *history* is the time since its storage was created or
//!   last received [`clone_from`](Clone::clone_from). The result of an owned
//!   operator continues the history of the input whose storage it reuses.
//! - `V` is the receiver's working width when the operation starts, and `W`
//!   the greatest working width it has reached in its history.
//! - `A` is an accumulator operand's working width, and `L` the number of
//!   input limbs.
//! - `G` is how far an update raises the receiver's working width beyond the
//!   greatest it has reached in its *window*: the part of its history since
//!   its last nonzero `<<=`, which can move the value into new storage. A
//!   large shift can make `G` large even for one input limb, but raising the
//!   width again to one reached in the window adds nothing to `G`; see
//!   [Raising the top across gaps](#raising-the-top-across-gaps). At shift 0,
//!   `G` exceeds the operand's own width by at most a constant, so it drops
//!   out of the shift-0 bounds.
//! - `Q` is the width of a read's result, and `S` the working width of a
//!   shift's result.
//!
//! Amortized bounds hold over the whole sequence of operations: one call may
//! finish carry or cancellation work that earlier calls prepared, but no work
//! is paid for twice. Additional space excludes inputs and returned output.
//!
//! | Operation | Time | Additional space |
//! |---|---|---|
//! | `+=` / `-=` a primitive | Amortized O(1) | Amortized O(1) retained |
//! | Add or subtract `L` limbs | Amortized O(`L`) at shift 0; amortized O(`L` + `G` + log(`W` + 1)) at a nonzero shift | Amortized O(`L` + `G`) retained |
//! | Add or subtract an accumulator | Amortized O(`A`) at shift 0; amortized O(`A` log(`W` + 1) + `G`) at a nonzero shift | Amortized O(`A` + `G`) retained |
//! | Nonzero `<<` or `<<=`, any spelling | Amortized O(`S`) | O(`S`) temporary |
//! | Sign queries, [`cmp_zero_stable_under`](Accumulator::cmp_zero_stable_under), comparison with a primitive, and primitive `TryFrom` | Amortized O(1) | O(1) |
//! | [`abs_diff`](Accumulator::abs_diff) | Amortized O(`A`), `A` the narrower; plus O(`V`), `V` the wider operand's working width, to negate the result when the wider operand is negative and the smaller | Amortized O(1) retained beyond the wider operand's storage |
//! | Compare two accumulators | Amortized O(`A`), `A` the narrower; amortized O(1) when one is far wider and its top decides the order | O(`A`) temporary |
//! | Magnitude read or `Hash` | Amortized O(1 + `Q`) | O(`Q`) temporary |
//! | `clone` / `clone_from` | The magnitude read plus O(`Q`) | O(`Q`) for the copy; `clone_from` reuses the receiver's allocation |
//! | `Display` | The magnitude read plus O(`Q`²) | O(`Q`) temporary |
//! | Working-width and known-zero queries | O(1) | O(1) |
//! | Unary `-` | Amortized O(1 + `Q`), `Q` the value's width | O(1) |
//! | [`clear`](Accumulator::clear) | Amortized O(1) | O(1) |
//!
//! The logarithmic terms appear only in the rows for updates at a nonzero
//! shift. They pay for the bookkeeping that lets comparisons skip positions
//! no update has written, grow as the base-64 logarithm of `W`, and cover
//! crossing a range of such positions, which only an update at a nonzero
//! shift, or the comparison that follows it, can need. Added at a shift that
//! is not a multiple of 32 bits, an accumulator operand can pay the term once
//! per 32 bits of its width, which is why that row multiplies `A` by it.
//!
//! ## Raising the top across gaps
//!
//! Cancellation can empty a long range of bit positions below the top, and a
//! large shift skips a range that no update has written; call either range
//! a *gap*. Raising the top across a gap, and finding the value's new top
//! below it after the value above it cancels, never visits the gap:
//!
//! - An update that raises the working width again, to a width reached in
//!   its window, has a `G` of zero however far the top moves, so its row's
//!   bounds apply as they would to an update that does not widen the value.
//! - When the value above the gap cancels, the update or comparison that
//!   finds the value's new highest nonzero bit crosses a gap of `d` bits in
//!   amortized O(1 + log(`d` + 1)) time, a base-64 logarithm, so no cycle of
//!   raising and cancelling pays the gap's width.
//!
//! The rows above already pay for these crossings. An update at a nonzero
//! shift pays, within its log(`W` + 1) term, for the crossings it makes
//! itself and for those of the next comparison, which may cross the gap
//! below the top the update left. An update at shift 0 reaches the top only
//! through positions it writes, which its own width pays for. So a value
//! whose top is raised across a gap and cancelled again, cycle after cycle,
//! costs each cycle only the bounds of the updates it makes, never the gap's
//! width.
//!
//! ## Updates of one sign
//!
//! Updates that only add, or only subtract, never cancel the value's top,
//! and cost less than the rows above say. Call a sequence of operations on
//! one accumulator a *stretch of one sign* when:
//!
//! - it begins where the receiver's window begins: at its creation
//!   (including the result of a borrowed `<<`, a new accumulator), at its
//!   last [`clone_from`](Clone::clone_from), or at its last nonzero `<<=` or
//!   owned `<<`;
//! - every update in it adds or subtracts a primitive or limbs, all their
//!   contributions have one sign, and the value at its start is zero or has
//!   that sign;
//! - its only other operations are working-width and known-zero queries.
//!
//! Any other operation ends the stretch. Then:
//!
//! - each primitive update in the stretch costs amortized O(1), and each
//!   update of `L` limbs amortized O(`L` + `G`), at any shift;
//! - the first comparison with zero after the stretch, which every read,
//!   sign query, comparison, and nonzero shift begins with, can cost
//!   O(log(`V` + 1)) beyond its own row, once.
//!
//! That is, a stretch of one sign pays the logarithmic term of the limb row
//! at most once, after it ends, instead of on every update at a nonzero
//! shift. A nonzero shift in any spelling of `<<` costs amortized O(`S`)
//! and starts a new stretch, so a sum of terms of one sign can shift its
//! total to a finer scale between stretches and pay each shift only its
//! width.
//!
//! A [`clear`](Accumulator::clear) or a read does not begin a stretch of one
//! sign: it leaves the window running, and it empties the bookkeeping that
//! lets comparisons skip unwritten positions while keeping that
//! bookkeeping's storage. Repeating a clear, an update at a large shift that
//! reaches a width the window reached before, and a one-limb update below
//! it costs a base-64 logarithm of that width per round, while each update's
//! `L` + `G` stays constant and the clear costs amortized O(1). Updates after
//! a clear or a
//! read therefore get only the rows above.
//!
//! ## Retained space
//!
//! Every retained bound is amortized, because storage grows geometrically, as
//! `Vec` does: an operation that extends a full allocation by one position can
//! retain as much spare capacity as the allocation it replaced, and later
//! growth fills that capacity. The bookkeeping can also grow during an update
//! that does not widen the value, to cover positions earlier updates wrote.
//! Growing an allocation may briefly keep the old and new buffers alive. A
//! read can extend the working width by at most a constant, and grows the
//! buffer by exactly that much when it must. An accumulator may retain more
//! allocation than its working width after cancellation or
//! [`clear`](Accumulator::clear); [`reserve_bits`](Accumulator::reserve_bits)
//! avoids repeated growth.
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
//!   establish the bounds above. Counts are deterministic for a fixed
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
