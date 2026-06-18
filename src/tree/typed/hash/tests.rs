//! Pins the wire-visible hash preimage convention, so the contiguous-buffer
//! `branch` rewrite cannot silently change any on-the-wire hash.

use super::{BRANCH_TAG, ContentHash, Hash, LEAF_TAG, MERKLE_HASH_LEN};

/// A branch commits to exactly `BRANCH_TAG ‖ (radix ‖ child_hash)*` over its
/// children in the iteration order given — radix byte first, then the 16-byte
/// child hash, with no length prefix or padding.
#[test]
fn branch_preimage_is_tag_then_radix_hash_records() {
    let children = [
        (7u8, Hash([0xab; MERKLE_HASH_LEN])),
        (200u8, Hash([0x11; MERKLE_HASH_LEN])),
    ];

    let mut expected = vec![BRANCH_TAG];
    for (radix, child) in &children {
        expected.push(*radix);
        expected.extend_from_slice(&child.0);
    }

    assert_eq!(Hash::branch(children), Hash::of(&expected));
}

/// The empty branch and a leaf commit to nothing but their domain tags, and the
/// two tags differ so the two can never collide.
#[test]
fn empty_root_and_leaf_are_their_bare_tags() {
    assert_eq!(Hash::empty_root(), Hash::of(&[BRANCH_TAG]));
    assert_eq!(Hash::leaf(), Hash::of(&[LEAF_TAG]));
    assert_ne!(Hash::empty_root(), Hash::leaf());
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
