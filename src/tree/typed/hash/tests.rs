//! Pins the wire-visible hash preimage convention, so the contiguous-buffer
//! `branch` rewrite cannot silently change any on-the-wire hash.

use super::{BRANCH_TAG, Hash, LEAF_TAG};

/// A branch commits to exactly `BRANCH_TAG ‖ (radix ‖ child_hash)*` over its
/// children in the iteration order given — radix byte first, then the 32-byte
/// child hash, with no length prefix or padding.
#[test]
fn branch_preimage_is_tag_then_radix_hash_records() {
    let children = [(7u8, Hash([0xab; 32])), (200u8, Hash([0x11; 32]))];

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
