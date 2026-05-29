//! Bit I/O: the Elias-gamma integer code, the preorder id/event encodings, and the
//! iterative `decode` with normal-form validation. See `IMPLEMENTATION_PLAN.md` §5.
//!
//! At rest, a `Party`/`Version` holds its canonical packed preorder bit stream
//! (no trailing padding), so bit-equality is semantic equality. `encode` pads that
//! stream to a byte boundary; `decode` parses and *strictly validates* normal form,
//! then stores the (canonical) consumed prefix.

use bitvec::prelude::*;

use crate::DecodeError;

#[cfg(test)]
mod tests;

/// The packed storage form: a most-significant-bit-first bit stream over bytes.
pub(crate) type Bits = BitVec<u8, Msb0>;
/// A borrowed view of the packed storage form.
pub(crate) type BitsSlice = BitSlice<u8, Msb0>;

// ───────────────────────── integer code (Elias gamma of n+1) ─────────────────────────

/// Append `n` as the Elias gamma code of `m = n + 1`: `floor(log2(m))` zero bits,
/// then `m` in `floor(log2(m)) + 1` bits, most-significant first. Cost is
/// `2*floor(log2(n+1)) + 1` bits; `0` costs a single bit. Canonical and prefix-free.
// Used by the cfg(test) oracle bridge now and by `repack` from Phase 2 onward.
#[allow(dead_code)]
pub(crate) fn encode_int(out: &mut Bits, n: u64) {
    let m = n + 1; // m >= 1
    let k = 63 - m.leading_zeros(); // floor(log2(m)) = bit_length(m) - 1
    for _ in 0..k {
        out.push(false);
    }
    for i in (0..=k).rev() {
        out.push((m >> i) & 1 == 1);
    }
}

/// Read an Elias-gamma-coded integer at `pos`, returning the value and the new
/// position. Running past the end (or a code too long to fit `u64`) is `Truncated`.
pub(crate) fn decode_int(bits: &BitsSlice, pos: usize) -> Result<(u64, usize), DecodeError> {
    let mut k = 0usize;
    loop {
        let idx = pos + k;
        if idx >= bits.len() {
            return Err(DecodeError::Truncated);
        }
        if bits[idx] {
            break; // the leading 1 of m
        }
        k += 1;
        if k > 63 {
            // m would need more than 64 bits: not representable, treat as malformed.
            return Err(DecodeError::Truncated);
        }
    }
    let start = pos + k;
    if start + k + 1 > bits.len() {
        return Err(DecodeError::Truncated);
    }
    let mut m: u64 = 0;
    for i in 0..=k {
        m = (m << 1) | (bits[start + i] as u64);
    }
    Ok((m - 1, start + k + 1))
}

// ───────────────────────── id (party) parse + validate ─────────────────────────

/// While building a node bottom-up, what we still need from the stream.
enum IdFrame {
    /// Parsed the node flag; the next subtree is the left child.
    NeedLeft,
    /// Parsed the left child (a leaf with this value, or `None` if internal); the
    /// next subtree is the right child.
    NeedRight { left_leaf: Option<bool> },
}

/// Parse one `enc_id` tree at `pos`, validating id normal form (no node whose two
/// children are leaves of equal value). Returns the position just past the tree.
/// Iterative: depth lives on an explicit stack, never the call stack.
pub(crate) fn parse_id(bits: &BitsSlice, mut pos: usize) -> Result<usize, DecodeError> {
    let mut stack: Vec<IdFrame> = Vec::new();
    loop {
        if pos >= bits.len() {
            return Err(DecodeError::Truncated);
        }
        let flag = bits[pos];
        pos += 1;

        // `enc_id(Leaf v) = 0, v`; `enc_id(Node l r) = 1, l, r`.
        let mut summary: Option<bool> = if flag {
            stack.push(IdFrame::NeedLeft);
            continue; // descend into the left child
        } else {
            if pos >= bits.len() {
                return Err(DecodeError::Truncated);
            }
            let v = bits[pos];
            pos += 1;
            Some(v)
        };

        // Attach the completed subtree to its parent, possibly completing it too.
        loop {
            match stack.pop() {
                None => return Ok(pos), // the root is complete
                Some(IdFrame::NeedLeft) => {
                    stack.push(IdFrame::NeedRight { left_leaf: summary });
                    break; // go parse the right child
                }
                Some(IdFrame::NeedRight { left_leaf }) => {
                    if let (Some(a), Some(b)) = (left_leaf, summary) {
                        if a == b {
                            return Err(DecodeError::NotCanonical); // collapsible (v,v)
                        }
                    }
                    summary = None; // this node is internal to its own parent
                }
            }
        }
    }
}

// ───────────────────────── event (version) parse + validate ─────────────────────────

/// What a parsed event subtree contributes to its parent's normal-form check: its
/// stored (relative) base, and whether it is a leaf.
#[derive(Clone, Copy)]
struct EvChild {
    base: u64,
    is_leaf: bool,
}

enum EvFrame {
    NeedLeft { base: u64 },
    NeedRight { base: u64, left: EvChild },
}

/// Parse one `enc_ev` tree at `pos`, validating event normal form: every node has at
/// least one child with base `0`, and no node's two children are equal-valued leaves.
/// Returns the position just past the tree. Iterative.
pub(crate) fn parse_ev(bits: &BitsSlice, mut pos: usize) -> Result<usize, DecodeError> {
    let mut stack: Vec<EvFrame> = Vec::new();
    loop {
        if pos >= bits.len() {
            return Err(DecodeError::Truncated);
        }
        let flag = bits[pos];
        pos += 1;
        let (base, next) = decode_int(bits, pos)?;
        pos = next;

        // `enc_ev(Leaf n) = 0, gamma(n)`; `enc_ev(Node n l r) = 1, gamma(n), l, r`.
        let mut summary = if flag {
            stack.push(EvFrame::NeedLeft { base });
            continue; // descend into the left child
        } else {
            EvChild {
                base,
                is_leaf: true,
            }
        };

        loop {
            match stack.pop() {
                None => return Ok(pos),
                Some(EvFrame::NeedLeft { base: node_base }) => {
                    stack.push(EvFrame::NeedRight {
                        base: node_base,
                        left: summary,
                    });
                    break;
                }
                Some(EvFrame::NeedRight {
                    base: node_base,
                    left,
                }) => {
                    let right = summary;
                    if left.base != 0 && right.base != 0 {
                        return Err(DecodeError::NotCanonical); // no child at base 0
                    }
                    if left.is_leaf && right.is_leaf && left.base == right.base {
                        return Err(DecodeError::NotCanonical); // collapsible (n,m,m)
                    }
                    summary = EvChild {
                        base: node_base,
                        is_leaf: false,
                    };
                }
            }
        }
    }
}

// ───────────────────────── byte packing / padding ─────────────────────────

/// Pack a canonical bit stream into bytes, zero-padding the final partial byte.
pub(crate) fn pack_to_bytes(bits: &BitsSlice) -> Vec<u8> {
    let mut padded: Bits = bits.to_bitvec();
    while !padded.len().is_multiple_of(8) {
        padded.push(false);
    }
    padded.into_vec()
}

/// Require that all bits from `pos` onward are zero padding.
pub(crate) fn require_zero_padding(bits: &BitsSlice, pos: usize) -> Result<(), DecodeError> {
    if bits[pos..].any() {
        Err(DecodeError::TrailingBits)
    } else {
        Ok(())
    }
}
