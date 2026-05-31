//! Codec tests: round-trip, canonical
//! injectivity, and strict rejection of malformed / non-canonical input.
//!
//! Impl values are built from oracle trees via the bridge (canonical bits emitted
//! directly), so these test the *codec* in isolation from the operation algorithms.

use bitvec::prelude::*;
use proptest::prelude::*;

use super::{decode_int, encode_int, Base, Bits};
use crate::oracle;
use crate::test_support::{
    from_oracle_clock, from_oracle_party, from_oracle_version, run, versions, world_strategy,
};
use crate::{Clock, DecodeError, Party, Version};

// ───────────────────────────── integer code ─────────────────────────────

proptest! {
    /// `decode_int ∘ encode_int == id`, and the code is self-delimiting (consumes
    /// exactly the bits it wrote).
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
    /// The integer code round-trips arbitrary-width magnitudes with no cap: a value
    /// built from many random `u64` limbs (well past `u64::MAX`) survives
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

/// The Elias-gamma-of-`n+1` bit costs match the plan's table; `0` is a single bit.
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

/// `decode_int` never panics and reports `Truncated` when the code runs off the end
/// (empty input, or all-zeros with no terminating `1`).
#[test]
fn gamma_truncated() {
    let empty = Bits::new();
    assert!(matches!(decode_int(&empty, 0), Err(DecodeError::Truncated)));
    let zeros: Bits = bitvec![u8, Msb0; 0, 0, 0, 0, 0];
    assert!(matches!(decode_int(&zeros, 0), Err(DecodeError::Truncated)));
}

// ───────────────────────── A1: round-trip ─────────────────────────

proptest! {
    /// A1. `decode(encode(x)) == x` for `Party`, `Version`, and `Clock`.
    #[test]
    fn a1_roundtrip(ops in world_strategy(), i in 0usize..64) {
        let cs = run(&ops);
        let n = cs.len();
        let oc = &cs[i % n];
        let (op, ov) = oc.trees();

        let party = from_oracle_party(op);
        prop_assert!(Party::decode(&party.encode()).expect("valid") == party);

        let version = from_oracle_version(ov);
        prop_assert!(Version::decode(&version.encode()).expect("valid") == version);

        let clock = from_oracle_clock(oc);
        let clock2 = Clock::decode(&clock.encode()).expect("valid");
        prop_assert!(clock.party() == clock2.party());
        prop_assert!(clock.version() == clock2.version());
    }
}

// ───────────────────────── A2: canonical injectivity ─────────────────────────

proptest! {
    /// A2. `a == b` ⇔ `encode(a) == encode(b)`; equality also matches the oracle's
    /// (encode is injective on normal forms).
    #[test]
    fn a2_canonical(ops in world_strategy(), i in 0usize..64, j in 0usize..64) {
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

// ───────────────────────── A3: rejection ─────────────────────────

/// A3. A collapsible id node `(v, v)` is non-canonical.
#[test]
fn a3_reject_noncanonical_id() {
    use oracle::Party::{Leaf, Node};
    let denormal = Node(Box::new(Leaf(true)), Box::new(Leaf(true)));
    let bytes = from_oracle_party(&denormal).encode();
    assert!(matches!(
        Party::decode(&bytes),
        Err(DecodeError::NotCanonical)
    ));
}

/// A3. An event node with no zero-base child, and a collapsible `(n,m,m)` node, are
/// both non-canonical.
#[test]
fn a3_reject_noncanonical_event() {
    use oracle::Version::{Leaf, Node};

    // No child has base 0: violates the one-child-min-is-zero invariant.
    let no_zero = Node(0, Box::new(Leaf(1)), Box::new(Leaf(2)));
    let bytes = from_oracle_version(&no_zero).encode();
    assert!(matches!(
        Version::decode(&bytes),
        Err(DecodeError::NotCanonical)
    ));

    // Two equal-valued leaf children: collapsible to a single integer.
    let collapsible = Node(0, Box::new(Leaf(5)), Box::new(Leaf(5)));
    let bytes = from_oracle_version(&collapsible).encode();
    assert!(matches!(
        Version::decode(&bytes),
        Err(DecodeError::NotCanonical)
    ));
}

/// PAP-2. The byte `decode` paths are the only ones that yield a top-level `Party`
/// without passing through `finish_id`; both reject the anonymous identity `0` (the
/// empty id region), so an empty-region `Party`/`Clock` cannot be constructed. The
/// paper forbids `event` on an anonymous stamp (§3, `i ≠ 0`), and a standalone party
/// is by definition a nonzero share.
#[test]
fn pap2_decode_rejects_anonymous_id() {
    // The single `0` leaf is the only canonical empty id; encode it as a bare party.
    let anon = from_oracle_party(&oracle::Party::Leaf(false)).encode();
    assert!(matches!(Party::decode(&anon), Err(DecodeError::Anonymous)));

    // The same id as a clock's party region (id `0`, event `0`) must also be rejected.
    let anon_clock = from_oracle_clock(&oracle::Clock::from_parts(
        oracle::Party::Leaf(false),
        oracle::Version::Leaf(0),
    ))
    .encode();
    assert!(matches!(
        Clock::decode(&anon_clock),
        Err(DecodeError::Anonymous)
    ));
}

/// A3. A stream that ends mid-tree is `Truncated`.
#[test]
fn a3_reject_truncated() {
    // 0xFF is eight node flags in a row — the tree never bottoms out.
    assert!(matches!(
        Party::decode(&[0xFF]),
        Err(DecodeError::Truncated)
    ));
    assert!(matches!(
        Version::decode(&[0xFF]),
        Err(DecodeError::Truncated)
    ));
}

/// A3. A non-zero bit after a complete tree is `TrailingBits`.
#[test]
fn a3_reject_trailing_bits() {
    let mut bytes = from_oracle_party(&oracle::Party::Leaf(true)).encode();
    bytes.push(0x01); // a set bit beyond the (complete) tree and its zero padding
    assert!(matches!(
        Party::decode(&bytes),
        Err(DecodeError::TrailingBits)
    ));

    let mut bytes = from_oracle_version(&oracle::Version::Leaf(0)).encode();
    bytes.push(0x80);
    assert!(matches!(
        Version::decode(&bytes),
        Err(DecodeError::TrailingBits)
    ));
}
