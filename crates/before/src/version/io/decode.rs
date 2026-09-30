//! Strict decoding of a Version stream into stored form.

use crate::bits::BitsWriter;
use crate::error::Decode;
use crate::Version;

use crate::version::io::validate;

/// Strictly decode one Version stream into stored form.
///
/// Acceptance is [`validate::whole`]'s, bit for bit; the stream then becomes the
/// version's storage directly (the stored form is the wire encoding), so
/// decoding materializes nothing beyond the copy.
pub fn writer(bits: BitsWriter) -> Result<Version, Decode> {
    validate::whole(bits.reader())?;
    Ok(crate::version::io::finish(bits))
}
