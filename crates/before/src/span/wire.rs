//! The wire form: [`Span`]'s canonical byte encoding and its strict decode.
//!
//! The composite is the two endpoints' canonical encodings concatenated —
//! each byte-aligned, independently canonical, and self-delimiting, so no
//! length prefix exists — and the decode proves the pair ordered in the
//! same pass that parses it. The public wire-form contract lives on
//! [`Span`]; this module is its implementation.

use std::borrow::Cow;
use std::io::{self, Read, Write};

use crate::codec;
use crate::codec::BitCursor;
use crate::error::Decode;
use crate::version::skyline;
use crate::Version;

use super::Span;

impl<'a> Span<'a> {
    /// Encodes this [`Span`] as canonical bytes.
    ///
    /// Each endpoint is byte-aligned, independently canonical, and
    /// self-delimiting, so the two concatenate with no length prefix ([`Span`]'s
    /// docs carry the wire form). Byte equality on these composites is exactly
    /// span equality.
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
    /// // The framing: the meet's bytes, then the join's.
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
    /// Whatever the writer itself reports; the encoding side is infallible.
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
    /// - [`Decode::Truncated`]: the bytes end before the composite does —
    ///   inside either version's tree, ahead of a component's final
    ///   padding byte, or with the second component missing entirely.
    /// - [`Decode::TrailingBits`]: live bits past a component's
    ///   complete tree, or nonzero padding.
    /// - [`Decode::NotCanonical`]: a non-canonical component, or a
    ///   pair that no [`Span`] encodes — crossed or concurrent — the
    ///   canonical spelling of no value.
    /// - [`Decode::Io`]: the reader itself fails.
    ///
    /// On an input defective several ways at once, the components'
    /// structural genres win.
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
        let (lo_bytes, admission) = {
            let lo_end = skyline::validate_prefix(codec::BitsView::whole(&buf))?;
            let lo_bytes = (lo_end + 1).div_ceil(8);
            if lo_bytes > buf.len() as u64 {
                return Err(Decode::Truncated);
            }
            let lo_bytes =
                usize::try_from(lo_bytes).expect("the meet's prefix ends within the read buffer");
            codec::require_marker_padding(&buf[..lo_bytes], lo_end)?;
            let lo = codec::BitsView::new(&buf[..lo_bytes], lo_end);
            let tail = &buf[lo_bytes..];
            let mut cursor = codec::DsiCursor::new(codec::BitsView::whole(tail));
            let admission = skyline::validate_dominating_from(lo, &mut cursor)?;
            let hi_end = cursor.position();
            codec::require_marker_padding(tail, hi_end)?;
            if admission == skyline::Admission::Refuted {
                return Err(Decode::NotCanonical);
            }
            (lo_bytes, admission)
        };
        let lo = Version::from_frozen(codec::Bits::from_canonical(buf.slice(..lo_bytes)));
        let hi = match admission {
            skyline::Admission::Equal => lo.clone(),
            skyline::Admission::Dominates => {
                Version::from_frozen(codec::Bits::from_canonical(buf.slice(lo_bytes..)))
            }
            skyline::Admission::Refuted => unreachable!("refuted admissions rejected above"),
        };
        Ok(Span {
            lo: Cow::Owned(lo),
            hi: Cow::Owned(hi),
        })
    }
}
