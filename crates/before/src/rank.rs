//! Exact causal ranks and their order-preserving byte encoding.
//!
//! A [`Rank`] is the exact area represented by a [`Version`](crate::Version),
//! expressed as a nonnegative dyadic rational. Area is strictly monotone in
//! causal order, so rank can order causes before their effects even though it
//! does not uniquely identify a version.
//!
//! # Canonical representation
//!
//! A rank is stored as `num / 2^exp`. The numerator is odd unless the value is
//! zero, whose exponent is also zero. This normalization gives every value one
//! in-memory representation and makes structural equality exact numeric
//! equality.
//!
//! # Canonical byte encoding
//!
//! The byte encoding has two consecutive parts, written most-significant bit
//! first:
//!
//! 1. Let `I` be the integer part, `m = I + 1`, `w = bits(m)`, and
//!    `rho = bits(w) - 1`. The stream contains `rho` one bits, a zero bit, the
//!    `rho` low bits of `w`, and the `w - 1` low bits of `m`. This is an Elias
//!    delta code whose initial run is inverted so that larger integers sort
//!    after smaller ones. The bias gives zero a code, while the nested width
//!    supports arbitrary magnitudes in `N + O(log N)` bits.
//! 2. The fractional binary digits follow in eight-bit groups. A one bit opens
//!    each group, its eight digits follow, and a zero bit closes the fraction.
//!    The last group is padded with zeros. Because a normalized fractional
//!    numerator is odd, the last meaningful digit is one; the decoder can
//!    therefore distinguish its trailing padding from value digits.
//!
//! Fraction digits must remain in comparison order: fraction length alone does
//! not determine value (`0.1` is greater than `0.0111`). The per-group marker
//! keeps length information beside the digits it qualifies. It also ensures
//! that a fraction sorts before any proper extension of itself.
//!
//! Together, the integer code and fractional groups make byte-wise
//! lexicographic order equal [`Ord`] on ranks. The closing marker makes the stream
//! self-delimiting, and two distinct ranks always differ before either stream
//! ends. Appending arbitrary suffixes therefore cannot change their order.
//!
//! Decoding is strict and incremental. Storage grows only in proportion to
//! bytes already read, never from an untrusted length claim. It rejects
//! truncation, bytes or set padding bits after the closing marker, an all-zero
//! final fraction group, and an integer-width header that cannot fit the
//! representation.

use core::cmp::Ordering;
use core::fmt::{self, Alignment, Debug, Display};
use core::iter::Sum;
use core::ops::{Add, AddAssign};
use core::str::FromStr;
use std::io::{self, Read, Write};

use num_bigint::BigUint;
use suanpan::Accumulator;

use crate::accumulator::BigIntAccumulator as _;
use crate::error::{Decode, ParseRank};

/// An exact nonnegative measure of causal history.
///
/// Conceptually, a [`Version`](crate::Version) records an event count at every
/// point of the unit interval. Its rank is the exact area under that function.
/// The result is a dyadic rational and is strictly monotone in causal order:
///
/// ```text
/// v < w  implies  v.rank() < w.rank()
/// ```
///
/// Rank is therefore useful when a total order must place causes before their
/// effects. Concurrent versions may have equal ranks, so
/// [`Ranked`](crate::Ranked) adds a deterministic tiebreak when each version
/// needs a distinct position.
///
/// # Canonical encoding
///
/// [`Rank::encode`] produces canonical bytes whose lexicographic order
/// equals [`Rank`]'s numeric order. The encoding is also self-delimiting: an
/// arbitrary suffix may be appended to each encoded rank without changing the
/// order between distinct ranks. This makes the encoding suitable as the first
/// component of a composite key in a byte-ordered store.
///
/// ```
/// use before::{Party, Version};
/// let mut p = Party::seed();
/// let q = p.fork();
/// let mut v = Version::new();
/// let mut w = Version::new();
/// p.tick(&mut v);
/// p.tick(&mut w);
/// q.tick(&mut w);
/// assert!(w.rank() > v.rank());
/// assert!(w.rank().encode() > v.rank().encode());
/// ```
///
/// # Complexity
///
/// Write `‖r‖ = bits(numerator) + exponent` for a rank's binary width, and
/// `|v|` for a version's size. For `r = v.rank()`, `‖r‖ = O(|v|)`, though it
/// may be much smaller. The rank's memory use and canonical byte length are
/// `O(‖r‖)`.
///
/// Comparison, equality, hashing, and cloning take `O(‖r‖)` time. Addition
/// and subtraction take `O(‖a‖ + ‖b‖)` time. Encoding and decoding are linear
/// in the bytes written or read.
///
/// # Rank and minimum ticks
///
/// [`Version::min_ticks`](crate::Version::min_ticks) is a lower bound on how
/// many events could have produced a version. Rank instead measures how those
/// events cover the unit interval, so it can distinguish versions with the
/// same minimum event count:
///
/// ```
/// use before::{Party, Version};
/// let mut p = Party::seed();
/// let q = p.fork();
/// let mut v = Version::new();
/// let mut w = Version::new();
/// p.tick(&mut v);
/// p.tick(&mut w);
/// q.tick(&mut w);
/// assert_eq!(v.min_ticks(), w.min_ticks());
/// assert!(v.rank() < w.rank());
/// ```
#[derive(Clone, PartialEq, Eq, Hash)]
#[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscape-assets.html")))]
pub struct Rank {
    /// The numerator. Normalized: odd, or zero with `exp` zero, so each
    /// value has exactly one representation.
    ///
    num: BigUint,
    /// The binary exponent of the denominator `2^exp`.
    ///
    /// For a [`Rank`] computed from a [`Version`](crate::Version), this is
    /// bounded by the version's tree depth, which cannot feasibly exceed 2^64.
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
    #[must_use = "`Rank::checked_sub` does not modify `self` or `other`; discarding its result means that it has no effect"]
    pub fn checked_sub(&self, other: &Rank) -> Option<Rank> {
        // The ordering pre-check rides the class-first comparison, so the
        // `None` and zero arms cost no alignment at all; only a strictly
        // positive difference aligns to the common exponent and subtracts, and
        // that transient is the output's own value content.
        match self.cmp(other) {
            Ordering::Less => None,
            Ordering::Equal => Some(Rank::ZERO),
            Ordering::Greater => {
                // Aligned as in `&Rank + &Rank`, whose comment gives the
                // argument that no step depends on the width of `usize`. Here
                // each shifted numerator is at most one bit wider than the
                // difference or an unshifted numerator.
                let exp = self.exp.max(other.exp);
                let a = &self.num << (exp - self.exp);
                let b = &other.num << (exp - other.exp);
                Some(Rank::from_raw(a - &b, exp))
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
    #[must_use = "`Rank::saturating_sub` does not modify `self` or `other`; discarding its result means that it has no effect"]
    pub fn saturating_sub(&self, other: &Rank) -> Rank {
        self.checked_sub(other).unwrap_or(Rank::ZERO)
    }

    /// Encodes this rank into its canonical, order-preserving bytes.
    ///
    /// Byte-wise lexicographic order on encoded ranks equals [`Ord`] on the
    /// values. A byte-ordered store can therefore use the encoding as a key
    /// prefix while still placing causes before their effects.
    ///
    /// Versions with equal ranks are identical or concurrent, so any
    /// deterministic suffix is a causally safe tiebreak. [`Ranked`] provides a
    /// canonical choice.
    ///
    /// The encoding is self-delimiting. Appending arbitrary suffixes to encoded
    /// ranks cannot change the order between distinct values.
    ///
    /// # Serialized size
    ///
    /// The representation uses at most `9⁄8 · ‖r‖ + O(log ‖r‖)` bits: one
    /// bit per integral bit and nine bits per eight fractional bits.
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
        let mut bytes = Vec::new();
        Self::encode_parts_to(&self.num, self.exp, &mut bytes)
            .expect("writing to a Vec cannot fail");
        bytes
    }

    /// Encodes this rank to an arbitrary writer: exactly
    /// [`encode`](Rank::encode)'s canonical bytes, without an up-front
    /// whole allocation.
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
        Self::encode_parts_to(&self.num, self.exp, writer)
    }

    /// Decodes one rank from a reader.
    ///
    /// A successful decode requires exactly one canonical encoding; trailing
    /// bytes are an error.
    ///
    /// # Errors
    ///
    /// - [`Decode::Truncated`] when the stream ends inside the integral header,
    ///   its mantissa, or the fraction (a group or its continuation bit);
    /// - [`Decode::TrailingBits`] when the byte string is not the minimal packing
    ///   of its content (bytes past the stream's own, a set bit in the padding,
    ///   or an all-zero final fraction group);
    /// - [`Decode::NotCanonical`] when the integral header declares a mantissa
    ///   width outside the format's `u64` range;
    /// - [`Decode::Io`] with the first error the reader returns other than
    ///   [`Interrupted`](io::ErrorKind::Interrupted), which the decoder
    ///   retries, as [`Read::read_exact`] does.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/rank_decode.html")))]
    #[cfg_attr(
        not(doc),
        doc = "`O(n)` in total input bytes; `O(n)`, `n` the bytes read, accepted or rejected"
    )]
    ///
    /// The decoder reads incrementally and does not retain a copy of the encoded
    /// input.
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
    pub fn decode<R: Read>(mut reader: R) -> Result<Rank, Decode> {
        let mut buf = [0; DECODE_CHUNK_BYTES];
        let mut end = 0;
        let at_eof = loop {
            match reader.read(&mut buf[end..]) {
                Ok(0) => break true,
                Ok(read) => {
                    end += read;
                    if end == buf.len() {
                        break false;
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) => return Err(Decode::Io(error)),
            }
        };

        // Most inputs end within the fixed prefix. Decode those through the
        // simpler slice path; only an incomplete full prefix needs incremental
        // reading. Retrying a bounded prefix keeps that slow path linear.
        match Self::decode_bytes(&buf[..end]) {
            Ok(rank) if at_eof => return Ok(rank),
            Ok(rank) => loop {
                match reader.read(&mut buf[..1]) {
                    Ok(0) => return Ok(rank),
                    Ok(_) => return Err(Decode::TrailingBits),
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                    Err(error) => return Err(Decode::Io(error)),
                }
            },
            Err(Decode::Truncated) if at_eof => return Err(Decode::Truncated),
            Err(Decode::Truncated) => {}
            Err(error) => return Err(error),
        }

        let mut next = 0;
        let rank = Self::decode_stream(|| loop {
            if next < end {
                let byte = buf[next];
                next += 1;
                return Ok(byte);
            }
            match reader.read(&mut buf) {
                Ok(0) => return Err(Decode::Truncated),
                Ok(read) => {
                    next = 0;
                    end = read;
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) => return Err(Decode::Io(error)),
            }
        })?;

        if next < end {
            return Err(Decode::TrailingBits);
        }
        loop {
            match reader.read(&mut buf[..1]) {
                Ok(0) => return Ok(rank),
                Ok(_) => return Err(Decode::TrailingBits),
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) => return Err(Decode::Io(error)),
            }
        }
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
    /// The meter denominator for `Rank` operands, whose value is not stored as
    /// bytes: the numerator's bit width plus the exponent bounds the
    /// information the value carries. Every public construction path emits
    /// ranks whose content is linear in its input, so a cost linear in this
    /// quantity is linear in input bytes too.
    #[cfg(any(test, feature = "meter"))]
    pub(crate) fn content_bits(&self) -> u64 {
        self.num.bits() + self.exp
    }

    /// The stored parts `(numerator, exponent)`.
    ///
    /// The raw normalized form that reference computations and differential
    /// oracles use to re-derive order and arithmetic.
    ///
    /// It is **VERY IMPORTANT** that these not be exposed together, with the
    /// `from_raw` constructor, as this creates an affordance for constructing
    /// exponential serialization-size bombs.
    #[cfg(any(test, feature = "meter"))]
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

    /// Write the canonical prefix-ascending stream for `num · 2⁻ᵉˣᵖ`.
    fn encode_parts_to<W: Write>(num: &BigUint, exp: u64, writer: &mut W) -> io::Result<()> {
        // The header encodes the integer part: m = ⌊r⌋ + 1,
        // w = bits(m), ρ = bits(w) − 1. A shift past the numerator's width
        // yields zero, meaning only that r < 1; the fraction loop below still
        // writes the remaining value exactly.
        let biased = (num >> exp) + 1u32;
        let w = biased.bits();
        let rho = u64::from(63 - w.leading_zeros());
        let groups = exp.div_ceil(FRACTION_GROUP_BITS);
        let mut sink = BitWriter::new(writer);
        // The header: ρ ones, the terminating zero, then w's bits below its
        // leading bit — the Elias delta length header with the run's bit
        // sense inverted, so longer (larger) integral parts sort after
        // shorter ones instead of before.
        for _ in 0..rho {
            sink.push(true)?;
        }
        sink.push(false)?;
        for i in (0..rho).rev() {
            sink.push(w >> i & 1 == 1)?;
        }
        // The integral mantissa: m's bits below its leading bit.
        for i in (0..w - 1).rev() {
            sink.push(biased.bit(i))?;
        }
        // The fraction: the binary expansion (expansion bit `j`, counting
        // from the binary point, is the numerator's bit `exp − j`) in
        // groups of eight, each opened by a set continuation bit, the last
        // zero-padded; a clear bit closes the stream. Normalization (an odd
        // numerator whenever exp > 0) puts the expansion's final set
        // bit inside the last group, which keeps the padding recoverable
        // and the group order numeric (the module doc's argument).
        for g in 0..groups {
            sink.push(true)?;
            for j in g * FRACTION_GROUP_BITS + 1..=(g + 1) * FRACTION_GROUP_BITS {
                sink.push(j <= exp && num.bit(exp - j))?;
            }
        }
        sink.push(false)?;
        sink.finish()
    }

    /// Whether `expected` is this rank's canonical encoding.
    ///
    /// Comparison streams the encoding into a constant-space sink rather than
    /// materializing another copy.
    pub(crate) fn encoding_matches(&self, expected: &[u8]) -> bool {
        let mut writer = MatchingWriter::new(expected);
        self.encode_to(&mut writer)
            .expect("the in-memory comparison writer cannot fail");
        writer.matches()
    }
}

/// The width of one fraction group: the expansion rides in byte-sized groups,
/// each opened by a continuation bit, so the fraction costs nine bits per eight
/// expansion bits plus the one closing bit.
const FRACTION_GROUP_BITS: u64 = 8;

/// Bytes examined directly before decoding falls back to incremental reads.
///
/// Retrying this fixed prefix bounds the extra work independently of input size.
///
/// `pub(crate)` so the reader tests can end rank encodings beside the end
/// of the prefix and of the first refill.
pub(crate) const DECODE_CHUNK_BYTES: usize = 64;

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
            // The format stores the integral width in `u64`; a longer header
            // cannot name a representable width.
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
        // The fraction follows as 8-bit groups, each preceded by a 1 bit; a 0
        // bit ends it. With `g` groups whose bytes read big-endian as `G`, the
        // rank is `integral + G / 2^(8g)`, so its numerator over `2^(8g)` is
        // `integral * 2^(8g) + G`. That number's big-endian bytes are the
        // integral part's bytes followed by the group bytes, so we collect
        // exactly those, in `image`, as the groups arrive. `image` is created
        // by the first group, so a rank without a fraction never allocates it,
        // and a zero integral part adds no bytes to it.
        let mut image: Option<Vec<u8>> = None;
        let mut integral_len = 0;
        loop {
            if !src.bit()? {
                break;
            }
            let mut group = 0u8;
            for _ in 0..FRACTION_GROUP_BITS {
                group = group << 1 | u8::from(src.bit()?);
            }
            image
                .get_or_insert_with(|| {
                    let seed = if integral == BigUint::ZERO {
                        Vec::new()
                    } else {
                        integral.to_bytes_be()
                    };
                    integral_len = seed.len();
                    seed
                })
                .push(group);
        }
        let mut image = image.unwrap_or_default();
        // Strict minimal packing within the final byte: the bits after the close
        // bit are padding and must be zero.
        if src.used < 8 && src.current & (0xFF >> src.used) != 0 {
            return Err(Decode::TrailingBits);
        }
        // The final group carries the expansion's last set bit (normalization:
        // the expansion never ends in zero), so an all-zero final group is pure
        // padding — non-minimal packing — and its trailing zeros locate the
        // fraction's true depth.
        let groups = &image[integral_len..];
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
            // `image` holds `integral * 2^(8g) + G`. Its low `pad` bits are the
            // final group's trailing zeros, which are padding, so the numerator
            // over `2^exp` is that value shifted right by `pad`. Building it
            // from bytes avoids computing `integral << exp`, whose shift amount
            // can exceed `usize` on a 32-bit target. `from_bytes_be` would copy
            // the image first, so we reverse it in place and read it
            // little-endian, trimming high zero bytes so the allocation fits
            // the value.
            drop(integral);
            image.reverse();
            let len = image
                .iter()
                .rposition(|&byte| byte != 0)
                .map_or(0, |top| top + 1);
            BigUint::from_bytes_le(&image[..len]) >> pad
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

    fn push(&mut self, bit: bool) {
        if self.used == 0 {
            self.bytes.push(0);
        }
        if bit {
            *self.bytes.last_mut().expect("just ensured nonempty") |= 1 << (7 - self.used);
        }
        self.used = (self.used + 1) % 8;
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

/// Bytes staged before rank encoding writes to its destination.
const ENCODE_BUFFER_BYTES: usize = 256;

/// An MSB-first bit writer with fixed-size staging storage.
///
/// Staging bounds auxiliary space independently of the encoded rank while
/// avoiding one write call per output byte.
struct BitWriter<'a, W> {
    writer: &'a mut W,
    bytes: [u8; ENCODE_BUFFER_BYTES],
    len: usize,
    current: u8,
    used: u8,
}

impl<'a, W: Write> BitWriter<'a, W> {
    /// Construct an empty writer.
    fn new(writer: &'a mut W) -> Self {
        BitWriter {
            writer,
            bytes: [0; ENCODE_BUFFER_BYTES],
            len: 0,
            current: 0,
            used: 0,
        }
    }

    /// Append one bit, flushing full staging buffers as needed.
    fn push(&mut self, bit: bool) -> io::Result<()> {
        if bit {
            self.current |= 1 << (7 - self.used);
        }
        self.used += 1;
        if self.used == 8 {
            self.push_byte()?;
        }
        Ok(())
    }

    /// Stage the completed current byte.
    fn push_byte(&mut self) -> io::Result<()> {
        if self.len == self.bytes.len() {
            self.flush()?;
        }
        self.bytes[self.len] = self.current;
        self.len += 1;
        self.current = 0;
        self.used = 0;
        Ok(())
    }

    /// Write staged bytes without flushing the caller's writer.
    fn flush(&mut self) -> io::Result<()> {
        self.writer.write_all(&self.bytes[..self.len])?;
        self.len = 0;
        Ok(())
    }

    /// Pad the final byte with zeros and write all remaining bytes.
    fn finish(mut self) -> io::Result<()> {
        if self.used != 0 {
            self.push_byte()?;
        }
        self.flush()
    }
}

/// A writer that compares bytes with one expected slice.
struct MatchingWriter<'a> {
    expected: &'a [u8],
    written: usize,
    equal: bool,
}

impl<'a> MatchingWriter<'a> {
    /// Begin comparison at the start of `expected`.
    fn new(expected: &'a [u8]) -> Self {
        MatchingWriter {
            expected,
            written: 0,
            equal: true,
        }
    }

    /// Whether every written byte matched and the lengths are equal.
    fn matches(&self) -> bool {
        self.equal && self.written == self.expected.len()
    }
}

impl Write for MatchingWriter<'_> {
    /// Compare one emitted chunk and accept it in full.
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let end = self.written.saturating_add(bytes.len());
        self.equal &= self
            .expected
            .get(self.written..end)
            .is_some_and(|expected| expected == bytes);
        self.written = end;
        Ok(bytes.len())
    }

    /// No buffering is owned at this layer.
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
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

/// Orders ranks by their exact numeric values.
///
/// # Complexity
///
/// Ranks in different power-of-two ranges compare in `O(1)`. Ranks in the same
/// range take `O(‖self‖ + ‖other‖)` time in the worst case.
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
        // Align both numerators to the larger exponent. Each gap is a `u64`
        // bit count, and `BigUint`'s `u64` shift takes it without narrowing:
        // the `usize` values it derives from the gap, the count of zero
        // digits to prepend and the length of the shifted buffer, are lengths
        // the shifted value must hold in memory. A shifted numerator is no
        // wider than the sum, so that count outgrows `usize` only for a sum
        // no memory could hold: alignment computes the same values whatever
        // the width of `usize`. Shifting by reference copies each numerator
        // once, with no separate clone.
        let exp = self.exp.max(rhs.exp);
        let a = &self.num << (exp - self.exp);
        let b = &rhs.num << (exp - rhs.exp);
        Rank::from_raw(a + &b, exp)
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
                acc.add_shifted_limbs(0, rank.num.iter_u64_digits());
                has_value = true;
                continue;
            }
            if rank.exp > exp {
                let gap = rank.exp - exp;
                let held_span = acc.stored_bits();
                let shift = gap.max(held_span).min(u64::MAX - exp);
                acc <<= shift;
                exp += shift;
            }
            acc.add_shifted_limbs(exp - rank.exp, rank.num.iter_u64_digits());
        }
        let (sign, num) = acc.biguint_parts();
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
/// `r = v.rank()`, `‖r‖ = O(|v|)`, where `|v|` is the size of `v`, so
/// rendering is `O(|v|)`. Explicit padding adds time proportional to the
/// padding written. Precision limits work to the retained prefix. Formatting
/// uses constant auxiliary space.
///
#[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/rank_display.html")))]
#[cfg_attr(
    not(doc),
    doc = "`O(n)` in total input bytes; `O(n)` in the size of the `Version` from which the rank was derived"
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
        // The spelling is ASCII, so its character count follows directly from
        // the numerator width and exponent. Knowing the count lets us apply
        // string-style padding and truncation without building a temporary
        // String.
        let content_len = self
            .text_len()
            .min(f.precision().map_or(u64::MAX, |precision| precision as u64));
        let padding = (f.width().unwrap_or(0) as u64).saturating_sub(content_len);
        let (left, right) = match f.align().unwrap_or(Alignment::Left) {
            Alignment::Left => (0, padding),
            Alignment::Right => (padding, 0),
            Alignment::Center => (padding / 2, padding - padding / 2),
        };

        Self::write_padding(f, left)?;
        self.write_binary(f, content_len)?;
        Self::write_padding(f, right)
    }
}

impl Rank {
    /// Number of characters in the canonical binary form.
    fn text_len(&self) -> u64 {
        let numerator_bits = self.num.bits();
        let integer_bits = numerator_bits.saturating_sub(self.exp);
        let integer_len = integer_bits.max(1);
        integer_len.saturating_add(if self.exp == 0 {
            0
        } else {
            self.exp.saturating_add(1)
        })
    }

    /// Writes at most `remaining` characters of the canonical binary form.
    fn write_binary(&self, out: &mut impl fmt::Write, mut remaining: u64) -> fmt::Result {
        let numerator_bits = self.num.bits();
        let integer_bits = numerator_bits.saturating_sub(self.exp);

        let mut write = |character| {
            if remaining == 0 {
                return Ok(false);
            }
            out.write_char(character)?;
            remaining -= 1;
            Ok(true)
        };

        // `exp` is the number of digits to the right of the binary point. The
        // more significant numerator bits form the integer part; when there
        // are none, its canonical spelling is `0`.
        if integer_bits == 0 {
            write('0')?;
        } else {
            for position in (self.exp..numerator_bits).rev() {
                if !write(if self.num.bit(position) { '1' } else { '0' })? {
                    return Ok(());
                }
            }
        }

        // The low `exp` numerator bits are the fractional digits. A normalized
        // non-integral rank has a one in bit zero, so this part never ends in
        // a redundant zero.
        if self.exp != 0 && write('.')? {
            for position in (0..self.exp).rev() {
                if !write(if self.num.bit(position) { '1' } else { '0' })? {
                    break;
                }
            }
        }
        Ok(())
    }

    /// Writes `count` copies of the formatter's fill character.
    fn write_padding(f: &mut fmt::Formatter<'_>, count: u64) -> fmt::Result {
        let fill = f.fill();
        for _ in 0..count {
            fmt::Write::write_char(f, fill)?;
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
/// # Errors
///
/// Returns [`ParseRank`] unless the input is the canonical binary form
/// described above.
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
        let stored_digit_count = integer.len() + fraction.len();
        let bits_per_digit = u32::BITS as usize;
        let mut digits = vec![0u32; stored_digit_count.div_ceil(bits_per_digit)];

        // Text is most-significant-bit first, while `BigUint::new` takes
        // little-endian base-2^32 digits. The rightmost text digit is therefore
        // bit zero regardless of where the point appeared.
        for (offset, digit) in integer.iter().chain(fraction).enumerate() {
            if *digit == b'1' {
                let position = stored_digit_count - offset - 1;
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
