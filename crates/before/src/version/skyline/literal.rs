//! Skyline literal constructors: the `TryFrom` surface's leaf and node
//! composers, building canonical streams from paper-notation structure.
//!
//! The node composer enforces *event-tree* normal form on the literal —
//! every node must have a zero-base child, and `(n, m, m)` collapses — so
//! the `TryFrom` impls reject exactly the non-normal-form literals the
//! paper notation can spell. This is stricter than skyline canonicality:
//! a literal like `(0, (0, 1, 2), 3)` denotes a perfectly canonical step
//! function, but its inner node hoards a liftable minimum, so it is not
//! the normal spelling and is refused.

use crate::codec::{self, Base, BitCursor, Bits, SliceCursor};
use crate::error::Parse;

use super::{unzigzag, validate_bits, zigzag, Encoded};

/// The skyline stream of an event leaf with base `n`.
pub(crate) fn leaf(n: u64) -> Encoded {
    let mut bits = Bits::new();
    bits.push(false); // topology: a leaf
    codec::encode_int(&mut bits, &Base::from(n)); // absolute height
    seal(bits)
}

/// Compose an event node with base `n` from two already-canonical child
/// streams, enforcing normal form.
///
/// Rejects [`Parse::NotCanonical`] when the node hoards a liftable minimum
/// (neither child's minimum leaf height is zero — normal form stores the
/// shared minimum at the parent) or when the node is collapsible (two leaf
/// children of equal height, which is just the leaf itself).
pub(crate) fn node(n: u64, l: &Encoded, r: &Encoded) -> Result<Encoded, Parse> {
    let (l_topo, l_heights) = scan(l);
    let (r_topo, r_heights) = scan(r);

    let l_min = l_heights
        .iter()
        .min()
        .expect("a tree has at least one leaf");
    let r_min = r_heights
        .iter()
        .min()
        .expect("a tree has at least one leaf");
    if *l_min != Base::ZERO && *r_min != Base::ZERO {
        return Err(Parse::NotCanonical); // a liftable minimum: not min-lifted
    }
    if l_topo.len() == 1 && r_topo.len() == 1 && l_heights[0] == r_heights[0] {
        return Err(Parse::NotCanonical); // (n, m, m) collapses to a leaf
    }

    let n = Base::from(n);
    let mut bits = Bits::new();
    bits.push(true); // topology: this node
    bits.extend_from_bitslice(&l_topo); // then the left subtree's topology…
    bits.extend_from_bitslice(&r_topo); // …then the right's — but the
                                        // payloads interleave, so re-emit.
    let mut out = Bits::new();
    let mut topo = bits.iter().by_vals();
    let mut heights = l_heights.iter().chain(r_heights.iter());
    let mut prev: Option<Base> = None;
    for internal in &mut topo {
        out.push(internal);
        if internal {
            continue;
        }
        let height = heights.next().expect("one height per leaf") + &n;
        match &prev {
            None => codec::encode_int(&mut out, &height),
            Some(p) => codec::encode_int(&mut out, &zigzag(p, &height)),
        }
        prev = Some(height);
    }
    debug_assert!(
        validate_bits(&out).is_ok(),
        "a min-lift-checked literal composition is canonical",
    );
    Ok(seal(out))
}

/// Split a canonical stream into its topology flags and absolute leaf
/// heights.
fn scan(enc: &Encoded) -> (Bits, Vec<Base>) {
    let all = codec::bytes_as_bits(&enc.bytes);
    let bits = &all[..enc.bits];
    let mut cursor = SliceCursor::new(bits, 0);
    let mut topology = Bits::new();
    let mut heights: Vec<Base> = Vec::new();
    let mut pending = 1usize;
    while pending > 0 {
        pending -= 1;
        let internal = cursor
            .read_bit()
            .expect("a canonical stream holds a complete tree");
        topology.push(internal);
        if internal {
            pending += 2;
            continue;
        }
        let code = codec::decode_int_from(&mut cursor)
            .expect("a canonical stream holds a complete payload per leaf");
        let value = match heights.last() {
            None => code,
            Some(prev) => {
                let (negative, magnitude) = unzigzag(code);
                if negative {
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

/// Canonicalize a composed stream's storage.
fn seal(mut bits: Bits) -> Encoded {
    let live = bits.len();
    codec::zero_dead_bits(&mut bits);
    Encoded {
        bytes: bits.into_vec(),
        bits: live,
    }
}
