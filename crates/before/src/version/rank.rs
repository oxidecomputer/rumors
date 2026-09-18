//! The causal rank: [`Rank`], the exact measure of an event tree, and its
//! canonical order-preserving byte encoding.
//!
//! The public contract lives on the type and on
//! [`Version::rank`](crate::Version::rank); the fold that computes it is the
//! skyline query kernel. This module is private.
//!
//! # The wire form
//!
//! [`Rank::encode`] emits a *prefix-ascending* bit stream — one whose
//! lexicographic order equals the ranks' numeric order even when each stream is
//! followed by arbitrary further bits, because two distinct ranks' streams
//! always differ at a bit position inside both — encoded MSB-first into
//! zero-padded bytes:
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
//!    otherwise dsi's own.
//!
//!    *Why delta among its siblings.* The trade is size-pin margin
//!    against proof surface. Gamma spends its unary run re-paying the
//!    mantissa's whole width — `2N` bits for an `N`-bit integral —
//!    and a [`Version`](crate::Version) already stores its counters
//!    gamma-coded, so a gamma integral part would pay that doubled
//!    width *again* and drive the worst committed provenance family
//!    (the lone wide counter, measured at 0.56 encoded bits per
//!    encoded input bit) up against the 1.0-per-family
//!    provenance-linearity pin; delta's `N + O(log N)` is what keeps
//!    the canonical form a mild compression of its provenance. Omega,
//!    one rung further, trims the header by only `O(log w)` bits —
//!    noise against an `N`-bit mantissa — while every one of its
//!    recursion levels is another length boundary owing its own
//!    inverted-polarity monotonicity argument and its own boundary
//!    goldens; on a frozen wire format the order argument's
//!    auditability outranks a handful of header bits, and delta's
//!    single nested length layer is why the argument stays short and
//!    the golden matrix small. Byte-oriented varints are not
//!    order-preserving as stored and are byte-granular where the
//!    fraction below is bit-granular, and a flat one-byte length
//!    header caps magnitudes — generalizing it recursively just
//!    re-derives the Elias family.
//!
//!    Even under the polarity transform dsi's own codecs
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
//!
//! 2. **The fractional part**, as its binary expansion (the
//!    numerator's low `exp` bits MSB-first) in groups of eight bits,
//!    each group opened by a set *continuation* bit and the last
//!    zero-padded to full width; one clear bit closes the fraction and
//!    the stream. Normalization keeps the numerator odd whenever
//!    `exp > 0`, so the expansion never ends in a zero bit: the final
//!    group is nonzero, and its trailing zeros are recoverably
//!    padding. Group order is numeric order: two same-integral streams
//!    align group-for-group, a difference inside a group is decided at
//!    its first differing expansion bit, and where only one fraction
//!    continues, its set continuation bit beats the other's closing
//!    zero — the extension carries a further set bit, so it denotes
//!    the larger value.
//!
//!    *Why framing, not a length header.* The integral part's trick —
//!    a prefix-ascending length code ahead of the payload — is sound
//!    only because integer order is graded by length: with the
//!    leading bit implied, more mantissa bits is strictly larger, so
//!    sorting by length first agrees with value order. Fraction order
//!    has no such grading — the one-bit `0.1₂ = 1/2` exceeds the
//!    four-bit `0.0111₂ = 7/16` — and a header-first stream orders by
//!    length at the first differing header bit, before any expansion
//!    bit is compared: ascending polarity sorts 1/2 below 7/16,
//!    descending sorts 3/4 below 5/8, and no polarity can work,
//!    because fraction comparison is positional — decided at the
//!    first differing expansion bit, with end-of-stream sorting below
//!    any continuation (a fraction precedes its proper extensions).
//!    That is an in-band requirement, and the continuation bit is its
//!    direct spelling — the same reason the FoundationDB tuple
//!    encoding chunk-escapes its variable-length byte strings rather
//!    than length-prefixing them. The cost runs opposite the usual
//!    trade: a length header would be asymptotically cheaper
//!    (`O(log k)` against `k⁄8`) if it were sound, so the 9⁄8 is the
//!    minimum rent for in-band delimitation at byte granularity, not
//!    a missed compression — and a fraction length header would put a
//!    forgeable depth claim on the wire, recreating the
//!    allocation-bomb rejection surface the framed form structurally
//!    lacks (every allocation is fed by bits actually read).
//!
//! The closing bit makes every stream self-delimiting, so distinct ranks'
//! streams are never prefixes of one another: they differ at a bit position
//! **inside both**, the padded byte forms differ at that byte, and neither byte
//! string is a byte prefix of the other. Hence zero-padding can create neither
//! ties nor inversions, **byte-wise lexicographic order on encodings equals
//! [`Ord`] on ranks**, and no appended suffix can flip the order between
//! distinct ranks — the laws the committed sweep and proptests pin.
//!
//! Every piece of the stream is forced: the `I + 1` bias does two jobs — it
//! gives zero (a codeless value in the delta family) the smallest codeword, and
//! it keeps `m ≥ 1` so the leading bits of both `m` and `w` stay implied, which
//! is what makes the header bijective (each `(ρ, payload)` pair decodes to a
//! width `w` whose own width is exactly `ρ + 1`, so non-minimal headers are
//! unrepresentable and no rejection genre exists for them) — and the fraction's
//! depth is recovered from the final group's last set bit (a "fraction with
//! trailing zeros" is not expressible — inside the final group those bits *are*
//! the padding, and spilling them into a further group leaves that group
//! all-zero, which the decoder rejects as non-minimal). The decoder rejects
//! exactly: truncation (the unary run, the header payload, the mantissa, a
//! group, or its continuation bit running off the end), non-minimal packing
//! (byte length beyond the stream's own, a set bit in the padding, or an
//! all-zero final group), and the format's one representation bound (an
//! integral mantissa of `2⁶⁴` or more bits — beyond any rank this crate can
//! hold, and beyond any input under 2 EiB; the fraction's depth is counted from
//! bits actually read, so it can never outrun the exponent that stores it).

use core::cmp::Ordering;
use core::fmt::{self, Debug, Display};
use core::iter::Sum;
use core::ops::{Add, AddAssign};
use core::str::FromStr;
use std::io::{self, Write};

use num_bigint::BigUint;
use suanpan::Accumulator;

use crate::codec::accumulator;
use crate::error::{Decode, ParseRank};

/// The causal rank of a [`Version`](crate::Version) as an exact dyadic
/// rational.
///
/// The [`Rank`] of a [`Version`] is **strictly monotone** in
/// [`tick`](crate::Version::tick)s; that is, for every pair of versions `v` and
/// `w`:
///
/// > if `v < w` then `v.rank() < w.rank()`.
///
/// Contrapositively, **equal ranks are never causally ordered** (they are the
/// same version or concurrent). This means that any tiebreak between equal
/// ranks therefore extends [`Rank`]'s induced causal order to a total one. This
/// makes `Rank` well-fitted for sorted-container keys that must deliver causes
/// before effects. Indeed, the [`Ranked`](crate::Ranked) view builds exactly
/// such a total order in, with the version's own bytes as the tiebreak.
///
/// [`Rank`], and its companion view [`Ranked`], are both totally ordered
/// ([`Ord`]), unlike the [`Version`]s it ranks.
///
/// # Serialization and causal ordering
///
/// Like [`Clock`], [`Party`], [`Version`], and [`Span`], [`Rank`] and its
/// companion view [`Ranked`] have a canonical representation as encoded bytes.
/// This representation has a very deliberate property: *lexicographic ordering
/// on encoded [`Rank`]/[`Ranked`] exactly matches comparison by [`Ord`]*.
///
/// ```
/// use before::{Party, Version};
/// let mut p = Party::seed();
/// let q = p.fork();
/// let mut v = Version::new();
/// let mut w = Version::new();
/// // Tick `v` only once, by `p`:
/// p.tick(&mut v);
/// // Tick `w` concurrently by `p` and `q`:
/// p.tick(&mut w);
/// q.tick(&mut w);
/// // `w` out-ranks `v`, and the encoded ranks sort the same way:
/// assert!(w.rank() > v.rank());
/// assert!(w.rank().encode() > v.rank().encode());
/// ```
///
/// As a consequence, [`Rank`] and [`Ranked`] may be used to provide a
/// deterministic causal ordering to keys in an external data store which only
/// understands lexicographic ordering. See also the documentation for
/// [`Ranked`] for a fuller discussion.
///
/// # Complexity
///
/// Write `‖r‖` for a rank's binary width and `|v|` for a version's encoded
/// size. If `r = v.rank()`, then `‖r‖ = O(|v|)`; the rank may be exponentially
/// smaller. Its in-memory and encoded sizes are both `O(‖r‖)`. A bound stated
/// in `‖r‖` therefore gives the same upper bound in `|v|` for a rank derived
/// from a version.
///
/// In brief: comparison, equality, hashing, and cloning are linear in the
/// in-memory size. Addition and subtraction are `O(‖a‖ + ‖b‖)`.
/// Serialization/deserialization is linear in the bytes produced or read.
///
/// # Relationship to [`min_ticks`](crate::Version::min_ticks)
///
/// [`Rank`] buys a more fine-grained differentiation than
/// [`min_ticks`](crate::Version::min_ticks), since the latter cannot
/// differentiate between a [`Version`] comprising one
/// [`tick`](crate::Version::tick) and a [`Version`] comprising two concurrent
/// [`tick`](crate::Version::tick)s. By contrast, the latter strictly
/// out-[`Rank`]s the former:
///
/// ```
/// use before::{Party, Version};
/// let mut p = Party::seed();
/// let q = p.fork();
/// let mut v = Version::new();
/// let mut w = Version::new();
/// // Tick `v` only once, by `p`:
/// p.tick(&mut v);
/// // Tick `w` concurrently by `p` and `q`:
/// p.tick(&mut w);
/// q.tick(&mut w);
/// // The two versions have equal `min_ticks` but ordered `rank`s:
/// assert_eq!(v.min_ticks(), w.min_ticks());
/// assert!(v.rank() < w.rank());
/// ```
///
/// [`Clock`]: crate::Clock
/// [`Party`]: crate::Party
/// [`Ranked`]: crate::Ranked
/// [`Span`]: crate::Span
/// [`Version`]: crate::Version
#[derive(Clone, PartialEq, Eq, Hash)]
#[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscape-assets.html")))]
pub struct Rank {
    /// The numerator. Normalized: odd, or zero with `exp` zero, so each
    /// value has exactly one representation.
    ///
    num: BigUint,
    /// The (binary) exponent of the denominator `2^exp`. Bounded by the
    /// event tree's depth, since each level halves the interval width.
    exp: u64,
}

impl Rank {
    /// The [`Rank`] of [`Version::new()`](crate::Version::new).
    ///
    /// Equal to [`Version::new().rank()`](crate::Version::rank).
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Party, Rank, Version};
    /// assert_eq!(Version::new().rank(), Rank::ZERO);
    /// let mut seven = Version::new();
    /// Party::seed().ticks(&mut seven, 7u8);
    /// assert_eq!(seven.rank() + Rank::ZERO, seven.rank());
    /// ```
    pub const ZERO: Rank = Rank {
        num: BigUint::ZERO,
        exp: 0,
    };

    /// The difference `self - rhs`, or [`None`] when `rhs` exceeds `self`.
    ///
    /// When a deficit should floor at zero instead of being handled, use
    /// [`saturating_sub`](Rank::saturating_sub).
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/rank_checked_sub.html")))]
    #[cfg_attr(
        not(doc),
        doc = "`O(n)` in total input bytes; `O(‖self‖ + ‖other‖)`; a `None` or zero result costs only the comparison"
    )]
    ///
    /// A `None` or zero result allocates nothing.
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Party, Version};
    /// let mut five = Version::new();
    /// Party::seed().ticks(&mut five, 5u8);
    /// let mut three = Version::new();
    /// Party::seed().ticks(&mut three, 3u8);
    /// let (five, three) = (five.rank(), three.rank());
    /// assert_eq!(five.checked_sub(&three).unwrap().to_string(), "10");
    /// assert!(three.checked_sub(&five).is_none()); // 3 - 5 has no nonnegative value
    /// ```
    pub fn checked_sub(&self, other: &Rank) -> Option<Rank> {
        // The ordering pre-check rides the class-first comparison, so the
        // `None` and zero arms cost no alignment at all; only a strictly
        // positive difference aligns to the common exponent and subtracts, and
        // that transient is the output's own value content.
        match self.cmp(other) {
            Ordering::Less => None,
            Ordering::Equal => Some(Rank::ZERO),
            Ordering::Greater => {
                let e = self.exp.max(other.exp);
                if Self::alignment_fits(self.exp, other.exp, e) {
                    let a = self.num.clone() << (e - self.exp);
                    let b = other.num.clone() << (e - other.exp);
                    return Some(Rank::from_raw(a - &b, e));
                }
                let difference = self.accumulate(other, e, true);
                debug_assert!(
                    difference > Rank::ZERO,
                    "the Greater pre-check promises a strictly positive difference"
                );
                Some(difference)
            }
        }
    }

    /// The difference `self - rhs`, or [`Rank::ZERO`] when `rhs` exceeds
    /// `self`.
    ///
    /// [`checked_sub`](Rank::checked_sub) with the nonexistent difference
    /// floored at zero: use this when a deficit should read as "no distance
    /// remaining" rather than be handled arm by arm.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/rank_checked_sub.html")))]
    #[cfg_attr(
        not(doc),
        doc = "`O(n)` in total input bytes; `O(‖self‖ + ‖other‖)`; a `None` or zero result costs only the comparison"
    )]
    ///
    /// A floored result allocates nothing.
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Party, Rank, Version};
    /// let mut five = Version::new();
    /// Party::seed().ticks(&mut five, 5u8);
    /// let mut three = Version::new();
    /// Party::seed().ticks(&mut three, 3u8);
    /// let (five, three) = (five.rank(), three.rank());
    /// assert_eq!(five.saturating_sub(&three).to_string(), "10");
    /// assert_eq!(three.saturating_sub(&five), Rank::ZERO); // 3 - 5 floors at zero
    /// ```
    pub fn saturating_sub(&self, other: &Rank) -> Rank {
        self.checked_sub(other).unwrap_or(Rank::ZERO)
    }

    /// Encodes this rank into its canonical byte form, whose **byte-wise
    /// lexicographic order equals [`Ord`] on ranks**.
    ///
    /// The reason this encoding is crafted this way is to support
    /// **causal-ordering keys in a sorted KV store**: store each entry under
    /// its version's rank encoding, so a plain key iteration delivers causes
    /// before effects, with no rank-aware comparator on the store's side.
    ///
    /// Equal ranks never result from causally ordered [`Version`]s, so entries
    /// colliding on the rank prefix are concurrent or identical, and any
    /// deterministic tiebreak is causally safe. [`Ranked`] provides a canonical
    /// composite key for causally ordering [`Version`]s, which uses the
    /// [`Version`]'s own representation as the tiebreak.
    ///
    /// More generally, the encoding of a [`Rank`] is suffix-safe: for any set
    /// of [`Rank`]s, arbitrary suffixes may be appended to any or all of them,
    /// and their lexicographic ordering will not change. This means [`Rank`] is
    /// safe to use generically as one part of *any* composite key, not merely
    /// when composed with [`Version`] as it is in [`Ranked`].
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/rank_encode.html")))]
    #[cfg_attr(
        not(doc),
        doc = "`O(n)` in total input bytes; `O(‖self‖)` time and space"
    )]
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Clock, Rank};
    /// let mut half_clock = Clock::seed();
    /// let _other_half = half_clock.fork();
    /// let half = half_clock.tick().clone();
    /// let one = Clock::seed().tick().clone();
    /// let (ka, kb) = (half.rank().encode(), one.rank().encode());
    /// assert!(ka < kb); // byte order is rank order: 1/2 < 1
    /// assert_eq!(Rank::decode(&ka[..]).unwrap(), half.rank());
    /// ```
    ///
    /// [`Ranked`]: crate::Ranked
    /// [`Version`]: crate::Version
    pub fn encode(&self) -> Vec<u8> {
        Self::encode_parts(&self.num, self.exp)
    }

    /// Encodes this rank to an arbitrary writer: exactly
    /// [`encode`](Rank::encode)'s canonical bytes, without handing the caller
    /// the intermediate `Vec`.
    ///
    /// # Errors
    ///
    /// Whatever the writer itself reports; the encoding side is
    /// infallible.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/rank_encode.html")))]
    #[cfg_attr(
        not(doc),
        doc = "`O(n)` in total input bytes; `O(‖self‖)` time and space"
    )]
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Party, Version};
    /// let mut version = Version::new();
    /// Party::seed().ticks(&mut version, 5u8);
    /// let rank = version.rank();
    /// let mut buf = Vec::new();
    /// rank.encode_to(&mut buf).unwrap();
    /// assert_eq!(buf, rank.encode());
    /// ```
    pub fn encode_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&Self::encode_parts(&self.num, self.exp))
    }

    /// Decodes a rank from a reader of canonical [`encode`](Rank::encode)
    /// bytes, strictly rejecting everything else.
    ///
    /// # Decoded size
    ///
    /// The serialized representation of a [`Rank`] is at most `9⁄8 · ‖r‖ +
    /// O(log ‖r‖)` bits: one bit per integral bit, nine bits per eight
    /// fractional bits (this is required to keep distinct ranks' encodings
    /// prefix-free, providing the above generalized suffix-safety).
    ///
    /// # Errors
    ///
    /// - [`Decode::Truncated`] when the stream ends inside the integral header,
    ///   its mantissa, or the fraction (a group or its continuation bit);
    /// - [`Decode::TrailingBits`] when the byte string is not the minimal packing
    ///   of its content (bytes past the stream's own, a set bit in the padding,
    ///   or an all-zero final fraction group);
    /// - [`Decode::NotCanonical`] when otherwise valid content exceeds the type's
    ///   representation bound (an integral mantissa of `2⁶⁴` or more bits, effectively
    ///   unreachable, since it can only be hit by reading inputs of 2 EiB or more);
    /// - [`Decode::Io`] when the reader itself fails.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/rank_decode.html")))]
    #[cfg_attr(
        not(doc),
        doc = "`O(n)` in total input bytes; `O(n)`, `n` the bytes read, accepted or rejected"
    )]
    ///
    /// # Example
    ///
    /// ```
    /// use before::{error::Decode, Party, Rank, Version};
    /// let mut version = Version::new();
    /// Party::seed().ticks(&mut version, 5u8);
    /// let key = version.rank().encode();
    /// assert_eq!(Rank::decode(&key[..]).unwrap().to_string(), "101");
    /// // A trailing zero byte is not the minimal packing: rejected.
    /// let padded = [key.clone(), vec![0]].concat();
    /// assert!(matches!(Rank::decode(&padded[..]), Err(Decode::TrailingBits)));
    /// ```
    pub fn decode<R: io::Read>(mut reader: R) -> Result<Rank, Decode> {
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).map_err(Decode::Io)?;
        Self::decode_bytes(&buf)
    }

    /// Decodes canonical bytes already held in memory.
    pub(crate) fn decode_bytes(bytes: &[u8]) -> Result<Rank, Decode> {
        let mut iter = bytes.iter();
        let rank = Self::decode_stream(|| iter.next().copied().ok_or(Decode::Truncated))?;
        if iter.next().is_some() {
            // The caller supplied the whole input, so bytes past the
            // self-delimited stream are non-minimal packing.
            return Err(Decode::TrailingBits);
        }
        Ok(rank)
    }

    /// The rank's value content in bits: `bits(num) + exp`.
    ///
    /// The meter denominator for `Rank` operands, which have no byte encoding:
    /// the numerator's bit width plus the
    /// exponent bounds the information the value carries, and every public
    /// construction path emits ranks whose content is linear in the encoded bits
    /// it read, so a cost linear in this quantity is linear in wire terms too.
    #[cfg(any(test, feature = "meter"))]
    pub(crate) fn content_bits(&self) -> u64 {
        self.num.bits() + self.exp
    }

    /// The stored parts `(numerator, exponent)`.
    ///
    /// The fused encode's hand-off from a rank fold's output to the canonical
    /// emission ([`Self::encode_parts`]), and the raw normalized form the reference
    /// computations and differential oracles re-derive order and arithmetic
    /// from.
    ///
    /// It is **VERY IMPORTANT** that these not be exposed together, with the
    /// `from_raw` constructor, as this creates an affordance for constructing
    /// exponential serialization-size bombs.
    pub(crate) fn raw_parts(&self) -> (&BigUint, u64) {
        (&self.num, self.exp)
    }

    /// Normalize raw fold output `num · 2⁻ᵉˣᵖ` into canonical form: strip the
    /// factors of two shared by numerator and denominator, and pin zero to
    /// exponent zero, so structural equality is value equality.
    ///
    /// `pub(crate)` for the reference computations (the oracle's tree fold, the
    /// semantic oracle's Riemann sum), which produce the same raw form.
    ///
    /// It is **VERY IMPORTANT** that these not be exposed, together with the
    /// `raw_parts` destructor, as this creates an affordance for constructing
    /// exponential serialization-size bombs.
    pub(crate) fn from_raw(num: BigUint, exp: u64) -> Self {
        match num.trailing_zeros() {
            None => Rank {
                num: BigUint::ZERO,
                exp: 0,
            },
            Some(tz) => {
                let shift = tz.min(exp);
                Rank {
                    num: num >> shift,
                    exp: exp - shift,
                }
            }
        }
    }

    /// Whether both exponent gaps fit the big-integer shift interface.
    fn alignment_fits(a_exp: u64, b_exp: u64, common_exp: u64) -> bool {
        usize::try_from(common_exp - a_exp).is_ok() && usize::try_from(common_exp - b_exp).is_ok()
    }

    /// Combine `self ± rhs` at exponent `exp` through the streaming
    /// accumulator.
    ///
    /// Reserving for the wider aligned operand avoids a transient created by
    /// growth-doubling the buffer.
    fn accumulate(&self, rhs: &Rank, exp: u64, subtract_rhs: bool) -> Rank {
        let mut acc = Accumulator::new();
        let aligned_bits = |rank: &Rank| {
            if rank.num.bits() == 0 {
                0
            } else {
                rank.num.bits().saturating_add(exp - rank.exp)
            }
        };
        let widest = aligned_bits(self).max(aligned_bits(rhs)).saturating_add(1);
        if let Ok(digits) = usize::try_from(widest / 32 + 2) {
            acc.reserve_digits(digits);
        }
        accumulator::fold(&mut acc, &self.num, exp - self.exp, false);
        accumulator::fold(&mut acc, &rhs.num, exp - rhs.exp, subtract_rhs);
        let (sign, num) = accumulator::value(&acc);
        debug_assert_ne!(
            sign,
            Ordering::Less,
            "rank addition and pre-checked subtraction are nonnegative"
        );
        Rank::from_raw(num, exp)
    }

    /// Emit the canonical prefix-ascending stream for `num · 2⁻ᵉˣᵖ`.
    ///
    /// The ranked view calls this after its fused rank fold, avoiding another
    /// walk merely to construct a `Rank`.
    pub(crate) fn encode_parts(num: &BigUint, exp: u64) -> Vec<u8> {
        // The integral part, biased so zero has a (smallest) codeword:
        // m = ⌊r⌋ + 1, w = bits(m), ρ = bits(w) − 1. The shift is total at any
        // exponent — right shift clamps past the value's width — so
        // a fraction-heavy rank whose `exp` outruns a 32-bit `usize` (from
        // ~604 MB of decoded input) floors to zero here exactly as any other
        // sub-unit value does.
        let biased = (num.clone() >> exp) + 1u32;
        let w = biased.bits();
        let rho = u64::from(63 - w.leading_zeros());
        let groups = exp.div_ceil(FRACTION_GROUP_BITS);
        let mut sink =
            BitSink::with_capacity_bits(2 * rho + w + groups * (FRACTION_GROUP_BITS + 1) + 1);
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
        // The fraction: the binary expansion (expansion bit `j`, counting
        // from the binary point, is the numerator's bit `exp − j`) in
        // groups of eight, each opened by a set continuation bit, the last
        // zero-padded; a clear bit closes the stream. Normalization (an odd
        // numerator whenever exp > 0) puts the expansion's final set
        // bit inside the last group, which keeps the padding recoverable
        // and the group order numeric (the module doc's argument).
        for g in 0..groups {
            sink.push(true);
            for j in g * FRACTION_GROUP_BITS + 1..=(g + 1) * FRACTION_GROUP_BITS {
                sink.push(j <= exp && num.bit(exp - j));
            }
        }
        sink.push(false);
        sink.into_bytes()
    }
}

/// The width of one fraction group: the expansion rides in byte-sized groups,
/// each opened by a continuation bit, so the fraction costs nine bits per eight
/// expansion bits plus the one closing bit.
const FRACTION_GROUP_BITS: u64 = 8;

/// A byte-at-a-time source dressed as an MSB-first bit reader: one byte
/// buffered, refilled strictly on demand.
struct BitSource<F> {
    next_byte: F,
    current: u8,
    /// Bits consumed of `current`, `0..=8`; `8` means refill first.
    used: u8,
}

impl<F: FnMut() -> Result<u8, Decode>> BitSource<F> {
    fn bit(&mut self) -> Result<bool, Decode> {
        if self.used == 8 {
            self.current = (self.next_byte)()?;
            self.used = 0;
        }
        let bit = self.current & (0x80 >> self.used) != 0;
        self.used += 1;
        Ok(bit)
    }
}

impl Rank {
    /// Parse one canonical stream from a byte source, consuming exactly the
    /// bytes the stream spans.
    ///
    /// The closing bit makes the stream self-delimiting, so this never asks
    /// for a byte belonging to a following field. Allocations grow only from
    /// bits already read, never from a claimed width.
    pub(crate) fn decode_stream(
        next_byte: impl FnMut() -> Result<u8, Decode>,
    ) -> Result<Rank, Decode> {
        let mut src = BitSource {
            next_byte,
            current: 0,
            used: 8,
        };
        // The header's unary run: ρ ones ended by a zero.
        let mut rho = 0u64;
        while src.bit()? {
            rho += 1;
        }
        if rho >= 64 {
            // The format bound: an integral width of 2⁶⁴ or more bits exceeds
            // both the numerator this crate can hold and any input under 2 EiB
            // (the mantissa alone would need 2⁶⁴ − 1 bits).
            return Err(Decode::NotCanonical);
        }
        // w's bits below its (implied) leading bit: ρ of them, so w < 2⁶⁴.
        let mut w = 1u64;
        for _ in 0..rho {
            w = w << 1 | u64::from(src.bit()?);
        }
        // The biased integral m: its implied leading bit, then w − 1 stream
        // bits, sunk MSB-first and unbiased at materialization.
        let mut mantissa = BitSink::new();
        mantissa.push(true);
        for _ in 0..w - 1 {
            mantissa.push(src.bit()?);
        }
        let integral = mantissa.into_num() - 1u32;
        // The fraction's groups, each opened by a set continuation bit; the
        // stream's one clear closing bit ends the loop. Group bytes stay plain
        // `u8`s until the single `BigUint` materialization below.
        let mut groups: Vec<u8> = Vec::new();
        loop {
            if !src.bit()? {
                break;
            }
            let mut group = 0u8;
            for _ in 0..FRACTION_GROUP_BITS {
                group = group << 1 | u8::from(src.bit()?);
            }
            groups.push(group);
        }
        // Strict minimal packing within the final byte: the bits after the close
        // bit are padding and must be zero.
        if src.used < 8 && src.current & (0xFF >> src.used) != 0 {
            return Err(Decode::TrailingBits);
        }
        // The final group carries the expansion's last set bit (normalization:
        // the expansion never ends in zero), so an all-zero final group is pure
        // padding — non-minimal packing — and its trailing zeros locate the
        // fraction's true depth.
        let (frac_len, pad) = match groups.last() {
            None => (0, 0),
            Some(0) => return Err(Decode::TrailingBits),
            Some(&last) => {
                let pad = last.trailing_zeros();
                (
                    groups.len() as u64 * FRACTION_GROUP_BITS - u64::from(pad),
                    pad,
                )
            }
        };
        // The fraction's depth needs no bound of its own: every expansion bit
        // was read from the stream, so `frac_len` never exceeds the input's own
        // bit count and always fits the u64 exponent — an input long enough to
        // overflow it cannot be allocated.
        let exp = frac_len;
        let num = if frac_len == 0 {
            integral
        } else {
            // The numerator by byte assembly, never by a value-width shift:
            // `num · 2^pad = integral · 2^(8·groups) + G` with `G` the groups'
            // big-endian value, and the `pad` low bits shifted out are exactly
            // the final group's trailing zeros — so `num` is the concatenated
            // image's value shifted right by the sub-byte pad. The
            // `integral << exp` spelling is not available at every scale this
            // decoder accepts: on a 32-bit target `exp` outruns `usize` from
            // ~604 MB of input. Leading zero bytes are stripped before
            // materializing so the allocation reflects the value rather than
            // zero padding in its byte image.
            let mut image = integral.to_bytes_be();
            image.extend_from_slice(&groups);
            drop(groups);
            let lead = image.iter().take_while(|&&byte| byte == 0).count();
            BigUint::from_bytes_be(&image[lead..]) >> pad
        };
        debug_assert!(
            exp == 0 || num.bit(0),
            "a nonempty fraction ends in its last set bit, so the numerator is odd"
        );
        Ok(Rank { num, exp })
    }
}

/// An MSB-first bit sink packing into bytes, the final byte zero-padded.
struct BitSink {
    bytes: Vec<u8>,
    /// Bits already used in the final byte, `0..8` (`0` also when empty).
    used: u8,
}

impl BitSink {
    /// An empty sink growing as bits arrive: the decoder's shape,
    /// where preallocating from a header's claimed width would let a
    /// few malicious bytes provoke a large buffer.
    fn new() -> BitSink {
        BitSink {
            bytes: Vec::new(),
            used: 0,
        }
    }

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

    /// The pushed bits as a magnitude, MSB-first.
    ///
    /// The final byte's zero padding is stripped by one shift, and the
    /// materialization does not require an aligned copy.
    ///
    /// The caller's first pushed bit is set (the mantissa's implied
    /// leading one), which is the materialization's no-leading-zero-byte
    /// contract.
    fn into_num(self) -> BigUint {
        let pad = if self.used == 0 { 0 } else { 8 - self.used };
        BigUint::from_bytes_be(&self.bytes) >> u32::from(pad)
    }
}

impl Rank {
    /// The power-of-two range containing this rank; zero has no range.
    fn magnitude_class(&self) -> Option<i128> {
        (self.num.bits() != 0).then(|| i128::from(self.num.bits()) - i128::from(self.exp))
    }

    /// Stream the numerator's binary spelling in left-aligned words.
    ///
    /// Lexicographic iterator order is bit-string order, without materializing
    /// a shifted integer.
    fn aligned_words(&self) -> impl Iterator<Item = u64> + '_ {
        let shift = ((64 - self.num.bits() % 64) % 64) as u32;
        let mut words = self.num.iter_u64_digits().rev();
        let mut current = words.next();
        let mut next = words.next();
        std::iter::from_fn(move || {
            let word = current?;
            let aligned = if shift == 0 {
                word
            } else {
                (word << shift) | (next.unwrap_or(0) >> (64 - shift))
            };
            current = next;
            next = words.next();
            Some(aligned)
        })
    }
}

/// Orders ranks as the exact rationals they are.
///
/// # Complexity
///
/// Unequal magnitude classes settle in `O(1)`:
///
#[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/rank_cmp.html")))]
#[cfg_attr(not(doc), doc = "`O(n)` in total input bytes; `O(‖self‖ + ‖other‖)`")]
impl Ord for Rank {
    fn cmp(&self, other: &Self) -> Ordering {
        // The class orders disjoint power-of-two ranges, with zero first.
        // Within one class, aligned numerator words are the exact binary
        // fractions in lexicographic order.
        self.magnitude_class()
            .cmp(&other.magnitude_class())
            .then_with(|| self.aligned_words().cmp(other.aligned_words()))
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
// reference forms mirror [`BigUint`]'s own `Add` matrix so callers need not place
// borrows by hand.

/// Adds two ranks.
///
/// Rank addition is *measure* arithmetic, not history arithmetic: areas add,
/// histories join. Its meaning comes from the rank being a valuation on the
/// version lattice: `(a | b).rank() + (a & b).rank() == a.rank() + b.rank()`.
/// This is what makes [`distance`](crate::Version::distance) a metric and lets
/// its directed halves recombine (`a.lag(b) + b.lag(a) == a.distance(b)`). Use
/// `+` to aggregate measures (a total replication backlog across peers, a
/// budget consumed so far) not to combine histories.
///
/// # Complexity
///
#[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/rank_add.html")))]
#[cfg_attr(not(doc), doc = "`O(n)` in total input bytes; `O(‖self‖ + ‖rhs‖)`")]
impl Add<&Rank> for &Rank {
    type Output = Rank;
    fn add(self, rhs: &Rank) -> Rank {
        // Shift and add directly when both exponent gaps fit `usize`. The
        // accumulator handles larger gaps without narrowing the exponent.
        let e = self.exp.max(rhs.exp);
        if Rank::alignment_fits(self.exp, rhs.exp, e) {
            let a = self.num.clone() << (e - self.exp);
            let b = rhs.num.clone() << (e - rhs.exp);
            return Rank::from_raw(a + &b, e);
        }
        self.accumulate(rhs, e, false)
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

/// Sums owned ranks, with [`Rank::ZERO`] as the empty sum.
///
/// # Complexity
///
/// For `k` ranks whose binary widths sum to `n`, `O(k + n)` time and `O(n)`
/// space, including the result.
impl Sum<Rank> for Rank {
    fn sum<I: Iterator<Item = Rank>>(iter: I) -> Rank {
        Rank::sum_iter(iter)
    }
}

/// Sums borrowed ranks, with [`Rank::ZERO`] as the empty sum.
///
/// # Complexity
///
/// For `k` ranks whose binary widths sum to `n`, `O(k + n)` time and `O(n)`
/// space, including the result.
impl<'a> Sum<&'a Rank> for Rank {
    fn sum<I: Iterator<Item = &'a Rank>>(iter: I) -> Rank {
        Rank::sum_iter(iter)
    }
}

/// Sums ranks through one accumulator and normalizes once at the end.
///
/// `exp` is the denominator exponent shared by the held numerator. A summand
/// with a smaller exponent enters at the corresponding bit offset. If a
/// summand needs a larger exponent, the running numerator must shift; choosing
/// at least its current bit span may put `exp` beyond that summand, but those
/// extra trailing zeroes disappear during final normalization. Each such shift
/// at least doubles the occupied prefix, so the widths shifted form a geometric
/// series bounded by the final span. Input order therefore cannot multiply the
/// cost by the number of summands.
impl Rank {
    fn sum_iter<T: core::borrow::Borrow<Rank>, I: Iterator<Item = T>>(iter: I) -> Rank {
        // The accumulator's shifted entry points document a panic at digit
        // positions past `usize` (`shift / 32 > usize::MAX`, so from
        // `shift = 2^37` on a 32-bit target). The exponent gaps fed here stay
        // orders of magnitude below it on any addressable input: a decoded
        // rank's exponent is counted from fraction bits actually read — under
        // 2^35 even if a whole 32-bit address space were one fraction — and a
        // version-derived exponent is bounded by its tree's stored bit length
        // (under 2^32, the storage bound), so the documented panic is
        // unreachable from this fold.
        let mut acc = Accumulator::new();
        let mut exp = 0u64;
        let mut has_value = false;
        for rank in iter {
            let rank = rank.borrow();
            if rank.num == BigUint::ZERO {
                continue;
            }
            if !has_value {
                exp = rank.exp;
                accumulator::fold(&mut acc, &rank.num, 0, false);
                has_value = true;
                continue;
            }
            if rank.exp > exp {
                let gap = rank.exp - exp;
                let held_span = accumulator::bit_span(&acc);
                let shift = gap.max(held_span).min(u64::MAX - exp);
                acc.shl(shift);
                exp += shift;
            }
            accumulator::fold(&mut acc, &rank.num, exp - rank.exp, false);
        }
        let (sign, num) = accumulator::value(&acc);
        debug_assert_ne!(
            sign,
            Ordering::Less,
            "a sum of nonnegative ranks is nonnegative"
        );
        Rank::from_raw(num, exp)
    }
}

/// [`Rank::ZERO`], the additive identity.
impl Default for Rank {
    fn default() -> Self {
        Rank::ZERO
    }
}

/// Renders as a canonical binary number with an optional binary point.
///
/// The integer part has no leading zeroes. A fractional part appears exactly
/// when the value is not integral and never ends in zero. Width, fill,
/// alignment, and precision apply to the whole rendered value; sign and
/// sign-aware zero-padding flags have no effect.
///
/// # Complexity
///
/// Writing the canonical value is linear in its binary width, `O(‖r‖)`. When
/// `r = v.rank()`, `‖r‖ = O(|v|)`, where `|v|` is the encoded size of `v`, so
/// rendering is `O(|v|)`. Explicit padding adds time proportional to the
/// padding written.
///
#[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/rank_display.html")))]
#[cfg_attr(
    not(doc),
    doc = "`O(n)` in total input bytes; `O(n)` in the encoded size of the `Version` from which the rank was derived"
)]
///
/// # Example
///
/// ```
/// use before::{Clock, Party, Version};
/// let mut five = Version::new();
/// Party::seed().ticks(&mut five, 5u8);
/// assert_eq!(five.rank().to_string(), "101");
/// let mut half_clock = Clock::seed();
/// let _other_half = half_clock.fork();
/// let half = half_clock.tick().clone();
/// assert_eq!(half.rank().to_string(), "0.1");
/// ```
impl Display for Rank {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Ordinary formatting can stream directly. Padding and precision need
        // the complete text so `Formatter` can align or truncate it.
        if f.width().is_none() && f.precision().is_none() {
            return self.write_binary(f);
        }

        let mut rendered = String::new();
        self.write_binary(&mut rendered)?;
        f.pad(&rendered)
    }
}

impl Rank {
    /// Writes the canonical binary form before formatter padding or
    /// truncation.
    fn write_binary(&self, out: &mut impl fmt::Write) -> fmt::Result {
        let numerator_bits = self.num.bits();
        let integer_bits = numerator_bits.saturating_sub(self.exp);

        // `exp` is the number of digits to the right of the binary point. The
        // more significant numerator bits form the integer part; when there
        // are none, its canonical spelling is `0`.
        if integer_bits == 0 {
            out.write_char('0')?;
        } else {
            for position in (self.exp..numerator_bits).rev() {
                out.write_char(if self.num.bit(position) { '1' } else { '0' })?;
            }
        }

        // The low `exp` numerator bits are the fractional digits. A normalized
        // non-integral rank has a one in bit zero, so this part never ends in
        // a redundant zero.
        if self.exp != 0 {
            out.write_char('.')?;
            for position in (0..self.exp).rev() {
                out.write_char(if self.num.bit(position) { '1' } else { '0' })?;
            }
        }
        Ok(())
    }
}

/// Parses the exact binary form emitted by [`Display`].
///
/// The integer part is nonempty and has no leading zeroes. The optional
/// fractional part is nonempty and ends in `1`. Other digits, signs, and
/// surrounding whitespace are rejected.
///
/// # Complexity
///
/// Linear in the input length, using storage proportional to its binary
/// digits.
impl FromStr for Rank {
    type Err = ParseRank;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        // Keep the absent point distinct from an empty fractional part: `1`
        // is canonical, while `1.` is not.
        let (integer, fraction) = match text.split_once('.') {
            Some((integer, fraction)) => (integer.as_bytes(), Some(fraction.as_bytes())),
            None => (text.as_bytes(), None),
        };

        let all_binary_digits =
            |digits: &[u8]| digits.iter().all(|digit| matches!(digit, b'0' | b'1'));
        let integer_is_canonical = !integer.is_empty()
            && all_binary_digits(integer)
            && (integer.len() == 1 || integer[0] == b'1');
        let fraction_is_canonical = match fraction {
            None => true,
            Some(digits) => {
                !digits.is_empty() && all_binary_digits(digits) && digits.last() == Some(&b'1')
            }
        };
        if !integer_is_canonical || !fraction_is_canonical {
            return Err(ParseRank);
        }

        let fraction = fraction.unwrap_or_default();
        let exponent = u64::try_from(fraction.len()).map_err(|_| ParseRank)?;
        let digit_count = integer.len() + fraction.len();
        let bits_per_digit = u32::BITS as usize;
        let mut digits = vec![0u32; digit_count.div_ceil(bits_per_digit)];

        // Text is most-significant-bit first, while `BigUint::new` takes
        // little-endian base-2^32 digits. The rightmost text digit is therefore
        // bit zero regardless of where the point appeared.
        for (offset, digit) in integer.iter().chain(fraction).enumerate() {
            if *digit == b'1' {
                let position = digit_count - offset - 1;
                let word = position / bits_per_digit;
                let bit = position % bits_per_digit;
                digits[word] |= 1u32 << bit;
            }
        }

        Ok(Rank {
            num: BigUint::new(digits),
            exp: exponent,
        })
    }
}

/// The same format as `Display`.
impl Debug for Rank {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <Self as Display>::fmt(self, f)
    }
}
