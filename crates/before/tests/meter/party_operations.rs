//! Resource envelopes for party operations on deep ownership trees.

use super::*;

// ─── id spine pair scenarios ────────────────────────────────────────────────

/// The combined operand bytes of an id-spine pair scenario.
pub fn id_pair_input_bytes(a: &meter::Encoding, b: &meter::Encoding) -> usize {
    a.bytes.len() + b.bytes.len()
}

/// Joining the diverted id-spine pair stays within its envelope (the
/// two-tree walk runs to full lockstep depth) and produces the
/// construction-known bytes.
///
/// The output pin: the divert arms are the two children of the node at
/// depth `d − 1`, so their union collapses that node to a terminal — the
/// joined party is exactly the spine one level shorter, byte for byte.
#[test]
fn id_join_envelope() {
    let pa = Shape::IdSpine.build_flagged(ID_DEPTH, false);
    let pb = Shape::IdSpine.build_flagged(ID_DEPTH, true);
    let input = id_pair_input_bytes(&pa, &pb);
    let mut a = party_of(&pa);
    let b = party_of(&pb);
    let r = metered("id_join", input, &envelope::ID_JOIN, || a.join(b));
    assert!(
        r.is_ok(),
        "the divert arms are disjoint, so join must succeed"
    );
    assert_eq!(
        a.encode(),
        Shape::IdSpine.build_flagged(ID_DEPTH - 1, false).bytes,
        "the divert arms rejoin into the spine one level shorter"
    );
}

/// `covers` over the diverted id-spine pair stays within its envelope (the
/// two-tree walk runs to full lockstep depth).
#[test]
fn id_covers_envelope() {
    let pa = Shape::IdSpine.build_flagged(ID_DEPTH, false);
    let pb = Shape::IdSpine.build_flagged(ID_DEPTH, true);
    let input = id_pair_input_bytes(&pa, &pb);
    let a = party_of(&pa);
    let b = party_of(&pb);
    let r = metered("id_covers", input, &envelope::ID_COVERS, || a.covers(&b));
    assert!(
        !r,
        "the divert arms are disjoint, so neither covers the other"
    );
}

/// `is_disjoint` over the diverted id-spine pair stays within its envelope
/// (the walk runs to completion: the pair is disjoint, so no early exit).
#[test]
fn id_disjoint_envelope() {
    let pa = Shape::IdSpine.build_flagged(ID_DEPTH, false);
    let pb = Shape::IdSpine.build_flagged(ID_DEPTH, true);
    let input = id_pair_input_bytes(&pa, &pb);
    let a = party_of(&pa);
    let b = party_of(&pb);
    let r = metered("id_disjoint", input, &envelope::ID_DISJOINT, || {
        a.is_disjoint(&b)
    });
    assert!(r, "the divert arms own disjoint regions");
}

/// Joining an id spine with its exact complement yields the seed, byte for
/// byte.
///
/// A region and its complement union to the full owner. On the spine pair
/// the collapse cascades: the deepest output node closes as two terminals,
/// which turns its parent into two terminals, and so on for all `d` levels
/// up to the root — the deep-shape pin on the join emitter's close-time
/// `(1, 1) → 1` repair, whose output is the seed's single-terminal encoding.
#[test]
fn id_join_spine_with_complement_collapses_to_seed() {
    let pa = Shape::IdSpine.build_flagged(ID_DEPTH, false);
    let mut a = party_of(&pa);
    let complement = Party::seed()
        .without(&a)
        .expect("the seed strictly covers a spine, so the complement is non-empty");
    a.join(complement)
        .expect("a region and its complement are disjoint");
    assert_eq!(
        a.encode(),
        Party::seed().encode(),
        "a spine and its complement union to the full seed region"
    );
}

/// Joining spines that lean into opposite root halves splices both operands
/// verbatim under a root fork, byte for byte.
///
/// The operands share no level below the root, so no output node collapses:
/// the joined encoding is the both-children root tag followed by each
/// operand's subtree bits unchanged — the deep-shape pin that the join
/// emitter copies non-overlapping subtrees without rewriting them.
#[test]
fn id_join_opposite_spines_splice_verbatim() {
    let pa = Shape::IdSpine.build_flagged(ID_DEPTH, false);
    let pb_tags = right_spine_tags(ID_DEPTH);
    let mut a = party_of(&pa);
    let b = Party::decode(&pack_bits(&pb_tags)[..])
        .expect("the right-leaning spine is strict normal form");
    a.join(b)
        .expect("spines in opposite root halves are disjoint");
    // The expected bytes, assembled from the constructions: a both-children
    // root tag, then each operand's bits below its root node, verbatim.
    let mut expected = vec![true, true];
    expected.extend((2..pa.bits).map(|i| encoded_bit(&pa.bytes, i)));
    expected.extend_from_slice(&pb_tags[2..]);
    assert_eq!(
        a.encode(),
        pack_bits(&expected),
        "opposite spines splice verbatim under a root fork"
    );
}

/// The right-leaning id spine's tag stream: `d` right-only tags ending in a
/// terminal, the mirror of [`meter::id_spine`]'s unary chain.
fn right_spine_tags(d: usize) -> Vec<bool> {
    let mut tags = Vec::with_capacity(2 * d + 2);
    for _ in 0..d {
        tags.push(false); // left child absent ...
        tags.push(true); // ... right child present
    }
    tags.push(false); // terminal tag "00": the single owned tip
    tags.push(false);
    tags
}

/// Pack a most-significant-bit-first bit stream into its marker-padded
/// wire bytes, the generators' packing convention.
fn pack_bits(bits: &[bool]) -> Vec<u8> {
    let mut bytes = vec![0u8; (bits.len() + 1).div_ceil(8)];
    for (i, &bit) in bits.iter().enumerate() {
        if bit {
            bytes[i / 8] |= 0x80 >> (i % 8);
        }
    }
    bytes[bits.len() / 8] |= 0x80 >> (bits.len() % 8); // the padding marker
    bytes
}

/// Read live bit `i` of an encoded stream (most significant bit first).
fn encoded_bit(bytes: &[u8], i: usize) -> bool {
    bytes[i / 8] & (0x80 >> (i % 8)) != 0
}

/// `without` subtracting an id spine from the seed stays within its
/// envelope: the complement sweep runs the subtrahend's full depth.
#[test]
fn id_without_envelope() {
    let pb = Shape::IdSpine.build_flagged(ID_DEPTH, true);
    let input = pb.bytes.len();
    let b = party_of(&pb);
    let r = metered("id_without", input, &envelope::ID_WITHOUT, || {
        Party::seed().without(&b)
    });
    assert!(
        r.is_some(),
        "the seed strictly covers a spine, so the complement is non-empty"
    );
}
