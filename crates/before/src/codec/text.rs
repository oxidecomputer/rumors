use crate::error::Parse;
use crate::recurse::descend;

use super::{encode_int, validate_ev, validate_id, Base, Bits};

/// A whitespace-skipping byte cursor over the input string. The grammar is pure
/// ASCII (`(`, `)`, `,`, digits, `0`/`1`), so byte-level scanning is exact.
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

/// Read a run of ASCII digits as a [`Base`] magnitude (no surrounding
/// whitespace consumed except a leading skip). Arbitrary width: an event base
/// has no value cap. Empty input is a syntax error.
fn parse_base(cur: &mut Cur) -> Result<Base, Parse> {
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
        Err(Parse::Syntax)
    }
}

/// Parse one id tree in the paper's grammar (`0 | 1 | (i1, i2)`) into canonical
/// bits, strictly validating normal form. Recursive, guarded by
/// [`crate::recurse`] so deep nesting grows the stack onto the heap rather than
/// overflowing.
pub(crate) fn parse_id_str(s: &str) -> Result<Bits, Parse> {
    let mut cur = Cur::new(s);
    let mut bits = Bits::new();
    descend!(0, parse_id_node(&mut cur, &mut bits, 0))?;
    if cur.peek().is_some() {
        return Err(Parse::Syntax); // trailing junk
    }
    validate_id(&bits)?;
    Ok(bits)
}

/// Parse one id subtree, appending its canonical bits. Routed through the
/// amortized stack-growth guard.
fn parse_id_node(cur: &mut Cur, bits: &mut Bits, depth: usize) -> Result<(), Parse> {
    match cur.bump() {
        Some(b'(') => {
            bits.push(true);
            descend!(depth + 1, parse_id_node(cur, bits, depth + 1))?; // left
            if cur.bump() != Some(b',') {
                return Err(Parse::Syntax);
            }
            descend!(depth + 1, parse_id_node(cur, bits, depth + 1))?; // right
            if cur.bump() != Some(b')') {
                return Err(Parse::Syntax);
            }
            Ok(())
        }
        Some(b'0') => {
            bits.push(false);
            bits.push(false);
            Ok(())
        }
        Some(b'1') => {
            bits.push(false);
            bits.push(true);
            Ok(())
        }
        _ => Err(Parse::Syntax),
    }
}

/// Parse one event tree in the paper's grammar (`n | (n, e1, e2)`) into
/// canonical bits, strictly validating normal form. Recursive, as
/// [`parse_id_str`].
pub(crate) fn parse_ev_str(s: &str) -> Result<Bits, Parse> {
    let mut cur = Cur::new(s);
    let mut bits = Bits::new();
    descend!(0, parse_ev_node(&mut cur, &mut bits, 0))?;
    if cur.peek().is_some() {
        return Err(Parse::Syntax); // trailing junk
    }
    validate_ev(&bits)?;
    Ok(bits)
}

/// Parse one event subtree, appending its canonical bits. Routed through the
/// amortized stack-growth guard.
fn parse_ev_node(cur: &mut Cur, bits: &mut Bits, depth: usize) -> Result<(), Parse> {
    match cur.peek() {
        Some(b'(') => {
            cur.bump();
            bits.push(true);
            let base = parse_base(cur)?;
            encode_int(bits, &base);
            if cur.bump() != Some(b',') {
                return Err(Parse::Syntax);
            }
            descend!(depth + 1, parse_ev_node(cur, bits, depth + 1))?; // left
            if cur.bump() != Some(b',') {
                return Err(Parse::Syntax);
            }
            descend!(depth + 1, parse_ev_node(cur, bits, depth + 1))?; // right
            if cur.bump() != Some(b')') {
                return Err(Parse::Syntax);
            }
            Ok(())
        }
        Some(c) if c.is_ascii_digit() => {
            let n = parse_base(cur)?;
            bits.push(false);
            encode_int(bits, &n);
            Ok(())
        }
        _ => Err(Parse::Syntax),
    }
}

/// Parse a stamp `(i, e)` into its id and event bit streams. Splits at the
/// top-level (depth-0) comma, then parses each side. Iterative.
pub(crate) fn parse_clock_str(s: &str) -> Result<(Bits, Bits), Parse> {
    let t = s.trim();
    let bytes = t.as_bytes();
    if bytes.first() != Some(&b'(') || bytes.last() != Some(&b')') {
        return Err(Parse::Syntax);
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
                    return Err(Parse::Syntax);
                }
            }
            b',' if depth == 0 => {
                split = Some(k);
                break;
            }
            _ => {}
        }
    }
    let k = split.ok_or(Parse::Syntax)?;
    let id_bits = parse_id_str(&inner[..k])?;
    let ev_bits = parse_ev_str(&inner[k + 1..])?;
    Ok((id_bits, ev_bits))
}
