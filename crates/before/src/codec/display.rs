use super::{decode_int, BitsSlice};

/// Write an id tree in the paper's grammar with `sep` between a node's two
/// children (`", "`). Iterative: deep ids must not overflow the formatter.
/// Leaves render as `0`/`1`, nodes as `(l<sep>r)`.
pub(crate) fn write_id(
    bits: &BitsSlice,
    f: &mut core::fmt::Formatter<'_>,
    sep: &str,
) -> core::fmt::Result {
    /// A pending node in the preorder print.
    enum Frame {
        /// Node open, left child printed: emit the separator, then the right child.
        NeedLeft,
        /// Right child printed: emit the closing `)`.
        NeedRight,
    }
    let mut pos = 0;
    let mut stack: Vec<Frame> = Vec::new();
    loop {
        let flag = bits[pos];
        pos += 1;
        if flag {
            f.write_str("(")?;
            stack.push(Frame::NeedLeft);
            continue;
        }
        f.write_str(if bits[pos] { "1" } else { "0" })?;
        pos += 1;
        loop {
            match stack.pop() {
                None => return Ok(()),
                Some(Frame::NeedLeft) => {
                    f.write_str(sep)?;
                    stack.push(Frame::NeedRight);
                    break;
                }
                Some(Frame::NeedRight) => f.write_str(")")?,
            }
        }
    }
}

/// Write an event tree in the paper's grammar with `sep` between a node's
/// parts. Leaves render as `n`, nodes as `(n<sep>l<sep>r)`. Iterative, as
/// [`write_id`].
pub(crate) fn write_ev(
    bits: &BitsSlice,
    f: &mut core::fmt::Formatter<'_>,
    sep: &str,
) -> core::fmt::Result {
    /// A pending node in the preorder print.
    enum Frame {
        /// Node open, left child printed: emit the separator, then the right child.
        NeedLeft,
        /// Right child printed: emit the closing `)`.
        NeedRight,
    }
    let mut pos = 0;
    let mut stack: Vec<Frame> = Vec::new();
    loop {
        let internal = bits[pos];
        let (base, next) = decode_int(bits, pos + 1).expect("a stored event tree is canonical");
        pos = next;
        if internal {
            write!(f, "({base}{sep}")?;
            stack.push(Frame::NeedLeft);
            continue;
        }
        write!(f, "{base}")?;
        loop {
            match stack.pop() {
                None => return Ok(()),
                Some(Frame::NeedLeft) => {
                    f.write_str(sep)?;
                    stack.push(Frame::NeedRight);
                    break;
                }
                Some(Frame::NeedRight) => f.write_str(")")?,
            }
        }
    }
}
