//! The rejection rows' operand defects: the placed flaws the fallible surface
//! is priced against.
//!
//! Cost claims are total: rejecting an input is an outcome with a cost, bounded
//! like any other, whether or not the caller honored the usage invariants. The
//! rejection rows price the fallible surface — overlap (`*_join_overlap`,
//! `clock_sync_overlap`, `party_join_all_overlap`), the empty difference
//! (`party_without_none`), strict decode
//! (`*_decode_truncated`/`_trailing`/`_noncanon`), and text parse
//! (`*_parse_trailing`/`_noncanon`, driving `FromStr`) — with the defect
//! **maximally deferred** in every shape: an early-exit-only measurement would
//! be the cheapest artifact that passes, so each row places its defect where
//! rejection must consume as much input as possible (the last byte truncated,
//! trailing bits after the complete stream, a non-canonical pair closing at the
//! stream's last position, the one overlapping region at both operands'
//! preorder ends, junk after the whole valid text). Rejections produce no
//! output, so every rejection row is denominated against the fed stream alone —
//! packed bytes, or text bytes on the parse rows at the general (not κ) limb
//! ceiling: the radix-work term prices conversion of the accepting direction,
//! and a rejection forces no conversion. Overlap operands come from the
//! overlap-mount adapter, the disjoint-mount adapter's counterpart; its outputs
//! are semantically void by design (see
//! [`overlap_mounted_pair`](super::family::overlap_mounted_pair)'s docs). The
//! `coverage` module's table is the durable record of which fallible operations
//! are rowed and which carry a bounded-or-delegated reason.

use crate::codec::{self, Base};
use crate::{Party, Version};

/// `bytes` cut short through its live bits.
///
/// A strict prefix of a preorder stream has an open subtree at every
/// position before its true end, so this is the maximally-deferred
/// [`Truncated`](crate::error::Decode) defect — discoverable only by
/// parsing to the cut. The final byte is pure padding exactly when it is
/// the whole-byte marker `1000_0000` (the live bits end flush against
/// the byte boundary); dropping only that byte would leave a *complete*
/// tree missing its padding — a `TrailingBits` defect, not a cut — so
/// the cut then takes the last live byte with it.
pub(super) fn truncated_bytes(bytes: &[u8]) -> Vec<u8> {
    let cut = if bytes.last() == Some(&0b1000_0000) {
        2
    } else {
        1
    };
    assert!(
        bytes.len() > cut,
        "a truncation row needs a stream with live bits beyond the cut"
    );
    bytes[..bytes.len() - cut].to_vec()
}

/// `bytes` with a `0xFF` byte appended after the complete valid stream: the
/// maximally-deferred [`TrailingBits`](crate::error::Decode) defect — the whole
/// tree parses before the nonzero tail is seen.
pub(super) fn trailing_bytes(bytes: &[u8]) -> Vec<u8> {
    let mut out = bytes.to_vec();
    out.push(0xFF);
    out
}

/// The bit position of a version stream's preorder-last leaf flag.
///
/// Iterative over the packed form, outside any measurement; the last node of a
/// preorder event stream is always a leaf (an internal node's children would
/// follow it).
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

/// `v`'s stream with its preorder-last leaf split into an equal-sibling pair.
///
/// The left child keeps the old leaf's delta code (same predecessor, same
/// value); the right child's delta is zero — the minimality violation the
/// validator can only judge at that pair's close, the stream's last position.
/// The maximally-deferred [`NotCanonical`](crate::error::Decode) defect.
pub(super) fn version_noncanonical_bytes(v: &Version) -> Vec<u8> {
    let all = codec::bytes_as_bits(v.as_bytes());
    let bits = &all[..v.encoded_bits()];
    let leaf = last_leaf_flag_pos(v);
    let mut out = codec::BitsMut::with_capacity(bits.len() + 4);
    out.extend_from_bitslice(&bits[..leaf]);
    out.push(false); // the old leaf's position becomes an internal node
    out.extend_from_bitslice(&bits[leaf..]); // left child: the old leaf verbatim
    out.push(true); // right child: a leaf equal to its sibling
    codec::encode_int(&mut out, &Base::from(0u32)); // zero delta
    codec::seal_padding(&mut out);
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
    let mut out = codec::BitsMut::with_capacity(end + 4);
    out.extend_from_bitslice(&bits[..end - 2]);
    out.push(true); // the last terminal becomes a node with both children
    out.push(true);
    for _ in 0..2 {
        out.push(false); // each child a terminal: the collapsible (1, 1)
        out.push(false);
    }
    codec::seal_padding(&mut out);
    out.into_vec()
}

/// `text` with junk appended after the complete valid notation: the parser
/// consumes the whole text before the trailing defect is seen
/// ([`Parse::Syntax`](crate::error::Parse)).
pub(super) fn trailing_text(text: &str) -> String {
    let mut out = text.to_owned();
    out.push('x');
    out
}

/// A clock's text with junk inserted before the closing paren, inside the
/// version component.
///
/// The clock parser's outer-paren check rejects *appended* junk in O(1), so the
/// deferred defect rides the version side, which parses in full first.
pub(super) fn clock_trailing_text(text: &str) -> String {
    let mut out = text.to_owned();
    assert_eq!(out.pop(), Some(')'), "a clock renders as (id, event)");
    out.push_str("x)");
    out
}

/// `text` with its last spelled value `t` re-spelled `(0, t, t)`: equal sibling
/// leaves, well-formed and judged non-canonical at that node's close — the
/// text's end ([`Parse::NotCanonical`](crate::error::Parse)).
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

/// `text` with its last `1` token re-spelled `(1, 1)`: the collapsible pair,
/// judged non-normal at the node's close, at the text's end
/// ([`Parse::NotCanonical`](crate::error::Parse)).
pub(super) fn party_noncanonical_text(text: &str) -> String {
    let at = text
        .rfind('1')
        .expect("a party's text spells at least one owned leaf");
    format!("{}(1, 1){}", &text[..at], &text[at + 1..])
}
