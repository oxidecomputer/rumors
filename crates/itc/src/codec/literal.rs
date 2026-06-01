use crate::ParseError;

use super::{encode_int, parse_id, validate_ev, validate_id, Base, Bits, BitsSlice};

/// Whether a normal-form id stream is the anonymous (empty) identity. In
/// canonical normal form the only empty id is the single `0` leaf — any `(0,
/// 0)` would have collapsed — so this is an O(1) check. Callers must pass
/// already-validated bits; the O(1) shortcut is only sound for normal-form
/// input, so we assert that in debug builds.
pub(crate) fn id_is_empty(bits: &BitsSlice) -> bool {
    debug_assert!(
        matches!(parse_id(bits, 0), Ok(end) if end == bits.len()),
        "id_is_empty requires canonical normal-form bits",
    );
    bits.len() == 2 && !bits[0] && !bits[1]
}

/// The bits for an id leaf (`0` empty, `1` full).
pub(crate) fn id_leaf(v: bool) -> Bits {
    let mut b = Bits::with_capacity(2);
    b.push(false); // leaf flag
    b.push(v);
    b
}

/// The bits for an event leaf with base `n`.
pub(crate) fn ev_leaf(n: u64) -> Bits {
    let mut b = Bits::new();
    b.push(false); // leaf flag
    encode_int(&mut b, &Base::from(n));
    b
}

/// Assemble an id node from two already-normal child streams, then validate the
/// result is itself normal (rejecting a collapsible `(v, v)`).
pub(crate) fn id_node(l: &BitsSlice, r: &BitsSlice) -> Result<Bits, ParseError> {
    let mut b = Bits::with_capacity(1 + l.len() + r.len());
    b.push(true); // node flag
    b.extend_from_bitslice(l);
    b.extend_from_bitslice(r);
    validate_id(&b)?;
    Ok(b)
}

/// Assemble an event node with base `n` from two already-normal child streams,
/// then validate the result is itself normal (a zero-base child, no collapsible
/// `(n, m, m)`).
pub(crate) fn ev_node(n: u64, l: &BitsSlice, r: &BitsSlice) -> Result<Bits, ParseError> {
    let mut b = Bits::with_capacity(2 + l.len() + r.len());
    b.push(true); // node flag
    encode_int(&mut b, &Base::from(n));
    b.extend_from_bitslice(l);
    b.extend_from_bitslice(r);
    validate_ev(&b)?;
    Ok(b)
}
