use super::*;

/// The largest record `push` admits: with its own record header charged, the
/// smallest run body holding it exactly fills the outer `u32` frame header.
const LARGEST_ENCODABLE_RECORD: usize = u32::MAX as usize - LENGTH_HEADER_LEN;

/// Push's capacity check is eager and charges the record header.
///
/// A record is admitted exactly when its bytes plus its own header fit the
/// outer `u32` frame header, so a record with length in
/// `(u32::MAX - 4, u32::MAX]` fails at record level rather than later at
/// the outer frame.
#[test]
fn record_capacity_charges_the_record_header() {
    assert!(checked_record_header(LARGEST_ENCODABLE_RECORD).is_ok());
    for unshippable in [
        LARGEST_ENCODABLE_RECORD + 1,
        u32::MAX as usize,
        u32::MAX as usize + 1,
    ] {
        let error = checked_record_header(unshippable)
            .expect_err("a record past the header-charged boundary must fail");
        assert_eq!(error.len, unshippable.saturating_add(LENGTH_HEADER_LEN));
    }
}

/// The checked header encodes the record's own length, not the charged sum:
/// the header-charged boundary changes only admission, never the wire bytes
/// of an admitted record.
#[test]
fn checked_header_encodes_the_bare_record_length() {
    let len = 7;
    assert_eq!(
        checked_record_header(len).expect("a small record is admitted"),
        (len as u32).to_be_bytes(),
    );
}
