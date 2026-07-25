//! Strict decoding of a skyline stream into a stored [`Version`].

use crate::codec::BitsSlice;
use crate::error::Decode;
use crate::Version;

use super::validate_bits;

/// Strictly decode one skyline stream into a stored [`Version`].
///
/// Acceptance is [`validate_bits`]'s, bit for bit; the stream then becomes
/// the version's storage directly (the stored form *is* the skyline
/// coding), so decoding materializes nothing beyond the copy.
pub(crate) fn decode_bits(bits: &BitsSlice) -> Result<Version, Decode> {
    validate_bits(bits)?;
    Ok(Version::from_bits(bits.to_bitvec()))
}
