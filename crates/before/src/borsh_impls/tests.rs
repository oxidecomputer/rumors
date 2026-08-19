use borsh::io::{Error, ErrorKind, Read};
use borsh::{BorshDeserialize, BorshSerialize};
use proptest::prelude::*;
use proptest::test_runner::TestCaseError;

use super::decode_error;
use crate::codec::{self, BitCursor, BitsBuf};
use crate::error::Decode;
use crate::span::Span;
use crate::testing::bridge::{from_oracle_party, from_oracle_version};
use crate::testing::generators::{
    arb_oracle_party_nonempty, arb_oracle_version, deep_left_spine_party,
};
use crate::testing::optrace::{step_impl, world_strategy};
use crate::version::decode_rank_stream;
use crate::version::skyline::{validate_dominating_from, Admission};
use crate::{Clock, Party, Rank, Ranked, Version};

/// A borsh stream that ends mid-tree surfaces the reader's own I/O error
/// (`UnexpectedEof`), never a masked decode error.
///
/// `Decode::Io` is the one rich variant the bit-level error split must still
/// deliver losslessly through `ReaderCursor`: the prefix-free encodings pad
/// only to the next byte boundary, so every proper byte prefix of a canonical
/// encoding ends mid-tree or ahead of the tree's padding, and the next
/// per-bit refill hits end-of-input. The
/// wide value's 129-bit gamma code exceeds the 64-bit decode window, so its
/// cuts land inside the per-bit fallback's refill loop as well as around it.
#[test]
fn truncated_borsh_stream_reports_unexpected_eof() {
    let mut party = Party::seed();
    for _ in 0..4 {
        let _ = party.fork(); // deepen the id tree past one byte
    }
    let version: Version = "(1, 2, (0, (1, 0, 2), 0))".parse().unwrap();
    let wide: Version = "(0, 18446744073709551615, 0)".parse().unwrap();

    let party_bytes = borsh::to_vec(&party).unwrap();
    let version_bytes = borsh::to_vec(&version).unwrap();
    let wide_bytes = borsh::to_vec(&wide).unwrap();
    assert!(
        party_bytes.len() >= 2,
        "the id tree must span several bytes"
    );
    assert!(
        version_bytes.len() >= 2,
        "the event tree must span several bytes"
    );
    assert!(
        wide_bytes.len() >= 16,
        "the wide gamma code must overrun one 64-bit window"
    );

    for cut in 0..party_bytes.len() {
        let err = Party::try_from_slice(&party_bytes[..cut]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::UnexpectedEof, "cut at byte {cut}");
    }
    for cut in 0..version_bytes.len() {
        let err = Version::try_from_slice(&version_bytes[..cut]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::UnexpectedEof, "cut at byte {cut}");
    }
    for cut in 0..wide_bytes.len() {
        let err = Version::try_from_slice(&wide_bytes[..cut]).unwrap_err();
        assert_eq!(
            err.kind(),
            ErrorKind::UnexpectedEof,
            "wide cut at byte {cut}"
        );
    }
    // And uncut, the window-defying code still decodes through the fallback.
    assert_eq!(
        Version::try_from_slice(&wide_bytes).expect("the full wide code decodes"),
        wide,
    );
}

/// A canonicality violation crosses the borsh boundary as `InvalidData`
/// carrying the exact [`Decode`] variant, never conflated with truncation.
///
/// The input is the collapsible id `(1, 1)` — tag `11`, then two terminal
/// `00`s — which id normal form forbids ([`Decode::NotCanonical`]).
#[test]
fn non_canonical_borsh_bytes_report_invalid_data() {
    let err = Party::try_from_slice(&[0b1100_0000]).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::InvalidData);
    let inner = err
        .get_ref()
        .expect("the io error carries the Decode error");
    assert!(
        matches!(inner.downcast_ref::<Decode>(), Some(Decode::NotCanonical)),
        "expected NotCanonical, got: {inner:?}"
    );
}

/// An arbitrary impl `Version` whose live bits end flush against a byte
/// boundary, so its canonical padding occupies a whole final `1000_0000`
/// byte.
fn arb_flush_version() -> impl Strategy<Value = Version> {
    arb_oracle_version()
        .prop_map(|t| from_oracle_version(&t))
        .prop_filter("live bits must end on a byte boundary", |v| {
            v.encoded_bits().is_multiple_of(8)
        })
}

/// As [`arb_flush_version`], for `Party`.
fn arb_flush_party() -> impl Strategy<Value = Party> {
    arb_oracle_party_nonempty()
        .prop_map(|t| from_oracle_party(&t))
        .prop_filter("live bits must end on a byte boundary", |p| {
            p.encoded_bits().is_multiple_of(8)
        })
}

/// The 1-byte flush-cut witness reads the truncation genre through both
/// doors.
///
/// `Version::try_from(7)` encodes to eight live bits plus a whole
/// `1000_0000` padding byte; on its first byte alone the reader door
/// starves reading the absent padding byte (`UnexpectedEof`), and the
/// slice door reports [`Decode::Truncated`] for the same bytes. This is
/// the mapping the decode differential relies on: `UnexpectedEof` is
/// exactly raw `Truncated`.
#[test]
fn flush_cut_is_the_truncation_genre_through_both_doors() {
    let bytes = Version::try_from(7).unwrap().encode();
    assert_eq!(
        bytes,
        vec![0b1000_1000, 0b1000_0000],
        "witness canonical encoding"
    );
    let cut = &bytes[..1];
    assert!(matches!(Version::decode(cut), Err(Decode::Truncated)));
    let err = Version::try_from_slice(cut).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::UnexpectedEof);
}

proptest! {
    /// A stream cut exactly at a flush byte boundary reads the truncation
    /// genre through both doors of every version-tailed wire type.
    ///
    /// The borsh reader starves on the absent `1000_0000` padding byte
    /// (`UnexpectedEof`) exactly where the whole-slice decode reports
    /// [`Decode::Truncated`] — the two doors report the same genre for the
    /// same malformed input, at the end of the input (`Version`, and the
    /// version tail of `Clock`, `Ranked`, and `Span`) and at the interior
    /// seam (`Span`'s meet cut short of its own padding byte).
    #[test]
    fn flush_cut_version_truncation_genre_agrees_across_doors(
        v in arb_flush_version(),
        pa in arb_oracle_party_nonempty(),
    ) {
        let bytes = v.encode();
        let cut = &bytes[..bytes.len() - 1];
        prop_assert!(matches!(Version::decode(cut), Err(Decode::Truncated)));
        prop_assert_eq!(
            Version::try_from_slice(cut).unwrap_err().kind(),
            ErrorKind::UnexpectedEof
        );

        // The version tail of a clock.
        let clock = Clock::from_parts(from_oracle_party(&pa), v.clone());
        let clock_bytes = clock.encode();
        let clock_cut = &clock_bytes[..clock_bytes.len() - 1];
        prop_assert!(matches!(Clock::decode(clock_cut), Err(Decode::Truncated)));
        prop_assert_eq!(
            Clock::try_from_slice(clock_cut).unwrap_err().kind(),
            ErrorKind::UnexpectedEof
        );

        // The version tail of a ranked key.
        let key = Ranked::from(&v).encode();
        let key_cut = &key[..key.len() - 1];
        prop_assert!(matches!(Ranked::decode(key_cut), Err(Decode::Truncated)));
        prop_assert_eq!(
            Ranked::try_from_slice(key_cut).unwrap_err().kind(),
            ErrorKind::UnexpectedEof
        );

        // The join tail of a span (the hull of the empty version and `v`).
        let span = Version::new().span(&v).encode();
        let span_cut = &span[..span.len() - 1];
        prop_assert!(matches!(Span::decode(span_cut), Err(Decode::Truncated)));
        prop_assert_eq!(
            Span::try_from_slice(span_cut).unwrap_err().kind(),
            ErrorKind::UnexpectedEof
        );

        // The interior seam: the meet cut short of its own padding byte,
        // the join missing entirely.
        prop_assert!(matches!(Span::decode(cut), Err(Decode::Truncated)));
        prop_assert_eq!(
            Span::try_from_slice(cut).unwrap_err().kind(),
            ErrorKind::UnexpectedEof
        );
    }
}

proptest! {
    /// A party stream cut exactly at a flush byte boundary reads the
    /// truncation genre through both doors.
    ///
    /// `UnexpectedEof` from the borsh reader, [`Decode::Truncated`] from
    /// the whole-slice decode — at the end of the input (`Party`) and at
    /// the clock door's interior seam (the id section cut short of its own
    /// padding byte, the version then missing entirely).
    #[test]
    fn flush_cut_party_truncation_genre_agrees_across_doors(p in arb_flush_party()) {
        let bytes = p.encode();
        let cut = &bytes[..bytes.len() - 1];
        prop_assert!(matches!(Party::decode(cut), Err(Decode::Truncated)));
        prop_assert_eq!(
            Party::try_from_slice(cut).unwrap_err().kind(),
            ErrorKind::UnexpectedEof
        );
        prop_assert!(matches!(Clock::decode(cut), Err(Decode::Truncated)));
        prop_assert_eq!(
            Clock::try_from_slice(cut).unwrap_err().kind(),
            ErrorKind::UnexpectedEof
        );
    }
}

/// Regression: a `Party` grown by [`join`](Party::join) — the operation
/// [`reclaim`](crate::bookmark) drives on a reboot — must survive the borsh
/// wire round-trip.
///
/// `fork; fork; join` reunites two quarter-regions into `(0, 1)`, normalizing
/// the build buffer's tree to a smaller one. The collapse sheds bits into the
/// buffer's final byte, where an unsealed freeze would leave them for
/// [`as_bytes`](Party::as_bytes) — which borsh serializes — to carry as
/// garbage the peer's [`Party::decode`] rejects as `TrailingBits`, silently
/// dropping a donated identity on the wire. This is the one-shot witness that
/// the freeze seals the shed bits behind the canonical padding.
#[test]
fn joined_party_roundtrips_through_borsh() {
    let mut left = Party::seed();
    let right = left.fork(); // left = (1, 0), right = (0, 1)
    let mut right = right;
    let rb = right.fork(); // right = (0, (1, 0)), rb = (0, (0, 1))
    right.join(rb).expect("the two quarters are disjoint");

    assert_eq!(
        right,
        "(0, 1)".parse().unwrap(),
        "the quarters reunite to (0, 1)"
    );
    assert_eq!(
        right.as_bytes(),
        right.encode().as_slice(),
        "stored bytes must be canonical after a normalizing join",
    );
    let bytes = borsh::to_vec(&right).expect("serialize");
    assert_eq!(
        Party::try_from_slice(&bytes).expect("a joined party must decode"),
        right,
    );
}

proptest! {
    /// Every `Party`/`Version`/`Clock` reachable by an arbitrary impl-driven
    /// history (`fork`/`join`/`sync`/`tick` via [`step_impl`]) round-trips
    /// through borsh — the on-wire form the gossip protocol ships.
    ///
    /// Drives the impl's own operations, not oracle-lowered values, so it
    /// covers the reused-buffer path that the per-view equivalence test also
    /// guards.
    #[test]
    fn borsh_roundtrips_over_impl_history(ops in world_strategy()) {
        let mut imp = vec![Clock::seed()];
        for op in &ops {
            step_impl(&mut imp, op);
        }
        for c in &imp {
            let (p, v) = (c.party(), c.version());
            let party_bytes = borsh::to_vec(p).unwrap();
            let version_bytes = borsh::to_vec(v).unwrap();
            let clock_bytes = borsh::to_vec(c).unwrap();

            prop_assert_eq!(party_bytes.as_slice(), p.as_bytes());
            prop_assert_eq!(version_bytes.as_slice(), v.as_bytes());
            prop_assert_eq!(&clock_bytes, &c.encode());

            prop_assert_eq!(&Party::try_from_slice(&party_bytes).unwrap(), p);
            prop_assert_eq!(&Version::try_from_slice(&version_bytes).unwrap(), v);
            let back = Clock::try_from_slice(&clock_bytes).unwrap();
            prop_assert_eq!(back.party(), p);
            prop_assert_eq!(back.version(), v);
        }
    }
}

// ─────────────── wire cursor ≡ per-bit reference (differential) ───────────────
//
// `ReaderCursor` reads bits by byte-index and integers through the word
// window over its already-read bytes; the per-bit cursor below — a growing
// bit buffer, per-bit reads only — is the specification. These tests pin the
// wire decode to it differentially: same accepts, same rejects, same error
// variants, and byte-for-byte identical consumption from the reader.

/// The per-bit reference cursor, the wire decode's differential oracle.
///
/// The definitional shape with no fast paths: a growing `BitVec` refilled one
/// byte at a time, per-bit reads only, and the default per-bit `read_int`.
struct BitwiseReaderCursor<'a, R> {
    reader: &'a mut R,
    bits: BitsBuf,
    position: u64,
}

impl<R: Read> BitCursor for BitwiseReaderCursor<'_, R> {
    type Error = Decode;

    fn read_bit(&mut self) -> Result<bool, Decode> {
        if self.position == self.bits.len() {
            let mut byte = [0];
            self.reader.read_exact(&mut byte).map_err(Decode::Io)?;
            self.bits.push_bits(u64::from(byte[0]), 8);
        }
        let bit = self.bits.get(self.position);
        self.position += 1;
        Ok(bit)
    }

    fn position(&self) -> u64 {
        self.position
    }
}

/// Consume a tree's padding through per-bit reads: one `1` marker, then
/// zeros to the byte boundary — reading the whole-byte marker when the
/// live bits end flush against a boundary, exactly as the wire cursor's
/// `finish` does.
fn reference_consume_padding<R: Read>(
    cursor: &mut BitwiseReaderCursor<'_, R>,
) -> Result<(), Decode> {
    if !cursor.read_bit()? {
        return Err(Decode::TrailingBits);
    }
    while !cursor.position.is_multiple_of(8) {
        if cursor.read_bit()? {
            return Err(Decode::TrailingBits);
        }
    }
    Ok(())
}

/// Decode one event tree per-bit through [`BitwiseReaderCursor`].
///
/// Replicates the wire pipeline stage for stage: parse, padding
/// consumption, truncate to the consumed live bits.
fn reference_version<R: Read>(reader: &mut R) -> Result<Version, Decode> {
    let mut cursor = BitwiseReaderCursor {
        reader,
        bits: BitsBuf::new(),
        position: 0,
    };
    crate::version::skyline::validate_from(&mut cursor)?;
    let position = cursor.position;
    reference_consume_padding(&mut cursor)?;
    let mut bits = cursor.bits;
    bits.truncate(position);
    Ok(Version::from_bits(bits))
}

/// Decode one id tree per-bit through [`BitwiseReaderCursor`].
///
/// Replicates the wire pipeline stage for stage: parse, padding
/// consumption, truncate to the consumed live bits. The id grammar has no
/// empty production, so the parsed id is a nonzero share — exactly as
/// `Party::deserialize_reader` relies on.
fn reference_party<R: Read>(reader: &mut R) -> Result<Party, Decode> {
    let mut cursor = BitwiseReaderCursor {
        reader,
        bits: BitsBuf::new(),
        position: 0,
    };
    codec::parse_id_from(&mut cursor)?;
    let position = cursor.position;
    reference_consume_padding(&mut cursor)?;
    let mut bits = cursor.bits;
    bits.truncate(position);
    Ok(Party::from_bits(bits))
}

/// Assert two wire decode errors agree on `ErrorKind` and, when both carry a
/// [`Decode`], on its variant.
fn assert_same_error(subject: &Error, oracle: &Error) -> Result<(), TestCaseError> {
    prop_assert_eq!(subject.kind(), oracle.kind());
    let variant = |e: &Error| {
        e.get_ref()
            .and_then(|inner| inner.downcast_ref::<Decode>())
            .map(std::mem::discriminant)
    };
    prop_assert_eq!(variant(subject), variant(oracle));
    Ok(())
}

/// Layer the adversarial wire shapes over a canonical-encoding strategy: raw
/// noise, or an encoding optionally bit-flipped, truncated, and tailed with
/// junk that only a speculative read would touch.
fn arb_stream(encoding: impl Strategy<Value = Vec<u8>>) -> impl Strategy<Value = Vec<u8>> {
    prop_oneof![
        proptest::collection::vec(any::<u8>(), 0..24),
        (
            encoding,
            proptest::option::of(any::<proptest::sample::Index>()),
            proptest::option::of(any::<proptest::sample::Index>()),
            proptest::collection::vec(any::<u8>(), 0..8),
        )
            .prop_map(|(mut bytes, flip, cut, tail)| {
                if let Some(flip) = flip {
                    let bit = flip.index(bytes.len() * 8);
                    bytes[bit / 8] ^= 0x80u8 >> (bit % 8);
                }
                if let Some(cut) = cut {
                    bytes.truncate(cut.index(bytes.len() + 1));
                }
                bytes.extend(tail);
                bytes
            }),
    ]
}

proptest! {
    /// The windowed wire cursor decodes a `Version` stream exactly like the
    /// per-bit reference: same value or same error, same bytes consumed.
    ///
    /// Byte consumption is the wire contract — the bytes after the encoding
    /// belong to the next borsh field — so it is asserted unconditionally,
    /// on accepts and rejects alike. Streams cover canonical encodings, bit
    /// flips, truncations, raw noise, and trailing junk.
    #[test]
    fn version_wire_decode_matches_bitwise_reference(
        stream in arb_stream(arb_oracle_version().prop_map(|t| from_oracle_version(&t).encode())),
    ) {
        let mut subject_reader: &[u8] = &stream;
        let subject = Version::deserialize_reader(&mut subject_reader);
        let mut oracle_reader: &[u8] = &stream;
        let oracle = reference_version(&mut oracle_reader).map_err(decode_error);
        prop_assert_eq!(
            subject_reader.len(),
            oracle_reader.len(),
            "byte consumption diverged",
        );
        match (subject, oracle) {
            (Ok(s), Ok(o)) => prop_assert_eq!(s, o),
            (Err(s), Err(o)) => assert_same_error(&s, &o)?,
            (s, o) => prop_assert!(false, "accept/reject diverged: {:?} vs {:?}", s, o),
        }
    }
}

proptest! {
    /// The windowed wire cursor decodes a `Party` stream exactly like the
    /// per-bit reference: same value or same error, same bytes consumed.
    ///
    /// The id parse never reads integers, so this pins the byte-indexed
    /// `read_bit` and its refill discipline in isolation from the window.
    #[test]
    fn party_wire_decode_matches_bitwise_reference(
        stream in arb_stream(
            arb_oracle_party_nonempty().prop_map(|t| from_oracle_party(&t).encode()),
        ),
    ) {
        let mut subject_reader: &[u8] = &stream;
        let subject = Party::deserialize_reader(&mut subject_reader);
        let mut oracle_reader: &[u8] = &stream;
        let oracle = reference_party(&mut oracle_reader).map_err(decode_error);
        prop_assert_eq!(
            subject_reader.len(),
            oracle_reader.len(),
            "byte consumption diverged",
        );
        match (subject, oracle) {
            (Ok(s), Ok(o)) => prop_assert_eq!(s, o),
            (Err(s), Err(o)) => assert_same_error(&s, &o)?,
            (s, o) => prop_assert!(false, "accept/reject diverged: {:?} vs {:?}", s, o),
        }
    }
}

/// Decode one clock — an id tree, then an event tree — per-bit through
/// [`BitwiseReaderCursor`], replicating the wire door's composition: the
/// version decode starts exactly where the party decode stopped.
fn reference_clock<R: Read>(reader: &mut R) -> Result<Clock, Decode> {
    let party = reference_party(reader)?;
    let version = reference_version(reader)?;
    Ok(Clock::from_parts(party, version))
}

/// Decode one composite key — a self-delimiting rank stream, then an
/// event tree — with the version leg per-bit through
/// [`BitwiseReaderCursor`] and the wire door's own cross-check after it.
///
/// The rank stream has exactly one parser, shared with the wire door
/// byte for byte, so the surface this reference pins differentially is
/// the version stream (window, refills, padding) and the composite
/// cross-check around it.
fn reference_ranked<R: Read>(reader: &mut R) -> Result<Ranked<'static>, Decode> {
    let rank = decode_rank_stream(|| {
        let mut byte = [0];
        reader.read_exact(&mut byte).map_err(Decode::Io)?;
        Ok(byte[0])
    })?;
    let version = reference_version(reader)?;
    if version.rank() != rank {
        return Err(Decode::NotCanonical);
    }
    Ok(Ranked::from(version))
}

/// Decode one span composite per-bit through [`BitwiseReaderCursor`],
/// replicating the wire pipeline stage for stage.
///
/// The stages, in the wire door's order: the meet's tree and padding, the
/// fused admission walk over the join, the join's padding consumption —
/// which outranks the pair verdict — then the verdict.
fn reference_span<R: Read>(reader: &mut R) -> Result<Span<'static>, Decode> {
    let lo = reference_version(reader)?;
    let mut cursor = BitwiseReaderCursor {
        reader,
        bits: BitsBuf::new(),
        position: 0,
    };
    let admission = validate_dominating_from((lo.view()).live(), &mut cursor)?;
    let position = cursor.position;
    reference_consume_padding(&mut cursor)?;
    let mut bits = cursor.bits;
    bits.truncate(position);
    let hi = match admission {
        Admission::Refuted => return Err(Decode::NotCanonical),
        Admission::Equal => lo.clone(),
        Admission::Dominates => Version::from_bits(bits),
    };
    Ok(Span::owned(lo, hi))
}

proptest! {
    /// The windowed wire cursor decodes a `Clock` stream — an id tree,
    /// then an event tree — exactly like the per-bit reference: same
    /// value or same error, same bytes consumed.
    ///
    /// The clock door is the two component doors composed, so beyond the
    /// components' own differentials this pins the hand-off between them
    /// on accepts and rejects alike. Streams cover canonical clock
    /// encodings, bit flips, truncations, raw noise, and trailing junk.
    #[test]
    fn clock_wire_decode_matches_bitwise_reference(
        stream in arb_stream(
            (arb_oracle_party_nonempty(), arb_oracle_version()).prop_map(|(p, v)| {
                [
                    from_oracle_party(&p).encode(),
                    from_oracle_version(&v).encode(),
                ]
                .concat()
            }),
        ),
    ) {
        let mut subject_reader: &[u8] = &stream;
        let subject = Clock::deserialize_reader(&mut subject_reader);
        let mut oracle_reader: &[u8] = &stream;
        let oracle = reference_clock(&mut oracle_reader).map_err(decode_error);
        prop_assert_eq!(
            subject_reader.len(),
            oracle_reader.len(),
            "byte consumption diverged",
        );
        match (subject, oracle) {
            (Ok(s), Ok(o)) => prop_assert_eq!(s, o),
            (Err(s), Err(o)) => assert_same_error(&s, &o)?,
            (s, o) => prop_assert!(false, "accept/reject diverged: {:?} vs {:?}", s, o),
        }
    }
}

proptest! {
    /// The windowed wire cursor decodes a `Ranked` composite key exactly
    /// like the per-bit reference: same value or same error, same bytes
    /// consumed.
    ///
    /// The rank stream has exactly one parser, shared by both sides, so
    /// the differential surface is the version leg — window, refills, and
    /// the padding genre — and the mismatched-pair cross-check around it,
    /// which bit flips in the rank prefix breed. Streams cover canonical
    /// composite keys, bit flips, truncations, raw noise, and trailing
    /// junk.
    #[test]
    fn ranked_wire_decode_matches_bitwise_reference(
        stream in arb_stream(
            arb_oracle_version().prop_map(|t| from_oracle_version(&t).ranked().encode()),
        ),
    ) {
        let mut subject_reader: &[u8] = &stream;
        let subject = Ranked::deserialize_reader(&mut subject_reader);
        let mut oracle_reader: &[u8] = &stream;
        let oracle = reference_ranked(&mut oracle_reader).map_err(decode_error);
        prop_assert_eq!(
            subject_reader.len(),
            oracle_reader.len(),
            "byte consumption diverged",
        );
        match (subject, oracle) {
            (Ok(s), Ok(o)) => prop_assert_eq!(s, o),
            (Err(s), Err(o)) => assert_same_error(&s, &o)?,
            (s, o) => prop_assert!(false, "accept/reject diverged: {:?} vs {:?}", s, o),
        }
    }
}

proptest! {
    /// The borsh rank door reads exactly one self-delimiting rank stream
    /// and agrees with the whole-slice decode on the exact error variant:
    /// no genre may shift between the two doors.
    ///
    /// Reader starvation surfaces as the reader's own `UnexpectedEof`
    /// exactly where the slice door reports [`Decode::Truncated`]; every
    /// other rejection crosses as `InvalidData` carrying the identical
    /// [`Decode`] variant. On accepts, the consumed prefix is the value's
    /// canonical encoding, and a nonempty remainder is exactly what makes
    /// the whole-slice decode reject as [`Decode::TrailingBits`]. Streams
    /// cover version-derived and deep-fraction ranks, bit flips,
    /// truncations, raw noise, and trailing junk.
    #[test]
    fn rank_wire_decode_matches_slice_reference(
        stream in arb_stream(prop_oneof![
            arb_oracle_version().prop_map(|t| from_oracle_version(&t).rank().encode()),
            (any::<u128>(), 0u64..64).prop_map(|(num, exp)| {
                Rank::from_raw(codec::Base::from(num), exp).encode()
            }),
        ]),
    ) {
        let mut reader: &[u8] = &stream;
        match Rank::deserialize_reader(&mut reader) {
            Ok(rank) => {
                let consumed = &stream[..stream.len() - reader.len()];
                let reencoded = rank.encode();
                prop_assert_eq!(
                    reencoded.as_slice(),
                    consumed,
                    "accepted rank re-encodes to the consumed prefix",
                );
                prop_assert_eq!(
                    &Rank::decode(consumed).expect("the slice door accepts the same bytes"),
                    &rank,
                );
                if !reader.is_empty() {
                    let whole = Rank::decode(&stream[..])
                        .expect_err("bytes remain past the rank: the whole slice must reject");
                    prop_assert!(
                        matches!(whole, Decode::TrailingBits),
                        "input past a complete rank is the trailing genre: {:?}",
                        whole,
                    );
                }
            }
            Err(err) => {
                let raw = Rank::decode(&stream[..])
                    .expect_err("the borsh door rejects: the slice door must reject");
                let variant = err
                    .get_ref()
                    .and_then(|inner| inner.downcast_ref::<Decode>())
                    .map(std::mem::discriminant);
                match raw {
                    Decode::Truncated => {
                        prop_assert_eq!(err.kind(), ErrorKind::UnexpectedEof);
                        prop_assert_eq!(variant, None);
                    }
                    raw => {
                        prop_assert_eq!(err.kind(), ErrorKind::InvalidData);
                        prop_assert_eq!(variant, Some(std::mem::discriminant(&raw)));
                    }
                }
            }
        }
    }
}

proptest! {
    /// The windowed wire cursor decodes a `Span` composite exactly like
    /// the per-bit reference: same value or same error, same bytes
    /// consumed.
    ///
    /// The agreement is total — the admission verdict, the padding check
    /// that outranks it, and every structural genre included. Streams cover canonical span composites and bare meet prefixes,
    /// each optionally bit-flipped, truncated, and tailed with junk, plus
    /// raw noise: the junk-tailed meet prefixes drive the admission walk
    /// over arbitrary join streams (height dips, collapsible siblings,
    /// dirty padding), pinning the wire cursor's error plumbing under the
    /// fused walk. Accepted spans additionally re-encode to the consumed
    /// prefix and decode identically through the byte-slice door.
    #[test]
    fn span_wire_decode_matches_bitwise_reference(
        stream in arb_stream(prop_oneof![
            (arb_oracle_version(), arb_oracle_version()).prop_map(|(a, b)| {
                from_oracle_version(&a).span(&from_oracle_version(&b)).encode()
            }),
            arb_oracle_version().prop_map(|t| from_oracle_version(&t).encode()),
        ]),
    ) {
        let mut subject_reader: &[u8] = &stream;
        let subject = Span::deserialize_reader(&mut subject_reader);
        let mut oracle_reader: &[u8] = &stream;
        let oracle = reference_span(&mut oracle_reader).map_err(decode_error);
        prop_assert_eq!(
            subject_reader.len(),
            oracle_reader.len(),
            "byte consumption diverged",
        );
        match (subject, oracle) {
            (Ok(s), Ok(o)) => {
                prop_assert_eq!(&s, &o);
                let consumed = &stream[..stream.len() - subject_reader.len()];
                let reencoded = s.encode();
                prop_assert_eq!(
                    reencoded.as_slice(),
                    consumed,
                    "accepted span re-encodes to the consumed prefix",
                );
                prop_assert_eq!(
                    Span::decode(consumed).expect("the slice door accepts the same bytes"),
                    s,
                );
            }
            (Err(s), Err(o)) => assert_same_error(&s, &o)?,
            (s, o) => prop_assert!(false, "accept/reject diverged: {:?} vs {:?}", s, o),
        }
    }
}

proptest! {
    /// A canonical encoding embedded in a longer borsh stream decodes to its
    /// value while consuming exactly the encoding's bytes.
    ///
    /// The encodings are prefix-free with no length prefix, so the decoder
    /// must find its own end: the trailing bytes — the next borsh fields —
    /// stay in the reader untouched, for every `Party`/`Version`/`Clock` an
    /// impl-driven history reaches. A `Clock` doubles as the composition
    /// witness: its `Version` decode starts wherever its `Party` decode
    /// stopped.
    #[test]
    fn embedded_decode_consumes_exactly_the_encoding(
        ops in world_strategy(),
        trailing in proptest::collection::vec(any::<u8>(), 0..16),
    ) {
        let mut imp = vec![Clock::seed()];
        for op in &ops {
            step_impl(&mut imp, op);
        }
        for c in &imp {
            let (p, v) = (c.party(), c.version());

            let mut stream = borsh::to_vec(p).unwrap();
            stream.extend_from_slice(&trailing);
            let mut reader: &[u8] = &stream;
            prop_assert_eq!(&Party::deserialize_reader(&mut reader).unwrap(), p);
            prop_assert_eq!(reader, &trailing[..]);

            let mut stream = borsh::to_vec(v).unwrap();
            stream.extend_from_slice(&trailing);
            let mut reader: &[u8] = &stream;
            prop_assert_eq!(&Version::deserialize_reader(&mut reader).unwrap(), v);
            prop_assert_eq!(reader, &trailing[..]);

            let mut stream = borsh::to_vec(c).unwrap();
            stream.extend_from_slice(&trailing);
            let mut reader: &[u8] = &stream;
            let back = Clock::deserialize_reader(&mut reader).unwrap();
            prop_assert_eq!(back.party(), p);
            prop_assert_eq!(back.version(), v);
            prop_assert_eq!(reader, &trailing[..]);
        }
    }
}

/// Trees far deeper than any real id or event tree round-trip on the wire
/// path with the parse frames grown on the heap.
///
/// A deep version and a deep party round-trip through borsh with trailing
/// bytes intact: growing the frame stack mid-parse must disturb neither
/// the normal-form checks the frames carry nor the cursor's byte
/// consumption.
#[test]
fn deep_trees_roundtrip_through_borsh() {
    // Deep enough that the frame stack regrows several times mid-parse
    // (`codec::tests::parse_stacks_handle_deep_spines` is the direct pin).
    const DEPTH: usize = 48;
    const TRAILING: &[u8] = &[0xA5, 0x5A, 0xFF];

    // A right-spine event tree: every node has a base-0 left leaf, and the
    // innermost pair of leaves differ, so the whole spine is canonical.
    let mut spine = String::from("2");
    for _ in 0..DEPTH {
        spine = format!("(1, 0, {spine})");
    }
    let version: Version = spine.parse().expect("a deep right spine is canonical");
    let mut stream = borsh::to_vec(&version).unwrap();
    stream.extend_from_slice(TRAILING);
    let mut reader: &[u8] = &stream;
    assert_eq!(
        Version::deserialize_reader(&mut reader).expect("deep version decodes"),
        version,
    );
    assert_eq!(reader, TRAILING, "trailing bytes stay for the next field");

    let party = deep_left_spine_party(DEPTH);
    let mut stream = borsh::to_vec(&party).unwrap();
    stream.extend_from_slice(TRAILING);
    let mut reader: &[u8] = &stream;
    assert_eq!(
        Party::deserialize_reader(&mut reader).expect("deep party decodes"),
        party,
    );
    assert_eq!(reader, TRAILING, "trailing bytes stay for the next field");
}

// ─────────────────────────────── rank ───────────────────────────────

/// A [`Rank`]'s borsh bytes are exactly its canonical encoding — no
/// length prefix, no second format — and the [`Ranked`] view
/// serializes its own composite key; both round-trip.
///
/// Zero, an integral-only rank, and a deep small fraction cover the
/// stream's three shapes (empty fraction in one byte, empty fraction
/// spilling its close bit, multi-group fraction).
#[test]
fn rank_borsh_is_the_canonical_encoding() {
    let battery = [
        crate::Rank::ZERO,
        Version::try_from(7).unwrap().rank(),
        crate::version::Rank::from_raw(crate::codec::Base::from(1u8), 40),
    ];
    for rank in &battery {
        let bytes = borsh::to_vec(rank).unwrap();
        assert_eq!(bytes, rank.encode(), "raw framing: the one wire form");
        assert_eq!(&crate::Rank::try_from_slice(&bytes).unwrap(), rank);
    }
    let v = Version::try_from(7).unwrap();
    let view_bytes = borsh::to_vec(&v.ranked()).unwrap();
    assert_eq!(
        view_bytes,
        v.ranked().encode(),
        "the view serializes its composite key: raw framing, one wire form"
    );
    assert_eq!(
        crate::Ranked::try_from_slice(&view_bytes)
            .unwrap()
            .version(),
        &v,
        "the composite round-trips to the same version"
    );
}

/// A [`Ranked`] composite key composes inside a larger borsh stream,
/// and its rejection genres cross the borsh boundary intact.
///
/// A `(Ranked, Party, Rank)` concatenation deserializes field by
/// field, each read consuming exactly its own bytes.
///
/// A cut anywhere mid-composite surfaces the reader's own
/// `UnexpectedEof`; a mismatched rank prefix crosses as `InvalidData`
/// carrying [`Decode::NotCanonical`] (the redundancy check
/// `Ranked::decode` documents).
#[test]
fn ranked_borsh_composes_and_keeps_its_genres() {
    let half: Version = "(0, 1, 0)".parse().unwrap();
    let mut party = Party::seed();
    let _ = party.fork();
    let rank = Version::try_from(5).unwrap().rank();
    let mut stream = Vec::new();
    borsh::BorshSerialize::serialize(&half.ranked(), &mut stream).unwrap();
    borsh::BorshSerialize::serialize(&party, &mut stream).unwrap();
    borsh::BorshSerialize::serialize(&rank, &mut stream).unwrap();
    let mut reader: &[u8] = &stream;
    assert_eq!(
        crate::Ranked::deserialize_reader(&mut reader)
            .unwrap()
            .version(),
        &half,
    );
    assert_eq!(Party::deserialize_reader(&mut reader).unwrap(), party);
    assert_eq!(crate::Rank::deserialize_reader(&mut reader).unwrap(), rank);
    assert!(reader.is_empty(), "every byte belonged to some field");

    let key = borsh::to_vec(&half.ranked()).unwrap();
    for cut in 0..key.len() {
        let err = crate::Ranked::try_from_slice(&key[..cut]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::UnexpectedEof, "cut at byte {cut}");
    }
    let forged = [
        Version::try_from(6).unwrap().rank().encode(),
        half.as_bytes().to_vec(),
    ]
    .concat();
    let err = crate::Ranked::try_from_slice(&forged).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::InvalidData);
    let inner = err
        .get_ref()
        .expect("the io error carries the Decode error");
    assert!(
        matches!(inner.downcast_ref::<Decode>(), Some(Decode::NotCanonical)),
        "expected NotCanonical, got: {inner:?}"
    );
}

/// A rank stream is self-delimiting inside a larger borsh stream: a
/// `(Rank, Version, Rank)` concatenation deserializes field by field,
/// each read consuming exactly its own bytes.
#[test]
fn rank_composes_in_a_borsh_stream() {
    let a = crate::version::Rank::from_raw(crate::codec::Base::from(129u8), 8);
    let v: Version = "(0, 1, 0)".parse().unwrap();
    let b = Version::try_from(5).unwrap().rank();
    let mut stream = Vec::new();
    borsh::BorshSerialize::serialize(&a, &mut stream).unwrap();
    borsh::BorshSerialize::serialize(&v, &mut stream).unwrap();
    borsh::BorshSerialize::serialize(&b, &mut stream).unwrap();
    let mut reader: &[u8] = &stream;
    assert_eq!(crate::Rank::deserialize_reader(&mut reader).unwrap(), a);
    assert_eq!(Version::deserialize_reader(&mut reader).unwrap(), v);
    assert_eq!(crate::Rank::deserialize_reader(&mut reader).unwrap(), b);
    assert!(reader.is_empty(), "every byte belonged to some field");
}

/// A borsh rank stream cut anywhere mid-stream surfaces the reader's
/// own `UnexpectedEof`, and a set padding bit crosses the boundary as
/// `InvalidData` carrying the exact [`Decode`] variant.
#[test]
fn rank_borsh_rejections_keep_their_genres() {
    let deep = crate::version::Rank::from_raw(crate::codec::Base::from(5u128 << 40 | 1), 40);
    let bytes = borsh::to_vec(&deep).unwrap();
    assert!(bytes.len() >= 7, "the fraction spans several groups");
    for cut in 0..bytes.len() {
        let err = crate::Rank::try_from_slice(&bytes[..cut]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::UnexpectedEof, "cut at byte {cut}");
    }
    // The encoding of 1 ("10000") with a set bit in its padding.
    let err = crate::Rank::try_from_slice(&[0x84]).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::InvalidData);
    let inner = err
        .get_ref()
        .expect("the io error carries the Decode error");
    assert!(
        matches!(inner.downcast_ref::<Decode>(), Some(Decode::TrailingBits)),
        "expected TrailingBits, got: {inner:?}"
    );
}

proptest! {
    /// Version-derived ranks round-trip through borsh, and — the raw
    /// framing's inheritance — byte-wise order on the serialized
    /// bytes is rank order, the lexicographic law surviving the
    /// transport unchanged.
    #[test]
    fn rank_borsh_roundtrips_and_orders(oa in arb_oracle_version(), ob in arb_oracle_version()) {
        let a = from_oracle_version(&oa).rank();
        let b = from_oracle_version(&ob).rank();
        let (sa, sb) = (borsh::to_vec(&a).unwrap(), borsh::to_vec(&b).unwrap());
        prop_assert_eq!(&crate::Rank::try_from_slice(&sa).unwrap(), &a);
        prop_assert_eq!(&crate::Rank::try_from_slice(&sb).unwrap(), &b);
        prop_assert_eq!(sa.cmp(&sb), a.cmp(&b), "serialized order is rank order");
    }
}

proptest! {
    /// [`Ranked`] composite keys round-trip through borsh, and byte
    /// order on the serialized bytes is the views' total order.
    ///
    /// The raw framing's inheritance, ties included: the causal
    /// ordering (and its deterministic tiebreak) survives the
    /// transport unchanged.
    #[test]
    fn ranked_borsh_roundtrips_and_orders(oa in arb_oracle_version(), ob in arb_oracle_version()) {
        let a = from_oracle_version(&oa);
        let b = from_oracle_version(&ob);
        let (ra, rb) = (crate::Ranked::from(&a), crate::Ranked::from(&b));
        let (sa, sb) = (borsh::to_vec(&ra).unwrap(), borsh::to_vec(&rb).unwrap());
        let back_a = crate::Ranked::try_from_slice(&sa).unwrap();
        let back_b = crate::Ranked::try_from_slice(&sb).unwrap();
        prop_assert!(back_a == ra && *back_a.version() == a);
        prop_assert!(back_b == rb && *back_b.version() == b);
        prop_assert_eq!(sa.cmp(&sb), ra.cmp(&rb), "serialized order is the total order");
        prop_assert_eq!(sa == sb, a == b, "byte equality is version identity");
    }
}

/// A [`Span`](crate::Span) composite composes inside a
/// larger borsh stream, and its rejection genres cross the borsh
/// boundary intact.
///
/// A `(Span, Party, Rank)` concatenation deserializes field by field,
/// each read consuming exactly its own bytes. A cut anywhere
/// mid-composite surfaces the reader's own `UnexpectedEof`; a crossed
/// composite crosses as `InvalidData` carrying
/// [`Decode::NotCanonical`] (the fused pair validation
/// `Span::decode` documents).
#[test]
fn span_borsh_composes_and_keeps_its_genres() {
    let mut clock = Clock::seed();
    let older = clock.tick().clone();
    let newer = clock.tick().clone();
    let span = Span::new(&older, &newer).unwrap();
    let mut party = Party::seed();
    let _ = party.fork();
    let rank = Version::try_from(5).unwrap().rank();
    let mut stream = Vec::new();
    borsh::BorshSerialize::serialize(&span, &mut stream).unwrap();
    borsh::BorshSerialize::serialize(&party, &mut stream).unwrap();
    borsh::BorshSerialize::serialize(&rank, &mut stream).unwrap();
    let mut reader: &[u8] = &stream;
    assert_eq!(Span::deserialize_reader(&mut reader).unwrap(), span);
    assert_eq!(Party::deserialize_reader(&mut reader).unwrap(), party);
    assert_eq!(crate::Rank::deserialize_reader(&mut reader).unwrap(), rank);
    assert!(reader.is_empty(), "every byte belonged to some field");

    let composite = borsh::to_vec(&span).unwrap();
    assert_eq!(
        composite,
        span.encode(),
        "borsh is the one wire form, unframed"
    );
    for cut in 0..composite.len() {
        let err = Span::try_from_slice(&composite[..cut]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::UnexpectedEof, "cut at byte {cut}");
    }
    let crossed = [newer.encode(), older.encode()].concat();
    let err = Span::try_from_slice(&crossed).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::InvalidData);
    let inner = err
        .get_ref()
        .expect("the io error carries the Decode error");
    assert!(
        matches!(inner.downcast_ref::<Decode>(), Some(Decode::NotCanonical)),
        "expected NotCanonical, got: {inner:?}"
    );

    // A crossed composite whose join also carries a set padding bit:
    // the structural genre crosses the borsh boundary ahead of the
    // pair verdict, exactly as the raw decode orders them.
    let mut crossed_padding = crossed.clone();
    assert_ne!(
        older.encoded_bits() % 8,
        0,
        "the join witness ends mid-byte"
    );
    *crossed_padding.last_mut().unwrap() |= 0x01;
    let err = Span::try_from_slice(&crossed_padding).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::InvalidData);
    let inner = err
        .get_ref()
        .expect("the io error carries the Decode error");
    assert!(
        matches!(inner.downcast_ref::<Decode>(), Some(Decode::TrailingBits)),
        "expected TrailingBits, got: {inner:?}"
    );
}

proptest! {
    /// [`Span`](crate::Span) composites round-trip through
    /// borsh: the raw framing carries exactly `Span::encode`'s bytes,
    /// and the fused wire validation accepts every hull the borrowing
    /// constructor admits.
    #[test]
    fn span_borsh_roundtrips(oa in arb_oracle_version(), ob in arb_oracle_version()) {
            let a = from_oracle_version(&oa);
        let b = from_oracle_version(&ob);
        let span = a.span(&b);
        let bytes = borsh::to_vec(&span).unwrap();
        prop_assert_eq!(&bytes, &span.encode(), "borsh is the one wire form, unframed");
        prop_assert_eq!(Span::try_from_slice(&bytes).unwrap(), span);
    }
}

/// Every ordered pair of the six wire types composes adjacently in one
/// borsh stream with exact parse boundaries.
///
/// Self-delimitation totality across types: for each of the 36 ordered
/// pairs `(A, B)`, serializing `a` then `b` into one stream and reading
/// them back field by field consumes exactly the first encoding's bytes
/// before the second begins, leaves nothing after the second, and each
/// recovered value re-serializes to exactly its own segment — so no
/// type's decoder can over- or under-read into a neighbor of any type.
#[test]
fn borsh_every_type_pair_composes_with_exact_boundaries() {
    // Multi-byte, structure-bearing values of each type.
    let mut party = Party::seed();
    for _ in 0..4 {
        let _ = party.fork(); // a several-byte id tree
    }
    let version: Version = "(1, 2, (0, (1, 0, 2), 0))".parse().unwrap();
    let clock = {
        let mut c = Clock::seed();
        for _ in 0..5 {
            c.tick();
        }
        let _ = c.fork(); // a non-seed party beside a non-empty version
        c
    };
    let rank = crate::version::Rank::from_raw(crate::codec::Base::from(129u8), 8);
    let ranked: Ranked<'static> = Ranked::from(version.clone());
    let span: Span<'static> = {
        let mut c = Clock::seed();
        let older = c.tick().clone();
        let newer = c.tick().clone();
        Span::new(&older, &newer).unwrap().into_owned()
    };

    // One ordered pair: serialize a then b, read them back, and hold
    // both boundaries exact (values compared through their canonical
    // re-serialization, which the round-trip pins elsewhere).
    macro_rules! pair {
        ($a:expr, $ta:ty, $b:expr, $tb:ty) => {{
            let mut stream = Vec::new();
            borsh::BorshSerialize::serialize(&$a, &mut stream).unwrap();
            let first_len = stream.len();
            borsh::BorshSerialize::serialize(&$b, &mut stream).unwrap();
            let mut reader: &[u8] = &stream;
            let got_a = <$ta>::deserialize_reader(&mut reader).unwrap_or_else(|e| {
                panic!(
                    "first field ({}) must decode ahead of {}: {e}",
                    stringify!($ta),
                    stringify!($tb),
                )
            });
            assert_eq!(
                stream.len() - reader.len(),
                first_len,
                "{} consumed past (or short of) its own encoding ahead of {}",
                stringify!($ta),
                stringify!($tb),
            );
            let got_b = <$tb>::deserialize_reader(&mut reader).unwrap_or_else(|e| {
                panic!(
                    "second field ({}) must decode after {}: {e}",
                    stringify!($tb),
                    stringify!($ta),
                )
            });
            assert!(
                reader.is_empty(),
                "{} after {} left bytes unconsumed",
                stringify!($tb),
                stringify!($ta),
            );
            assert_eq!(
                borsh::to_vec(&got_a).unwrap(),
                &stream[..first_len],
                "first value re-serializes to its own segment",
            );
            assert_eq!(
                borsh::to_vec(&got_b).unwrap(),
                &stream[first_len..],
                "second value re-serializes to its own segment",
            );
        }};
    }
    // The full 6x6 ordered-pair matrix.
    macro_rules! pairs_from {
        ($a:expr, $ta:ty) => {
            pair!($a, $ta, party, Party);
            pair!($a, $ta, version, Version);
            pair!($a, $ta, clock, Clock);
            pair!($a, $ta, rank, Rank);
            pair!($a, $ta, ranked, Ranked<'static>);
            pair!($a, $ta, span, Span<'static>);
        };
    }
    pairs_from!(party, Party);
    pairs_from!(version, Version);
    pairs_from!(clock, Clock);
    pairs_from!(rank, Rank);
    pairs_from!(ranked, Ranked<'static>);
    pairs_from!(span, Span<'static>);
}

/// A defect inside element `N` of a borsh sequence keeps its genre and
/// leaves the elements before it intact.
///
/// A `Vec<Version>` is a length prefix and then each element's
/// self-delimiting encoding: a set padding bit inside the middle
/// element's final byte crosses the boundary as `InvalidData` carrying
/// [`Decode::TrailingBits`], and a stream cut inside the middle element
/// surfaces as the reader's own `UnexpectedEof` — the sequence context
/// neither masks the genre nor shifts the defect's attribution, and the
/// same corrupted stream still yields element 0 when read field by
/// field.
#[test]
fn borsh_sequence_defect_in_element_n_keeps_its_genre() {
    let elems: Vec<Version> = ["(1, 2, (0, (1, 0, 2), 0))", "(1, 0, 4)", "(2, 0, 5)"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();
    // The middle element must end mid-byte so a padding bit exists to set.
    assert_ne!(elems[1].encoded_bits() % 8, 0, "the witness ends mid-byte");
    let bytes = borsh::to_vec(&elems).unwrap();
    let e0 = borsh::to_vec(&elems[0]).unwrap().len();
    let e1 = borsh::to_vec(&elems[1]).unwrap().len();
    let mid_last = 4 + e0 + e1 - 1; // length prefix, element 0, element 1's final byte

    // Set a padding bit inside element 1's final byte.
    let mut corrupted = bytes.clone();
    corrupted[mid_last] |= 0x01;
    let err = <Vec<Version>>::try_from_slice(&corrupted).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::InvalidData);
    let inner = err
        .get_ref()
        .expect("the io error carries the Decode error");
    assert!(
        matches!(inner.downcast_ref::<Decode>(), Some(Decode::TrailingBits)),
        "expected TrailingBits from element 1, got: {inner:?}"
    );
    // Element 0 still reads intact off the corrupted stream.
    let mut reader: &[u8] = &corrupted[4..];
    assert_eq!(
        Version::deserialize_reader(&mut reader).unwrap(),
        elems[0],
        "the defect in element 1 does not reach element 0"
    );

    // Cut the stream inside element 1: the reader's own truncation genre.
    let err = <Vec<Version>>::try_from_slice(&bytes[..4 + e0 + e1 / 2]).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::UnexpectedEof);
}

/// The borsh span door dedups the coincident span's storage exactly as
/// the byte-slice decode does.
///
/// The fused admission verdict detects `hi == lo` on the wire, so the
/// deserialized endpoints share one buffer (clone identity holds) and
/// the composite still re-serializes byte-identically.
#[test]
fn span_borsh_dedups_the_coincident_span() {
    let mut clock = crate::Clock::seed();
    for _ in 0..12 {
        clock.tick();
    }
    let v = clock.version().clone();
    let span = v.span(&v);
    let bytes = borsh::to_vec(&span).unwrap();
    let decoded = Span::try_from_slice(&bytes).unwrap();
    assert_eq!(decoded, span, "the wire round-trips the coincident span");
    assert_eq!(borsh::to_vec(&decoded).unwrap(), bytes);
    assert!(
        decoded.lo().view().ptr_eq(decoded.hi().view()),
        "the borsh admission verdict must dedup the coincident span's storage"
    );
}

/// A coincident span inside a larger borsh stream consumes exactly its
/// own bytes: the Equal admission arm keeps the container framing.
///
/// [`span_borsh_composes_and_keeps_its_genres`] drives a *dominating*
/// span ahead of its neighbors; the Equal arm is the one admission
/// verdict it never takes in a container, and that arm alone drops the
/// parsed join bits and stores the meet's clone instead — the seam
/// where an EOF-tolerant over-read in consumed bytes would silently
/// corrupt every following field while every lone-value test (which
/// sees a fully drained buffer either way) stays green. The witness: a
/// coincident span, then a forked party, then a rank, in one unframed
/// stream; each field decodes to its value, the reader drains exactly,
/// and the composite re-serializes byte-identically.
#[test]
fn coincident_span_keeps_borsh_container_framing() {
    let mut clock = Clock::seed();
    for _ in 0..9 {
        clock.tick();
    }
    let v = clock.version().clone();
    let span = v.span(&v);
    assert_eq!(span.lo(), span.hi(), "the hull of (v, v) is coincident");
    let mut party = Party::seed();
    let _ = party.fork();
    let rank = v.rank();
    let mut stream = Vec::new();
    BorshSerialize::serialize(&span, &mut stream).expect("span serializes");
    BorshSerialize::serialize(&party, &mut stream).expect("party serializes");
    BorshSerialize::serialize(&rank, &mut stream).expect("rank serializes");
    let mut reader: &[u8] = &stream;
    let decoded =
        Span::deserialize_reader(&mut reader).expect("the coincident span decodes in place");
    assert_eq!(decoded, span);
    assert_eq!(
        Party::deserialize_reader(&mut reader).expect("the party field survives"),
        party,
    );
    assert_eq!(
        Rank::deserialize_reader(&mut reader).expect("the rank field survives"),
        rank,
    );
    assert!(reader.is_empty(), "every byte belonged to some field");
    let mut again = Vec::new();
    BorshSerialize::serialize(&decoded, &mut again).expect("re-serializes");
    BorshSerialize::serialize(&party, &mut again).expect("re-serializes");
    BorshSerialize::serialize(&rank, &mut again).expect("re-serializes");
    assert_eq!(
        again, stream,
        "the composite re-serializes byte-identically"
    );
}

/// The coincident span's padding check survives the Equal admission
/// arm: a set padding bit in the join component's final byte rejects as
/// `TrailingBits` through the borsh door.
///
/// The Equal arm drops the parsed join bits and stores the meet's
/// clone, so a rewrite that settled the verdict before the cursor's
/// final-byte padding check would accept a dirty join stream and
/// re-encode it clean — invisible to every round-trip assert. The
/// padding check outranks the pair verdict at this door exactly as the
/// byte-slice decode orders them.
#[test]
fn coincident_span_borsh_rejects_tampered_join_padding() {
    let v: Version = "(1, 0, 4)".parse().unwrap();
    assert_ne!(
        v.encoded_bits() % 8,
        0,
        "the witness needs a mid-byte tail so a padding bit exists"
    );
    let span = v.span(&v);
    let mut bytes = borsh::to_vec(&span).unwrap();
    let last = bytes.len() - 1;
    bytes[last] |= 0x01; // a padding bit inside the join's final byte
    let err = <Span as BorshDeserialize>::try_from_slice(&bytes).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::InvalidData);
    let inner = err
        .get_ref()
        .expect("the io error carries the Decode error");
    assert!(
        matches!(inner.downcast_ref::<Decode>(), Some(Decode::TrailingBits)),
        "expected TrailingBits from the tampered join padding, got: {inner:?}"
    );
}

/// A structurally whole join whose running height dips negative rejects
/// through the borsh span door as `InvalidData` carrying
/// [`Decode::NotCanonical`]: the refuted verdict subsumes the dip on the
/// wire path too.
///
/// The bytes are the fuzz seed set's `span_negative_join` witness: a
/// canonical empty meet, then a join whose height dips negative — root
/// internal `0`, left leaf height gamma(0), right leaf delta zigzag(-1),
/// padding marker. No encode produces it, so only a constructed stream
/// reaches the verdict's rejection arm under a `ReaderCursor`; it rides
/// beside [`span_wire_decode_matches_bitwise_reference`] as the
/// deterministic tripwire for that arm's genre.
#[test]
fn span_borsh_rejects_negative_height_join() {
    let bytes = [Version::new().encode(), vec![0x75]].concat();
    assert!(
        matches!(Span::decode(&bytes[..]), Err(Decode::NotCanonical)),
        "the byte-slice door rejects the dip as NotCanonical"
    );
    let err = <Span as BorshDeserialize>::try_from_slice(&bytes).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::InvalidData);
    let inner = err
        .get_ref()
        .expect("the io error carries the Decode error");
    assert!(
        matches!(inner.downcast_ref::<Decode>(), Some(Decode::NotCanonical)),
        "expected NotCanonical from the negative-height join, got: {inner:?}"
    );
}

/// A join carrying a collapsible sibling pair rejects through the borsh
/// span door as `InvalidData` carrying [`Decode::NotCanonical`]: the
/// close-out canonicality check fires under a `ReaderCursor` too.
///
/// The bytes are a canonical empty meet, then an internal node whose two
/// leaf children carry height 0 and delta 0 — a sibling pair normal form
/// forbids. It rides beside
/// [`span_wire_decode_matches_bitwise_reference`] as the deterministic
/// tripwire for the close-out arm's genre.
#[test]
fn span_borsh_rejects_collapsible_join() {
    let bytes = [Version::new().encode(), vec![0b0111_1000]].concat();
    assert!(
        matches!(Span::decode(&bytes[..]), Err(Decode::NotCanonical)),
        "the byte-slice door rejects the collapsible join as NotCanonical"
    );
    let err = <Span as BorshDeserialize>::try_from_slice(&bytes).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::InvalidData);
    let inner = err
        .get_ref()
        .expect("the io error carries the Decode error");
    assert!(
        matches!(inner.downcast_ref::<Decode>(), Some(Decode::NotCanonical)),
        "expected NotCanonical from the collapsible join, got: {inner:?}"
    );
}
