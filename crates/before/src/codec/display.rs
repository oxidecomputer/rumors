use super::{decode_int, BitsSlice};

/// Write an id tree in the paper's grammar with `sep` between a node's two
/// children (`", "`). Leaves render as `0`/`1`, nodes as `(l<sep>r)`. Recursive,
/// guarded by [`crate::recurse`] so deep ids do not overflow the formatter.
pub(crate) fn write_id(
    bits: &BitsSlice,
    f: &mut core::fmt::Formatter<'_>,
    sep: &str,
) -> core::fmt::Result {
    write_id_node(bits, 0, f, sep, 0).map(|_end| ())
}

/// Write the id subtree at `pos`, returning the position just past it. Routed
/// through the amortized stack-growth guard.
fn write_id_node(
    bits: &BitsSlice,
    pos: usize,
    f: &mut core::fmt::Formatter<'_>,
    sep: &str,
    depth: usize,
) -> Result<usize, core::fmt::Error> {
    crate::recurse::guarded(depth, move || {
        if !bits[pos] {
            f.write_str(if bits[pos + 1] { "1" } else { "0" })?;
            return Ok(pos + 2);
        }
        f.write_str("(")?;
        let mid = write_id_node(bits, pos + 1, f, sep, depth + 1)?;
        f.write_str(sep)?;
        let end = write_id_node(bits, mid, f, sep, depth + 1)?;
        f.write_str(")")?;
        Ok(end)
    })
}

/// Write an event tree in the paper's grammar with `sep` between a node's
/// parts. Leaves render as `n`, nodes as `(n<sep>l<sep>r)`. Recursive, as
/// [`write_id`].
pub(crate) fn write_ev(
    bits: &BitsSlice,
    f: &mut core::fmt::Formatter<'_>,
    sep: &str,
) -> core::fmt::Result {
    write_ev_node(bits, 0, f, sep, 0).map(|_end| ())
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
    crate::recurse::guarded(depth, move || {
        let internal = bits[pos];
        let (base, next) = decode_int(bits, pos + 1).expect("a stored event tree is canonical");
        if !internal {
            write!(f, "{base}")?;
            return Ok(next);
        }
        write!(f, "({base}{sep}")?;
        let mid = write_ev_node(bits, next, f, sep, depth + 1)?;
        f.write_str(sep)?;
        let end = write_ev_node(bits, mid, f, sep, depth + 1)?;
        f.write_str(")")?;
        Ok(end)
    })
}
