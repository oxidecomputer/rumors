//! The wire form: [`Span`]'s canonical byte encoding and its strict decode.
//!
//! The composite is the two endpoints' canonical encodings concatenated —
//! each byte-aligned, independently canonical, and self-delimiting, so no
//! length prefix exists — and the decode proves the pair ordered in the
//! same pass that parses it. The public wire-form contract lives in the
//! [span module docs](super); this module is its implementation.

use std::borrow::Cow;
use std::io::{self, Read, Write};

use crate::codec;
use crate::error::Decode;
use crate::version::skyline;
use crate::Version;

use super::Span;

impl<'a> Span<'a> {
    /// Encodes this [`Span`] as canonical bytes.
    ///
    /// Each endpoint is byte-aligned, independently canonical, and
    /// self-delimiting, so the two concatenate with no length prefix (the
    /// [module docs](super) carry the wire form). Byte equality on these
    /// composites is exactly span equality.
    ///
    /// # Complexity
    ///
    /// `O(|self|)`.
    ///
    /// # Example
    ///
    /// ```
    /// use before::{causally::Span, Clock};
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
    /// `O(|self|)`.
    ///
    /// # Example
    ///
    /// ```
    /// use before::{causally::Span, Clock};
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

    /// Decodes a [`Span`] from a reader of canonical bytes, strictly
    /// rejecting everything else.
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
    /// `O(n)`, with `n` the bytes read, regardless of whether accepted or
    /// rejected.
    ///
    /// # Example
    ///
    /// ```
    /// use before::{causally::Span, error::Decode, Clock};
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
        // Both components are validated against the borrowed buffer first; the
        // endpoints then adopt slices of that one buffer, so the whole span
        // shares a single allocation.
        //
        // The meet is the byte-aligned self-delimiting prefix: parse its tree
        // to find the split and check its padding. The join's admission walk
        // parses its stream while deciding, in the same pass, whether it
        // dominates — or equals — the meet: never a parse and then a second
        // comparison walk.
        //
        // The pair verdict is pronounced last, after the padding check, so a
        // composite defective several ways rejects by its structural genre
        // first, exactly as decoding the components would.
        let (lo_bytes, admission) = {
            let bits = codec::bytes_as_bits(&buf);
            let lo_end = skyline::validate_prefix(bits)?;
            // The meet's padding marker rides in its final byte — which an
            // input cut right after a flush stream lacks. That cut is
            // missing required data (the marker byte, and the whole join
            // after it): the truncation genre, exactly as a byte-starved
            // reader reports the same boundary.
            let lo_bytes = (lo_end + 1).div_ceil(8);
            if 8 * lo_bytes > bits.len() {
                return Err(Decode::Truncated);
            }
            codec::require_marker_padding(&bits[..8 * lo_bytes], lo_end)?;
            let tail = &bits[8 * lo_bytes..];
            let mut cursor = codec::DsiCursor::new(tail);
            let admission = skyline::validate_dominating_from(&bits[..lo_end], &mut cursor)?;
            let hi_end = codec::BitCursor::position(&cursor);
            codec::require_marker_padding(tail, hi_end)?;
            if admission == skyline::Admission::Refuted {
                return Err(Decode::NotCanonical);
            }
            (lo_bytes, admission)
        };
        let buf = bytes::Bytes::from(buf);
        let lo = Version::from_frozen(codec::Bits::from_canonical(buf.slice(..lo_bytes)));
        let hi = match admission {
            // The coincident span stores one buffer twice: the admission walk
            // proved the second stream byte-equal to the first, so the join is
            // the meet's clone: an `O(1)` refcount bump the ptr_eq fast paths
            // then recognize.
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
