//! Cliff-immune signed accumulators: redundant balanced signed-digit
//! arithmetic that keeps machine-word deltas and sign reads amortized O(1)
//! and wide deltas O(operand limbs), on every input sequence.
//!
//! [`Accumulator`] holds a running signed integer — a running total, a
//! running difference of two totals, a running weighted sum — under
//! interleaved adds, subtracts, and sign reads. Every cost this page quotes
//! is a worst-case amortized bound over the whole operation sequence, and
//! every one is *derived*: the two arguments that carry them (the lazy
//! zone, the collapsing sign fold) are below, in full.
//!
//! # The problem: carry cliffs
//!
//! Keep a running total in a normalized big integer and park its value at
//! `2^k`. Adding 1 then subtracting 1, over and over, propagates a full
//! k-bit carry and then a full k-bit borrow per pair: Θ(k) limb work bought
//! by O(1) bits of delta, quadratic over the stream. The cliff is not a
//! quirk of one library; it is the price of *normal form*. A normalized
//! representation spells each value exactly one way, so two values that
//! differ by 1 can differ in every digit — and a type that must always hold
//! the normal spelling must pay the full rewrite every time a small delta
//! crosses a carry boundary. Any workload that cannot control where its
//! totals sit relative to those boundaries inherits the quadratic; an
//! adversarial workload seeks them out.
//!
//! # The representation
//!
//! An accumulator stores little-endian signed digits `dᵢ: i64` denoting
//! `value = Σ dᵢ · 2^(32·i)`, each digit kept in the *lazy zone*
//! `|dᵢ| < 2^33` — twice the digit base, and symmetric about zero. The
//! representation is *redundant*: a value has many spellings, and the type
//! never normalizes. It is *balanced*: digits carry their own signs, so a
//! subtraction is just a negated addition and no borrow machinery exists.
//!
//! A write adds its delta into one digit. If the digit stays in the zone,
//! that is the whole write. If not, the digit *recenters*: it carries
//! `c = (t + 2^31) >> 32` upward and keeps the remainder, which lands in
//! `[−2^31, 2^31)`. A freshly recentered digit therefore needs at least
//! `2^33 − 2^31` of further net drift before it can carry again — so every
//! carry is funded, several times over, by the deltas that drove the digit
//! out of its zone. Machine-word deltas are amortized O(1) digit work; a
//! wide delta splits into one contribution per operand limb, each entering
//! at its own digit position, for O(operand limbs) — independent of how
//! wide the *held* value is, and of any power-of-two shift applied on the
//! way in.
//!
//! Because *every* write recenters, no region of the representation is ever
//! in normal form — hence no boundary an adversarial delta stream can
//! oscillate across at less than the cost the stream itself paid, at any
//! delta width. The obvious halfway design fails exactly there: a two-zone
//! form (a normalized prefix plus a fixed-width lazy window over the low
//! digits) has a boundary at the window's top, and a stream of deltas one
//! code wider than the window forces the normalized prefix through a full
//! carry per delta. Widening the window moves the boundary; only having no
//! normalized region removes it.
//!
//! # Reading the sign
//!
//! The sign of a redundant value is not visible in any one digit — high
//! digits may cancel lower ones. [`Accumulator::sign`] folds digits from
//! the top: at digit index `i` the running partial
//! `s = Σ_{j≥i} dⱼ · 2^(32·(j−i))` is the scanned suffix's exact value in
//! units of `2^(32·i)`, while the unscanned digits below contribute less
//! than `2.01 · 2^(32·i)` in magnitude (a geometric series: each digit is
//! under `2^33`, each level down worth `2^32` times less). So once
//! `|s| ≥ 3`, the suffix dominates everything below — `3 > 2.01` — and the
//! fold stops. While `|s| < 3` it must descend, but the partial stays small
//! enough for machine arithmetic at every step, and if it reaches digit 0
//! the partial *is* the value, exactly.
//!
//! A cancelling prefix — high digits summing to a tiny net value, as built
//! by `+2^k` then `−(2^k − 1)` — forces the fold below the top digit. The
//! fold therefore *collapses* what it scanned: the scanned digits are
//! zeroed and their exact partial is re-deposited at the scan's floor, so
//! the next sign read re-reads none of them. A digit is scanned at most
//! once per write that made it nonzero, so sign reads amortize against the
//! writes that built the prefix — amortized O(1) however sign reads and
//! writes interleave. This is why the sign queries take `&mut self`: they
//! may rewrite the representation. The rewrite is always value-preserving —
//! the digits change, the integer they denote never does.
//!
//! # Domination certificates
//!
//! A comparison between totals of wildly different scales should not cost
//! the wide one's width. [`Accumulator::sign_dominates_at`] returns the
//! sign plus a *certificate*: `decided` is true only when the fold's
//! partial reached `|s| ≥ 3` far enough above a caller-supplied digit floor
//! that no value fitting under the floor could flip the sign — so the
//! caller compares against a word-scale (or floor-scale) adjustment without
//! ever folding it in. [`Accumulator::sign_dominates_word`] is the
//! word-sized special case.
//!
//! # The operations
//!
//! All costs in digit touches, worst-case amortized, derived above.
//!
//! | Operation | Cost |
//! |---|---|
//! | [`add_small`](Accumulator::add_small), [`sub_small`](Accumulator::sub_small), [`add_u64`](Accumulator::add_u64), [`sub_u64`](Accumulator::sub_u64) | amortized O(1) |
//! | [`add_wide`](Accumulator::add_wide), [`sub_wide`](Accumulator::sub_wide), [`add_wide_shl`](Accumulator::add_wide_shl), [`sub_wide_shl`](Accumulator::sub_wide_shl) | O(operand limbs), any held width, any shift |
//! | [`add_base`](Accumulator::add_base), [`sub_base`](Accumulator::sub_base), [`add_base_shl`](Accumulator::add_base_shl), [`sub_base_shl`](Accumulator::sub_base_shl) | the small or wide cost, at the operand's stored width |
//! | [`add_accum`](Accumulator::add_accum), [`sub_accum`](Accumulator::sub_accum), [`add_accum_shl`](Accumulator::add_accum_shl) | O(operand's held digits), any shift |
//! | [`merge_into_wider`](Accumulator::merge_into_wider) | O(narrower operand's held digits) |
//! | [`sign`](Accumulator::sign), [`is_negative`](Accumulator::is_negative), [`sign_dominates_word`](Accumulator::sign_dominates_word), [`sign_dominates_at`](Accumulator::sign_dominates_at) | amortized O(1) |
//! | [`is_zero`](Accumulator::is_zero), [`digit_count`](Accumulator::digit_count) | O(1) |
//! | [`shl`](Accumulator::shl), [`negate`](Accumulator::negate), [`reset`](Accumulator::reset), [`sign_magnitude`](Accumulator::sign_magnitude) | O(held digits) |
//!
//! The `*_base` entry points are generic over [`Magnitude`], the seam for a
//! caller's own stored-magnitude type: the operand reports whether it fits
//! a machine word, and the accumulator dispatches to the small or wide path
//! accordingly.
//!
//! # When not to reach for it
//!
//! The accumulator spends representation slack to buy worst-case bounds;
//! when nothing exploits the slack, simpler types win. If the total fits
//! `i64`/`i128`, use `i64`/`i128`. If the workload is a one-shot sum read
//! once at the end, with no interleaved sign reads, a plain big integer
//! costs one normalization you were going to pay anyway in
//! [`sign_magnitude`]. And this is an accumulator, not a number type: it
//! adds, subtracts, shifts by powers of two, signs, and drains — no
//! multiplication, no division, no ordering between two accumulators
//! except through their difference's sign.
//!
//! # Metering
//!
//! The `touch-meter` feature counts every digit read-modify-write (plus one
//! per operand limb read by a wide operation) into [`touch_meter`], a
//! process-global counter. Digit-touch cost is invisible to heap meters and
//! step counters — the work is wider, not more frequent — so this counter
//! is what a consumer's resource envelopes should pin. Off by default; when
//! on, each touch is one relaxed atomic increment.
//!
//! # Testing
//!
//! Differential proptests drive mixed small/wide operation streams against
//! an exact signed big-integer oracle, comparing the sign after every
//! operation and the full value at periodic snapshots; deterministic
//! adversarial streams (boundary-comb oscillation, wide teeth across a high
//! carry cliff, cancelling-prefix chains) pin the shapes the representation
//! exists to survive.
//!
//! # Traditions, and the name
//!
//! Nothing here is novel so much as assembled. Signed-digit redundancy is
//! Avizienis (1961), and it is the trick inside hardware carry-save adders:
//! spend representation slack, defer carry propagation. Redundant *number
//! representations* as an amortization device are the theme of Okasaki's
//! purely functional data structures. Accumulating wide addends at their
//! own offsets into a fixed-radix array is the Kulisch long accumulator.
//! And unsaturated-limb big-integer pipelines in cryptographic code leave
//! headroom bits in every limb so carries can batch. This crate's
//! contribution is the combination: balanced digits with *no normalized
//! region anywhere*, plus a sign fold that pays for itself by collapsing
//! what it reads.
//!
//! A *suanpan* is the Chinese abacus. Each rod carries two heaven beads
//! (worth five) and five earth beads (worth one): a rod holds 0–15, though
//! a decimal digit needs only 0–9. The slack is the point — a skilled
//! operator parks intermediate values in the redundant range and defers
//! carries until a convenient moment. The Japanese soroban's 1:4 reduction
//! is exactly the normalization this crate refuses. A suanpan rod holds
//! more than a digit so the carries can wait; so do ours.

#![forbid(unsafe_code)]

use core::cmp::Ordering;

use dashu_int::{UBig, Word};

/// Process-global counter of accumulator digit touches.
///
/// Counts one per digit read-modify-write in [`Accumulator`]'s own code
/// (plus one per operand limb read by a wide operation): the unit every
/// cost on the crate page is denominated in. Process-global, relaxed
/// ordering: a metering harness runs one scenario per process and reads
/// the counter only after the metered call returns.
#[cfg(feature = "touch-meter")]
pub mod touch_meter {
    use core::sync::atomic::{AtomicU64, Ordering};

    static TOUCHES: AtomicU64 = AtomicU64::new(0);

    /// Add `n` digit touches to the counter.
    pub(crate) fn record(n: u64) {
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
/// Compiles to nothing without the `touch-meter` feature, so the hot paths
/// call it unconditionally.
#[inline(always)]
fn touch(n: u64) {
    #[cfg(feature = "touch-meter")]
    touch_meter::record(n);
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

/// Stored words per 64-bit limb: 1 where the backend word is 64 bits, 2
/// where it is 32 (wasm32).
///
/// Wide-operand costs are counted in 64-bit limbs, so pairing narrower
/// storage words keeps digit-touch counts identical across targets.
const WORDS_PER_LIMB: usize = (u64::BITS / Word::BITS) as usize;

/// Pack one limb's worth of stored words (the top chunk may be partial).
fn pack_limb(chunk: &[Word]) -> u64 {
    // One face of this cast is a no-op: `Word` is `u64` on 64-bit targets
    // and `u32` on 32-bit ones, and the cast is what compiles on both.
    #[allow(clippy::unnecessary_cast)]
    chunk.iter().enumerate().fold(0u64, |limb, (i, &word)| {
        limb | ((word as u64) << (i as u32 * Word::BITS))
    })
}

/// The 64-bit limbs of a magnitude, least significant first.
///
/// Borrows the stored word slice, so iteration allocates nothing; the top
/// limb zero-pads any missing high words. A zero value has no limbs.
fn limbs(value: &UBig) -> impl Iterator<Item = u64> + '_ {
    value.as_words().chunks(WORDS_PER_LIMB).map(pack_limb)
}

/// An unsigned operand readable at the width it is stored at.
///
/// The seam that lets a caller's own stored-magnitude type drive the
/// accumulator's `*_base` entry points without conversion: the operand
/// reports whether it fits a machine word — the dispatch onto the
/// amortized-O(1) small path — and otherwise lends its full value to the
/// O(operand limbs) wide path. Implementations must agree with themselves:
/// when [`to_word`](Magnitude::to_word) returns `Some(n)`,
/// [`as_wide`](Magnitude::as_wide) must denote that same `n`.
pub trait Magnitude {
    /// The value as a single machine word, or `None` past the `u64` range.
    ///
    /// Must be O(1): this is the dispatch read the small path's amortized
    /// cost accounting assumes is free.
    fn to_word(&self) -> Option<u64>;

    /// The full value, borrowed for the wide path.
    fn as_wide(&self) -> &UBig;
}

impl Magnitude for UBig {
    fn to_word(&self) -> Option<u64> {
        u64::try_from(self).ok()
    }

    fn as_wide(&self) -> &UBig {
        self
    }
}

/// A running signed integer over redundant balanced base-2^32 digits.
///
/// Deltas are added or subtracted at machine-word or arbitrary width; the
/// sign is readable at any point in amortized O(1); one low-to-high carry
/// pass converts the final value to a normalized magnitude. The crate docs
/// carry the representation and both cost arguments. Sign queries take
/// `&mut self` because they may collapse a scanned cancelling prefix; the
/// rewrite never changes the value the digits denote.
#[derive(Debug, Clone)]
pub struct Accumulator {
    /// Little-endian signed digits: `value = Σ digits[i] · 2^(32·i)`, every
    /// digit in the lazy zone `|d| < 2^33`.
    digits: Vec<i64>,
    /// Index of the highest nonzero digit; 0 when the value is zero. Digits
    /// above it are all zero.
    top: usize,
}

impl Accumulator {
    /// Create an accumulator holding zero.
    pub fn new() -> Accumulator {
        Accumulator {
            digits: vec![0],
            top: 0,
        }
    }

    /// Add a signed machine-word delta: amortized O(1).
    pub fn add_small(&mut self, delta: i64) {
        if delta != 0 {
            self.add_at(0, i128::from(delta));
        }
    }

    /// Subtract a signed machine-word delta: amortized O(1).
    pub fn sub_small(&mut self, delta: i64) {
        if delta != 0 {
            self.add_at(0, -i128::from(delta));
        }
    }

    /// Add an unsigned machine-word delta: amortized O(1).
    pub fn add_u64(&mut self, delta: u64) {
        if delta != 0 {
            self.add_at(0, i128::from(delta));
        }
    }

    /// Subtract an unsigned machine-word delta: amortized O(1).
    pub fn sub_u64(&mut self, delta: u64) {
        if delta != 0 {
            self.add_at(0, -i128::from(delta));
        }
    }

    /// Add a wide delta: O(operand limbs), paid by the operand's own width.
    pub fn add_wide(&mut self, delta: &UBig) {
        self.apply_limbs(limbs(delta), false, 0);
    }

    /// Subtract a wide delta: O(operand limbs), paid by the operand's own
    /// width.
    pub fn sub_wide(&mut self, delta: &UBig) {
        self.apply_limbs(limbs(delta), true, 0);
    }

    /// Add `delta · 2^shift`: O(operand limbs), independent of the shift.
    ///
    /// The scaled entry point behind weighted folds — a summand carrying
    /// its own exponent, such as a value weighted by a dyadic interval
    /// width or a numerator aligned to a larger scale. The shift routes
    /// each operand limb to its target digit position directly, so a wide
    /// shift costs no more than an unshifted add of the same operand and
    /// no shifted copy of the operand ever exists.
    pub fn add_wide_shl(&mut self, delta: &UBig, shift: u64) {
        self.apply_limbs(limbs(delta), false, shift);
    }

    /// Subtract `delta · 2^shift`: O(operand limbs), independent of the
    /// shift.
    ///
    /// The subtractive twin of
    /// [`add_wide_shl`](Accumulator::add_wide_shl).
    pub fn sub_wide_shl(&mut self, delta: &UBig, shift: u64) {
        self.apply_limbs(limbs(delta), true, shift);
    }

    /// Add a stored magnitude times `2^shift`, at the width it is stored
    /// at: O(operand limbs), independent of the shift.
    pub fn add_base_shl<M: Magnitude>(&mut self, delta: &M, shift: u64) {
        match delta.to_word() {
            Some(0) => {}
            Some(n) => self.add_shifted_word(n, false, shift),
            None => self.add_wide_shl(delta.as_wide(), shift),
        }
    }

    /// Subtract a stored magnitude times `2^shift`: O(operand limbs),
    /// independent of the shift.
    pub fn sub_base_shl<M: Magnitude>(&mut self, delta: &M, shift: u64) {
        match delta.to_word() {
            Some(0) => {}
            Some(n) => self.add_shifted_word(n, true, shift),
            None => self.apply_limbs(limbs(delta.as_wide()), true, shift),
        }
    }

    /// Add another accumulator's held value times `2^shift` into this one:
    /// O(the operand's held digits), independent of the shift.
    ///
    /// The merge move of a weighted fold: a finished partial sum lands in
    /// its parent's accumulator at the exponent gap between their scales,
    /// each digit routed to its target position directly.
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

    /// Scale the held value by `2^shift` in place: O(held digits).
    ///
    /// Rewrites every held digit to its shifted position. A fold that
    /// anchors its accumulator at a running maximum exponent pays one
    /// shift per exponent raise, each bounded by the operand that raised
    /// it.
    pub fn shl(&mut self, shift: u64) {
        if shift == 0 || (self.top == 0 && self.digits[0] == 0) {
            return;
        }
        let held = core::mem::take(self);
        self.add_accum_shl(&held, shift);
    }

    /// Add another accumulator's held value into this one: O(the
    /// operand's held digits).
    ///
    /// Reading every held digit of `other` is the cost discipline to
    /// watch: fold an operand into a longer-lived accumulator when the
    /// operand is dying, or when something else already prices reads of
    /// it — never repeatedly from a loop that pays nothing for it.
    pub fn add_accum(&mut self, other: &Accumulator) {
        self.add_accum_shl(other, 0);
    }

    /// Subtract another accumulator's held value from this one: O(the
    /// operand's held digits).
    ///
    /// The subtractive twin of [`add_accum`](Accumulator::add_accum):
    /// each operand digit lands negated at its own position (a balanced
    /// digit's negation is a balanced digit, so nothing carries beyond
    /// `add_at`'s own recentering).
    pub fn sub_accum(&mut self, other: &Accumulator) {
        for (i, &digit) in other.digits[..=other.top].iter().enumerate() {
            touch(1);
            if digit != 0 {
                self.add_at(i, -i128::from(digit));
            }
        }
    }

    /// Negate the held value in place: O(held digits).
    ///
    /// Digit-wise: a balanced digit's negation stays in the lazy zone, so
    /// no carries move.
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
    pub fn reset(&mut self) {
        for digit in &mut self.digits[..=self.top] {
            touch(1);
            *digit = 0;
        }
        self.top = 0;
    }

    /// The sign, plus whether the held magnitude certainly dominates any
    /// machine-word adjustment: amortized O(1), collapsing like
    /// [`sign`](Accumulator::sign).
    ///
    /// [`sign_dominates_at`](Accumulator::sign_dominates_at) with floor 1:
    /// a machine word holds at most two digits, so `decided` means no
    /// word-scale operand folded in could flip the sign. A comparison
    /// against a word-scale adjustment reads this instead of folding, so
    /// a wide running total is never touched across its width by a cheap
    /// comparison.
    pub fn sign_dominates_word(&mut self) -> (Ordering, bool) {
        self.sign_dominates_at(1)
    }

    /// The sign, plus whether the held magnitude certainly dominates any
    /// value held in digits `0..=floor`: amortized O(1), collapsing like
    /// [`sign`](Accumulator::sign).
    ///
    /// Returns `(sign, decided)` where `decided` is true only when the
    /// sign fold's partial reached `|s| ≥ 3` at digit index `floor + 2`
    /// or higher. At decision index `i` the unscanned digits below
    /// contribute under `2.01 · 2^(32·i)` (the crate docs' domination
    /// bound), so `|value| ≥ 0.99 · 2^(32·i)`; an operand with top digit
    /// index at most `floor` holds under `2.01 · 2^(32·(floor + 1))`
    /// (the same geometric bound one level up), and
    /// `0.99 · 2^(32·(floor + 2)) > 2.01 · 2^(32·(floor + 1))` by a
    /// factor over `2^30` — so folding any such operand in could flip
    /// neither the sign nor which magnitude is larger. Scale-disparate
    /// comparisons read this instead of folding, so a wide accumulator
    /// is never touched across its width by a comparison a top index
    /// already decides.
    pub fn sign_dominates_at(&mut self, floor: usize) -> (Ordering, bool) {
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
        let decided = partial.abs() >= SIGN_DECIDED && index >= floor + 2;
        if index < self.top {
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
        (partial.cmp(&0), decided)
    }

    /// Whether the held value is zero, without any scan or rewrite.
    ///
    /// Exact: every write keeps `top` on the highest nonzero digit, so a
    /// zero value is always the single zero digit at index 0.
    pub fn is_zero(&self) -> bool {
        self.top == 0 && self.digits[0] == 0
    }

    /// The number of digits up to and including the highest nonzero one.
    ///
    /// The size a merge or a scaled add will read: a caller balancing
    /// fold costs compares it against a spill threshold, or merges the
    /// smaller operand into the larger.
    pub fn digit_count(&self) -> usize {
        self.top + 1
    }

    /// Fold `other`'s held value into this one, always folding the
    /// buffer holding fewer digits into the wider one: O(the narrower
    /// operand's held digits) plus an O(1) buffer swap.
    ///
    /// The result lands in whichever buffer is wider and `self` takes
    /// it; the drained buffer (still holding its stale digits) is
    /// returned for the caller's pool. The min-into-max merge move: a
    /// fold that nothing else prices must deposit its result in a buffer
    /// at least as wide as the wider operand and read only the narrower
    /// one, so the digits a dying operand holds fund the fold that
    /// consumes it.
    pub fn merge_into_wider(&mut self, mut other: Accumulator) -> Accumulator {
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

    /// Add a stored magnitude, at the width it is stored at.
    ///
    /// A word-scale operand takes the amortized-O(1) small path, a wider
    /// one the O(operand limbs) wide path; [`Magnitude`] is the dispatch.
    pub fn add_base<M: Magnitude>(&mut self, delta: &M) {
        match delta.to_word() {
            Some(n) => self.add_u64(n),
            None => self.add_wide(delta.as_wide()),
        }
    }

    /// Subtract a stored magnitude, at the width it is stored at.
    ///
    /// The subtractive twin of [`add_base`](Accumulator::add_base).
    pub fn sub_base<M: Magnitude>(&mut self, delta: &M) {
        match delta.to_word() {
            Some(n) => self.sub_u64(n),
            None => self.sub_wide(delta.as_wide()),
        }
    }

    /// The sign of the held value, relative to zero: amortized O(1).
    ///
    /// Folds digits from the top and decides at running partial `|s| ≥ 3`
    /// (the crate docs' domination bound). When the fold had to descend —
    /// a cancelling prefix — the scanned digits are collapsed to their
    /// partial at the scan's floor, so the scan is paid at most once per
    /// write (the crate docs' amortization argument). The rewrite is
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
    /// [`sign`](Accumulator::sign).
    pub fn is_negative(&mut self) -> bool {
        self.sign() == Ordering::Less
    }

    /// The held value as a sign and a normalized magnitude.
    ///
    /// One low-to-high pass with a signed carry: O(held digits). The
    /// magnitude is zero exactly when the sign is [`Ordering::Equal`].
    pub fn sign_magnitude(&self) -> (Ordering, UBig) {
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

    /// Add `value` (any sign, any `i128` magnitude) into the digit at
    /// `pos`, carrying upward until every touched digit is in the zone.
    ///
    /// O(value bits / 32) digit touches, amortized O(1) for word-scale
    /// values.
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

    /// Apply a little-endian 64-bit limb stream scaled by `2^shift`.
    ///
    /// Digit-aligned: each limb lands as two independent 32-bit
    /// contributions at its own shifted position, so a wide operand costs
    /// O(its limbs) regardless of the held width or the shift. The wide
    /// entry points feed this from a borrowed word slice, so streaming a
    /// stored operand allocates nothing.
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
