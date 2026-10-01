//! Canonical acceptance and rejection tests for Version encoding.
//!
//! The tests exercise both sides of the codec. Generated canonical streams must
//! validate and round-trip exactly. The rejection corpus includes collapsible
//! siblings and other alternative spellings of the same value. Mutation tests
//! independently reconstruct every accepted value through the oracle, ensuring
//! that the decoder accepts only its unique canonical spelling.

use std::collections::BTreeSet;

use proptest::prelude::*;

use num_bigint::{BigUint, Sign};

use crate::bits::{BitRead, BitsReader, BitsWriter};
use crate::error::Decode;
use crate::testing::bridge::{from_oracle_version, to_oracle_version};
use crate::testing::exhaustive::{all_normal_events, EV_SMALL_DEPTH};
use crate::testing::meter::registry::Shape;
use crate::testing::meter::Encoding;
use crate::testing::oracles::tree;
use crate::testing::version::zigzag_difference;
use crate::testing::{generators, optrace};
use crate::{Clock, Version};

use crate::version::io::regions::PayloadKind;

/// Decode one payload code from mutable test storage.
fn decode_gamma(bits: &BitsWriter, position: u64) -> (BigUint, u64) {
    let frozen = bits.clone().finalize();
    let mut reader = BitsReader::at(&frozen, position);
    let value = reader.read_gamma().expect("test payload is complete");
    (value, reader.position())
}
use crate::version::io::validate::whole;

/// Validate one live bit stream and adopt it as canonical Version storage.
fn decode_stream(bits: BitsWriter) -> Result<Version, Decode> {
    whole(bits.reader())?;
    Ok(Version::from_test_bits(bits))
}

/// Lift a meter-generated encoded shape into a [`Version`].
fn version_of(p: &Encoding) -> Version {
    p.version()
}

/// A Version's stored stream as live bits.
fn stream_of(v: &Version) -> BitsWriter {
    crate::version::instrument::bits(v)
}

// ─── hand-pinned streams ────────────────────────────────────────────────────

/// The empty version is the leaf 0: topology `1` (a leaf) plus `gamma(0)`, the
/// two-bit stream `11`, and it round-trips.
#[test]
fn empty_version_is_the_two_bit_stream() {
    let v = Version::new();
    let bits = stream_of(&v);
    assert_eq!(bits.len(), 2);
    assert!(bits.bit(0), "a leaf's topology flag is 1");
    assert!(bits.bit(1), "gamma(0) is the single bit 1");
    assert_eq!(decode_stream(bits).expect("canonical"), v);
}

/// One fork `(1, 0, 2)` codes as hand-derived: 3 topology bits, `gamma(1)` for
/// the first leaf (height 1), and `zigzag(+2) = 4 -> gamma(4)` for the second
/// (height 3), 11 bits total — and round-trips.
#[test]
fn one_fork_matches_hand_derivation() {
    let v = from_oracle_version(&tree::Version::node(
        1u64,
        tree::Version::leaf(0u64),
        tree::Version::leaf(2u64),
    ));
    let bits = stream_of(&v);
    // Preorder: internal root, leaf(gamma(1) = 010), leaf(gamma(4) = 00101).
    let expected: Vec<bool> = [
        false, // root: internal (flag 0)
        true, false, true, false, // left leaf: flag 1, gamma(1)
        true, false, false, true, false, true, // right leaf: flag 1, gamma(4)
    ]
    .to_vec();
    assert_eq!(
        (0..bits.len()).map(|i| bits.bit(i)).collect::<Vec<_>>(),
        expected
    );
    assert_eq!(decode_stream(bits).expect("canonical"), v);
}

// ─── the strict-reject corpus ───────────────────────────────────────────────

/// Append a leaf carrying a raw payload value (the caller pre-zigzags).
fn push_leaf(bits: &mut BitsWriter, payload: u64) {
    bits.push(true);
    bits.write_gamma(&BigUint::from(payload));
}

/// An internal node whose two leaf children carry a zero right delta is
/// the collapsible pair: rejected as [`Decode::NotCanonical`].
#[test]
fn rejects_zero_right_sibling_delta() {
    // (5, 5): internal root, leaf height 5, then delta 0.
    let mut bits = BitsWriter::new();
    bits.push(false); // root: internal
    push_leaf(&mut bits, 5); // gamma(5): the first leaf, absolute
    push_leaf(&mut bits, 0); // zigzag(0) = 0 -> gamma(0): equal sibling
    assert!(matches!(whole(bits.reader()), Err(Decode::NotCanonical)));
}

/// A collapsible sibling pair whose closing ancestor is NOT the root —
/// root(leaf 5, node(leaf 5, leaf 5)), the pair closing one level down — is
/// rejected as [`Decode::NotCanonical`].
///
/// This ensures validation checks every completed sibling pair, not only the
/// root's children. Accepting the stream would give the constant value 5 a
/// second encoding, making byte equality disagree with value equality.
#[test]
fn rejects_non_root_collapsible_pair() {
    let mut bits = BitsWriter::new();
    bits.push(false); // root: internal
    push_leaf(&mut bits, 5); // gamma(5): the first leaf, absolute
    bits.push(false); // right child: internal
    push_leaf(&mut bits, 0); // zigzag(0): leaf 5 again, non-sibling (legal)
    push_leaf(&mut bits, 0); // zigzag(0): its equal sibling — the pair
    assert!(matches!(whole(bits.reader()), Err(Decode::NotCanonical)));
}

/// The `(flag, end)` bit positions of every leaf code in a stored stream: the
/// leaf's topology flag and the position just past its payload code, in
/// preorder.
fn leaf_code_ranges(bits: &BitsWriter) -> Vec<(u64, u64)> {
    let mut out = Vec::new();
    let mut pos = 0u64;
    let mut pending = 1usize;
    while pending > 0 {
        pending -= 1;
        let leaf = bits.bit(pos);
        pos += 1;
        if !leaf {
            pending += 2;
            continue;
        }
        let (_, next) = decode_gamma(bits, pos);
        out.push((pos - 1, next));
        pos = next;
    }
    assert_eq!(pos, bits.len(), "a stored stream is exactly one tree");
    out
}

proptest! {
    /// Splitting any single leaf of a canonical stream into an equal-sibling
    /// zero-delta pair — the collapsible pair, planted at a proptest-chosen
    /// preorder position — is rejected as [`Decode::NotCanonical`].
    ///
    /// Generalizes the hand-pinned pair rejects beyond the root-sibling
    /// position: the planted node closes at an arbitrary ancestor depth, so a
    /// validator weakened to judge pairs only at particular closes reads red
    /// here. The suffix rides verbatim — the planted sibling's zero delta
    /// leaves the running height unchanged, so every later delta stays valid
    /// and the stream stays well-formed; only canonicality breaks.
    #[test]
    fn planted_collapsible_pairs_are_rejected_at_every_leaf(
        t in generators::arb_oracle_version(),
        leaf_seed in any::<prop::sample::Index>(),
    ) {
        let bits = stream_of(&from_oracle_version(&t));
        let leaves = leaf_code_ranges(&bits);
        let (flag, end) = leaves[leaf_seed.index(leaves.len())];
        let mut planted = BitsWriter::with_capacity(bits.len() + 4);
        planted.splice_writer(&bits, 0, flag);
        planted.push(false); // the chosen leaf's position becomes internal
        planted.splice_writer(&bits, flag, end); // left child: the old leaf
        push_leaf(&mut planted, 0); // right child: zigzag(0), the equal sibling
        planted.splice_writer(&bits, end, bits.len());
        prop_assert!(matches!(whole(planted.reader()), Err(Decode::NotCanonical)));
    }
}

/// A zero delta between non-sibling consecutive leaves is a canonical shape:
/// two equal plateaus across a subtree boundary validate and round-trip to the
/// expected version.
#[test]
fn accepts_zero_delta_across_a_subtree_boundary() {
    // (0, (0, 0, 1), 1): preorder leaves 0, 1, 1 — the second delta is 0, legal
    // because the leaves flank a subtree boundary.
    let expected = from_oracle_version(&tree::Version::node(
        0u64,
        tree::Version::node(0u64, tree::Version::leaf(0u64), tree::Version::leaf(1u64)),
        tree::Version::leaf(1u64),
    ));
    let mut bits = BitsWriter::new();
    bits.push(false); // root: internal
    bits.push(false); // left child: internal
    push_leaf(&mut bits, 0); // leaf 0: gamma(0), absolute
    push_leaf(&mut bits, 2); // leaf 1: zigzag(+1) = 2
    push_leaf(&mut bits, 0); // leaf 1 again: zigzag(0) = 0, non-sibling
    assert!(whole(bits.reader()).is_ok());
    assert_eq!(decode_stream(bits.clone()).expect("canonical"), expected);
    assert_eq!(stream_of(&expected), bits);
}

/// A delta that drives the running leaf height negative is rejected as
/// [`Decode::NotCanonical`]: leaf heights are naturals, and no topology bit can
/// see this — only the running value state can.
#[test]
fn rejects_negative_running_height() {
    // (1, -1): internal root, leaf height 1, then delta -2.
    let mut bits = BitsWriter::new();
    bits.push(false); // root: internal
    push_leaf(&mut bits, 1); // first leaf: height 1
    push_leaf(&mut bits, 3); // zigzag(-2) = 3: height would be -1
    assert!(matches!(whole(bits.reader()), Err(Decode::NotCanonical)));
}

/// A negative excursion is rejected even when later deltas would climb back up:
/// validity is per prefix, not per total.
#[test]
fn rejects_negative_height_midstream() {
    // Root over leaf(1) and (node over leaf(-1), leaf(5)): the middle leaf
    // dips negative before the last one recovers.
    let mut bits = BitsWriter::new();
    bits.push(false); // root: internal
    push_leaf(&mut bits, 1); // first leaf: height 1
    bits.push(false); // right child: internal
    push_leaf(&mut bits, 3); // zigzag(-2) = 3: height -1, invalid here
    push_leaf(&mut bits, 12); // zigzag(+6) = 12: would recover to 5
    assert!(matches!(whole(bits.reader()), Err(Decode::NotCanonical)));
}

/// Every proper prefix of a valid stream is rejected as [`Decode::Truncated`],
/// swept over every cut point of several shapes: the coding is self-delimiting,
/// so no prefix is a complete tree.
#[test]
fn rejects_every_truncation() {
    let shapes: Vec<Version> = vec![
        Version::new(),
        version_of(&Shape::Dense.build1(3)),
        version_of(&Shape::CliffComb.build2(4, 3)),
        version_of(&Shape::Hugeleaf.build1(9)),
        version_of(&Shape::AltSpine.build1(4)),
    ];
    for v in &shapes {
        let bits = stream_of(v);
        for cut in 0..bits.len() {
            assert!(
                matches!(
                    whole(BitsReader::with_len(bits.as_raw_slice(), cut)),
                    Err(Decode::Truncated)
                ),
                "a {cut}-bit prefix of a {}-bit stream must read as truncated",
                bits.len(),
            );
        }
    }
}

/// Live bits after a complete tree are rejected as [`Decode::TrailingBits`]:
/// one zero bit, one set bit, or a whole second tree.
#[test]
fn rejects_trailing_bits() {
    let v = version_of(&Shape::Dense.build1(3));
    let clean = stream_of(&v);
    for extra in [false, true] {
        let mut bits = clean.clone();
        bits.push(extra);
        assert!(matches!(whole(bits.reader()), Err(Decode::TrailingBits)));
    }
    let mut two_trees = clean.clone();
    two_trees.extend_from_writer(&clean);
    assert!(matches!(
        whole(two_trees.reader()),
        Err(Decode::TrailingBits)
    ));
}

/// The zigzag map is a bijection with no negative-zero spelling, checked
/// exhaustively at small scope.
///
/// This is why the reject corpus has no "non-canonical zigzag" member — the
/// class is empty by construction, as is non-minimal gamma (a prefix code with
/// one spelling per natural).
#[test]
fn zigzag_is_a_bijection_without_negative_zero() {
    let mut seen: BTreeSet<(bool, u64)> = BTreeSet::new();
    for m in 0..=100u64 {
        let (sign, magnitude) = PayloadKind::Delta.decode(BigUint::from(m)).into_parts();
        let mag = u64::try_from(&magnitude).expect("small codes decode to small magnitudes");
        assert!(
            !(sign == Sign::Minus && mag == 0),
            "no code may spell a negative zero"
        );
        assert!(
            seen.insert((sign == Sign::Minus, mag)),
            "two codes decoded to one delta: the map is not injective"
        );
        // Re-encode through the encoder's map: the round-trip pins the two
        // helpers as mutual inverses over the same sign convention.
        let (prev, cur) = if sign == Sign::Minus {
            (BigUint::from(mag), BigUint::ZERO)
        } else {
            (BigUint::ZERO, BigUint::from(mag))
        };
        assert_eq!(zigzag_difference(&prev, &cur), BigUint::from(m));
    }
}

/// Magnitudes spanning inline words and arbitrary-width digit sequences.
fn arb_signed_magnitude() -> impl Strategy<Value = BigUint> {
    prop_oneof![
        any::<u64>().prop_map(BigUint::from),
        prop::collection::vec(any::<u32>(), 0..8).prop_map(BigUint::new),
        (0u32..=255, -1i8..=1).prop_map(|(shift, offset)| {
            let power = BigUint::from(1u8) << shift;
            match offset {
                -1 if shift > 0 => power - 1u8,
                1 => power + 1u8,
                _ => power,
            }
        }),
    ]
}

proptest! {
    /// Zigzag interpretation and adjacent-height encoding agree at every tested width.
    #[test]
    fn signed_payload_mapping_round_trips(
        magnitude in arb_signed_magnitude(),
        negative in any::<bool>(),
    ) {
        let sign = if negative && magnitude != BigUint::ZERO {
            Sign::Minus
        } else {
            Sign::Plus
        };
        let expected = num_bigint::BigInt::from_biguint(sign, magnitude.clone());
        let code = if sign == Sign::Minus {
            (&magnitude << 1u32) - 1u32
        } else {
            &magnitude << 1u32
        };
        prop_assert_eq!(PayloadKind::Delta.decode(code.clone()), expected);

        let (previous, current) = if sign == Sign::Minus {
            (magnitude, BigUint::ZERO)
        } else {
            (BigUint::ZERO, magnitude)
        };
        prop_assert_eq!(zigzag_difference(&previous, &current), code);
    }
}

// ─── the topology-flag bijection ────────────────────────────────────────────
//
// Stored versions use `0` for an internal node and `1` for a leaf. Test
// generators use the opposite flags. An independent encoder writes the stored
// leaf values with the generator convention; flipping only its node flags must
// then reproduce the stored stream byte for byte. This verifies that conversion
// changes no payload or tree boundary.

/// Encode an oracle tree with generator node flags and stored leaf values.
///
/// The first leaf is absolute and later leaves are changes from their
/// predecessor, just as in a stored version.
fn inverted_flag_stream(t: &tree::Version) -> BitsWriter {
    fn walk(t: &tree::Version, offset: &BigUint, prev: &mut Option<BigUint>, out: &mut BitsWriter) {
        match t {
            tree::Version::Leaf(n) => {
                out.push(false); // leaf flag, inverted spelling
                let height = offset + n;
                match prev.replace(height.clone()) {
                    None => out.write_gamma(&height),
                    Some(p) => out.write_gamma(&zigzag_difference(&p, &height)),
                }
            }
            tree::Version::Node(n, l, r) => {
                out.push(true); // internal flag, inverted spelling
                let offset = offset + n;
                walk(l, &offset, prev, out);
                walk(r, &offset, prev, out);
            }
        }
    }
    let mut out = BitsWriter::new();
    walk(t, &BigUint::ZERO, &mut None, &mut out);
    out
}

/// Invert each node flag while copying every leaf code unchanged.
///
/// `internal` identifies the input convention. Following that convention is
/// enough to find every node and payload boundary without decoding the values.
fn flip_topology_flags(bits: &BitsWriter, internal: bool) -> BitsWriter {
    let mut out = BitsWriter::with_capacity(bits.len());
    let mut pos = 0u64;
    let mut pending = 1usize;
    while pending > 0 {
        pending -= 1;
        let flag = bits.bit(pos);
        out.push(!flag);
        pos += 1;
        if flag == internal {
            pending += 2;
            continue;
        }
        let (_, next) = decode_gamma(bits, pos);
        out.splice_writer(bits, pos, next);
        pos = next;
    }
    assert_eq!(pos, bits.len(), "the transcode consumes exactly one tree");
    out
}

/// Assert the bijection on one tree: inverting the inverted-flag spelling's
/// topology bits yields exactly the stored stream, and inverting the stored
/// stream's yields the inverted spelling back.
fn assert_flag_bijection(t: &tree::Version) {
    let stored = stream_of(&from_oracle_version(t));
    let inverted = inverted_flag_stream(t);
    assert_eq!(
        flip_topology_flags(&inverted, true),
        stored,
        "flipping the inverted spelling's topology flags must land on the stored stream"
    );
    assert_eq!(
        flip_topology_flags(&stored, false),
        inverted,
        "flipping the stored stream's topology flags must land on the inverted spelling"
    );
}

/// The flag bijection holds across the registered input families, wide (the gamma
/// wide arm) and deep (long unary runs) members included.
#[test]
fn topology_flag_bijection_on_generator_families() {
    for p in [
        Shape::Dense.build1(1),
        Shape::Dense.build1(1_000),
        Shape::Bigroot.build2(1_000, 200),
        Shape::Hugeleaf.build1(5_000),
        Shape::CliffComb.build2(64, 64),
        Shape::WideToothComb.build3(512, 192, 64),
        Shape::AltSpine.build1(64),
        Shape::CancellingChain.build2(64, 64),
    ] {
        assert_flag_bijection(&to_oracle_version(&version_of(&p)));
    }
    assert_flag_bijection(&to_oracle_version(&Version::new()));
}

proptest! {
    /// The flag bijection holds on arbitrary normal-form trees (magnitudes past
    /// `u64::MAX` included) and on every version of an organic history.
    #[test]
    fn topology_flag_bijection_on_arbitrary_trees(t in generators::arb_oracle_version()) {
        assert_flag_bijection(&t);
    }
}

// ─── single-bit mutation sweeps ─────────────────────────────────────────────

/// Assert one mutated stream never aliases its origin: it is rejected, or its
/// decoded value, re-derived through the oracle bridge into a fresh canonical
/// encoding, spells exactly the mutated stream.
///
/// The re-derivation is the accept side's whole strength: decode *adopts* the
/// accepted bytes as storage verbatim and `Eq` is byte equality, so comparing
/// the decoded version's own stream against the mutated bytes would hold under
/// any validator behavior. Only an independently rebuilt encoding can detect a
/// validator that accepted a non-canonical spelling.
fn assert_mutation_never_aliases(v: &Version, bits: &BitsWriter, flip: u64) {
    let mut mutated = bits.clone();
    let old = mutated.bit(flip);
    mutated.patch_bit(flip, !old);
    match decode_stream(mutated.clone()) {
        Err(_) => {}
        Ok(w) => {
            assert_ne!(
                &w, v,
                "a single-bit mutation decoded back to the same version: \
                 two spellings of one value were both accepted"
            );
            let canon = from_oracle_version(&to_oracle_version(&w));
            assert_eq!(
                stream_of(&canon),
                mutated,
                "an accepted stream must be the canonical encoding of its value: \
                 the oracle re-derivation spells it differently"
            );
        }
    }
}

/// Exhaustive small scope: flipping any single bit of any depth-2 normal form's
/// encoding either rejects or round-trips to a different canonical value — no
/// silent acceptance of non-canonical spellings.
#[test]
fn exhaustive_single_bit_mutations_never_alias() {
    for t in all_normal_events(EV_SMALL_DEPTH) {
        let v = from_oracle_version(&t);
        let bits = stream_of(&v);
        for flip in 0..bits.len() {
            assert_mutation_never_aliases(&v, &bits, flip);
        }
    }
}

proptest! {
    /// Arbitrary trees under a proptest-chosen single-bit flip either reject or
    /// round-trip to a different canonical value.
    #[test]
    fn arbitrary_single_bit_mutations_never_alias(
        t in generators::arb_oracle_version(),
        flip_seed in any::<prop::sample::Index>(),
    ) {
        let v = from_oracle_version(&t);
        let bits = stream_of(&v);
        let flip = flip_seed.index(usize::try_from(bits.len()).expect("test streams are small")) as u64;
        assert_mutation_never_aliases(&v, &bits, flip);
    }
}

// ─── round trips over the generator families ────────────────────────────────

/// Assert that a version's stored stream validates and decodes to the same value.
fn assert_round_trip(v: &Version) {
    let bits = stream_of(v);
    assert!(
        whole(bits.reader()).is_ok(),
        "the encoder emits canonical streams"
    );
    let back = decode_stream(bits).expect("a canonical stream decodes");
    assert_eq!(
        &back, v,
        "the Version round-trip reproduces the version exactly"
    );
}

/// Every registered generator family round-trips exactly across a deterministic
/// size grid.
#[test]
fn generator_families_round_trip() {
    let shapes: Vec<Encoding> = vec![
        Shape::Dense.build1(1),
        Shape::Dense.build1(2),
        Shape::Dense.build1(64),
        Shape::Dense.build1(1_000),
        Shape::Bigroot.build2(7, 3),
        Shape::Bigroot.build2(200, 50),
        Shape::Bigroot.build2(1_000, 200),
        Shape::Hugeleaf.build1(1),
        Shape::Hugeleaf.build1(64),
        Shape::Hugeleaf.build1(5_000),
        Shape::CliffComb.build2(3, 2),
        Shape::CliffComb.build2(64, 64),
        Shape::CliffComb.build2(512, 512),
        Shape::WideToothComb.build3(64, 8, 16),
        Shape::WideToothComb.build3(512, 192, 64),
        Shape::CliffFan.build2(64, 64),
        Shape::CliffFan.build2(512, 128),
        Shape::CancellingChain.build2(64, 64),
        Shape::CancellingChain.build2(512, 128),
        Shape::AltSpine.build1(1),
        Shape::AltSpine.build1(2),
        Shape::AltSpine.build1(3),
        Shape::AltSpine.build1(64),
        Shape::AltSpine.build1(1_001),
    ];
    for p in &shapes {
        assert_round_trip(&version_of(p));
    }
}

/// Exhaustive small scope: every normal-form tree to depth 2 round-trips,
/// and no two distinct versions share a stored stream.
#[test]
fn exhaustive_small_scope_round_trips_and_is_injective() {
    let pool = all_normal_events(EV_SMALL_DEPTH);
    let mut seen: BTreeSet<Vec<u8>> = BTreeSet::new();
    for t in &pool {
        let v = from_oracle_version(t);
        assert_round_trip(&v);
        // Key on the stored bytes alone: marker padding makes them injective,
        // so distinct versions must differ somewhere a decoder can see.
        let key = v.as_bytes().to_vec();
        assert!(
            seen.insert(key),
            "two distinct versions encoded to one Version stream: {v:?}"
        );
    }
}

proptest! {
    /// Arbitrary normal-form trees, including values beyond `u64::MAX`,
    /// round-trip exactly.
    #[test]
    fn arbitrary_trees_round_trip(t in generators::arb_oracle_version()) {
        assert_round_trip(&from_oracle_version(&t));
    }

    /// Every version produced by an organic fork/tick/send/sync/join history
    /// round-trips exactly.
    #[test]
    fn organic_histories_round_trip(ops in optrace::world_strategy_up_to(120)) {
        let mut clocks = vec![Clock::seed()];
        for op in &ops {
            optrace::step_impl(&mut clocks, op);
        }
        for clock in &clocks {
            assert_round_trip(clock.version());
        }
    }

    /// Value-equal versions built along different operation paths produce
    /// byte-identical Version streams: the coding is a function of the value,
    /// never of the op path that constructed it.
    #[test]
    fn op_paths_yield_identical_bytes(
        a in generators::arb_oracle_version(),
        b in generators::arb_oracle_version(),
        c in generators::arb_oracle_version(),
    ) {
        let (a, b, c) = (
            from_oracle_version(&a),
            from_oracle_version(&b),
            from_oracle_version(&c),
        );
        prop_assert_eq!(stream_of(&(&a | &b)), stream_of(&(&b | &a)));
        prop_assert_eq!(
            stream_of(&(&(&a | &b) | &c)),
            stream_of(&(&a | &(&b | &c)))
        );
        prop_assert_eq!(stream_of(&(&a & &b)), stream_of(&(&b & &a)));
    }
}
