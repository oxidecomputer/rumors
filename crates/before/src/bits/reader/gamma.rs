//! Elias-gamma decoding for bounded and streaming readers.

use dsi_bitstream::codes::gamma_tables;
use dsi_bitstream::traits::BitRead as DsiBitRead;
use num_bigint::BigUint;

use super::{BitRead, BitsReader};
use crate::error::Decode;

#[cfg(any(test, feature = "borsh"))]
mod window;

/// Decode through the bitwise reference path.
pub(super) fn decode_slow<R>(reader: &mut R) -> Result<BigUint, Decode>
where
    R: BitRead,
    Decode: From<R::Error>,
{
    let mut zeros = 0u64;
    while !reader.read_bit()? {
        zeros += 1;
    }

    if zeros < u64::from(u64::BITS) {
        let mut mantissa = 1u64;
        for _ in 0..zeros {
            mantissa <<= 1;
            if reader.read_bit()? {
                mantissa |= 1;
            }
        }
        return Ok(BigUint::from(mantissa - 1));
    }

    let mut mantissa = BigUint::ZERO;
    mantissa.set_bit(zeros, true);
    for bit in (0..zeros).rev() {
        if reader.read_bit()? {
            mantissa.set_bit(bit, true);
        }
    }
    Ok(mantissa - 1u32)
}

/// The length of a gamma code with `zeros` leading zeros, `2 · zeros + 1`,
/// if it fits within `remaining` bits.
///
/// A run of 2^63 zeros or more fits in a stream of fewer than 2^64 bits, but
/// its code's length does not fit a `u64`, so the length is computed with
/// checked arithmetic and compared against the remainder rather than summed
/// with a position.
pub(super) fn code_len_within(zeros: u64, remaining: u64) -> Option<u64> {
    zeros
        .checked_mul(2)
        .and_then(|prefix_and_mantissa| prefix_and_mantissa.checked_add(1))
        .filter(|&code_len| code_len <= remaining)
}

/// Decode with table, word, and arbitrary-width paths over a bounded reader.
pub(super) fn decode(reader: &mut BitsReader<'_>) -> Result<BigUint, Decode> {
    if reader.len - reader.position >= gamma_tables::READ_BITS as u64 {
        if let Some((value, used)) = gamma_tables::read_table_be(&mut reader.reader) {
            let used = used as u64;
            reader.position += used;
            return Ok(BigUint::from(value));
        }
    }

    let zeros = reader.unary_raw().map_err(|_| Decode::Truncated)?;
    let Some(code_len) = code_len_within(zeros, reader.len - reader.position) else {
        reader.truncated();
        return Err(Decode::Truncated);
    };

    if zeros < u64::from(u64::BITS) {
        let rest = reader
            .reader
            .read_bits(zeros as usize)
            .expect("the mantissa was proven to fit the live length");
        let mantissa = (1u64 << zeros) | rest;
        reader.position += code_len;
        return Ok(BigUint::from(mantissa - 1));
    }

    // The value's one allocation, at its full width, comes from setting the
    // implicit leading one. The mantissa's `zeros` bits then follow the prefix
    // most significant first, and we set only the ones among them, each
    // 64-bit chunk visiting its set bits alone. Assembling whole limbs instead
    // would need a second buffer as wide as the value, since `BigUint` builds
    // only from 32-bit limbs or bytes and copies them into its own digits.
    let mut mantissa = BigUint::ZERO;
    mantissa.set_bit(zeros, true);
    let mut remaining = zeros;
    while remaining > 0 {
        let chunk_bits = remaining.min(u64::from(u64::BITS));
        let mut chunk = reader
            .reader
            .read_bits(chunk_bits as usize)
            .expect("the mantissa was proven to fit the live length");
        remaining -= chunk_bits;
        while chunk != 0 {
            mantissa.set_bit(remaining + u64::from(chunk.trailing_zeros()), true);
            chunk &= chunk - 1;
        }
    }
    reader.position += code_len;
    Ok(mantissa - 1u32)
}

/// Try the one-word decoder over already-buffered bytes.
#[cfg(any(test, feature = "borsh"))]
pub(super) fn from_window(bytes: &[u8], len: u64, position: u64) -> Option<(u64, u64)> {
    window::decode(bytes, len, position)
}
