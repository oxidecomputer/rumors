//! Invalid inputs used to measure rejection costs.
//!
//! Each constructor puts the error near the end of the input so the measured
//! operation cannot pass merely by rejecting early. Rejection rows are measured
//! against input bytes because they produce no encoded output. The coverage
//! table records which public operations use these measurements.

use num_bigint::BigUint;

use crate::codec::{self, gamma};
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
/// Iterative over the encoded form, outside any measurement; the last node of a
/// preorder event stream is always a leaf (an internal node's children would
/// follow it).
fn last_leaf_flag_pos(v: &Version) -> u64 {
    let bits = v.as_bits();
    let mut pos = 0u64;
    let mut pending = 1usize;
    let mut last = 0u64;
    while pending > 0 {
        pending -= 1;
        let flag = pos;
        let internal = !bits.bit(pos); // skyline flag: 0 internal, 1 leaf
        pos += 1;
        if internal {
            pending += 2;
            continue;
        }
        let (_, next) = codec::gamma::decode(bits, pos).expect("a stored stream is canonical");
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
    let bits = v.as_bits();
    let leaf = last_leaf_flag_pos(v);
    let mut out = codec::BitsBuf::with_capacity(bits.len() + 4);
    codec::extend_from_view(&mut out, bits, 0, leaf);
    out.push(false); // the old leaf's position becomes an internal node
    codec::extend_from_view(&mut out, bits, leaf, bits.len()); // left child: the old leaf verbatim
    out.push(true); // right child: a leaf equal to its sibling
    gamma::encode(&BigUint::ZERO, &mut out); // zero delta
    codec::seal_padding(&mut out);
    out.into_bytes()
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
        !bits.bit(end - 2) && !bits.bit(end - 1),
        "a preorder id stream ends in a terminal tag"
    );
    let mut out = codec::BitsBuf::with_capacity(end + 4);
    codec::extend_from_view(&mut out, bits, 0, end - 2);
    out.push(true); // the last terminal becomes a node with both children
    out.push(true);
    for _ in 0..2 {
        out.push(false); // each child a terminal: the collapsible (1, 1)
        out.push(false);
    }
    codec::seal_padding(&mut out);
    out.into_bytes()
}
