//! Codec round-trips, canonical uniqueness, and malformed-input rejection.
//!
//! Impl values are built from oracle trees via the bridge (canonical bits
//! emitted directly), isolating the codec from production operations.

use std::sync::Arc;

use num_bigint::BigUint;
use proptest::prelude::*;

use proptest::test_runner::TestCaseError;

use super::{bits_buf, gamma, BitCursor, BitsBuf, BitsView, DsiCursor, SliceCursor};
use crate::oracle;
use crate::span::Span;
use crate::testing::bridge::{
    from_oracle_clock, from_oracle_party, from_oracle_version, to_oracle_clock, to_oracle_party,
    to_oracle_version,
};
use crate::testing::generators::{arb_oracle_party_nonempty, arb_oracle_version};
use crate::testing::optrace::{run, versions, world_strategy};
use crate::{error::Decode, Clock, Party, Rank, Ranked, Version};

// ───────────────────────────── integer code ─────────────────────────────

proptest! {
    /// Gamma decoding inverts encoding, and the code is self-delimiting
    /// (consumes exactly the bits it wrote).
    #[test]
    fn gamma_roundtrip(n in 0u64..1_000_000) {
        let n = BigUint::from(n);
        let mut bits = BitsBuf::new();
        gamma::encode(&n, &mut bits);
        let (decoded, pos) = gamma::decode(crate::codec::built_view(&bits), 0).expect("well-formed");
        prop_assert_eq!(decoded, n);
        prop_assert_eq!(pos, bits.len());
    }
}

proptest! {
    /// The integer code round-trips arbitrary-width magnitudes with no cap: a
    /// value built from many random `u64` limbs (well past `u64::MAX`) survives
    /// gamma decoding exactly and remains self-delimiting.
    #[test]
    fn gamma_roundtrip_wide(limbs in proptest::collection::vec(any::<u64>(), 1..40)) {
        let mut n = BigUint::ZERO;
        for limb in limbs {
            n = (n << 64) | BigUint::from(limb);
        }
        let mut bits = BitsBuf::new();
        gamma::encode(&n, &mut bits);
        let (decoded, pos) = gamma::decode(crate::codec::built_view(&bits), 0).expect("well-formed");
        prop_assert_eq!(decoded, n);
        prop_assert_eq!(pos, bits.len());
    }
}

/// The integer code is Elias-gamma of `n + 1`, so its bit cost is
/// `2⌊log2(n+1)⌋ + 1`.
///
/// `0` costs a single bit, and the cost steps up by two at each power-of-two
/// boundary of `n + 1` (`1`/`2` → 3 bits, `6` → 5, `7` → 7). Pinning these
/// widths guards the canonical prefix-code property the byte-equality
/// `Eq`/`Hash` relies on.
#[test]
fn gamma_costs() {
    let cost = |n: u64| {
        let mut bits = BitsBuf::new();
        gamma::encode(&BigUint::from(n), &mut bits);
        bits.len()
    };
    assert_eq!(cost(0), 1);
    assert_eq!(cost(1), 3);
    assert_eq!(cost(2), 3);
    assert_eq!(cost(6), 5);
    assert_eq!(cost(7), 7);
}

/// The integer codec remains exact immediately beyond the machine-word range.
#[test]
fn gamma_roundtrip_just_above_u64_max() {
    let n = BigUint::from(u64::MAX) + BigUint::from(1u8);
    let mut bits = BitsBuf::new();
    gamma::encode(&n, &mut bits);
    let (decoded, pos) = gamma::decode(crate::codec::built_view(&bits), 0).expect("well-formed");
    assert_eq!(decoded, n);
    assert_eq!(decoded.to_string(), "18446744073709551616");
    assert_eq!(pos, bits.len());
}

/// Gamma decoding never panics and reports `Truncated` when the code runs off the
/// end (empty input, or all-zeros with no terminating `1`).
#[test]
fn gamma_truncated() {
    let empty = BitsBuf::new();
    assert!(matches!(
        gamma::decode(crate::codec::built_view(&empty), 0),
        Err(Decode::Truncated)
    ));
    let zeros: BitsBuf = bits_buf![0, 0, 0, 0, 0];
    assert!(matches!(
        gamma::decode(crate::codec::built_view(&zeros), 0),
        Err(Decode::Truncated)
    ));
}

// ───────────────────────── frozen storage (Bits) ─────────────────────────

/// Freezing a truncated buffer preserves live bits and writes canonical padding.
#[test]
fn freeze_canonicalizes_storage() {
    // Write a byte of ones, then truncate to 3 live bits: the shed ones
    // leave the buffer's final byte at the truncation, so the raw image
    // is already `1110_0000` before the freeze appends the marker.
    let mut buf: BitsBuf = bits_buf![1; 8];
    buf.truncate(3);
    assert_eq!(buf.as_raw_slice(), &[0b1110_0000]);
    let frozen = super::Bits::freeze(buf.clone());
    assert_eq!(frozen.len(), 3);
    assert_eq!(frozen.as_raw_slice(), &[0b1111_0000]);
    assert!(super::padding_is_canonical(&frozen));
    assert_eq!(frozen.live().to_buf(), buf);
}

/// The marker padding makes stored bytes injective on streams: a stream
/// and its zero-extension — bit-identical up to length — freeze to
/// distinct raw slices, so the byte compare alone decides equality.
#[test]
fn marker_padding_separates_length_collisions() {
    let a = super::Bits::freeze(bits_buf![0, 1]);
    let b = super::Bits::freeze(bits_buf![0, 1, 0]);
    assert_eq!(a.as_raw_slice(), &[0b0110_0000]);
    assert_eq!(b.as_raw_slice(), &[0b0101_0000]);
    assert!(!super::canonical_eq(&a, &b));
}

/// A stream whose live bits fill its final byte exactly still owes a
/// marker: the padding becomes a whole trailing `1000_0000` byte, and
/// the live length reads back through it.
#[test]
fn flush_stream_carries_a_whole_marker_byte() {
    let frozen = super::Bits::freeze(bits_buf![1; 8]);
    assert_eq!(frozen.len(), 8);
    assert_eq!(frozen.as_raw_slice(), &[0xFF, 0b1000_0000]);
    assert!(super::padding_is_canonical(&frozen));
}

/// `Bits::ptr_eq` implies value equality, and nonempty clones share storage.
///
/// A clone shares the frozen buffer (`ptr_eq` true), while two independent
/// freezes of the same *nonempty* content are equal (`canonical_eq`) but not
/// pointer-identical. Independent empty streams may share the same dangling
/// pointer, so pointer identity proves equality but not clone provenance.
#[test]
fn ptr_eq_implies_equality_with_clones_the_nonempty_source() {
    let build = || super::Bits::freeze(bits_buf![1, 0, 1, 1, 0]);
    let a = build();
    let clone = a.clone();
    assert!(a.ptr_eq(&clone));
    assert!(super::canonical_eq(&a, &clone));
    let b = build();
    assert!(!a.ptr_eq(&b));
    assert!(super::canonical_eq(&a, &b));
    // Independently frozen empty streams alias: ptr_eq true with no clone
    // anywhere — and still value-equal, the only fact a fast path may use.
    let e1 = super::Bits::freeze(BitsBuf::new());
    let e2 = super::Bits::freeze(BitsBuf::new());
    assert!(e1.ptr_eq(&e2));
    assert!(super::canonical_eq(&e1, &e2));
}

/// Freezing built bits and adopting their canonical bytes produce equal streams.
#[test]
fn from_canonical_matches_freeze() {
    let frozen = super::Bits::freeze(bits_buf![1, 0, 1]);
    let adopted = super::Bits::from_canonical(bytes::Bytes::copy_from_slice(frozen.as_raw_slice()));
    assert!(super::canonical_eq(&frozen, &adopted));
    assert!(!frozen.ptr_eq(&adopted)); // distinct buffers, equal content

    let empty = super::Bits::from_canonical(bytes::Bytes::new());
    assert!(empty.is_empty());
    assert_eq!(empty.len(), 0);
    assert!(super::canonical_eq(
        &empty,
        &super::Bits::freeze(BitsBuf::new())
    ));
}

// ───────────── build-history family (the buffer's invariants) ─────────────
//
// A buffer's bytes depend only on its live bits. Generated edit sequences are
// compared with a fresh buffer rebuilt from the surviving bits.

/// One step of an arbitrary build history: the buffer's mutating move set.
#[derive(Debug, Clone)]
enum BuildOp {
    /// Append one bit.
    Push(bool),
    /// Append `len` bits of `value` word-wide (`len <= 64`).
    PushBits { value: u64, len: u32 },
    /// Append a selected range from a fresh source stream.
    Extend { src: Vec<bool>, sub: (u16, u16) },
    /// Truncate to empty, a byte boundary, or an arbitrary bit position.
    Truncate { sel: u8, frac: u16 },
}

fn arb_build_op() -> impl Strategy<Value = BuildOp> {
    prop_oneof![
        any::<bool>().prop_map(BuildOp::Push),
        (any::<u64>(), 0u32..=64).prop_map(|(value, len)| BuildOp::PushBits {
            value: if len == 64 {
                value
            } else {
                value & ((1u64 << len) - 1)
            },
            len,
        }),
        (
            proptest::collection::vec(any::<bool>(), 0..100),
            any::<u16>(),
            any::<u16>()
        )
            .prop_map(|(src, a, b)| BuildOp::Extend { src, sub: (a, b) }),
        (any::<u8>(), any::<u16>()).prop_map(|(sel, frac)| BuildOp::Truncate { sel, frac }),
    ]
}

/// Apply one history step to the buffer under test and the `Vec<bool>` model
/// in lockstep.
fn apply_build_op(buf: &mut BitsBuf, model: &mut Vec<bool>, op: &BuildOp) {
    match op {
        BuildOp::Push(bit) => {
            buf.push(*bit);
            model.push(*bit);
        }
        BuildOp::PushBits { value, len } => {
            buf.push_bits(*value, *len);
            for i in (0..*len).rev() {
                model.push(value >> i & 1 == 1);
            }
        }
        BuildOp::Extend { src, sub } => {
            let source: BitsBuf = src.iter().copied().collect();
            let (a, b) = (
                u64::from(sub.0) % (source.len() + 1),
                u64::from(sub.1) % (source.len() + 1),
            );
            let (start, end) = (a.min(b), a.max(b));
            crate::codec::extend_from_view(buf, crate::codec::built_view(&source), start, end);
            model.extend(&src[start as usize..end as usize]);
        }
        BuildOp::Truncate { sel, frac } => {
            let pos = u64::from(*frac) % (buf.len() + 1);
            let target = match sel % 3 {
                0 => 0,           // truncate-to-empty
                1 => pos / 8 * 8, // the deepest byte boundary at or under `pos`
                _ => pos,         // arbitrary, usually mid-byte
            };
            buf.truncate(target);
            model.truncate(usize::try_from(target).expect("test histories are small"));
        }
    }
}

/// A clean rebuild of `content`: single-bit pushes only, no truncation —
/// the reference spelling of the surviving content.
fn clean_rebuild(content: &[bool]) -> BitsBuf {
    content.iter().copied().collect()
}

/// The standard `Hash` image of a frozen stream, for the eq/hash agreement
/// leg.
fn hash_of(bits: &super::Bits) -> u64 {
    use core::hash::{Hash, Hasher};
    let mut hasher = std::hash::DefaultHasher::new();
    // `Bits` hashes through `canonical_hash` via the value types' derives;
    // feed the raw slice exactly as `canonical_hash` does.
    bits.as_raw_slice().hash(&mut hasher);
    hasher.finish()
}

proptest! {
    /// Every edit sequence has the same bytes as a clean rebuild of its live bits.
    ///
    /// The comparison runs after each edit and after freezing, including equality
    /// and hashing of the frozen values.
    #[test]
    fn build_history_spelling_is_a_function_of_content(
        ops in proptest::collection::vec(arb_build_op(), 0..40),
    ) {
        let mut buf = BitsBuf::new();
        let mut model: Vec<bool> = Vec::new();
        for op in &ops {
            apply_build_op(&mut buf, &mut model, op);
            // The representation invariants hold at every intermediate
            // state, not only at the seal: the byte image already equals
            // the clean rebuild's.
            let clean = clean_rebuild(&model);
            prop_assert_eq!(buf.len(), clean.len());
            prop_assert_eq!(buf.as_raw_slice(), clean.as_raw_slice());
        }
        let clean = clean_rebuild(&model);
        prop_assert!(buf == clean, "Eq agrees with bit-level equality");
        let frozen = super::Bits::freeze(buf);
        let reference = super::Bits::freeze(clean);
        prop_assert_eq!(
            frozen.as_raw_slice(),
            reference.as_raw_slice(),
            "one content, one sealed spelling"
        );
        prop_assert!(super::canonical_eq(&frozen, &reference));
        prop_assert_eq!(hash_of(&frozen), hash_of(&reference));
    }

    /// Sealed spellings are injective on contents.
    ///
    /// Two arbitrary build histories freeze to equal spellings exactly
    /// when they end holding equal bit sequences, and the canonical hash
    /// refines the same partition (equal contents hash equal).
    #[test]
    fn build_history_spellings_are_injective(
        ops_a in proptest::collection::vec(arb_build_op(), 0..25),
        ops_b in proptest::collection::vec(arb_build_op(), 0..25),
    ) {
        let (mut a, mut model_a) = (BitsBuf::new(), Vec::new());
        for op in &ops_a {
            apply_build_op(&mut a, &mut model_a, op);
        }
        let (mut b, mut model_b) = (BitsBuf::new(), Vec::new());
        for op in &ops_b {
            apply_build_op(&mut b, &mut model_b, op);
        }
        prop_assert_eq!(a == b, model_a == model_b, "Eq is bit-content equality");
        let (fa, fb) = (super::Bits::freeze(a), super::Bits::freeze(b));
        prop_assert_eq!(
            fa.as_raw_slice() == fb.as_raw_slice(),
            model_a == model_b,
            "spellings collide exactly on equal contents"
        );
        prop_assert_eq!(super::canonical_eq(&fa, &fb), model_a == model_b);
        if model_a == model_b {
            prop_assert_eq!(hash_of(&fa), hash_of(&fb));
        }
    }
}

// ───────────────── word-window fast paths (differential) ─────────────────
//
// The per-bit implementation is the reference for word-wide integer encoding,
// decoding, and skipping. Generated streams emphasize word and window bounds.

/// Encode one integer bit by bit: unary prefix, then an MSB-first mantissa.
fn encode_int_bitwise(out: &mut BitsBuf, n: &BigUint) {
    let m = n + 1u32;
    let k = m.bits() - 1;
    for _ in 0..k {
        out.push(false);
    }
    for i in (0..=k).rev() {
        out.push(m.bit(i));
    }
}

/// Skip one integer bit by bit and return the end position.
fn skip_int_bitwise(bits: BitsView<'_>, pos: u64) -> Result<u64, Decode> {
    let mut k = 0u64;
    loop {
        let idx = pos + k;
        if idx >= bits.len() {
            return Err(Decode::Truncated);
        }
        if bits.bit(idx) {
            break;
        }
        k += 1;
    }
    let end = pos + (2 * k) + 1;
    if end > bits.len() {
        Err(Decode::Truncated)
    } else {
        Ok(end)
    }
}

/// Assert the windowed decoder agrees with the pure bit loop at `pos`: same
/// accept/reject, same error variant, same value, same end position.
fn assert_decode_matches_bit_loop(bits: BitsView<'_>, pos: u64) -> Result<(), TestCaseError> {
    let subject = gamma::decode(bits, pos);
    let mut cursor = SliceCursor::new(bits, pos);
    let oracle = gamma::decode_from(&mut cursor);
    match (subject, oracle) {
        (Ok((value, end)), Ok(oracle_value)) => {
            prop_assert_eq!(value, oracle_value);
            prop_assert_eq!(end, cursor.position());
        }
        (Err(s), Err(o)) => {
            prop_assert_eq!(std::mem::discriminant(&s), std::mem::discriminant(&o));
        }
        (s, o) => prop_assert!(false, "decode disagreement at {}: {:?} vs {:?}", pos, s, o),
    }
    Ok(())
}

/// Assert the word-parallel cursor's `skip_int` agrees with the per-bit
/// reference at `pos` on distance and accept/reject.
///
/// Runs inside the cursor's stated domain — a byte-aligned slice origin and
/// `pos` at or inside the live length; every production skip site satisfies
/// both (stored streams, positions from the same cursor).
fn assert_skip_matches_bit_loop(bits: BitsView<'_>, pos: u64) -> Result<(), TestCaseError> {
    let mut cursor = DsiCursor::new_at(bits, pos);
    match (cursor.skip_int(), skip_int_bitwise(bits, pos)) {
        (Ok(()), Ok(o)) => prop_assert_eq!(cursor.position(), o),
        (Err(_), Err(o)) => {
            prop_assert!(
                matches!(o, Decode::Truncated),
                "the reference rejects Truncated"
            );
        }
        (s, o) => prop_assert!(false, "skip disagreement at {}: {:?} vs {:?}", pos, s, o),
    }
    Ok(())
}

/// `u64` values biased toward power-of-two boundaries, where the gamma code
/// length steps and the emitter's word/loop split sits.
fn arb_boundary_u64() -> impl Strategy<Value = u64> {
    prop_oneof![
        any::<u64>(),
        (0u32..64).prop_map(|b| 1u64 << b),
        (0u32..64).prop_map(|b| (1u64 << b) - 1),
        (0u32..63).prop_map(|b| (1u64 << b) + 1),
        Just(u64::MAX),
    ]
}

/// Bit streams shaped like gamma codes at every window boundary.
///
/// `pad` positions the read mid-byte, `zeros` spans prefix lengths across the
/// 31/32 window split and the 63/64/65 word widths, and `rest` supplies — or,
/// when short, truncates — the mantissa, plus trailing junk.
fn arb_gamma_stream() -> impl Strategy<Value = (BitsBuf, usize)> {
    (
        proptest::collection::vec(any::<bool>(), 0..17),
        prop_oneof![
            0usize..=70,
            Just(31usize),
            Just(32usize),
            Just(63usize),
            Just(64usize),
            Just(65usize),
        ],
        proptest::collection::vec(any::<bool>(), 0..80),
    )
        .prop_map(|(pad, zeros, rest)| {
            let pos = pad.len();
            let mut bits = BitsBuf::new();
            bits.extend(pad);
            for _ in 0..zeros {
                bits.push(false);
            }
            bits.extend(rest);
            (bits, pos)
        })
}

proptest! {
    /// The word-wise encoder is byte-identical to the per-bit emitter.
    ///
    /// Holds for every value — `u64`-range codes (the `store_be` path) and
    /// spilled wide values alike — even appending at an unaligned mid-stream
    /// position; and the windowed decoder reads its output back exactly.
    #[test]
    fn gamma_word_encode_matches_bit_encode(
        prefix in proptest::collection::vec(any::<bool>(), 0..17),
        n in arb_boundary_u64(),
        limbs in proptest::collection::vec(any::<u64>(), 0..3),
    ) {
        let mut value = BigUint::from(n);
        for limb in limbs {
            value = (value << 64) | BigUint::from(limb);
        }
        let pos = prefix.len();
        let mut word = BitsBuf::new();
        let mut bit = BitsBuf::new();
        for b in prefix {
            word.push(b);
            bit.push(b);
        }
        gamma::encode(&value, &mut word);
        encode_int_bitwise(&mut bit, &value);
        prop_assert_eq!(&word, &bit);

        // Word-decode of the word-encode round-trips value and position.
        let (decoded, end) =
            gamma::decode(crate::codec::built_view(&word), pos as u64).expect("well-formed");
        prop_assert_eq!(decoded, value);
        prop_assert_eq!(end, word.len());
    }
}

proptest! {
    /// On window-boundary streams, the windowed decoder and the
    /// word-parallel cursor's `skip_int` behave exactly like the per-bit
    /// loops.
    ///
    /// Agreement covers accept/reject, error variant, value, and consumed bits
    /// — at the code position, near and past the stream end, and on a mid-byte
    /// re-slice (where the window declines and only the loop runs).
    #[test]
    fn gamma_word_decode_matches_bit_loop(
        (bits, pos) in arb_gamma_stream(),
        extra in 0usize..3,
    ) {
        let view = crate::codec::built_view(&bits);
        let pos = pos as u64;
        let extra = extra as u64;
        assert_decode_matches_bit_loop(view, pos)?;
        assert_skip_matches_bit_loop(view, pos)?;

        // The end of the stream, just before it, and past it (the skip cursor's
        // domain ends at the live length; the gamma decoder alone covers the
        // past-the-end positions).
        assert_decode_matches_bit_loop(view, view.len().saturating_sub(extra))?;
        assert_skip_matches_bit_loop(view, view.len().saturating_sub(extra))?;
        assert_decode_matches_bit_loop(view, view.len() + extra)?;
    }
}

proptest! {
    /// On arbitrary raw byte streams — mostly invalid input — the windowed
    /// Gamma decoding and the word-parallel cursor's `skip_int` agree with the
    /// per-bit loops.
    ///
    /// Agreement covers accept/reject, error variant, value, and consumed bits
    /// at every position (the skip at every position inside the live length,
    /// its cursor's domain).
    #[test]
    fn gamma_word_paths_match_on_arbitrary_bytes(
        bytes in proptest::collection::vec(any::<u8>(), 0..12),
        pos in 0usize..104,
    ) {
        let bits = crate::codec::BitsView::whole(&bytes);
        let pos = pos as u64;
        assert_decode_matches_bit_loop(bits, pos)?;
        assert_skip_matches_bit_loop(bits, pos.min(bits.len()))?;
    }
}

/// The window decoder accepts a code exactly filling its 64 provable bits and
/// declines one bit past that; junk after a code never leaks into the mantissa.
///
/// Prefix `k = 31` (a 63-bit code) is the widest code one window proves and
/// must decode; `k = 32` (65 bits) straddles the window edge and must fall back
/// — where the bit loop still decodes it — as must the 63-bit code cut one bit
/// short of complete.
#[test]
fn gamma_window_edge() {
    // k = 31: the widest code a 64-bit window proves.
    let n = (1u64 << 31) - 1;
    let mut bits = BitsBuf::new();
    gamma::encode(&BigUint::from(n), &mut bits);
    assert_eq!(bits.len(), 63);
    assert_eq!(
        gamma::decode_window(crate::codec::built_view(&bits), 0),
        Some((n, 63))
    );

    // The same code cut one bit short: nothing provable, decline.
    assert_eq!(
        gamma::decode_window(crate::codec::BitsView::new(bits.as_raw_slice(), 62), 0),
        None
    );

    // k = 32: a 65-bit code straddles the window edge — decline, and the full
    // decoder still reads it through the loop.
    let n = (1u64 << 32) - 1;
    let mut bits = BitsBuf::new();
    gamma::encode(&BigUint::from(n), &mut bits);
    assert_eq!(bits.len(), 65);
    assert_eq!(
        gamma::decode_window(crate::codec::built_view(&bits), 0),
        None
    );
    let (decoded, end) = gamma::decode(crate::codec::built_view(&bits), 0).expect("well-formed");
    assert_eq!(decoded, BigUint::from(n));
    assert_eq!(end, 65u64);

    // Junk after a short code must not leak into its mantissa.
    let mut bits = BitsBuf::new();
    gamma::encode(&BigUint::from(5u64), &mut bits);
    let code_len = bits.len();
    for _ in 0..64 {
        bits.push(true);
    }
    assert_eq!(
        gamma::decode_window(crate::codec::built_view(&bits), 0),
        Some((5, code_len))
    );
}

/// The window decoder never guesses at unprovable input: a position at or
/// past the stream end and an all-zeros (truncated) stream both decline to
/// the bit loop.
#[test]
fn gamma_window_declines_conservatively() {
    let mut bits = BitsBuf::new();
    bits.push(false);
    bits.push(true);

    // A mid-stream position addressed as (whole view, pos) proves its code
    // and the fast path fires.
    assert_eq!(
        gamma::decode_window(crate::codec::built_view(&bits), 1),
        Some((0, 2))
    );

    // At and past the end of the stream.
    assert_eq!(
        gamma::decode_window(crate::codec::built_view(&bits), 2),
        None
    );
    assert_eq!(
        gamma::decode_window(crate::codec::built_view(&bits), 7),
        None
    );

    // All zeros: no terminating 1 in the stream (bit loop: `Truncated`).
    let zeros = BitsBuf::repeat(false, 70);
    assert_eq!(
        gamma::decode_window(crate::codec::built_view(&zeros), 0),
        None
    );
}

/// A gamma code wide enough to spill machine-word decoding round-trips exactly
/// and remains self-delimiting (the whole mantissa is one spilled value,
/// byte-unaligned on both ends).
#[test]
fn gamma_roundtrip_wide_value() {
    // 2^1000 + 12345: a 1001-bit mantissa with live bits at both ends.
    let n = (BigUint::from(1u8) << 1000u32) + 12345u64;
    let mut bits = BitsBuf::new();
    gamma::encode(&n, &mut bits);
    let (decoded, pos) = gamma::decode(crate::codec::built_view(&bits), 0).expect("well-formed");
    assert_eq!(decoded, n);
    assert_eq!(pos, bits.len());
}

/// A stream that ends anywhere inside a wide mantissa is `Truncated`: the
/// wide-decode accept/reject boundary sits exactly at the declared code length,
/// wherever the cut falls relative to byte alignment.
#[test]
fn gamma_truncated_inside_wide_mantissa() {
    let n = (BigUint::from(1u8) << 1000u32) + 12345u64;
    let mut bits = BitsBuf::new();
    gamma::encode(&n, &mut bits);
    // Cuts inside the unary prefix, at the leading mantissa 1, just after it,
    // at byte-scale offsets into the mantissa, and one bit short.
    for cut in [1, 500, 1001, 1002, 1009, 1500, bits.len() - 1] {
        let truncated = crate::codec::BitsView::new(bits.as_raw_slice(), cut);
        assert!(
            matches!(gamma::decode(truncated, 0), Err(Decode::Truncated)),
            "cut at bit {cut} must report Truncated",
        );
    }
    // The full code still decodes: the cuts, not the value, are the failure.
    let (decoded, pos) = gamma::decode(crate::codec::built_view(&bits), 0).expect("well-formed");
    assert_eq!(decoded, n);
    assert_eq!(pos, bits.len());
}

// ───────────────────────── decode∘encode round-trip ─────────────────────────

proptest! {
    /// `decode(encode(x)) == x` for `Party`, `Version`, and `Clock`.
    #[test]
    fn decode_encode_roundtrip(ops in world_strategy(), i in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        let oc = &cs[i % n];
        let (op, ov) = oc.trees();

        let party = from_oracle_party(op);
        prop_assert!(Party::decode(&party.encode()[..]).expect("valid") == party);

        let version = from_oracle_version(ov);
        prop_assert!(Version::decode(&version.encode()[..]).expect("valid") == version);

        let clock = from_oracle_clock(oc);
        let clock2 = Clock::decode(&clock.encode()[..]).expect("valid");
        prop_assert!(clock.party() == clock2.party());
        prop_assert!(clock.version() == clock2.version());
    }
}

proptest! {
    /// `encode_to` writes the same bytes as `encode` to an arbitrary writer —
    /// here a `BufWriter`, a distinct buffered `Write` impl.
    ///
    /// For `Clock` this exercises the streamed id/event boundary (the partial
    /// byte merged across the two component streams with no combined buffer).
    #[test]
    fn encode_to_matches_encode(ops in world_strategy(), i in 0usize..64) {
        use std::io::BufWriter;
        let cs = run(&ops);
        let n = cs.len();
        let oc = &cs[i % n];
        let (op, ov) = oc.trees();

        let party = from_oracle_party(op);
        let mut bw = BufWriter::new(Vec::new());
        party.encode_to(&mut bw).unwrap();
        prop_assert_eq!(bw.into_inner().unwrap(), party.encode());

        let version = from_oracle_version(ov);
        let mut bw = BufWriter::new(Vec::new());
        version.encode_to(&mut bw).unwrap();
        prop_assert_eq!(bw.into_inner().unwrap(), version.encode());

        let clock = from_oracle_clock(oc);
        let mut bw = BufWriter::new(Vec::new());
        clock.encode_to(&mut bw).unwrap();
        prop_assert_eq!(bw.into_inner().unwrap(), clock.encode());
    }
}

// ───────────────────────── canonical encoding injectivity ─────────────────────────

proptest! {
    /// `a == b` ⇔ `encode(a) == encode(b)`; equality also matches the oracle's
    /// (encode is injective on normal forms).
    #[test]
    fn canonical_encoding_is_injective(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        let vs = versions(&cs);

        let a = from_oracle_version(&vs[i % n]);
        let b = from_oracle_version(&vs[j % n]);
        prop_assert_eq!(a == b, a.encode() == b.encode());
        prop_assert_eq!(a == b, vs[i % n] == vs[j % n]);

        let pa = from_oracle_party(cs[i % n].party());
        let pb = from_oracle_party(cs[j % n].party());
        prop_assert_eq!(pa == pb, pa.encode() == pb.encode());
        prop_assert_eq!(pa == pb, cs[i % n].party() == cs[j % n].party());
    }
}

// Clock canonical byte-injectivity (`Eq` ⟺ encode-bytes equality, `Eq`/`Hash`
// coherence) is the `laws::CLOCK_PAIR` group, driven over arbitrary
// party/version pairings, organic op-trace populations, and the fuzz target's
// decoded values.

// ───────────────────────── decode rejection of non-canonical input ─────────────────────────

/// The only collapsible id node representable in the pruned encoding is `(1,
/// 1)` — a node with two terminal children — which must be rejected as
/// `NotCanonical` (it collapses to `1`).
///
/// The other collapsible form, `(0, 0)`, cannot even be written: a `0` is the
/// *absence* of a child, so a node with two `0` children has no bits and simply
/// is `0`.
#[test]
fn reject_noncanonical_id() {
    use oracle::Party::{Leaf, Node};
    let denormal = Node(Arc::new(Leaf(true)), Arc::new(Leaf(true)));
    let bytes = from_oracle_party(&denormal).encode();
    assert!(
        matches!(Party::decode(&bytes[..]), Err(Decode::NotCanonical)),
        "collapsible id node (1, 1) must be rejected as NotCanonical",
    );
}

/// The id validator runs bottom-up by recursion, so a collapsible `(v, v)` node
/// buried under deep, otherwise-canonical nesting must still be caught.
///
/// The `NotCanonical` check fires when *any* node completes, not only at the
/// root. Build a left-leaning spine `(((… (1,1) …, 0), 0), 0)` whose deepest
/// node is the denormal `(1, 1)`, exercising the validator's recursion past a
/// single byte.
#[test]
fn reject_deep_nested_denormal_id() {
    use oracle::Party::{Leaf, Node};

    // Innermost collapsible node, then 16 layers of canonical `(_, 0)`
    // wrapping. Each wrapper is itself normal (a node child paired with a `0`
    // leaf), so the only non-canonical node is the buried `(1, 1)`.
    const DEPTH: usize = 16;
    let mut tree = Node(Arc::new(Leaf(true)), Arc::new(Leaf(true)));
    for _ in 0..DEPTH {
        tree = Node(Arc::new(tree), Arc::new(Leaf(false)));
    }
    let bytes = from_oracle_party(&tree).encode();

    // The encoding spans several bytes, so this drives the stack-based
    // validator well past the trivial single-node case.
    assert!(bytes.len() > 1, "deep denormal must span multiple bytes");
    assert!(matches!(
        Party::decode(&bytes[..]),
        Err(Decode::NotCanonical)
    ));
}

/// Padding rejection is bit-granular, not byte-granular: a complete tree
/// that ends mid-byte must be followed by *exactly* the marker and zeros
/// within that final byte.
///
/// A perturbed bit inside the same byte as the tree — a cleared marker or
/// a set bit after it — is `TrailingBits`, just as a whole spurious
/// trailing byte is. The id leaf `1` encodes to the two-bit terminal tag
/// (`0, 0`), marker-padded as `0010_0000`; perturbing any padding bit
/// within that byte must be rejected.
#[test]
fn reject_intra_byte_padding() {
    // `Leaf(true)` = the terminal tag bits [0, 0], then the marker at
    // bit 2 → one byte 0b0010_0000; bits 3..8 are zero padding.
    let clean = from_oracle_party(&oracle::Party::Leaf(true)).encode();
    assert_eq!(clean, vec![0b0010_0000], "an id leaf fits in a single byte");
    assert!(Party::decode(&clean[..]).is_ok(), "clean padding decodes");

    // Clearing the marker (bit 2) leaves the padding un-delimited.
    assert!(
        matches!(Party::decode(&[0u8][..]), Err(Decode::TrailingBits)),
        "a cleared padding marker must be rejected",
    );

    // Set each zero padding bit (positions 3..8) in turn; each is
    // `TrailingBits`.
    for bit in 3u8..8 {
        let mut bytes = clean.clone();
        bytes[0] |= 0b1000_0000u8 >> bit;
        assert!(
            matches!(Party::decode(&bytes[..]), Err(Decode::TrailingBits)),
            "a set intra-byte padding bit at position {bit} must be rejected",
        );
    }
}

/// `Version` and `Clock` decoding reject every set bit within their padding.
///
/// The clock cases corrupt both its interior party padding and its final
/// version padding.
#[test]
fn version_and_clock_decoding_rejects_intra_byte_padding() {
    // The empty version is the 2-bit stream `11`, marker-padded in one
    // byte: the marker at bit 2, zeros at 3..8. Set each zero in turn;
    // every one must reject.
    let clean = Version::new().encode();
    assert_eq!(clean, vec![0b1110_0000]);
    for bit in 3u8..8 {
        let mut bytes = clean.clone();
        bytes[0] |= 0b1000_0000u8 >> bit;
        assert!(
            matches!(Version::decode(&bytes[..]), Err(Decode::TrailingBits)),
            "version: set padding bit {bit} must be rejected",
        );
    }

    // The seed clock is the party byte `0x20` (2 live bits, marker at
    // bit 2) then the version byte `0xE0` (likewise): four set-bit
    // defect sites, two per component's zero padding, plus each
    // component's cleared marker.
    let clock = Clock::seed().encode();
    assert_eq!(clock, vec![0b0010_0000, 0b1110_0000]);
    for (byte, bit) in [(0usize, 3u8), (0, 7), (1, 3), (1, 7)] {
        let mut bytes = clock.clone();
        bytes[byte] |= 0b1000_0000u8 >> bit;
        assert!(
            matches!(Clock::decode(&bytes[..]), Err(Decode::TrailingBits)),
            "clock: set padding bit {bit} of byte {byte} must be rejected",
        );
    }
    for byte in [0usize, 1] {
        let mut bytes = clock.clone();
        bytes[byte] &= !(0b1000_0000u8 >> 2);
        assert!(
            matches!(Clock::decode(&bytes[..]), Err(Decode::TrailingBits)),
            "clock: cleared marker of byte {byte} must be rejected",
        );
    }
}

/// Transcoding a non-normal recursive tree produces the canonical stream for
/// the same step function.
#[test]
fn transcoding_normalizes_noncanonical_event() {
    use oracle::Version::{Leaf, Node};

    // No child has base 0: unspellable on the wire — the transcoding
    // quotients the spelling onto the normalized tree's stream.
    let no_zero = Node(
        0u64.into(),
        Arc::new(Leaf(1u64.into())),
        Arc::new(Leaf(2u64.into())),
    );
    let normalized = Node(
        1u64.into(),
        Arc::new(Leaf(0u64.into())),
        Arc::new(Leaf(1u64.into())),
    );
    assert_eq!(
        from_oracle_version(&no_zero).encode(),
        from_oracle_version(&normalized).encode(),
        "the wire coding admits exactly one spelling per value",
    );
}

/// Every raw decoder classifies an empty slice as truncated input.
///
/// The wire grammar has no empty production: an anonymous (`0`) id is spelled
/// by a zero presence bit in its parent's 2-bit tag (structural absence), never
/// by bits of its own, and no encoder emits a value with zero bytes — the tree
/// codes are prefix-free, and a zero-length spelling cannot be self-delimiting.
/// So the empty input is not a parse of anything; in particular,
/// `Party::decode` and `Clock::decode` reject it as truncation before any
/// anonymity question could arise.
#[test]
fn empty_input_is_truncated() {
    // The test-only anonymous id has no wire spelling: it encodes to zero
    // bytes, which no self-delimiting decoder can be handed as a value.
    let anon = from_oracle_party(&oracle::Party::Leaf(false)).encode();
    assert!(anon.is_empty(), "the anonymous id encodes to no bytes");

    assert!(matches!(Party::decode(&[][..]), Err(Decode::Truncated)));
    assert!(matches!(Version::decode(&[][..]), Err(Decode::Truncated)));
    assert!(matches!(Clock::decode(&[][..]), Err(Decode::Truncated)));
    assert!(matches!(Rank::decode(&[][..]), Err(Decode::Truncated)));
    assert!(matches!(Ranked::decode(&[][..]), Err(Decode::Truncated)));
    assert!(matches!(Span::decode(&[][..]), Err(Decode::Truncated)));
}

/// `Clock::decode` can never yield a clock with an anonymous (`0`) party — the
/// invariant the whole stack rests on (paper §3: a live share is `i ≠ 0`).
///
/// The party is the byte-aligned prefix and the id grammar has no empty
/// production (the only would-be-empty prefix is the whole empty stream, itself
/// rejected as exhausted input), so an anonymous-party clock has *no* encoding:
/// its bytes (just the version, since the `0` party contributes none) decode to
/// a *different*, non-anonymous clock or fail canonicity — never round-trip
/// back. This sweeps every byte string up to two bytes, where the empty-prefix
/// boundary lives: each either fails to decode or yields a nonzero party, and
/// none panics.
#[test]
fn decode_never_yields_anonymous_party() {
    // A test-only anonymous clock encodes to just its version bytes; decoding
    // reinterprets them and never recovers the anonymous party.
    let anon = from_oracle_clock(&oracle::Clock::from_parts(
        oracle::Party::Leaf(false),
        oracle::Version::from(5u64),
    ));
    if let Ok(c) = Clock::decode(&anon.encode()[..]) {
        assert!(
            !c.party().as_bytes().is_empty(),
            "an anonymous-party clock must not round-trip",
        );
    }

    // Exhaustive over the small-clock space (`len = 0` is the empty stream).
    for len in 0..=2usize {
        for v in 0u32..(1u32 << (8 * len)) {
            let bytes = &v.to_be_bytes()[4 - len..];
            if let Ok(c) = Clock::decode(bytes) {
                assert!(
                    !c.party().as_bytes().is_empty(),
                    "decoded an anonymous-party clock from {bytes:?}",
                );
            }
        }
    }
}

/// A stream that ends mid-tree is `Truncated`.
#[test]
fn reject_truncated() {
    // 0xFF is eight both-present id tags in a row — the tree never
    // bottoms out.
    assert!(matches!(Party::decode(&[0xFF][..]), Err(Decode::Truncated)));
    // 0x00 is eight internal-node flags in a row — the tree never
    // bottoms out.
    assert!(matches!(
        Version::decode(&[0x00][..]),
        Err(Decode::Truncated)
    ));
}

/// A non-zero bit after a complete tree is `TrailingBits`.
#[test]
fn reject_trailing_bits() {
    let mut bytes = from_oracle_party(&oracle::Party::Leaf(true)).encode();
    bytes.push(0x01); // a set bit beyond the (complete) tree and its padding
    assert!(matches!(
        Party::decode(&bytes[..]),
        Err(Decode::TrailingBits)
    ));

    let mut bytes = from_oracle_version(&oracle::Version::new()).encode();
    bytes.push(0x80);
    assert!(matches!(
        Version::decode(&bytes[..]),
        Err(Decode::TrailingBits)
    ));
}

// ───────────────────── decode mutation tests ─────────────────────
//
// Mutations of valid encodings exercise the boundary of the accepted language
// more directly than uniform random bytes. A mutation must either be rejected
// or decode to a normal value that re-encodes to the same bytes.

/// Assert the accept-canonically contract for a `Party` decode of `bytes`: if
/// it decodes, the value is normal form and re-encodes to exactly `bytes`.
fn assert_party_accept_canonical(bytes: &[u8]) {
    if let Ok(p) = Party::decode(bytes) {
        assert!(
            to_oracle_party(&p).is_normal(),
            "decode accepted a non-normal Party from {bytes:02x?}",
        );
        assert_eq!(
            p.encode(),
            bytes,
            "accepted Party does not re-encode to its own input bytes",
        );
    }
}

/// As [`assert_party_accept_canonical`], for a `Version` decode.
fn assert_version_accept_canonical(bytes: &[u8]) {
    if let Ok(v) = Version::decode(bytes) {
        assert!(
            to_oracle_version(&v).is_normal(),
            "decode accepted a non-normal Version from {bytes:02x?}",
        );
        assert_eq!(
            v.encode(),
            bytes,
            "accepted Version does not re-encode to its own input bytes",
        );
    }
}

/// As [`assert_party_accept_canonical`], for a `Clock` decode. Both lowered
/// components must be normal form, and the clock must re-encode to its own
/// input bytes.
fn assert_clock_accept_canonical(bytes: &[u8]) {
    if let Ok(c) = Clock::decode(bytes) {
        let (p, v) = to_oracle_clock(&c);
        assert!(
            p.is_normal() && v.is_normal(),
            "decode accepted a non-normal Clock from {bytes:02x?}",
        );
        assert_eq!(
            c.encode(),
            bytes,
            "accepted Clock does not re-encode to its own input bytes",
        );
    }
}

/// Run the accept-canonically contract for all three decoders against the same
/// bytes.
fn assert_all_accept_canonical(bytes: &[u8]) {
    assert_party_accept_canonical(bytes);
    assert_version_accept_canonical(bytes);
    assert_clock_accept_canonical(bytes);
}

proptest! {
    /// Flipping any single bit of a valid clock encoding yields a stream that
    /// `decode` either rejects or accepts canonically (normal-form,
    /// re-encode-stable), for every bit position and every decoder.
    ///
    /// Single-bit flips are the most targeted mutation: each lands one Hamming
    /// step from the accepted language.
    ///
    /// A flip can shift the tree to end early enough that a whole trailing byte
    /// follows the padding; `decode` must reject any remainder past one padded
    /// byte (`require_marker_padding`'s length bound), keeping `decode`
    /// injective on bytes.
    #[test]
    fn bit_flip_rejects_or_decodes_canonically(
        pa in arb_oracle_party_nonempty(),
        va in arb_oracle_version(),
    ) {
        let clock = Clock::from_parts(from_oracle_party(&pa), from_oracle_version(&va));
        let valid = clock.encode();

        // The unmutated stream must of course be accepted canonically.
        assert_all_accept_canonical(&valid);

        for byte in 0..valid.len() {
            for bit in 0u8..8 {
                let mut m = valid.clone();
                m[byte] ^= 0b1000_0000u8 >> bit;
                assert_all_accept_canonical(&m);
            }
        }
    }
}

proptest! {
    /// Truncating a valid encoding at any byte boundary yields a stream that
    /// `decode` rejects or accepts canonically.
    ///
    /// A prefix of a complete tree is almost always `Truncated`, but a prefix
    /// can occasionally itself be a complete smaller tree (e.g. the leading id
    /// leaf of a clock), which must then decode canonically, never to a
    /// malformed value.
    ///
    /// A truncation can cut a valid stream just *after* a complete tree and its
    /// padding but inside later bytes; `decode` must reject any remainder past
    /// one padded byte rather than accept a value that re-encodes to fewer
    /// bytes than its own input.
    #[test]
    fn truncation_rejects_or_decodes_canonically(
        pa in arb_oracle_party_nonempty(),
        va in arb_oracle_version(),
    ) {
        let clock = Clock::from_parts(from_oracle_party(&pa), from_oracle_version(&va));
        let valid = clock.encode();
        for cut in 0..valid.len() {
            assert_all_accept_canonical(&valid[..cut]);
        }
    }
}

/// Direct examples cover each marker-padding boundary exercised by mutation.
///
/// A canonical encoding pads with exactly one `1` marker and then zeros to the
/// byte boundary, all within one byte, which is what makes `decode` **injective
/// on byte strings**: every stream has one padded spelling, and no accepted
/// input re-encodes to different bytes than its own. Each clause below rejects
/// one way of breaking that: a spurious whole zero byte after the padding and a
/// zeroed marker byte are [`Decode::TrailingBits`] (the padding is present but
/// malformed, or input runs beyond it), and a flush stream cut before its whole
/// marker byte is [`Decode::Truncated`] (the padding is missing, not malformed).
#[test]
fn malformed_padding_rejected_witness() {
    // Canonical encoding of the event `(2, 0, 1)`: the 9-bit stream
    // `0 1 011 1 011` — internal root (flag 0), leaf flag 1 + gamma(2),
    // leaf flag 1 + zigzag(+1) = gamma(2) — then the marker and pad.
    let canonical = from_oracle_version(&oracle::Version::node(
        2u8,
        oracle::Version::leaf(0u8),
        oracle::Version::leaf(1u8),
    ))
    .encode();
    assert_eq!(
        canonical,
        vec![93, 0b1100_0000],
        "witness canonical encoding"
    );

    // Appending a whole zero byte must be rejected: the padding already
    // ended within the second byte, so more bytes are spurious.
    let mut with_zero = canonical.clone();
    with_zero.push(0);
    assert!(
        matches!(Version::decode(&with_zero[..]), Err(Decode::TrailingBits)),
        "a whole trailing zero byte is non-canonical and must be rejected",
    );

    // An id whose 8 live bits fill its first byte exactly: `(1, (0, 1))`
    // owes its marker in a whole second byte.
    let party = from_oracle_party(&oracle::Party::node(
        oracle::Party::Leaf(true),
        oracle::Party::node(oracle::Party::Leaf(false), oracle::Party::Leaf(true)),
    ))
    .encode();
    assert_eq!(
        party,
        vec![196, 0b1000_0000],
        "witness party canonical encoding"
    );

    // Truncating the marker byte leaves a parseable tree with no padding
    // at all: rejected as missing required data, so the flush stream has
    // exactly one spelling.
    assert!(
        matches!(Party::decode(&party[..1]), Err(Decode::Truncated)),
        "a flush stream cut before its marker byte is missing required data",
    );

    // Zeroing the marker byte is the same defect spelled longer.
    assert!(
        matches!(Party::decode(&[196, 0][..]), Err(Decode::TrailingBits)),
        "an all-zero padding byte has no marker and must be rejected",
    );

    // A zero byte after the marker byte is spurious.
    assert!(
        matches!(
            Party::decode(&[196, 0b1000_0000, 0][..]),
            Err(Decode::TrailingBits)
        ),
        "a whole trailing zero byte after the padding must be rejected",
    );
}

proptest! {
    /// The padding after the live bits is exactly one `1` marker then
    /// zeros, and perturbing any of it — clearing the marker, or setting
    /// any zero bit after it — must be rejected (`TrailingBits`), never
    /// silently accepted.
    ///
    /// A perturbed padding makes the stream non-canonical, which would
    /// break the byte-equality `Eq`/`Hash` contract. The whole-byte and
    /// intra-byte cases are pinned by hand in the canonical-rejection
    /// suite; this sweeps every padding position over arbitrary trees.
    #[test]
    fn padding_perturbation_rejects(pa in arb_oracle_party_nonempty()) {
        let party = from_oracle_party(&pa);
        let valid = party.encode();

        // The marker sits at the live bit length; zeros fill the rest of
        // the final byte.
        let used = party.as_bits().len() as usize;
        let total = valid.len() * 8;

        // Clearing the marker leaves the padding without its delimiter.
        {
            let (byte, bit) = (used / 8, (used % 8) as u8);
            let mut m = valid.clone();
            m[byte] &= !(0b1000_0000u8 >> bit);
            prop_assert!(
                matches!(Party::decode(&m[..]), Err(Decode::TrailingBits)),
                "a cleared padding marker must be rejected",
            );
        }

        // Setting any zero bit after the marker breaks the `1 0*` form.
        for pad in used + 1..total {
            let (byte, bit) = (pad / 8, (pad % 8) as u8);
            let mut m = valid.clone();
            m[byte] |= 0b1000_0000u8 >> bit;
            prop_assert!(
                matches!(Party::decode(&m[..]), Err(Decode::TrailingBits)),
                "a set bit at padding position {pad} must be rejected",
            );
        }
    }
}

// ─────────────────────── flush-boundary truncation ───────────────────────
//
// A canonical encoding whose live bits end flush against a byte boundary
// carries its padding in a whole final `1000_0000` byte; cutting the input just
// before that byte leaves a complete tree with its required padding missing
// entirely. The family spans the five marker-padded wire types (`Party`,
// `Version`, `Clock`, `Ranked`, `Span`); `Rank` has no marker padding (its
// stream is self-delimiting within its final byte), so no flush-cut input
// exists for it and its truncations are all mid-stream.

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

/// A stream missing its full padding byte reports [`Decode::Truncated`].
///
/// A uniform version at height 7 encodes to eight live bits (leaf flag `1`,
/// gamma(7) `0001000`) plus a whole `1000_0000` padding byte, so its first
/// byte alone is a complete tree whose required padding byte is absent
/// entirely: missing required data.
#[test]
fn flush_version_cut_before_its_marker_byte_is_truncated() {
    let bytes = from_oracle_version(&oracle::Version::leaf(7u8)).encode();
    assert_eq!(
        bytes,
        vec![0b1000_1000, 0b1000_0000],
        "witness canonical encoding"
    );
    assert!(matches!(
        Version::decode(&bytes[..1]),
        Err(Decode::Truncated)
    ));
}

proptest! {
    /// A stream cut exactly at a flush byte boundary reads
    /// [`Decode::Truncated`] from every encoding with a version tail.
    ///
    /// Live bits end on the boundary and the whole `1000_0000` padding byte is
    /// absent: required data is missing, not malformed. Exercised at the end of
    /// the input (`Version`, and the version tail of `Clock`, `Ranked`, and
    /// `Span`) and at the boundary inside `Span` (its meet cut short of its own
    /// padding byte, the join then missing entirely).
    #[test]
    fn flush_cut_version_reads_truncated_for_every_version_tail(
        v in arb_flush_version(),
        pa in arb_oracle_party_nonempty(),
    ) {
        let bytes = v.encode();
        prop_assert_eq!(
            bytes.len() as u64 * 8,
            v.encoded_bits() + 8,
            "the padding occupies a whole final byte",
        );
        let cut = &bytes[..bytes.len() - 1];
        prop_assert!(matches!(Version::decode(cut), Err(Decode::Truncated)));

        // The version tail of a clock.
        let clock = Clock::from_parts(from_oracle_party(&pa), v.clone());
        let clock_bytes = clock.encode();
        prop_assert!(matches!(
            Clock::decode(&clock_bytes[..clock_bytes.len() - 1]),
            Err(Decode::Truncated)
        ));

        // The version tail of a ranked key.
        let key = Ranked::from(&v).encode();
        prop_assert!(matches!(
            Ranked::decode(&key[..key.len() - 1]),
            Err(Decode::Truncated)
        ));

        // The join tail of a span (the hull of the empty version and `v`).
        let span = Version::new().span(&v).encode();
        prop_assert!(matches!(
            Span::decode(&span[..span.len() - 1]),
            Err(Decode::Truncated)
        ));

        // The meet is cut short of its padding byte; the join is absent.
        prop_assert!(matches!(Span::decode(cut), Err(Decode::Truncated)));
    }
}

proptest! {
    /// A party stream cut exactly at a flush byte boundary reads
    /// [`Decode::Truncated`] from both party and clock decoding.
    ///
    /// Live bits end on the boundary and the whole `1000_0000` padding byte is
    /// absent: missing required data at the end of the input (`Party`) and at
    /// the boundary inside a clock (the party cut short of its own padding
    /// byte, the version then missing entirely).
    #[test]
    fn flush_cut_party_reads_truncated_for_party_and_clock(p in arb_flush_party()) {
        let bytes = p.encode();
        prop_assert_eq!(
            bytes.len() as u64 * 8,
            p.encoded_bits() + 8,
            "the padding occupies a whole final byte",
        );
        let cut = &bytes[..bytes.len() - 1];
        prop_assert!(matches!(Party::decode(cut), Err(Decode::Truncated)));
        prop_assert!(matches!(Clock::decode(cut), Err(Decode::Truncated)));
    }
}
