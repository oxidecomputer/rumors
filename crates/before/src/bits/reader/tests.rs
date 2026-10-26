//! Equivalence pins for the word-parallel cursor against the per-bit readers it
//! replaces: same bits consumed, same values decoded, same rejects, or the
//! cursor is not the same reader.

use proptest::prelude::*;

use num_bigint::BigUint;

use crate::bits::{BitRead, BitsWriter, ReferenceBitsReader};
use crate::error::Decode;

use super::gamma::code_len_within;
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
/// (`k ≈ 100`): widths a reader passing on narrow values alone cannot
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

/// Skipping a gamma code advances the cursor exactly as far as reading it.
///
/// Tree walks skip payloads whose values they do not need, so a skip that
/// stopped short or ran on would misalign every later read. The codes span
/// word-sized and arbitrary-width values, the machine-word ceiling included.
#[test]
fn skipping_gamma_advances_exactly_as_far_as_a_read() {
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
        reader
            .read_gamma()
            .expect("the reader reads its own encoding");
        let mut skipper = bits.reader();
        skipper
            .skip_gamma()
            .expect("the skip accepts what the read accepts");
        assert_eq!(
            skipper.position(),
            reader.position(),
            "skip_gamma must consume exactly the code read_gamma consumes at {value}"
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

/// One step of a differential stream: a unary run of `0` bits closed by a
/// `1`, or one gamma code.
#[derive(Clone, Debug)]
enum StreamOp {
    /// A run of this many zeros, then its terminator.
    Unary(u64),
    /// The gamma code of this value.
    Gamma(BigUint),
}

/// Gamma values from every decoding tier: the table, one machine word, and
/// arbitrary width.
///
/// The edge arm sets the mantissa (the value plus one) to a power of two
/// `2^zeros` up to [`WIDE_GAMMA_BITS`], with its low bits nearly all clear or
/// nearly all set, so the codes' widths cover every limb alignment and their
/// mantissas cover both carry extremes of the final decrement. The random arm
/// draws whole magnitudes of every limb count below that width.
fn arb_gamma_value() -> impl Strategy<Value = BigUint> {
    prop_oneof![
        (0u64..1000).prop_map(BigUint::from),
        (u64::MAX - 2..=u64::MAX).prop_map(BigUint::from),
        (0..WIDE_GAMMA_BITS, 0u8..4, any::<bool>()).prop_map(|(zeros, offset, set)| {
            let floor = BigUint::ONE << zeros;
            let offset = BigUint::from(offset) % &floor;
            let mantissa = if set {
                (&floor << 1u8) - 1u8 - offset
            } else {
                floor + offset
            };
            mantissa - 1u8
        }),
        prop::collection::vec(any::<u32>(), 0..(WIDE_GAMMA_BITS / 32) as usize)
            .prop_map(BigUint::new),
    ]
}

/// The widest mantissa [`arb_gamma_value`] draws, in bits past its leading
/// one: several 64-bit words, so a wide code spans many limbs and buffer
/// refills.
const WIDE_GAMMA_BITS: u64 = 600;

proptest! {
    /// Differential: the word-parallel cursor matches the per-bit
    /// slice cursor on arbitrary interleavings of unary runs and
    /// gamma codes of every width.
    ///
    /// Identical bits consumed, identical values decoded (and equal to the
    /// values written), and `skip_gamma` lands exactly where `read_gamma`
    /// does. The unary runs shift each code to an arbitrary bit offset.
    #[test]
    fn arbitrary_streams_match_the_slice_cursor(
        ops in prop::collection::vec(
            prop_oneof![
                (0u64..70).prop_map(StreamOp::Unary),
                arb_gamma_value().prop_map(StreamOp::Gamma),
            ],
            1..40,
        ),
    ) {
        let mut bits = BitsWriter::new();
        for op in &ops {
            match op {
                StreamOp::Unary(run) => {
                    for _ in 0..*run {
                        bits.push(false);
                    }
                    bits.push(true);
                }
                StreamOp::Gamma(value) => bits.write_gamma(value),
            }
        }
        let mut reader = bits.reader();
        let mut reference = ReferenceBitsReader::new(bits.as_raw_slice(), bits.len(), 0);
        for op in &ops {
            match op {
                StreamOp::Unary(_) => {
                    let want = reference.read_unary().expect("the stream holds the run");
                    let got = reader.read_unary().expect("the stream holds the run");
                    prop_assert_eq!(got, want);
                }
                StreamOp::Gamma(value) => {
                    let want = reference.read_gamma().expect("the stream holds the code");
                    let mut skipper = BitsReader::from_storage(
                        bits.as_raw_slice(),
                        bits.len(),
                        reader.position(),
                    );
                    let got = reader.read_gamma().expect("the stream holds the code");
                    prop_assert_eq!(&got, &want);
                    prop_assert_eq!(&got, value);
                    skipper.skip_gamma().expect("the skip accepts the code");
                    prop_assert_eq!(skipper.position(), reader.position());
                }
            }
            prop_assert_eq!(reader.position(), reference.position());
        }
    }
}

/// Prefix lengths within this distance of the overflow point are checked
/// exhaustively.
const PREFIX_WINDOW: u64 = 64;

/// The fewest leading zeros whose gamma code is too long for a `u64`.
const OVERSIZED_PREFIX: u64 = 1 << 63;

/// The length a gamma code with `zeros` leading zeros needs, computed in
/// `u128`, if it fits within `remaining` bits: the oracle for
/// [`code_len_within`].
fn widened_code_len(zeros: u64, remaining: u64) -> Option<u64> {
    let code_len = 2 * u128::from(zeros) + 1;
    (code_len <= u128::from(remaining)).then_some(code_len as u64)
}

/// A gamma code fits exactly when its full length fits in the remaining bits,
/// even for prefixes of 2^63 zeros or more, whose length no `u64` holds.
///
/// The length counts the prefix, the terminator, and the mantissa. The check
/// compares against widened arithmetic over every prefix near 2^63 zeros and
/// every remainder at the top of the range.
#[test]
fn gamma_code_length_fits_exactly_near_the_u64_limit() {
    let prefixes = OVERSIZED_PREFIX - PREFIX_WINDOW..=OVERSIZED_PREFIX + PREFIX_WINDOW;
    for zeros in prefixes {
        for remaining in (u64::MAX - PREFIX_WINDOW..=u64::MAX).chain([zeros, zeros + 1]) {
            assert_eq!(
                code_len_within(zeros, remaining),
                widened_code_len(zeros, remaining),
                "{zeros} zeros within {remaining} bits"
            );
        }
    }
    assert_eq!(
        code_len_within(OVERSIZED_PREFIX - 1, u64::MAX),
        Some(u64::MAX)
    );
    assert_eq!(code_len_within(OVERSIZED_PREFIX, u64::MAX), None);
}

proptest! {
    /// Across all of `u64`, a gamma code's fit agrees with widened
    /// arithmetic.
    #[test]
    fn gamma_code_length_fit_agrees_with_widened_arithmetic(
        zeros in prop_oneof![any::<u64>(), OVERSIZED_PREFIX - PREFIX_WINDOW..=OVERSIZED_PREFIX],
        remaining in any::<u64>(),
    ) {
        prop_assert_eq!(code_len_within(zeros, remaining), widened_code_len(zeros, remaining));
    }
}
