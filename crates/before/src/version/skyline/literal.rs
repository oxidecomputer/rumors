//! Skyline literal constructors: the `TryFrom` surface's leaf and node
//! composers, building canonical streams from paper-notation structure.
//!
//! The node composer enforces *event-tree* normal form on the literal — every
//! node must have a zero-base child, and `(n, m, m)` collapses — so the
//! `TryFrom` impls reject exactly the non-normal-form literals the paper
//! notation can spell. This is stricter than skyline canonicality: a literal
//! like `(0, (0, 1, 2), 3)` denotes a perfectly canonical step function, but
//! its inner node hoards a liftable minimum, so it is not the normal spelling
//! and is refused.

use crate::codec::{self, Base, BitCursor, BitsMut, BitsSlice, DsiCursor};
use crate::error::Parse;

use super::signed::{unzigzag_base, zigzag};

/// The skyline stream of an event leaf with base `base`.
pub(crate) fn leaf(base: u64) -> BitsMut {
    let mut bits = BitsMut::new();
    bits.push(true); // topology: a leaf
    codec::encode_int(&mut bits, &Base::from(base)); // absolute height
    bits
}

/// Compose an event node with base `base` from two already-canonical child
/// streams, enforcing normal form.
///
/// Rejects [`Parse::NotCanonical`] when the node is collapsible (two leaf
/// children of equal height, which is just the leaf itself) or when the node
/// hoards a liftable minimum (neither child's minimum leaf height is zero —
/// normal form stores the shared minimum at the parent).
pub(crate) fn node(base: u64, left: &BitsSlice, right: &BitsSlice) -> Result<BitsMut, Parse> {
    let (left_topology, left_heights) = scan(left);
    let (right_topology, right_heights) = scan(right);

    let left_min = left_heights
        .iter()
        .min()
        .expect("a tree has at least one leaf");
    let right_min = right_heights
        .iter()
        .min()
        .expect("a tree has at least one leaf");
    if left_topology.len() == 1 && right_topology.len() == 1 && left_heights[0] == right_heights[0]
    {
        return Err(Parse::NotCanonical); // (n, m, m) collapses to a leaf, whatever m
    }
    if *left_min != Base::ZERO && *right_min != Base::ZERO {
        return Err(Parse::NotCanonical); // a liftable minimum: not min-lifted
    }

    let base = Base::from(base);
    let mut bits = BitsMut::new();
    bits.push(false); // topology: this node
    bits.extend_from_bitslice(&left_topology); // then the left subtree's topology…
    bits.extend_from_bitslice(&right_topology); // …then the right's — but the
                                                // payloads interleave, so re-emit.
    let mut out = BitsMut::new();
    let mut flags = bits.iter().by_vals();
    let mut heights = left_heights.iter().chain(right_heights.iter());
    let mut prev: Option<Base> = None;
    for flag in &mut flags {
        out.push(flag);
        if !flag {
            continue; // an internal node carries no payload
        }
        let height = heights.next().expect("one height per leaf") + &base;
        match &prev {
            None => codec::encode_int(&mut out, &height),
            Some(previous) => codec::encode_int(&mut out, &zigzag(previous, &height)),
        }
        prev = Some(height);
    }
    // Canonicalizing the storage is `Version::from_bits`'s job, the single gate
    // a stream passes through when it becomes a stored value.
    Ok(out)
}

/// Split a canonical stream into its topology flags (wire convention: `0`
/// internal, `1` leaf) and absolute leaf heights.
fn scan(bits: &BitsSlice) -> (BitsMut, Vec<Base>) {
    let mut cursor = DsiCursor::new(bits);
    let mut topology = BitsMut::new();
    let mut heights: Vec<Base> = Vec::new();
    let mut pending = 1usize;
    while pending > 0 {
        // One whole descent per unary read: the run's internal nodes, then the
        // leaf whose flag terminates the run. Each internal node opens two
        // children and closes itself; the leaf closes itself.
        let internal_nodes = cursor
            .read_unary()
            .expect("a canonical stream holds a complete tree");
        for _ in 0..internal_nodes {
            topology.push(false);
        }
        topology.push(true);
        pending = pending + internal_nodes - 1;
        // The cursor's own `read_int`: word-parallel payload decode.
        let code = cursor
            .read_int()
            .expect("a canonical stream holds a complete payload per leaf");
        let value = match heights.last() {
            None => code.into_base(),
            Some(prev) => {
                let (sign, magnitude) = unzigzag_base(code.into_base());
                if sign.is_negative() {
                    prev.clone() - &magnitude
                } else {
                    prev + &magnitude
                }
            }
        };
        heights.push(value);
    }
    (topology, heights)
}
