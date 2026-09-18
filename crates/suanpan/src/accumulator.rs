//! A signed accumulator over redundant balanced digits.
//!
//! Writes leave digits in a bounded lazy range, sign reads collapse the
//! portion they inspect, and a compact ledger skips unwritten zero runs. Every
//! digit read or write passes through [`touch`], allowing the optional meter to
//! measure the work directly.

use core::cmp::Ordering;
use std::collections::BTreeMap;

/// Record `count` accumulator digit touches.
///
/// Compiles to nothing without the `touch-meter` feature, so the hot paths
/// call it unconditionally.
#[inline(always)]
fn touch(count: u64) {
    #[cfg(feature = "touch-meter")]
    crate::touch_meter::record(count);
    #[cfg(not(feature = "touch-meter"))]
    let _ = count;
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
/// in magnitude (the crate docs' domination bound), so a partial of
/// magnitude 3 or more cannot be overturned from below.
const SIGN_DECIDED: i128 = 3;

/// The quick register's ceiling: a value of magnitude at most this stays
/// in the register, exact.
///
/// Three digits' worth. Wide enough that a running total or difference
/// over machine-word deltas essentially never leaves the register;
/// narrow enough that folding any machine-word delta — or another
/// register, or a register shifted by up to [`QUICK_SHIFT_MAX`] — into
/// it stays far from `i128` overflow, and a spill deposits a handful of
/// digits.
const QUICK_MAX: u128 = 1 << 96;

/// The widest in-register left shift: `QUICK_MAX << QUICK_SHIFT_MAX`
/// still sits comfortably inside `i128`.
const QUICK_SHIFT_MAX: u64 = 30;

/// A running signed integer over redundant balanced base-2^32 digits.
///
/// Deltas are added or subtracted as machine words or streams of 64-bit
/// limbs. The sign is readable at any point in amortized O(1), and one
/// low-to-high carry pass ([`sign_limbs`](Accumulator::sign_limbs)) returns the
/// normalized magnitude. Sign queries take `&mut self` because they may
/// collapse a scanned cancelling prefix; the rewrite never changes the value.
///
/// # Complexity
///
/// `Default` is `O(1)`. `Clone` and `Debug` are `O(b)`, where `b` is the digit
/// buffer's current width: the buffer grows to cover the highest
/// position written, so after a wide interlude collapses to a narrow
/// value a clone still pays the wide width. A
/// [`reset`](Accumulator::reset) keeps the buffer's capacity for the
/// next spill — the pooled-reuse contract — while
/// [`shl`](Accumulator::shl) on a digit-engine value rebuilds the held
/// value and may release the buffer.
#[derive(Debug, Clone)]
pub struct Accumulator {
    /// The quick register: `Some(v)` means the held value is exactly
    /// `v` and the digit engine below is idle (digits all zero, ledger
    /// empty).
    ///
    /// Every accumulator starts here and stays while the value and the
    /// operands fit ([`QUICK_MAX`]); the first wide operand or
    /// outgrown sum spills the register into the digits, once per
    /// [`reset`](Accumulator::reset) epoch — an exact small-integer
    /// mode with a one-way, O(1) exit. A delta stream cannot repeatedly cross
    /// this boundary because an accumulator spills at most once between
    /// resets.
    quick: Option<i128>,
    /// Little-endian signed digits: `value = Σ digits[i] · 2^(32·i)`, every
    /// digit in the lazy zone `|d| < 2^33`.
    digits: Vec<i64>,
    /// Index of the highest nonzero digit; 0 when the value is zero. Digits
    /// above it are all zero.
    top: usize,
    /// The lowest digit index any write has deposited at since the last
    /// [`reset`](Accumulator::reset) (or construction); [`usize::MAX`]
    /// when none has.
    ///
    /// Every digit below it is zero — the invariant that lets
    /// [`sign_limbs_shl`](Accumulator::sign_limbs_shl) skip the
    /// never-written prefix instead of scanning it. Conservative: a
    /// cancelling write may zero digits at or above it without raising
    /// it back. A collapsing sign read deposits through
    /// [`add_at`](Accumulator::add_at) too, and its re-deposit index can
    /// sit below every caller-written position — the fold may overshoot
    /// the lowest nonzero digit by one level — so sign queries also
    /// lower this watermark.
    bottom: usize,
    /// The zero-run ledger: certificates `lo → hi`, each stating that
    /// every digit strictly between `lo` and `hi` is zero.
    ///
    /// Three maintainers: a write landing above `top + 1` records the
    /// never-written run it jumps ([`add_at`](Accumulator::add_at)); a
    /// write whose carries land inside a certified run splits the
    /// certificate around the digits written
    /// ([`crop_runs`](Accumulator::crop_runs)); scans consume
    /// certificates to skip runs whole
    /// ([`consume_run_at`](Accumulator::consume_run_at)). Structural
    /// invariants beyond soundness: runs are pairwise disjoint, and
    /// every run lies at or below the settled `top` — the geometry
    /// behind [`crop_runs`](Accumulator::crop_runs)' descending early
    /// stop and the half-the-held-positions ledger cap. Containment
    /// is standing, collapse included: a sign fold carrying a nonzero
    /// partial into a certified run decides at the run's first
    /// interior digit (the partial shifts past the decision bound
    /// over a zero digit), so the collapse re-deposit starts there
    /// and its crop can keep only the run's lower remnant; a zero
    /// partial consumes the run before stepping in; and a fold that
    /// empties the value clears the ledger. Consumers nonetheless
    /// rely on soundness alone, and that is unconditional:
    /// certificates are split around every digit write (each goes
    /// through [`add_at`](Accumulator::add_at), which crops), and the
    /// only other digit rewrites set digits to zero, which can
    /// falsify no interior-zero claim. Every clause here is checked
    /// after every step of every schedule the exhaustive ledger
    /// driver explores (`ledger_invariants_hold_exhaustively`); the
    /// crate docs' zero-run ledger section carries the amortization
    /// argument the structure pays for.
    zero_runs: BTreeMap<usize, usize>,
}

impl Accumulator {
    /// Create an accumulator holding zero.
    ///
    /// # Complexity
    ///
    /// `O(1)`.
    pub fn new() -> Accumulator {
        Accumulator {
            quick: Some(0),
            digits: Vec::new(),
            top: 0,
            bottom: usize::MAX,
            zero_runs: BTreeMap::new(),
        }
    }

    /// Add a signed machine-word delta: amortized O(1).
    ///
    /// The signed (`i64`) twin of [`add_u64`](Accumulator::add_u64).
    /// Exact over the full `i64` range: the delta widens before any carry
    /// arithmetic, so even `i64::MIN` lands intact.
    ///
    /// # Complexity
    ///
    /// Amortized `O(1)` digit touches.
    #[inline]
    pub fn add_small(&mut self, delta: i64) {
        if delta != 0 {
            let delta = i128::from(delta);
            if !self.quick_add(delta) {
                self.add_at(0, delta);
            }
        }
    }

    /// Subtract a signed machine-word delta: amortized O(1).
    ///
    /// Exact over the full `i64` range, `i64::MIN` included (the delta
    /// widens before it is negated).
    ///
    /// # Complexity
    ///
    /// Amortized `O(1)` digit touches.
    #[inline]
    pub fn sub_small(&mut self, delta: i64) {
        if delta != 0 {
            let delta = -i128::from(delta);
            if !self.quick_add(delta) {
                self.add_at(0, delta);
            }
        }
    }

    /// Add an unsigned machine-word delta: amortized O(1).
    ///
    /// Use this over [`add_small`](Accumulator::add_small) when the delta
    /// may exceed `i64::MAX`; otherwise the two are interchangeable.
    ///
    /// # Complexity
    ///
    /// Amortized `O(1)` digit touches.
    #[inline]
    pub fn add_u64(&mut self, delta: u64) {
        if delta != 0 {
            let delta = i128::from(delta);
            if !self.quick_add(delta) {
                self.add_at(0, delta);
            }
        }
    }

    /// Subtract an unsigned machine-word delta: amortized O(1).
    ///
    /// Use this over [`sub_small`](Accumulator::sub_small) when the delta
    /// may exceed `i64::MAX`; otherwise the two are interchangeable.
    ///
    /// # Complexity
    ///
    /// Amortized `O(1)` digit touches.
    #[inline]
    pub fn sub_u64(&mut self, delta: u64) {
        if delta != 0 {
            let delta = -i128::from(delta);
            if !self.quick_add(delta) {
                self.add_at(0, delta);
            }
        }
    }

    /// Add a stream of little-endian 64-bit limbs times `2^shift`:
    /// amortized O(limbs yielded) digit touches, independent of the
    /// shift.
    ///
    /// Each limb is deposited directly at its shifted position; no normalized
    /// integer or shifted copy is materialized. High zero limbs are permitted
    /// and value-neutral, but each yielded limb costs one touch, so callers
    /// should stream the minimal form.
    ///
    /// # Complexity
    ///
    /// Amortized `O(limbs yielded)` digit touches, independent of the
    /// shift; the digit buffer grows to cover the shifted positions.
    ///
    /// # Panics
    ///
    /// Panics if a nonzero digit would land at or beyond `usize::MAX`, where
    /// the digit buffer would need the unrepresentable length `position + 1`.
    pub fn add_limbs_shl<I: IntoIterator<Item = u64>>(&mut self, limbs: I, shift: u64) {
        self.spill();
        self.apply_limbs(limbs.into_iter(), false, shift);
    }

    /// Subtract a stream of little-endian 64-bit limbs times `2^shift`:
    /// amortized O(limbs yielded) digit touches, independent of the
    /// shift.
    ///
    /// The subtractive twin of [`add_limbs_shl`](Accumulator::add_limbs_shl),
    /// with the same memory bound.
    ///
    /// # Complexity
    ///
    /// Amortized `O(limbs yielded)` digit touches, independent of the
    /// shift; the digit buffer grows to cover the shifted positions.
    ///
    /// # Panics
    ///
    /// Panics under the same condition as
    /// [`add_limbs_shl`](Accumulator::add_limbs_shl).
    pub fn sub_limbs_shl<I: IntoIterator<Item = u64>>(&mut self, limbs: I, shift: u64) {
        self.spill();
        self.apply_limbs(limbs.into_iter(), true, shift);
    }

    /// Add another accumulator's held value into this one: amortized
    /// O(the operand's held digits).
    ///
    /// The cost discipline to watch: folding a long-lived accumulator in
    /// from a loop re-reads all of its digits every iteration — O(n) per
    /// pass, quadratic over the loop. Fold an operand in once, when it is
    /// about to be discarded or has served its purpose.
    ///
    /// # Complexity
    ///
    /// Amortized `O(|other|)` digit touches, whatever the receiver's
    /// width.
    pub fn add_accum(&mut self, other: &Accumulator) {
        self.fold_accum(other, 0, false);
    }

    /// Subtract another accumulator's held value from this one: amortized
    /// O(the operand's held digits).
    ///
    /// The subtractive twin of [`add_accum`](Accumulator::add_accum),
    /// with the same once-not-per-iteration cost discipline.
    ///
    /// # Complexity
    ///
    /// Amortized `O(|other|)` digit touches, whatever the receiver's
    /// width.
    pub fn sub_accum(&mut self, other: &Accumulator) {
        self.fold_accum(other, 0, true);
    }

    /// Add another accumulator's held value times `2^shift` into this one:
    /// amortized O(the operand's held digits) digit touches, independent
    /// of the shift.
    ///
    /// The merge move of a weighted fold: a finished partial sum lands in
    /// its parent's accumulator at the exponent gap between their scales,
    /// each digit routed directly to the positions it spans. The digit
    /// buffer grows to cover the shifted positions (memory O(shift / 32)
    /// plus the operand's digits).
    ///
    /// # Complexity
    ///
    /// Amortized `O(|other|)` digit touches, independent of the shift;
    /// the digit buffer grows to cover the shifted positions.
    ///
    /// # Panics
    ///
    /// Panics under the same condition as
    /// [`add_limbs_shl`](Accumulator::add_limbs_shl).
    pub fn add_accum_shl(&mut self, other: &Accumulator, shift: u64) {
        self.fold_accum(other, shift, false);
    }

    /// Subtract another accumulator's held value times `2^shift` from
    /// this one: amortized O(the operand's held digits) digit touches,
    /// independent of the shift.
    ///
    /// The subtractive twin of
    /// [`add_accum_shl`](Accumulator::add_accum_shl): each operand digit
    /// lands negated at the shifted position(s) it spans (the zone is
    /// symmetric about zero, so a negated digit is still in it —
    /// subtraction needs no borrow machinery of its own).
    ///
    /// # Complexity
    ///
    /// Amortized `O(|other|)` digit touches, independent of the shift;
    /// the digit buffer grows to cover the shifted positions.
    ///
    /// # Panics
    ///
    /// Panics under the same condition as
    /// [`add_limbs_shl`](Accumulator::add_limbs_shl).
    pub fn sub_accum_shl(&mut self, other: &Accumulator, shift: u64) {
        self.fold_accum(other, shift, true);
    }

    /// Fold `± other · 2^shift` into this accumulator: the one body
    /// behind the four `*_accum` entry points.
    ///
    /// A register-held operand folds as one exact value — into the
    /// receiver's register when the shifted sum fits, through the digit
    /// engine otherwise — and a digit-held operand folds digit by
    /// digit, each landing negated when `negative` (the zone is
    /// symmetric about zero, so a negated digit is still in it —
    /// subtraction needs no borrow machinery of its own).
    fn fold_accum(&mut self, other: &Accumulator, shift: u64, negative: bool) {
        if let Some(operand_value) = other.quick {
            touch(1);
            if operand_value == 0 {
                return;
            }
            let operand_value = if negative {
                -operand_value
            } else {
                operand_value
            };
            if shift == 0 {
                if !self.quick_add(operand_value) {
                    self.add_at(0, operand_value);
                }
            } else if shift > QUICK_SHIFT_MAX || !self.quick_add(operand_value << shift) {
                self.spill();
                self.deposit_value(operand_value, shift);
            }
            return;
        }
        self.spill();
        let (digit_shift, bit_shift) =
            (shift / u64::from(DIGIT_BITS), shift % u64::from(DIGIT_BITS));
        for (offset, &digit) in other.digits[..=other.top].iter().enumerate() {
            touch(1);
            if digit != 0 {
                let contribution = i128::from(digit) << bit_shift;
                self.add_at(
                    landing(u128::from(digit_shift) + offset as u128),
                    if negative {
                        -contribution
                    } else {
                        contribution
                    },
                );
            }
        }
    }

    /// Scale the held value by `2^shift` in place: O(held digits) digit
    /// touches, and the digit buffer covers the shifted positions.
    ///
    /// The re-denomination move of a weighted fold that keeps its running
    /// sum in units of the finest scale seen so far: when a summand
    /// arrives at a finer scale than the current unit, the held digits
    /// shift up by the gap and the unit drops to match — one in-place
    /// shift per unit change, and every other summand enters through a
    /// shifted add at its own gap.
    ///
    /// On a digit-engine value the shift rebuilds the held value and may
    /// release the digit buffer: a pooled accumulator — one re-armed by
    /// [`reset`](Accumulator::reset) to keep its capacity — loses its
    /// warm buffer here.
    ///
    /// # Complexity
    ///
    /// `O(|self|)` digit touches, independent of the shift; the digit
    /// buffer covers the shifted positions.
    ///
    /// # Panics
    ///
    /// Panics under the same condition as
    /// [`add_limbs_shl`](Accumulator::add_limbs_shl).
    pub fn shl(&mut self, shift: u64) {
        // Identity fast path: a zero shift or a literal zero changes
        // nothing, and returning here keeps both free — no rebuild of a
        // digit-engine value, no spill of a zero register on a wide
        // shift. Value-observable behavior is identical without this
        // guard; what it routes is cost and representation.
        if shift == 0 || self.is_literally_zero() {
            return;
        }
        if let Some(value) = self.quick {
            touch(1);
            if shift <= QUICK_SHIFT_MAX {
                let shifted = value << shift;
                if shifted.unsigned_abs() <= QUICK_MAX {
                    self.quick = Some(shifted);
                } else {
                    self.enter_digit_engine();
                    self.deposit_value(shifted, 0);
                }
            } else {
                self.enter_digit_engine();
                self.deposit_value(value, shift);
            }
            return;
        }
        let held = core::mem::take(self);
        self.add_accum_shl(&held, shift);
    }

    /// Negate the held value in place: O(held digits).
    ///
    /// Digit-wise: a balanced digit's negation stays in the lazy zone, so
    /// no carries move.
    ///
    /// # Complexity
    ///
    /// `O(|self|)` digit touches.
    pub fn negate(&mut self) {
        if let Some(value) = &mut self.quick {
            touch(1);
            *value = -*value;
            return;
        }
        for digit in &mut self.digits[..=self.top] {
            touch(1);
            *digit = -*digit;
        }
    }

    /// Reset to zero, keeping the digit buffer's capacity.
    ///
    /// A caller that opens and closes many scoped totals can re-arm one
    /// accumulator instead of allocating per scope.
    ///
    /// A reset **scans**: it zeroes every held digit to keep the
    /// allocation. Replacing the accumulator with a fresh
    /// [`new`](Accumulator::new) is O(1) and drops the buffer instead.
    /// Choose by what happens next: reset wins when the capacity will
    /// be spilled into again, replacement when a wide buffer has served
    /// its purpose and the allocation is not worth carrying.
    ///
    /// # Complexity
    ///
    /// `O(|self|)` digit touches.
    pub fn reset(&mut self) {
        if self.quick.is_none() {
            for digit in &mut self.digits[..=self.top] {
                touch(1);
                *digit = 0;
            }
            self.top = 0;
            self.bottom = usize::MAX;
            self.zero_runs.clear();
        }
        // Back to the register: the zeroed digit buffer stays for the
        // next spill, so reuse keeps its capacity.
        self.quick = Some(0);
    }

    /// Pre-size the digit buffer to cover positions `0..digits`: no
    /// digit touches, one allocation at most.
    ///
    /// An allocation-shaping hint, value-neutral: a caller that knows
    /// the scale its writes will reach (a fold aligning summands to a
    /// known common exponent) reserves once and every later buffer
    /// growth is in-place, so the peak transient is the buffer itself —
    /// without the hint, incremental growth's doubling can briefly hold
    /// twice the final width, which is the difference between fitting
    /// and failing near a 32-bit target's memory ceiling. Reserving
    /// less than the writes reach costs nothing but the doubling; extra
    /// reserved capacity is plain unused memory until
    /// [`shl`](Accumulator::shl) on a digit-engine value or a
    /// replacement drops the buffer.
    ///
    /// # Complexity
    ///
    /// No digit touches; one buffer allocation when capacity grows.
    pub fn reserve_digits(&mut self, digits: usize) {
        self.digits
            .reserve_exact(digits.saturating_sub(self.digits.len()));
    }

    /// The sign of the held value — `value.cmp(&0)`, so `Less` means
    /// negative: amortized O(1).
    ///
    /// Folds digits from the top and decides at running partial `|s| ≥ 3`
    /// (the crate docs' domination bound). When the fold had to descend —
    /// a cancelling prefix — the scanned digits are collapsed to their
    /// partial at the scan's floor, so the scan is paid at most once per
    /// write (the crate docs' amortization argument). The rewrite is
    /// value-preserving.
    ///
    /// # Complexity
    ///
    /// Amortized `O(1)` digit touches.
    #[inline]
    pub fn sign(&mut self) -> Ordering {
        if let Some(value) = self.quick {
            touch(1);
            return value.cmp(&0);
        }
        let (_, partial) = self.fold_and_collapse();
        partial.cmp(&0)
    }

    /// Whether the held value is strictly negative: amortized O(1).
    ///
    /// Takes `&mut self` for the same value-preserving collapse as
    /// [`sign`](Accumulator::sign).
    ///
    /// # Complexity
    ///
    /// Amortized `O(1)` digit touches.
    #[inline]
    pub fn is_negative(&mut self) -> bool {
        self.sign() == Ordering::Less
    }

    /// The sign, plus whether the held magnitude certainly dominates any
    /// machine-word adjustment: amortized O(1), collapsing like
    /// [`sign`](Accumulator::sign).
    ///
    /// Returns `(sign, decided)`. Equivalent to
    /// [`sign_dominates_at`](Accumulator::sign_dominates_at)`(1)`: every
    /// `u64` value is below `2^(32·2)`, the bound `floor = 1` covers. A
    /// comparison against a word-scale adjustment reads this instead of
    /// folding, so a wide running total is never touched across its
    /// width by a cheap comparison.
    ///
    /// # Complexity
    ///
    /// Amortized `O(1)` digit touches.
    pub fn sign_dominates_word(&mut self) -> (Ordering, bool) {
        self.sign_dominates_at(1)
    }

    /// The sign, plus whether the held magnitude certainly dominates any
    /// value fitting in digits `0..=floor`: amortized O(1), collapsing
    /// like [`sign`](Accumulator::sign).
    ///
    /// Returns `(sign, decided)`. The sign is exact regardless of
    /// `decided`; `decided = true` guarantees `sign(v + a) = sign(v)` and
    /// `|v| > |a|` for every adjustment `a` with
    /// `|a| < 2^(32·(floor + 1))`, and moreover for any accumulator held
    /// in digits `0..=floor` (its redundant spelling can exceed that,
    /// bounded by `2.01 · 2^(32·(floor + 1))`; the margin covers it). To
    /// cover an adjustment below `2^b`, pass `floor = b.div_ceil(32) - 1`;
    /// to compare against another accumulator, `floor = its
    /// digit_count - 1`.
    /// `decided = false` means only that the fold could not certify
    /// domination — it is no evidence that an adjustment can flip the
    /// sign; fold the adjustment in (into a
    /// [`clone`](Clone::clone) when the held value must survive the
    /// probe) and read [`sign`](Accumulator::sign).
    ///
    /// `decided` is a property of the value's *current representation* —
    /// which tier holds it, and within the digit engine, the spelling
    /// the operation history left — never of the value alone. A
    /// register-held value certifies by direct magnitude comparison:
    /// `decided` is true exactly when
    /// `|value| ≥ 3 · 2^(32·(floor + 1))` (the register is exact, and
    /// the factor 3 clears the `2.01 · 2^(32·(floor + 1))`
    /// redundant-spelling operand bound with the same margin the digit
    /// fold uses). A digit-engine value certifies when the sign fold's
    /// running partial reaches `|s| ≥ 3` at digit index `floor + 2` or
    /// higher: at decision index `i` the unscanned digits below
    /// contribute under `2.01 · 2^(32·i)` (the crate docs' domination
    /// bound), so `|value| ≥ 0.99 · 2^(32·i)`; an operand with top digit
    /// index at most `floor` holds under `2.01 · 2^(32·(floor + 1))`
    /// (the same geometric bound one level up), and
    /// `0.99 · 2^(32·(floor + 2)) > 2.01 · 2^(32·(floor + 1))` by a
    /// factor over `2^30` — so folding any such operand in could flip
    /// neither the sign nor which magnitude is larger. The register's
    /// direct comparison certifies more values than the fold's index
    /// test: a decided fold implies `|value| ≥ 0.99 · 2^(32·(floor + 2))`,
    /// so a value between `3 · 2^(32·(floor + 1))` and that bound
    /// certifies only while register-held — after a spill the same
    /// value reads `decided = false`, and whether a wider spelled value
    /// certifies depends on the spelling, not only the magnitude.
    ///
    /// # Complexity
    ///
    /// Amortized `O(1)` digit touches.
    pub fn sign_dominates_at(&mut self, floor: usize) -> (Ordering, bool) {
        if let Some(value) = self.quick {
            touch(1);
            // The register holds the exact value, so the certificate is
            // the direct comparison: `3 · 2^(32·(floor + 1))` dominates
            // the `2.01 · 2^(32·(floor + 1))` redundant-spelling bound
            // with the same margin the digit fold's decision uses.
            let decided = floor
                .checked_add(1)
                .and_then(|digits| digits.checked_mul(32))
                .and_then(|bits| u32::try_from(bits).ok())
                .and_then(|bits| 3u128.checked_shl(bits))
                .is_some_and(|bound| value.unsigned_abs() >= bound);
            return (value.cmp(&0), decided);
        }
        let (index, partial) = self.fold_and_collapse();
        // Saturating: a floor within 2 of `usize::MAX` names an
        // adjustment bound no held value can dominate, so the decision
        // index must stay unsatisfiable rather than wrap to a tiny one
        // (a wrapped `floor + 2` would certify domination over an
        // astronomically wide adjustment from a 3-digit value).
        let decided = partial.abs() >= SIGN_DECIDED && index >= floor.saturating_add(2);
        (partial.cmp(&0), decided)
    }

    /// Fold digits from the top until the running partial decides the
    /// sign or the scan reaches digit 0, collapsing whatever was
    /// scanned: returns the scan's floor index and the exact partial
    /// there.
    ///
    /// The shared kernel behind [`sign`](Accumulator::sign) and
    /// [`sign_dominates_at`](Accumulator::sign_dominates_at). Digits
    /// are zeroed as the fold descends past them and the partial is
    /// re-deposited whole at the floor, so no future fold re-reads
    /// them (the crate docs' collapse amortization). A zero partial
    /// skips certified zero runs whole — a nonzero partial decides
    /// within one step, so the fold never walks into a certified run
    /// while carrying value. The rewrite is value-preserving: the
    /// digits change, the integer they denote never does.
    fn fold_and_collapse(&mut self) -> (usize, i128) {
        debug_assert!(self.quick.is_none(), "the sign fold reads the digit engine");
        let start_top = self.top;
        let mut index = start_top;
        let mut partial: i128 = 0;
        loop {
            touch(1);
            partial = (partial << DIGIT_BITS) + i128::from(self.digits[index]);
            if partial.abs() >= SIGN_DECIDED || index == 0 {
                break;
            }
            // Descending: this digit's value lives in `partial` now;
            // zero it so the floor re-deposit preserves the value.
            self.digits[index] = 0;
            touch(1);
            if partial == 0 {
                if let Some(lo) = self.consume_run_at(index) {
                    // A zero partial shifts to zero, so the skip needs
                    // no positional bookkeeping.
                    index = lo;
                    continue;
                }
            }
            index -= 1;
        }
        if index < start_top {
            // Collapse: the descent zeroed everything above; zero the
            // floor digit too and re-deposit the exact partial there.
            self.digits[index] = 0;
            touch(1);
            self.top = index;
            if partial != 0 {
                self.add_at(index, partial);
            } else {
                // The fold reached digit 0 with nothing left: the value
                // is zero, so every outstanding certificate is moot —
                // and clearing keeps the every-run-at-or-below-top
                // structural invariant.
                self.zero_runs.clear();
            }
        }
        (index, partial)
    }

    /// Whether the held value is *literally* zero — every stored digit
    /// zero — without any scan or rewrite: O(1).
    ///
    /// **One-sided**: `true` means the value is zero; `false` means
    /// unknown. A zero built out of cancelling nonzero digits reads
    /// `false` until a sign read collapses it —
    /// [`sign`](Accumulator::sign)`() == Equal` is the exact zero test,
    /// and after it this reads `true`. Use this only where a false
    /// negative costs nothing (skipping work a literal zero makes
    /// unnecessary); never gate correctness on the `false` arm:
    ///
    /// ```
    /// use core::cmp::Ordering;
    /// use suanpan::Accumulator;
    ///
    /// let mut acc = Accumulator::new();
    /// acc.add_u64_shl(1, 32);
    /// // The machine-word write lands whole in digit 0, so the two writes
    /// // cancel across two digits instead of clearing one:
    /// acc.sub_small(1 << 32);
    /// assert!(!acc.is_literally_zero());       // zero, but spelled redundantly
    /// assert_eq!(acc.sign(), Ordering::Equal); // the exact test — and it collapses,
    /// assert!(acc.is_literally_zero());        // so the spelling is now canonical
    /// ```
    ///
    /// # Complexity
    ///
    /// `O(1)`.
    #[inline]
    pub fn is_literally_zero(&self) -> bool {
        match self.quick {
            Some(value) => value == 0,
            None => self.top == 0 && self.digits[0] == 0,
        }
    }

    /// Move out an exact `i64` held in the quick register, returning `self`
    /// unchanged when the value is wider or has entered the digit engine.
    ///
    /// A digit-engine value may happen to fit in `i64`; this method deliberately
    /// does not normalize it to find out. It is a representation query for
    /// callers choosing compact temporary storage, not a numeric conversion.
    ///
    /// # Complexity
    ///
    /// `O(1)` with no digit touches or allocation.
    pub fn into_i64(self) -> Result<i64, Self> {
        match self.quick.and_then(|value| i64::try_from(value).ok()) {
            Some(value) => Ok(value),
            None => Err(self),
        }
    }

    /// The number of digits up to and including the highest nonzero one;
    /// at least 1 (a zero accumulator counts its one zero digit): O(1).
    ///
    /// Exact, not a watermark: when a write zeroes the top digit, the
    /// top settles onto the next nonzero digit below, stepping only
    /// through digits some write paid for and skipping certified zero
    /// runs whole — amortized O(1), the crate docs' zero-run ledger
    /// argument.
    /// This is the size a scaled add of this accumulator will read (and a
    /// merge, when this is the narrower operand) — a caller balancing
    /// fold costs compares counts and merges the smaller operand into the
    /// larger, as [`merge_into_wider`](Accumulator::merge_into_wider)
    /// does.
    ///
    /// # Complexity
    ///
    /// `O(1)`.
    #[inline]
    pub fn digit_count(&self) -> usize {
        match self.quick {
            Some(0) => 1,
            Some(value) => (128 - value.unsigned_abs().leading_zeros() as usize).div_ceil(32),
            None => self.top + 1,
        }
    }

    /// The held value as a sign and normalized little-endian 64-bit
    /// limbs: O(held digits).
    ///
    /// The limbs are minimal (no high zero limb) and empty exactly when the
    /// sign is [`Ordering::Equal`]. This is a read-out, not a drain;
    /// accumulation may continue afterwards.
    ///
    /// # Complexity
    ///
    /// `O(|self|)` digit touches and a same-order limb allocation.
    pub fn sign_limbs(&self) -> (Ordering, Vec<u64>) {
        self.with_sign_limbs(|sign, limbs| (sign, limbs.to_vec()))
    }

    /// Pass the held value's sign and normalized limbs to `read`.
    ///
    /// This is the allocation-sensitive form of [`sign_limbs`](Self::sign_limbs):
    /// a register-held value is lent from a two-limb stack buffer, while a
    /// digit-held value allocates the same-order conversion buffer that the
    /// ordinary readout returns. The limb slice is valid only during `read`.
    ///
    /// # Complexity
    ///
    /// `O(|self|)` digit touches and, after the accumulator spills, same-order
    /// temporary space.
    pub fn with_sign_limbs<R>(&self, read: impl FnOnce(Ordering, &[u64]) -> R) -> R {
        if let Some(value) = self.quick {
            touch(self.digit_count() as u64);
            let (limbs, len) = limbs_from_u128(value.unsigned_abs());
            return read(value.cmp(&0), &limbs[..len]);
        }
        let (sign, digits) = self.read_digits(0);
        let limbs = limbs_from_digits(digits);
        debug_assert_eq!(
            sign == Ordering::Equal,
            limbs.is_empty(),
            "the readout's limbs are empty exactly at zero"
        );
        read(sign, &limbs)
    }

    /// The held value as a sign, normalized limbs, and a power-of-two scale:
    /// `value = ±magnitude · 2^shift`.
    ///
    /// The all-zero prefix below the lowest position written since the last
    /// [`reset`](Accumulator::reset) becomes `shift`, so a narrow value at a
    /// large scale costs its written span rather than its scale. Gaps between
    /// written positions remain part of that span. The limbs may retain low
    /// zero bits after cancellation; this is an exact spelling, not a
    /// maximally shifted one.
    ///
    /// # Complexity
    ///
    /// `O(w)` digit touches and space, where `w` is the written span from the
    /// lowest written digit through the highest nonzero digit.
    pub fn sign_limbs_shl(&self) -> (Ordering, Vec<u64>, u64) {
        self.with_sign_limbs_shl(|sign, limbs, shift| (sign, limbs.to_vec(), shift))
    }

    /// Pass the held value's sign, limbs, and retained scale to `read`.
    ///
    /// The allocation-sensitive form of [`sign_limbs_shl`](Self::sign_limbs_shl),
    /// with the same stack-backed register read and callback lifetime as
    /// [`with_sign_limbs`](Self::with_sign_limbs).
    ///
    /// # Complexity
    ///
    /// `O(w)` digit touches and, after the accumulator spills, `O(w)` temporary
    /// space, where `w` is the written span.
    pub fn with_sign_limbs_shl<R>(&self, read: impl FnOnce(Ordering, &[u64], u64) -> R) -> R {
        if let Some(value) = self.quick {
            touch(self.digit_count() as u64);
            let (limbs, len) = limbs_from_u128(value.unsigned_abs());
            return read(value.cmp(&0), &limbs[..len], 0);
        }
        let start = self.bottom.min(self.top);
        let (sign, digits) = self.read_digits(start);
        let limbs = limbs_from_digits(digits);
        read(sign, &limbs, 32 * start as u64)
    }

    /// Read out the suffix at or above `start` as a sign and normalized
    /// unsigned base-2^32 digits (little-endian, possibly with high
    /// zeros): the one carry pass behind every magnitude readout.
    ///
    /// Every digit below `start` must be zero, so the suffix is the whole value
    /// at scale `2^(32·start)`.
    fn read_digits(&self, start: usize) -> (Ordering, Vec<u32>) {
        // Low-to-high signed carry: after the pass, the collected unsigned
        // digits hold `M` with `value = carry · 2^(32·len) + M`,
        // `0 ≤ M < 2^(32·len)`. The final carry has magnitude at most 3:
        // each step floors `(digit + carry) / 2^32` with `|digit| < 2^33`,
        // so `[−3, 2]` is closed under the recurrence from 0 — the
        // high-part drains below each emit at most one nonzero digit.
        let start = start.min(self.top);
        let mut collected: Vec<u32> = Vec::with_capacity(self.top - start + 2);
        let mut carry: i128 = 0;
        for &digit in &self.digits[start..=self.top] {
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
            let low_nonzero = collected.iter().any(|&digit| digit != 0);
            if low_nonzero {
                let mut complement_carry = 1u64;
                for digit in collected.iter_mut() {
                    touch(1);
                    let complemented = (DIGIT_MASK - u64::from(*digit)) + complement_carry;
                    *digit = (complemented & DIGIT_MASK) as u32;
                    complement_carry = complemented >> DIGIT_BITS;
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
            (Ordering::Less, collected)
        } else {
            let mut high = carry as u128;
            while high > 0 {
                touch(1);
                collected.push((high & u128::from(DIGIT_MASK)) as u32);
                high >>= DIGIT_BITS;
            }
            let sign = if collected.iter().all(|&digit| digit == 0) {
                Ordering::Equal
            } else {
                Ordering::Greater
            };
            (sign, collected)
        }
    }

    /// Fold `other`'s held value into this one — `self` ends holding the
    /// sum — and return the spare buffer, **not** the sum: amortized
    /// O(the narrower operand's held digits) plus an O(1) buffer swap.
    ///
    /// Only the operand with fewer held digits is read: the sum always
    /// lands in whichever buffer held more (buffers are swapped first
    /// when `other` is the wider; on a tie, `other` is the one read and
    /// `self`'s buffer keeps the sum), so the digits a dying operand
    /// holds fund the fold that consumes it. *Amortized* is the write
    /// bound's usual accounting: a merge whose operands nearly cancel
    /// zeroes the receiver's top digits, and the settlement scan that
    /// re-finds the top spends credits prepaid by the writes that built
    /// those digits (the crate docs' zero-run ledger argument) — no
    /// merge schedule pays more than the narrower operand plus that
    /// prepaid settlement. The returned buffer is for the caller's
    /// pool: a valid accumulator holding an unspecified value — every
    /// operation on it remains memory-safe, but answers about that
    /// value are meaningless until [`reset`](Accumulator::reset).
    ///
    /// ```
    /// use suanpan::Accumulator;
    ///
    /// let mut sum = Accumulator::new();
    /// sum.add_small(7);
    /// let mut wide = Accumulator::new();
    /// wide.add_u64_shl(1, 640);
    /// let mut spare = sum.merge_into_wider(wide); // reads 1 digit, not 21
    /// let (_, limbs) = sum.sign_limbs();           // the sum lives in `sum`,
    /// assert_eq!(limbs, vec![7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
    /// spare.reset();                              // NOT in `spare`: reset it
    /// assert!(spare.is_literally_zero());         // before any reuse
    /// ```
    ///
    /// # Complexity
    ///
    /// Amortized `O(min(|self|, |other|))` digit touches, plus an
    /// `O(1)` buffer swap.
    pub fn merge_into_wider(&mut self, mut other: Accumulator) -> Accumulator {
        if other.digit_count() > self.digit_count() {
            core::mem::swap(self, &mut other);
        }
        self.add_accum(&other);
        other
    }

    /// Add one machine word times `2^shift`: amortized O(1) digit
    /// touches, independent of the shift.
    ///
    /// The digit buffer grows to cover the shifted positions.
    ///
    /// # Complexity
    ///
    /// Amortized `O(1)` digit touches, independent of the shift; the
    /// digit buffer grows to cover the shifted positions.
    ///
    /// # Panics
    ///
    /// Panics under the same condition as
    /// [`add_limbs_shl`](Accumulator::add_limbs_shl).
    #[inline]
    pub fn add_u64_shl(&mut self, word: u64, shift: u64) {
        self.add_shifted_word(word, false, shift);
    }

    /// Subtract one machine word times `2^shift`: amortized O(1) digit
    /// touches, independent of the shift.
    ///
    /// The subtractive twin of [`add_u64_shl`](Accumulator::add_u64_shl).
    ///
    /// # Complexity
    ///
    /// Amortized `O(1)` digit touches, independent of the shift; the
    /// digit buffer grows to cover the shifted positions.
    ///
    /// # Panics
    ///
    /// Panics under the same condition as
    /// [`add_limbs_shl`](Accumulator::add_limbs_shl).
    #[inline]
    pub fn sub_u64_shl(&mut self, word: u64, shift: u64) {
        self.add_shifted_word(word, true, shift);
    }

    /// Add or subtract one machine word times `2^shift`: amortized O(1).
    fn add_shifted_word(&mut self, word: u64, negative: bool, shift: u64) {
        if word == 0 {
            return;
        }
        if shift <= QUICK_SHIFT_MAX {
            // At most 94 bits shifted: inside the register's headroom.
            let value = i128::from(word) << shift;
            let value = if negative { -value } else { value };
            if self.quick_add(value) {
                return;
            }
        } else {
            self.spill();
        }
        let (digit_shift, bit_shift) =
            (shift / u64::from(DIGIT_BITS), shift % u64::from(DIGIT_BITS));
        // At most 96 bits after the sub-digit shift: well inside `i128`.
        let value = i128::from(word) << bit_shift;
        self.add_at(
            landing(u128::from(digit_shift)),
            if negative { -value } else { value },
        );
    }

    /// Fold `delta` into the quick register, spilling to the digit
    /// engine when the sum outgrows it.
    ///
    /// Returns `true` when the register held the value, `false` when
    /// the accumulator is already in digit mode and the caller must
    /// deposit the delta itself.
    ///
    /// `|delta|` must stay within the register's headroom
    /// (`QUICK_MAX << QUICK_SHIFT_MAX` and change): every caller passes
    /// a machine word, another register's value, or one of those
    /// shifted by at most [`QUICK_SHIFT_MAX`], so the sum sits far from
    /// `i128` overflow.
    #[inline]
    fn quick_add(&mut self, delta: i128) -> bool {
        let Some(held) = self.quick else {
            return false;
        };
        touch(1);
        let sum = held + delta;
        if sum.unsigned_abs() <= QUICK_MAX {
            self.quick = Some(sum);
        } else {
            self.enter_digit_engine();
            self.deposit_value(sum, 0);
        }
        true
    }

    /// Leave the quick register (if held), seeding the digit engine
    /// with the register's value: the one-way exit, O(1) plus the
    /// deposit of at most four digits.
    fn spill(&mut self) {
        if let Some(value) = self.quick {
            self.enter_digit_engine();
            self.deposit_value(value, 0);
        }
    }

    /// Arm the idle digit engine and retire the register.
    ///
    /// The buffer may be a retained pool allocation from an earlier
    /// epoch; [`reset`](Accumulator::reset) left it all zero, which is
    /// exactly the engine's starting state.
    fn enter_digit_engine(&mut self) {
        debug_assert!(self.quick.is_some(), "the digit engine arms once per epoch");
        self.quick = None;
        if self.digits.is_empty() {
            // Capacity exactly one, as a fresh digit buffer: growth from
            // here follows the resize path's own amortization, and a
            // never-spilled accumulator allocated nothing at all.
            self.digits.reserve_exact(1);
            self.digits.push(0);
        }
        debug_assert!(
            self.digits.iter().all(|&digit| digit == 0),
            "a retired register leaves the digit engine idle"
        );
        self.top = 0;
        self.bottom = usize::MAX;
        self.zero_runs.clear();
    }

    /// Deposit `value · 2^shift` into the digit engine, digit by digit.
    ///
    /// The register's exit move: like `apply_limbs` but for an exact
    /// `i128`, splitting the value into 32-bit contributions so no
    /// shifted intermediate can overflow.
    fn deposit_value(&mut self, value: i128, shift: u64) {
        debug_assert!(self.quick.is_none(), "deposits land in the digit engine");
        let (digit_shift, bit_shift) =
            (shift / u64::from(DIGIT_BITS), shift % u64::from(DIGIT_BITS));
        let mut position = u128::from(digit_shift);
        let negative = value.is_negative();
        let mut magnitude = value.unsigned_abs();
        while magnitude != 0 {
            touch(1);
            let digit = (magnitude & u128::from(DIGIT_MASK)) as i128;
            if digit != 0 {
                let contribution = digit << bit_shift;
                self.add_at(
                    landing(position),
                    if negative {
                        -contribution
                    } else {
                        contribution
                    },
                );
            }
            magnitude >>= DIGIT_BITS;
            position += 1;
        }
    }

    /// Add `value` (any sign, any `i128` magnitude) into the digit at
    /// `pos`, carrying upward until every touched digit is in the zone.
    ///
    /// O(value bits / 32) digit touches for the carry run, amortized
    /// O(1) for word-scale values, plus amortized O(1) top settlement
    /// (the crate docs' zero-run ledger argument). A landing site above
    /// the current top certifies the never-written run it jumps; a
    /// carry run landing inside a certified run splits the certificate
    /// around the digits it wrote.
    fn add_at(&mut self, mut pos: usize, mut value: i128) {
        debug_assert!(
            self.quick.is_none(),
            "digit writes land in the digit engine"
        );
        self.bottom = self.bottom.min(pos);
        if pos > self.top + 1 {
            // Every digit strictly between the old top and the landing
            // site is zero (all sit above the old top), and no run at
            // or below the old top can overlap the new one, so the
            // ledger stays disjoint.
            self.zero_runs.insert(self.top, pos);
        }
        let run_start = pos;
        // Invariant window: "`top` covers every nonzero digit" holds at
        // this loop's entry and exit but not within it — a carry step
        // may write a nonzero remainder above `top` without raising it.
        // The final landing restores the invariant by itself: the carry
        // arm always continues (recentering keeps `|carry| >= 2`
        // whenever `|total| >= LAZY_LIMIT`), so the chain ends in the
        // in-zone arm; if the chain climbed past the entry `top`, every
        // digit it reached up there was zero, so the landing sees
        // `total = value != 0` at the chain's highest position and
        // raises `top` past every remainder below it. Any exit added
        // inside this loop must re-establish the invariant itself; the
        // assert at the end of this function fails loudly if it does
        // not.
        while value != 0 {
            if pos >= self.digits.len() {
                self.digits.resize(pos + 1, 0);
            }
            touch(1);
            let total = i128::from(self.digits[pos]) + value;
            if total.abs() < LAZY_LIMIT {
                self.digits[pos] = total as i64;
                if total != 0 {
                    self.top = self.top.max(pos);
                }
                value = 0;
            } else {
                let carry = (total + RECENTER_BIAS) >> DIGIT_BITS;
                let remainder = total - (carry << DIGIT_BITS);
                self.digits[pos] = remainder as i64;
                value = carry;
                pos += 1;
            }
        }
        self.crop_runs(run_start, pos);
        self.settle_top();
        debug_assert!(
            (self.top == 0 || self.digits[self.top] != 0)
                && self.digits[self.top + 1..].iter().all(|&digit| digit == 0),
            "top must rest on the highest nonzero digit at add_at exit"
        );
    }

    /// Re-certify the ledger after the digits `[from, to]` were
    /// written: every certificate whose run the write landed in is
    /// split around it, keeping the sub-runs the write left untouched.
    ///
    /// O(runs intruded on) ledger operations — the write's own carry
    /// run bounds how many — plus one O(log ledger size) map descent.
    fn crop_runs(&mut self, from: usize, to: usize) {
        if self.zero_runs.is_empty() {
            return;
        }
        // A certificate `(lo, hi)` covers digits strictly between its
        // ends, so the write intrudes exactly when `lo < to` and
        // `hi > from`; runs are disjoint and sorted, so removing down
        // from the highest `lo` below `to` visits every intruded run
        // before reaching one entirely below the write. A kept lower
        // remnant `(lo, from)` ends the walk on the next probe: its end
        // is not past `from`, and no run below it intrudes either.
        while let Some((&lo, &hi)) = self.zero_runs.range(..to).next_back() {
            if hi <= from {
                break;
            }
            self.zero_runs.remove(&lo);
            if from > lo + 1 {
                self.zero_runs.insert(lo, from);
            }
            if hi > to + 1 {
                self.zero_runs.insert(to, hi);
            }
        }
    }

    /// Consume the certificate covering the digits just below `above`,
    /// if one exists: returns `lo` with every digit in `(lo, above)`
    /// zero, removing the certificate from the ledger.
    fn consume_run_at(&mut self, above: usize) -> Option<usize> {
        let (&lo, &hi) = self.zero_runs.range(..above).next_back()?;
        if hi >= above {
            self.zero_runs.remove(&lo);
            Some(lo)
        } else {
            None
        }
    }

    /// Settle `top` onto the highest nonzero digit: one touch per zero
    /// digit stepped past, one per certified run skipped whole.
    ///
    /// The exact-`top` invariant's maintenance scan, amortized O(1)
    /// per write (the crate docs' zero-run ledger argument): every
    /// plain step spends the credit deposited by the metered write
    /// that last touched that digit, and every skip consumes a
    /// certificate recorded in O(1) by the write that jumped the run.
    fn settle_top(&mut self) {
        while self.top > 0 && self.digits[self.top] == 0 {
            touch(1);
            self.top = match self.consume_run_at(self.top) {
                Some(lo) => lo,
                None => self.top - 1,
            };
        }
    }

    /// Apply a little-endian 64-bit limb stream scaled by `2^shift`.
    ///
    /// Digit-aligned: each limb lands as two independent contributions at
    /// its own shifted positions, so a wide operand costs O(its limbs)
    /// regardless of the held width or the shift. Streaming a borrowed stored
    /// representation allocates nothing.
    fn apply_limbs<I: Iterator<Item = u64>>(&mut self, limbs: I, negative: bool, shift: u64) {
        let (digit_shift, bit_shift) =
            (shift / u64::from(DIGIT_BITS), shift % u64::from(DIGIT_BITS));
        for (limb_index, limb) in limbs.enumerate() {
            touch(1);
            // At most 33 + 31 bits per contribution after the sub-digit
            // shift: well inside the `i128` `add_at` carries from.
            let low = i128::from(limb & DIGIT_MASK) << bit_shift;
            let high = i128::from(limb >> DIGIT_BITS) << bit_shift;
            let position = u128::from(digit_shift) + 2 * limb_index as u128;
            if low != 0 {
                self.add_at(landing(position), if negative { -low } else { low });
            }
            if high != 0 {
                self.add_at(landing(position + 1), if negative { -high } else { high });
            }
        }
    }
}

/// Convert a digit position to an index whose buffer length is representable.
fn landing(position: u128) -> usize {
    usize::try_from(position)
        .ok()
        .filter(|&index| index < usize::MAX)
        .expect("digit landing fits the accumulator buffer")
}

impl Default for Accumulator {
    fn default() -> Accumulator {
        Accumulator::new()
    }
}

/// Construct an accumulator from a signed machine word without allocation.
impl From<i64> for Accumulator {
    fn from(value: i64) -> Accumulator {
        Accumulator {
            quick: Some(i128::from(value)),
            ..Accumulator::new()
        }
    }
}

/// Spell an unsigned `u128` in a two-limb stack buffer.
fn limbs_from_u128(value: u128) -> ([u64; 2], usize) {
    let limbs = [value as u64, (value >> 64) as u64];
    let len = usize::from(value != 0) + usize::from(value > u128::from(u64::MAX));
    (limbs, len)
}

/// Pack little-endian base-2^32 digits into minimal 64-bit limbs.
fn limbs_from_digits(digits: Vec<u32>) -> Vec<u64> {
    let mut limbs: Vec<u64> = digits
        .chunks(2)
        .map(|pair| u64::from(pair[0]) | (pair.get(1).copied().map_or(0, u64::from) << 32))
        .collect();
    drop(digits);
    while limbs.last() == Some(&0) {
        limbs.pop();
    }
    limbs
}

#[cfg(test)]
mod tests;
