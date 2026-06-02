//! Codec tests: round-trip, canonical injectivity, and strict rejection of
//! malformed / non-canonical input.
//!
//! Impl values are built from oracle trees via the bridge (canonical bits
//! emitted directly), so these test the *codec* in isolation from the operation
//! algorithms.

use bitvec::prelude::*;
use proptest::prelude::*;

use super::{decode_int, encode_int, Base, Bits};
use crate::oracle;
use crate::testing::bridge::{
    from_oracle_clock, from_oracle_party, from_oracle_version, to_oracle_clock, to_oracle_party,
    to_oracle_version,
};
use crate::testing::generators::{arb_oracle_party_nonempty, arb_oracle_version};
use crate::testing::optrace::{run, versions, world_strategy};
use crate::{Clock, DecodeError, Party, Version};

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
    fn gamma_roundtrip_wide(limbs in proptest::collection::vec(any::<u64>(), 1..8)) {
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
/// + 1`: `0` costs a single bit, and the cost steps up by two at each
/// power-of-two boundary of `n + 1` (`1`/`2` → 3 bits, `6` → 5, `7` → 7).
/// Pinning these widths guards the canonical prefix-code property the
/// byte-equality `Eq`/`Hash` relies on.
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
    assert!(matches!(decode_int(&empty, 0), Err(DecodeError::Truncated)));
    let zeros: Bits = bitvec![u8, Msb0; 0, 0, 0, 0, 0];
    assert!(matches!(decode_int(&zeros, 0), Err(DecodeError::Truncated)));
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
    /// here a `BufWriter`, a distinct buffered `Write` impl. For `Clock` this
    /// exercises the streamed id/event boundary (the partial byte merged across
    /// the two component streams with no combined buffer).
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
    /// only reaches it transitively through the harness): two clocks encode to
    /// identical bytes **iff** they lower to the same `(Party, Version)` oracle
    /// structure. Both directions matter — equal structure must produce
    /// identical bytes (well-defined canonical encoding), and *distinct*
    /// structure must produce *distinct* bytes (injectivity, the property
    /// byte-equality `Eq`/`Hash` relies on). The clock encoding is
    /// `enc_id(party) ‖ enc_ev(version)` with no padding between the two
    /// halves, so this also pins that the id/event boundary is unambiguous: a
    /// difference in *either* component alone changes the bytes.
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

/// A collapsible id node `(v, v)` is non-canonical, for *both* leaf values: a
/// `(1, 1)` node collapses to `1`, and a `(0, 0)` node to `0`. Both cases are
/// checked; the `(0, 0)` case is distinct because `0`-leaf children also form
/// the anonymous share, so it must be rejected as `NotCanonical` (a collapsible
/// node) rather than slipping through to the `Anonymous` empty-id check.
#[test]
fn reject_noncanonical_id() {
    use oracle::Party::{Leaf, Node};
    for v in [false, true] {
        let denormal = Node(Box::new(Leaf(v)), Box::new(Leaf(v)));
        let bytes = from_oracle_party(&denormal).encode();
        assert!(
            matches!(Party::decode(&bytes[..]), Err(DecodeError::NotCanonical)),
            "collapsible id node ({v}, {v}) must be rejected as NotCanonical",
        );
    }
}

/// The id validator runs bottom-up by recursion, so a collapsible `(v,
/// v)` node buried under deep, otherwise-canonical nesting must still be caught
/// — the `NotCanonical` check fires when *any* node completes, not only at the
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
        Err(DecodeError::NotCanonical)
    ));
}

/// Padding rejection is bit-granular, not byte-granular: a complete tree that
/// ends mid-byte must have *every* remaining bit of that final byte be zero. A
/// non-zero bit inside the same byte as the tree (intra-byte padding) is
/// `TrailingBits`, just as a whole spurious trailing byte is. The id leaf `1`
/// encodes to two bits (`0, 1`) packed as `0100_0000`; setting any padding bit
/// within that byte must be rejected.
#[test]
fn reject_intra_byte_padding() {
    // `Leaf(true)` = bits [0, 1] → one byte 0b0100_0000; bits 2..8 are zero
    // padding.
    let clean = from_oracle_party(&oracle::Party::Leaf(true)).encode();
    assert_eq!(clean.len(), 1, "an id leaf fits in a single byte");
    assert!(Party::decode(&clean[..]).is_ok(), "clean padding decodes");

    // Flip each intra-byte padding bit (positions 2..8) in turn; each is
    // `TrailingBits`.
    for bit in 2u8..8 {
        let mut bytes = clean.clone();
        bytes[0] |= 0b1000_0000u8 >> bit;
        assert!(
            matches!(Party::decode(&bytes[..]), Err(DecodeError::TrailingBits)),
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
        Err(DecodeError::NotCanonical)
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
        Err(DecodeError::NotCanonical)
    ));
}

/// The byte `decode` paths are the only ones that yield a top-level `Party`
/// without passing through `finish_id`; both reject the anonymous identity `0`
/// (the empty id region), so an empty-region `Party`/`Clock` cannot be
/// constructed. The paper forbids `event` on an anonymous stamp (§3, `i ≠ 0`),
/// and a standalone party is by definition a nonzero share.
#[test]
fn decode_rejects_anonymous_id() {
    // The single `0` leaf is the only canonical empty id; encode it as a bare
    // party.
    let anon = from_oracle_party(&oracle::Party::Leaf(false)).encode();
    assert!(matches!(
        Party::decode(&anon[..]),
        Err(DecodeError::Anonymous)
    ));

    // The same id as a clock's party region (id `0`, event `0`) must also be
    // rejected.
    let anon_clock = from_oracle_clock(&oracle::Clock::from_parts(
        oracle::Party::Leaf(false),
        oracle::Version::new(),
    ))
    .encode();
    assert!(matches!(
        Clock::decode(&anon_clock[..]),
        Err(DecodeError::Anonymous)
    ));
}

/// A stream that ends mid-tree is `Truncated`.
#[test]
fn reject_truncated() {
    // 0xFF is eight node flags in a row — the tree never bottoms out.
    assert!(matches!(
        Party::decode(&[0xFF][..]),
        Err(DecodeError::Truncated)
    ));
    assert!(matches!(
        Version::decode(&[0xFF][..]),
        Err(DecodeError::Truncated)
    ));
}

/// A non-zero bit after a complete tree is `TrailingBits`.
#[test]
fn reject_trailing_bits() {
    let mut bytes = from_oracle_party(&oracle::Party::Leaf(true)).encode();
    bytes.push(0x01); // a set bit beyond the (complete) tree and its zero padding
    assert!(matches!(
        Party::decode(&bytes[..]),
        Err(DecodeError::TrailingBits)
    ));

    let mut bytes = from_oracle_version(&oracle::Version::new()).encode();
    bytes.push(0x80);
    assert!(matches!(
        Version::decode(&bytes[..]),
        Err(DecodeError::TrailingBits)
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
    /// re-encode-stable) — for every bit position and every decoder. Single-bit
    /// flips are the most targeted mutation: each lands one Hamming step from
    /// the accepted language, where a validator that under-checks would leak a
    /// non-canonical accept.
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
    /// `decode` rejects or accepts canonically. A prefix of a complete tree is
    /// almost always `Truncated`, but a prefix can occasionally itself be a
    /// complete smaller tree (e.g. the leading id leaf of a clock) — which must
    /// then decode canonically, never to a malformed value.
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
/// leaf `(1, (0, 1))` = `[177]` exhibits the same with one appended `0x00`.
#[test]
fn trailing_zero_byte_rejected_witness() {
    // Canonical encoding of the event `(2, 0, 1)` is exactly two bytes.
    let canonical = Version::try_from((2u64, 0u64, 1u64)).unwrap().encode();
    assert_eq!(canonical, vec![180, 128], "witness canonical encoding");

    // Appending a whole zero byte must be rejected as TrailingBits — it is NOT
    // padding, because the canonical stream already ended on a byte boundary.
    let mut with_zero = canonical.clone();
    with_zero.push(0);
    assert_eq!(
        Version::decode(&with_zero[..]),
        Err(DecodeError::TrailingBits),
        "a whole trailing zero byte is non-canonical and must be rejected",
    );

    // The same for an id (party): `(1, (0, 1))` packs to one byte; a second
    // zero byte is spurious.
    let party = "(1, (0, 1))".parse::<Party>().unwrap().encode();
    assert_eq!(party, vec![177], "witness party canonical encoding");
    let mut party_zero = party.clone();
    party_zero.push(0);
    assert_eq!(
        Party::decode(&party_zero[..]),
        Err(DecodeError::TrailingBits),
        "a whole trailing zero byte on an id must be rejected",
    );
}

proptest! {
    /// The trailing bits of the final byte are zero padding. Setting any one of
    /// them must be rejected (`TrailingBits`) — never silently accepted —
    /// because a non-zero padding bit makes the stream non-canonical, which
    /// would break the byte-equality `Eq`/`Hash` contract. The whole-byte and
    /// intra-byte cases are pinned by hand in the canonical-rejection suite;
    /// this sweeps every padding position over arbitrary trees.
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
                matches!(Party::decode(&m[..]), Err(DecodeError::TrailingBits)),
                "non-zero padding bit at position {pad} must be rejected",
            );
        }
    }
}
