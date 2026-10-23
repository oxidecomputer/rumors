//! Shared mechanics for the public textual forms.
//!
//! [`Party`](crate::Party) and [`Version`](crate::Version) bytes use strict
//! hexadecimal text. This module keeps that shared policy in one place while
//! leaving byte validation to each type.

use core::fmt;

use crate::error::{Decode, ParseValue};

/// Write bytes as lowercase hexadecimal.
pub fn write_hex(bytes: &[u8], f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(&hex::encode(bytes))
}

/// Decode an even-length hexadecimal component, then validate the resulting
/// canonical bytes as `T`.
///
/// The bytes land in a vector allocated once at their exact length, so the
/// value that adopts them needs no reallocation to drop spare capacity.
pub fn decode_hex<T>(
    text: &str,
    decode: impl FnOnce(Vec<u8>) -> Result<T, Decode>,
) -> Result<T, ParseValue> {
    let mut decoded = vec![0; text.len() / 2];
    hex::decode_to_slice(text, &mut decoded).map_err(|_| ParseValue::InvalidSyntax)?;
    decode(decoded).map_err(ParseValue::InvalidEncoding)
}

#[cfg(test)]
mod tests;
