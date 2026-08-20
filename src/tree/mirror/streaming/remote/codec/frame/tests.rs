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

/// `record_len` prices exactly what `push` writes, at every CBOR
/// byte-string header width a version can occupy.
///
/// The two are the same quantity computed two ways — arithmetic against
/// actual encoding — so the run-budget math can trust the closed form.
/// Deep version chains grow the canonical encoding through the 1-byte
/// (< 24), 2-byte (< 256), and 3-byte (< 65536) CBOR header regimes; the
/// chain lengths below land encodings in the first two and the message
/// sizes sweep the payload term.
#[test]
fn record_len_matches_an_actual_push() {
    let mut version = crate::Version::new();
    let mut checked_regimes = std::collections::BTreeSet::new();
    for parties in 1..=128u32 {
        // One tick on a fresh disjoint party per step: each new party's
        // event widens the canonical encoding, marching it through the
        // CBOR header-width regimes.
        version.tick(&crate::tree::arb::nth_party(parties as usize));
        if !(parties == 1 || parties % 17 == 0) {
            continue;
        }
        checked_regimes.insert(super::cbor_bytes_header_len(version.as_bytes().len()));
        for message in [Message::new(0u64), Message::new(u64::MAX)] {
            let mut run = LeafRun::<u64>::new();
            run.push(&version, &message).expect("test records fit");
            assert_eq!(
                run.encoded_len(),
                LeafRun::<u64>::record_len(&version, &message),
                "record_len must price exactly one pushed record",
            );
            // The version atom `push` writes is byte-identical to the
            // serde form the decoder parses (ciborium's byte string).
            let mut serde_form = Vec::new();
            ciborium::ser::into_writer(&version, &mut serde_form).unwrap();
            assert_eq!(
                &run.as_bytes()[LENGTH_HEADER_LEN..LENGTH_HEADER_LEN + serde_form.len()],
                serde_form.as_slice(),
                "push's hand-written CBOR header must match ciborium's",
            );
        }
    }
    assert!(
        checked_regimes.len() >= 2,
        "the sweep must cross at least two CBOR header-width regimes, got {checked_regimes:?}",
    );
}
