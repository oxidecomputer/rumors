//! The oracle⇄impl bridge for differential structural agreement.
//!
//! [`from_oracle_party`]/[`from_oracle_version`] build an impl value by
//! emitting its canonical stored bits from an oracle tree directly (NOT via the
//! public codec), keeping algorithm correctness decoupled from codec
//! correctness. The inverse `to_oracle_*` rebuild the oracle's tree shape from
//! the impl's *internal* stored bits — the party bits, the version's skyline
//! stream — so a differential test can compare structures with `==` without
//! round-tripping the byte codec (which is exercised separately). Both forms
//! are normalized, so structural `==` ⇔ semantic equality. Recursive over
//! bounded test trees (the impl's own traversals are iterative).

use std::sync::Arc;

use num_bigint::BigUint;

use crate::bits::BitsWriter;
use crate::party::io::{PartyNode, PartyReader};
use crate::recurse::descend;
use crate::testing::oracles::tree;
use crate::version::io::tree::{VersionNode, VersionTreeReader};
use crate::{Clock, Party, Version};

// ───────────────────────────── oracle → impl ─────────────────────────────

/// Whether an oracle id subtree is the empty `0` region. In normal form that is
/// exactly the `Leaf(false)`; the bridge only ever emits normalized oracle trees.
fn id_is_zero(t: &tree::Party) -> bool {
    matches!(t, tree::Party::Leaf(false))
}

fn emit_id(out: &mut BitsWriter, t: &tree::Party) {
    match t {
        tree::Party::Leaf(false) => {} // `0`: absence, no bits
        tree::Party::Leaf(true) => {
            out.push(false); // terminal tag `00`
            out.push(false);
        }
        tree::Party::Node(l, r) => {
            // 2-bit presence tag, then the present children (a `0` child emits
            // nothing).
            out.push(!id_is_zero(l)); // bit 0 = left present
            out.push(!id_is_zero(r)); // bit 1 = right present
            emit_id(out, l);
            emit_id(out, r);
        }
    }
}

fn emit_ev(out: &mut BitsWriter, t: &tree::Version) {
    match t {
        tree::Version::Leaf(n) => {
            out.push(false);
            out.write_gamma(n);
        }
        tree::Version::Node(n, l, r) => {
            out.push(true);
            out.write_gamma(n);
            descend!(0, emit_ev(out, l));
            descend!(0, emit_ev(out, r));
        }
    }
}

/// The min-lifted preorder stream of an oracle tree: the
/// construction language the generators and the skyline transcoder share.
pub(crate) fn encoded_bits_of(t: &tree::Version) -> BitsWriter {
    let mut bits = BitsWriter::new();
    emit_ev(&mut bits, t);
    bits
}

/// Build the impl `Party` whose canonical bits encode `t`. Recursive over a bounded
/// oracle tree (test-only; the impl's own traversals are iterative).
pub(crate) fn from_oracle_party(t: &tree::Party) -> Party {
    assert!(
        !t.is_empty(),
        "the production Party type represents nonempty ownership"
    );
    let mut bits = BitsWriter::new();
    emit_id(&mut bits, t);
    crate::party::io::finish(bits)
}

/// Build the impl `Version` whose canonical bits encode `t`.
///
/// Recursive over a bounded oracle tree (test-only; the impl's own
/// traversals are iterative): emits the min-lifted preorder stream,
/// then transcodes it into the skyline coding the version stores.
pub(crate) fn from_oracle_version(t: &tree::Version) -> Version {
    let mut bits = BitsWriter::new();
    emit_ev(&mut bits, t);
    crate::version::io::finish(crate::version::io::encode::encode_bits(bits.reader()))
}

/// Build the impl `Clock` mirroring an oracle clock.
pub(crate) fn from_oracle_clock(c: &tree::Clock) -> Clock {
    let (party, version) = c.trees();
    Clock::from_parts(from_oracle_party(party), from_oracle_version(version))
}

// ───────────────────────────── impl → oracle ─────────────────────────────
//
// Structural lowering for differential agreement: rebuild the oracle's tree
// shape from the impl's *internal* stored bits (the party bits; the version's
// skyline stream), then compare with `==`. This is the inverse of
// `from_oracle_*`. It walks the stored bits directly — the impl's at-rest
// storage — rather than round-tripping the public `encode`/`decode`, so the
// master harness checks algorithm correctness without sharing a failure mode
// with the byte codec (which is exercised separately). Recursive over a bounded
// tree (test-only; the impl's own traversals are iterative). Both
// forms are normalized, so structural `==` ⇔ semantic equality.

fn read_id(reader: &mut PartyReader<'_>) -> tree::Party {
    let PartyNode::Branch(branch) = reader.read() else {
        return tree::Party::Leaf(true);
    };
    let left = if branch.has_left_child() {
        descend!(0, read_id(reader))
    } else {
        tree::Party::Leaf(false)
    };
    let right = if branch.has_right_child() {
        descend!(0, read_id(reader))
    } else {
        tree::Party::Leaf(false)
    };
    tree::Party::Node(Arc::new(left), Arc::new(right))
}

/// Read one skyline subtree at `pos` into a raw oracle tree.
///
/// Threads the running previous-leaf height: leaves carry their *absolute*
/// heights, internal nodes a zero base. The caller normalizes once at the
/// root.
///
/// The oracle base is the arbitrary-precision `BigUint` (matching the impl),
/// so lowering is lossless for any magnitude: no `u64` truncation point.
fn read_ev(reader: &mut VersionTreeReader<'_>, prev: &mut Option<BigUint>) -> tree::Version {
    if reader.node() == VersionNode::Branch {
        let left = descend!(0, read_ev(reader, prev));
        let right = descend!(0, read_ev(reader, prev));
        return tree::Version::Node(BigUint::ZERO, Arc::new(left), Arc::new(right));
    }
    let code = reader.payload();
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
    tree::Version::Leaf(value)
}

/// Lower an impl `Party` to the oracle's structural tree by reading its encoded bits.
pub(crate) fn to_oracle_party(p: &Party) -> tree::Party {
    read_id(&mut p.reader())
}

/// Lower an impl `Version` to the oracle's structural tree by reading its
/// stored skyline stream: absolute leaf heights become a raw tree, which
/// one normalization pass min-lifts into the oracle's canonical spelling.
pub(crate) fn to_oracle_version(v: &Version) -> tree::Version {
    let raw = read_ev(&mut VersionTreeReader::new(v), &mut None);
    raw.normalized_for_test()
}

/// Lower an impl `Clock` to the oracle's `(Party, Version)` structural form.
pub(crate) fn to_oracle_clock(c: &Clock) -> (tree::Party, tree::Version) {
    (to_oracle_party(c.party()), to_oracle_version(c.version()))
}
