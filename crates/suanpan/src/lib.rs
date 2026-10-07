//! Exact signed accumulation with bounded carry and cancellation work.
//!
//! [`Accumulator`] maintains a running integer while positive and negative
//! updates interleave. It is designed for workloads in which a normalized big
//! integer could repeatedly traverse the same high-order bits—for example, a
//! weighted sum that moves back and forth across a carry boundary. Suanpan
//! postpones that work and ensures that later comparisons do not repeatedly
//! pay for the same cancellation.
//!
//! ```
//! use core::cmp::Ordering;
//! use suanpan::Accumulator;
//!
//! let mut total = Accumulator::new();
//! total.add_shifted_limbs(512, [1]);
//! for _ in 0..1_000 {
//!     total -= 1_i64;
//!     assert_eq!(total.cmp_zero(), Ordering::Greater);
//!     total += 1_i64;
//! }
//! let (sign, limbs) = total.signed_magnitude();
//! assert_eq!(sign, Ordering::Greater);
//! assert_eq!(limbs.as_ref(), [0, 0, 0, 0, 0, 0, 0, 0, 1]);
//! ```
//!
//! # Operations
//!
//! Construct zero with [`Accumulator::new`] or `Default`, or convert any
//! primitive integer with `From`. Standard `+`, `-`, `+=`, `-=`, unary `-`,
//! `<<`, and `<<=` operations accept the corresponding owned, borrowed, or
//! primitive values. Consuming operations reuse their left operand; owned
//! addition may instead reuse either input's allocation because addition is
//! commutative.
//!
//! Arbitrary-width magnitudes can be streamed as little-endian `u64` limbs
//! through [`add_shifted_limbs`](Accumulator::add_shifted_limbs) and
//! [`sub_shifted_limbs`](Accumulator::sub_shifted_limbs). The shift comes first
//! and multiplies the streamed value by that power of two; use zero for an
//! unscaled value. The analogous accumulator methods avoid constructing a
//! shifted temporary.
//!
//! [`cmp_zero`](Accumulator::cmp_zero) returns the exact comparison. It takes `&mut self`
//! because the query compacts cancellation so later queries do not repeat the
//! work. [`is_known_zero`](Accumulator::is_known_zero) is a cheaper one-sided
//! check: `true` proves zero; `false` means the stored form is inconclusive.
//!
//! [`signed_magnitude`](Accumulator::signed_magnitude) returns normalized limbs
//! and may allocate in proportion to the accumulator's working width. The scaled form returns
//! a known power-of-two factor separately, which can avoid materializing an
//! unused low zero prefix. Primitive `TryFrom` conversions are exact, and
//! return the original accumulator on failure so its storage can still be
//! reused.
//!
//! To compare two totals, subtract one from a clone of the other and compare
//! the difference with zero. When their scales differ greatly,
//! [`cmp_zero_stable_under`](Accumulator::cmp_zero_stable_under) can avoid reading the
//! smaller total: `Some(ordering)` means every adjustment within the supplied
//! bit bound is too small to change that comparison; `None` means the caller
//! must perform the subtraction.
//!
//! # Costs and storage
//!
//! Cancellation can shorten the mathematical result while the accumulator
//! still retains contributions at higher bit positions. The costs below
//! therefore use *working width*, reported by
//! [`stored_bits`](Accumulator::stored_bits), rather than the result's bit
//! length. Let `W` be the greatest working width reached by the receiver, `A`
//! the operand's working width, `L` the number of input limbs, and `G` the
//! increase in the receiver's working width. A large shift can make `G` large
//! even when the input is one limb.
//!
//! Amortized bounds apply to the whole operation sequence: one call may finish
//! carry or cancellation work prepared by earlier calls, but the sequence does
//! not pay for that work repeatedly. Additional space excludes inputs and
//! returned output. Growing an allocation can briefly keep both allocations
//! alive.
//!
//! | Operation | Time | Additional space |
//! |---|---|---|
//! | `+=` / `-=` a primitive | Amortized O(log(`W` + 1)) | O(1) retained |
//! | Add or subtract `L` limbs | Amortized O(`L` log(`W` + 1) + `G`) | O(`L` + `G`) retained |
//! | Add or subtract an accumulator | Amortized O(`A` log(`W` + 1) + `G`) | O(`A` + `G`) retained |
//! | Exact or bounded comparison with zero | Amortized O(log(`W` + 1)) | O(1) retained |
//! | Working-width and known-zero queries | O(1) | O(1) |
//! | Unary `-` and [`reset`](Accumulator::reset) | O(`W`) | O(1) |
//! | [`normalize`](Accumulator::normalize), producing working width `Q` | O(`W` + `Q` log(`Q` + 1)) | O(1) scratch; existing O(`W`) capacity retained for reuse |
//! | Primitive `TryFrom` | O(`W`) | O(`W`) temporary |
//!
//! Any operation that grows the receiver may need O(`W` + `G`) temporary
//! space while replacing its allocation. This is allocator work, not an
//! additional copy made by the arithmetic. The retained bounds in the table
//! describe what remains after the operation.
//! Normalization can remove arbitrarily much cancelled width, but can extend
//! the working width by at most one 32-bit position: `Q <= W + 32`. It rewrites
//! the existing allocation rather than constructing a normalized copy, and
//! deliberately retains that O(`W`) capacity for later updates. Its rebuilt
//! skip metadata occupies O(`Q`) space within that retained bound. Only an
//! extension beyond the existing capacity can temporarily keep both the old
//! and replacement buffers alive.
//!
//! An unscaled magnitude read producing `Q` limbs takes O(`W` + `Q`) time,
//! O(`W`) temporary space, and O(`Q`) output space. A scaled read can omit a
//! low range known to be zero; for that operation, replace `W` with the number
//! of bits between the lowest relevant update and the top of the working range.
//!
//! The accumulator may retain more allocation than its current working width
//! after cancellation or reset. A nonzero left shift can replace that
//! allocation; the old and new allocations may coexist while the shift runs.
//!
//! [`reserve_bits`](Accumulator::reserve_bits) can avoid repeated allocation
//! growth. [`reset`](Accumulator::reset) clears the value while retaining its
//! allocation for reuse. Cloning and debug formatting take time and space
//! proportional to all retained allocation, not merely the current value.
//!
//! # Scope
//!
//! This is an accumulator, not a general-purpose integer type: it provides no
//! general multiplication, division, or right shift. If the value fits a
//! fixed-width integer, that is usually the simpler choice. The benefit here
//! is predictable amortized work when updates change sign near carry
//! boundaries and comparisons with zero interleave with cancellation.
//!
//! `Accumulator` deliberately has no `PartialEq` or `Ord`: exact comparison may
//! mutate an operand to preserve the amortized bound. Subtract and compare the
//! difference with zero instead. The crate requires `std`.
//!
//! # Optional metering
//!
//! The only feature, `touch-meter`, exposes a process-global implementation
//! counter in the `touch_meter` module. It supports regression tests for the
//! arithmetic core; it does not measure allocation or all bookkeeping and
//! therefore does not by itself establish the bounds above. Counts are
//! deterministic for a fixed implementation and operation sequence, but
//! individual totals are not an API compatibility promise. Run measured
//! scenarios serially. With the feature disabled, counting compiles away.

#![forbid(unsafe_code)]

mod accumulator;
#[cfg(feature = "touch-meter")]
pub mod touch_meter;

pub use accumulator::Accumulator;
