//! Deterministic operand-content walks: the quantities the liveness floors and
//! denominators are stated in, derived from encoded operands entirely outside
//! any measurement.

use crate::codec::{self, Base};
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
        let (payload, next) = codec::decode_int(bits, pos).expect("a stored stream is canonical");
        pos = next;
        if !first && payload != Base::ZERO {
            nonzero += 1;
        }
        first = false;
    }
    nonzero
}

/// The mandatory limb count of a version's stored stream: one limb per 64 bits
/// of every payload code wider than [`MACHINE_WORD_MAGNITUDE_BITS`].
///
/// A walk over the stream must decode each stored code to fold it, and decoding
/// a wide code cannot touch fewer limbs than the code has; narrower codes may
/// legitimately live in machine words and count zero. This counts the stream's
/// delta codes, not the decoded tree's absolute values, so it is the floor for
/// operations that read the stored form directly. Iterative over the encoded
/// form, outside measurement.
pub(super) fn mandatory_limbs_stream(v: &Version) -> u64 {
    let bits = v.as_bits();
    let mut pos = 0u64;
    let mut pending = 1usize;
    let mut limbs = 0u64;
    while pending > 0 {
        pending -= 1;
        let internal = !bits.bit(pos); // skyline flag: 0 internal, 1 leaf
        pos += 1;
        if internal {
            pending += 2;
            continue;
        }
        let (code, next) = codec::decode_int(bits, pos).expect("a stored stream is canonical");
        pos = next;
        let width = code.bits();
        if width > MACHINE_WORD_MAGNITUDE_BITS {
            limbs += width.div_ceil(64);
        }
    }
    limbs
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
    let mut last: Option<Base> = None;
    let mut content = 0u64;
    while pending > 0 {
        pending -= 1;
        let internal = !bits.bit(pos); // skyline flag: 0 internal, 1 leaf
        pos += 1;
        if internal {
            pending += 2;
            continue;
        }
        let (code, next) = codec::decode_int(bits, pos).expect("a stored stream is canonical");
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
