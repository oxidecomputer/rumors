//! Strict decoding of a skyline stream into a stored [`Version`].

use crate::codec::BitsView;
use crate::error::Decode;
use crate::Version;

use super::validate_bits;

/// Strictly decode one skyline stream into a stored [`Version`].
///
/// Acceptance is [`validate_bits`]'s, bit for bit; the stream then becomes the
/// version's storage directly (the stored form *is* the skyline coding), so
/// decoding materializes nothing beyond the copy.
pub(crate) fn decode_bits(bits: BitsView<'_>) -> Result<Version, Decode> {
    validate_bits(bits)?;
    // One spare bit of capacity: the storage gate's padding marker lands
    // without regrowing (and re-copying) an exactly-sized buffer.
    let mut copy = crate::codec::BitsBuf::with_capacity(bits.len() + 1);
    crate::codec::extend_from_view(&mut copy, bits, 0, bits.len());
    Ok(Version::from_bits(copy))
}
