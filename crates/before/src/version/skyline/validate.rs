//! The streaming strict validator: canonical form on ~2 bits per level.
//!
//! One forward pass enforcing the module doc's canonical-form conditions.
//! The transient state is exactly:
//!
//! - a packed bit stack holding two bits per open ancestor —
//!   *left-complete* ("is my left child done") and *left-was-leaf* ("was
//!   that child a leaf", the fact the sibling-collapse check needs) —
//!   where machine-word parse frames would cost tens of bytes per level;
//! - one cliff-immune [`Accum`] carrying the running leaf height for the
//!   nonnegativity check (a plain big-integer here re-imports the
//!   boundary comb's quadratic carry genre; the module doc carries the
//!   argument);
//! - the cursor.
//!
//! No height, base, or node is ever materialized beyond the one decoded
//! payload in flight.

use core::cmp::Ordering;

use crate::codec::accum::Accum;
use crate::codec::{Base, BitCursor, Bits, BitsSlice, DsiCursor};
use crate::error::Decode;

use super::unzigzag;

/// Strictly validate one whole skyline stream.
///
/// The stream must be exactly one canonical tree: a tree that completes
/// before the last live bit is [`Decode::TrailingBits`]; everything else
/// is [`validate_from`]'s contract.
pub(crate) fn validate_bits(bits: &BitsSlice) -> Result<(), Decode> {
    let mut cursor = DsiCursor::new(bits);
    validate_from(&mut cursor)?;
    if cursor.position() != bits.len() {
        return Err(Decode::TrailingBits);
    }
    Ok(())
}

/// Strictly validate one skyline tree at the head of a bit stream,
/// returning the position just past it.
///
/// The wire decoder's entry: a version's skyline stream is
/// bit-self-delimiting (one complete tree), so the returned end position
/// is where any zero padding must begin.
pub(crate) fn validate_prefix(bits: &BitsSlice) -> Result<usize, Decode> {
    let mut cursor = DsiCursor::new(bits);
    validate_from(&mut cursor)?;
    Ok(cursor.position())
}

/// Validate one skyline tree from a sequential bit cursor.
///
/// Returns with the cursor just past the tree. Errors: running out of
/// bits mid-tree or mid-code is [`Decode::Truncated`]; a collapsible
/// sibling pair (an internal node's two leaf children with a zero right
/// delta) or a delta driving the running leaf height negative is
/// [`Decode::NotCanonical`].
pub(crate) fn validate_from<C: BitCursor>(cursor: &mut C) -> Result<(), Decode>
where
    Decode: From<C::Error>,
{
    // Two bits per open ancestor, pushed [left-complete, left-was-leaf]
    // and popped in reverse order below. A packed bit stack, so depth
    // costs bits, not frames.
    let mut open: Bits = Bits::new();
    // The running leaf height. Only its sign is ever read, and only after
    // a subtracting delta: an adding delta cannot take a valid height
    // negative, and the first leaf's absolute payload is a natural.
    let mut height = Accum::new();
    let mut seen_leaf = false;

    loop {
        // One whole descent per unary read: `k` internal nodes opened,
        // then the leaf whose flag terminates the run.
        let k = cursor.read_unary()?;
        for _ in 0..k {
            open.push(false); // left-complete: the left child comes next
            open.push(false); // left-was-leaf: placeholder until it does
        }

        // The leaf: decode its payload and update the running height,
        // through the cursor's own `read_int` so a word-parallel cursor
        // (the production reader; the wire-side reader's window) takes
        // its fast path.
        let code = cursor.read_int()?;
        let mut zero_delta = false;
        if seen_leaf {
            zero_delta = code == Base::ZERO;
            let (negative, magnitude) = unzigzag(code);
            if negative {
                height.sub_base(&magnitude);
            } else {
                height.add_base(&magnitude);
            }
            if negative && height.sign() == Ordering::Less {
                return Err(Decode::NotCanonical); // a leaf height fell below zero
            }
        } else {
            height.add_base(&code);
            seen_leaf = true;
        }

        // Close every subtree this leaf completes, walking up the open
        // ancestors; `is_leaf`/`leaf_zero_delta` describe the completed
        // subtree (the leaf itself on the first iteration).
        let mut is_leaf = true;
        let mut leaf_zero_delta = zero_delta;
        loop {
            let Some(left_was_leaf) = open.pop() else {
                return Ok(()); // the root is complete
            };
            let left_complete = open
                .pop()
                .expect("the open stack holds two bits per ancestor");
            if !left_complete {
                // The completed subtree was this ancestor's left child;
                // its right child comes next in the stream.
                open.push(true);
                open.push(is_leaf);
                break;
            }
            // The completed subtree was the right child: the ancestor
            // closes. Two leaf children with a zero right delta are the
            // collapsible pair minimal topology prohibits.
            if left_was_leaf && is_leaf && leaf_zero_delta {
                return Err(Decode::NotCanonical);
            }
            is_leaf = false;
            leaf_zero_delta = false;
        }
    }
}
