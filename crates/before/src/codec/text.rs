use crate::error::Parse;

use super::{validate_id, Base, BitsMut};

/// A whitespace-skipping byte cursor over the input string. The grammar is pure
/// ASCII (`(`, `)`, `,`, digits, `0`/`1`), so byte-level scanning is exact.
///
/// Shared with the skyline text kernel (`version::skyline::text`), whose parser
/// must make byte-identical grammar decisions to this module's.
pub(crate) struct Cur<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Cur<'a> {
    pub(crate) fn new(s: &'a str) -> Self {
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
    pub(crate) fn peek(&mut self) -> Option<u8> {
        self.skip_ws();
        self.bytes.get(self.pos).copied()
    }

    /// Consume and return the next non-whitespace byte.
    pub(crate) fn bump(&mut self) -> Option<u8> {
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
///
/// The cursor slices the whole digit run and hands it to
/// [`Base::parse_decimal`], which delegates the radix conversion to the
/// backend's subquadratic divide-and-conquer parser; leading zeros are
/// value-preserving (`"007"` is 7), exactly as digit-at-a-time accumulation
/// would read them.
pub(crate) fn parse_base(cur: &mut Cur) -> Result<Base, Parse> {
    cur.skip_ws();
    let start = cur.pos;
    while cur.bytes.get(cur.pos).is_some_and(|d| d.is_ascii_digit()) {
        cur.pos += 1;
    }
    if cur.pos == start {
        return Err(Parse::Syntax);
    }
    let digits = core::str::from_utf8(&cur.bytes[start..cur.pos])
        .expect("an ASCII digit run is valid UTF-8");
    Ok(Base::parse_decimal(digits))
}

/// Parse one id tree in the paper's grammar (`0 | 1 | (i1, i2)`) into canonical
/// bits, strictly validating normal form.
///
/// Iterative, like the packed-tree parsers in [`super::tree`]: depth lives on
/// an explicit frame stack, never the call stack, so nesting depth cannot
/// overflow.
pub(crate) fn parse_id_str(s: &str) -> Result<BitsMut, Parse> {
    let mut cur = Cur::new(s);
    let mut bits = BitsMut::new();
    parse_id_tree(&mut cur, &mut bits)?;
    if cur.peek().is_some() {
        return Err(Parse::Syntax); // trailing junk
    }
    validate_id(super::built_view(&bits))?;
    Ok(bits)
}

/// What a parsed id subtree turned out to be, so its parent can pick a presence
/// tag and reject a collapsible `(0, 0)` / `(1, 1)`.
#[derive(Clone, Copy, PartialEq)]
enum IdKind {
    /// A `0`: no bits emitted (absence).
    Empty,
    /// A `1`: the terminal tag `00`.
    Terminal,
    /// An internal node.
    Node,
}

/// While building a node bottom-up, what its subtree still needs from the text:
/// the node's reserved 2-bit tag slot rides in the frame so the children's
/// presence patches in at the close paren.
///
/// One frame per unfinished ancestor, on an explicit heap `Vec` — as deep as
/// the nesting, never the call stack, exactly the packed parsers' discipline
/// ([`super::tree`]).
enum IdFrame {
    /// The next subtree is the node's left child.
    NeedLeft {
        /// The node's tag position in the output bits.
        tag: usize,
    },
    /// The left child is parsed and its `,` consumed; the next subtree is the
    /// node's right child.
    NeedRight {
        /// The node's tag position in the output bits.
        tag: usize,
        /// What the left child was (the presence patch and the collapsible-node
        /// check both need it).
        left: IdKind,
    },
}

/// Parse one id tree, appending its canonical bits.
///
/// A `0` emits nothing (absence); a node reserves a 2-bit tag, parses its
/// children, then patches the tag to their presence — rejecting a collapsible
/// `(0, 0)` / `(1, 1)` once its `)` has parsed (a structural defect outranks
/// the canonicality check, exactly the token order of the grammar). One frame
/// per unfinished ancestor.
fn parse_id_tree(cur: &mut Cur, bits: &mut BitsMut) -> Result<(), Parse> {
    let mut stack: Vec<IdFrame> = Vec::new();
    loop {
        // One atom: a leaf token, or a `(` opening the next unfinished node.
        let mut kind = match cur.bump() {
            Some(b'(') => {
                let tag = bits.len();
                bits.push(false); // placeholder, patched once the children are known
                bits.push(false);
                stack.push(IdFrame::NeedLeft { tag });
                continue; // descend into the left child
            }
            Some(b'0') => IdKind::Empty, // a `0` is absence: no bits
            Some(b'1') => {
                bits.push(false); // terminal tag `00`
                bits.push(false);
                IdKind::Terminal
            }
            _ => return Err(Parse::Syntax),
        };
        // Attach the completed subtree to its parent, possibly completing the
        // parent too.
        loop {
            match stack.pop() {
                None => return Ok(()), // the root is complete
                Some(IdFrame::NeedLeft { tag }) => {
                    if cur.bump() != Some(b',') {
                        return Err(Parse::Syntax);
                    }
                    stack.push(IdFrame::NeedRight { tag, left: kind });
                    break; // go parse the right child
                }
                Some(IdFrame::NeedRight { tag, left }) => {
                    if cur.bump() != Some(b')') {
                        return Err(Parse::Syntax);
                    }
                    match (left, kind) {
                        (IdKind::Empty, IdKind::Empty) => return Err(Parse::NotCanonical), // (0, 0)
                        (IdKind::Terminal, IdKind::Terminal) => {
                            return Err(Parse::NotCanonical); // (1, 1)
                        }
                        _ => {
                            bits.set(tag, left != IdKind::Empty); // bit 0 = left present
                            bits.set(tag + 1, kind != IdKind::Empty); // bit 1 = right present
                            kind = IdKind::Node;
                        }
                    }
                }
            }
        }
    }
}

/// Parse a stamp `(i, e)` into its id bit stream and the event component's
/// text. Splits at the top-level (depth-0) comma, parses the id side, and
/// returns the event side for the caller's version parser. Iterative.
pub(crate) fn parse_clock_str(s: &str) -> Result<(BitsMut, &str), Parse> {
    let t = s.trim();
    let bytes = t.as_bytes();
    if bytes.first() != Some(&b'(') || bytes.last() != Some(&b')') {
        return Err(Parse::Syntax);
    }
    let inner = &t[1..t.len() - 1];
    // i64 cannot overflow: depth moves by at most one per input byte, and an
    // allocation holds at most `isize::MAX` (< 2⁶³) bytes.
    let mut depth: i64 = 0;
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
    Ok((id_bits, &inner[k + 1..]))
}
