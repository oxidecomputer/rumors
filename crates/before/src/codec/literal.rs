use crate::error::Parse;

use super::{encode_int, parse_id, validate_ev, validate_id, Base, Bits, BitsSlice};

/// Whether a normal-form id stream is the anonymous (empty) identity.
///
/// In the pruned encoding a `0` is structural absence, so the only empty id is
/// the empty bit stream — an O(1) check. Callers must pass already-validated
/// bits; the O(1) shortcut is only sound for normal-form input, so we assert
/// that in debug builds.
pub(crate) fn id_is_empty(bits: &BitsSlice) -> bool {
    debug_assert!(
        matches!(parse_id(bits, 0), Ok(end) if end == bits.len()),
        "id_is_empty requires canonical normal-form bits",
    );
    bits.is_empty()
}

/// The bits for an id leaf: the empty stream for `0` (absence), the terminal tag
/// `00` for `1`.
pub(crate) fn id_leaf(v: bool) -> Bits {
    let mut b = Bits::with_capacity(2);
    if v {
        b.push(false); // terminal tag `00`: an owned leaf, no children
        b.push(false);
    }
    b
}

/// The bits for an event leaf with base `n`.
pub(crate) fn ev_leaf(n: u64) -> Bits {
    let mut b = Bits::new();
    b.push(false); // leaf flag
    encode_int(&mut b, &Base::from(n));
    b
}

/// Whether `bits` is exactly the terminal tag `00` (the `1` leaf).
fn id_is_terminal(bits: &BitsSlice) -> bool {
    bits.len() == 2 && !bits[0] && !bits[1]
}

/// Assemble an id node from two already-normal child streams: a `0` child is the
/// empty stream (absent), so the 2-bit tag records which children are present.
///
/// Rejects a collapsible `(0, 0)` or `(1, 1)`, then validates the result.
pub(crate) fn id_node(l: &BitsSlice, r: &BitsSlice) -> Result<Bits, Parse> {
    if l.is_empty() && r.is_empty() {
        return Err(Parse::NotCanonical); // (0, 0) → 0, not a node
    }
    if id_is_terminal(l) && id_is_terminal(r) {
        return Err(Parse::NotCanonical); // (1, 1) → 1, not a node
    }
    let mut b = Bits::with_capacity(2 + l.len() + r.len());
    b.push(!l.is_empty()); // bit 0 = left present
    b.push(!r.is_empty()); // bit 1 = right present
    b.extend_from_bitslice(l);
    b.extend_from_bitslice(r);
    validate_id(&b)?;
    Ok(b)
}

/// Assemble an event node with base `n` from two already-normal child streams,
/// then validate the result is itself normal (a zero-base child, no collapsible
/// `(n, m, m)`).
pub(crate) fn ev_node(n: u64, l: &BitsSlice, r: &BitsSlice) -> Result<Bits, Parse> {
    let mut b = Bits::with_capacity(2 + l.len() + r.len());
    b.push(true); // node flag
    encode_int(&mut b, &Base::from(n));
    b.extend_from_bitslice(l);
    b.extend_from_bitslice(r);
    validate_ev(&b)?;
    Ok(b)
}
