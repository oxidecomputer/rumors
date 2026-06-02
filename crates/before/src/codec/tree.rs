use crate::{DecodeError, ParseError};

use super::{decode_int, Base, BitsSlice};

/// Parse one `enc_id` tree at `pos`, validating id normal form (no node whose
/// two children are leaves of equal value). Returns the position just past the
/// tree. Recursive, guarded by [`crate::recurse`] so deep input grows the stack
/// onto the heap rather than overflowing.
pub(crate) fn parse_id(bits: &BitsSlice, pos: usize) -> Result<usize, DecodeError> {
    parse_id_node(bits, pos, 0).map(|(_summary, end)| end)
}

/// Parse one id subtree at `pos`. Returns `(summary, end)`: the leaf's value
/// (`Some`) or `None` for an internal node, and the position just past the
/// subtree. Routed through the amortized stack-growth guard.
fn parse_id_node(
    bits: &BitsSlice,
    pos: usize,
    depth: usize,
) -> Result<(Option<bool>, usize), DecodeError> {
    crate::recurse::guarded(depth, move || {
        if pos >= bits.len() {
            return Err(DecodeError::Truncated);
        }
        // `enc_id(Leaf v) = 0, v`; `enc_id(Node l r) = 1, l, r`.
        if !bits[pos] {
            if pos + 1 >= bits.len() {
                return Err(DecodeError::Truncated);
            }
            return Ok((Some(bits[pos + 1]), pos + 2));
        }
        let (left, mid) = parse_id_node(bits, pos + 1, depth + 1)?;
        let (right, end) = parse_id_node(bits, mid, depth + 1)?;
        if let (Some(a), Some(b)) = (left, right) {
            if a == b {
                return Err(DecodeError::NotCanonical); // collapsible (v,v)
            }
        }
        Ok((None, end)) // this node is internal to its own parent
    })
}

/// What a parsed event subtree contributes to its parent's normal-form check:
/// its stored (relative) base, and whether it is a leaf.
struct EvChild {
    base: Base,
    is_leaf: bool,
}

/// Parse one `enc_ev` tree at `pos`, validating event normal form: every node
/// has at least one child with base `0`, and no node's two children are
/// equal-valued leaves. Returns the position just past the tree. Recursive,
/// guarded by [`crate::recurse`] against deep input.
pub(crate) fn parse_ev(bits: &BitsSlice, pos: usize) -> Result<usize, DecodeError> {
    parse_ev_node(bits, pos, 0).map(|(_summary, end)| end)
}

/// Parse one event subtree at `pos`. Returns `(summary, end)`: what the subtree
/// contributes to its parent's checks, and the position just past it. Routed
/// through the amortized stack-growth guard.
fn parse_ev_node(
    bits: &BitsSlice,
    pos: usize,
    depth: usize,
) -> Result<(EvChild, usize), DecodeError> {
    crate::recurse::guarded(depth, move || {
        if pos >= bits.len() {
            return Err(DecodeError::Truncated);
        }
        let flag = bits[pos];
        let (base, next) = decode_int(bits, pos + 1)?;
        // `enc_ev(Leaf n) = 0, gamma(n)`; `enc_ev(Node n l r) = 1, gamma(n), l, r`.
        if !flag {
            return Ok((
                EvChild {
                    base,
                    is_leaf: true,
                },
                next,
            ));
        }
        let (left, mid) = parse_ev_node(bits, next, depth + 1)?;
        let (right, end) = parse_ev_node(bits, mid, depth + 1)?;
        if left.base != Base::ZERO && right.base != Base::ZERO {
            return Err(DecodeError::NotCanonical); // no child at base 0
        }
        if left.is_leaf && right.is_leaf && left.base == right.base {
            return Err(DecodeError::NotCanonical); // collapsible (n,m,m)
        }
        Ok((
            EvChild {
                base,
                is_leaf: false,
            },
            end,
        ))
    })
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
