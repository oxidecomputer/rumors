use crate::error::Parse;

use super::{validate_id, Bits, BitsSlice};

/// Whether a normal-form id stream is the anonymous (empty) identity.
///
/// In the pruned encoding a `0` is structural absence, so the only empty id is
/// the empty bit stream — an O(1) check. Callers must pass already-validated
/// bits (every caller sits directly downstream of a full parse or a
/// normal-form-emitting kernel whose output is asserted at its own seam). The
/// debug assertion spot-checks the contract's O(1) consequences only — root
/// tag arity vs stream length — never a full re-parse: this helper is on
/// every decode path's metered hot loop, and asserted work here would make
/// dev builds meter a different program than the release board of record.
pub(crate) fn id_is_empty(bits: &BitsSlice) -> bool {
    debug_assert!(
        bits.is_empty()
            || (bits.len() >= 2
                && if bits[..2].any() {
                    // a root with a present child carries at least one more tag
                    bits.len() >= 4
                } else {
                    // a terminal root (`00`) is exactly one tag long
                    bits.len() == 2
                }),
        "id_is_empty requires canonical normal-form bits: bad root tag or length",
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
