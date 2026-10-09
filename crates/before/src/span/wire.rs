//! [`Span`]'s canonical byte encoding and strict decoder.
//!
//! The composite is the two endpoints' canonical encodings concatenated —
//! each byte-aligned, independently canonical, and self-delimiting, so no
//! length prefix is needed. Decoding validates both component encodings and
//! their order before constructing the span.
//!
//! Decoding the lower endpoint first reveals the byte boundary between them.
//! The upper endpoint is then validated and compared with the lower endpoint
//! in one pass. Accepted endpoints retain slices of the input buffer; equal
//! endpoints share the lower endpoint's slice.

use std::io::{self, Read, Write};

use crate::bits::{BitRead, Bits, BitsReader};
use crate::error::Decode;
use crate::version::io::validate::{self, Admission};
use crate::Version;

use super::Span;

impl<'a> Span<'a> {
    /// Encodes this [`Span`] as canonical bytes.
    ///
    /// Each endpoint is byte-aligned, independently canonical, and
    /// self-delimiting, so the two concatenate without a length prefix. Byte
    /// equality of the resulting encoding is exactly span equality.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_encode.html")))]
    #[cfg_attr(not(doc), doc = "`O(n)` in total input bytes; `O(|self|)`")]
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Span, Clock};
    /// let mut clock = Clock::seed();
    /// let older = clock.tick().clone();
    /// let newer = clock.tick().clone();
    /// let span = Span::new(&older, &newer).unwrap();
    /// // The lower endpoint's bytes precede the upper endpoint's.
    /// assert_eq!(span.encode(), [older.encode(), newer.encode()].concat());
    /// assert_eq!(Span::decode(&span.encode()[..]).unwrap(), span);
    /// ```
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = self.lo.encode();
        bytes.extend_from_slice(self.hi.as_bytes());
        bytes
    }

    /// Encodes this [`Span`]'s canonical bytes to an arbitrary writer.
    ///
    /// # Errors
    ///
    /// Returns any error reported by the writer.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_encode.html")))]
    #[cfg_attr(not(doc), doc = "`O(n)` in total input bytes; `O(|self|)`")]
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Span, Clock};
    /// let mut clock = Clock::seed();
    /// let v = clock.tick().clone();
    /// let span = Span::new(&v, &v).unwrap();
    /// let mut buf = Vec::new();
    /// span.encode_to(&mut buf).unwrap();
    /// assert_eq!(buf, span.encode());
    /// ```
    pub fn encode_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        self.lo.encode_to(writer)?;
        self.hi.encode_to(writer)
    }

    /// Decodes one [`Span`] from a reader.
    ///
    /// A successful decode requires exactly one canonical encoding; trailing
    /// bytes are an error.
    ///
    /// # Errors
    ///
    /// - [`Decode::Truncated`]: either version is incomplete or the upper
    ///   endpoint is missing.
    /// - [`Decode::TrailingBits`]: a component has malformed padding or bytes
    ///   follow the span.
    /// - [`Decode::NotCanonical`]: a component is non-canonical or the
    ///   endpoints are reversed or incomparable.
    /// - [`Decode::Io`]: the reader itself fails.
    ///
    /// Structural encoding errors take precedence over endpoint ordering.
    ///
    /// # Complexity
    ///
    #[cfg_attr(doc, doc = include_str!(concat!(env!("OUT_DIR"), "/fuelscapes/span_decode.html")))]
    #[cfg_attr(
        not(doc),
        doc = "`O(n)` in total input bytes; `O(n)`, with `n` the bytes read, accepted or rejected"
    )]
    ///
    /// # Example
    ///
    /// ```
    /// use before::{Span, error::Decode, Clock};
    /// let mut clock = Clock::seed();
    /// let older = clock.tick().clone();
    /// let newer = clock.tick().clone();
    /// let bytes = Span::new(&older, &newer).unwrap().encode();
    /// let span = Span::decode(&bytes[..]).unwrap();
    /// assert_eq!(span.lo(), &older);
    /// assert_eq!(span.hi(), &newer);
    /// // A reversed pair is the canonical spelling of no span.
    /// let crossed = [newer.encode(), older.encode()].concat();
    /// assert!(matches!(
    ///     Span::decode(&crossed[..]),
    ///     Err(Decode::NotCanonical)
    /// ));
    /// ```
    pub fn decode<R: Read>(mut reader: R) -> Result<Span<'static>, Decode> {
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).map_err(Decode::Io)?;
        Self::decode_bytes(buf.into())
    }

    /// Validates an owned canonical encoding and shares its storage between
    /// the endpoints.
    pub(crate) fn decode_bytes(buf: bytes::Bytes) -> Result<Span<'static>, Decode> {
        // A version encoding is self-delimiting and byte-aligned, so the
        // lower endpoint's encoding ends at the byte where the upper
        // endpoint's begins.
        let (lo, lo_bytes) = Version::decode_prefix(&buf)?;

        // The lower endpoint is now known to be canonical. That lets the
        // admission walk validate the upper endpoint while deciding whether it
        // equals, dominates, or fails to dominate the lower endpoint. A span
        // accepts only the first two relations.
        let hi_bytes = buf.slice(lo_bytes..);
        let mut hi_reader = BitsReader::from_bytes(&hi_bytes);
        let relation = validate::dominating_from(&lo, &mut hi_reader)?;

        // Check the upper endpoint's marker and padding before acting on the
        // relation. Thus malformed or trailing bytes remain structural errors,
        // even when the values seen so far are crossed or equal.
        Bits::validate_padding(&hi_bytes, hi_reader.position())?;

        let hi = match relation {
            Admission::Refuted => return Err(Decode::NotCanonical),
            Admission::Equal => lo.clone(),
            Admission::Dominates => Version::from_canonical(Bits::from_canonical(hi_bytes)),
        };
        Ok(Span::owned(lo, hi))
    }
}
