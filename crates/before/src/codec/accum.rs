//! A cliff-immune signed accumulator: redundant balanced base-2^32 digits.
//!
//! [`Accum`] holds the running signed integer maintained by a sweep — a
//! running leaf value, a running difference of two leaf values, a running
//! path sum — under two cost guarantees no normalized big-integer
//! representation offers together: amortized O(1) digit work per
//! machine-word delta and O(operand limbs) per wide delta, on *every* input
//! sequence. The value is `Σ dᵢ · 2^(32·i)` over signed digits `dᵢ: i64`,
//! each kept in the *lazy zone* `|dᵢ| < 2^33`. A write that pushes a digit
//! out of the zone carries `c = (t + 2^31) >> 32` upward and recenters the
//! digit's remainder into `[−2^31, 2^31)`, so a freshly recentered digit
//! needs at least `2^33 − 2^31` of further net drift before it can carry
//! again: every carry is funded by the deltas that drove the digit out of
//! its zone. Because *every* write recenters, the representation has no
//! normalized region anywhere — hence no boundary an adversarial delta
//! stream can oscillate across at less than the cost the stream itself
//! paid, at any delta width. (Any two-zone form — a normalized prefix plus
//! a fixed-width lazy window — has such a boundary, and a stream of deltas
//! one code wider than the window forces the normalized prefix through a
//! full carry per delta.)
//!
//! # Why the top digits decide the sign: the `|s| ≥ 3` domination bound
//!
//! [`Accum::sign`] folds digits from the top: at digit index `i` the
//! running partial `s = Σ_{j=i..=top} dⱼ · 2^(32·(j−i))` is the scanned
//! suffix's exact value in units of `2^(32·i)`, while the unscanned digits
//! below `i` contribute at most
//! `Σ_{j<i} (2^33 − 1) · 2^(32·j) < 2.01 · 2^(32·i)` in magnitude
//! (a geometric series: each digit is under `2^33`, and each level down is
//! worth `2^32` times less). So once `|s| ≥ 3`, the suffix dominates
//! everything below it — `3 > 2.01` — and `sign(value) = sign(s)`. While
//! `|s| < 3` the fold must descend, but `s` stays under `3 · 2^32 + 2^33`
//! at every step, so the partial never itself needs wide arithmetic; if the
//! fold reaches digit 0 the partial *is* the value, exactly.
//!
//! # Why reads mutate: the collapse/write amortization
//!
//! A cancelling prefix — high digits summing to a tiny net value, as built
//! by `+2^k` followed by `−(2^k − 1)` — forces the sign fold below the top
//! digit. The fold therefore *collapses* what it scanned: the scanned
//! digits are zeroed and their exact partial is re-deposited at the scan's
//! floor, so the next sign check re-reads none of them. A digit is scanned
//! at most once per write that made it nonzero, so sign checks amortize
//! against the writes that built the prefix, and [`Accum::sign`] is
//! amortized O(1) regardless of how sign checks and writes interleave.
//! This is why the sign queries take `&mut self`: they may rewrite the
//! representation. The mutation is always value-preserving — the digits
//! change, the integer they denote never does.
//!
//! # Testing
//!
//! Differential proptests drive mixed small/wide streams against an exact
//! `BigInt` oracle with the sign compared after every operation and the
//! full value snapshotted periodically; deterministic adversarial streams
//! (boundary-comb oscillation, wide teeth across a high carry cliff,
//! cancelling-prefix chains) pin the shapes the representation exists to
//! survive. The `limb-meter` feature counts every digit read-modify-write
//! into [`touch_meter`], and the resource-envelope suite pins the per-delta
//! touch cost flat across size doublings on those same streams.

use core::cmp::Ordering;

use num_bigint::BigUint;

use super::Base;

/// Process-global counter of accumulator digit touches.
///
/// Counts one per digit read-modify-write in [`Accum`]'s own code (plus one
/// per operand limb read by a wide operation), the same unit the probe tier
/// priced the representation in. Digit-touch cost is invisible to heap and
/// node meters — the work is wider, not more frequent — so this counter is
/// what the accumulator's flat-per-delta envelopes pin. Process-global,
/// relaxed ordering: the metering binaries run one scenario per process and
/// read the counter only after the metered call returns.
#[cfg(feature = "limb-meter")]
pub mod touch_meter {
    use core::sync::atomic::{AtomicU64, Ordering};

    static TOUCHES: AtomicU64 = AtomicU64::new(0);

    /// Add `n` digit touches to the counter.
    pub(super) fn record(n: u64) {
        TOUCHES.fetch_add(n, Ordering::Relaxed);
    }

    /// The digit touches recorded since the last [`reset`].
    pub fn touches() -> u64 {
        TOUCHES.load(Ordering::Relaxed)
    }

    /// Reset the counter to zero.
    pub fn reset() {
        TOUCHES.store(0, Ordering::Relaxed);
    }
}

/// Record `n` accumulator digit touches.
///
/// Compiles to nothing without the `limb-meter` feature, so the hot paths
/// call it unconditionally.
#[inline(always)]
fn touch(n: u64) {
    #[cfg(feature = "limb-meter")]
    touch_meter::record(n);
    #[cfg(not(feature = "limb-meter"))]
    let _ = n;
}

/// Bits per digit: the digit base is `2^32`.
const DIGIT_BITS: u32 = 32;

/// Mask selecting one digit's worth of a 64-bit limb.
const DIGIT_MASK: u64 = (1 << DIGIT_BITS) - 1;

/// The lazy-zone bound: every stored digit satisfies `|d| < LAZY_LIMIT`.
///
/// Twice the digit base: a digit recentered into `[−2^31, 2^31)` must absorb
/// at least `2^33 − 2^31` of net drift before its next carry, which is what
/// makes carries amortized O(1) per small delta.
const LAZY_LIMIT: i128 = 1 << (DIGIT_BITS + 1);

/// The recentering bias: carrying `c = (t + 2^31) >> 32` leaves the
/// remainder `t − c·2^32` in `[−2^31, 2^31)`.
const RECENTER_BIAS: i128 = 1 << (DIGIT_BITS - 1);

/// The sign fold's decision threshold on the running partial.
///
/// The digits below the scanned suffix contribute under `2.01 · 2^(32·i)`
/// in magnitude (the module doc's domination bound), so a partial of
/// magnitude 3 or more cannot be overturned from below.
const SIGN_DECIDED: i128 = 3;

/// A running signed integer over redundant balanced base-2^32 digits.
///
/// Deltas are added or subtracted at machine-word or arbitrary width; the
/// sign is readable at any point in amortized O(1); one low-to-high carry
/// pass converts the final value to a normalized magnitude. The module doc
/// carries the representation and both cost arguments. Sign queries take
/// `&mut self` because they may collapse a scanned cancelling prefix; the
/// rewrite never changes the value the digits denote.
#[derive(Debug, Clone)]
pub struct Accum {
    /// Little-endian signed digits: `value = Σ digits[i] · 2^(32·i)`, every
    /// digit in the lazy zone `|d| < 2^33`.
    digits: Vec<i64>,
    /// Index of the highest nonzero digit; 0 when the value is zero. Digits
    /// above it are all zero.
    top: usize,
}

impl Accum {
    /// Create an accumulator holding zero.
    pub fn new() -> Accum {
        Accum {
            digits: vec![0],
            top: 0,
        }
    }

    /// Add a signed machine-word delta.
    pub fn add_small(&mut self, delta: i64) {
        if delta != 0 {
            self.add_at(0, i128::from(delta));
        }
    }

    /// Subtract a signed machine-word delta.
    pub fn sub_small(&mut self, delta: i64) {
        if delta != 0 {
            self.add_at(0, -i128::from(delta));
        }
    }

    /// Add an unsigned machine-word delta.
    pub fn add_u64(&mut self, delta: u64) {
        if delta != 0 {
            self.add_at(0, i128::from(delta));
        }
    }

    /// Subtract an unsigned machine-word delta.
    pub fn sub_u64(&mut self, delta: u64) {
        if delta != 0 {
            self.add_at(0, -i128::from(delta));
        }
    }

    /// Add a wide delta: O(operand limbs), paid by the operand's own width.
    pub fn add_wide(&mut self, delta: &BigUint) {
        self.apply_limbs(delta.iter_u64_digits(), false);
    }

    /// Subtract a wide delta: O(operand limbs), paid by the operand's own
    /// width.
    pub fn sub_wide(&mut self, delta: &BigUint) {
        self.apply_limbs(delta.iter_u64_digits(), true);
    }

    /// Add a stored magnitude, at the width it is stored at.
    ///
    /// The stored-magnitude entry points carry the skyline sweep's leaf
    /// heights and deltas into the accumulator: a word-scale magnitude
    /// takes the small path, a spilled one the wide path.
    pub(crate) fn add_base(&mut self, delta: &Base) {
        match delta {
            Base::Small(n) => self.add_u64(*n),
            Base::Big(n) => self.add_wide(n),
        }
    }

    /// Subtract a stored magnitude, at the width it is stored at.
    pub(crate) fn sub_base(&mut self, delta: &Base) {
        match delta {
            Base::Small(n) => self.sub_u64(*n),
            Base::Big(n) => self.sub_wide(n),
        }
    }

    /// The sign of the held value, relative to zero: amortized O(1).
    ///
    /// Folds digits from the top and decides at running partial `|s| ≥ 3`
    /// (the module doc's domination bound). When the fold had to descend —
    /// a cancelling prefix — the scanned digits are collapsed to their
    /// partial at the scan's floor, so the scan is paid at most once per
    /// write (the module doc's amortization argument). The rewrite is
    /// value-preserving.
    pub fn sign(&mut self) -> Ordering {
        let mut index = self.top;
        let mut partial: i128 = 0;
        loop {
            touch(1);
            partial = (partial << DIGIT_BITS) + i128::from(self.digits[index]);
            if partial.abs() >= SIGN_DECIDED || index == 0 {
                break;
            }
            index -= 1;
        }
        if index < self.top {
            // Collapse: zero the scanned suffix and re-deposit its exact
            // partial at the scan floor, so no future sign fold re-reads it.
            for digit in &mut self.digits[index..=self.top] {
                *digit = 0;
                touch(1);
            }
            self.top = index;
            while self.top > 0 && self.digits[self.top] == 0 {
                self.top -= 1;
            }
            if partial != 0 {
                self.add_at(index, partial);
            }
        }
        partial.cmp(&0)
    }

    /// Whether the held value is strictly negative: amortized O(1).
    ///
    /// Takes `&mut self` for the same value-preserving collapse as
    /// [`Accum::sign`].
    pub fn is_negative(&mut self) -> bool {
        self.sign() == Ordering::Less
    }

    /// The held value as a sign and a normalized magnitude.
    ///
    /// One low-to-high pass with a signed carry: O(held digits). The
    /// magnitude is zero exactly when the sign is [`Ordering::Equal`], and
    /// converts onto the crate's stored-magnitude type through its
    /// `From<BigUint>` impl, which is where inline-range normalization
    /// happens.
    pub fn sign_magnitude(&self) -> (Ordering, BigUint) {
        // Low-to-high signed carry: after the pass, the collected unsigned
        // digits hold `M` with `value = carry · 2^(32·len) + M`,
        // `0 ≤ M < 2^(32·len)`.
        let mut collected: Vec<u32> = Vec::with_capacity(self.top + 2);
        let mut carry: i128 = 0;
        for &digit in &self.digits[..=self.top] {
            touch(1);
            let total = i128::from(digit) + carry;
            let low = total.rem_euclid(1 << DIGIT_BITS);
            collected.push(low as u32);
            carry = (total - low) >> DIGIT_BITS;
        }
        if carry < 0 {
            // Negative: |value| = |carry| · 2^(32·len) − M, which is
            // (|carry| − 1) high part plus the complement of M when M > 0,
            // and |carry| high part over untouched zeros when M = 0.
            let low_nonzero = collected.iter().any(|&d| d != 0);
            if low_nonzero {
                let mut complement_carry = 1u64;
                for digit in collected.iter_mut() {
                    touch(1);
                    let v = (DIGIT_MASK - u64::from(*digit)) + complement_carry;
                    *digit = (v & DIGIT_MASK) as u32;
                    complement_carry = v >> DIGIT_BITS;
                }
                debug_assert_eq!(
                    complement_carry, 0,
                    "complement of a nonzero low part cannot carry out"
                );
            }
            let mut high = (-carry) as u128 - u128::from(low_nonzero);
            while high > 0 {
                touch(1);
                collected.push((high & u128::from(DIGIT_MASK)) as u32);
                high >>= DIGIT_BITS;
            }
            // |carry| ≥ 1 makes |value| ≥ 2^(32·len) − M > 0: never zero.
            (Ordering::Less, BigUint::new(collected))
        } else {
            let mut high = carry as u128;
            while high > 0 {
                touch(1);
                collected.push((high & u128::from(DIGIT_MASK)) as u32);
                high >>= DIGIT_BITS;
            }
            let magnitude = BigUint::new(collected);
            let sign = if magnitude.bits() == 0 {
                Ordering::Equal
            } else {
                Ordering::Greater
            };
            (sign, magnitude)
        }
    }

    /// Add `value` (any sign, any magnitude up to a small multiple of
    /// `2^64`) into the digit at `pos`, carrying upward until every touched
    /// digit is back in the lazy zone.
    fn add_at(&mut self, mut pos: usize, mut value: i128) {
        while value != 0 {
            if pos >= self.digits.len() {
                self.digits.resize(pos + 1, 0);
            }
            touch(1);
            let total = i128::from(self.digits[pos]) + value;
            if total.abs() < LAZY_LIMIT {
                self.digits[pos] = total as i64;
                if total != 0 && pos > self.top {
                    self.top = pos;
                }
                value = 0;
            } else {
                let carry = (total + RECENTER_BIAS) >> DIGIT_BITS;
                let remainder = total - (carry << DIGIT_BITS);
                self.digits[pos] = remainder as i64;
                if remainder != 0 && pos > self.top {
                    self.top = pos;
                }
                value = carry;
                pos += 1;
            }
        }
        while self.top > 0 && self.digits[self.top] == 0 {
            self.top -= 1;
            touch(1);
        }
    }

    /// Apply a little-endian 64-bit limb stream, digit-aligned: each limb
    /// lands as two independent 32-bit contributions at its own position,
    /// so a wide operand costs O(its limbs) regardless of the held width.
    fn apply_limbs<I: Iterator<Item = u64>>(&mut self, limbs: I, negative: bool) {
        for (i, limb) in limbs.enumerate() {
            touch(1);
            let low = i128::from(limb & DIGIT_MASK);
            let high = i128::from(limb >> DIGIT_BITS);
            if low != 0 {
                self.add_at(2 * i, if negative { -low } else { low });
            }
            if high != 0 {
                self.add_at(2 * i + 1, if negative { -high } else { high });
            }
        }
    }
}

impl Default for Accum {
    fn default() -> Accum {
        Accum::new()
    }
}

#[cfg(test)]
mod tests;
