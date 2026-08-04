//! The accumulator: redundant balanced signed digits in the lazy zone,
//! the collapsing sign fold, and the zero-run ledger.
//!
//! The crate docs carry the representation and every cost argument; the
//! field docs on [`Accumulator`] state the structural invariants the
//! operations maintain, and the sibling tests hold both against an
//! exact big-integer oracle and the touch meter. Every digit
//! read-modify-write in this module is counted through [`touch`], the
//! seam the `touch-meter` feature makes observable.

use core::cmp::Ordering;
use std::collections::BTreeMap;

use crate::{Limbs, Magnitude, UBig};

/// Record `n` accumulator digit touches.
///
/// Compiles to nothing without the `touch-meter` feature, so the hot paths
/// call it unconditionally.
#[inline(always)]
fn touch(n: u64) {
    #[cfg(feature = "touch-meter")]
    crate::touch_meter::record(n);
    #[cfg(not(feature = "touch-meter"))]
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
/// in magnitude (the crate docs' domination bound), so a partial of
/// magnitude 3 or more cannot be overturned from below.
const SIGN_DECIDED: i128 = 3;

/// A running signed integer over redundant balanced base-2^32 digits.
///
/// Deltas are added or subtracted at machine-word or arbitrary width; the
/// sign is readable at any point in amortized O(1); one low-to-high carry
/// pass ([`sign_magnitude`](Accumulator::sign_magnitude)) converts the
/// held value to a normalized magnitude. The crate docs carry the
/// representation and both cost arguments. Sign queries take `&mut self`
/// because they may collapse a scanned cancelling prefix; the rewrite
/// never changes the value the digits denote.
///
/// # Complexity
///
/// `Clone` and `Debug` `O(the digit buffer: the highest position ever written)`; `Default` `O(1)`.
/// The costs of the operations live on the operations (the crate docs'
/// table is the overview); what is priced here is the derived surface.
/// `Clone` and `Debug` walk the digit buffer, which covers the highest
/// position ever written since construction and never shrinks — after a
/// wide interlude collapses to a narrow value, a clone still pays the
/// old width (a [`reset`](Accumulator::reset) does not release it
/// either; only dropping the accumulator does).
#[derive(Debug, Clone)]
pub struct Accumulator {
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
    /// [`sign_magnitude_shl`](Accumulator::sign_magnitude_shl) skip the
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
            digits: vec![0],
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
    pub fn add_small(&mut self, delta: i64) {
        if delta != 0 {
            self.add_at(0, i128::from(delta));
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
    pub fn sub_small(&mut self, delta: i64) {
        if delta != 0 {
            self.add_at(0, -i128::from(delta));
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
    pub fn add_u64(&mut self, delta: u64) {
        if delta != 0 {
            self.add_at(0, i128::from(delta));
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
    pub fn sub_u64(&mut self, delta: u64) {
        if delta != 0 {
            self.add_at(0, -i128::from(delta));
        }
    }

    /// Add a wide delta: amortized O(operand limbs), a limb being one
    /// 64-bit word of the operand — the cost scales with the operand's
    /// width, never the held value's.
    ///
    /// # Complexity
    ///
    /// Amortized `O(operand limbs)` digit touches, whatever the held width.
    pub fn add_wide(&mut self, delta: &UBig) {
        self.apply_limbs(Limbs::new(delta), false, 0);
    }

    /// Subtract a wide delta: amortized O(operand limbs), scaling with
    /// the operand's width, never the held value's.
    ///
    /// # Complexity
    ///
    /// Amortized `O(operand limbs)` digit touches, whatever the held width.
    pub fn sub_wide(&mut self, delta: &UBig) {
        self.apply_limbs(Limbs::new(delta), true, 0);
    }

    /// Add a stored magnitude, at the width it is stored at.
    ///
    /// A word-scale operand takes the amortized-O(1) small path, a wider
    /// one the amortized-O(operand limbs) wide path; [`Magnitude`] is the
    /// dispatch.
    ///
    /// # Complexity
    ///
    /// Word-scale operands amortized `O(1)` digit touches, wide operands amortized `O(operand limbs)`.
    pub fn add_magnitude<M: Magnitude>(&mut self, delta: &M) {
        match delta.to_word() {
            Some(n) => self.add_u64(n),
            None => self.add_wide(delta.as_wide()),
        }
    }

    /// Subtract a stored magnitude, at the width it is stored at.
    ///
    /// The subtractive twin of [`add_magnitude`](Accumulator::add_magnitude).
    ///
    /// # Complexity
    ///
    /// Word-scale operands amortized `O(1)` digit touches, wide operands amortized `O(operand limbs)`.
    pub fn sub_magnitude<M: Magnitude>(&mut self, delta: &M) {
        match delta.to_word() {
            Some(n) => self.sub_u64(n),
            None => self.sub_wide(delta.as_wide()),
        }
    }

    /// Add `delta · 2^shift`: amortized O(operand limbs) digit touches,
    /// independent of the shift.
    ///
    /// The scaled entry point behind weighted folds — a summand carrying
    /// its own exponent, such as a value weighted by a dyadic interval
    /// width or a numerator aligned to a larger scale. The shift routes
    /// each operand limb directly to the digit positions it spans, so no
    /// shifted copy of the operand ever exists. Memory is the exception
    /// to shift-independence: the digit buffer grows to cover the shifted
    /// position, O(shift / 32) plus the operand's digits.
    ///
    /// # Complexity
    ///
    /// Amortized `O(operand limbs)` digit touches, independent of the shift; the digit buffer grows to cover the shifted positions.
    ///
    /// # Panics
    ///
    /// Panics if the shifted digit position `shift / 32` overflows
    /// `usize` — possible only on targets narrower than 64 bits (from
    /// `shift = 2^37` on a 32-bit one). On 64-bit targets every `u64`
    /// shift fits, and an enormous one fails at allocation instead, like
    /// any collection asked to grow to `shift / 32` entries.
    pub fn add_wide_shl(&mut self, delta: &UBig, shift: u64) {
        self.apply_limbs(Limbs::new(delta), false, shift);
    }

    /// Subtract `delta · 2^shift`: amortized O(operand limbs) digit
    /// touches, independent of the shift.
    ///
    /// The subtractive twin of
    /// [`add_wide_shl`](Accumulator::add_wide_shl), with the same memory
    /// note.
    ///
    /// # Complexity
    ///
    /// Amortized `O(operand limbs)` digit touches, independent of the shift; the digit buffer grows to cover the shifted positions.
    ///
    /// # Panics
    ///
    /// As [`add_wide_shl`](Accumulator::add_wide_shl): a shifted digit
    /// position past `usize` panics.
    pub fn sub_wide_shl(&mut self, delta: &UBig, shift: u64) {
        self.apply_limbs(Limbs::new(delta), true, shift);
    }

    /// Add a stored magnitude times `2^shift`, at the width it is stored
    /// at.
    ///
    /// The same width dispatch as [`add_magnitude`](Accumulator::add_magnitude),
    /// with digit touches independent of the shift and
    /// [`add_wide_shl`](Accumulator::add_wide_shl)'s memory note.
    ///
    /// # Complexity
    ///
    /// Word-scale operands amortized `O(1)` digit touches, wide operands amortized `O(operand limbs)`, independent of the shift; the digit buffer grows to cover the shifted positions.
    ///
    /// # Panics
    ///
    /// As [`add_wide_shl`](Accumulator::add_wide_shl): a shifted digit
    /// position past `usize` panics.
    pub fn add_magnitude_shl<M: Magnitude>(&mut self, delta: &M, shift: u64) {
        match delta.to_word() {
            Some(0) => {}
            Some(n) => self.add_shifted_word(n, false, shift),
            None => self.add_wide_shl(delta.as_wide(), shift),
        }
    }

    /// Subtract a stored magnitude times `2^shift`, at the width it is
    /// stored at.
    ///
    /// The subtractive twin of
    /// [`add_magnitude_shl`](Accumulator::add_magnitude_shl): the same width
    /// dispatch, shift-independent digit touches, and memory note.
    ///
    /// # Complexity
    ///
    /// Word-scale operands amortized `O(1)` digit touches, wide operands amortized `O(operand limbs)`, independent of the shift; the digit buffer grows to cover the shifted positions.
    ///
    /// # Panics
    ///
    /// As [`add_wide_shl`](Accumulator::add_wide_shl): a shifted digit
    /// position past `usize` panics.
    pub fn sub_magnitude_shl<M: Magnitude>(&mut self, delta: &M, shift: u64) {
        match delta.to_word() {
            Some(0) => {}
            Some(n) => self.add_shifted_word(n, true, shift),
            None => self.apply_limbs(Limbs::new(delta.as_wide()), true, shift),
        }
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
    /// Amortized `O(the operand's held digits)` digit touches, whatever the receiver's width.
    pub fn add_accum(&mut self, other: &Accumulator) {
        self.add_accum_shl(other, 0);
    }

    /// Subtract another accumulator's held value from this one: amortized
    /// O(the operand's held digits).
    ///
    /// The subtractive twin of [`add_accum`](Accumulator::add_accum),
    /// with the same once-not-per-iteration cost discipline.
    ///
    /// # Complexity
    ///
    /// Amortized `O(the operand's held digits)` digit touches, whatever the receiver's width.
    pub fn sub_accum(&mut self, other: &Accumulator) {
        for (i, &digit) in other.digits[..=other.top].iter().enumerate() {
            touch(1);
            if digit != 0 {
                self.add_at(i, -i128::from(digit));
            }
        }
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
    /// Amortized `O(the operand's held digits)` digit touches, independent of the shift; the digit buffer grows to cover the shifted positions.
    ///
    /// # Panics
    ///
    /// As [`add_wide_shl`](Accumulator::add_wide_shl): a shifted digit
    /// position past `usize` panics.
    pub fn add_accum_shl(&mut self, other: &Accumulator, shift: u64) {
        let (digit_shift, bit_shift) =
            (shift / u64::from(DIGIT_BITS), shift % u64::from(DIGIT_BITS));
        let digit_shift = usize::try_from(digit_shift).expect("digit positions fit a usize");
        for (i, &digit) in other.digits[..=other.top].iter().enumerate() {
            touch(1);
            if digit != 0 {
                self.add_at(i + digit_shift, i128::from(digit) << bit_shift);
            }
        }
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
    /// Amortized `O(the operand's held digits)` digit touches, independent of the shift; the digit buffer grows to cover the shifted positions.
    ///
    /// # Panics
    ///
    /// As [`add_wide_shl`](Accumulator::add_wide_shl): a shifted digit
    /// position past `usize` panics.
    pub fn sub_accum_shl(&mut self, other: &Accumulator, shift: u64) {
        let (digit_shift, bit_shift) =
            (shift / u64::from(DIGIT_BITS), shift % u64::from(DIGIT_BITS));
        let digit_shift = usize::try_from(digit_shift).expect("digit positions fit a usize");
        for (i, &digit) in other.digits[..=other.top].iter().enumerate() {
            touch(1);
            if digit != 0 {
                self.add_at(i + digit_shift, -(i128::from(digit) << bit_shift));
            }
        }
    }

    /// Scale the held value by `2^shift` in place: O(held digits) digit
    /// touches, and the digit buffer grows to cover the shifted positions.
    ///
    /// The re-denomination move of a weighted fold that keeps its running
    /// sum in units of the finest scale seen so far: when a summand
    /// arrives at a finer scale than the current unit, the held digits
    /// shift up by the gap and the unit drops to match — one in-place
    /// shift per unit change, and every other summand enters through a
    /// shifted add at its own gap.
    ///
    /// # Complexity
    ///
    /// `O(held digits)` digit touches, independent of the shift; the digit buffer grows to cover the shifted positions.
    ///
    /// # Panics
    ///
    /// As [`add_wide_shl`](Accumulator::add_wide_shl): a shifted digit
    /// position past `usize` panics.
    pub fn shl(&mut self, shift: u64) {
        if shift == 0 || (self.top == 0 && self.digits[0] == 0) {
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
    /// `O(held digits)` digit touches.
    pub fn negate(&mut self) {
        for digit in &mut self.digits[..=self.top] {
            touch(1);
            *digit = -*digit;
        }
    }

    /// Reset to zero, keeping the digit buffer's capacity.
    ///
    /// The pool-reuse entry point: a caller that opens and closes many
    /// scoped totals re-arms one cleared accumulator instead of
    /// allocating per scope.
    ///
    /// # Complexity
    ///
    /// `O(held digits)` digit touches.
    pub fn reset(&mut self) {
        for digit in &mut self.digits[..=self.top] {
            touch(1);
            *digit = 0;
        }
        self.top = 0;
        self.bottom = usize::MAX;
        self.zero_runs.clear();
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
    pub fn sign(&mut self) -> Ordering {
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
    /// `decided` is true exactly when the sign fold's partial reached
    /// `|s| ≥ 3` at digit index `floor + 2` or higher (so a value held in
    /// fewer than `floor + 3` digits always reads `decided = false`). At
    /// decision index `i` the unscanned digits below contribute under
    /// `2.01 · 2^(32·i)` (the crate docs' domination bound), so
    /// `|value| ≥ 0.99 · 2^(32·i)`; an operand with top digit index at
    /// most `floor` holds under `2.01 · 2^(32·(floor + 1))` (the same
    /// geometric bound one level up), and
    /// `0.99 · 2^(32·(floor + 2)) > 2.01 · 2^(32·(floor + 1))` by a
    /// factor over `2^30` — so folding any such operand in could flip
    /// neither the sign nor which magnitude is larger.
    ///
    /// # Complexity
    ///
    /// Amortized `O(1)` digit touches.
    pub fn sign_dominates_at(&mut self, floor: usize) -> (Ordering, bool) {
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
    /// use suanpan::{Accumulator, UBig};
    ///
    /// let mut acc = Accumulator::new();
    /// acc.add_wide(&(UBig::from(1u8) << 32usize));
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
    pub fn is_literally_zero(&self) -> bool {
        self.top == 0 && self.digits[0] == 0
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
    pub fn digit_count(&self) -> usize {
        self.top + 1
    }

    /// The held value as a sign and a normalized magnitude: O(held
    /// digits).
    ///
    /// One low-to-high pass with a signed carry. The magnitude is zero
    /// exactly when the sign is [`Ordering::Equal`]. The accumulator
    /// itself is unchanged — this is a read-out, not a drain, and
    /// accumulation can continue after it.
    ///
    /// # Complexity
    ///
    /// `O(held digits)` digit touches and a same-order magnitude allocation.
    pub fn sign_magnitude(&self) -> (Ordering, UBig) {
        let (sign, magnitude) = self.read_magnitude(0);
        (sign, magnitude)
    }

    /// The held value as a sign, a magnitude, and a power-of-two scale —
    /// `value = ±magnitude · 2^shift`: O(the written span since the
    /// last [`reset`](Accumulator::reset)).
    ///
    /// [`sign_magnitude`](Accumulator::sign_magnitude)'s scaled twin,
    /// for totals accumulated far above digit zero (a weighted fold's
    /// per-segment mass, deposited at the exponent of each summand): the
    /// all-zero prefix below the lowest position any write has touched
    /// since the last reset is returned as the `shift` (always a
    /// multiple of 32) instead of being scanned into low zero bytes, so
    /// reading a narrow value parked at a large scale costs its written
    /// span, not its scale. The span is a distance, not a count: it runs
    /// from that lowest written position up to the top, and never-written
    /// gaps *between* writes are scanned like any other digit — parking
    /// one value far above another prices this read at the distance
    /// between them. The magnitude may still carry trailing zeros
    /// when written digits cancelled downward — the skip is exact only
    /// over the never-written region — and sign queries count as writers
    /// here: a collapsing sign read re-deposits its scanned partial
    /// through the ordinary write path, at an index that can sit below
    /// every position the caller's own writes touched, so interleaved
    /// sign reads can lower the returned `shift`. The
    /// `(magnitude, shift)` pair is therefore one honest spelling of the
    /// value, not a normal form.
    ///
    /// # Complexity
    ///
    /// `O(the written span)` digit touches — every digit from the lowest position written since the last reset up to the top, never-written gaps included — and a same-order magnitude allocation.
    pub fn sign_magnitude_shl(&self) -> (Ordering, UBig, u64) {
        let start = self.bottom.min(self.top);
        let (sign, magnitude) = self.read_magnitude(start);
        (sign, magnitude, 32 * start as u64)
    }

    /// Read out `Σ_{i ≥ start} digits[i] · 2^(32·(i − start))` as a sign
    /// and a normalized magnitude.
    ///
    /// Sound only when every digit below `start` is zero (the callers
    /// pass 0 or the write watermark [`Accumulator::bottom`]), so the
    /// suffix read is the whole value at scale `2^(32·start)`.
    fn read_magnitude(&self, start: usize) -> (Ordering, UBig) {
        debug_assert!(
            self.digits[..start.min(self.top + 1)]
                .iter()
                .all(|&d| d == 0),
            "read_magnitude below the write watermark: skipped digits must be zero"
        );
        // Low-to-high signed carry: after the pass, the collected unsigned
        // digits hold `M` with `value = carry · 2^(32·len) + M`,
        // `0 ≤ M < 2^(32·len)`.
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
            (Ordering::Less, magnitude_from_digits(collected))
        } else {
            let mut high = carry as u128;
            while high > 0 {
                touch(1);
                collected.push((high & u128::from(DIGIT_MASK)) as u32);
                high >>= DIGIT_BITS;
            }
            let magnitude = magnitude_from_digits(collected);
            let sign = if magnitude == UBig::ZERO {
                Ordering::Equal
            } else {
                Ordering::Greater
            };
            (sign, magnitude)
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
    /// holds fund the fold that consumes it. The returned buffer is for the caller's pool: a
    /// valid accumulator holding an unspecified value — every operation
    /// on it remains memory-safe, but answers about that value are
    /// meaningless until [`reset`](Accumulator::reset).
    ///
    /// ```
    /// use suanpan::{Accumulator, UBig};
    ///
    /// let mut sum = Accumulator::new();
    /// sum.add_small(7);
    /// let mut wide = Accumulator::new();
    /// wide.add_wide_shl(&UBig::from(1u8), 640);
    /// let mut spare = sum.merge_into_wider(wide); // reads 1 digit, not 21
    /// let (_, magnitude) = sum.sign_magnitude();  // the sum lives in `sum`,
    /// assert_eq!(magnitude, (UBig::from(1u8) << 640usize) + 7u8);
    /// spare.reset();                              // NOT in `spare`: reset it
    /// assert!(spare.is_literally_zero());         // before any reuse
    /// ```
    ///
    /// # Complexity
    ///
    /// Amortized `O(the narrower operand's held digits)` digit touches, plus an `O(1)` buffer swap.
    pub fn merge_into_wider(&mut self, other: Accumulator) -> Accumulator {
        let mut other = other;
        if other.digit_count() > self.digit_count() {
            core::mem::swap(self, &mut other);
        }
        self.add_accum(&other);
        other
    }

    /// Add or subtract one machine word times `2^shift`: amortized O(1).
    fn add_shifted_word(&mut self, word: u64, negative: bool, shift: u64) {
        if word == 0 {
            return;
        }
        let (digit_shift, bit_shift) =
            (shift / u64::from(DIGIT_BITS), shift % u64::from(DIGIT_BITS));
        let digit_shift = usize::try_from(digit_shift).expect("digit positions fit a usize");
        // At most 96 bits after the sub-digit shift: well inside `i128`.
        let value = i128::from(word) << bit_shift;
        self.add_at(digit_shift, if negative { -value } else { value });
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
        self.bottom = self.bottom.min(pos);
        if pos > self.top + 1 {
            // Every digit strictly between the old top and the landing
            // site is zero (all sit above the old top), and no run at
            // or below the old top can overlap the new one, so the
            // ledger stays disjoint.
            self.zero_runs.insert(self.top, pos);
        }
        let run_start = pos;
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
        self.crop_runs(run_start, pos);
        self.settle_top();
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

    /// Consume the certificate covering the digits just below `t`, if
    /// one exists: returns `lo` with every digit in `(lo, t)` zero,
    /// removing the certificate from the ledger.
    fn consume_run_at(&mut self, t: usize) -> Option<usize> {
        let (&lo, &hi) = self.zero_runs.range(..t).next_back()?;
        if hi >= t {
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
    /// regardless of the held width or the shift. The wide entry points
    /// feed this from a borrowed word slice, so streaming a stored
    /// operand allocates nothing.
    fn apply_limbs<I: Iterator<Item = u64>>(&mut self, limbs: I, negative: bool, shift: u64) {
        let (digit_shift, bit_shift) =
            (shift / u64::from(DIGIT_BITS), shift % u64::from(DIGIT_BITS));
        let digit_shift = usize::try_from(digit_shift).expect("digit positions fit a usize");
        for (i, limb) in limbs.enumerate() {
            touch(1);
            // At most 33 + 31 bits per contribution after the sub-digit
            // shift: well inside the `i128` `add_at` carries from.
            let low = i128::from(limb & DIGIT_MASK) << bit_shift;
            let high = i128::from(limb >> DIGIT_BITS) << bit_shift;
            if low != 0 {
                self.add_at(2 * i + digit_shift, if negative { -low } else { low });
            }
            if high != 0 {
                self.add_at(2 * i + 1 + digit_shift, if negative { -high } else { high });
            }
        }
    }
}

impl Default for Accumulator {
    fn default() -> Accumulator {
        Accumulator::new()
    }
}

/// Pack little-endian base-2^32 digits into a magnitude.
///
/// Consumes the digit buffer so the peak transient during a drain is two
/// width-proportional buffers — the digits and the byte image the
/// magnitude is built from — never three. Byte-denominated so one code
/// path serves every storage word width.
fn magnitude_from_digits(digits: Vec<u32>) -> UBig {
    let bytes: Vec<u8> = digits.iter().flat_map(|d| d.to_le_bytes()).collect();
    drop(digits);
    UBig::from_le_bytes(&bytes)
}

#[cfg(test)]
mod tests;
