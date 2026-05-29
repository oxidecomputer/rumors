//! Bit I/O: the Elias-gamma integer code, the preorder id/event encodings, and the
//! iterative `decode` with normal-form validation. See `IMPLEMENTATION_PLAN.md` §5.
//!
//! At rest, a `Party`/`Version` holds its canonical packed preorder bit stream
//! (no trailing padding), so bit-equality is semantic equality. `encode` pads that
//! stream to a byte boundary; `decode` parses and *strictly validates* normal form,
//! then stores the (canonical) consumed prefix.

use bitvec::prelude::*;

use crate::{DecodeError, ParseError};

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

/// While building an event node bottom-up, what we still need from the stream.
enum EvFrame {
    /// Parsed the node's flag and base; the next subtree is the left child.
    NeedLeft { base: u64 },
    /// Parsed the left child; the next subtree is the right child. `base` is the node's
    /// own (relative) base; `left` is what the left child contributes to the checks.
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

// ───────────────────────── normal-form validation (string / literal entry) ─────────────────────────

/// Confirm a freshly built id bit stream is exactly one canonical-normal-form tree.
/// Wraps [`parse_id`] (the single source of truth for id normal form), mapping its
/// outcome onto [`ParseError`].
pub(crate) fn validate_id(bits: &BitsSlice) -> Result<(), ParseError> {
    match parse_id(bits, 0) {
        Ok(end) if end == bits.len() => Ok(()),
        Ok(_) => Err(ParseError::Syntax),
        Err(DecodeError::NotCanonical) => Err(ParseError::NotCanonical),
        Err(_) => Err(ParseError::Syntax),
    }
}

/// Confirm a freshly built event bit stream is exactly one canonical-normal-form tree.
/// Wraps [`parse_ev`], mapping its outcome onto [`ParseError`].
pub(crate) fn validate_ev(bits: &BitsSlice) -> Result<(), ParseError> {
    match parse_ev(bits, 0) {
        Ok(end) if end == bits.len() => Ok(()),
        Ok(_) => Err(ParseError::Syntax),
        Err(DecodeError::NotCanonical) => Err(ParseError::NotCanonical),
        Err(_) => Err(ParseError::Syntax),
    }
}

// ───────────────────────── literal builders (for `TryFrom`) ─────────────────────────

/// Whether a normal-form id stream is the anonymous (empty) identity. In canonical
/// normal form the only empty id is the single `0` leaf — any `(0, 0)` would have
/// collapsed — so this is an O(1) check. Callers must pass already-validated bits; the
/// O(1) shortcut is only sound for normal-form input, so we assert that in debug builds.
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
    encode_int(&mut b, n);
    b
}

/// Assemble an id node from two already-normal child streams, then validate the result
/// is itself normal (rejecting a collapsible `(v, v)`).
pub(crate) fn id_node(l: &BitsSlice, r: &BitsSlice) -> Result<Bits, ParseError> {
    let mut b = Bits::with_capacity(1 + l.len() + r.len());
    b.push(true); // node flag
    b.extend_from_bitslice(l);
    b.extend_from_bitslice(r);
    validate_id(&b)?;
    Ok(b)
}

/// Assemble an event node with base `n` from two already-normal child streams, then
/// validate the result is itself normal (a zero-base child, no collapsible `(n, m, m)`).
pub(crate) fn ev_node(n: u64, l: &BitsSlice, r: &BitsSlice) -> Result<Bits, ParseError> {
    let mut b = Bits::with_capacity(2 + l.len() + r.len());
    b.push(true); // node flag
    encode_int(&mut b, n);
    b.extend_from_bitslice(l);
    b.extend_from_bitslice(r);
    validate_ev(&b)?;
    Ok(b)
}

// ───────────────────────── pretty-printing (Display / Debug) ─────────────────────────

/// Write an id tree in the paper's grammar with `sep` between a node's two children
/// (`", "` for `Display`, `" "` for `Debug`). Iterative: deep ids must not overflow the
/// formatter. Leaves render as `0`/`1`, nodes as `(l<sep>r)`.
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

/// Write an event tree in the paper's grammar with `sep` between a node's parts. Leaves
/// render as `n`, nodes as `(n<sep>l<sep>r)`. Iterative, as [`write_id`].
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

// ───────────────────────── string parsing (`FromStr`) ─────────────────────────

/// A whitespace-skipping byte cursor over the input string. The grammar is pure ASCII
/// (`(`, `)`, `,`, digits, `0`/`1`), so byte-level scanning is exact.
struct Cur<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> Cur<'a> {
    fn new(s: &'a str) -> Self {
        Cur {
            b: s.as_bytes(),
            i: 0,
        }
    }

    fn skip_ws(&mut self) {
        while self.i < self.b.len() && self.b[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }

    /// The next non-whitespace byte, without consuming it.
    fn peek(&mut self) -> Option<u8> {
        self.skip_ws();
        self.b.get(self.i).copied()
    }

    /// Consume and return the next non-whitespace byte.
    fn bump(&mut self) -> Option<u8> {
        self.skip_ws();
        let c = self.b.get(self.i).copied();
        if c.is_some() {
            self.i += 1;
        }
        c
    }
}

/// Read a run of ASCII digits as a `u64` (no surrounding whitespace consumed except a
/// leading skip). Empty or overflowing input is a syntax error.
fn parse_u64(cur: &mut Cur) -> Result<u64, ParseError> {
    cur.skip_ws();
    let mut n: u64 = 0;
    let mut any = false;
    while let Some(&d) = cur.b.get(cur.i) {
        if !d.is_ascii_digit() {
            break;
        }
        any = true;
        n = n
            .checked_mul(10)
            .and_then(|x| x.checked_add(u64::from(d - b'0')))
            .ok_or(ParseError::Syntax)?;
        cur.i += 1;
    }
    if any {
        Ok(n)
    } else {
        Err(ParseError::Syntax)
    }
}

/// Parse one id tree in the paper's grammar (`0 | 1 | (i1, i2)`) into canonical bits,
/// strictly validating normal form. Iterative (explicit stack): deep nesting cannot
/// overflow.
pub(crate) fn parse_id_str(s: &str) -> Result<Bits, ParseError> {
    /// A pending node being parsed.
    enum Frame {
        /// Node open, left child parsed: expect the separator, then the right child.
        NeedLeft,
        /// Right child parsed: expect the closing `)`.
        NeedRight,
    }
    let mut cur = Cur::new(s);
    let mut bits = Bits::new();
    let mut stack: Vec<Frame> = Vec::new();
    loop {
        match cur.bump() {
            Some(b'(') => {
                bits.push(true);
                stack.push(Frame::NeedLeft);
                continue;
            }
            Some(b'0') => {
                bits.push(false);
                bits.push(false);
            }
            Some(b'1') => {
                bits.push(false);
                bits.push(true);
            }
            _ => return Err(ParseError::Syntax),
        }
        loop {
            match stack.pop() {
                None => {
                    if cur.peek().is_some() {
                        return Err(ParseError::Syntax);
                    }
                    validate_id(&bits)?;
                    return Ok(bits);
                }
                Some(Frame::NeedLeft) => {
                    if cur.bump() != Some(b',') {
                        return Err(ParseError::Syntax);
                    }
                    stack.push(Frame::NeedRight);
                    break;
                }
                Some(Frame::NeedRight) => {
                    if cur.bump() != Some(b')') {
                        return Err(ParseError::Syntax);
                    }
                }
            }
        }
    }
}

/// Parse one event tree in the paper's grammar (`n | (n, e1, e2)`) into canonical bits,
/// strictly validating normal form. Iterative, as [`parse_id_str`].
pub(crate) fn parse_ev_str(s: &str) -> Result<Bits, ParseError> {
    /// A pending node being parsed.
    enum Frame {
        /// Node open, left child parsed: expect the separator, then the right child.
        NeedLeft,
        /// Right child parsed: expect the closing `)`.
        NeedRight,
    }
    let mut cur = Cur::new(s);
    let mut bits = Bits::new();
    let mut stack: Vec<Frame> = Vec::new();
    loop {
        match cur.peek() {
            Some(b'(') => {
                cur.bump();
                bits.push(true);
                let base = parse_u64(&mut cur)?;
                encode_int(&mut bits, base);
                if cur.bump() != Some(b',') {
                    return Err(ParseError::Syntax);
                }
                stack.push(Frame::NeedLeft);
                continue;
            }
            Some(c) if c.is_ascii_digit() => {
                let n = parse_u64(&mut cur)?;
                bits.push(false);
                encode_int(&mut bits, n);
            }
            _ => return Err(ParseError::Syntax),
        }
        loop {
            match stack.pop() {
                None => {
                    if cur.peek().is_some() {
                        return Err(ParseError::Syntax);
                    }
                    validate_ev(&bits)?;
                    return Ok(bits);
                }
                Some(Frame::NeedLeft) => {
                    if cur.bump() != Some(b',') {
                        return Err(ParseError::Syntax);
                    }
                    stack.push(Frame::NeedRight);
                    break;
                }
                Some(Frame::NeedRight) => {
                    if cur.bump() != Some(b')') {
                        return Err(ParseError::Syntax);
                    }
                }
            }
        }
    }
}

/// Parse a stamp `(i, e)` into its id and event bit streams. Splits at the top-level
/// (depth-0) comma, then parses each side. Iterative.
pub(crate) fn parse_clock_str(s: &str) -> Result<(Bits, Bits), ParseError> {
    let t = s.trim();
    let bytes = t.as_bytes();
    if bytes.first() != Some(&b'(') || bytes.last() != Some(&b')') {
        return Err(ParseError::Syntax);
    }
    let inner = &t[1..t.len() - 1];
    let mut depth: i32 = 0;
    let mut split = None;
    for (k, &c) in inner.as_bytes().iter().enumerate() {
        match c {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth < 0 {
                    return Err(ParseError::Syntax);
                }
            }
            b',' if depth == 0 => {
                split = Some(k);
                break;
            }
            _ => {}
        }
    }
    let k = split.ok_or(ParseError::Syntax)?;
    let id_bits = parse_id_str(&inner[..k])?;
    let ev_bits = parse_ev_str(&inner[k + 1..])?;
    Ok((id_bits, ev_bits))
}
