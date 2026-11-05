//! Hash input layouts, the kind, height and length fields that distinguish
//! them, and the keyed hash they are digested by.

use super::{BRANCH_TAG, HMAC_KEY_LEN, Hash, HashKey, LEAF_TAG, MERKLE_HASH_LEN};
use proptest::prelude::*;

/// The fixture network's hash key, the key these layout tests digest under.
fn key() -> HashKey {
    HashKey::fixture()
}

/// Generates two distinct one-byte heights.
fn arb_distinct_heights() -> impl Strategy<Value = (usize, usize)> {
    // Skipping `a` in the second draw's range makes every distinct pair
    // reachable, and no pair equal.
    (0..=u8::MAX, 0..u8::MAX).prop_map(|(a, b)| {
        let b = if b >= a { b + 1 } else { b };
        (usize::from(a), usize::from(b))
    })
}

/// A branch commits to exactly `BRANCH_TAG ‖ height ‖ prefix_len ‖ prefix ‖
/// child_count ‖ (radix ‖ child_hash)*`.
///
/// The height and the prefix length use one byte each; the child count uses
/// a big-endian `u16`. Each radix and hash follows in the supplied order,
/// without padding.
#[test]
fn branch_preimage_layout() {
    let prefix = [0x0a, 0x0b, 0x0c];
    let children = [
        (7u8, Hash([0xab; MERKLE_HASH_LEN])),
        (200u8, Hash([0x11; MERKLE_HASH_LEN])),
    ];

    let mut expected = vec![BRANCH_TAG, 20, 3, 0x0a, 0x0b, 0x0c, 0x00, 0x02];
    for (radix, child) in &children {
        expected.push(*radix);
        expected.extend_from_slice(&child.0);
    }

    assert_eq!(key().branch(20, &prefix, children), key().of(&expected));
}

/// A leaf hashes its kind tag, its height, and its length-tagged path
/// suffix. The version determines the full path; its encoding and the
/// payload are not repeated.
#[test]
fn leaf_preimage_layout() {
    let suffix = [0x01, 0x02, 0x03, 0x04];
    let expected = vec![LEAF_TAG, 4, 4, 0x01, 0x02, 0x03, 0x04];
    assert_eq!(key().leaf(4, &suffix), key().of(&expected));
}

proptest! {
    /// Duplicate or descending radixes cannot form a canonical branch hash.
    #[cfg(debug_assertions)]
    #[test]
    fn branch_rejects_nonascending_radixes(a in any::<u8>(), b in any::<u8>()) {
        let high = a.max(b);
        for low in [a.min(b), high] {
            let result = std::panic::catch_unwind(|| {
                key().branch(1, &[], [(high, Hash::default()), (low, Hash::default())]);
            });
            prop_assert!(result.is_err(), "accepted radixes {}, {}", high, low);
        }
    }

    /// Incremental leaf hashing matches the contiguous input at every accepted
    /// length, including inputs that span more than one SHA-256 block.
    #[test]
    fn leaf_hash_matches_contiguous_input(suffix in any::<[u8; 255]>()) {
        for len in 0..=u8::MAX {
            let suffix = &suffix[..usize::from(len)];
            let mut input = vec![LEAF_TAG, len, len];
            input.extend_from_slice(suffix);
            prop_assert_eq!(
                key().leaf(usize::from(len), suffix),
                key().of(&input),
                "suffix length {}",
                len
            );
        }
    }

    /// A digest is HMAC-SHA-256 of its input under the hash key, truncated
    /// to the Merkle width.
    ///
    /// The expected value is recomputed through the `hmac` crate's one-shot
    /// interface rather than the pre-keyed state the key clones. The
    /// network's tests check that the key schedule derives the key itself.
    #[test]
    fn a_digest_is_the_truncated_keyed_hash(
        raw in any::<[u8; HMAC_KEY_LEN]>(),
        input in proptest::collection::vec(any::<u8>(), 0..300),
    ) {
        use hmac::{Hmac, KeyInit, Mac};
        use sha2::Sha256;

        let expected = <Hmac<Sha256>>::new_from_slice(&raw)
            .expect("HMAC accepts a key of any length")
            .chain_update(&input)
            .finalize()
            .into_bytes();
        let derived = <[u8; MERKLE_HASH_LEN]>::from(HashKey::new(&raw).of(&input));
        prop_assert_eq!(&derived[..], &expected[..MERKLE_HASH_LEN]);
    }

    /// One branch layout at two different heights hashes to two different
    /// digests: the height separates the domains of nodes at different
    /// levels.
    ///
    /// The disagreement fails with probability about 2⁻¹²⁸ per case.
    #[test]
    fn the_height_separates_branch_digests(
        (low, high) in arb_distinct_heights(),
        prefix in proptest::collection::vec(any::<u8>(), 0..=32),
        digests in any::<[[u8; MERKLE_HASH_LEN]; 2]>(),
    ) {
        let children = [(0u8, Hash(digests[0])), (1u8, Hash(digests[1]))];
        prop_assert_ne!(
            key().branch(low, &prefix, children),
            key().branch(high, &prefix, children),
        );
    }

    /// One leaf layout and one branch layout each hash to unrelated digests
    /// under two different keys: the key enters every digest.
    ///
    /// The disagreement fails with probability about 2⁻¹²⁸ per comparison.
    #[test]
    fn different_keys_give_unrelated_digests(
        a in any::<[u8; HMAC_KEY_LEN]>(),
        byte in 0..HMAC_KEY_LEN,
        flip in 1..=u8::MAX,
        suffix in proptest::collection::vec(any::<u8>(), 0..=32),
        digests in any::<[[u8; MERKLE_HASH_LEN]; 2]>(),
    ) {
        let mut b = a;
        b[byte] ^= flip;
        let (a, b) = (HashKey::new(&a), HashKey::new(&b));
        prop_assert_ne!(a.leaf(suffix.len(), &suffix), b.leaf(suffix.len(), &suffix));
        let children = [(0u8, Hash(digests[0])), (1u8, Hash(digests[1]))];
        prop_assert_ne!(
            a.branch(suffix.len() + 1, &suffix, children),
            b.branch(suffix.len() + 1, &suffix, children),
        );
    }
}

/// A single-child branch must be compressed into the child's prefix before hashing.
#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "one-child branch is unrepresentable")]
fn branch_rejects_a_single_child() {
    key().branch(1, &[], [(0, Hash::default())]);
}

/// Reject a leaf whose height, and suffix length, cannot be represented by
/// the input's one-byte fields.
#[test]
#[should_panic(expected = "a trie position's height fits in one byte")]
fn leaf_height_must_fit_one_byte() {
    key().leaf(256, &[0; 256]);
}

/// The empty tree hashes as a prefixless branch with no children at the
/// root's height — `BRANCH_TAG ‖ 32 ‖ 0 ‖ 0u16` — and agrees with the
/// general branch rule applied to empty fields.
#[test]
fn empty_root_is_the_empty_branch() {
    assert_eq!(
        key().empty_root(),
        key().of(&[BRANCH_TAG, 32, 0, 0x00, 0x00])
    );
    assert_eq!(key().empty_root(), key().branch(32, &[], []));
}

/// An empty-suffix leaf (its parent sits at depth 31) and an empty branch
/// at height zero digest differently.
///
/// Their preimages differ in kind tag and in length, and each difference
/// alone separates them. The tag is not the only separation between kinds:
/// a leaf's preimage ends with the suffix its length byte declares, while a
/// branch's continues past the prefix its length byte declares with a
/// two-byte child count, so no leaf preimage equals a branch preimage even
/// under one tag.
#[test]
fn empty_suffix_leaf_is_not_an_empty_branch() {
    assert_ne!(key().leaf(0, &[]), key().branch(0, &[], []));
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
/// differ). Both sit at one height, so only the length tag separates the
/// two parses.
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
        key().branch(30, &prefix_a, children_a),
        key().branch(30, &prefix_b, children_b),
    );
}

/// A saturated 256-child branch encodes its count as big-endian `0x01 0x00`
/// — the u16's high byte, the entire reason the field is not a biased byte,
/// carries through `HashKey::branch` correctly.
#[test]
fn saturated_fan_count_uses_the_high_byte() {
    let children: Vec<(u8, Hash)> = (0u8..=255)
        .map(|radix| (radix, Hash([radix; MERKLE_HASH_LEN])))
        .collect();

    let mut expected = vec![BRANCH_TAG, 32, 0, 0x01, 0x00];
    for (radix, child) in &children {
        expected.push(*radix);
        expected.extend_from_slice(&child.0);
    }

    assert_eq!(key().branch(32, &[], children), key().of(&expected));
}
