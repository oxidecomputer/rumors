use crate::{DecodeError, ParseError};

use super::{decode_int, Base, BitsSlice};

/// While building a node bottom-up, what we still need from the stream.
enum IdFrame {
    /// Parsed the node flag; the next subtree is the left child.
    NeedLeft,
    /// Parsed the left child (a leaf with this value, or `None` if internal); the
    /// next subtree is the right child.
    NeedRight { left_leaf: Option<bool> },
}

/// Parse one `enc_id` tree at `pos`, validating id normal form (no node whose
/// two children are leaves of equal value). Returns the position just past the
/// tree. Iterative: depth lives on an explicit stack, never the call stack.
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
                    if left.base != Base::ZERO && right.base != Base::ZERO {
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

/// Confirm a freshly built id bit stream is exactly one canonical-normal-form
/// tree. Wraps [`parse_id`] (the single source of truth for id normal form),
/// mapping its outcome onto [`ParseError`].
pub(crate) fn validate_id(bits: &BitsSlice) -> Result<(), ParseError> {
    match parse_id(bits, 0) {
        Ok(end) if end == bits.len() => Ok(()),
        Ok(_) => Err(ParseError::Syntax),
        Err(DecodeError::NotCanonical) => Err(ParseError::NotCanonical),
        Err(_) => Err(ParseError::Syntax),
    }
}

/// Confirm a freshly built event bit stream is exactly one
/// canonical-normal-form tree. Wraps [`parse_ev`], mapping its outcome onto
/// [`ParseError`].
pub(crate) fn validate_ev(bits: &BitsSlice) -> Result<(), ParseError> {
    match parse_ev(bits, 0) {
        Ok(end) if end == bits.len() => Ok(()),
        Ok(_) => Err(ParseError::Syntax),
        Err(DecodeError::NotCanonical) => Err(ParseError::NotCanonical),
        Err(_) => Err(ParseError::Syntax),
    }
}
