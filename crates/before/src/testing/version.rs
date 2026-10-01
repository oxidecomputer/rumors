//! Test construction of canonical versions from tree-shaped streams.
//!
//! Oracles and adversarial generators describe a version as a min-lifted
//! preorder tree: every node carries a topology flag and a nonnegative base.
//! Production versions instead store absolute first-leaf height followed by
//! changes between adjacent leaves. This module is the test-only boundary
//! between those representations.

use num_bigint::BigUint;

use crate::bits::{BitRead, BitsReader, BitsWriter};
use crate::Version;

/// Zigzag-code the change between adjacent absolute leaf heights.
pub(crate) fn zigzag_difference(previous: &BigUint, current: &BigUint) -> BigUint {
    if current >= previous {
        (current - previous) << 1u32
    } else {
        ((previous - current) << 1u32) - 1u32
    }
}

/// Build a canonical version from a min-lifted preorder tree stream.
///
/// The walk keeps the inherited root-to-node sum for each pending subtree.
/// When it reaches a leaf, that sum plus the leaf's base is its absolute
/// height. The first leaf is stored directly; each later leaf is stored as its
/// zigzag-coded change from the preceding leaf. Both children inherit the same
/// sum, so a stack in reverse preorder is sufficient and every input node is
/// visited once.
///
/// # Panics
///
/// Panics if the generator-built input is incomplete or contains more than one
/// tree.
pub(crate) fn from_tree_stream(mut reader: BitsReader<'_>) -> Version {
    let mut out = BitsWriter::with_capacity(reader.len());
    let mut pending_offsets = vec![BigUint::ZERO];
    let mut previous_leaf: Option<BigUint> = None;

    while let Some(offset) = pending_offsets.pop() {
        let internal = reader.read_bit().expect("the test tree is complete");
        let base = reader.read_gamma().expect("the test tree is complete");

        // Test trees use `1` for a branch; stored versions use `1` for a leaf.
        out.push(!internal);
        let height = &offset + &base;
        if internal {
            pending_offsets.push(height.clone());
            pending_offsets.push(height);
        } else {
            match previous_leaf.replace(height.clone()) {
                None => out.write_gamma(&height),
                Some(previous) => out.write_gamma(&zigzag_difference(&previous, &height)),
            }
        }
    }

    assert_eq!(
        reader.position(),
        reader.len(),
        "the test stream contains exactly one tree"
    );
    crate::version::io::finish(out)
}
