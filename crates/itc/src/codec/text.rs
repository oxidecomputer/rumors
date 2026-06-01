use crate::ParseError;

use super::{encode_int, validate_ev, validate_id, Base, Bits};

/// A whitespace-skipping byte cursor over the input string. The grammar is pure ASCII
/// (`(`, `)`, `,`, digits, `0`/`1`), so byte-level scanning is exact.
struct Cur<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Cur<'a> {
    fn new(s: &'a str) -> Self {
        Cur {
            bytes: s.as_bytes(),
            pos: 0,
        }
    }

    fn skip_ws(&mut self) {
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    /// The next non-whitespace byte, without consuming it.
    fn peek(&mut self) -> Option<u8> {
        self.skip_ws();
        self.bytes.get(self.pos).copied()
    }

    /// Consume and return the next non-whitespace byte.
    fn bump(&mut self) -> Option<u8> {
        self.skip_ws();
        let c = self.bytes.get(self.pos).copied();
        if c.is_some() {
            self.pos += 1;
        }
        c
    }
}

/// Read a run of ASCII digits as a [`Base`] magnitude (no surrounding whitespace
/// consumed except a leading skip). Arbitrary width: an event base has no value cap.
/// Empty input is a syntax error.
fn parse_base(cur: &mut Cur) -> Result<Base, ParseError> {
    cur.skip_ws();
    let mut n = Base::ZERO;
    let mut any = false;
    while let Some(&d) = cur.bytes.get(cur.pos) {
        if !d.is_ascii_digit() {
            break;
        }
        any = true;
        n *= 10u32;
        n += u32::from(d - b'0');
        cur.pos += 1;
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
                let base = parse_base(&mut cur)?;
                encode_int(&mut bits, &base);
                if cur.bump() != Some(b',') {
                    return Err(ParseError::Syntax);
                }
                stack.push(Frame::NeedLeft);
                continue;
            }
            Some(c) if c.is_ascii_digit() => {
                let n = parse_base(&mut cur)?;
                bits.push(false);
                encode_int(&mut bits, &n);
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
