//! Elias-gamma encoding and decoding of the integers in a
//! [`Version`](crate::Version).
//!
//! A stored [`Version`](crate::Version) codes its integers as the skyline
//! payload stream (`version::skyline` documents the coding): the first leaf's
//! absolute height, then one zigzag delta per later leaf. A delta's width is
//! the height step between neighboring plateaus, which normalization keeps
//! small in organic histories, so most stored integers are small even after
//! many events. Elias-gamma encodes a zero in one bit and other integers in
//! bits proportional to the log of their magnitude, so the encoding is close to
//! minimal for this distribution.
//!
//! Both directions keep the coding's cost word-scale: the stream is
//! byte-backed, so a whole code is decoded from one 64-bit window
//! ([`decode_window`]) and emitted with one store, with per-bit loops as
//! the fallback — and, on decode, the sole arbiter of every reject.

use num_bigint::{BigInt, BigUint, Sign};

use crate::error::Decode;

use super::{BitCursor, BitsBuf, BitsView, SliceCursor};

/// A destination for a gamma code.
pub(crate) trait Sink {
    /// Append one bit.
    fn push_bit(&mut self, bit: bool);

    /// Append the low `len <= 64` bits of `value`, most-significant first.
    fn push_bits(&mut self, value: u64, len: u32);

    /// Append `len` zero bits.
    fn push_zeros(&mut self, mut len: u64) {
        while len >= u64::from(u64::BITS) {
            self.push_bits(0, u64::BITS);
            len -= u64::from(u64::BITS);
        }
        self.push_bits(0, len as u32);
    }

    /// Append a magnitude's binary digits, most-significant first.
    fn push_magnitude(&mut self, value: &BigUint) {
        let bits = value.bits();
        if bits == 0 {
            return;
        }
        let mut words = value.iter_u64_digits().rev();
        let top = words.next().expect("a nonzero magnitude has one word");
        let top_len = ((bits - 1) % u64::from(u64::BITS) + 1) as u32;
        self.push_bits(top, top_len);
        for word in words {
            self.push_bits(word, u64::BITS);
        }
    }
}

/// Writes gamma codes into a mutable bit buffer.
impl Sink for BitsBuf {
    fn push_bit(&mut self, bit: bool) {
        self.push(bit);
    }

    fn push_bits(&mut self, value: u64, len: u32) {
        BitsBuf::push_bits(self, value, len);
    }
}

/// Append `value`'s Elias gamma code.
///
/// The code for `n` is `floor(log2(n + 1))` zeros followed by the binary
/// representation of `n + 1`. It is canonical and prefix-free; zero takes
/// one bit, and arbitrary-width values have no upper bound.
pub(crate) fn encode(value: &BigUint, out: &mut (impl Sink + ?Sized)) {
    let mantissa = value + 1u32;
    out.push_zeros(mantissa.bits() - 1);
    out.push_magnitude(&mantissa);
}

/// Map `current - previous` through the signed zigzag encoding.
#[cfg(any(test, feature = "meter"))]
pub(crate) fn zigzag_difference(previous: &BigUint, current: &BigUint) -> BigUint {
    if current >= previous {
        (current.clone() - previous) << 1u32
    } else {
        ((previous.clone() - current) << 1u32) - 1u32
    }
}

/// Map a signed integer through the zigzag encoding.
#[cfg(test)]
pub(crate) fn zigzag(value: BigInt) -> BigUint {
    let (sign, magnitude) = value.into_parts();
    debug_assert!(sign != Sign::Minus || magnitude != BigUint::ZERO);
    match sign {
        Sign::Minus => (magnitude << 1u32) - 1u32,
        Sign::NoSign | Sign::Plus => magnitude << 1u32,
    }
}

/// Decode a zigzag magnitude as a signed integer.
pub(crate) fn decode_signed(code: BigUint) -> BigInt {
    if code.bit(0) {
        BigInt::from_biguint(Sign::Minus, (code + 1u32) >> 1u32)
    } else {
        BigInt::from_biguint(Sign::Plus, code >> 1u32)
    }
}

/// Write a signed integer's zigzag gamma code without materializing zigzag.
pub(crate) fn encode_signed(value: &BigInt, out: &mut (impl Sink + ?Sized)) {
    encode_zigzag(value.sign(), value.magnitude(), out);
}

/// Write a nonnegative magnitude's zigzag gamma code without allocating a
/// signed integer.
pub(crate) fn encode_positive(magnitude: &BigUint, out: &mut (impl Sink + ?Sized)) {
    encode_zigzag(Sign::Plus, magnitude, out);
}

/// Write a negative magnitude's zigzag gamma code without allocating a
/// signed integer.
pub(crate) fn encode_negative(magnitude: &BigUint, out: &mut (impl Sink + ?Sized)) {
    debug_assert_ne!(*magnitude, BigUint::ZERO, "negative zero has no encoding");
    encode_zigzag(Sign::Minus, magnitude, out);
}

/// Write a zigzag gamma code from a sign and magnitude.
fn encode_zigzag(sign: Sign, magnitude: &BigUint, out: &mut (impl Sink + ?Sized)) {
    debug_assert!(sign != Sign::Minus || *magnitude != BigUint::ZERO);
    out.push_zeros(magnitude.bits());
    out.push_magnitude(magnitude);
    out.push_bit(sign != Sign::Minus);
}

/// Read an Elias-gamma-coded integer at `pos`, returning the value and the new
/// position.
///
/// Running past the end is `Truncated`. Decodes an arbitrary-width value (no
/// cap): the unary prefix length `k` is bounded by the available bits, which
/// the `Truncated` checks enforce, so a declared code can never exceed the
/// input.
///
/// Reads word-wise when [`decode_window`] can prove the whole code from one
/// window; every other input — including every reject — is decided by the
/// per-bit loop ([`decode_from`]), so the two paths accept and reject
/// identically by construction (the routing lives in
/// [`SliceCursor::read_int`](BitCursor::read_int)).
pub(crate) fn decode(bits: BitsView<'_>, pos: u64) -> Result<(BigUint, u64), Decode> {
    let mut cursor = SliceCursor::new(bits, pos);
    let base = cursor.read_int()?;
    Ok((base, cursor.position()))
}

/// The number of bits a [`decode_window`] window holds.
const WINDOW_BITS: u64 = u64::BITS as u64;

/// One-window fast path of the gamma decoder: the value and end position of the
/// code at `pos`, when a single 64-bit window proves the whole code.
///
/// Loads a [`WINDOW_BITS`]-bit big-endian window at `pos`, takes one
/// `leading_zeros` for the whole unary prefix `k`, and shifts the mantissa out
/// of the same window — `O(1)` words per integer instead of ~10 ops per bit.
///
/// Returns `None` — decode nothing, let the caller run the per-bit loop from
/// `pos` instead — whenever the window cannot *prove* a complete code:
///
/// - `pos` lies past the end of the stream (the bit loop reports `Truncated`);
/// - the `2k+1`-bit code overruns the window's proven bits, either because the
///   stream ends first (the bit loop reports `Truncated`) or because the code
///   is wider than the window (the bit loop decodes it: its machine-word path
///   reads every `k ≤ 63` mantissa — the `k + 1`-bit mantissa is the value
///   itself and fits `u64` — and only wider codes take the big-integer path).
///
/// The conditions are conservative, never guesses: `Some` is returned only when
/// every bit of the code lies within the window *and* within the stream, so the
/// fallback loop remains the sole arbiter of every reject. Bits between the end
/// of the stream and the end of the window read as zero (the tail byte's dead
/// bits are masked, missing bytes are zero-filled), which only ever *lengthens*
/// the apparent prefix — pushing `2k+1` past the proven bits and into the
/// fallback — never shortens it into a bogus accept.
///
/// Positions are the view's own `u64`: the wire-side reader windows its
/// buffered bytes ([`BitsView::whole`]) at the same width its own position
/// runs at.
pub(crate) fn decode_window(bits: BitsView<'_>, pos: u64) -> Option<(u64, u64)> {
    let (body, tail) = bits.body_tail();
    window_int(body, tail, bits.len(), pos)
}

/// The one-window decoder's body over raw parts: `len` live bits across
/// `body` plus the masked partial `tail` byte, positions in `u64` (a byte
/// input buffer can hold more bit positions than a 32-bit `usize`).
fn window_int(body: &[u8], tail: Option<u8>, len: u64, pos: u64) -> Option<(u64, u64)> {
    // Bits of real stream between `pos` and the window's end.
    let proven = len.checked_sub(pos)?.min(WINDOW_BITS);
    let window = load_window(body, tail, pos);
    let k = u64::from(window.leading_zeros());
    let code_len = 2 * k + 1;
    if code_len > proven {
        return None;
    }
    // `code_len <= 64` bounds `k <= 31`, so the shift is in range and the
    // `k+1`-bit mantissa `m` (its leading 1 included) fits comfortably.
    let m = window >> (WINDOW_BITS - code_len);
    Some((m - 1, pos + code_len))
}

#[cfg(test)]
mod tests;

/// Load a 64-bit big-endian window starting at bit `pos` of the stream held
/// as `body` plus the masked partial `tail` byte: bit `pos` in the most
/// significant position, zero past the stream's end.
fn load_window(body: &[u8], tail: Option<u8>, pos: u64) -> u64 {
    // In-range byte indices fit `usize` (they index an allocated buffer);
    // the clamps below keep every computed index in range.
    let byte = usize::try_from(pos / 8).unwrap_or(usize::MAX);
    let shift = (pos % 8) as usize;
    // Gather the (up to) 9 bytes covering bits `pos..pos + 64`: 8 whole bytes
    // plus the partial ninth that a mid-byte `pos` shifts in. Bytes past the
    // stream stay zero — the tail byte arrives with its dead bits masked, and
    // the buffer zero-fills past the last byte — so phantom bits are always
    // zero.
    let mut buf = [0u8; 9];
    let start = byte.min(body.len());
    let end = byte.saturating_add(buf.len()).min(body.len());
    buf[..end - start].copy_from_slice(&body[start..end]);
    if let Some(t) = tail {
        // The callers bound `pos` by the live length, which puts `byte` at or
        // before the tail byte, so the index never underflows.
        let tail_at = body.len();
        if tail_at < byte.saturating_add(buf.len()) {
            buf[tail_at - byte] = t;
        }
    }
    let word = u64::from_be_bytes(buf[..8].try_into().expect("buf holds 8 whole bytes"));
    if shift == 0 {
        word
    } else {
        (word << shift) | (u64::from(buf[8]) >> (8 - shift))
    }
}

/// Read one Elias-gamma-coded integer from a sequential bit cursor.
pub(crate) fn decode_from<C: BitCursor>(cursor: &mut C) -> Result<BigUint, Decode>
where
    Decode: From<C::Error>,
{
    // `u64`, as every bit count here: each counted zero occupies real input
    // (a buffer bit or a byte the reader yielded), so the count is bounded
    // by memory, far below any `u64` wrap.
    let mut k = 0u64;
    while !cursor.read_bit()? {
        k += 1;
    }

    // Common case: read small codes into a machine integer, then widen once.
    if k < u64::from(u64::BITS) {
        let mut m = 1u64;
        for _ in 0..k {
            m <<= 1;
            if cursor.read_bit()? {
                m |= 1;
            }
        }
        return Ok(BigUint::from(m - 1));
    }

    // Wide fallback: the leading 1 has already been consumed, and it is the
    // mantissa's top bit, at position `k`; the next `k` stream bits are the
    // mantissa's remaining bits, most-significant first. Setting the top bit
    // first sizes the value's storage once, and each later set writes one word
    // in place, so the total arithmetic is linear in the code's bit width and
    // the only allocation is the value itself. A truncated stream still fails
    // at the same `read_bit` position it would reading into an accumulator, so
    // the accept/reject boundary is unchanged.
    let mut m = BigUint::ZERO;
    m.set_bit(k, true);
    for i in (0..k).rev() {
        if cursor.read_bit()? {
            m.set_bit(i, true);
        }
    }
    // Sizing `m` and subtracting one each take one pass over its words.
    Ok(m - 1u32)
}
