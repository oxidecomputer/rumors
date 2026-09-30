//! Exact scan pins for deep party walks.
//!
//! The id walks' entire cost is scan bits: they allocate nothing, recurse
//! nothing, and do no arithmetic, so the `ID_COVERS`/`ID_DISJOINT` envelope
//! columns are all structurally near-zero and this counter is the one
//! deterministic meter that sees the work. These pins hold each walk's scan
//! reading to an exact measured value at both depths (the walk reads every
//! stored tag of both operands exactly once, so the reading is
//! deterministic and two-sided: an undercounting tap and a re-scanning
//! walk both move it) over a full-examination liveness floor (one bit per
//! encoded operand byte — the diverted pair forces both walks to full
//! lockstep depth), and to per-byte flatness (×1.25) across a depth
//! doubling, so a walk that leaves the metered primitives moves a
//! committed number instead of passing every near-zero column unchanged.

use super::{id_pair_input_bytes, party_of, ID_DEPTH};
use before::testing::meter;
use before::testing::meter::registry::Shape;

/// One walk run: input bytes and the bits scanned by the
/// walk body alone.
struct Run {
    bytes: u64,
    bits: u64,
}

/// Exact scan readings at the [`ID_DEPTH`] pair, measured
/// with deterministic counters.
///
/// Every stored tag of both operands is read exactly once through
/// the metered primitives — [`SCAN_EXACT_BITS_SMALL`] on 62,502
/// input bytes at the half depth, [`SCAN_EXACT_BITS_LARGE`] on
/// 125,002 at the full depth, identical for the covers and disjoint
/// walks (the same full lockstep walk). Pinned with equality, not a
/// ceiling: a uniform tap undercount halves the reading yet clears
/// every slack floor in the tree, so only the exact number is
/// tamper-evident in both directions.
const SCAN_EXACT_BITS_SMALL: u64 = 500_004;

/// The full-depth reading paired with [`SCAN_EXACT_BITS_SMALL`].
const SCAN_EXACT_BITS_LARGE: u64 = 1_000_004;

/// Run one id-pair walk at `depth` and read the scan counter over
/// the body alone, enforcing the full-examination liveness floor.
fn walk_run(name: &str, depth: usize, body: impl FnOnce(&before::Party, &before::Party)) -> Run {
    let pa = Shape::IdSpine.build_flagged(depth, false);
    let pb = Shape::IdSpine.build_flagged(depth, true);
    let bytes = id_pair_input_bytes(&pa, &pb) as u64;
    let a = party_of(&pa);
    let b = party_of(&pb);
    meter::reset_scan_bits();
    body(&a, &b);
    let bits = meter::scan_bits();
    eprintln!("MEASURED id_walk_scan_{name}: depth={depth} bytes={bytes} scan_bits={bits}");
    assert!(
        bits >= bytes,
        "id_walk_scan_{name}: {bits} scanned bits under the one-bit-per-byte floor over \
         {bytes} encoded bytes: the walk left the metered primitives"
    );
    Run { bytes, bits }
}

/// Per-byte scan cost stays flat (×1.25) across the depth doubling.
fn assert_flat(name: &str, small: &Run, large: &Run) {
    assert!(
        u128::from(large.bits) * u128::from(small.bytes) * 4
            <= u128::from(small.bits) * u128::from(large.bytes) * 5,
        "id_walk_scan_{name}: per-byte scan cost grew more than x1.25 across the depth \
         doubling: {}/{} -> {}/{}",
        small.bits,
        small.bytes,
        large.bits,
        large.bytes,
    );
}

/// The covers walk's scan bits are exact-pinned at both depths,
/// floored, and flat per byte across the depth doubling of the
/// diverted spine pair (which admits no early exit).
///
/// The walk's cost is invisible to every other deterministic meter,
/// so this pin is what a re-scanning `covers` (quadratic restarts),
/// an unmetered raw-indexing walk, or a tap under- or over-count
/// moves — the equality is the two-sided form a ceiling-plus-slack
/// floor cannot give.
#[test]
fn id_covers_scan_cost_is_pinned_and_flat() {
    let small = walk_run("covers_small", ID_DEPTH / 2, |a, b| {
        assert!(!a.covers(b), "the divert arms are disjoint");
    });
    let large = walk_run("covers", ID_DEPTH, |a, b| {
        assert!(!a.covers(b), "the divert arms are disjoint");
    });
    assert_flat("covers", &small, &large);
    assert_eq!(
        (small.bits, large.bits),
        (SCAN_EXACT_BITS_SMALL, SCAN_EXACT_BITS_LARGE),
        "id_covers: the scanned bits moved off the exact pin: a moved \
         reading is a walk or tap change to re-pin deliberately",
    );
}

/// The disjoint walk's scan bits are exact-pinned at both depths,
/// floored, and flat per byte across the depth doubling of the
/// diverted spine pair (disjoint operands, so the walk runs to
/// completion).
///
/// Same rationale as the covers pin: scan is the one live column on
/// this walk, and only the exact equality reads a tap undercount.
#[test]
fn id_disjoint_scan_cost_is_pinned_and_flat() {
    let small = walk_run("disjoint_small", ID_DEPTH / 2, |a, b| {
        assert!(a.is_disjoint(b), "the divert arms own disjoint regions");
    });
    let large = walk_run("disjoint", ID_DEPTH, |a, b| {
        assert!(a.is_disjoint(b), "the divert arms own disjoint regions");
    });
    assert_flat("disjoint", &small, &large);
    assert_eq!(
        (small.bits, large.bits),
        (SCAN_EXACT_BITS_SMALL, SCAN_EXACT_BITS_LARGE),
        "id_disjoint: the scanned bits moved off the exact pin: a moved \
         reading is a walk or tap change to re-pin deliberately",
    );
}
