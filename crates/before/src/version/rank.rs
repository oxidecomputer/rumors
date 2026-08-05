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
//! lexicographic order equals the ranks' numeric order even when each
//! stream is followed by arbitrary further bits, because two distinct
//! ranks' streams always differ at a bit position inside both — packed
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
//!    otherwise dsi's own.
//!
//!    *Why delta among its siblings.* The trade is size-pin margin
//!    against proof surface. Gamma spends its unary run re-paying the
//!    mantissa's whole width — `2N` bits for an `N`-bit integral —
//!    and a [`Version`](crate::Version) already stores its counters
//!    gamma-coded, so a gamma integral part would pay that doubled
//!    width *again* and drive the worst committed provenance family
//!    (the lone wide counter, measured at 0.56 encoded bits per
//!    packed input bit) up against the 1.0-per-family
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
//! The closing bit makes every stream self-delimiting, so distinct
//! ranks' streams are never prefixes of one another: they differ at a
//! bit position **inside both**, the padded byte forms differ at that
//! byte, and neither byte string is a byte prefix of the other. Hence
//! zero-padding can create neither ties nor inversions, **byte-wise
//! lexicographic order on encodings equals [`Ord`] on ranks**, and no
//! appended suffix can flip the order between distinct ranks — the
//! laws the committed sweep and proptests pin.
//!
//! Every piece of the stream is forced: the `I + 1` bias does two
//! jobs — it gives zero (a codeless value in the delta family) the
//! smallest codeword, and it keeps `m ≥ 1` so the leading bits of
//! both `m` and `w` stay implied, which is what makes the header
//! bijective (each `(ρ, payload)` pair decodes to a width `w` whose
//! own width is exactly `ρ + 1`, so non-minimal headers are
//! unrepresentable and no rejection genre exists for them) — and
//! the fraction's depth is recovered from the final group's last set
//! bit (a "fraction with trailing zeros" is not expressible — inside
//! the final group those bits *are* the padding, and spilling them
//! into a further group leaves that group all-zero, which the decoder
//! rejects as non-minimal). The decoder rejects exactly: truncation
//! (the unary run, the header payload, the mantissa, a group, or its
//! continuation bit running off the end), non-minimal packing (byte
//! length beyond the stream's own, a set bit in the padding, or an
//! all-zero final group), and the format's one representation bound
//! (an integral mantissa of `2⁶⁴` or more bits — beyond any rank this
//! crate can hold, and beyond any input under 2 EiB; the fraction's
//! depth is counted from bits actually read, so it can never outrun
//! the exponent that stores it).

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
/// for sorted-container keys that must deliver causes before effects
/// (the [`Ranked`](crate::Ranked) view builds exactly such a total
/// order in, with the version's bytes as the tiebreak).
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
/// Comparison and addition `O(‖a‖ + ‖b‖)`, `Sum` `O(N)`; `Display`
/// superlinear in the numerator width (decimal conversion).
/// A rank's costs are denominated in its *numeric size* `‖r‖` — the
/// numerator's bit width plus the exponent — which every
/// producing fold ([`Version::rank`](crate::Version::rank),
/// [`distance`](crate::Version::distance), [`lag`](crate::Version::lag))
/// keeps linear in the packed bits it read, and which
/// [`encode`](Rank::encode) makes tangible: the canonical byte form is
/// at most `9⁄8 · ‖r‖ + O(log ‖r‖)` bits. Comparison (`==`, [`Ord`])
/// answers in `O(1)` when the two magnitudes differ in scale and
/// allocates nothing on scale ties; hashing and cloning are `O(‖r‖)`.
/// An n-ary [`Sum`]'s `N` is the summands' total numeric size: the fold
/// carries one running accumulator, and each summand pays its own width
/// rather than the accumulator's. Rendering (`Display`) is `O(d)` space
/// in the `d` decimal digits printed, but its time additionally pays
/// binary-to-decimal conversion of the numerator, superlinear (though
/// subquadratic) in its width past a machine word.
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
    exp: u64,
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
    /// `O(‖a‖ + ‖b‖)`, the operands' numeric sizes.
    /// The denomination is the operands' numeric size (see [the type's
    /// note](Rank#complexity)); a [`None`] or zero result costs only the
    /// comparison, which allocates nothing.
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
    /// causally safe. Reach for this form when rank-*class* semantics
    /// are the point (rank-equal versions sharing a key prefix under a
    /// tiebreak of your own); where the version itself should be the
    /// tiebreak and the key should determine the value,
    /// [`Ranked::encode`](crate::Ranked::encode) is the ready-made
    /// composite.
    ///
    /// The encoding is canonical and bijective: [`decode`](Rank::decode)
    /// accepts exactly the byte strings this method produces, and equal
    /// ranks encode byte-identically (distinct ranks never collide).
    /// The format itself — the prefix-ascending integral code, the
    /// group-framed fraction, the self-delimiting close that makes the
    /// streams prefix-free — is the module's business; callers need
    /// only the law above.
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
    /// The output is at most `9⁄8 · ‖r‖ + O(log ‖r‖)` bits — one bit
    /// per integral bit, nine bits per eight fractional digits (the
    /// framing that keeps distinct ranks' encodings prefix-free, which
    /// is what suffix safety costs). Every rank *reachable from this
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
    /// `O(‖r‖)` time and space; the output is at most `9⁄8 · ‖r‖ + O(log ‖r‖)` bits.
    /// The denomination is the rank's numeric size (see [the type's
    /// note](Rank#complexity)).
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

    /// Encodes this rank to an arbitrary writer: exactly
    /// [`encode`](Rank::encode)'s canonical bytes, without handing the
    /// caller the intermediate `Vec`.
    ///
    /// The stream is bit-packed, so the bytes are assembled in one
    /// internal buffer and delivered to the writer in a single
    /// `write_all` — the writer sees exactly what `encode` returns.
    ///
    /// # Errors
    ///
    /// Whatever the writer itself reports; the encoding side is
    /// infallible.
    ///
    /// # Complexity
    ///
    /// `O(‖r‖)` time and space; the output is at most `9⁄8 · ‖r‖ + O(log ‖r‖)` bits.
    /// The denomination is the rank's numeric size (see [the type's
    /// note](Rank#complexity)).
    ///
    /// ```
    /// use before::Version;
    /// let rank = Version::try_from(5).unwrap().rank();
    /// let mut buf = Vec::new();
    /// rank.encode_to(&mut buf).unwrap();
    /// assert_eq!(buf, rank.encode());
    /// ```
    pub fn encode_to<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_all(&encode_parts(&self.num, self.exp))
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
    /// header, its mantissa, or the fraction (a group or its
    /// continuation bit); [`Decode::TrailingBits`] when the byte
    /// string is not the minimal packing of its content (bytes past
    /// the stream's own, a set bit in the padding, or an all-zero
    /// final fraction group — the one spelling a trailing-zero
    /// fraction could take); [`Decode::NotCanonical`] when the stream
    /// carries content past the format's representation bound (an
    /// integral mantissa of `2⁶⁴` or more bits — reachable only
    /// through inputs of 2 EiB or more); [`Decode::Io`] when the
    /// reader itself fails.
    ///
    /// # Complexity
    ///
    /// `O(n)`.
    /// `n` is the bytes read, accepted or rejected: one pass over the
    /// stream, and a result linear in it.
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
        self.num.bits() + self.exp
    }

    /// The stored parts `(numerator, exponent)`.
    ///
    /// The fused encode's hand-off from a rank fold's output to the
    /// canonical emission ([`encode_parts`]), and the raw normalized
    /// form the reference computations and differential oracles
    /// re-derive order and arithmetic from.
    pub(crate) fn raw_parts(&self) -> (&Base, u64) {
        (&self.num, self.exp)
    }

    /// Normalize raw fold output `num · 2⁻ᵉˣᵖ` into canonical form: strip
    /// the factors of two shared by numerator and denominator, and pin zero
    /// to exponent zero, so structural equality is value equality.
    ///
    /// `pub(crate)` for the reference computations (the oracle's tree fold,
    /// the semantic oracle's Riemann sum), which produce the same raw form.
    pub(crate) fn from_raw(num: Base, exp: u64) -> Self {
        match num.trailing_zeros() {
            None => Rank {
                num: Base::ZERO,
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
}

/// Emit the canonical prefix-ascending stream for `num · 2⁻ᵉˣᵖ` (the
/// module doc carries the format and the order argument).
///
/// `pub(crate)` alongside [`Rank::encode`] so the ranked view's fused
/// emission can emit straight from its rank fold's
/// `(numerator, exponent)` output, with no walk beyond the fold's own.
pub(crate) fn encode_parts(num: &Base, exp: u64) -> Vec<u8> {
    // The integral part, biased so zero has a (smallest) codeword:
    // m = ⌊r⌋ + 1, w = bits(m), ρ = bits(w) − 1.
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
    // zero-padded; a clear bit closes the stream. Normalization (an
    // odd numerator whenever exp > 0) puts the expansion's final set
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

/// The width of one fraction group: the expansion rides in byte-sized
/// groups, each opened by a continuation bit, so the fraction costs
/// nine bits per eight expansion bits plus the one closing bit.
const FRACTION_GROUP_BITS: u64 = 8;

/// Parse one canonical stream from the whole input (strictly:
/// [`Rank::decode`]'s contract): the stream itself, then padding to
/// the byte boundary and not a byte more.
fn decode_bytes(bytes: &[u8]) -> Result<Rank, Decode> {
    let mut iter = bytes.iter();
    let rank = decode_stream(|| iter.next().copied().ok_or(Decode::Truncated))?;
    if iter.next().is_some() {
        // `decode` handed over the whole input, so bytes past the
        // self-delimited stream are non-minimal packing.
        return Err(Decode::TrailingBits);
    }
    Ok(rank)
}

/// A byte-at-a-time source dressed as an MSB-first bit reader: one
/// byte buffered, refilled strictly on demand.
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

/// Parse one canonical stream from a byte-at-a-time source, consuming
/// exactly the bytes the stream spans.
///
/// The stream is self-delimiting (the fraction's close bit), so the
/// parse never asks for a byte past the one holding the close bit —
/// which is what lets a rank compose inside a larger stream (the
/// `borsh` boundary): the bytes after it belong to the next field.
/// Every allocation is fed by bits actually read, never by a width a
/// header merely claims, so no small input can provoke a large
/// buffer. Strictness is [`Rank::decode`]'s except whole-input
/// minimality — a caller that owns the input's end rejects leftover
/// bytes itself ([`decode_bytes`]).
pub(crate) fn decode_stream(next_byte: impl FnMut() -> Result<u8, Decode>) -> Result<Rank, Decode> {
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
        // The format bound: an integral width of 2⁶⁴ or more bits
        // exceeds both the numerator this crate can hold and any input
        // under 2 EiB (the mantissa alone would need 2⁶⁴ − 1 bits).
        return Err(Decode::NotCanonical);
    }
    // w's bits below its (implied) leading bit: ρ of them, so w < 2⁶⁴.
    let mut w = 1u64;
    for _ in 0..rho {
        w = w << 1 | u64::from(src.bit()?);
    }
    // The biased integral m: its implied leading bit, then w − 1
    // stream bits, sunk MSB-first and unbiased at materialization.
    let mut mantissa = BitSink::new();
    mantissa.push(true);
    for _ in 0..w - 1 {
        mantissa.push(src.bit()?);
    }
    let integral = mantissa.into_base() - &Base::from(1u8);
    // The fraction's groups, each opened by a set continuation bit;
    // the stream's one clear closing bit ends the loop. Group bytes
    // stay plain `u8`s until the single width-metered materialization
    // below.
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
    // Strict minimal packing within the final byte: the bits after
    // the close bit are padding and must be zero.
    if src.used < 8 && src.current & (0xFF >> src.used) != 0 {
        return Err(Decode::TrailingBits);
    }
    // The final group carries the expansion's last set bit
    // (normalization: the expansion never ends in zero), so an
    // all-zero final group is pure padding — non-minimal packing —
    // and its trailing zeros locate the fraction's true depth.
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
    // The fraction's depth needs no bound of its own: every expansion
    // bit was read from the stream, so `frac_len` never exceeds the
    // input's own bit count and always fits the u64 exponent — an
    // input long enough to overflow it cannot be allocated.
    let exp = frac_len;
    let num = if frac_len == 0 {
        integral
    } else {
        (integral << exp) | (Base::from_be_bytes(&groups) >> pad)
    };
    debug_assert!(
        exp == 0 || num.bit(0),
        "a nonempty fraction ends in its last set bit, so the numerator is odd"
    );
    Ok(Rank { num, exp })
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

    /// The pushed bits as a magnitude, MSB-first: the final byte's
    /// zero padding is stripped by one shift, and the materialization
    /// rides the width-metered assembly ([`Base::from_be_bytes`]).
    fn into_base(self) -> Base {
        let pad = if self.used == 0 { 0 } else { 8 - self.used };
        Base::from_be_bytes(&self.bytes) >> u32::from(pad)
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
    let mut exp = 0u64;
    for rank in iter {
        let rank = rank.borrow();
        if rank.exp > exp {
            acc.shl(rank.exp - exp);
            exp = rank.exp;
        }
        acc.add_magnitude_shl(&rank.num, exp - rank.exp);
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
