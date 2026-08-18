//! Pins the wire-visible hash preimage convention: the exact field layout of
//! the single-preimage node hash, and the collision pairs its kind tags and
//! length fields exist to prevent.

use super::{BRANCH_TAG, ContentHash, Hash, LEAF_TAG, MERKLE_HASH_LEN};

/// A branch commits to exactly `BRANCH_TAG ‖ prefix_len ‖ prefix ‖
/// child_count ‖ (radix ‖ child_hash)*`.
///
/// The prefix is length-tagged in one byte, the child count is a big-endian
/// `u16`, then 17-byte records follow in the iteration order given, with no
/// other framing or padding.
#[test]
fn branch_preimage_layout() {
    let prefix = [0x0a, 0x0b, 0x0c];
    let children = [
        (7u8, Hash([0xab; MERKLE_HASH_LEN])),
        (200u8, Hash([0x11; MERKLE_HASH_LEN])),
    ];

    let mut expected = vec![BRANCH_TAG, 3, 0x0a, 0x0b, 0x0c, 0x00, 0x02];
    for (radix, child) in &children {
        expected.push(*radix);
        expected.extend_from_slice(&child.0);
    }

    assert_eq!(Hash::branch(&prefix, children), Hash::of(&expected));
}

/// A leaf commits to exactly `LEAF_TAG ‖ suffix_len ‖ suffix` — its
/// compressed suffix, length-tagged, and nothing else.
#[test]
fn leaf_preimage_layout() {
    let suffix = [0x01, 0x02, 0x03, 0x04];
    assert_eq!(
        Hash::leaf(&suffix),
        Hash::of(&[LEAF_TAG, 4, 0x01, 0x02, 0x03, 0x04]),
    );
}

/// The empty tree hashes as a prefixless branch with no children —
/// `blake3(BRANCH_TAG ‖ 0 ‖ 0u16)` — and the memoized constant agrees with
/// the general branch rule applied to empty fields.
#[test]
fn empty_root_is_the_empty_branch() {
    assert_eq!(Hash::empty_root(), Hash::of(&[BRANCH_TAG, 0, 0x00, 0x00]));
    assert_eq!(Hash::empty_root(), Hash::branch(&[], []));
}

/// An empty-suffix leaf (its parent sits at depth 31) and the empty root
/// must not collide: the kind tags are what separate them, so both tags are
/// load-bearing under the single-preimage rule.
#[test]
fn empty_suffix_leaf_is_not_the_empty_root() {
    assert_ne!(Hash::leaf(&[]), Hash::empty_root());
}

/// A prefix byte cannot masquerade as child-record bytes: two branches whose
/// preimages would coincide without the `prefix_len` field must hash
/// differently.
///
/// The pair shifts one whole child record across the prefix/children
/// boundary — a 1-byte prefix with four children versus a prefix
/// lengthened by one full record with three — contrived so the untagged
/// concatenation `prefix ‖ child_count ‖ records` is the same byte string
/// for both (the test asserts that premise before asserting the hashes
/// differ). Only the length tag separates the two parses.
#[test]
fn prefix_len_separates_boundary_shifts() {
    // First child's hash ends in the bytes 0x00 0x03, which the shifted
    // parse reads as its child count of three.
    let h0 = {
        let mut h = [0x55u8; MERKLE_HASH_LEN];
        h[MERKLE_HASH_LEN - 2] = 0x00;
        h[MERKLE_HASH_LEN - 1] = 0x03;
        h
    };
    let (h1, h2, h3) = (
        [0x66u8; MERKLE_HASH_LEN],
        [0x77u8; MERKLE_HASH_LEN],
        [0x88u8; MERKLE_HASH_LEN],
    );

    let prefix_a = vec![0x07u8];
    let children_a = [(0u8, Hash(h0)), (1, Hash(h1)), (2, Hash(h2)), (3, Hash(h3))];

    // B's prefix swallows A's count, first radix, and most of the first
    // child hash; B's count comes from that hash's trailing bytes.
    let mut prefix_b = vec![0x07u8, 0x00, 0x04, 0x00];
    prefix_b.extend_from_slice(&h0[..MERKLE_HASH_LEN - 2]);
    let children_b = [(1u8, Hash(h1)), (2, Hash(h2)), (3, Hash(h3))];

    // Premise: without the prefix length tag, the preimages coincide.
    let untagged = |prefix: &[u8], children: &[(u8, Hash)]| {
        let mut buf = prefix.to_vec();
        let count = u16::try_from(children.len()).expect("small fan-out");
        buf.extend_from_slice(&count.to_be_bytes());
        for (radix, child) in children {
            buf.push(*radix);
            buf.extend_from_slice(&child.0);
        }
        buf
    };
    assert_eq!(
        untagged(&prefix_a, &children_a),
        untagged(&prefix_b, &children_b),
    );

    assert_ne!(
        Hash::branch(&prefix_a, children_a),
        Hash::branch(&prefix_b, children_b),
    );
}

/// A saturated 256-child branch encodes its count as big-endian `0x01 0x00`
/// — the u16's high byte, the entire reason the field is not a biased byte,
/// carries through `Hash::branch` correctly.
#[test]
fn saturated_fan_count_uses_the_high_byte() {
    let children: Vec<(u8, Hash)> = (0u8..=255)
        .map(|radix| (radix, Hash([radix; MERKLE_HASH_LEN])))
        .collect();

    let mut expected = vec![BRANCH_TAG, 0, 0x01, 0x00];
    for (radix, child) in &children {
        expected.push(*radix);
        expected.extend_from_slice(&child.0);
    }

    assert_eq!(Hash::branch(&[], children), Hash::of(&expected));
}

/// A Merkle hash is the prefix truncation of the full-width content hash of
/// the same preimage: the leading `MERKLE_HASH_LEN` bytes, nothing
/// rearranged or re-hashed.
///
/// Pinned so an accidental change to either primitive's construction trips
/// here before it reaches the wire snapshots.
#[test]
fn merkle_hash_is_prefix_of_full_width() {
    let preimage = b"any preimage at all";
    let truncated = Hash::of(preimage);
    let full = ContentHash::of(preimage);
    assert_eq!(truncated.as_bytes()[..], full.as_bytes()[..MERKLE_HASH_LEN]);
}
