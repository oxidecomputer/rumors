//! The causal rank: [`Rank`], the exact measure of an event tree, and
//! its canonical order-preserving byte encoding.
//!
//! The public contract lives on the type and on
//! [`Version::rank`](crate::Version::rank); the fold that computes it is
//! the skyline query kernel. This module is private.
//!
//! # The wire form
//!
//! [`Rank::encode`] emits a *prefix-ascending* bit stream — one whose
//! unpadded lexicographic order equals the ranks' numeric order even
//! when each stream is followed by arbitrary further bits — packed
//! MSB-first into zero-padded bytes:
//!
//! 1. **The integral part** `I = ⌊r⌋`, as the Elias delta code of
//!    `I + 1` with the length run's bit sense inverted: for
//!    `m = I + 1`, `w = bits(m)`, and `ρ = bits(w) − 1`, the stream is
//!    `ρ` **ones**, a terminating zero, the `ρ` bits of `w` below its
//!    leading bit, then the `w − 1` bits of `m` below its leading bit.
//!    Standard length-prefixed universal codes (Elias gamma, delta,
//!    omega, dsi-bitstream's ζ and π families) all order *backwards*
//!    across a length boundary — the length prefix is a zeros-run, so
//!    a longer (larger) value sorts lexicographically *before* a
//!    shorter one — and no code in `dsi-bitstream` 0.10 is
//!    lexicographically order-preserving as stored. Inverting the run
//!    polarity (ones ended by a zero) is the minimal change that turns
//!    the delta code prefix-ascending, and the payload layout is
//!    otherwise dsi's own. Even under that transform dsi's own codecs
//!    cannot serve this seam: its code implementations take `u64`
//!    arguments while a rank's integral part is arbitrary-precision,
//!    and its decoders are documented non-total on untrusted input
//!    (malformed streams may panic) where this decoder must strictly
//!    and totally reject — so the writer is in-house like every
//!    writer in this crate (the byte-backed stores again), and the
//!    reader is a few dozen lines over a plain byte slice. No
//!    maintained order-preserving varint reaches arbitrary precision
//!    either: `ordered-varint` caps at 16-byte primitives, and the
//!    FoundationDB tuple encoding's arbitrary-precision integers cap
//!    at 255-byte magnitudes behind a one-byte length header — both
//!    would truncate a counter a version can legitimately carry.
//! 2. **The fractional part**, as its binary expansion: exactly `exp`
//!    bits, the numerator's low `exp` bits MSB-first. Normalization
//!    keeps the numerator odd whenever `exp > 0`, so the expansion
//!    never ends in a zero bit — expansions without trailing zeros are
//!    unique, and their lexicographic order (a proper prefix sorting
//!    first) is exactly their numeric order, because a proper
//!    extension contains a set bit and only adds value.
//!
//! Zero-padding to the byte boundary preserves the bit-level verdict:
//! two encodings that differ at a bit position differ at that byte; if
//! one bit stream is a proper prefix of the other, the longer one's
//! extension holds a set bit (its fraction ends in one), which either
//! falls inside the shorter one's padded length — where the padding is
//! zero, deciding the byte comparison the same way — or beyond it,
//! where the shorter byte string is a proper byte prefix and sorts
//! first. Equal padded byte strings therefore force equal bit streams,
//! so padding can create neither ties nor inversions, and **byte-wise
//! lexicographic order on encodings equals [`Ord`] on ranks** — the
//! law the committed sweep and proptests pin.
//!
//! Every piece of the stream is forced: the header is bijective (each
//! `(ρ, payload)` pair decodes to a width `w` whose own width is
//! exactly `ρ + 1`, so non-minimal headers are unrepresentable), the
//! fraction's length is recovered from the stream's last set bit (a
//! "fraction with trailing zeros" is not expressible — inside the
//! final byte those bits *are* the padding, and spilling them into a
//! further byte fails the minimal-length check). The decoder rejects
//! exactly: truncation (the unary run, the header payload, or the
//! mantissa running off the end), non-minimal byte length (trailing
//! zero bytes), and the format's representation bounds (an integral
//! mantissa of `2⁶⁴` or more bits, a fraction deeper than `2³² − 1` —
//! both beyond any rank this crate can hold, and beyond any input
//! under 2 EiB / 512 MiB respectively).

use core::cmp::Ordering;
use core::fmt;
use core::iter::Sum;
use core::ops::{Add, AddAssign};

use suanpan::Accumulator;

use crate::codec::Base;
use crate::error::Decode;

/// The causal rank of a [`Version`](crate::Version).
///
/// The exact area under its event tree, a nonnegative dyadic rational `num ·
/// 2⁻ᵉˣᵖ` with arbitrary-precision numerator. Produced by
/// [`Version::rank`](crate::Version::rank).
///
/// An event tree is a height function over the unit id interval: a leaf
/// `n` is height `n` everywhere, and a node `(n, l, r)` lifts its children
/// by `n`, each over half the parent's width. The area under that function
/// — `Σ base · 2⁻ᵈᵉᵖᵗʰ` over every node — grows whenever the function
/// grows anywhere, and the causal order on versions *is* pointwise
/// comparison of their height functions. The area is therefore a
/// **strictly monotone rank**:
///
/// > if `v < w` then `v.rank() < w.rank()`.
///
/// Heights are step functions on dyadic intervals, so two distinct
/// versions ordered by `<` differ over an interval of positive width, and
/// the dominated one strictly loses area there. The contrapositive is what
/// consumers lean on: **equal ranks are never causally ordered** (they are
/// the same version or concurrent). Any tiebreak between equal ranks — a
/// content hash, [`as_bytes`](crate::Version::as_bytes) — therefore
/// extends the causal order to a total one, which is what makes `Rank` fit
/// for sorted-container keys that must deliver causes before effects.
///
/// [`min_ticks`](crate::Version::min_ticks) is the integer shadow of this
/// measure (every width rounded up to the whole interval): a valid but
/// only *weakly* monotone rank, blind to growth that fills concurrent gaps
/// — `(0, 1, 0) < 1`, yet both count one tick. The rank separates every
/// such pair exactly.
///
/// Totally ordered ([`Ord`]), unlike the versions it ranks. Comparison is
/// exact at any magnitude: mismatched magnitude classes are decided from
/// the stored widths in O(1), and class ties stream the numerators
/// most-significant-first, so no alignment of the exponents is ever
/// materialized; equality is structural (the stored form is normalized, so
/// equal values are identical representations, consistent with [`Hash`]).
///
/// # Complexity
///
/// A rank's costs are denominated in its *numeric size* `‖r‖` — the
/// numerator's bit width plus the exponent — which every
/// producing fold ([`Version::rank`](crate::Version::rank),
/// [`distance`](crate::Version::distance), [`lag`](crate::Version::lag))
/// keeps linear in the packed bits it read, and which
/// [`encode`](Rank::encode) makes tangible: the canonical byte form is
/// `‖r‖ + O(log ‖r‖)` bits. Comparison (`==`, [`Ord`]) is
/// `O(1)` when the two magnitudes differ in scale and `O(‖a‖ + ‖b‖)` time
/// with no allocation on scale ties; hashing and cloning are `O(‖r‖)`.
/// Addition (`+`, `+=`) is `O(‖a‖ + ‖b‖)` time and space, and an n-ary
/// [`Sum`] is `O(N)` in the summands' total numeric size `N`: the fold
/// carries one running accumulator, and each summand pays its own width
/// rather than the accumulator's. Rendering (`Display`) is `O(d)` space
/// in the `d` decimal digits printed, but its time additionally pays
/// binary-to-decimal conversion of the numerator, superlinear (though
/// subquadratic) in its width past a machine word.
///
/// **Complexity**: comparison and addition `O(‖a‖ + ‖b‖)`, `Sum` `O(N)`; `Display` superlinear in the numerator width (decimal conversion).
///
/// ```
/// use before::Version;
/// let half: Version = "(0, 1, 0)".parse().unwrap(); // height 1 over half the interval
/// let one = Version::try_from(1).unwrap();          // height 1 everywhere
/// assert!(half < one);                              // strictly dominated...
/// assert!(half.rank() < one.rank());                // ...so strictly smaller rank
/// assert_eq!(half.min_ticks(), one.min_ticks());    // the tick floor cannot see it
/// ```
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Rank {
    /// The numerator. Normalized: odd, or zero with `exp` zero, so each
    /// value has exactly one representation.
    num: Base,
    /// The (binary) exponent of the denominator `2^exp`. Bounded by the
    /// event tree's depth, since each level halves the interval width.
    exp: u32,
}

impl Rank {
    /// The zero rank: the area under the empty [`Version`](crate::Version),
    /// and the identity for [`Rank`] addition. Equal to
    /// [`Version::new().rank()`](crate::Version::rank).
    ///
    /// ```
    /// use before::{Rank, Version};
    /// assert_eq!(Version::new().rank(), Rank::ZERO);
    /// assert_eq!(Version::try_from(7).unwrap().rank() + Rank::ZERO,
    ///            Version::try_from(7).unwrap().rank());
    /// ```
    pub const ZERO: Rank = Rank {
        num: Base::ZERO,
        exp: 0,
    };

    /// The difference `self - rhs`, or [`None`] when `rhs` exceeds `self`.
    ///
    /// Ranks are nonnegative dyadic rationals — a totally ordered commutative
    /// monoid under [`+`](Add), not a group — so subtraction is partial. The
    /// difference exists exactly when `rhs <= self`: a caller subtracting
    /// along the lattice order — a dominated version's rank from a
    /// dominating one's, as when re-deriving
    /// [`distance`](crate::Version::distance) or
    /// [`lag`](crate::Version::lag) from per-version ranks — never sees
    /// the [`None`] arm.
    ///
    /// # Complexity
    ///
    /// `O(‖a‖ + ‖b‖)` time and space in the operands' numeric size (see
    /// [the type's note](Rank#complexity)); a [`None`] or zero result costs
    /// only the comparison, which allocates nothing.
    ///
    /// **Complexity**: `O(‖a‖ + ‖b‖)`, the operands' numeric sizes.
    ///
    /// ```
    /// use before::Version;
    /// let five = Version::try_from(5).unwrap().rank();
    /// let three = Version::try_from(3).unwrap().rank();
    /// assert_eq!(five.checked_sub(&three).unwrap().to_string(), "2");
    /// assert!(three.checked_sub(&five).is_none()); // 3 - 5 has no nonnegative value
    /// ```
    pub fn checked_sub(&self, rhs: &Rank) -> Option<Rank> {
        // The ordering pre-check rides the class-first comparison, so the
        // `None` and zero arms cost no alignment at all; only a strictly
        // positive difference aligns to the common exponent and subtracts,
        // and that transient is the output's own value content.
        match self.cmp(rhs) {
            Ordering::Less => None,
            Ordering::Equal => Some(Rank::ZERO),
            Ordering::Greater => {
                let e = self.exp.max(rhs.exp);
                let a = self.num.clone() << (e - self.exp);
                let b = rhs.num.clone() << (e - rhs.exp);
                Some(Rank::from_raw(a - &b, e))
            }
        }
    }

    /// Encodes this rank into its canonical byte form, whose
    /// **byte-wise lexicographic order equals [`Ord`] on ranks**:
    /// `encode(a) < encode(b)` as byte strings exactly when `a < b`.
    ///
    /// The reach-for-it use is a **causal-ordering key in a sorted KV
    /// store**: store each entry under its version's rank encoding
    /// (plus any rank-tiebreak suffix — the encoding is prefix-safe,
    /// so a suffix can never flip the order between distinct ranks)
    /// and a plain key iteration delivers causes before effects, with
    /// no rank-aware comparator on the store's side. Equal ranks are
    /// never causally ordered (see [`Rank`]), so entries colliding on
    /// the rank prefix are concurrent or identical, and any
    /// deterministic tiebreak — the version's
    /// [`as_bytes`](crate::Version::as_bytes), a content hash — is
    /// causally safe.
    ///
    /// The encoding is canonical and bijective: [`decode`](Rank::decode)
    /// accepts exactly the byte strings this method produces, and equal
    /// ranks encode byte-identically (distinct ranks never collide).
    /// The format itself — the prefix-ascending integral code, the
    /// trailing-zero-free fraction, the padding argument — is the
    /// module's business; callers need only the law above.
    ///
    /// # There is no numerator–exponent serialization
    ///
    /// Deliberately. A rank is stored as `num · 2⁻ᵉˣᵖ`, and that pair
    /// is exponentially denser than this order-preserving form (the
    /// rank `2⁻ᵏ` costs a few bytes as a pair but `k` bits here): a
    /// library exposing both a compact pair codec and this expansion
    /// would hand any decoder of the pair form a decompression bomb.
    /// Only the lexicographic form is a wire format; the pair form
    /// never leaves memory ([`Display`](fmt::Display) renders the pair
    /// for humans, and nothing parses it back).
    ///
    /// # Cost, and where it is bounded
    ///
    /// The output is `‖r‖ + O(log ‖r‖)` bits — one bit per fractional
    /// digit and per integral bit. Every rank *reachable from this
    /// crate's public constructors* keeps that linear in what the
    /// caller already paid \[derived, and pinned per family by
    /// `rank_encoding_size_is_provenance_linear`\]: a fold's rank
    /// ([`Version::rank`](crate::Version::rank),
    /// [`distance`](crate::Version::distance),
    /// [`lag`](crate::Version::lag)) has numerator width and exponent
    /// each linear in the version's packed bits (the measured worst
    /// committed family — a lone wide counter, whose stored gamma code
    /// pays the width twice where this encoding pays it once — encodes
    /// at ~0.56 bits per packed input bit); sums and
    /// differences of such ranks stay linear in their operands'
    /// content; and [`decode`](Rank::decode) yields ranks linear in
    /// the bytes it read. In-memory arithmetic *can* mint a rank whose
    /// encoding is exponentially larger than the pair it is stored as
    /// — the caller who folds a deep version pays the version's size
    /// first, so the expansion never outruns what its producer already
    /// held.
    ///
    /// # Complexity
    ///
    /// `O(‖r‖)` time and space in the rank's numeric size (see
    /// [the type's note](Rank#complexity)); the output is
    /// `‖r‖ + O(log ‖r‖)` bits.
    ///
    /// **Complexity**: `O(‖r‖)` time and space; the output is `‖r‖ + O(log ‖r‖)` bits.
    ///
    /// ```
    /// use before::{Rank, Version};
    /// let half: Version = "(0, 1, 0)".parse().unwrap();
    /// let one = Version::try_from(1).unwrap();
    /// let (ka, kb) = (half.rank().encode(), one.rank().encode());
    /// assert!(ka < kb); // byte order is rank order: 1/2 < 1
    /// assert_eq!(Rank::decode(&ka[..]).unwrap(), half.rank());
    /// ```
    pub fn encode(&self) -> Vec<u8> {
        encode_parts(&self.num, self.exp)
    }

    /// Decodes a rank from a reader of canonical
    /// [`encode`](Rank::encode) bytes, strictly rejecting everything
    /// else.
    ///
    /// Total over arbitrary input: every byte string either decodes to
    /// the one rank that encodes to it, or is rejected — no accepted
    /// input re-encodes differently, so byte equality on encodings is
    /// rank equality. The result's in-memory size is linear in the
    /// bytes read (`k` input bits produce a rank of numeric size
    /// `O(k)`), so no input is a decompression bomb in either
    /// direction.
    ///
    /// # Errors
    ///
    /// [`Decode::Truncated`] when the stream ends inside the integral
    /// header or its mantissa; [`Decode::TrailingBits`] when the byte
    /// string is longer than the minimal packing of its content
    /// (trailing zero bytes — the one spelling a trailing-zero
    /// fraction could take); [`Decode::NotCanonical`] when the stream
    /// declares content past the format's representation bounds (an
    /// integral mantissa of `2⁶⁴` or more bits, a fraction deeper than
    /// `2³² − 1` — reachable only through inputs of 2 EiB and 512 MiB
    /// respectively); [`Decode::Io`] when the reader itself fails.
    ///
    /// # Complexity
    ///
    /// `O(n)` time and space in the bytes read, accepted or rejected:
    /// one pass over the stream, and a result linear in it.
    ///
    /// **Complexity**: `O(n)`.
    ///
    /// ```
    /// use before::{error::Decode, Rank, Version};
    /// let key = Version::try_from(5).unwrap().rank().encode();
    /// assert_eq!(Rank::decode(&key[..]).unwrap().to_string(), "5");
    /// // A trailing zero byte is not the minimal packing: rejected.
    /// let padded = [key.clone(), vec![0]].concat();
    /// assert!(matches!(Rank::decode(&padded[..]), Err(Decode::TrailingBits)));
    /// ```
    pub fn decode<R: std::io::Read>(mut reader: R) -> Result<Rank, Decode> {
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).map_err(Decode::Io)?;
        decode_bytes(&buf)
    }

    /// The rank's value content in bits: `bits(num) + exp`.
    ///
    /// The meter denominator of record for `Rank` operands, which have no
    /// packed encoding to charge against: the numerator's bit width plus
    /// the exponent bounds the information the value carries, and every
    /// public construction path emits ranks whose content is linear in the
    /// packed bits it read, so a cost linear in this quantity is linear in
    /// wire terms too.
    #[cfg(any(test, feature = "meter"))]
    pub(crate) fn content_bits(&self) -> u64 {
        self.num.bits() + u64::from(self.exp)
    }

    /// The stored parts `(numerator, exponent)`, for the reference
    /// computations and differential oracles that re-derive the order and
    /// the arithmetic from the raw normalized form.
    #[cfg(test)]
    pub(crate) fn raw_parts(&self) -> (&Base, u32) {
        (&self.num, self.exp)
    }

    /// Normalize raw fold output `num · 2⁻ᵉˣᵖ` into canonical form: strip
    /// the factors of two shared by numerator and denominator, and pin zero
    /// to exponent zero, so structural equality is value equality.
    ///
    /// `pub(crate)` for the reference computations (the oracle's tree fold,
    /// the semantic oracle's Riemann sum), which produce the same raw form.
    pub(crate) fn from_raw(num: Base, exp: u32) -> Self {
        match num.trailing_zeros() {
            None => Rank {
                num: Base::ZERO,
                exp: 0,
            },
            Some(tz) => {
                let shift = u32::try_from(tz.min(u64::from(exp))).expect("min with a u32");
                Rank {
                    num: num >> shift,
                    exp: exp - shift,
                }
            }
        }
    }
}

/// Emit the canonical prefix-ascending stream for `num · 2⁻ᵉˣᵖ` (the
/// module doc carries the format and the order argument).
///
/// `pub(crate)` alongside [`Rank::encode`] so the ranked view's fused
/// emission can emit straight from its rank fold's
/// `(numerator, exponent)` output, with no walk beyond the fold's own.
pub(crate) fn encode_parts(num: &Base, exp: u32) -> Vec<u8> {
    // The integral part, biased so zero has a (smallest) codeword:
    // m = ⌊r⌋ + 1, w = bits(m), ρ = bits(w) − 1.
    let biased = (num.clone() >> exp) + 1u32;
    let w = biased.bits();
    let rho = u64::from(63 - w.leading_zeros());
    let mut sink = BitSink::with_capacity_bits(2 * rho + 1 + (w - 1) + u64::from(exp));
    // The header: ρ ones, the terminating zero, then w's bits below its
    // leading bit — the Elias delta length header with the run's bit
    // sense inverted, so longer (larger) integral parts sort after
    // shorter ones instead of before.
    for _ in 0..rho {
        sink.push(true);
    }
    sink.push(false);
    for i in (0..rho).rev() {
        sink.push(w >> i & 1 == 1);
    }
    // The integral mantissa: m's bits below its leading bit.
    for i in (0..w - 1).rev() {
        sink.push(biased.bit(i));
    }
    // The fraction: the numerator's low `exp` bits MSB-first — the
    // binary expansion of the fractional part, which normalization
    // (an odd numerator whenever exp > 0) keeps free of trailing
    // zeros, so the expansion is unique and lexicographic order on
    // expansions is numeric order.
    for i in (0..u64::from(exp)).rev() {
        sink.push(num.bit(i));
    }
    sink.into_bytes()
}

/// Parse one canonical stream (strictly: [`Rank::decode`]'s contract).
fn decode_bytes(bytes: &[u8]) -> Result<Rank, Decode> {
    // Bit addressing is MSB-first within each byte. `total` cannot
    // overflow: a `Vec` holds at most `isize::MAX` bytes.
    let total = bytes.len() as u64 * 8;
    let bit = |i: u64| bytes[(i / 8) as usize] >> (7 - i % 8) & 1 == 1;
    // The header's unary run: ρ ones ended by a zero.
    let mut pos = 0;
    while pos < total && bit(pos) {
        pos += 1;
    }
    if pos == total {
        // Empty input, or the run never terminated.
        return Err(Decode::Truncated);
    }
    let rho = pos;
    pos += 1; // the terminating zero
    if rho >= 64 {
        // The format bound: an integral width of 2⁶⁴ or more bits
        // exceeds both the numerator this crate can hold and any input
        // under 2 EiB (the mantissa alone would need 2⁶⁴ − 1 bits).
        return Err(Decode::NotCanonical);
    }
    if total - pos < rho {
        return Err(Decode::Truncated);
    }
    // w's bits below its (implied) leading bit: ρ of them, so w < 2⁶⁴.
    let mut w = 1u64;
    for _ in 0..rho {
        w = w << 1 | u64::from(bit(pos));
        pos += 1;
    }
    if w - 1 > total - pos {
        return Err(Decode::Truncated);
    }
    // The biased integral m (its leading bit implied), then unbias.
    let integral = read_magnitude(&bit, pos, w - 1, true) - &Base::from(1u8);
    pos += w - 1;
    // The fraction runs from the header's end through the stream's
    // last set bit: its expansion never ends in zero, so everything
    // after the last set bit is padding, never content.
    let last_set = (1..=bytes.len())
        .rev()
        .find(|&i| bytes[i - 1] != 0)
        .map_or(0, |i| {
            (i as u64 - 1) * 8 + u64::from(8 - bytes[i - 1].trailing_zeros())
        });
    let frac_len = last_set.saturating_sub(pos);
    let exp = u32::try_from(frac_len).map_err(|_| {
        // The format bound: Rank's exponent (the event-tree depth) is
        // u32; a deeper fraction needs over 512 MiB of input.
        Decode::NotCanonical
    })?;
    // Strict minimal packing: content bits, then zero bits to the byte
    // boundary and not a bit more. Bits past the content are zero by
    // construction (they sit past the last set bit), so the one check
    // left is that no whole trailing byte is padding.
    if (pos + frac_len).div_ceil(8) != bytes.len() as u64 {
        return Err(Decode::TrailingBits);
    }
    let num = if frac_len == 0 {
        integral
    } else {
        (integral << exp) | read_magnitude(&bit, pos, frac_len, false)
    };
    debug_assert!(
        exp == 0 || num.bit(0),
        "a nonempty fraction ends in its last set bit, so the numerator is odd"
    );
    Ok(Rank { num, exp })
}

/// Assemble the magnitude whose bits are (`lead_one` then) the `len`
/// stream bits at `start`, MSB-first.
fn read_magnitude(bit: &impl Fn(u64) -> bool, start: u64, len: u64, lead_one: bool) -> Base {
    let width = len + u64::from(lead_one);
    if width == 0 {
        return Base::ZERO;
    }
    // The callers bound `len` by the input's bit count, so the byte
    // buffer fits comfortably in memory.
    let mut buf = vec![0u8; usize::try_from(width.div_ceil(8)).expect("bounded by input bytes")];
    let offset = buf.len() as u64 * 8 - width;
    let mut set = |i: u64| buf[(i / 8) as usize] |= 1 << (7 - i % 8);
    if lead_one {
        set(offset);
    }
    for k in 0..len {
        if bit(start + k) {
            set(offset + u64::from(lead_one) + k);
        }
    }
    Base::from_be_bytes(&buf)
}

/// An MSB-first bit sink packing into bytes, the final byte zero-padded.
struct BitSink {
    bytes: Vec<u8>,
    /// Bits already used in the final byte, `0..8` (`0` also when empty).
    used: u8,
}

impl BitSink {
    fn with_capacity_bits(bits: u64) -> BitSink {
        BitSink {
            bytes: Vec::with_capacity(usize::try_from(bits.div_ceil(8)).expect("output fits")),
            used: 0,
        }
    }

    fn push(&mut self, bit: bool) {
        if self.used == 0 {
            self.bytes.push(0);
        }
        if bit {
            *self.bytes.last_mut().expect("just ensured nonempty") |= 1 << (7 - self.used);
        }
        self.used = (self.used + 1) % 8;
    }

    fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

impl Ord for Rank {
    fn cmp(&self, other: &Self) -> Ordering {
        // Class first: `bits(num) − exp` is `floor(log2 value) + 1`, so
        // unequal classes order the values in O(1) — value ranges
        // `[2^(c−1), 2^c)` at distinct `c` never overlap. A class tie
        // means the two numerators' bit strings are already MSB-aligned
        // as binary fractions, and the streamed window comparison settles
        // them without materializing an alignment shift; its
        // longer-string-wins tail rule is sound because normalization
        // keeps numerators odd (the longer string ends in a set bit). The
        // order is exact at any magnitude — a false tie here would let a
        // consumer deliver an effect before its cause. Zero (the one
        // even-numerator form, pinned to exponent zero) is settled before
        // classes: its class value would collide with genuine
        // `(0, 1]`-range ranks.
        match (self.num.bits() == 0, other.num.bits() == 0) {
            (true, true) => return Ordering::Equal,
            (true, false) => return Ordering::Less,
            (false, true) => return Ordering::Greater,
            (false, false) => {}
        }
        let class = |r: &Rank| i128::from(r.num.bits()) - i128::from(r.exp);
        class(self)
            .cmp(&class(other))
            .then_with(|| Base::msb_cmp(&self.num, &other.num))
    }
}

impl PartialOrd for Rank {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// `Rank` under `+` is a commutative monoid with identity [`Rank::ZERO`]: the
// exact sum of two dyadic rationals, normalized so equal values stay
// structurally equal. It is *not* the [`Version`](crate::Version) join — the
// join takes a pointwise maximum, whereas this adds areas — but the two meet in
// the valuation law `rank(a | b) + rank(a & b) == rank(a) + rank(b)`, which is
// what makes [`Version::distance`](crate::Version::distance) a metric. The four
// reference forms mirror [`Base`]'s own `Add` matrix so callers need not place
// borrows by hand.

/// Adds two ranks: the exact sum of the two areas.
///
/// Rank addition is *measure* arithmetic, not history arithmetic: areas
/// add, histories join. Its meaning comes from the rank being a valuation
/// on the version lattice — `rank(a | b) + rank(a & b) == rank(a) +
/// rank(b)` — which is what makes
/// [`distance`](crate::Version::distance) a metric and lets its directed
/// halves recombine (`a.lag(b) + b.lag(a) == a.distance(b)`). Use `+` to
/// aggregate measures — a total replication backlog across peers, a
/// budget consumed so far — never to combine histories; that is the
/// version join `|`.
impl Add<&Rank> for &Rank {
    type Output = Rank;
    fn add(self, rhs: &Rank) -> Rank {
        let e = self.exp.max(rhs.exp);
        let a = self.num.clone() << (e - self.exp);
        let b = rhs.num.clone() << (e - rhs.exp);
        Rank::from_raw(a + &b, e)
    }
}

impl Add<Rank> for Rank {
    type Output = Rank;
    fn add(self, rhs: Rank) -> Rank {
        &self + &rhs
    }
}

impl Add<&Rank> for Rank {
    type Output = Rank;
    fn add(self, rhs: &Rank) -> Rank {
        &self + rhs
    }
}

impl Add<Rank> for &Rank {
    type Output = Rank;
    fn add(self, rhs: Rank) -> Rank {
        self + &rhs
    }
}

impl AddAssign<&Rank> for Rank {
    fn add_assign(&mut self, rhs: &Rank) {
        *self = &*self + rhs;
    }
}

impl AddAssign<Rank> for Rank {
    fn add_assign(&mut self, rhs: Rank) {
        *self = &*self + &rhs;
    }
}

/// The empty sum is [`Rank::ZERO`], the additive identity.
impl Sum<Rank> for Rank {
    fn sum<I: Iterator<Item = Rank>>(iter: I) -> Rank {
        sum_ranks(iter)
    }
}

/// The empty sum is [`Rank::ZERO`], the additive identity.
impl<'a> Sum<&'a Rank> for Rank {
    fn sum<I: Iterator<Item = &'a Rank>>(iter: I) -> Rank {
        sum_ranks(iter)
    }
}

/// Sum ranks through one raw accumulator with a single final
/// normalization.
///
/// The accumulator holds the running numerator at the largest exponent
/// seen so far: a summand at a smaller exponent is digit-routed in at the
/// exponent gap (O(its own limbs), independent of the gap), and a summand
/// raising the maximum rescales the accumulator once, O(held digits) —
/// paid by the exponent the summand itself carries. Nothing renormalizes
/// per element, so a high-exponent summand costs its own width once
/// instead of once per later element, and the result is the identical
/// [`Rank`] the pairwise fold produces (one exact value, one shared
/// normalization at the end).
fn sum_ranks<T: core::borrow::Borrow<Rank>, I: Iterator<Item = T>>(iter: I) -> Rank {
    let mut acc = Accumulator::new();
    let mut exp = 0u32;
    for rank in iter {
        let rank = rank.borrow();
        if rank.exp > exp {
            acc.shl(u64::from(rank.exp - exp));
            exp = rank.exp;
        }
        acc.add_magnitude_shl(&rank.num, u64::from(exp - rank.exp));
    }
    let (sign, magnitude) = acc.sign_magnitude();
    debug_assert_ne!(
        sign,
        Ordering::Less,
        "a sum of nonnegative ranks is nonnegative"
    );
    Rank::from_raw(Base::from(magnitude), exp)
}

/// [`Rank::ZERO`], the additive identity.
impl Default for Rank {
    fn default() -> Self {
        Rank::ZERO
    }
}

/// Renders as the exact rational: the numerator alone when integral,
/// `num/2^exp` otherwise.
///
/// ```
/// use before::Version;
/// assert_eq!(Version::try_from(5).unwrap().rank().to_string(), "5");
/// let half: Version = "(0, 1, 0)".parse().unwrap();
/// assert_eq!(half.rank().to_string(), "1/2^1");
/// ```
impl fmt::Display for Rank {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.exp {
            0 => fmt::Display::fmt(&self.num, f),
            exp => write!(f, "{}/2^{}", self.num, exp),
        }
    }
}

/// The same format as `Display`.
impl fmt::Debug for Rank {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <Self as fmt::Display>::fmt(self, f)
    }
}
