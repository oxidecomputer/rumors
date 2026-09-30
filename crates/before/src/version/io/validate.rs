//! Strict validation of canonical version streams.
//!
//! One forward pass checks that cumulative leaf heights never become negative
//! and that no equal sibling leaves remain uncollapsed. The walk retains two
//! bits per open ancestor: whether its left child is complete and whether that
//! child was a leaf. An [`Accumulator`] carries the running height so a long
//! sequence of wide, cancelling deltas does not repeatedly normalize a large
//! integer. No tree or collection of decoded heights is materialized.
//!
//! [`admit`] combines the same validation with a dominance comparison when a
//! caller already has a canonical lower bound.

use core::cmp::Ordering;

use num_bigint::{BigUint, Sign};
use suanpan::Accumulator;

use crate::accumulator::BigIntAccumulator as _;
use crate::bits::stack::BitStack;
use crate::bits::{BitRead, BitsReader};
use crate::error::Decode;
use crate::version::io::regions::PayloadKind;

mod admit;

pub use admit::{dominating_from, Admission};

/// Strictly validate one complete Version stream.
///
/// The stream must be exactly one canonical tree: a tree that completes before
/// the last live bit is [`Decode::TrailingBits`]; everything else is
/// [`from_reader`]'s contract.
///
/// Test- and meter-only: the production entries run [`prefix`] and
/// [`from_reader`], which leave the tail to their callers.
#[cfg(any(test, feature = "meter"))]
pub fn whole(mut reader: BitsReader<'_>) -> Result<(), Decode> {
    from_reader(&mut reader)?;
    if reader.position() != reader.len() {
        return Err(Decode::TrailingBits);
    }
    Ok(())
}

/// Strictly validate one Version tree at the head of a reader.
///
/// A Version tree is bit-self-delimiting
/// (one complete tree), so the returned position is the end of the live stream.
/// In a byte encoding that position holds the required marker bit, followed by
/// zero padding. The position is `u64`, so every bit in a large 32-bit
/// allocation remains addressable.
pub fn prefix(mut reader: BitsReader<'_>) -> Result<u64, Decode> {
    from_reader(&mut reader)?;
    Ok(reader.position())
}

/// Validate one Version tree from a sequential bit reader.
///
/// Returns with the reader just past the tree.
///
/// # Errors
///
/// - [`Decode::Truncated`] if the reader ends within the tree or an integer;
/// - [`Decode::NotCanonical`] if sibling leaves collapse or a delta makes the
///   running height negative;
/// - [`Decode::Io`] if the reader reads from a failing byte stream. Slice
///   readers report exhaustion as [`Decode::Truncated`] instead.
pub fn from_reader<C: BitRead>(reader: &mut C) -> Result<(), Decode>
where
    Decode: From<C::Error>,
{
    // Two bits per open ancestor, pushed [left-complete, left-was-leaf] and
    // popped in reverse order below. A bit stack, so depth costs bits,
    // not frames.
    let mut open = BitStack::new();
    // The running leaf height. Only its sign is ever read, and only after a
    // subtracting delta: an adding delta cannot take a valid height negative,
    // and the first leaf's absolute payload is a natural.
    let mut height = Accumulator::new();
    let mut seen_leaf = false;

    loop {
        // One whole descent per unary read: the run's internal nodes opened,
        // then the leaf whose flag terminates the run.
        let internal_nodes = reader.read_unary()?;
        for _ in 0..internal_nodes {
            open.push(false); // left-complete: the left child comes next
            open.push(false); // left-was-leaf: placeholder until it does
        }

        // The leaf: decode its payload and update the running height, through
        // the reader's own `read_gamma` so a word-parallel implementation takes
        // its fast path.
        let code = reader.read_gamma()?;
        // The first leaf keeps `zero_delta` false even for a zero absolute
        // payload: preorder puts it leftmost, so it is no ancestor's right
        // child and the collapsible-pair check never reads its flag.
        let mut zero_delta = false;
        if seen_leaf {
            zero_delta = code == BigUint::ZERO;
            let delta = PayloadKind::Delta.decode(code);
            height.add_bigint(&delta);
            if delta.sign() == Sign::Minus && height.cmp_zero() == Ordering::Less {
                return Err(Decode::NotCanonical); // a leaf height fell below zero
            }
        } else {
            height.add_shifted_limbs(0, code.iter_u64_digits());
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
