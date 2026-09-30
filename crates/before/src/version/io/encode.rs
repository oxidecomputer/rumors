//! Transcodes generated trees into canonical Version storage.
//!
//! Resource tests construct trees in a convenient preorder language: one
//! topology flag and one gamma-coded base per node. This module converts that
//! language into the leaf-height deltas stored by [`Version`](crate::Version).

use num_bigint::BigUint;

use crate::bits::{BitRead, BitsReader, BitsWriter};

/// Zigzag-code the change between adjacent absolute leaf heights.
pub(super) fn zigzag_difference(previous: &BigUint, current: &BigUint) -> BigUint {
    if current >= previous {
        (current - previous) << 1u32
    } else {
        ((previous - current) << 1u32) - 1u32
    }
}

/// Transcode a min-lifted preorder stream into a Version stream.
///
/// One preorder pass: each node contributes its topology flag, and each leaf's
/// absolute height — the root-to-leaf path sum of stored bases — is emitted as
/// `gamma(v1)` for the first leaf and `zigzag-gamma(vi − vi−1)` for every later
/// one. Transient state is the inherited-path-sum stack (one [`BigUint`] per open
/// subtree), bounded by the encoded input's own depth and magnitudes. The walk
/// is iterative over a heap stack, so it needs no stack-growth guard at any
/// input depth.
///
/// # Panics
///
/// Panics if the encoded form does not parse cleanly; callers hand in
/// generator-built canonical streams.
pub fn encode_bits(mut reader: BitsReader<'_>) -> BitsWriter {
    let mut out = BitsWriter::with_capacity(reader.len());
    // Inherited root-to-node path sums for the nodes not yet visited, top of
    // stack belonging to the next node in the preorder stream. Both children of
    // an internal node inherit the same sum, and the stream lists the whole
    // left subtree before the right, so a plain stack stays aligned.
    let mut offsets: Vec<BigUint> = vec![BigUint::ZERO];
    let mut prev_leaf: Option<BigUint> = None;

    while let Some(offset) = offsets.pop() {
        let internal = reader.read_bit().expect("canonical Version parses cleanly");
        let base = reader
            .read_gamma()
            .expect("canonical Version parses cleanly");
        // The construction language flags `1` internal; the Version stream
        // flags `0` internal (`1` leaf), so the flag inverts at this transcode
        // boundary.
        out.push(!internal);
        let value = &offset + &base;
        if internal {
            offsets.push(value.clone());
            offsets.push(value);
        } else {
            match &prev_leaf {
                None => out.write_gamma(&value),
                Some(prev) => out.write_gamma(&zigzag_difference(prev, &value)),
            }
            prev_leaf = Some(value);
        }
    }
    assert_eq!(
        reader.position(),
        reader.len(),
        "a canonical encoded walk consumes every input bit"
    );
    out
}
