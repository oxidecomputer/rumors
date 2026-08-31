//! Lazy rendering of one wire item in RFC 8949 diagnostic notation.
//!
//! The adapter's events carry the observed item as one text field,
//! rendered by `cbor-diag`: extended diagnostic notation with encoding
//! indicators, embedded-CBOR tags (24 and 63) unfolded as `<<…>>`, and
//! nesting depth bounded by that library's default depth limit — a
//! structural walk past the limit fails the parse, and embedded
//! unfolds past it fall back to the plain byte-string form. On top of
//! that depth bound, one local cap bounds the rendered length, so
//! events stay cheap even when a megabyte supply run is observed.

use std::fmt;
use std::fmt::Write;

/// The rendered form's length cap in bytes. Once an item's rendering
/// crosses it, the remainder elides: no observed item, however large,
/// puts more than this in one event field.
const LENGTH_BUDGET: usize = 2048;

/// Renders a raw CBOR item in RFC 8949 diagnostic notation.
///
/// Bytes that cannot be rendered as one CBOR item (undecodable,
/// trailing bytes, or nested past the depth limit) fall back to an
/// explicit note plus capped hex: the adapter observes, it never
/// judges. A `Display` wrapper rather than an eager rendering, so the
/// work runs only when a subscriber actually formats the field.
pub(crate) struct DiagCbor<'a>(pub(crate) &'a [u8]);

impl fmt::Display for DiagCbor<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut out = match cbor_diag::parse_bytes(self.0) {
            Ok(item) => item.to_diag(),
            Err(_) => {
                // Cap the hex before encoding it, at a quarter of the
                // budget in bytes (half in hex characters), so the whole
                // note — prefix, hex, and true-length suffix — fits the
                // budget without the truncation below eating the suffix.
                let shown = self.0.len().min(LENGTH_BUDGET / 4);
                let mut out = format!("unrenderable CBOR h'{}'", hex::encode(&self.0[..shown]));
                if shown < self.0.len() {
                    let _ = write!(out, "…({} B)", self.0.len());
                }
                out
            }
        };
        if out.len() > LENGTH_BUDGET {
            // Truncation must land on a character boundary: the budget is
            // in bytes, and the rendering carries multibyte characters
            // (elision marks, escaped text) that may straddle it.
            let mut cut = LENGTH_BUDGET;
            while !out.is_char_boundary(cut) {
                cut -= 1;
            }
            out.truncate(cut);
            out.push('…');
        }
        f.write_str(&out)
    }
}

#[cfg(test)]
mod tests;
