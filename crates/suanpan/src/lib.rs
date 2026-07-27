//! Cliff-immune signed accumulators: redundant balanced signed digits
//! with no carry cliffs anywhere — machine-word deltas and sign reads
//! amortized O(1), wide deltas amortized O(operand limbs), on every input
//! sequence.
//!
//! [`Accumulator`] holds a running signed integer — a running total, a
//! running difference of two totals, a running weighted sum — under
//! interleaved adds, subtracts, and sign reads:
//!
//! ```
//! use core::cmp::Ordering;
//! use suanpan::{Accumulator, UBig};
//!
//! let mut acc = Accumulator::new();
//! acc.add_wide(&(UBig::from(1u8) << 512usize)); // park a wide total on a carry boundary
//! for _ in 0..1_000 {
//!     acc.sub_small(1);                         // oscillate across it: amortized O(1) each
//!     assert_eq!(acc.sign(), Ordering::Greater);
//!     acc.add_small(1);
//! }
//! let (sign, magnitude) = acc.sign_magnitude(); // one carry pass, at the very end
//! assert_eq!(sign, Ordering::Greater);
//! assert_eq!(magnitude, UBig::from(1u8) << 512usize);
//! ```
//!
//! Every cost this page quotes holds on adversarial input sequences —
//! the amortized bounds are worst-case over the whole sequence, not
//! average-case claims — and every one is *derived*: the two arguments
//! that carry them (the lazy zone, the collapsing sign fold) are below,
//! in full.
//!
//! # The problem: carry cliffs
//!
//! Keep a running total in a normalized big integer and park its value at
//! `2^k − 1`. Adding 1 then subtracting it back, over and over, propagates
//! a full k-bit carry and then a full k-bit borrow per pair: Θ(k) limb
//! work bought by O(1) bits of delta — and when the stream itself built
//! the k-bit total, quadratic in the stream's length. The cliff is not a
//! quirk of one library; it is the price of *normal form*. A normalized
//! representation spells each value exactly one way, so two values that
//! differ by 1 can differ in every digit — and a type that must always
//! hold the normal spelling must pay the full rewrite every time a small
//! delta crosses a carry boundary. Any workload whose deltas mix signs
//! near such a boundary inherits the cost; an adversarial workload seeks
//! it out.
//!
//! # The representation
//!
//! An accumulator stores little-endian signed digits `dᵢ: i64` denoting
//! `value = Σ dᵢ · 2^(32·i)`, each digit kept in the *lazy zone*
//! `|dᵢ| < 2^33` — twice the digit base, and symmetric about zero. The
//! representation is *redundant*: a value has many spellings, no
//! operation requires the normal one, and nothing eagerly normalizes. It
//! is *balanced*: digits carry their own signs, so a subtraction is just
//! a negated addition and no borrow machinery exists.
//!
//! A write adds its delta into one digit, forming the sum `t` in wider
//! (128-bit) intermediate arithmetic so nothing overflows. If `t` is in
//! the zone, it becomes the digit and that is the whole write. If not,
//! the digit *recenters*: it carries `c = (t + 2^31) >> 32` upward (an
//! arithmetic shift) and keeps the remainder `t − c·2^32`, which lands in
//! `[−2^31, 2^31)`. Two facts make this cheap \[derived\]: a freshly
//! recentered digit must absorb at least `2^33 − 2^31` of further net
//! inflow before it can carry again, and a carry chain attenuates fast —
//! the first carry out of a word-scale write is at most about `2^32`, and
//! the next is already a handful of units, tiny against the inflow the
//! digit above needs before it carries on. So sustained carry traffic
//! thins out geometrically with height, and the total carry work is
//! dominated by the deltas that entered below. The write bounds are
//! amortized per call as well as per delta: a single write can be caught
//! repaying a run of digits that earlier writes parked near the zone's
//! edge, but never more than those writes prepaid.
//!
//! Machine-word deltas are therefore amortized O(1) digit work. A wide
//! delta enters limb by limb — throughout this page a *limb* is one
//! 64-bit word of the operand's value, independent of the backend's
//! internal word size — each limb landing as two contributions at the
//! digit positions it spans, for amortized O(operand limbs) total:
//! independent of how wide the *held* value is, and of any power-of-two
//! shift applied on the way in.
//!
//! Because *every* write recenters, no region of the representation is
//! ever kept in normal form — hence no boundary an adversarial delta
//! stream can oscillate across at less than the cost the stream itself
//! paid, at any delta width. The obvious halfway design fails exactly
//! there: a two-zone form (a normalized prefix plus a fixed-width lazy
//! window over the low digits) has a boundary at the window's top, and a
//! stream of deltas one digit wider than the window forces the normalized
//! prefix through a full carry per delta. Widening the window moves the
//! boundary; only having no normalized region removes it.
//!
//! # Reading the sign
//!
//! The sign of a redundant value is not visible in any one digit — high
//! digits may cancel lower ones. [`Accumulator::sign`] folds digits from
//! the top: at digit index `i` the running partial
//! `s = Σ_{j≥i} dⱼ · 2^(32·(j−i))` is the scanned suffix's exact value in
//! units of `2^(32·i)`, while the unscanned digits below contribute less
//! than `2.01 · 2^(32·i)` in magnitude (a geometric series — each digit
//! under `2^33`, each level down worth `2^32` times less — summing to
//! just over `2 · 2^(32·i)`; `2.01` is that bound rounded up for slack).
//! So once `|s| ≥ 3`, the suffix dominates everything below —
//! `3 > 2.01` — and the fold stops. While `|s| < 3` it must descend, but
//! the partial stays small enough for machine arithmetic at every step,
//! and if it reaches digit 0 the partial *is* the value, exactly.
//!
//! A cancelling prefix — high digits summing to a tiny net value, as built
//! by `+2^k` then `−(2^k − 1)` — forces the fold below the top digit. The
//! fold therefore *collapses* what it scanned: the scanned digits are
//! zeroed and their exact partial is re-deposited at the scan's floor
//! (recentering upward like any write when the partial exceeds the zone),
//! so the next sign read re-reads none of them — the re-deposited digit is
//! that next fold's first step, inside its O(1) budget. A digit is scanned
//! at most once per write that made it nonzero, so sign reads amortize
//! against the writes that built the prefix — amortized O(1) however sign
//! reads and writes interleave. This is why the sign queries take
//! `&mut self`: they may rewrite the representation. The rewrite is always
//! value-preserving — the digits change, the integer they denote never
//! does.
//!
//! # Domination certificates
//!
//! A comparison between totals of wildly different scales should not cost
//! the wide one's width. [`Accumulator::sign_dominates_at`] returns the
//! (always exact) sign plus a *certificate*: `decided = true` guarantees
//! `sign(v + a) = sign(v)` and `|v| > |a|` for every adjustment `a` with
//! `|a| < 2^(32·(floor + 1))` — and moreover for any accumulator held in
//! digits `0..=floor`: its redundant spelling can reach
//! `2.01 · 2^(32·(floor + 1))`, and the decision margin covers that too.
//! So the caller compares against anything at or below the floor's scale
//! without ever folding it in:
//!
//! ```
//! use core::cmp::Ordering;
//! use suanpan::{Accumulator, UBig};
//!
//! let mut watermark = Accumulator::new();
//! watermark.add_wide(&(UBig::from(1u8) << 300usize));
//! // Could any adjustment below 2^128 flip the watermark's sign?
//! // floor = 128.div_ceil(32) − 1 = 3: certainty without a wide fold.
//! let (sign, decided) = watermark.sign_dominates_at(3);
//! assert_eq!((sign, decided), (Ordering::Greater, true));
//! ```
//!
//! For `u64`-scale adjustments,
//! [`sign_dominates_word`](Accumulator::sign_dominates_word) is the
//! shorthand.
//!
//! # The operations
//!
//! All costs in digit touches, derived above. *Amortized* bounds hold
//! over the whole operation sequence — one write can be caught repaying
//! carries that earlier writes parked near the zone's edge, never more
//! than they prepaid; unmarked rows are worst-case per call.
//!
//! | Operation | Cost |
//! |---|---|
//! | [`add_small`](Accumulator::add_small), [`sub_small`](Accumulator::sub_small), [`add_u64`](Accumulator::add_u64), [`sub_u64`](Accumulator::sub_u64) | amortized O(1) |
//! | [`add_wide`](Accumulator::add_wide), [`sub_wide`](Accumulator::sub_wide) | amortized O(operand limbs), whatever the held width |
//! | [`add_wide_shl`](Accumulator::add_wide_shl), [`sub_wide_shl`](Accumulator::sub_wide_shl) | amortized O(operand limbs), independent of the shift |
//! | [`add_base`](Accumulator::add_base), [`sub_base`](Accumulator::sub_base) | word-scale: amortized O(1); wide: amortized O(operand limbs) |
//! | [`add_base_shl`](Accumulator::add_base_shl), [`sub_base_shl`](Accumulator::sub_base_shl) | as [`add_base`](Accumulator::add_base)/[`sub_base`](Accumulator::sub_base), at any shift |
//! | [`add_accum`](Accumulator::add_accum), [`sub_accum`](Accumulator::sub_accum) | amortized O(operand's held digits) |
//! | [`add_accum_shl`](Accumulator::add_accum_shl), [`sub_accum_shl`](Accumulator::sub_accum_shl) | amortized O(operand's held digits), independent of the shift |
//! | [`merge_into_wider`](Accumulator::merge_into_wider) | amortized O(narrower operand's held digits) |
//! | [`sign`](Accumulator::sign), [`is_negative`](Accumulator::is_negative), [`sign_dominates_word`](Accumulator::sign_dominates_word), [`sign_dominates_at`](Accumulator::sign_dominates_at) | amortized O(1) |
//! | [`is_zero`](Accumulator::is_zero), [`digit_count`](Accumulator::digit_count) | O(1) |
//! | [`shl`](Accumulator::shl), [`negate`](Accumulator::negate), [`reset`](Accumulator::reset), [`sign_magnitude`](Accumulator::sign_magnitude) | O(held digits) |
//!
//! Digit touches are shift-independent; memory is not. A shifted entry
//! point grows the digit buffer to cover the shifted position, so memory
//! is O(shift / 32) plus the operand's own digits.
//!
//! The `*_base` entry points (*base*: the operand in its stored, base
//! form, whatever type holds it) are generic over [`Magnitude`], the seam
//! for a caller's own stored-magnitude type: the operand reports whether
//! it fits a machine word, and the accumulator dispatches to the small or
//! wide path accordingly. There is no from-value constructor: build with
//! [`new`](Accumulator::new) (or `Default`) and a single `add_*` call,
//! read out with [`sign_magnitude`](Accumulator::sign_magnitude).
//!
//! # When not to reach for it
//!
//! The accumulator spends representation slack to buy worst-case bounds;
//! when nothing exploits the slack, simpler types win. If the total fits
//! `i64`/`i128`, use `i64`/`i128`. If the deltas never change sign, a
//! plain big integer is already amortized O(1) per delta (the binary
//! counter argument: each carry clears a bit an earlier increment set, so
//! carries never outnumber increments) and needs no slack. The
//! accumulator earns its keep when deltas mix signs — when the total can
//! be driven onto a carry boundary and oscillated — or when sign reads
//! interleave with cancelling updates. And this is an accumulator, not a
//! number type: it adds, subtracts, scales by powers of two (left only —
//! a right shift would need normalization), reads its sign, and converts
//! out through [`sign_magnitude`](Accumulator::sign_magnitude) — no
//! multiplication, no division, and no ordering between two accumulators
//! except by subtracting one from the other and reading the difference's
//! sign (subtract from a [`clone`](Clone::clone) when the receiver's
//! value must survive the comparison) — or, when the scales differ
//! wildly, a domination certificate
//! ([`sign_dominates_at`](Accumulator::sign_dominates_at) with
//! `floor = other.digit_count() − 1`) that decides without folding.
//!
//! # Metering
//!
//! The `touch-meter` feature counts every digit read-modify-write (plus
//! one per operand limb read by a wide operation) into the
//! [`touch_meter`] module's process-global counter. Digit-touch cost is
//! invisible to heap meters and step counters — the work is wider, not
//! more frequent — so this counter is what a consumer's resource
//! envelopes should pin. Off by default, and without the feature the
//! module is absent and the counting compiles to nothing; with it, each
//! touch is one relaxed atomic increment.
//!
//! # Interop
//!
//! [`UBig`] is `dashu_int::UBig` (compiled against `dashu-int` 0.5;
//! bumping that dependency is a breaking change to this crate's API),
//! re-exported so callers can name exactly the type this crate compiled
//! against. The crate requires `std`; no `no_std` build is offered.
//! [`Accumulator`] is `Clone`, `Default`, `Debug`, and `Send + Sync` —
//! though `Sync` buys less than usual: every amortized-O(1) sign query
//! takes `&mut self`, so behind a shared reference only
//! [`is_zero`](Accumulator::is_zero),
//! [`digit_count`](Accumulator::digit_count), and the O(held digits)
//! [`sign_magnitude`](Accumulator::sign_magnitude) are callable — wrap in
//! a lock for shared sign reads. It is deliberately not `PartialEq`: two
//! spellings of one value would compare unequal, so compare by
//! subtracting and reading the difference's sign. `touch-meter` is the
//! crate's only feature.
//!
//! # Testing
//!
//! Differential proptests drive mixed small/wide operation streams
//! against an exact signed big-integer oracle, comparing the sign after
//! every operation and the full value at periodic snapshots; deterministic
//! adversarial streams pin the shapes the representation exists to
//! survive — the boundary comb (a ±1 oscillation parked on a `2^k` carry
//! boundary), wide teeth (±2^w strides across a higher boundary), and
//! cancelling-prefix chains (repeated falls from `2^k` to 1 and back,
//! each forcing the sign fold below the top digit).
//!
//! # Traditions, and the name
//!
//! Nothing here is novel so much as assembled. Signed-digit redundancy is
//! Avizienis (1961), and it is the trick inside hardware carry-save
//! adders: spend representation slack, defer carry propagation. Redundant
//! *number representations* as an amortization device are the theme of
//! Okasaki's purely functional data structures. Accumulating wide addends
//! at their own offsets into a fixed-radix array is the Kulisch long
//! accumulator. And unsaturated-limb big-integer pipelines in
//! cryptographic code leave headroom bits in every limb so carries can
//! batch. This crate's contribution is the combination: balanced digits
//! with *no normalized region anywhere*, plus a sign fold that pays for
//! itself by collapsing what it reads.
//!
//! A *suanpan* is the Chinese abacus. Each rod carries two heaven beads
//! (worth five) and five earth beads (worth one): a rod holds 0–15,
//! though a decimal digit needs only 0–9. The slack is the point — a
//! skilled operator parks intermediate values in the redundant range and
//! defers carries until a convenient moment. The Japanese soroban keeps
//! one heaven and four earth beads — a rod holds exactly 0–9, no slack —
//! which is exactly the normalization this crate refuses. A suanpan rod
//! holds more than a digit so the carries can wait; so do ours.

#![forbid(unsafe_code)]

use core::cmp::Ordering;

pub use dashu_int::UBig;
use dashu_int::Word;

/// Process-global counter of accumulator digit touches.
///
/// Present only with the `touch-meter` cargo feature. Counts one per
/// digit read-modify-write in [`Accumulator`]'s own code (a sign-fold
/// step counts one touch whether or not it rewrites the digit, and a
/// wide operation adds one per operand limb read): the unit every cost
/// on the crate page is denominated in. Because the counter is
/// process-global with relaxed ordering, readings are meaningful only
/// when metered scenarios run serially — [`reset`](touch_meter::reset)
/// between them, read after the metered call returns; a default-parallel
/// test runner interleaves scenarios into one count.
#[cfg(feature = "touch-meter")]
pub mod touch_meter {
    use core::sync::atomic::{AtomicU64, Ordering};

    static TOUCHES: AtomicU64 = AtomicU64::new(0);

    /// Add `n` digit touches to the counter.
    pub(crate) fn record(n: u64) {
        TOUCHES.fetch_add(n, Ordering::Relaxed);
    }

    /// The digit touches recorded since process start or the last
    /// [`reset`], whichever is later.
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
/// wide path. Signedness stays with the caller: route the operand's sign
/// to the `add_*` or `sub_*` entry point. Implementors necessarily own a
/// [`UBig`] to lend from [`as_wide`](Magnitude::as_wide); a type whose
/// values always fit a machine word has nothing to lend and should call
/// [`add_u64`](Accumulator::add_u64)/[`sub_u64`](Accumulator::sub_u64)
/// directly instead of implementing the trait. Implementations must agree
/// with themselves: when [`to_word`](Magnitude::to_word) returns
/// `Some(n)`, [`as_wide`](Magnitude::as_wide) must denote that same `n`.
pub trait Magnitude {
    /// The value as a single machine word, or `None` past the `u64` range.
    ///
    /// Must be O(1): this is the dispatch read the small path's amortized
    /// cost accounting assumes is free. Returning `None` for a value that
    /// does fit a word is permitted — it forfeits the small path, never
    /// correctness.
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
/// pass ([`sign_magnitude`](Accumulator::sign_magnitude)) converts the
/// held value to a normalized magnitude. The crate docs carry the
/// representation and both cost arguments. Sign queries take `&mut self`
/// because they may collapse a scanned cancelling prefix; the rewrite
/// never changes the value the digits denote.
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
    ///
    /// The signed (`i64`) twin of [`add_u64`](Accumulator::add_u64).
    /// Exact over the full `i64` range: the delta widens before any carry
    /// arithmetic, so even `i64::MIN` lands intact.
    pub fn add_small(&mut self, delta: i64) {
        if delta != 0 {
            self.add_at(0, i128::from(delta));
        }
    }

    /// Subtract a signed machine-word delta: amortized O(1).
    ///
    /// Exact over the full `i64` range, `i64::MIN` included (the delta
    /// widens before it is negated).
    pub fn sub_small(&mut self, delta: i64) {
        if delta != 0 {
            self.add_at(0, -i128::from(delta));
        }
    }

    /// Add an unsigned machine-word delta: amortized O(1).
    ///
    /// Use this over [`add_small`](Accumulator::add_small) when the delta
    /// may exceed `i64::MAX`; otherwise the two are interchangeable.
    pub fn add_u64(&mut self, delta: u64) {
        if delta != 0 {
            self.add_at(0, i128::from(delta));
        }
    }

    /// Subtract an unsigned machine-word delta: amortized O(1).
    ///
    /// Use this over [`sub_small`](Accumulator::sub_small) when the delta
    /// may exceed `i64::MAX`; otherwise the two are interchangeable.
    pub fn sub_u64(&mut self, delta: u64) {
        if delta != 0 {
            self.add_at(0, -i128::from(delta));
        }
    }

    /// Add a wide delta: amortized O(operand limbs), a limb being one
    /// 64-bit word of the operand — the cost scales with the operand's
    /// width, never the held value's.
    pub fn add_wide(&mut self, delta: &UBig) {
        self.apply_limbs(limbs(delta), false, 0);
    }

    /// Subtract a wide delta: amortized O(operand limbs), scaling with
    /// the operand's width, never the held value's.
    pub fn sub_wide(&mut self, delta: &UBig) {
        self.apply_limbs(limbs(delta), true, 0);
    }

    /// Add a stored magnitude, at the width it is stored at.
    ///
    /// A word-scale operand takes the amortized-O(1) small path, a wider
    /// one the amortized-O(operand limbs) wide path; [`Magnitude`] is the
    /// dispatch.
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

    /// Add `delta · 2^shift`: amortized O(operand limbs) digit touches,
    /// independent of the shift.
    ///
    /// The scaled entry point behind weighted folds — a summand carrying
    /// its own exponent, such as a value weighted by a dyadic interval
    /// width or a numerator aligned to a larger scale. The shift routes
    /// each operand limb to its target digit position directly, so no
    /// shifted copy of the operand ever exists. Memory is the exception
    /// to shift-independence: the digit buffer grows to cover the shifted
    /// position, O(shift / 32) plus the operand's digits.
    ///
    /// # Panics
    ///
    /// Panics if the shifted digit position `shift / 32` overflows
    /// `usize` — possible only on targets narrower than 64 bits (past
    /// `shift = 2^37` on a 32-bit one). On 64-bit targets every `u64`
    /// shift fits, and an enormous one fails at allocation instead, like
    /// any collection asked to grow to `shift / 32` entries.
    pub fn add_wide_shl(&mut self, delta: &UBig, shift: u64) {
        self.apply_limbs(limbs(delta), false, shift);
    }

    /// Subtract `delta · 2^shift`: amortized O(operand limbs) digit
    /// touches, independent of the shift.
    ///
    /// The subtractive twin of
    /// [`add_wide_shl`](Accumulator::add_wide_shl), with the same memory
    /// note.
    ///
    /// # Panics
    ///
    /// As [`add_wide_shl`](Accumulator::add_wide_shl): a shifted digit
    /// position past `usize` panics.
    pub fn sub_wide_shl(&mut self, delta: &UBig, shift: u64) {
        self.apply_limbs(limbs(delta), true, shift);
    }

    /// Add a stored magnitude times `2^shift`, at the width it is stored
    /// at.
    ///
    /// The same width dispatch as [`add_base`](Accumulator::add_base),
    /// with digit touches independent of the shift and
    /// [`add_wide_shl`](Accumulator::add_wide_shl)'s memory note.
    ///
    /// # Panics
    ///
    /// As [`add_wide_shl`](Accumulator::add_wide_shl): a shifted digit
    /// position past `usize` panics.
    pub fn add_base_shl<M: Magnitude>(&mut self, delta: &M, shift: u64) {
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
    /// [`add_base_shl`](Accumulator::add_base_shl): the same width
    /// dispatch, shift-independent digit touches, and memory note.
    ///
    /// # Panics
    ///
    /// As [`add_wide_shl`](Accumulator::add_wide_shl): a shifted digit
    /// position past `usize` panics.
    pub fn sub_base_shl<M: Magnitude>(&mut self, delta: &M, shift: u64) {
        match delta.to_word() {
            Some(0) => {}
            Some(n) => self.add_shifted_word(n, true, shift),
            None => self.apply_limbs(limbs(delta.as_wide()), true, shift),
        }
    }

    /// Add another accumulator's held value into this one: amortized
    /// O(the operand's held digits).
    ///
    /// The cost discipline to watch: folding a long-lived accumulator in
    /// from a loop re-reads all of its digits every iteration — O(n) per
    /// pass, quadratic over the loop. Fold an operand in once, when it is
    /// about to be discarded or has served its purpose.
    pub fn add_accum(&mut self, other: &Accumulator) {
        self.add_accum_shl(other, 0);
    }

    /// Subtract another accumulator's held value from this one: amortized
    /// O(the operand's held digits).
    ///
    /// The subtractive twin of [`add_accum`](Accumulator::add_accum),
    /// with the same once-not-per-iteration cost discipline.
    pub fn sub_accum(&mut self, other: &Accumulator) {
        self.sub_accum_shl(other, 0);
    }

    /// Add another accumulator's held value times `2^shift` into this one:
    /// amortized O(the operand's held digits) digit touches, independent
    /// of the shift.
    ///
    /// The merge move of a weighted fold: a finished partial sum lands in
    /// its parent's accumulator at the exponent gap between their scales,
    /// each digit routed to its target position directly. The digit
    /// buffer grows to cover the shifted positions (memory O(shift / 32)
    /// plus the operand's digits).
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

    /// The sign of the held value — `value.cmp(&0)`, so `Less` means
    /// negative: amortized O(1).
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

    /// The sign, plus whether the held magnitude certainly dominates any
    /// machine-word adjustment: amortized O(1), collapsing like
    /// [`sign`](Accumulator::sign).
    ///
    /// Returns `(sign, decided)`. Equivalent to
    /// [`sign_dominates_at`](Accumulator::sign_dominates_at)`(1)`: a
    /// `u64` spans at most two digit positions. A comparison against a
    /// word-scale adjustment reads this instead of folding, so a wide
    /// running total is never touched across its width by a cheap
    /// comparison.
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
    /// in digits `0..=floor` (its redundant spelling can reach
    /// `2.01 · 2^(32·(floor + 1))`; the margin covers that too). To cover
    /// an adjustment below `2^b`, pass `floor = b.div_ceil(32) − 1`; to
    /// compare against another accumulator, `floor = its digit_count − 1`.
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

    /// Whether the held value is *canonically* zero, without any scan or
    /// rewrite: O(1).
    ///
    /// One-sided: `true` guarantees the value is zero, but a zero built
    /// out of cancelling nonzero digits reads `false` until a sign read
    /// collapses it — [`sign`](Accumulator::sign)`() == Equal` is the
    /// exact zero test, and after it this reads `true`:
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
    /// assert!(!acc.is_zero());                 // zero, but spelled redundantly
    /// assert_eq!(acc.sign(), Ordering::Equal); // the exact test — and it collapses,
    /// assert!(acc.is_zero());                  // so the spelling is now canonical
    /// ```
    pub fn is_zero(&self) -> bool {
        self.top == 0 && self.digits[0] == 0
    }

    /// The number of digits up to and including the highest nonzero one;
    /// at least 1 (a zero accumulator counts its one zero digit): O(1).
    ///
    /// Exact, not a watermark: a write that zeroes the top digit pays the
    /// scan down to the next nonzero one inside that write's own budget.
    /// This is the size a merge or a scaled add of this accumulator will
    /// read — a caller balancing fold costs compares counts and merges
    /// the smaller operand into the larger, as
    /// [`merge_into_wider`](Accumulator::merge_into_wider) does.
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

    /// Fold `other`'s held value into this one — `self` ends holding the
    /// sum — and return the spare buffer, **not** the sum: amortized
    /// O(the narrower operand's held digits) plus an O(1) buffer swap.
    ///
    /// Only the operand with fewer held digits is read: the sum always
    /// lands in whichever buffer held more (buffers are swapped first
    /// when `other` is the wider; on a tie, `self`'s buffer keeps the
    /// sum), so the digits a dying operand holds fund the fold that
    /// consumes it. The returned buffer is for the caller's pool: a
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
    /// assert!(spare.is_zero());                   // before any reuse
    /// ```
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
