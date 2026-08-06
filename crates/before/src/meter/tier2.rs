//! The exact encoded size a [`Version`](crate::Version) would have under the
//! Tier 2 coding: preorder topology bits plus delta-coded absolute leaf values.
//!
//! A measurement tool, not a codec: [`tier2_size`] computes, bit-exactly, how
//! large a canonical [`Version`](crate::Version) would be if re-encoded as its
//! preorder topology (one flag bit per node, exactly as today) plus its leaf
//! values in preorder — the first leaf's absolute value as `gamma(v1)`, every
//! later leaf as `zigzag-gamma(vi − vi−1)` over consecutive leaves. Internal
//! bases are derivable from the absolute leaf values and store nothing. The
//! compactness ratio between this size and today's encoded size is the evidence
//! the representation decision turns on, so the walk here is written for
//! obvious correctness over economy: one preorder pass over the packed form,
//! absolute leaf values accumulated as root-to-leaf path sums in
//! arbitrary-precision arithmetic (the crate's `Base`, so no magnitude
//! saturates or overflows the measurement).
//!
//! The zigzag map is the canonical sign convention `k >= 0 -> 2k`, `k < 0 ->
//! 2|k| - 1` (no negative zero), and each mapped delta is then gamma-coded
//! exactly like today's stored bases.

use crate::codec::{self, Base, BitsSlice};

/// The Tier 2 encoded bit length of a [`Version`](crate::Version), split into
/// the terms the compactness envelope is stated over.
///
/// `total_bits` is always `nodes + first_leaf_bits + delta_bits`: topology
/// costs one flag bit per node (identical in both encodings), and the leaf
/// payload is one absolute gamma code plus one zigzag-gamma code per later
/// leaf. The parts are exposed separately so the compactness suite can charge
/// the delta stream against today's stored bases (today's encoded size is
/// `nodes` flag bits plus its stored gamma codes, so the stored-base code bits
/// are exactly `encoded_bits - nodes`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tier2Size {
    /// The whole Tier 2 stream: `nodes + first_leaf_bits + delta_bits`.
    pub total_bits: u64,
    /// Nodes in the event tree; topology flag bits in either encoding.
    pub nodes: u64,
    /// Leaves in the event tree; the delta stream has `leaves - 1` codes.
    pub leaves: u64,
    /// Bits of `gamma(v1)`: the first preorder leaf's absolute value.
    pub first_leaf_bits: u64,
    /// Bits of `zigzag-gamma(vi - vi-1)` summed over the later leaves.
    pub delta_bits: u64,
}

/// Compute the exact Tier 2 encoded bit length of a canonical [`Version`](crate::Version).
///
/// One preorder pass over the packed form. The traversal is iterative over a
/// heap stack of inherited path sums (no stack-depth recursion), so it needs no
/// stack-growth guard at any input depth.
///
/// # Panics
///
/// Panics if the packed form does not parse cleanly; callers hand in
/// generator-built canonical streams.
pub fn tier2_size(bits: &BitsSlice) -> Tier2Size {
    let mut pos = 0usize;
    // Inherited root-to-node path sums for the nodes not yet visited, top of
    // stack belonging to the next node in the preorder stream. Both children of
    // an internal node inherit the same sum, and the stream lists the whole
    // left subtree before the right, so a plain stack stays aligned.
    let mut offsets: Vec<Base> = vec![Base::ZERO];
    let mut nodes = 0u64;
    let mut leaves = 0u64;
    let mut first_leaf_bits = 0u64;
    let mut delta_bits = 0u64;
    let mut prev_leaf: Option<Base> = None;

    while let Some(offset) = offsets.pop() {
        let internal = bits[pos];
        pos += 1;
        let (base, next) = codec::decode_int(bits, pos).expect("canonical Version parses cleanly");
        pos = next;
        nodes += 1;
        let value = &offset + &base;
        if internal {
            offsets.push(value.clone());
            offsets.push(value);
        } else {
            leaves += 1;
            match &prev_leaf {
                None => first_leaf_bits = gamma_bits(&value),
                Some(prev) => delta_bits += gamma_bits(&zigzag(prev, &value)),
            }
            prev_leaf = Some(value);
        }
    }
    assert_eq!(
        pos,
        bits.len(),
        "canonical Version walk consumes every packed bit"
    );

    Tier2Size {
        total_bits: nodes + first_leaf_bits + delta_bits,
        nodes,
        leaves,
        first_leaf_bits,
        delta_bits,
    }
}

/// The Elias-gamma code length of `n` in bits: `2 * floor(log2(n + 1)) + 1`.
///
/// Matches [`codec::encode_int`] exactly: the code for `m = n + 1` is
/// `floor(log2(m))` zeros then `m` in `floor(log2(m)) + 1` bits.
fn gamma_bits(n: &Base) -> u64 {
    2 * (n + 1u32).bits() - 1
}

/// Map the signed difference `cur - prev` to its zigzag magnitude:
/// `k >= 0 -> 2k`, `k < 0 -> 2|k| - 1`.
fn zigzag(prev: &Base, cur: &Base) -> Base {
    if cur >= prev {
        (cur.clone() - prev) << 1u32
    } else {
        ((prev.clone() - cur) << 1u32) - &Base::from(1u8)
    }
}

#[cfg(test)]
mod tests;
