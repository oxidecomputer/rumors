use crate::error::{Decode, Parse};

use super::{decode_int_from, Base, BitCursor, BitsSlice, SliceCursor};

/// While building a node bottom-up, what we still need from the stream.
enum IdFrame {
    /// A both-present node: the next subtree is its left child.
    BothNeedLeft,
    /// A both-present node whose left child is parsed (a terminal? — needed for
    /// the `(1, 1)` check); the next subtree is its right child.
    BothNeedRight { left_terminal: bool },
    /// A unary node (left- or right-only): the next subtree is its one child.
    UnaryNeedChild,
}

/// Parse one packed id tree at `pos`, validating id normal form (no node with
/// two terminal children, that is `(1, 1)`).
///
/// Returns the position just past the tree. Iterative: depth lives on an
/// explicit stack, never the call stack.
///
/// Each node is a 2-bit presence tag (bit 0 = left child follows, bit 1 = right
/// child follows): `00` a terminal, `10`/`01` a unary node, `11` a both-present
/// node. A `0` is never a node, so an empty input is the `0` tree itself (only
/// valid at the root; `Party::decode` rejects the resulting anonymous id).
pub(crate) fn parse_id(bits: &BitsSlice, pos: usize) -> Result<usize, Decode> {
    // The empty `0` tree is representable only as an empty root input.
    if pos == bits.len() {
        return Ok(pos);
    }
    let mut cursor = SliceCursor::new(bits, pos);
    parse_id_from(&mut cursor)
}

/// Parse and validate one id tree from a sequential bit cursor.
pub(crate) fn parse_id_from(cursor: &mut impl BitCursor) -> Result<usize, Decode> {
    let mut stack: Vec<IdFrame> = Vec::new();
    loop {
        let left = cursor.read_bit()?;
        let right = cursor.read_bit()?;

        // `summary` is whether the just-completed subtree is a terminal — the
        // only fact a parent needs, to reject `(1, 1)`.
        let mut summary = match (left, right) {
            (true, true) => {
                stack.push(IdFrame::BothNeedLeft);
                continue; // descend into the left child
            }
            (true, false) | (false, true) => {
                stack.push(IdFrame::UnaryNeedChild);
                continue; // descend into the one present child
            }
            (false, false) => true, // a terminal
        };

        // Attach the completed subtree to its parent, possibly completing it too.
        loop {
            match stack.pop() {
                None => return Ok(cursor.position()), // the root is complete
                Some(IdFrame::BothNeedLeft) => {
                    stack.push(IdFrame::BothNeedRight {
                        left_terminal: summary,
                    });
                    break; // go parse the right child
                }
                Some(IdFrame::BothNeedRight { left_terminal }) => {
                    if left_terminal && summary {
                        return Err(Decode::NotCanonical); // collapsible (1, 1)
                    }
                    summary = false; // this node is internal to its own parent
                }
                Some(IdFrame::UnaryNeedChild) => {
                    summary = false; // a unary node is internal, never a terminal
                }
            }
        }
    }
}

/// What a parsed event subtree contributes to its parent's normal-form check:
/// its stored (relative) base, and whether it is a leaf.
#[derive(Clone)]
struct EvChild {
    base: Base,
    is_leaf: bool,
}

/// While building an event node bottom-up, what we still need from the stream.
enum EvFrame {
    /// Parsed the node's flag and base; the next subtree is the left child.
    NeedLeft { base: Base },
    /// Parsed the left child; the next subtree is the right child. `base` is the node's
    /// own (relative) base; `left` is what the left child contributes to the checks.
    NeedRight { base: Base, left: EvChild },
}

/// Parse one `enc_ev` tree at `pos`, validating event normal form: every node
/// has at least one child with base `0`, and no node's two children are
/// equal-valued leaves. Returns the position just past the tree. Iterative.
pub(crate) fn parse_ev(bits: &BitsSlice, pos: usize) -> Result<usize, Decode> {
    let mut cursor = SliceCursor::new(bits, pos);
    parse_ev_from(&mut cursor)
}

/// Parse and validate one event tree from a sequential bit cursor.
pub(crate) fn parse_ev_from(cursor: &mut impl BitCursor) -> Result<usize, Decode> {
    let mut stack: Vec<EvFrame> = Vec::new();
    loop {
        let flag = cursor.read_bit()?;
        let base = decode_int_from(cursor)?;

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
                None => return Ok(cursor.position()),
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
                    if left.base != Base::ZERO && right.base != Base::ZERO {
                        return Err(Decode::NotCanonical); // no child at base 0
                    }
                    if left.is_leaf && right.is_leaf && left.base == right.base {
                        return Err(Decode::NotCanonical); // collapsible (n,m,m)
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

/// Confirm a freshly built id bit stream is exactly one canonical-normal-form
/// tree. Wraps [`parse_id`] (the single source of truth for id normal form),
/// mapping its outcome onto [`Parse`].
pub(crate) fn validate_id(bits: &BitsSlice) -> Result<(), Parse> {
    match parse_id(bits, 0) {
        Ok(end) if end == bits.len() => Ok(()),
        Ok(_) => Err(Parse::Syntax),
        Err(Decode::NotCanonical) => Err(Parse::NotCanonical),
        Err(_) => Err(Parse::Syntax),
    }
}

/// Confirm a freshly built event bit stream is exactly one
/// canonical-normal-form tree. Wraps [`parse_ev`], mapping its outcome onto
/// [`Parse`].
pub(crate) fn validate_ev(bits: &BitsSlice) -> Result<(), Parse> {
    match parse_ev(bits, 0) {
        Ok(end) if end == bits.len() => Ok(()),
        Ok(_) => Err(Parse::Syntax),
        Err(Decode::NotCanonical) => Err(Parse::NotCanonical),
        Err(_) => Err(Parse::Syntax),
    }
}
