//! The sign-magnitude currency: the zigzag maps, the signed folds and sums, and
//! the gamma codes of signed deltas — one home for the vocabulary every skyline
//! walk exchanges heights in.
//!
//! # The zigzag bijection
//!
//! Payload deltas are signed, but every stored integer is a gamma-coded
//! natural, so a delta passes through the zigzag map first: `k >= 0 -> 2k`, `k
//! < 0 -> 2|k| − 1`. The map is a bijection between the integers and the
//! naturals with no negative-zero form — an odd code decodes to magnitude at
//! least 1 — so every delta has exactly one spelling, and a non-canonical
//! spelling cannot be written at all. The root module doc's canonical-form
//! argument leans on exactly this; the tests pin the bijection exhaustively at
//! small scope.
//!
//! # The fused gamma fast path
//!
//! Gamma codes a natural `n` as the binary spelling of `n + 1` under its own
//! leading zeros. For a signed delta, `zigzag + 1` is one machine expression:
//! `2·magnitude + !negative` — a negative delta's `2m − 1` and a nonnegative
//! one's `2m` both absorb the `+ 1` into the low bit. That mantissa under its
//! own leading zeros is therefore the *whole* code: a word-scale delta zigzags
//! and codes in machine arithmetic with no intermediate value built
//! ([`gamma_code_signed`] and its [`Int`] twin), and only wider deltas take the
//! arbitrary-precision pair.
//!
//! # Signed conventions
//!
//! [`Signed`] is the walks' exchange pair: a sign tag beside a magnitude, the
//! shape the scans return extrema in and the watermark webs price emissions
//! against. No producer emits a negative-tagged zero — every construction
//! normalizes: the zigzag maps have no spelling for one, the accumulator
//! read-out ties a zero magnitude to the `Equal` sign
//! ([`Accumulator::sign_magnitude`]'s contract, which
//! [`Signed::from_sign_magnitude`] inherits), and [`signed_sum`] returns the
//! positive zero on exact cancellation. The comparisons tolerate one anyway —
//! deliberate slack, so no future fold is obligated to normalize: it means
//! zero, and [`signed_le`] discharges the tag before comparing.

use core::cmp::Ordering;

use suanpan::{Accumulator, UBig};

use crate::codec::{self, Base, Code, Int};

/// A signed relative quantity — a sign tag beside a magnitude — the exchange
/// shape between the zigzag coding, the scans, and the accumulator folds.
#[derive(Clone)]
pub(super) struct Signed {
    /// Whether the value is negative. A zero magnitude may carry either tag
    /// and means zero under both (the module doc's conventions).
    pub(super) negative: bool,
    /// The absolute value.
    pub(super) magnitude: Int,
}

impl Signed {
    /// The [`Signed`] reading of an accumulator's sign-and-magnitude
    /// decomposition ([`Accumulator::sign_magnitude`]).
    pub(super) fn from_sign_magnitude(sign: Ordering, magnitude: UBig) -> Self {
        Signed {
            negative: sign == Ordering::Less,
            magnitude: Int::from_ubig(magnitude),
        }
    }

    /// Whether the value is zero, under either sign tag.
    pub(super) fn is_zero(&self) -> bool {
        self.magnitude.is_zero()
    }

    /// The sum of two [`Signed`] values: [`signed_sum_int`] lifted to the
    /// struct.
    ///
    /// Never yields a negative zero, as [`signed_sum`].
    pub(super) fn sum(self, other: &Signed) -> Signed {
        let (negative, magnitude) = signed_sum_int(
            self.negative,
            self.magnitude,
            other.negative,
            &other.magnitude,
        );
        Signed {
            negative,
            magnitude,
        }
    }
}

/// Map the signed difference `cur − prev` to its zigzag magnitude:
/// `k >= 0 -> 2k`, `k < 0 -> 2|k| − 1`.
pub(super) fn zigzag(prev: &Base, cur: &Base) -> Base {
    if cur >= prev {
        (cur.clone() - prev) << 1u32
    } else {
        ((prev.clone() - cur) << 1u32) - &Base::from(1u8)
    }
}

/// Map a delta given as sign and absolute value to its zigzag magnitude: `+m ->
/// 2m`, `−m -> 2m − 1`.
///
/// [`zigzag`] for a difference already in sign-magnitude form — the shape the
/// emission sweep computes deltas in. A negative delta must have a nonzero
/// magnitude (there is no negative zero to map).
pub(super) fn zigzag_signed(negative: bool, magnitude: Base) -> Base {
    debug_assert!(
        !negative || magnitude != Base::ZERO,
        "a negative delta has a nonzero magnitude"
    );
    if negative {
        (magnitude << 1u32) - &Base::from(1u8)
    } else {
        magnitude << 1u32
    }
}

/// Split a zigzag magnitude into its delta's sign and absolute value, staying
/// in machine words for word-scale codes: even `m -> +m/2`, odd `m -> −(m +
/// 1)/2`.
///
/// The [`Int`] form of [`unzigzag_base`], the shape the sweeps and the fill
/// walk consume; total (odd `u64::MAX` maps within the word range).
pub(super) fn unzigzag(code: Int) -> (bool, Int) {
    match code {
        // Odd `c`: `(c + 1) / 2 = c / 2 + 1`, in range at any `c`.
        Int::Small(c) if c & 1 == 1 => (true, Int::Small(c / 2 + 1)),
        Int::Small(c) => (false, Int::Small(c / 2)),
        Int::Wide(base) => {
            let (negative, magnitude) = unzigzag_base(base);
            (negative, Int::from_base(magnitude))
        }
    }
}

/// Split a zigzag magnitude into its delta's sign and absolute value: even `m
/// -> +m/2`, odd `m -> −(m + 1)/2`.
///
/// The inverse of [`zigzag`]: total, and never yields a negative zero (an odd
/// code's magnitude is at least 1).
pub(super) fn unzigzag_base(code: Base) -> (bool, Base) {
    if code.bit(0) {
        (true, (code + 1u32) >> 1u32)
    } else {
        (false, code >> 1u32)
    }
}

/// Fold a signed magnitude into an accumulator: subtracted when negative, added
/// otherwise.
///
/// The one home of the sign-magnitude fold every height walk applies — the
/// exchange move between this module's sign-magnitude currency (the zigzag
/// maps above) and the cliff-immune [`Accumulator`].
pub(super) fn fold_signed(acc: &mut Accumulator, negative: bool, magnitude: &Base) {
    if negative {
        acc.sub_magnitude(magnitude);
    } else {
        acc.add_magnitude(magnitude);
    }
}

/// Fold a signed [`Int`] delta into an accumulator: subtracted when negative,
/// added otherwise — the [`Int`] twin of [`fold_signed`], dispatching
/// word-scale values straight to the word entry points.
pub(super) fn fold_signed_int(acc: &mut Accumulator, negative: bool, magnitude: &Int) {
    match (negative, magnitude) {
        (false, Int::Small(n)) => acc.add_u64(*n),
        (true, Int::Small(n)) => acc.sub_u64(*n),
        (false, Int::Wide(base)) => acc.add_magnitude(base),
        (true, Int::Wide(base)) => acc.sub_magnitude(base),
    }
}

/// The magnitude bound of the fused gamma fast path: `mag < 2^31` keeps the
/// whole signed code inside the small-code word (`mag < 2^31` ⇒ mantissa
/// `2·mag + 1 < 2^32` ⇒ `k <= 31` ⇒ code length `2k + 1 <= 63` bits).
const GAMMA_SMALL_MAG_BOUND: u64 = 1 << 31;

/// A value's gamma code as a payload-code value.
pub(super) fn gamma_code(value: &Base) -> Code {
    codec::code_int(value)
}

/// A value's gamma code as a payload-code value, from either width.
pub(super) fn gamma_code_int(value: &Int) -> Code {
    match value {
        Int::Small(n) => codec::code_int_small(*n),
        Int::Wide(base) => codec::code_int(base),
    }
}

/// The gamma code of a signed delta's zigzag, as a payload-code value.
///
/// [`zigzag_signed`] fused with [`gamma_code`]: a word-scale magnitude zigzags
/// and codes in machine arithmetic — no intermediate value is built — and a
/// wider one takes the arbitrary-precision pair. A negative delta must have a
/// nonzero magnitude, as in [`zigzag_signed`].
pub(super) fn gamma_code_signed(negative: bool, magnitude: &Base) -> Code {
    debug_assert!(
        !negative || *magnitude != Base::ZERO,
        "a negative delta has a nonzero magnitude"
    );
    if let Some(mag) = magnitude.to_u64() {
        if mag < GAMMA_SMALL_MAG_BOUND {
            // negative: gamma of `zigzag + 1 = (2m − 1) + 1 = 2m`;
            // positive: gamma of `2m + 1`. Either way the whole code is
            // that mantissa under its own leading zeros.
            let m = 2 * mag + u64::from(!negative);
            let k = (u64::BITS - 1 - m.leading_zeros()) as usize;
            return Code::Small {
                bits: m,
                len: (2 * k + 1) as u8,
            };
        }
    }
    codec::code_int(&zigzag_signed(negative, magnitude.clone()))
}

/// The gamma code of a signed delta's zigzag, from either width: the [`Int`]
/// twin of [`gamma_code_signed`].
pub(super) fn gamma_code_signed_int(negative: bool, magnitude: &Int) -> Code {
    match magnitude {
        Int::Small(mag) if *mag < GAMMA_SMALL_MAG_BOUND => {
            debug_assert!(
                !negative || *mag != 0,
                "a negative delta has a nonzero magnitude"
            );
            let m = 2 * mag + u64::from(!negative);
            let k = (u64::BITS - 1 - m.leading_zeros()) as usize;
            Code::Small {
                bits: m,
                len: (2 * k + 1) as u8,
            }
        }
        Int::Small(mag) => gamma_code_signed(negative, &Base::from(*mag)),
        Int::Wide(base) => gamma_code_signed(negative, base),
    }
}

/// The sign and magnitude of a sum of two signed magnitudes.
///
/// Never yields a negative zero: a cancelling pair returns the positive zero,
/// so the zigzag coding downstream stays canonical.
pub(super) fn signed_sum(x_neg: bool, x: Base, y_neg: bool, y: &Base) -> (bool, Base) {
    if x_neg == y_neg {
        return (x_neg, &x + y);
    }
    match x.cmp(y) {
        Ordering::Greater => (x_neg, x - y),
        Ordering::Less => (y_neg, y.clone() - &x),
        Ordering::Equal => (false, Base::ZERO),
    }
}

/// The sign and magnitude of a sum of two signed [`Int`] magnitudes:
/// [`signed_sum`]'s value form, word-scale pairs summed in machine arithmetic.
///
/// Never yields a negative zero, as [`signed_sum`].
pub(super) fn signed_sum_int(x_neg: bool, x: Int, y_neg: bool, y: &Int) -> (bool, Int) {
    if let (Int::Small(a), Int::Small(b)) = (&x, y) {
        let a = if x_neg {
            -i128::from(*a)
        } else {
            i128::from(*a)
        };
        let b = if y_neg {
            -i128::from(*b)
        } else {
            i128::from(*b)
        };
        let sum = a + b;
        let magnitude = match u64::try_from(sum.unsigned_abs()) {
            Ok(word) => Int::Small(word),
            Err(_) => Int::Wide(Base::from(sum.unsigned_abs())),
        };
        return (sum < 0, magnitude);
    }
    let y_widened;
    let y = match y {
        Int::Wide(base) => base,
        Int::Small(n) => {
            y_widened = Base::from(*n);
            &y_widened
        }
    };
    let (negative, magnitude) = signed_sum(x_neg, x.into_base(), y_neg, y);
    (negative, Int::from_base(magnitude))
}

/// Whether `x <= y` over signed relative quantities. Zero compares equal under
/// either sign, so a negative-tagged zero that a fold produced is ordered
/// correctly.
pub(super) fn signed_le(x: &Signed, y: &Signed) -> bool {
    let x_neg = x.negative && !x.magnitude.is_zero();
    let y_neg = y.negative && !y.magnitude.is_zero();
    match (x_neg, y_neg) {
        (true, false) => true,
        (false, true) => false,
        (false, false) => x.magnitude.cmp_magnitude(&y.magnitude) != Ordering::Greater,
        (true, true) => x.magnitude.cmp_magnitude(&y.magnitude) != Ordering::Less,
    }
}

/// The larger of two signed relative quantities.
pub(super) fn signed_max(x: &Signed, y: &Signed) -> Signed {
    if signed_le(x, y) {
        y.clone()
    } else {
        x.clone()
    }
}
