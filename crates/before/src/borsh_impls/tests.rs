use borsh::io::{Error, ErrorKind, Read};
use borsh::BorshDeserialize;
use proptest::prelude::*;
use proptest::test_runner::TestCaseError;

use super::decode_error;
use crate::codec::{self, BitCursor, BitsMut, PARSE_STACK_INLINE};
use crate::error::Decode;
use crate::testing::bridge::{from_oracle_party, from_oracle_version};
use crate::testing::generators::{
    arb_oracle_party_nonempty, arb_oracle_version, deep_left_spine_party,
};
use crate::testing::optrace::{step_impl, world_strategy};
use crate::{Clock, Party, Version};

/// A borsh stream that ends mid-tree surfaces the reader's own I/O error
/// (`UnexpectedEof`), never a masked decode error.
///
/// `Decode::Io` is the one rich variant the bit-level error split must still
/// deliver losslessly through `ReaderCursor`: the prefix-free encodings pad
/// only to the next byte boundary, so every proper byte prefix of a canonical
/// encoding cuts mid-tree and the next per-bit refill hits end-of-input. The
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

/// Regression: a `Party` grown by [`join`](Party::join) — the operation
/// [`reclaim`](crate::bookmark) drives on a reboot — must survive the borsh
/// wire round-trip.
///
/// `fork; fork; join` reunites two quarter-regions into `(0, 1)`, normalizing
/// the build buffer's tree to a smaller one. The buffer once kept the bits it
/// shed, so [`as_bytes`](Party::as_bytes) — which borsh serializes — carried
/// trailing garbage that the peer's [`Party::decode`] rejected as
/// `TrailingBits`. On the wire that silently dropped a donated identity; here
/// it is a one-shot witness that the dead bits are now zeroed.
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
    bits: BitsMut,
    position: usize,
}

impl<R: Read> BitCursor for BitwiseReaderCursor<'_, R> {
    type Error = Decode;

    fn read_bit(&mut self) -> Result<bool, Decode> {
        if self.position == self.bits.len() {
            let mut byte = [0];
            self.reader.read_exact(&mut byte).map_err(Decode::Io)?;
            self.bits.extend_from_bitslice(codec::bytes_as_bits(&byte));
        }
        let bit = self.bits[self.position];
        self.position += 1;
        Ok(bit)
    }

    fn position(&self) -> usize {
        self.position
    }
}

/// Decode one event tree per-bit through [`BitwiseReaderCursor`].
///
/// Replicates the wire pipeline stage for stage: parse, padding check,
/// truncate to the consumed bits.
fn reference_version<R: Read>(reader: &mut R) -> Result<Version, Decode> {
    let mut cursor = BitwiseReaderCursor {
        reader,
        bits: BitsMut::new(),
        position: 0,
    };
    crate::version::skyline::validate_from(&mut cursor)?;
    codec::require_zero_padding(&cursor.bits, cursor.position)?;
    let position = cursor.position;
    let mut bits = cursor.bits;
    bits.truncate(position);
    Ok(Version::from_bits(bits))
}

/// Decode one id tree per-bit through [`BitwiseReaderCursor`].
///
/// Replicates the wire pipeline stage for stage: parse, padding check,
/// truncate to the consumed bits. The id grammar has no empty production,
/// so the parsed id is a nonzero share — exactly as
/// `Party::deserialize_reader` relies on.
fn reference_party<R: Read>(reader: &mut R) -> Result<Party, Decode> {
    let mut cursor = BitwiseReaderCursor {
        reader,
        bits: BitsMut::new(),
        position: 0,
    };
    codec::parse_id_from(&mut cursor)?;
    codec::require_zero_padding(&cursor.bits, cursor.position)?;
    let position = cursor.position;
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

/// Trees deeper than the parsers' inline stack capacity spill to the heap on
/// the wire path with behavior unchanged.
///
/// A version and a party three times [`PARSE_STACK_INLINE`] deep round-trip
/// through borsh with trailing bytes intact: the spill must disturb neither
/// the normal-form checks the frames carry nor the cursor's byte consumption.
#[test]
fn deep_trees_roundtrip_through_borsh() {
    const DEPTH: usize = 3 * PARSE_STACK_INLINE;
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

/// A [`Span`](crate::causally::Span) composite composes inside a
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
    use crate::causally::Span;
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
    /// [`Span`](crate::causally::Span) composites round-trip through
    /// borsh: the raw framing carries exactly `Span::encode`'s bytes,
    /// and the fused wire validation accepts every hull the borrowing
    /// constructor admits.
    #[test]
    fn span_borsh_roundtrips(oa in arb_oracle_version(), ob in arb_oracle_version()) {
        use crate::causally::Span;
        let a = from_oracle_version(&oa);
        let b = from_oracle_version(&ob);
        let span = a.span(&b);
        let bytes = borsh::to_vec(&span).unwrap();
        prop_assert_eq!(&bytes, &span.encode(), "borsh is the one wire form, unframed");
        prop_assert_eq!(Span::try_from_slice(&bytes).unwrap(), span);
    }
}

/// The borsh span door dedups the coincident span's storage exactly as
/// the byte-slice decode does.
///
/// The fused admission verdict detects `hi == lo` on the wire, so the
/// deserialized endpoints share one buffer (clone identity holds) and
/// the composite still re-serializes byte-identically.
#[test]
fn span_borsh_dedups_the_coincident_span() {
    use crate::causally::Span;
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
        decoded.meet().view().ptr_eq(decoded.join().view()),
        "the borsh admission verdict must dedup the coincident span's storage"
    );
}
