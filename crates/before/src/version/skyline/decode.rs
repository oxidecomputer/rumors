//! The skyline-to-packed-form transcoder.
//!
//! [`decode_bits`] gates on the streaming validator, then rebuilds the
//! stored packed form in three linear passes: materialize topology and
//! absolute leaf heights; derive every subtree's floor (its minimum leaf
//! height) bottom-up; emit the min-lifted preorder stream, each node's
//! stored base being its floor minus its parent's. Transient state is
//! `O(nodes)` heights and floors — priced by the packed form being
//! emitted, which itself materializes every base (the module doc's cost
//! section; only the validator is skyline-linear).

use crate::codec::{self, Base, BitCursor, Bits, BitsSlice, SliceCursor};
use crate::error::Decode;
use crate::Version;

use super::{unzigzag, validate_bits};

/// Strictly decode one skyline stream into a stored [`Version`].
///
/// Validation is [`validate_bits`]'s, bit for bit; the transcode below it
/// assumes a canonical stream. The emitted packed form is canonical by
/// construction: min-lifting gives every internal node a zero-base child,
/// and skyline minimal topology rules out equal sibling leaves.
pub(crate) fn decode_bits(bits: &BitsSlice) -> Result<Version, Decode> {
    validate_bits(bits)?;

    // Pass 1: split the stream into topology flags and absolute leaf
    // heights (running value: first payload absolute, later payloads
    // signed deltas; validation guarantees no height goes negative).
    let mut cursor = SliceCursor::new(bits, 0);
    let mut topology: Bits = Bits::new();
    let mut heights: Vec<Base> = Vec::new();
    let mut pending = 1usize;
    while pending > 0 {
        pending -= 1;
        let internal = cursor
            .read_bit()
            .expect("a validated stream holds a complete tree");
        topology.push(internal);
        if internal {
            pending += 2;
            continue;
        }
        let code = codec::decode_int_from(&mut cursor)
            .expect("a validated stream holds a complete payload per leaf");
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

    // Pass 2: every node's floor — the minimum leaf height in its subtree
    // — bottom-up over the topology, indexed by preorder position. A
    // leaf's floor is its height; an internal node's is the smaller of
    // its children's.
    let nodes = topology.len();
    let mut floors: Vec<Base> = vec![Base::ZERO; nodes];
    // Open internal nodes: preorder index, plus the left child's floor
    // once that child has completed.
    let mut open: Vec<(usize, Option<Base>)> = Vec::new();
    let mut next_leaf = 0usize;
    for (index, internal) in topology.iter().by_vals().enumerate() {
        if internal {
            open.push((index, None));
            continue;
        }
        floors[index] = heights[next_leaf].clone();
        next_leaf += 1;
        // Close every subtree this leaf completes.
        let mut summary = floors[index].clone();
        loop {
            match open.pop() {
                None => break, // the root is complete
                Some((parent, None)) => {
                    // The completed subtree was the left child; its floor
                    // waits here for the right sibling, which comes next.
                    open.push((parent, Some(summary)));
                    break;
                }
                Some((parent, Some(left))) => {
                    let floor = if left <= summary { left } else { summary };
                    floors[parent] = floor.clone();
                    summary = floor;
                }
            }
        }
    }

    // Pass 3: emit the min-lifted preorder stream. Each node stores its
    // floor relative to its parent's (the root relative to zero); both
    // children inherit the node's own floor, preorder order keeping a
    // plain stack aligned exactly as in the encoder.
    let mut out = Bits::with_capacity(bits.len());
    let mut parent_floors: Vec<Base> = vec![Base::ZERO];
    for (index, internal) in topology.iter().by_vals().enumerate() {
        let parent = parent_floors
            .pop()
            .expect("preorder supplies one inherited floor per node");
        let stored = floors[index].clone() - &parent;
        out.push(internal);
        codec::encode_int(&mut out, &stored);
        if internal {
            parent_floors.push(floors[index].clone());
            parent_floors.push(floors[index].clone());
        }
    }
    Ok(Version::from_bits(out))
}
