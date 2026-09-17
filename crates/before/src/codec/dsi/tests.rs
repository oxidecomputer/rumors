//! Equivalence pins for the word-parallel cursor against the per-bit readers it
//! replaces: same bits consumed, same values decoded, same rejects — or the
//! cursor is not the same reader.

use proptest::prelude::*;

use crate::codec::cursor::Truncated;
use num_bigint::BigUint;

use crate::codec::{self, gamma, BitCursor, BitsBuf, SliceCursor};

use super::DsiCursor;

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
        let mut bits = BitsBuf::new();
        gamma::encode(value, &mut bits);
        let (want, want_end) = codec::gamma::decode(crate::codec::built_view(&bits), 0)
            .expect("the committed decoder reads its own encoding");
        let mut cursor = DsiCursor::new(crate::codec::built_view(&bits));
        let got = cursor
            .read_int()
            .expect("the word-parallel reader reads the same code");
        assert_eq!(&got, &want, "value diverges at {value}");
        assert_eq!(
            cursor.position(),
            want_end,
            "consumed bits diverge at {value}"
        );
        let mut skipper = DsiCursor::new(crate::codec::built_view(&bits));
        skipper
            .skip_int()
            .expect("the skip accepts what the read accepts");
        assert_eq!(
            skipper.position(),
            want_end,
            "skip width diverges at {value}"
        );
    }
}

/// `skip_int` meters exactly the code width it skips: the same scan
/// movement `read_int` records over the identical code, which is the
/// position delta both land on.
///
/// The skip tap is the topology walks' only scan accounting for
/// payload codes they bypass, and this equality is its committed
/// floor witness: the board's universal scan floor sits far under the
/// stored rate, so a silenced or partial skip tap clears every slack
/// floor and is otherwise caught only by an argmax ranking table. The
/// pin is two-sided — an undercount and an overcount both move it —
/// and covers both gamma paths (machine-word and big-integer).
#[cfg(feature = "scan-meter")]
#[test]
fn skip_int_meters_exactly_the_code_width_read_int_pays() {
    for value in [
        BigUint::from(0u64),
        BigUint::from(30u64),
        BigUint::from(u64::MAX - 1), // k = 63: the machine-word ceiling
        BigUint::from(u64::MAX),     // k = 64: the first big-integer code
        BigUint::ONE << 100usize,
    ] {
        let mut bits = BitsBuf::new();
        gamma::encode(&value, &mut bits);
        let mut reader = DsiCursor::new(crate::codec::built_view(&bits));
        crate::meter::reset_scan_bits();
        reader
            .read_int()
            .expect("the reader reads its own encoding");
        let read_record = crate::meter::scan_bits();
        let width = reader.position();
        let mut skipper = DsiCursor::new(crate::codec::built_view(&bits));
        crate::meter::reset_scan_bits();
        skipper
            .skip_int()
            .expect("the skip accepts what the read accepts");
        let skip_record = crate::meter::scan_bits();
        assert_eq!(
            skip_record, width,
            "skip_int must meter exactly the {width}-bit code it skips at {value}"
        );
        assert_eq!(
            skip_record, read_record,
            "skip_int and read_int must meter the identical code identically at {value}"
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
        let mut bits = BitsBuf::new();
        gamma::encode(&value, &mut bits);
        for cut in 0..bits.len() {
            let prefix = codec::BitsView::new(bits.as_raw_slice(), cut);
            assert!(
                codec::gamma::decode(prefix, 0).is_err(),
                "the per-bit loop accepts a truncated code at {cut} of {value}"
            );
            let mut cursor = DsiCursor::new(prefix);
            assert!(
                cursor.read_int().is_err(),
                "the word-parallel reader accepts a truncated code at {cut} of {value}"
            );
            let mut skipper = DsiCursor::new(prefix);
            assert!(
                skipper.skip_int().is_err(),
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
        let mut bits = BitsBuf::new();
        for _ in 0..run {
            bits.push(false);
        }
        bits.push(true);
        bits.push(true); // one trailing live bit so the terminator is interior
        let mut cursor = DsiCursor::new(crate::codec::built_view(&bits));
        assert_eq!(
            cursor.read_unary().expect("a terminated run reads"),
            run,
            "unary count diverges at run {run}"
        );
        assert_eq!(cursor.position(), run + 1, "the terminating 1 is consumed");
        // The same bits through the default per-bit trait loop.
        let mut slice = SliceCursor::new(crate::codec::built_view(&bits), 0);
        assert_eq!(slice.read_unary().expect("a terminated run reads"), run);
        assert_eq!(slice.position(), run + 1);

        let unterminated = codec::BitsView::new(bits.as_raw_slice(), run);
        let mut cursor = DsiCursor::new(unterminated);
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
    let mut bits = BitsBuf::new();
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
        gamma::encode(&BigUint::from(value), &mut bits);
    }
    for pos in 0..=bits.len() {
        let mut fresh = DsiCursor::new_at(crate::codec::built_view(&bits), pos);
        let mut walked = DsiCursor::new(crate::codec::built_view(&bits));
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
    /// `skip_int` lands exactly where `read_int` does.
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
        let mut bits = BitsBuf::new();
        for (unary, v) in &ops {
            if *unary {
                for _ in 0..*v {
                    bits.push(false);
                }
                bits.push(true);
            } else {
                gamma::encode(&BigUint::from(*v), &mut bits);
            }
        }
        let mut dsi = DsiCursor::new(crate::codec::built_view(&bits));
        let mut slice = SliceCursor::new(crate::codec::built_view(&bits), 0);
        for (unary, _) in &ops {
            if *unary {
                let want = slice.read_unary().expect("the stream holds the run");
                let got = dsi.read_unary().expect("the stream holds the run");
                prop_assert_eq!(got, want);
            } else {
                let want = slice.read_int().expect("the stream holds the code");
                let mut skipper =
                    DsiCursor::new_at(crate::codec::built_view(&bits), dsi.position());
                let got = dsi.read_int().expect("the stream holds the code");
                prop_assert_eq!(&got, &want);
                skipper.skip_int().expect("the skip accepts the code");
                prop_assert_eq!(skipper.position(), dsi.position());
            }
            prop_assert_eq!(dsi.position(), slice.position());
        }
    }
}
