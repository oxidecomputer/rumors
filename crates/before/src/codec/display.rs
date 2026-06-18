use crate::recurse::descend;

use super::{decode_int, BitsSlice};

/// Write an id tree in the paper's grammar with `sep` between a node's two
/// children (`", "`). Leaves render as `0`/`1`, nodes as `(l<sep>r)`. Recursive,
/// guarded by [`crate::recurse`] so deep ids do not overflow the formatter. The
/// empty `0` id (no bits) renders as `0`.
pub(crate) fn write_id(
    bits: &BitsSlice,
    f: &mut core::fmt::Formatter<'_>,
    sep: &str,
) -> core::fmt::Result {
    if bits.is_empty() {
        return f.write_str("0");
    }
    descend!(0, write_id_node(bits, 0, f, sep, 0)).map(|_end| ())
}

/// Write the id subtree at `pos`, returning the position just past it. Each node
/// is a 2-bit presence tag (bit 0 = left present, bit 1 = right present); a
/// terminal renders as `1` and an absent child as `0`. Routed through the
/// amortized stack-growth guard.
fn write_id_node(
    bits: &BitsSlice,
    pos: usize,
    f: &mut core::fmt::Formatter<'_>,
    sep: &str,
    depth: usize,
) -> Result<usize, core::fmt::Error> {
    let left = bits[pos];
    let right = bits[pos + 1];
    if !left && !right {
        f.write_str("1")?; // terminal
        return Ok(pos + 2);
    }
    f.write_str("(")?;
    let mut next = pos + 2;
    if left {
        next = descend!(depth + 1, write_id_node(bits, next, f, sep, depth + 1))?;
    } else {
        f.write_str("0")?; // absent left child
    }
    f.write_str(sep)?;
    if right {
        next = descend!(depth + 1, write_id_node(bits, next, f, sep, depth + 1))?;
    } else {
        f.write_str("0")?; // absent right child
    }
    f.write_str(")")?;
    Ok(next)
}

/// Write an event tree in the paper's grammar with `sep` between a node's
/// parts. Leaves render as `n`, nodes as `(n<sep>l<sep>r)`. Recursive, as
/// [`write_id`].
pub(crate) fn write_ev(
    bits: &BitsSlice,
    f: &mut core::fmt::Formatter<'_>,
    sep: &str,
) -> core::fmt::Result {
    descend!(0, write_ev_node(bits, 0, f, sep, 0)).map(|_end| ())
}

/// Write the event subtree at `pos`, returning the position just past it. Routed
/// through the amortized stack-growth guard.
fn write_ev_node(
    bits: &BitsSlice,
    pos: usize,
    f: &mut core::fmt::Formatter<'_>,
    sep: &str,
    depth: usize,
) -> Result<usize, core::fmt::Error> {
    let internal = bits[pos];
    let (base, next) = decode_int(bits, pos + 1).expect("a stored event tree is canonical");
    if !internal {
        write!(f, "{base}")?;
        return Ok(next);
    }
    write!(f, "({base}{sep}")?;
    let mid = descend!(depth + 1, write_ev_node(bits, next, f, sep, depth + 1))?;
    f.write_str(sep)?;
    let end = descend!(depth + 1, write_ev_node(bits, mid, f, sep, depth + 1))?;
    f.write_str(")")?;
    Ok(end)
}
