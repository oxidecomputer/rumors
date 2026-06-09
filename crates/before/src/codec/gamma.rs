//! Elias-gamma encoding and decoding of the integers in a
//! [`Version`](crate::Version).
//!
//! The normal form of a [`Version`](crate::Version) guarantees that at least
//! half of the integers in its event tree are zero, and normalization pushes
//! magnitude toward the root, so most stored integers are small even after
//! many events. Elias-gamma encodes a zero in one bit and other integers in
//! bits proportional to the log of their magnitude, so the encoding is close
//! to minimal for this distribution.
//!
//! The trade-off is decode cost on every operation that examines a version.
//! That cost buys a heap-size reduction of one to two orders of magnitude,
//! and [`Batch`](crate::version::Batch) amortizes the decoding across a run
//! of operations.

use num_bigint::BigUint;

use crate::error::Decode;

use super::{Base, Bits, BitsSlice};

/// Append `n` as the Elias gamma code of `m = n + 1`: `floor(log2(m))` zero
/// bits, then `m` in `floor(log2(m)) + 1` bits, most-significant first. Cost is
/// `2*floor(log2(n+1)) + 1` bits; `0` costs a single bit. Canonical and
/// prefix-free, for an arbitrary-width non-negative `n` (there is no value
/// cap).
pub(crate) fn encode_int(out: &mut Bits, n: &Base) {
    // m >= 1, so `m.bits() >= 1` and computing `k = floor(log2(m)) =
    // bit_length(m) - 1` never underflows. `k` is a bit count and fits a
    // `u64` even when `m` itself does not.
    let m = n + 1u32;
    let k = m.bits() - 1;
    for _ in 0..k {
        out.push(false);
    }
    // Emit `m` in `k + 1` bits, most-significant first.
    for i in (0..=k).rev() {
        out.push(m.bit(i));
    }
}

/// Read an Elias-gamma-coded integer at `pos`, returning the value and the new
/// position. Running past the end is `Truncated`. Decodes an arbitrary-width
/// value (no cap): the unary prefix length `k` is bounded by the available
/// bits, which the `Truncated` checks enforce, so a declared code can never
/// exceed the input.
pub(crate) fn decode_int(bits: &BitsSlice, pos: usize) -> Result<(Base, usize), Decode> {
    let mut k = 0usize;
    loop {
        let idx = pos + k;
        if idx >= bits.len() {
            return Err(Decode::Truncated);
        }
        if bits[idx] {
            break; // the leading 1 of m
        }
        k += 1;
    }
    let start = pos + k;
    if start + k + 1 > bits.len() {
        return Err(Decode::Truncated);
    }
    let end = start + k + 1;

    // Common case: read small codes into a machine integer, then widen once.
    if k < u64::BITS as usize {
        let mut m = 0u64;
        for i in 0..=k {
            m <<= 1;
            if bits[start + i] {
                m |= 1;
            }
        }
        return Ok((Base::from(m - 1), end));
    }

    // Wide fallback: read the `k + 1` bits of `m` most-significant first into a `BigUint`.
    let mut m = BigUint::ZERO;
    for i in 0..=k {
        m <<= 1;
        if bits[start + i] {
            m |= BigUint::from(1u32);
        }
    }
    Ok((Base::from(m - 1u32), end))
}

/// Skip an Elias-gamma-coded integer at `pos`, returning the position just past
/// it without materializing the integer. Used by topology-only event scans.
pub(crate) fn skip_int(bits: &BitsSlice, pos: usize) -> Result<usize, Decode> {
    let mut k = 0usize;
    loop {
        let idx = pos + k;
        if idx >= bits.len() {
            return Err(Decode::Truncated);
        }
        if bits[idx] {
            break;
        }
        k += 1;
    }
    let end = pos + (2 * k) + 1;
    if end > bits.len() {
        Err(Decode::Truncated)
    } else {
        Ok(end)
    }
}
