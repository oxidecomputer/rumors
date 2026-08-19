//! The rank numerator's two-arm storage: the backend magnitude while the
//! backend can represent it, a raw limb vector past that.
//!
//! # Why two arms exist
//!
//! The big-integer backend deliberately caps a magnitude's buffer at
//! `usize::MAX / word-bits` words so its internal bit counts always fit a
//! `usize`. On 64-bit targets that ceiling (2⁶⁴ − 64 bits) sits
//! astronomically past allocatable memory and never binds. On a 32-bit
//! target it is 2³² − 32 bits — about 512 MiB of value in a 4 GiB address
//! space — and the rank wire door can honestly outgrow it: the fraction
//! form reaches a wider numerator from ~604 MB of input, and the integral
//! form from ~512 MiB, both loud backend panics rather than values without
//! this module. [`Num`] closes that gap: the [`Base`] arm keeps the
//! backend as the arithmetic engine of record everywhere it can represent
//! the value, and the [`Wide`] arm stores the sliver it cannot — bounded
//! only by memory — implementing exactly the operation set rank arithmetic
//! needs (byte assembly, bit reads, right shifts, ±1, MSB-window
//! comparison, and limb streaming into the accumulator).
//!
//! # Canonical arm dispatch
//!
//! Every constructor normalizes through the one ceiling
//! ([`arm_ceiling_bits`]): a value is stored [`Wide`] **iff** its bit width
//! exceeds the ceiling. Canonical dispatch is what lets `Rank` keep its
//! derived structural equality and hashing as value equality — equal
//! values are always the same arm — and it makes the arm choice pure
//! routing: both arms denote the same integers exactly, so a misplaced
//! ceiling could misroute cost, never value. The production ceiling is the
//! backend capacity itself, held to the real backend by the wasm32
//! boundary pins (the below/at-capacity decode pins fill the backend's
//! last word on the [`Base`] arm; the past-capacity pins decode on the
//! [`Wide`] arm); tests may lower it (the test-only `ceiling` module) so
//! every public door drives both arms and the seam between them at
//! host-friendly sizes.
//!
//! # Metering
//!
//! Wide-arm work records into the same limb meter as [`Base`]'s own
//! operations, under the same denomination: 64-bit limbs of operand (and,
//! for materializations, result) value width, independent of which arm
//! stores the value. Base-arm operations delegate to [`Base`]'s already
//! metered methods, so a value below the ceiling meters exactly as it did
//! when [`Base`] was the numerator's only storage. Work routed through the
//! accumulator (`Rank`'s wide-path addition and subtraction) is priced by
//! suanpan's digit-touch meter; the limb meter records those operations'
//! materializations (operands streamed in, the result read out), not the
//! digit engine's internals.

use core::cmp::Ordering;

use dashu_int::UBig;
use suanpan::{Accumulator, Limbs};

use crate::codec::base::{msb_cmp_windows, MsbWindows};
use crate::codec::Base;

/// Record `limbs` 64-bit limbs of wide-arm work into the limb meter.
///
/// Compiles to nothing without the `limb-meter` feature, so wide-arm
/// operations call it unconditionally — the same shape as the shims in
/// `codec::base`.
#[inline(always)]
fn meter_wide(limbs: u64) {
    #[cfg(feature = "limb-meter")]
    crate::codec::base::limb_meter::record(limbs);
    #[cfg(not(feature = "limb-meter"))]
    let _ = limbs;
}

/// The backend's magnitude capacity in bits: the widest value a [`Base`]
/// can hold on this target.
///
/// Derived from the backend's own buffer cap of `usize::MAX / word-bits`
/// words (each word `Word::BITS` bits): 2³² − 32 on 32-bit targets,
/// 2⁶⁴ − 64 on 64-bit ones. The constant is a *routing* bound, not a
/// correctness bound — both arms compute exact values, so only its upper
/// side is load-bearing (a ceiling above the true capacity would let the
/// [`Base`] arm hand the backend a value it panics on), and that side is
/// held to the real backend where it is reachable: the wasm32 boundary
/// pins decode at exactly this width on the [`Base`] arm and one fraction
/// group past it on the [`Wide`] arm.
pub(crate) const BACKEND_CAPACITY_BITS: u64 =
    (usize::MAX / dashu_int::Word::BITS as usize) as u64 * dashu_int::Word::BITS as u64;

/// The arm ceiling in force: values at most this many bits wide store as
/// [`Base`], wider ones as [`Wide`].
///
/// In production this is [`BACKEND_CAPACITY_BITS`]; under test an
/// override (the test-only `ceiling` module) may lower it so host-scale
/// inputs drive the wide arm through the public doors.
#[inline]
pub(crate) fn arm_ceiling_bits() -> u64 {
    #[cfg(test)]
    if let Some(bits) = ceiling::override_bits() {
        return bits;
    }
    BACKEND_CAPACITY_BITS
}

/// The test-only arm-ceiling override: a scoped, thread-local lowering
/// of [`arm_ceiling_bits`].
///
/// The lowering makes the wide arm — honestly reachable only past ~2³²
/// bits on a 32-bit target — drivable through the public doors at
/// host-friendly sizes.
///
/// The override changes routing only, never values: both arms are exact,
/// so every suite that runs under a lowered ceiling checks the same
/// value-level contracts production serves. Values built under one ceiling
/// must not outlive its guard — canonical arm dispatch is relative to the
/// ceiling in force.
#[cfg(test)]
pub(crate) mod ceiling {
    use std::cell::Cell;

    thread_local! {
        static OVERRIDE: Cell<Option<u64>> = const { Cell::new(None) };
    }

    /// The override in force on this thread, if any.
    pub(crate) fn override_bits() -> Option<u64> {
        OVERRIDE.with(Cell::get)
    }

    /// Lower the arm ceiling to `bits` until the guard drops.
    pub(crate) fn force(bits: u64) -> Guard {
        Guard(OVERRIDE.with(|cell| cell.replace(Some(bits))))
    }

    /// Restores the previous ceiling on drop, so forced scopes nest.
    pub(crate) struct Guard(Option<u64>);

    impl Drop for Guard {
        fn drop(&mut self) {
            OVERRIDE.with(|cell| cell.set(self.0));
        }
    }
}

/// A rank numerator: the backend magnitude while the backend can hold it,
/// a raw limb vector past that.
///
/// Canonical by construction (the module doc's dispatch invariant), so the
/// derived equality and hash are value equality and every consumer may
/// match on the arm as a width fact.
#[derive(Clone, Debug)]
pub(crate) enum Num {
    /// At most [`arm_ceiling_bits`] bits: the backend arm, whose
    /// operations are [`Base`]'s own metered methods.
    Base(Base),
    /// Strictly more than [`arm_ceiling_bits`] bits: the limb arm.
    Wide(Wide),
}

/// A magnitude wider than the backend arm's ceiling, as little-endian
/// 64-bit limbs.
///
/// Invariants: the top limb is nonzero (minimal spelling — what makes the
/// derived equality value equality), and the bit width exceeds the arm
/// ceiling in force (canonical dispatch; [`Num`]'s constructors enforce
/// it). Never zero.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Wide {
    limbs: Vec<u64>,
}

impl Num {
    pub(crate) const ZERO: Num = Num::Base(Base::ZERO);

    /// The magnitude's bit length: zero for zero, `floor(log2 n) + 1`
    /// otherwise. O(1), unmetered — a stored-width read, like
    /// [`Base::bits`].
    pub(crate) fn bits(&self) -> u64 {
        match self {
            Num::Base(base) => base.bits(),
            Num::Wide(wide) => wide.bits(),
        }
    }

    /// Bit `i` of the magnitude (bit 0 the least significant): O(1),
    /// unmetered. A position past the width reads zero.
    pub(crate) fn bit(&self, i: u64) -> bool {
        match self {
            Num::Base(base) => base.bit(i),
            Num::Wide(wide) => wide.bit(i),
        }
    }

    /// The number of trailing zero bits, or [`None`] for zero: the
    /// normalization read. Width-scale (a bottom-up limb scan), metered
    /// on the operand's width in both arms.
    pub(crate) fn trailing_zeros(&self) -> Option<u64> {
        match self {
            Num::Base(base) => base.trailing_zeros(),
            Num::Wide(wide) => Some(wide.trailing_zeros()),
        }
    }

    /// The magnitude shifted right by `n`, re-dispatched onto the
    /// canonical arm.
    ///
    /// A wide value whose shift lands at or below the ceiling comes back
    /// as [`Base`]. Width-scale, metered on the operand in both arms;
    /// total — a shift at or past the width yields zero.
    pub(crate) fn shr(self, n: u64) -> Num {
        match self {
            // The base arm can only shrink, so it stays canonical with no
            // re-dispatch — including the shift-by-zero spelling, which
            // keeps this arm's cost and metering exactly the historical
            // numerator path's.
            Num::Base(base) => Num::Base(base >> n),
            Num::Wide(wide) => {
                if n == 0 {
                    meter_wide(wide.limb_count());
                    return Num::Wide(wide);
                }
                let shifted = wide.shr_limbs(n);
                Num::from_limbs(shifted)
            }
        }
    }

    /// The magnitude plus one (the encoder's `⌊r⌋ + 1` bias), re-dispatched
    /// onto the canonical arm: a carry out of the base arm's last
    /// representable bit crosses to wide.
    ///
    /// Base-arm values strictly below the ceiling take the backend's own
    /// metered `+ 1`; a base value exactly at the ceiling (where the
    /// backend could not hold a carried result) and every wide value go
    /// through the limb spelling, metered on the operand's width.
    pub(crate) fn plus_one(self) -> Num {
        match self {
            Num::Base(base) if base.bits() < arm_ceiling_bits() => Num::Base(base + 1u32),
            Num::Base(base) => {
                // At the ceiling exactly: the backend may not survive the
                // carry, so the increment runs in limb space and
                // re-dispatches (an all-ones value grows one bit, past the
                // ceiling; anything else stays base).
                meter_wide(base.bits().div_ceil(64).max(1));
                Num::from_limbs(increment(Limbs::new(&base.0).collect()))
            }
            Num::Wide(wide) => {
                meter_wide(wide.limb_count());
                Num::from_limbs(increment(wide.limbs))
            }
        }
    }

    /// The magnitude minus one (the decoder's bias removal), re-dispatched
    /// onto the canonical arm: a wide power of two can shrink back to the
    /// base arm.
    ///
    /// The caller guarantees the value is at least one (the decoder's
    /// biased mantissa always is).
    pub(crate) fn minus_one(self) -> Num {
        match self {
            Num::Base(base) => Num::Base(base - &Base::from(1u8)),
            Num::Wide(mut wide) => {
                meter_wide(wide.limb_count());
                for limb in wide.limbs.iter_mut() {
                    let (next, borrowed) = limb.overflowing_sub(1);
                    *limb = next;
                    if !borrowed {
                        break;
                    }
                    // A borrow rewrites the limb to all-ones and keeps
                    // borrowing upward; the invariant value ≥ 1 (indeed,
                    // wide values exceed the ceiling) means the borrow
                    // always terminates before running off the top.
                }
                Num::from_limbs(wide.limbs)
            }
        }
    }

    /// Order two numerators as MSB-aligned bit strings (the class-tie
    /// comparison behind `Rank`'s [`Ord`]): the order of `a · 2^x` versus
    /// `b · 2^y` whenever the two share a magnitude class.
    ///
    /// Same-arm base pairs take [`Base::msb_cmp`] — the historical path,
    /// metering included; every other pairing streams both arms' windows
    /// through the same shared kernel, so the tail rule and the per-window
    /// metering are one implementation across arms.
    pub(crate) fn msb_cmp(a: &Num, b: &Num) -> Ordering {
        match (a, b) {
            (Num::Base(x), Num::Base(y)) => Base::msb_cmp(x, y),
            (Num::Base(x), Num::Wide(y)) => msb_cmp_windows(x.msb_windows(), y.msb_windows()),
            (Num::Wide(x), Num::Base(y)) => msb_cmp_windows(x.msb_windows(), y.msb_windows()),
            (Num::Wide(x), Num::Wide(y)) => msb_cmp_windows(x.msb_windows(), y.msb_windows()),
        }
    }

    /// The magnitude's minimal big-endian bytes: empty for zero, no
    /// leading zero byte otherwise. Width-scale, metered on the operand in
    /// both arms (the decoder's image-assembly read).
    pub(crate) fn to_be_bytes(&self) -> Vec<u8> {
        match self {
            Num::Base(base) => base.to_be_bytes(),
            Num::Wide(wide) => {
                meter_wide(wide.limb_count());
                let mut bytes: Vec<u8> = Vec::with_capacity(wide.limbs.len() * 8);
                for limb in wide.limbs.iter().rev() {
                    bytes.extend_from_slice(&limb.to_be_bytes());
                }
                let lead = bytes.iter().take_while(|&&byte| byte == 0).count();
                bytes.drain(..lead);
                bytes
            }
        }
    }

    /// Materialize `BE(bytes) >> pad` onto the canonical arm.
    ///
    /// The decoder's one value-materialization point: `bytes` carry no
    /// leading zero byte (the caller strips them — the backend sizes
    /// buffers from the image's byte count, and zeros would pay capacity
    /// for value they don't carry), and `pad < 8` is the sub-byte
    /// alignment shift. Below the ceiling this is exactly the historical
    /// spelling ([`Base::from_be_bytes`] then the metered sub-byte shift);
    /// above it the limbs are assembled directly — no backend value ever
    /// exists — with the materialization metered on the value's width, the
    /// wide-decode convention: a meter that missed it would let a decoder
    /// build arbitrarily wide values while recording nothing.
    pub(crate) fn materialize_be(bytes: &[u8], pad: u32) -> Num {
        debug_assert!(pad < 8, "pad is the sub-byte alignment");
        debug_assert!(
            bytes.first() != Some(&0),
            "the caller strips leading zero bytes"
        );
        let bits = (bytes.len() as u64 * 8)
            .saturating_sub(u64::from(
                bytes.first().map_or(8, |byte| byte.leading_zeros()),
            ))
            .saturating_sub(u64::from(pad));
        if bits <= arm_ceiling_bits() {
            return Num::Base(Base::from_be_bytes(bytes) >> pad);
        }
        meter_wide(bits.div_ceil(64));
        // LE limbs from the BE image: 8-byte chunks off the tail, the
        // partial head chunk last, then the sub-byte shift in place.
        let mut limbs: Vec<u64> = Vec::with_capacity(bytes.len().div_ceil(8));
        let mut chunks = bytes.rchunks_exact(8);
        for chunk in chunks.by_ref() {
            limbs.push(u64::from_be_bytes(
                chunk.try_into().expect("an exact chunk"),
            ));
        }
        let head = chunks.remainder();
        if !head.is_empty() {
            let mut top = [0u8; 8];
            top[8 - head.len()..].copy_from_slice(head);
            limbs.push(u64::from_be_bytes(top));
        }
        if pad > 0 {
            for i in 0..limbs.len() {
                let high = limbs.get(i + 1).copied().unwrap_or(0);
                limbs[i] = (limbs[i] >> pad) | (high << (64 - pad));
            }
        }
        Num::from_limbs(limbs)
    }

    /// Dispatch little-endian limbs (the accumulator readout's spelling)
    /// onto the canonical arm. High zero limbs are stripped; empty (or
    /// all-zero) limbs are zero.
    ///
    /// The base arm materializes through the backend's byte constructor
    /// unmetered — the readout that produced the limbs already carries the
    /// pass's cost — while a wide materialization records the value's
    /// width, the same convention as [`Num::materialize_be`].
    pub(crate) fn from_limbs(mut limbs: Vec<u64>) -> Num {
        while limbs.last() == Some(&0) {
            limbs.pop();
        }
        let wide = Wide { limbs };
        if wide.bits() <= arm_ceiling_bits() {
            // Exact-capacity byte image, and the limb vector dropped
            // before the backend materializes: at the seam's widest
            // crossings (a borrow falling back from one bit past a 32-bit
            // target's capacity) the value is ~512 MiB, so an amortized
            // growth double or one extra live copy is the difference
            // between fitting the address space and an honest exhaustion.
            let mut bytes: Vec<u8> = Vec::with_capacity(wide.limbs.len() * 8);
            for limb in &wide.limbs {
                bytes.extend_from_slice(&limb.to_le_bytes());
            }
            drop(wide);
            // The top limb's padding must go: the backend sizes its buffer
            // from the image's byte count, so at the seam's widest
            // crossings the high zero bytes alone would push the word
            // count past its capacity while the value fits exactly.
            while bytes.last() == Some(&0) {
                bytes.pop();
            }
            return Num::Base(Base::from(UBig::from_le_bytes(&bytes)));
        }
        meter_wide(wide.limb_count());
        Num::Wide(wide)
    }

    /// Re-dispatch a backend-held value onto the canonical arm.
    ///
    /// In production the base arm's ceiling is the backend's own capacity,
    /// so the conversion never fires — a backend value above the backend's
    /// capacity cannot exist — and every historical `Base` numerator passes
    /// through unchanged, unmetered. Under a lowered test ceiling this is
    /// where fold outputs and raw constructions cross into the wide arm.
    pub(crate) fn from_base(base: Base) -> Num {
        if base.bits() <= arm_ceiling_bits() {
            return Num::Base(base);
        }
        meter_wide(base.bits().div_ceil(64));
        Num::Wide(Wide {
            limbs: Limbs::new(&base.0).collect(),
        })
    }

    /// Fold `±self · 2^shift` into an accumulator, at the width this arm
    /// stores: the base arm through the magnitude dispatch, the wide arm
    /// through the streaming limb entry.
    ///
    /// The accumulator's digit-touch meter prices the fold; the limb
    /// meter records the operand's width here, so limb-denominated
    /// envelopes see the operand materialization whichever arm streams it.
    pub(crate) fn fold_into(&self, acc: &mut Accumulator, shift: u64, subtract: bool) {
        match self {
            Num::Base(base) => {
                if subtract {
                    acc.sub_magnitude_shl(base, shift);
                } else {
                    acc.add_magnitude_shl(base, shift);
                }
            }
            Num::Wide(wide) => {
                meter_wide(wide.limb_count());
                if subtract {
                    acc.sub_limbs_shl(wide.limbs.iter().copied(), shift);
                } else {
                    acc.add_limbs_shl(wide.limbs.iter().copied(), shift);
                }
            }
        }
    }

    /// The magnitude's minimal little-endian bytes, for the test oracles'
    /// backend-independent reconstruction.
    #[cfg(test)]
    pub(crate) fn to_bytes_le(&self) -> Vec<u8> {
        match self {
            Num::Base(base) => base.to_bytes_le(),
            Num::Wide(wide) => {
                let mut bytes: Vec<u8> = wide
                    .limbs
                    .iter()
                    .flat_map(|limb| limb.to_le_bytes())
                    .collect();
                while bytes.last() == Some(&0) {
                    bytes.pop();
                }
                bytes
            }
        }
    }

    /// Whether this numerator is stored on the wide arm, for the
    /// canonicity assertions.
    #[cfg(test)]
    pub(crate) fn is_wide(&self) -> bool {
        matches!(self, Num::Wide(_))
    }
}

// Manual equality so the base arm keeps [`Base`]'s metered comparison and
// the wide arm records the same two-operand convention; canonical arm
// dispatch makes cross-arm values unequal by construction, recorded at the
// same widths for uniformity. Equality of equal values is structural in
// both arms, so the derived `Hash` on `Wide` and [`Base`]'s own metered
// `Hash` stay consistent with this.
impl PartialEq for Num {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Num::Base(a), Num::Base(b)) => a == b,
            (Num::Wide(a), Num::Wide(b)) => {
                meter_wide(a.limb_count() + b.limb_count());
                a == b
            }
            (a, b) => {
                meter_wide(a.bits().div_ceil(64).max(1) + b.bits().div_ceil(64).max(1));
                false
            }
        }
    }
}

impl Eq for Num {}

/// Renders the exact decimal value, honoring integer padding flags.
///
/// The base arm is the backend's own (subquadratic) conversion; the wide
/// arm is schoolbook long division by 10¹⁹ — quadratic in the value's
/// width, the honest price of exact decimal past the backend's reach, and
/// metered as one record of the operand's width per emitted 19-digit group
/// (the division pass that produced it).
impl core::fmt::Display for Num {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Num::Base(base) => core::fmt::Display::fmt(base, f),
            Num::Wide(wide) => f.pad_integral(true, "", &wide.to_decimal()),
        }
    }
}

impl core::hash::Hash for Num {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        match self {
            Num::Base(base) => base.hash(state),
            Num::Wide(wide) => {
                meter_wide(wide.limb_count());
                wide.hash(state);
            }
        }
    }
}

impl Wide {
    /// The bit width: `64·(limbs − 1) + width(top limb)`. O(1) — the top
    /// limb is nonzero by invariant.
    fn bits(&self) -> u64 {
        match self.limbs.last() {
            None => 0,
            Some(top) => {
                debug_assert_ne!(*top, 0, "the top limb is nonzero");
                self.limbs.len() as u64 * 64 - u64::from(top.leading_zeros())
            }
        }
    }

    /// The stored limb count, the meter denomination.
    fn limb_count(&self) -> u64 {
        self.limbs.len() as u64
    }

    /// Bit `i`, zero past the width.
    fn bit(&self, i: u64) -> bool {
        match usize::try_from(i / 64) {
            Ok(index) => self
                .limbs
                .get(index)
                .is_some_and(|limb| limb >> (i % 64) & 1 == 1),
            // A bit index past `usize` can only address zeros: the stored
            // limbs are `usize`-indexed.
            Err(_) => false,
        }
    }

    /// The trailing zero count: a bottom-up limb scan, metered on the
    /// operand's width. Never `None` — a wide value is nonzero.
    fn trailing_zeros(&self) -> u64 {
        meter_wide(self.limb_count());
        for (index, limb) in self.limbs.iter().enumerate() {
            if *limb != 0 {
                return index as u64 * 64 + u64::from(limb.trailing_zeros());
            }
        }
        unreachable!("a wide value is nonzero by invariant");
    }

    /// The limbs of `self >> n`, minimal at the top: the shift's
    /// width-scale work, metered on the operand.
    fn shr_limbs(&self, n: u64) -> Vec<u64> {
        meter_wide(self.limb_count());
        let Ok(whole) = usize::try_from(n / 64) else {
            // A shift amount past `usize` limbs exceeds the stored width
            // (which is `usize`-indexed): the result is zero.
            return Vec::new();
        };
        if whole >= self.limbs.len() {
            return Vec::new();
        }
        let bit = (n % 64) as u32;
        let mut limbs: Vec<u64> = Vec::with_capacity(self.limbs.len() - whole);
        for i in whole..self.limbs.len() {
            let low = self.limbs[i] >> bit;
            let high = if bit == 0 {
                0
            } else {
                self.limbs.get(i + 1).copied().unwrap_or(0) << (64 - bit)
            };
            limbs.push(low | high);
        }
        limbs
    }

    /// The MSB-first 64-bit windows of the bit string (the comparison
    /// stream).
    fn msb_windows(&self) -> MsbWindows<impl Iterator<Item = u64> + '_> {
        MsbWindows::new(self.limbs.iter().rev().copied(), self.bits())
    }

    /// The exact decimal rendering: schoolbook long division by 10¹⁹,
    /// quadratic in the width, one operand-width meter record per pass.
    fn to_decimal(&self) -> String {
        /// The largest power of ten in a limb: each division pass peels
        /// 19 decimal digits.
        const TEN_POW_19: u128 = 10_000_000_000_000_000_000;
        let mut current = self.limbs.clone();
        // 19-digit groups, least significant first.
        let mut groups: Vec<u64> = Vec::new();
        while !current.is_empty() {
            meter_wide(current.len() as u64);
            let mut remainder: u128 = 0;
            for limb in current.iter_mut().rev() {
                let carried = (remainder << 64) | u128::from(*limb);
                *limb = (carried / TEN_POW_19) as u64;
                remainder = carried % TEN_POW_19;
            }
            while current.last() == Some(&0) {
                current.pop();
            }
            groups.push(remainder as u64);
        }
        let mut rendered = String::new();
        for (index, group) in groups.iter().enumerate().rev() {
            if index == groups.len() - 1 {
                rendered.push_str(&group.to_string());
            } else {
                rendered.push_str(&format!("{group:019}"));
            }
        }
        debug_assert!(!rendered.is_empty(), "a wide value is nonzero");
        rendered
    }
}

/// Little-endian limbs plus one, growing by a limb on a full carry.
fn increment(mut limbs: Vec<u64>) -> Vec<u64> {
    for limb in limbs.iter_mut() {
        let (next, carried) = limb.overflowing_add(1);
        *limb = next;
        if !carried {
            return limbs;
        }
    }
    limbs.push(1);
    limbs
}

#[cfg(test)]
mod tests;
