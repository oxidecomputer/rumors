//! Test construction of canonical versions from tree-shaped streams.
//!
//! Oracles and adversarial generators describe a version as a min-lifted
//! preorder tree: every node carries a topology flag and a nonnegative base.
//! Production versions instead store absolute first-leaf height followed by
//! changes between adjacent leaves. This module is the test-only boundary
//! between those representations.

use num_bigint::{BigInt, BigUint};

use crate::bits::{BitRead, BitsReader};
use crate::version::io::writer::VersionWriter;
use crate::Version;

/// Zigzag-code the change between adjacent absolute leaf heights.
#[cfg(test)]
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
    let mut out = VersionWriter::new();
    let mut pending = vec![(BigUint::ZERO, 0u64)];
    let mut previous_leaf: Option<BigUint> = None;

    while let Some((offset, depth)) = pending.pop() {
        let internal = reader.read_bit().expect("the test tree is complete");
        let base = reader.read_gamma().expect("the test tree is complete");

        let height = &offset + &base;
        if internal {
            pending.push((height.clone(), depth + 1));
            pending.push((height, depth + 1));
        } else {
            match previous_leaf.replace(height.clone()) {
                None => out.height(depth, &height),
                Some(previous) => {
                    out.change(depth, &(BigInt::from(height) - BigInt::from(previous)))
                }
            }
        }
    }

    assert_eq!(
        reader.position(),
        reader.len(),
        "the test stream contains exactly one tree"
    );
    out.finish()
}
