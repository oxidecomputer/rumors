//! Equivalence pins for the word-parallel cursor against the per-bit readers it
//! replaces: same bits consumed, same values decoded, same rejects — or the
//! cursor is not the same reader.

use proptest::prelude::*;

use num_bigint::BigUint;

use crate::bits::{BitRead, BitsWriter, ReferenceBitsReader};
use crate::error::Decode;

use super::{BitsReader, Truncated};

/// Decode one gamma value from a declared live range for comparison tests.
fn decode_gamma(bytes: &[u8], len: u64, position: u64) -> Result<(BigUint, u64), Decode> {
    if position > len {
        return Err(Decode::Truncated);
    }
    let mut reader = BitsReader::from_storage(bytes, len, position);
    let value = reader.read_gamma()?;
    Ok((value, reader.position()))
}

/// The gamma reader decodes the same value from the same bits as the
/// committed decoder across the machine-word boundary.
///
/// Witnessed at the largest machine-word code (`k = 63`), the first
/// big-integer code (`k = 64`), the next (`k = 65`), and wider codes
/// (`k ≈ 100`) — widths a reader passing on narrow values alone cannot
/// fake, since our coding has no value cap while dsi-bitstream's own
/// `read_gamma` stops at `u64`.
#[test]
fn gamma_reader_matches_decoder_across_the_word_seam() {
    let wide = |p: u32| BigUint::ONE << p as usize;
    let values: Vec<BigUint> = vec![
        BigUint::from(0u64),
        BigUint::from(1u64),
        BigUint::from(30u64),        // last table-tier value
        BigUint::from(31u64),        // first past the 9-bit table
        BigUint::from(u64::MAX - 1), // k = 63: the machine-word ceiling
        BigUint::from(u64::MAX),     // k = 64: the first big-integer code
        wide(64),                    // k = 65
        wide(100),                   // far wide
        (BigUint::ONE << 100usize) + 12345u32,
    ];
    for value in &values {
        let mut bits = BitsWriter::new();
        bits.write_gamma(value);
        let (want, want_end) = decode_gamma(bits.as_raw_slice(), bits.len(), 0)
            .expect("the committed decoder reads its own encoding");
        let mut cursor = bits.reader();
        let got = cursor
            .read_gamma()
            .expect("the word-parallel reader reads the same code");
        assert_eq!(&got, &want, "value diverges at {value}");
        assert_eq!(
            cursor.position(),
            want_end,
            "consumed bits diverge at {value}"
        );
        let mut skipper = bits.reader();
        skipper
            .skip_gamma()
            .expect("the skip accepts what the read accepts");
        assert_eq!(
            skipper.position(),
            want_end,
            "skip width diverges at {value}"
        );
    }
}

/// Skipping a gamma code records exactly the bits that reading it records.
///
/// Tree walks skip payloads whose values they do not need. This test prevents
/// that shortcut from hiding work from the scan meter or charging work twice,
/// across both word-sized and arbitrary-width codes.
#[cfg(feature = "scan-meter")]
#[test]
fn skipping_gamma_records_exactly_the_bits_a_read_records() {
    for value in [
        BigUint::from(0u64),
        BigUint::from(30u64),
        BigUint::from(u64::MAX - 1), // k = 63: the machine-word ceiling
        BigUint::from(u64::MAX),     // k = 64: the first big-integer code
        BigUint::ONE << 100usize,
    ] {
        let mut bits = BitsWriter::new();
        bits.write_gamma(&value);
        let mut reader = bits.reader();
        crate::testing::meter::reset_scan_bits();
        reader
            .read_gamma()
            .expect("the reader reads its own encoding");
        let read_record = crate::testing::meter::scan_bits();
        let width = reader.position();
        let mut skipper = bits.reader();
        crate::testing::meter::reset_scan_bits();
        skipper
            .skip_gamma()
            .expect("the skip accepts what the read accepts");
        let skip_record = crate::testing::meter::scan_bits();
        assert_eq!(
            skip_record, width,
            "skip_gamma must meter exactly the {width}-bit code it skips at {value}"
        );
        assert_eq!(
            skip_record, read_record,
            "skip_gamma and read_gamma must meter the identical code identically at {value}"
        );
    }
}

/// A code truncated at every cut point reads `Truncated` from the
/// word-parallel reader exactly where the per-bit loop rejects it, at
/// widths on both sides of the word seam.
#[test]
fn truncated_codes_reject_at_every_cut_point() {
    for value in [
        BigUint::from(0u64),
        BigUint::from(500u64),
        BigUint::from(u64::MAX),
        BigUint::ONE << 100usize,
    ] {
        let mut bits = BitsWriter::new();
        bits.write_gamma(&value);
        for cut in 0..bits.len() {
            let prefix = BitsReader::with_len(bits.as_raw_slice(), cut);
            assert!(
                decode_gamma(bits.as_raw_slice(), cut, 0).is_err(),
                "the per-bit loop accepts a truncated code at {cut} of {value}"
            );
            let mut cursor = prefix;
            assert!(
                cursor.read_gamma().is_err(),
                "the word-parallel reader accepts a truncated code at {cut} of {value}"
            );
            let mut skipper = BitsReader::with_len(bits.as_raw_slice(), cut);
            assert!(
                skipper.skip_gamma().is_err(),
                "the word-parallel skip accepts a truncated code at {cut} of {value}"
            );
        }
    }
}

/// `read_unary` agrees with the per-bit loop across the buffered
/// reader's refill seams.
///
/// Runs longer than one 32-bit word and one 64-bit buffer are read
/// whole, the terminating `1` is consumed, and a run the live bits
/// never terminate rejects.
#[test]
fn unary_reads_match_the_per_bit_loop_across_word_seams() {
    for run in [0u64, 1, 7, 8, 31, 32, 33, 63, 64, 65, 200] {
        let mut bits = BitsWriter::new();
        for _ in 0..run {
            bits.push(false);
        }
        bits.push(true);
        bits.push(true); // one trailing live bit so the terminator is interior
        let mut cursor = bits.reader();
        assert_eq!(
            cursor.read_unary().expect("a terminated run reads"),
            run,
            "unary count diverges at run {run}"
        );
        assert_eq!(cursor.position(), run + 1, "the terminating 1 is consumed");
        // The same bits through the default per-bit trait loop.
        let mut reference = ReferenceBitsReader::new(bits.as_raw_slice(), bits.len(), 0);
        assert_eq!(reference.read_unary().expect("a terminated run reads"), run);
        assert_eq!(reference.position(), run + 1);

        let mut cursor = BitsReader::with_len(bits.as_raw_slice(), run);
        assert!(
            matches!(cursor.read_unary(), Err(Truncated)),
            "an unterminated run of {run} zeros must reject"
        );
    }
}

/// A mid-stream open (`new_at`) reads exactly the bits the whole-stream
/// cursor reads from that position: every prefix offset of a mixed
/// stream, at and off byte boundaries.
#[test]
fn mid_stream_opens_read_the_same_suffix() {
    let mut bits = BitsWriter::new();
    // A mixed stream: alternating flags and codes of assorted widths.
    for (flag, value) in [
        (true, 0u64),
        (false, 3),
        (true, 77),
        (false, 4096),
        (true, u64::MAX),
        (false, 12),
    ] {
        bits.push(flag);
        bits.write_gamma(&BigUint::from(value));
    }
    for pos in 0..=bits.len() {
        let mut fresh = BitsReader::from_storage(bits.as_raw_slice(), bits.len(), pos);
        let mut walked = bits.reader();
        let mut consumed = 0u64;
        while consumed < pos {
            walked.read_bit().expect("within the live length");
            consumed += 1;
        }
        for _ in 0..bits.len() - pos {
            assert_eq!(
                fresh.read_bit().expect("within the live length"),
                walked.read_bit().expect("within the live length"),
                "bit diverges after opening at {pos}"
            );
        }
        assert!(
            matches!(fresh.read_bit(), Err(Truncated)),
            "the mid-stream cursor must end at the live length"
        );
    }
}

proptest! {
    /// Differential: the word-parallel cursor matches the per-bit
    /// slice cursor on arbitrary interleavings of unary runs and
    /// gamma codes.
    ///
    /// Identical bits consumed, identical values decoded, and
    /// `skip_gamma` lands exactly where `read_gamma` does.
    #[test]
    fn arbitrary_streams_match_the_slice_cursor(
        ops in prop::collection::vec(
            prop_oneof![
                (0usize..70).prop_map(|run| (true, run as u64)),
                prop_oneof![
                    (0u64..1000).boxed(),
                    (u64::MAX - 2..=u64::MAX).boxed(),
                ].prop_map(|v| (false, v)),
            ],
            1..40,
        ),
    ) {
        let mut bits = BitsWriter::new();
        for (unary, v) in &ops {
            if *unary {
                for _ in 0..*v {
                    bits.push(false);
                }
                bits.push(true);
            } else {
                bits.write_gamma(&BigUint::from(*v));
            }
        }
        let mut reader = bits.reader();
        let mut reference = ReferenceBitsReader::new(bits.as_raw_slice(), bits.len(), 0);
        for (unary, _) in &ops {
            if *unary {
                let want = reference.read_unary().expect("the stream holds the run");
                let got = reader.read_unary().expect("the stream holds the run");
                prop_assert_eq!(got, want);
            } else {
                let want = reference.read_gamma().expect("the stream holds the code");
                let mut skipper = BitsReader::from_storage(
                    bits.as_raw_slice(),
                    bits.len(),
                    reader.position(),
                );
                let got = reader.read_gamma().expect("the stream holds the code");
                prop_assert_eq!(&got, &want);
                skipper.skip_gamma().expect("the skip accepts the code");
                prop_assert_eq!(skipper.position(), reader.position());
            }
            prop_assert_eq!(reader.position(), reference.position());
        }
    }
}
