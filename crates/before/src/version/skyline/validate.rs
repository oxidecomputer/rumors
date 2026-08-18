//! The streaming strict validator: canonical form on ~2 bits per level.
//!
//! One forward pass enforcing the module doc's canonical-form conditions. The
//! transient state is exactly:
//!
//! - a packed bit stack holding two bits per open ancestor —
//!   *left-complete* ("is my left child done") and *left-was-leaf* ("was
//!   that child a leaf", the fact the sibling-collapse check needs) —
//!   where machine-word parse frames would cost tens of bytes per level;
//! - one cliff-free [`Accumulator`] carrying the running leaf height for the
//!   nonnegativity check (a plain big-integer here re-imports the
//!   boundary comb's quadratic carry genre; the module doc carries the
//!   argument);
//! - the cursor.
//!
//! No height, base, or node is ever materialized beyond the one decoded payload
//! in flight.
//!
//! The span wire form's second component never takes this pass: the admission
//! walk ([`admit`](super::admit)) enforces the same strict obligations fused
//! with its dominance sweep, the height check subsumed by the verdict.

use core::cmp::Ordering;

use suanpan::Accumulator;

#[cfg(any(test, feature = "meter"))]
use crate::codec::BitsSlice;
use crate::codec::{BitCursor, BitsMut, DsiCursor};
use crate::error::Decode;

use super::signed::{fold_signed_int, unzigzag, Sign};

/// Strictly validate one whole skyline stream.
///
/// The stream must be exactly one canonical tree: a tree that completes before
/// the last live bit is [`Decode::TrailingBits`]; everything else is
/// [`validate_from`]'s contract.
///
/// Test- and meter-only: the production entries run [`validate_prefix`] and
/// [`validate_from`], which leave the tail to their callers.
#[cfg(any(test, feature = "meter"))]
pub(crate) fn validate_bits(bits: &BitsSlice) -> Result<(), Decode> {
    let mut cursor = DsiCursor::new(bits);
    validate_from(&mut cursor)?;
    if cursor.position() != bits.len() {
        return Err(Decode::TrailingBits);
    }
    Ok(())
}

/// Strictly validate one skyline tree at the head of a raw byte buffer's
/// whole `8 · bytes.len()`-bit view, returning the position just past it.
///
/// The wire decoder's entry: a version's skyline stream is bit-self-delimiting
/// (one complete tree), so the returned end position is where any zero padding
/// must begin. Raw bytes and a `u64` end, never a borrowed bit view: the
/// doors admit buffers past the view encoding's cap (64 MiB and up on a
/// 32-bit target), whose bit positions outgrow a 32-bit `usize`.
pub(crate) fn validate_prefix_bytes(bytes: &[u8]) -> Result<u64, Decode> {
    let mut cursor = DsiCursor::over_bytes(bytes);
    validate_from(&mut cursor)?;
    Ok(cursor.position_u64())
}

/// Validate one skyline tree from a sequential bit cursor.
///
/// Returns with the cursor just past the tree. Errors: running out of bits
/// mid-tree or mid-code is [`Decode::Truncated`]; a collapsible sibling pair
/// (an internal node's two leaf children with a zero right delta) or a delta
/// driving the running leaf height negative is [`Decode::NotCanonical`].
pub(crate) fn validate_from<C: BitCursor>(cursor: &mut C) -> Result<(), Decode>
where
    Decode: From<C::Error>,
{
    // Two bits per open ancestor, pushed [left-complete, left-was-leaf] and
    // popped in reverse order below. A packed bit stack, so depth costs bits,
    // not frames.
    let mut open: BitsMut = BitsMut::new();
    // The running leaf height. Only its sign is ever read, and only after a
    // subtracting delta: an adding delta cannot take a valid height negative,
    // and the first leaf's absolute payload is a natural.
    let mut height = Accumulator::new();
    let mut seen_leaf = false;

    loop {
        // One whole descent per unary read: the run's internal nodes opened,
        // then the leaf whose flag terminates the run.
        let internal_nodes = cursor.read_unary()?;
        for _ in 0..internal_nodes {
            open.push(false); // left-complete: the left child comes next
            open.push(false); // left-was-leaf: placeholder until it does
        }

        // The leaf: decode its payload and update the running height, through
        // the cursor's own `read_int` so a word-parallel cursor (the production
        // reader; the wire-side reader's window) takes its fast path.
        let code = cursor.read_int()?;
        // The first leaf keeps `zero_delta` false even for a zero absolute
        // payload: preorder puts it leftmost, so it is no ancestor's right
        // child and the collapsible-pair check never reads its flag.
        let mut zero_delta = false;
        if seen_leaf {
            zero_delta = code.is_zero();
            let (sign, magnitude) = unzigzag(code);
            fold_signed_int(&mut height, sign, &magnitude);
            if sign == Sign::Negative && height.sign() == Ordering::Less {
                return Err(Decode::NotCanonical); // a leaf height fell below zero
            }
        } else {
            fold_signed_int(&mut height, Sign::Positive, &code);
            seen_leaf = true;
        }

        // Close every subtree this leaf completes, walking up the open
        // ancestors; `is_leaf`/`leaf_zero_delta` describe the completed subtree
        // (the leaf itself on the first iteration).
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
