use bitvec::domain::Domain;
use bitvec::prelude::*;

use crate::DecodeError;

/// The packed storage form: a most-significant-bit-first bit stream over bytes.
pub(crate) type Bits = BitVec<u8, Msb0>;

/// A borrowed view of the packed storage form.
pub(crate) type BitsSlice = BitSlice<u8, Msb0>;

/// Borrow bytes as an MSB-first bit stream without first copying them into a
/// [`Bits`].
pub(crate) fn bytes_as_bits(bytes: &[u8]) -> &BitsSlice {
    bytes.view_bits::<Msb0>()
}

/// Pack a canonical bit stream into bytes, zero-padding the final partial byte.
pub(crate) fn pack_to_bytes(bits: &BitsSlice) -> Vec<u8> {
    let byte_len = bits.len().div_ceil(8);
    if byte_len == 0 {
        return Vec::new();
    }

    match bits.domain() {
        Domain::Enclave(elem) if elem.head().into_inner() == 0 => {
            return vec![elem.load_value()];
        }
        Domain::Region {
            head: None,
            body,
            tail,
        } => {
            let mut bytes = Vec::with_capacity(byte_len);
            bytes.extend(body.iter().map(BitStore::load_value));
            if let Some(elem) = tail {
                bytes.push(elem.load_value());
            }
            debug_assert_eq!(bytes.len(), byte_len);
            return bytes;
        }
        _ => {}
    }

    let mut padded: Bits = bits.to_bitvec();
    while !padded.len().is_multiple_of(8) {
        padded.push(false);
    }
    padded.into_vec()
}

/// Require that the bits from `pos` onward are exactly the canonical padding: a
/// run of zeros shorter than a byte. [`pack_to_bytes`] only pads the final
/// partial byte, so a canonical stream has at most 7 trailing zero bits; both a
/// nonzero padding bit AND a whole spurious zero byte (`>= 8` trailing bits,
/// even if all zero) are non-canonical. Bounding the length is what makes
/// `decode` injective on bytes — without it, `decode([.., 0x00])` would accept
/// the same value under infinitely many byte strings, re-encoding to a shorter
/// stream than its own input.
pub(crate) fn require_zero_padding(bits: &BitsSlice, pos: usize) -> Result<(), DecodeError> {
    if bits.len() - pos >= 8 || bits[pos..].any() {
        Err(DecodeError::TrailingBits)
    } else {
        Ok(())
    }
}
