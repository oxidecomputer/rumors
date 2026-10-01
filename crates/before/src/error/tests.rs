//! Error-chain behavior.

use std::error::Error as _;
use std::io;

use super::{Decode, ParseValue};

/// `Decode::Io` exposes the original reader failure through the standard error
/// chain without changing its direct display text.
#[test]
fn io_decode_error_preserves_its_source() {
    let error = Decode::Io(io::Error::other("reader failed"));
    assert_eq!(error.to_string(), "read error: reader failed");
    assert_eq!(error.source().unwrap().to_string(), "reader failed");
}

/// `ParseValue::InvalidEncoding` exposes the canonical-decoding failure that
/// distinguishes it from malformed text.
#[test]
fn value_parse_error_preserves_its_source() {
    let error = ParseValue::InvalidEncoding(Decode::NotCanonical);
    assert!(
        matches!(error.source(), Some(source) if source.to_string() == "input is not canonical")
    );
}
