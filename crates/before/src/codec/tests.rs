//! Codec tests: round-trip, canonical injectivity, and strict rejection of
//! malformed / non-canonical input.
//!
//! Impl values are built from oracle trees via the bridge (canonical bits
//! emitted directly), so these test the *codec* in isolation from the operation
//! algorithms.

use bitvec::prelude::*;
use proptest::prelude::*;

use proptest::test_runner::TestCaseError;

use super::{
    bytes_as_bits, decode_int, decode_int_from, encode_int, skip_int, Base, BitCursor, Bits,
    BitsSlice, SliceCursor, PARSE_STACK_INLINE,
};
use crate::oracle;
use crate::testing::bridge::{
    from_oracle_clock, from_oracle_party, from_oracle_version, to_oracle_clock, to_oracle_party,
    to_oracle_version,
};
use crate::testing::generators::{
    arb_oracle_party_nonempty, arb_oracle_version, deep_left_spine_party,
};
use crate::testing::optrace::{run, versions, world_strategy};
use crate::{error::Decode, Clock, Party, Version};

// ───────────────────────────── integer code ─────────────────────────────

proptest! {
    /// `decode_int ∘ encode_int == id`, and the code is self-delimiting
    /// (consumes exactly the bits it wrote).
    #[test]
    fn gamma_roundtrip(n in 0u64..1_000_000) {
        let n = Base::from(n);
        let mut bits = Bits::new();
        encode_int(&mut bits, &n);
        let (decoded, pos) = decode_int(&bits, 0).expect("well-formed");
        prop_assert_eq!(decoded, n);
        prop_assert_eq!(pos, bits.len());
    }
}

proptest! {
    /// The integer code round-trips arbitrary-width magnitudes with no cap: a
    /// value built from many random `u64` limbs (well past `u64::MAX`) survives
    /// `decode_int ∘ encode_int` exactly and remains self-delimiting.
    #[test]
    fn gamma_roundtrip_wide(limbs in proptest::collection::vec(any::<u64>(), 1..40)) {
        let mut n = Base::ZERO;
        for limb in limbs {
            n = (n << 64) | Base::from(limb);
        }
        let mut bits = Bits::new();
        encode_int(&mut bits, &n);
        let (decoded, pos) = decode_int(&bits, 0).expect("well-formed");
        prop_assert_eq!(decoded, n);
        prop_assert_eq!(pos, bits.len());
    }
}

/// The integer code is Elias-gamma of `n + 1`, so its bit cost is `2⌊log2(n+1)⌋
/// + 1`.
///
/// `0` costs a single bit, and the cost steps up by two at each power-of-two
/// boundary of `n + 1` (`1`/`2` → 3 bits, `6` → 5, `7` → 7). Pinning these
/// widths guards the canonical prefix-code property the byte-equality
/// `Eq`/`Hash` relies on.
#[test]
fn gamma_costs() {
    let cost = |n: u64| {
        let mut bits = Bits::new();
        encode_int(&mut bits, &Base::from(n));
        bits.len()
    };
    assert_eq!(cost(0), 1);
    assert_eq!(cost(1), 3);
    assert_eq!(cost(2), 3);
    assert_eq!(cost(6), 5);
    assert_eq!(cost(7), 7);
}

/// The small inline `Base` representation must spill exactly at the `u64`
/// boundary without changing the arbitrary-width integer codec.
#[test]
fn gamma_roundtrip_just_above_u64_max() {
    let n = Base::from(u64::MAX) + Base::from(1u8);
    let mut bits = Bits::new();
    encode_int(&mut bits, &n);
    let (decoded, pos) = decode_int(&bits, 0).expect("well-formed");
    assert_eq!(decoded, n);
    assert_eq!(decoded.to_string(), "18446744073709551616");
    assert_eq!(pos, bits.len());
}

/// `decode_int` never panics and reports `Truncated` when the code runs off the
/// end (empty input, or all-zeros with no terminating `1`).
#[test]
fn gamma_truncated() {
    let empty = Bits::new();
    assert!(matches!(decode_int(&empty, 0), Err(Decode::Truncated)));
    let zeros: Bits = bitvec![u8, Msb0; 0, 0, 0, 0, 0];
    assert!(matches!(decode_int(&zeros, 0), Err(Decode::Truncated)));
}

// ───────────────── word-window fast paths (differential) ─────────────────
//
// `encode_int`, `decode_int`, and `skip_int` each carry a word-wise fast path
// riding on `gamma::decode_int_window` / `store_be`; the per-bit loop is the
// specification. These tests pin the fast paths to it differentially, with
// generators seeded at the window-edge boundaries (prefix length 31/32 around
// the window's widest provable code, 63/64/65 around the word width, codes
// straddling the window edge, streams ending mid-code) where a window bug
// would hide.

/// The per-bit reference emitter, the encode-side differential oracle:
/// unary prefix then MSB-first mantissa, one push per bit.
fn encode_int_bitwise(out: &mut Bits, n: &Base) {
    let m = n + 1u32;
    let k = m.bits() - 1;
    for _ in 0..k {
        out.push(false);
    }
    for i in (0..=k).rev() {
        out.push(m.bit(i));
    }
}

/// The per-bit reference `skip_int`, the skip-side differential oracle:
/// counts the unary prefix, then steps over the mantissa bit by bit.
fn skip_int_bitwise(bits: &BitsSlice, pos: usize) -> Result<usize, Decode> {
    let mut k = 0usize;
    loop {
        let idx = pos + k;
        if idx >= bits.len() {
            return Err(Decode::Truncated);
        }
        if bits[idx] {
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

/// Assert `decode_int` (windowed) agrees with the pure bit loop at `pos`:
/// same accept/reject, same error variant, same value, same end position.
fn assert_decode_matches_bit_loop(bits: &BitsSlice, pos: usize) -> Result<(), TestCaseError> {
    let subject = decode_int(bits, pos);
    let mut cursor = SliceCursor::new(bits, pos);
    let oracle = decode_int_from(&mut cursor);
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

/// Assert `skip_int` (windowed) agrees with the per-bit reference at `pos` on
/// distance and error variant.
fn assert_skip_matches_bit_loop(bits: &BitsSlice, pos: usize) -> Result<(), TestCaseError> {
    match (skip_int(bits, pos), skip_int_bitwise(bits, pos)) {
        (Ok(s), Ok(o)) => prop_assert_eq!(s, o),
        (Err(s), Err(o)) => {
            prop_assert_eq!(std::mem::discriminant(&s), std::mem::discriminant(&o));
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
fn arb_gamma_stream() -> impl Strategy<Value = (Bits, usize)> {
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
            let mut bits = Bits::new();
            bits.extend(pad);
            for _ in 0..zeros {
                bits.push(false);
            }
            bits.extend(rest);
            (bits, pos)
        })
}

proptest! {
    /// The word-wise `encode_int` is byte-identical to the per-bit emitter.
    ///
    /// Holds for every value — `u64`-range codes (the `store_be` path) and
    /// spilled `Base::Big` values alike — even appending at an unaligned
    /// mid-stream position; and the windowed decoder reads its output back
    /// exactly.
    #[test]
    fn gamma_word_encode_matches_bit_encode(
        prefix in proptest::collection::vec(any::<bool>(), 0..17),
        n in arb_boundary_u64(),
        limbs in proptest::collection::vec(any::<u64>(), 0..3),
    ) {
        let mut value = Base::from(n);
        for limb in limbs {
            value = (value << 64) | Base::from(limb);
        }
        let pos = prefix.len();
        let mut word = Bits::new();
        let mut bit = Bits::new();
        for b in prefix {
            word.push(b);
            bit.push(b);
        }
        encode_int(&mut word, &value);
        encode_int_bitwise(&mut bit, &value);
        prop_assert_eq!(&word, &bit);

        // Word-decode of the word-encode round-trips value and position.
        let (decoded, end) = decode_int(&word, pos).expect("well-formed");
        prop_assert_eq!(decoded, value);
        prop_assert_eq!(end, word.len());
    }
}

proptest! {
    /// On window-boundary streams, windowed `decode_int` and `skip_int`
    /// behave exactly like the per-bit loops.
    ///
    /// Agreement covers accept/reject, error variant, value, and consumed
    /// bits — at the code position, near and past the stream end, and on a
    /// mid-byte re-slice (where the window declines and only the loop runs).
    #[test]
    fn gamma_word_decode_matches_bit_loop(
        (bits, pos) in arb_gamma_stream(),
        extra in 0usize..3,
    ) {
        assert_decode_matches_bit_loop(&bits, pos)?;
        assert_skip_matches_bit_loop(&bits, pos)?;

        // The end of the stream, just before it, and past it.
        assert_decode_matches_bit_loop(&bits, bits.len().saturating_sub(extra))?;
        assert_skip_matches_bit_loop(&bits, bits.len().saturating_sub(extra))?;
        assert_decode_matches_bit_loop(&bits, bits.len() + extra)?;
        assert_skip_matches_bit_loop(&bits, bits.len() + extra)?;

        // A slice whose origin is mid-byte in its backing store.
        if !bits.is_empty() {
            assert_decode_matches_bit_loop(&bits[1..], pos.saturating_sub(1))?;
            assert_skip_matches_bit_loop(&bits[1..], pos.saturating_sub(1))?;
        }
    }
}

proptest! {
    /// On arbitrary raw byte streams — mostly invalid input — the windowed
    /// `decode_int` and `skip_int` agree with the per-bit loops on
    /// accept/reject, error variant, value, and consumed bits at every
    /// position.
    #[test]
    fn gamma_word_paths_match_on_arbitrary_bytes(
        bytes in proptest::collection::vec(any::<u8>(), 0..12),
        pos in 0usize..104,
    ) {
        let bits = bytes_as_bits(&bytes);
        assert_decode_matches_bit_loop(bits, pos)?;
        assert_skip_matches_bit_loop(bits, pos)?;
    }
}

/// The window decoder accepts a code exactly filling its 64 provable bits and
/// declines one bit past that; junk after a code never leaks into the mantissa.
///
/// Prefix `k = 31` (a 63-bit code) is the widest code one window proves and
/// must decode; `k = 32` (65 bits) straddles the window edge and must fall
/// back — where the bit loop still decodes it — as must the 63-bit code cut
/// one bit short of complete.
#[test]
fn gamma_window_edge() {
    use super::gamma::decode_int_window;

    // k = 31: the widest code a 64-bit window proves.
    let n = (1u64 << 31) - 1;
    let mut bits = Bits::new();
    encode_int(&mut bits, &Base::from(n));
    assert_eq!(bits.len(), 63);
    assert_eq!(decode_int_window(&bits, 0), Some((n, 63)));

    // The same code cut one bit short: nothing provable, decline.
    assert_eq!(decode_int_window(&bits[..62], 0), None);

    // k = 32: a 65-bit code straddles the window edge — decline, and the
    // full decoder still reads it through the loop.
    let n = (1u64 << 32) - 1;
    let mut bits = Bits::new();
    encode_int(&mut bits, &Base::from(n));
    assert_eq!(bits.len(), 65);
    assert_eq!(decode_int_window(&bits, 0), None);
    let (decoded, end) = decode_int(&bits, 0).expect("well-formed");
    assert_eq!(decoded, Base::from(n));
    assert_eq!(end, 65);

    // Junk after a short code must not leak into its mantissa.
    let mut bits = Bits::new();
    encode_int(&mut bits, &Base::from(5u64));
    let code_len = bits.len();
    for _ in 0..64 {
        bits.push(true);
    }
    assert_eq!(decode_int_window(&bits, 0), Some((5, code_len)));
}

/// The window decoder never guesses at unprovable input: a slice whose origin
/// is mid-byte, a position at or past the stream end, and an all-zeros
/// (truncated) stream all decline to the bit loop.
#[test]
fn gamma_window_declines_conservatively() {
    use super::gamma::decode_int_window;

    let mut bits = Bits::new();
    bits.push(false);
    bits.push(true);

    // Mid-byte slice origin: no byte view, decline — but the same bit
    // addressed as (whole slice, pos) has one, and the fast path fires.
    assert_eq!(decode_int_window(&bits[1..], 0), None);
    assert_eq!(decode_int_window(&bits, 1), Some((0, 2)));

    // At and past the end of the stream.
    assert_eq!(decode_int_window(&bits, 2), None);
    assert_eq!(decode_int_window(&bits, 7), None);

    // All zeros: no terminating 1 in the stream (bit loop: `Truncated`).
    let zeros = Bits::repeat(false, 70);
    assert_eq!(decode_int_window(&zeros, 0), None);
}

/// A gamma code wide enough to spill machine-word decoding round-trips
/// exactly and remains self-delimiting (the whole mantissa is one spilled
/// value, byte-unaligned on both ends).
#[test]
fn gamma_roundtrip_wide_value() {
    // 2^1000 + 12345: a 1001-bit mantissa with live bits at both ends.
    let n = (Base::from(1u8) << 1000u32) + 12345u64;
    let mut bits = Bits::new();
    encode_int(&mut bits, &n);
    let (decoded, pos) = decode_int(&bits, 0).expect("well-formed");
    assert_eq!(decoded, n);
    assert_eq!(pos, bits.len());
}

/// A stream that ends anywhere inside a wide mantissa is `Truncated`: the
/// wide-decode accept/reject boundary sits exactly at the declared code
/// length, wherever the cut falls relative to byte alignment.
#[test]
fn gamma_truncated_inside_wide_mantissa() {
    let n = (Base::from(1u8) << 1000u32) + 12345u64;
    let mut bits = Bits::new();
    encode_int(&mut bits, &n);
    // Cuts inside the unary prefix, at the leading mantissa 1, just after
    // it, at byte-scale offsets into the mantissa, and one bit short.
    for cut in [1, 500, 1001, 1002, 1009, 1500, bits.len() - 1] {
        let truncated = &bits[..cut];
        assert!(
            matches!(decode_int(truncated, 0), Err(Decode::Truncated)),
            "cut at bit {cut} must report Truncated",
        );
    }
    // The full code still decodes: the cuts, not the value, are the failure.
    let (decoded, pos) = decode_int(&bits, 0).expect("well-formed");
    assert_eq!(decoded, n);
    assert_eq!(pos, bits.len());
}

// ──────────────────── metered Base equality and hashing ────────────────────

/// A mirror of `Base` carrying the compiler-derived `PartialEq`/`Hash`: the
/// semantics of record that `Base`'s manual limb-metered impls must
/// reproduce exactly.
#[derive(PartialEq, Hash)]
enum DerivedBase {
    Small(u64),
    Big(num_bigint::BigUint),
}

impl DerivedBase {
    /// The same value as `b`, carried by the derived-impl mirror.
    fn of(b: &Base) -> DerivedBase {
        match b {
            Base::Small(n) => DerivedBase::Small(*n),
            Base::Big(n) => DerivedBase::Big(n.clone()),
        }
    }
}

/// One value's `DefaultHasher` output, so hash streams can be compared
/// across `Base` and its derived-impl mirror.
fn default_hash<T: std::hash::Hash>(v: &T) -> u64 {
    use std::hash::Hasher;
    let mut h = std::collections::hash_map::DefaultHasher::new();
    v.hash(&mut h);
    h.finish()
}

/// The manual (limb-metered) `PartialEq` and `Hash` on `Base` agree with the
/// compiler-derived semantics over a value grid spanning the `Small`/`Big`
/// boundary.
///
/// Every pairwise equality answer matches the derived impl's, every hash
/// stream matches the derived impl's, and equal values hash equally:
/// metering must never change an answer.
#[test]
fn base_eq_hash_agree_with_derived_semantics() {
    let grid: Vec<Base> = vec![
        Base::ZERO,
        Base::from(1u8),
        Base::from(2u8),
        Base::from(u64::MAX - 1),
        Base::from(u64::MAX),
        // The first spilled value, spelled two ways: an equal `Big` pair.
        Base::from(u64::MAX) + 1u64,
        Base::from(1u128 << 64),
        Base::from(u128::MAX),
        (Base::from(1u8) << 200u32) - &Base::from(1u8),
        Base::from(1u8) << 200u32,
    ];
    for a in &grid {
        assert_eq!(
            default_hash(a),
            default_hash(&DerivedBase::of(a)),
            "hash stream must match the derived impl for {a}"
        );
        for b in &grid {
            assert_eq!(
                a == b,
                DerivedBase::of(a) == DerivedBase::of(b),
                "equality answer must match the derived impl for ({a}, {b})"
            );
            if a == b {
                assert_eq!(
                    default_hash(a),
                    default_hash(b),
                    "equal values must hash equally: ({a}, {b})"
                );
            }
        }
    }
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

// ──────────────────── Clock canonical byte-injectivity ────────────────────

proptest! {
    /// `Clock::encode` is injective on normal forms, asserted *directly* on
    /// `Clock` (which has no `PartialEq`, so the encoding-injectivity property
    /// only reaches it transitively through the harness).
    ///
    /// Two clocks encode to identical bytes **iff** they lower to the same
    /// `(Party, Version)` oracle structure. Both directions matter — equal
    /// structure must produce identical bytes (well-defined canonical
    /// encoding), and *distinct* structure must produce *distinct* bytes
    /// (injectivity, the property byte-equality `Eq`/`Hash` relies on). The
    /// clock encoding is `enc_id(party) ‖ enc_ev(version)` with no padding
    /// between the two halves, so this also pins that the id/event boundary is
    /// unambiguous: a difference in *either* component alone changes the bytes.
    ///
    /// Inputs are arbitrary normal-form trees, so the pairs are genuinely
    /// unrelated structures spanning shapes and base magnitudes the op pipeline
    /// never produces — exactly where a non-injective boundary would hide.
    #[test]
    fn clock_byte_injective_arbitrary(
        pa in arb_oracle_party_nonempty(),
        va in arb_oracle_version(),
        pb in arb_oracle_party_nonempty(),
        vb in arb_oracle_version(),
    ) {
        let a = Clock::from_parts(from_oracle_party(&pa), from_oracle_version(&va));
        let b = Clock::from_parts(from_oracle_party(&pb), from_oracle_version(&vb));

        // Lower through the impl's packed bits, not the source oracle trees, so
        // the structural identity reflects what the impl actually stored
        // (normalized).
        prop_assert_eq!(
            to_oracle_clock(&a) == to_oracle_clock(&b),
            a.encode() == b.encode()
        );
    }
}

proptest! {
    /// The same `Clock` byte-injectivity biconditional over *causally related*
    /// clocks drawn from a seed-derived op trace — the population the protocol
    /// actually produces, complementing the unrelated arbitrary pairs above.
    #[test]
    fn clock_byte_injective_op_trace(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        let a = from_oracle_clock(&cs[i % n]);
        let b = from_oracle_clock(&cs[j % n]);
        prop_assert_eq!(
            to_oracle_clock(&a) == to_oracle_clock(&b),
            a.encode() == b.encode()
        );
    }
}

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
    let denormal = Node(Box::new(Leaf(true)), Box::new(Leaf(true)));
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
    let mut tree = Node(Box::new(Leaf(true)), Box::new(Leaf(true)));
    for _ in 0..DEPTH {
        tree = Node(Box::new(tree), Box::new(Leaf(false)));
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

/// Padding rejection is bit-granular, not byte-granular: a complete tree that
/// ends mid-byte must have *every* remaining bit of that final byte be zero.
///
/// A non-zero bit inside the same byte as the tree (intra-byte padding) is
/// `TrailingBits`, just as a whole spurious trailing byte is. The id leaf `1`
/// encodes to the two-bit terminal tag (`0, 0`) packed as `0000_0000`; setting
/// any padding bit within that byte must be rejected.
#[test]
fn reject_intra_byte_padding() {
    // `Leaf(true)` = the terminal tag bits [0, 0] → one byte 0b0000_0000; bits
    // 2..8 are zero padding.
    let clean = from_oracle_party(&oracle::Party::Leaf(true)).encode();
    assert_eq!(clean.len(), 1, "an id leaf fits in a single byte");
    assert!(Party::decode(&clean[..]).is_ok(), "clean padding decodes");

    // Flip each intra-byte padding bit (positions 2..8) in turn; each is
    // `TrailingBits`.
    for bit in 2u8..8 {
        let mut bytes = clean.clone();
        bytes[0] |= 0b1000_0000u8 >> bit;
        assert!(
            matches!(Party::decode(&bytes[..]), Err(Decode::TrailingBits)),
            "non-zero intra-byte padding at bit {bit} must be rejected",
        );
    }
}

/// An event node with no zero-base child, and a collapsible `(n,m,m)` node, are
/// both non-canonical.
#[test]
fn reject_noncanonical_event() {
    use oracle::Version::{Leaf, Node};

    // No child has base 0: violates the one-child-min-is-zero invariant.
    let no_zero = Node(
        0u64.into(),
        Box::new(Leaf(1u64.into())),
        Box::new(Leaf(2u64.into())),
    );
    let bytes = from_oracle_version(&no_zero).encode();
    assert!(matches!(
        Version::decode(&bytes[..]),
        Err(Decode::NotCanonical)
    ));

    // Two equal-valued leaf children: collapsible to a single integer.
    let collapsible = Node(
        0u64.into(),
        Box::new(Leaf(5u64.into())),
        Box::new(Leaf(5u64.into())),
    );
    let bytes = from_oracle_version(&collapsible).encode();
    assert!(matches!(
        Version::decode(&bytes[..]),
        Err(Decode::NotCanonical)
    ));
}

/// The byte `decode` paths are the only ones that yield a top-level `Party`
/// without passing through `finish_id`; both reject the anonymous identity `0`,
/// so an empty-region `Party`/`Clock` cannot be constructed.
///
/// The paper forbids `event` on an anonymous stamp (§3, `i ≠ 0`), and a
/// standalone party is by definition a nonzero share. In the pruned encoding
/// the anonymous id `0` is the empty bit stream, so a party with no bytes — and
/// a clock whose byte-aligned party prefix is empty — is the anonymous case.
#[test]
fn decode_rejects_anonymous_id() {
    // The anonymous id `0` encodes to no bits at all; as a bare party that is
    // the empty byte stream, rejected as `Anonymous`.
    let anon = from_oracle_party(&oracle::Party::Leaf(false)).encode();
    assert!(anon.is_empty(), "the anonymous id encodes to no bytes");
    assert!(matches!(Party::decode(&anon[..]), Err(Decode::Anonymous)));

    // A clock byte-concatenates its (byte-aligned) party and version. The only
    // empty party prefix is the empty stream, so the anonymous clock is rejected
    // when its party region decodes as anonymous.
    assert!(matches!(Clock::decode(&[][..]), Err(Decode::Anonymous)));
}

/// `Clock::decode` can never yield a clock with an anonymous (`0`) party — the
/// invariant the whole stack rests on (paper §3: a live share is `i ≠ 0`).
///
/// The party is the byte-aligned prefix, `Party::decode` rejects the empty id,
/// and the only empty prefix is the empty stream (itself rejected as
/// `Anonymous`), so an anonymous-party clock has *no* encoding: its bytes (just
/// the version, since the `0` party contributes none) decode to a *different*,
/// non-anonymous clock or fail canonicity — never round-trip back. This sweeps
/// every byte string up to two bytes, where the empty-prefix boundary lives:
/// each either fails to decode or yields a nonzero party, and none panics.
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
    // 0xFF is eight node flags in a row — the tree never bottoms out.
    assert!(matches!(Party::decode(&[0xFF][..]), Err(Decode::Truncated)));
    assert!(matches!(
        Version::decode(&[0xFF][..]),
        Err(Decode::Truncated)
    ));
}

/// A non-zero bit after a complete tree is `TrailingBits`.
#[test]
fn reject_trailing_bits() {
    let mut bytes = from_oracle_party(&oracle::Party::Leaf(true)).encode();
    bytes.push(0x01); // a set bit beyond the (complete) tree and its zero padding
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
// The 256 uniform-random vectors in `decode_never_panics` are a thin panic net:
// truly random bytes almost never form a *nearly*-valid stream, so they barely
// exercise the validator's accept boundary. These tests instead start from a
// *valid* canonical encoding and perturb it minimally — flip one bit, truncate
// at one position — so the mutated input lands right at the edge of the
// accepted language. The contract for every mutation is the same disjunction:
// `decode` either **rejects** (`Err`) or **accepts-canonically** — the accepted
// value lowers to a normal-form oracle tree (the keystone byte-canonicity
// invariant, the thing byte-equality `Eq`/`Hash` rests on) *and* re-encodes to
// exactly the bytes it was decoded from (so the mutated stream was itself the
// canonical encoding of some value). A decode that accepts a non-normal value,
// or one whose re-encode disagrees with its own input, is a major finding.

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
    /// re-encode-stable) — for every bit position and every decoder.
    ///
    /// Single-bit flips are the most targeted mutation: each lands one Hamming
    /// step from the accepted language, where a validator that under-checks
    /// would leak a non-canonical accept.
    ///
    /// Regression guard for the trailing-zero-byte defect (fixed in
    /// `require_zero_padding`): a flip can shift the tree to end on a byte
    /// boundary one byte before the input's end, leaving a spurious all-zero
    /// trailing byte; `decode` now rejects that (a run of `>= 8` trailing bits
    /// is non-canonical even when zero), keeping `decode` injective on bytes.
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
    /// leaf of a clock) — which must then decode canonically, never to a
    /// malformed value.
    ///
    /// Regression guard for the trailing-zero-byte defect (fixed in
    /// `require_zero_padding`): a truncation can cut a valid stream just
    /// *after* a complete tree but still inside one or more trailing zero
    /// bytes; `decode` now rejects that rather than accepting a value that
    /// re-encodes to fewer bytes than its own input.
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

/// WITNESS — the minimal reproduction of the trailing-zero-byte defect that the
/// two mutation proptests above (bit-flip and truncation) surface.
///
/// `pack_to_writer` zero-pads a canonical stream only to the next byte boundary,
/// so a canonical encoding has **at most 7 trailing zero bits**. The original
/// [`require_zero_padding`] (`codec.rs`) only checked that the bits after the
/// tree are all zero — it never bounded how *many* there are, so appending one
/// or more whole `0x00` bytes (≥8 zero bits) was wrongly accepted, making
/// `decode` **non-injective on byte strings** (the accepted value re-encoded to
/// a *shorter* stream than its own input), violating `decode`'s contract
/// ("strictly rejects ... non-canonical input") and the keystone
/// byte-canonicity property. The fix bounds the trailing run: `bits.len() -
/// pos` must be `< 8`. This test is the permanent regression guard.
///
/// `(2, 0, 1)` is the smallest witness: its canonical encoding is the 2 bytes
/// `[180, 128]` (16 bits exactly — no intra-byte padding), so a third `0x00`
/// byte is unambiguously a spurious trailing byte, not padding. A bare party
/// `(1, (0, 1))` = `[196]` exhibits the same with one appended `0x00`.
#[test]
fn trailing_zero_byte_rejected_witness() {
    // Canonical encoding of the event `(2, 0, 1)` is exactly two bytes.
    let canonical = Version::try_from((2u64, 0u64, 1u64)).unwrap().encode();
    assert_eq!(canonical, vec![180, 128], "witness canonical encoding");

    // Appending a whole zero byte must be rejected as TrailingBits — it is NOT
    // padding, because the canonical stream already ended on a byte boundary.
    let mut with_zero = canonical.clone();
    with_zero.push(0);
    assert!(
        matches!(Version::decode(&with_zero[..]), Err(Decode::TrailingBits)),
        "a whole trailing zero byte is non-canonical and must be rejected",
    );

    // The same for an id (party): `(1, (0, 1))` packs to one byte; a second
    // zero byte is spurious.
    let party = "(1, (0, 1))".parse::<Party>().unwrap().encode();
    assert_eq!(party, vec![196], "witness party canonical encoding");
    let mut party_zero = party.clone();
    party_zero.push(0);
    assert!(
        matches!(Party::decode(&party_zero[..]), Err(Decode::TrailingBits)),
        "a whole trailing zero byte on an id must be rejected",
    );
}

proptest! {
    /// The trailing bits of the final byte are zero padding, and setting any
    /// one of them must be rejected (`TrailingBits`), never silently accepted.
    ///
    /// A non-zero padding bit makes the stream non-canonical, which would break
    /// the byte-equality `Eq`/`Hash` contract. The whole-byte and intra-byte
    /// cases are pinned by hand in the canonical-rejection suite; this sweeps
    /// every padding position over arbitrary trees.
    #[test]
    fn padding_perturbation_rejects(pa in arb_oracle_party_nonempty()) {
        let party = from_oracle_party(&pa);
        let valid = party.encode();

        // Number of meaningful bits = bit length of the packed id with no
        // trailing padding.
        let used = party.as_bits().len();
        let total = valid.len() * 8;
        for pad in used..total {
            let (byte, bit) = (pad / 8, (pad % 8) as u8);
            let mut m = valid.clone();
            m[byte] |= 0b1000_0000u8 >> bit;
            prop_assert!(
                matches!(Party::decode(&m[..]), Err(Decode::TrailingBits)),
                "non-zero padding bit at position {pad} must be rejected",
            );
        }
    }
}

// ───────────────────────────── parse stacks ─────────────────────────────

/// Trees deeper than the parsers' inline stack capacity spill to the heap
/// with behavior unchanged: they still validate, encode, and round-trip
/// exactly.
///
/// The tree parsers keep their explicit stacks in [`PARSE_STACK_INLINE`]
/// inline frames; a deeper tree moves the frames to the heap mid-parse. A
/// right-spine event tree and a left-spine id tree three times that depth
/// cross the spill boundary in both parsers (and, since the spill happens
/// while ancestors are still open, the spilled frames must survive to
/// complete the normal-form checks on the way back up).
#[test]
fn parse_stacks_spill_past_inline_capacity() {
    const DEPTH: usize = 3 * PARSE_STACK_INLINE;

    // Event tree: a right spine `(1, 0, (1, 0, … 2))`. Every node has a
    // base-0 left leaf, and the innermost pair of leaves differ, so the
    // whole spine is canonical.
    let mut spine = String::from("2");
    for _ in 0..DEPTH {
        spine = format!("(1, 0, {spine})");
    }
    let version: Version = spine.parse().expect("a deep right spine is canonical");
    assert_eq!(
        Version::decode(&version.encode()[..]).expect("deep event tree decodes"),
        version,
    );

    // Id tree: a left spine, one frame per level in `parse_id_from`.
    let party = deep_left_spine_party(DEPTH);
    assert_eq!(
        Party::decode(&party.encode()[..]).expect("deep id tree decodes"),
        party,
    );
}
