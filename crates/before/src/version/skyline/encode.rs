//! The packed-form-to-skyline transcoder: the bridge from the adversarial
//! generators' construction language (min-lifted preorder packed streams,
//! one gamma-coded base per node) to the stored skyline coding.

use crate::codec::{self, Base, BitsMut, BitsSlice};

use super::zigzag;

/// Transcode a min-lifted packed preorder stream into its skyline stream.
///
/// One preorder pass: each node contributes its topology flag, and each
/// leaf's absolute height — the root-to-leaf path sum of stored bases — is
/// emitted as `gamma(v1)` for the first leaf and `zigzag-gamma(vi − vi−1)`
/// for every later one. Transient state is the inherited-path-sum stack
/// (one [`Base`] per open subtree), bounded by the packed input's own
/// depth and magnitudes. The walk is iterative over a heap stack, so it
/// needs no stack-growth guard at any input depth.
///
/// # Panics
///
/// Panics if the packed form does not parse cleanly; callers hand in
/// generator-built canonical streams.
pub(crate) fn encode_bits(bits: &BitsSlice) -> BitsMut {
    let mut out = BitsMut::with_capacity(bits.len());
    let mut pos = 0usize;
    // Inherited root-to-node path sums for the nodes not yet visited, top
    // of stack belonging to the next node in the preorder stream. Both
    // children of an internal node inherit the same sum, and the stream
    // lists the whole left subtree before the right, so a plain stack
    // stays aligned.
    let mut offsets: Vec<Base> = vec![Base::ZERO];
    let mut prev_leaf: Option<Base> = None;

    while let Some(offset) = offsets.pop() {
        let internal = bits[pos];
        pos += 1;
        let (base, next) = codec::decode_int(bits, pos).expect("canonical Version parses cleanly");
        pos = next;
        // The construction language flags `1` internal; the skyline
        // stream flags `0` internal (`1` leaf), so the flag inverts at
        // this transcode boundary.
        out.push(!internal);
        let value = &offset + &base;
        if internal {
            offsets.push(value.clone());
            offsets.push(value);
        } else {
            match &prev_leaf {
                None => codec::encode_int(&mut out, &value),
                Some(prev) => codec::encode_int(&mut out, &zigzag(prev, &value)),
            }
            prev_leaf = Some(value);
        }
    }
    assert_eq!(
        pos,
        bits.len(),
        "a canonical packed walk consumes every input bit"
    );
    out
}
