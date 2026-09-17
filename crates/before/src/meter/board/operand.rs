//! Deterministic operand-content walks: the quantities the liveness floors and
//! denominators are stated in, derived from encoded operands entirely outside
//! any measurement.

use num_bigint::BigUint;

use crate::codec;
use crate::Version;

use super::ceilings::MACHINE_WORD_MAGNITUDE_BITS;

/// The *nonzero* stored delta codes of a version's encoded stream: every leaf
/// payload code after the first (the absolute root height) whose delta is
/// nonzero.
///
/// Single-operand and pair walks add each of these to a running accumulator, so
/// the count is the touch column's
/// deterministic-liveness floor (the pair walks take the max over their two
/// operands: a shared boundary lands both codes in one fold). A zero delta is
/// decoded but folds nothing — an accumulator add of zero is a no-op — so a
/// floor that counted every delta would demand touch work no conforming fold
/// does (a plateau-heavy stream legitimately reads near zero). Iterative over
/// the encoded form, outside any measurement.
pub(super) fn stored_nonzero_deltas(v: &Version) -> u64 {
    let bits = v.as_bits();
    let mut pos = 0u64;
    let mut pending = 1usize;
    let mut first = true;
    let mut nonzero = 0u64;
    while pending > 0 {
        pending -= 1;
        let internal = !bits.bit(pos); // skyline flag: 0 internal, 1 leaf
        pos += 1;
        if internal {
            pending += 2;
            continue;
        }
        let (payload, next) =
            codec::gamma::decode(bits, pos).expect("a stored stream is canonical");
        pos = next;
        if !first && payload != BigUint::ZERO {
            nonzero += 1;
        }
        first = false;
    }
    nonzero
}

/// The 64-bit words occupied by a version's wide stored payload codes.
///
/// This supplies the touch floor for validation: wide codes enter the
/// accumulator word by word, while narrower codes stay in its register.
/// Iterative over the encoded form, outside measurement.
pub(super) fn wide_code_words(v: &Version) -> u64 {
    let bits = v.as_bits();
    let mut pos = 0u64;
    let mut pending = 1usize;
    let mut words = 0u64;
    while pending > 0 {
        pending -= 1;
        let internal = !bits.bit(pos); // skyline flag: 0 internal, 1 leaf
        pos += 1;
        if internal {
            pending += 2;
            continue;
        }
        let (code, next) = codec::gamma::decode(bits, pos).expect("a stored stream is canonical");
        pos = next;
        let width = code.bits();
        if width > MACHINE_WORD_MAGNITUDE_BITS {
            words += width.div_ceil(64);
        }
    }
    words
}

/// A version's value content in bytes: the summed bit widths of its absolute
/// leaf heights (one bit minimum per leaf), rounded to bytes.
///
/// This is the content that delta coding lets ride behind asymptotically fewer
/// wire bits, and the scaling denominator of the flat-denominator shape's
/// exponent fits: the boundary comb at fixed tooth magnitude doubles its value
/// content (and every operation's honest per-tooth work) per level while its
/// encoded bytes grow only by the unit delta codes over a fixed wide intercept.
/// Iterative over the encoded form, outside any measurement.
pub(super) fn value_content_bytes(v: &Version) -> usize {
    let bits = v.as_bits();
    let mut pos = 0u64;
    let mut pending = 1usize;
    let mut last: Option<BigUint> = None;
    let mut content = 0u64;
    while pending > 0 {
        pending -= 1;
        let internal = !bits.bit(pos); // skyline flag: 0 internal, 1 leaf
        pos += 1;
        if internal {
            pending += 2;
            continue;
        }
        let (code, next) = codec::gamma::decode(bits, pos).expect("a stored stream is canonical");
        pos = next;
        let value = match last {
            None => code,
            Some(prev) => {
                let odd = code.bit(0);
                let magnitude = if odd {
                    (code + 1u32) >> 1u32
                } else {
                    code >> 1u32
                };
                if odd {
                    prev - &magnitude
                } else {
                    prev + &magnitude
                }
            }
        };
        content += value.bits().max(1);
        last = Some(value);
    }
    (content.div_ceil(8)) as usize
}

/// The encoded byte size of a version produced by a measured body.
pub(super) fn version_output_bytes(v: &Version) -> usize {
    // The measured value's stored buffer is allocated on this host, so its
    // byte count fits `usize`.
    usize::try_from(v.encoded_bits().div_ceil(8)).expect("an allocated buffer's byte count")
}
