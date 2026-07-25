//! The oracle⇄impl bridge for differential structural agreement.
//!
//! [`from_oracle_party`]/[`from_oracle_version`] build an impl value by
//! emitting its canonical stored bits from an oracle tree directly (NOT via
//! the public codec), keeping algorithm correctness decoupled from codec
//! correctness. The inverse `to_oracle_*` rebuild the oracle's tree shape from
//! the impl's *internal* stored bits — the packed id bits, the version's
//! skyline stream — so a differential test can compare structures with `==`
//! without round-tripping the byte codec (which is exercised separately).
//! Both forms are normalized, so structural `==` ⇔ semantic equality.
//! Recursive over bounded test trees (the impl's own traversals are
//! iterative).

use crate::codec::{self, Bits};
use crate::oracle;
use crate::recurse::descend;
use crate::{Clock, Party, Version};

// ───────────────────────────── oracle → impl ─────────────────────────────

/// Whether an oracle id subtree is the empty `0` region. In normal form that is
/// exactly the `Leaf(false)`; the bridge only ever emits normalized oracle trees.
fn id_is_zero(t: &oracle::Party) -> bool {
    matches!(t, oracle::Party::Leaf(false))
}

fn emit_id(out: &mut Bits, t: &oracle::Party) {
    match t {
        oracle::Party::Leaf(false) => {} // `0`: absence, no bits
        oracle::Party::Leaf(true) => {
            out.push(false); // terminal tag `00`
            out.push(false);
        }
        oracle::Party::Node(l, r) => {
            // 2-bit presence tag, then the present children (a `0` child emits
            // nothing).
            out.push(!id_is_zero(l)); // bit 0 = left present
            out.push(!id_is_zero(r)); // bit 1 = right present
            emit_id(out, l);
            emit_id(out, r);
        }
    }
}

fn emit_ev(out: &mut Bits, t: &oracle::Version) {
    match t {
        oracle::Version::Leaf(n) => {
            out.push(false);
            codec::encode_int(out, n);
        }
        oracle::Version::Node(n, l, r) => {
            out.push(true);
            codec::encode_int(out, n);
            descend!(0, emit_ev(out, l));
            descend!(0, emit_ev(out, r));
        }
    }
}

/// The min-lifted packed preorder stream of an oracle tree: the
/// construction language the generators and the skyline transcoder share.
pub(crate) fn packed_bits_of(t: &oracle::Version) -> Bits {
    let mut bits = Bits::new();
    emit_ev(&mut bits, t);
    bits
}

/// Build the impl `Party` whose canonical bits encode `t`. Recursive over a bounded
/// oracle tree (test-only; the impl's own traversals are iterative).
pub(crate) fn from_oracle_party(t: &oracle::Party) -> Party {
    let mut bits = Bits::new();
    emit_id(&mut bits, t);
    Party::from_bits(bits)
}

/// Build the impl `Version` whose canonical bits encode `t`.
///
/// Recursive over a bounded oracle tree (test-only; the impl's own
/// traversals are iterative): emits the min-lifted packed preorder stream,
/// then transcodes it into the skyline coding the version stores.
pub(crate) fn from_oracle_version(t: &oracle::Version) -> Version {
    let mut bits = Bits::new();
    emit_ev(&mut bits, t);
    Version::from_bits(crate::version::skyline::encode_bits(&bits))
}

/// Build the impl `Clock` mirroring an oracle clock.
pub(crate) fn from_oracle_clock(c: &oracle::Clock) -> Clock {
    let (party, version) = c.trees();
    Clock::from_parts(from_oracle_party(party), from_oracle_version(version))
}

// ───────────────────────────── impl → oracle ─────────────────────────────
//
// Structural lowering for differential agreement: rebuild the oracle's tree
// shape from the impl's *internal* stored bits (the packed id bits; the
// version's skyline stream), then compare with `==`. This is the inverse of
// `from_oracle_*`. It walks the stored bits directly — the impl's at-rest
// storage — rather than round-tripping the public `encode`/`decode`, so the
// master harness checks algorithm correctness without sharing a failure mode
// with the byte codec (which is exercised separately). Recursive over a
// bounded tree (test-only; the impl's own traversals are iterative). Both
// forms are normalized, so structural `==` ⇔ semantic equality.

fn read_id(bits: &codec::BitsSlice, pos: usize) -> (oracle::Party, usize) {
    let left = bits[pos];
    let right = bits[pos + 1];
    if !left && !right {
        return (oracle::Party::Leaf(true), pos + 2); // terminal = `1`
    }
    // Read the present children; an absent child lowers to the `0` leaf.
    let mut next = pos + 2;
    let l = if left {
        let (l, np) = read_id(bits, next);
        next = np;
        l
    } else {
        oracle::Party::Leaf(false)
    };
    let r = if right {
        let (r, np) = read_id(bits, next);
        next = np;
        r
    } else {
        oracle::Party::Leaf(false)
    };
    (oracle::Party::Node(Box::new(l), Box::new(r)), next)
}

/// Read one skyline subtree at `pos` into a raw oracle tree.
///
/// Threads the running previous-leaf height: leaves carry their *absolute*
/// heights, internal nodes a zero base. The caller normalizes once at the
/// root.
///
/// The oracle base is the arbitrary-precision `Base` (matching the impl),
/// so lowering is lossless for any magnitude: no `u64` truncation point.
fn read_ev(
    bits: &codec::BitsSlice,
    pos: usize,
    prev: &mut Option<codec::Base>,
) -> (oracle::Version, usize) {
    let internal = bits[pos];
    if internal {
        let (l, after_l) = descend!(0, read_ev(bits, pos + 1, prev));
        let (r, after_r) = descend!(0, read_ev(bits, after_l, prev));
        return (
            oracle::Version::Node(codec::Base::ZERO, Box::new(l), Box::new(r)),
            after_r,
        );
    }
    let (code, after_n) = codec::decode_int(bits, pos + 1).expect("canonical impl bits decode");
    // First leaf: the absolute height. Later leaves: zigzag deltas
    // (`even -> +m/2`, `odd -> -(m + 1)/2`) off the previous leaf.
    let value = match prev.take() {
        None => code,
        Some(p) => {
            if code.bit(0) {
                p - &((code + 1u32) >> 1u32)
            } else {
                p + &(code >> 1u32)
            }
        }
    };
    *prev = Some(value.clone());
    (oracle::Version::Leaf(value), after_n)
}

/// Lower an impl `Party` to the oracle's structural tree by reading its packed bits.
pub(crate) fn to_oracle_party(p: &Party) -> oracle::Party {
    if p.as_bits().is_empty() {
        return oracle::Party::Leaf(false); // the anonymous `0` id
    }
    read_id(p.as_bits(), 0).0
}

/// Lower an impl `Version` to the oracle's structural tree by reading its
/// stored skyline stream: absolute leaf heights become a raw tree, which
/// one normalization pass min-lifts into the oracle's canonical spelling.
pub(crate) fn to_oracle_version(v: &Version) -> oracle::Version {
    let enc = v.as_encoded();
    let all = codec::bytes_as_bits(&enc.bytes);
    let raw = read_ev(&all[..enc.bits], 0, &mut None).0;
    raw.normalized_for_test()
}

/// Lower an impl `Clock` to the oracle's `(Party, Version)` structural form.
pub(crate) fn to_oracle_clock(c: &Clock) -> (oracle::Party, oracle::Version) {
    (to_oracle_party(c.party()), to_oracle_version(c.version()))
}
