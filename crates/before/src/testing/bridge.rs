//! The oracle⇄impl bridge for differential structural agreement.
//!
//! [`from_oracle_party`]/[`from_oracle_version`] build an impl value by
//! emitting the canonical packed bits of an oracle tree directly (NOT via the
//! public codec), keeping algorithm correctness decoupled from codec
//! correctness. The inverse `to_oracle_*` rebuild the oracle's tree shape from
//! the impl's *internal* packed representation, so a differential test can
//! compare structures with `==` without round-tripping the byte codec (which is
//! exercised separately). Both forms are normalized, so structural `==` ⇔
//! semantic equality. Recursive over bounded test trees (the impl's own
//! traversals are iterative).

use crate::codec::{self, Bits};
use crate::oracle;
use crate::{Clock, Party, Version};

// ───────────────────────────── oracle → impl ─────────────────────────────

fn emit_id(out: &mut Bits, t: &oracle::Party) {
    match t {
        oracle::Party::Leaf(b) => {
            out.push(false);
            out.push(*b);
        }
        oracle::Party::Node(l, r) => {
            out.push(true);
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
            emit_ev(out, l);
            emit_ev(out, r);
        }
    }
}

/// Build the impl `Party` whose canonical bits encode `t`. Recursive over a bounded
/// oracle tree (test-only; the impl's own traversals are iterative).
pub(crate) fn from_oracle_party(t: &oracle::Party) -> Party {
    let mut bits = Bits::new();
    emit_id(&mut bits, t);
    Party::from_bits(bits)
}

/// Build the impl `Version` whose canonical bits encode `t`. Recursive over a bounded
/// oracle tree (test-only; the impl's own traversals are iterative).
pub(crate) fn from_oracle_version(t: &oracle::Version) -> Version {
    let mut bits = Bits::new();
    emit_ev(&mut bits, t);
    Version::from_bits(bits)
}

/// Build the impl `Clock` mirroring an oracle clock.
pub(crate) fn from_oracle_clock(c: &oracle::Clock) -> Clock {
    let (party, version) = c.trees();
    Clock::from_parts(from_oracle_party(party), from_oracle_version(version))
}

// ───────────────────────────── impl → oracle ─────────────────────────────
//
// Structural lowering for differential agreement: rebuild the oracle's tree
// shape from the impl's *internal* packed representation, then compare with
// `==`. This is the inverse of `from_oracle_*`. It walks the packed bits
// directly — the impl's at-rest storage — rather than round-tripping the public
// `encode`/`decode`, so the master harness checks algorithm correctness without
// sharing a failure mode with the byte codec (which is exercised separately).
// Recursive over a bounded tree (test-only; the impl's own traversals are
// iterative). Both forms are normalized, so structural `==` ⇔ semantic
// equality.

fn read_id(bits: &codec::BitsSlice, pos: usize) -> (oracle::Party, usize) {
    if bits[pos] {
        let (l, after_l) = read_id(bits, pos + 1);
        let (r, after_r) = read_id(bits, after_l);
        (oracle::Party::Node(Box::new(l), Box::new(r)), after_r)
    } else {
        (oracle::Party::Leaf(bits[pos + 1]), pos + 2)
    }
}

fn read_ev(bits: &codec::BitsSlice, pos: usize) -> (oracle::Version, usize) {
    let internal = bits[pos];
    // The oracle base is the arbitrary-precision `Base` (matching the impl), so lowering is
    // lossless for any magnitude: no `u64` truncation point.
    let (n, after_n) = codec::decode_int(bits, pos + 1).expect("canonical impl bits decode");
    if internal {
        let (l, after_l) = read_ev(bits, after_n);
        let (r, after_r) = read_ev(bits, after_l);
        (oracle::Version::Node(n, Box::new(l), Box::new(r)), after_r)
    } else {
        (oracle::Version::Leaf(n), after_n)
    }
}

/// Lower an impl `Party` to the oracle's structural tree by reading its packed bits.
pub(crate) fn to_oracle_party(p: &Party) -> oracle::Party {
    read_id(p.as_bits(), 0).0
}

/// Lower an impl `Version` to the oracle's structural tree by reading its packed bits.
pub(crate) fn to_oracle_version(v: &Version) -> oracle::Version {
    read_ev(v.as_bits(), 0).0
}

/// Lower an impl `Clock` to the oracle's `(Party, Version)` structural form.
pub(crate) fn to_oracle_clock(c: &Clock) -> (oracle::Party, oracle::Version) {
    (to_oracle_party(c.party()), to_oracle_version(c.version()))
}
