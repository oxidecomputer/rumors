//! The rejection rows' operand defects, each maximally deferred: every
//! builder places its defect where rejection must consume as much of the
//! fed stream as possible, so an early-exit-only measurement cannot pass.

use crate::codec::{self, Base};
use crate::{Party, Version};

/// `bytes` with its last byte dropped.
///
/// A strict prefix of a preorder stream has an open subtree at every
/// position before its true end, so this is the maximally-deferred
/// [`Truncated`](crate::error::Decode) defect — discoverable only by
/// parsing to the cut.
pub(super) fn truncated_bytes(bytes: &[u8]) -> Vec<u8> {
    assert!(
        bytes.len() >= 2,
        "a truncation row needs a stream of at least two bytes"
    );
    bytes[..bytes.len() - 1].to_vec()
}

/// `bytes` with a `0xFF` byte appended after the complete valid stream:
/// the maximally-deferred [`TrailingBits`](crate::error::Decode) defect —
/// the whole tree parses before the nonzero tail is seen.
pub(super) fn trailing_bytes(bytes: &[u8]) -> Vec<u8> {
    let mut out = bytes.to_vec();
    out.push(0xFF);
    out
}

/// The bit position of a version stream's preorder-last leaf flag.
///
/// Iterative over the packed form, outside any measurement; the last
/// node of a preorder event stream is always a leaf (an internal node's
/// children would follow it).
fn last_leaf_flag_pos(v: &Version) -> usize {
    let all = codec::bytes_as_bits(v.as_bytes());
    let bits = &all[..v.encoded_bits()];
    let mut pos = 0usize;
    let mut pending = 1usize;
    let mut last = 0usize;
    while pending > 0 {
        pending -= 1;
        let flag = pos;
        let internal = !bits[pos]; // skyline flag: 0 internal, 1 leaf
        pos += 1;
        if internal {
            pending += 2;
            continue;
        }
        let (_, next) = codec::decode_int(bits, pos).expect("a stored stream is canonical");
        pos = next;
        last = flag;
    }
    last
}

/// `v`'s stream with its preorder-last leaf split into an equal-sibling
/// pair.
///
/// The left child keeps the old leaf's delta code (same predecessor,
/// same value); the right child's delta is zero — the minimality
/// violation the validator can only judge at that pair's close, the
/// stream's last position. The maximally-deferred
/// [`NotCanonical`](crate::error::Decode) defect.
pub(super) fn version_noncanonical_bytes(v: &Version) -> Vec<u8> {
    let all = codec::bytes_as_bits(v.as_bytes());
    let bits = &all[..v.encoded_bits()];
    let leaf = last_leaf_flag_pos(v);
    let mut out = codec::Bits::with_capacity(bits.len() + 4);
    out.extend_from_bitslice(&bits[..leaf]);
    out.push(false); // the old leaf's position becomes an internal node
    out.extend_from_bitslice(&bits[leaf..]); // left child: the old leaf verbatim
    out.push(true); // right child: a leaf equal to its sibling
    codec::encode_int(&mut out, &Base::from(0u32)); // zero delta
    codec::zero_dead_bits(&mut out);
    out.into_vec()
}

/// `p`'s stream with its preorder-last terminal split into a collapsible
/// `(1, 1)`.
///
/// Two full children, judged non-normal at the node's close — the
/// stream's last position: the maximally-deferred
/// [`NotCanonical`](crate::error::Decode) defect on the id side.
pub(super) fn party_noncanonical_bytes(p: &Party) -> Vec<u8> {
    let bits = p.as_bits();
    let end = bits.len();
    assert!(
        !bits[end - 2] && !bits[end - 1],
        "a preorder id stream ends in a terminal tag"
    );
    let mut out = codec::Bits::with_capacity(end + 4);
    out.extend_from_bitslice(&bits[..end - 2]);
    out.push(true); // the last terminal becomes a node with both children
    out.push(true);
    for _ in 0..2 {
        out.push(false); // each child a terminal: the collapsible (1, 1)
        out.push(false);
    }
    codec::zero_dead_bits(&mut out);
    out.into_vec()
}

/// `text` with junk appended after the complete valid notation: the
/// parser consumes the whole text before the trailing defect is seen
/// ([`Parse::Syntax`](crate::error::Parse)).
pub(super) fn trailing_text(text: &str) -> String {
    let mut out = text.to_owned();
    out.push('x');
    out
}

/// A clock's text with junk inserted before the closing paren, inside the
/// version component.
///
/// The clock parser's outer-paren check rejects *appended* junk in O(1),
/// so the deferred defect rides the version side, which parses in full
/// first.
pub(super) fn clock_trailing_text(text: &str) -> String {
    let mut out = text.to_owned();
    assert_eq!(out.pop(), Some(')'), "a clock renders as (id, event)");
    out.push_str("x)");
    out
}

/// `text` with its last spelled value `t` re-spelled `(0, t, t)`: equal
/// sibling leaves, well-formed and judged non-canonical at that node's
/// close — the text's end
/// ([`Parse::NotCanonical`](crate::error::Parse)).
pub(super) fn version_noncanonical_text(text: &str) -> String {
    let end = text
        .rfind(|c: char| c.is_ascii_digit())
        .expect("a version's text spells at least one value")
        + 1;
    let start = text[..end]
        .rfind(|c: char| !c.is_ascii_digit())
        .map_or(0, |i| i + 1);
    let t = &text[start..end];
    format!("{}(0, {t}, {t}){}", &text[..start], &text[end..])
}

/// `text` with its last `1` token re-spelled `(1, 1)`: the collapsible
/// pair, judged non-normal at the node's close, at the text's end
/// ([`Parse::NotCanonical`](crate::error::Parse)).
pub(super) fn party_noncanonical_text(text: &str) -> String {
    let at = text
        .rfind('1')
        .expect("a party's text spells at least one owned leaf");
    format!("{}(1, 1){}", &text[..at], &text[at + 1..])
}
