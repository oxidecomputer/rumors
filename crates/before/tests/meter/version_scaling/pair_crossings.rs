//! Flatness checks for pairwise distance and masked comparison crossings.

use super::*;

/// One public-distance run over the two-operand jump comb
/// `JP(k, m, d)`: both counters over the distance body alone, with
/// the operands' input bytes and stored delta codes as the per-unit
/// denominators.
///
/// Enforces the touch liveness floor (every stored delta lands in
/// the metered accumulator) and anchors the result by rank
/// modularity before returning.
fn distance_jump_pair_run(k: usize, m: usize, d: usize) -> Run {
    let (pa, pb) = Shape::JumpPair.build_pair3(k, m, d);
    let a = pa.version();
    let b = pb.version();
    let bytes = (a.encode().len() + b.encode().len()) as u64;
    // Per operand: one leaf per shared-spine level (33d), three per
    // comb level, and the comb terminal; deltas are leaves − 1.
    let deltas = 2 * (33 * d as u64 + 3 * m as u64);
    touch_meter::reset();
    let r = a.distance(&b);
    let run = Run {
        deltas,
        bytes,
        touches: touch_meter::touches(),
    };
    assert!(
        run.touches >= run.deltas,
        "distance_jump_pair m={m}: {} digit touches under the {}-delta floor: \
         the query height state is not running on the metered accumulator",
        run.touches,
        run.deltas,
    );
    assert_eq!(
        r,
        &a.lag(&b) + &b.lag(&a),
        "the distance must equal the two lags' sum (rank modularity)"
    );
    run
}

/// One public-rank run over a single [`meter::jump_pair`] operand:
/// the flat single-operand control for the jump-pair band below.
fn rank_jump_pair_operand_run(k: usize, m: usize, d: usize, band: bool) -> Run {
    let (pa, pb) = Shape::JumpPair.build_pair3(k, m, d);
    let v = if band { pb.version() } else { pa.version() };
    let bytes = v.encode().len() as u64;
    let deltas = 33 * d as u64 + 3 * m as u64;
    touch_meter::reset();
    let r = v.rank();
    let run = Run {
        deltas,
        bytes,
        touches: touch_meter::touches(),
    };
    assert!(
        run.touches >= run.deltas,
        "rank height state left the accumulator"
    );
    drop(r);
    run
}

/// Comb levels of the band's small run (the large run doubles both
/// parameters; the position digits stay an eighth of the teeth, the
/// board family's proportion).
const DISTANCE_JUMP_PAIR_SMALL_TEETH: usize = 512;

/// Freeze-position digits of the band's small run.
const DISTANCE_JUMP_PAIR_SMALL_DIGITS: usize = 64;

/// Absolute two-scale touch ceilings for the jump-pair distance,
/// measured ×1.25 (the record and every re-pin's movement live in
/// the pin commits).
///
/// The anchored-segment co-sweep reads flat per encoded byte across
/// the doubling; the composed form this family was built to expose
/// reads superlinear, several times over these ceilings.
const DISTANCE_JUMP_PAIR_TOUCH_CEILINGS: (u64, u64) = (158_194, 316_347);
/// The jump-pair distance is linear in the pair size: per-byte
/// touch work stays flat (×1.25) across a (teeth, digits)
/// doubling, under absolute two-scale ceilings.
///
/// Both single-operand ranks are pinned flat beside the pair, so
/// the family's separation stays whole: the shape exists only in
/// the two-operand composition.
///
/// The family interleaves one operand's wide teeth with the other's
/// near-flat band over a shared spine whose right turns plant
/// isolated position bits, so the overlay's height difference
/// crests wide once per comb level while every absolute position
/// stays dense under balanced compaction. A freeze accounting that
/// multiplies evicted drift by absolute positions pays
/// teeth × digits × magnitude here and reads superlinear — one
/// operand's cheap codes firing corrections against drift only the
/// other operand funded, the wedge this family exists to expose.
/// The anchored-segment co-sweep settles each crest against its
/// own segment's mass, whose compacted span the spine's shared
/// prefix never enters, so the flatness bound holds at both scales
/// and each operand alone stays the flat control.
#[test]
fn skyline_distance_jump_pair_is_flat_per_unit() {
    let k = super::JUMP_PAIR_MAGNITUDE_BITS;
    let (m, d) = (
        DISTANCE_JUMP_PAIR_SMALL_TEETH,
        DISTANCE_JUMP_PAIR_SMALL_DIGITS,
    );
    let small = distance_jump_pair_run(k, m, d);
    let large = distance_jump_pair_run(k, 2 * m, 2 * d);
    for (run, scale) in [(&small, "small"), (&large, "large")] {
        eprintln!(
            "MEASURED distance_jump_pair_{scale}: bytes={} touches={}",
            run.bytes, run.touches,
        );
    }
    for (run, touch_ceiling, scale) in [
        (&small, DISTANCE_JUMP_PAIR_TOUCH_CEILINGS.0, "small"),
        (&large, DISTANCE_JUMP_PAIR_TOUCH_CEILINGS.1, "large"),
    ] {
        assert!(
            run.touches <= touch_ceiling,
            "distance_jump_pair_{scale}: {} touches exceed the pinned ceiling \
             {touch_ceiling}: an absolute-position product is back in the \
             co-sweep's freeze accounting",
            run.touches,
        );
    }
    // The flatness bound: per-byte cost must not grow across the
    // doubling — the reading that separates the anchored-segment
    // accounting from any absolute-position one.
    assert_flat(
        "distance_jump_pair_touches",
        "byte",
        (small.touches, small.bytes),
        (large.touches, large.bytes),
    );
    // The separation witnesses: either operand alone stays flat —
    // the teeth operand's wide folds cancel adjacently (bounded
    // oscillation), the band operand pays its width once.
    let teeth_small = rank_jump_pair_operand_run(k, m, d, false);
    let teeth_large = rank_jump_pair_operand_run(k, 2 * m, 2 * d, false);
    let band_small = rank_jump_pair_operand_run(k, m, d, true);
    let band_large = rank_jump_pair_operand_run(k, 2 * m, 2 * d, true);
    assert_flat(
        "rank_jump_pair_teeth_touches",
        "byte",
        (teeth_small.touches, teeth_small.bytes),
        (teeth_large.touches, teeth_large.bytes),
    );
    assert_flat(
        "rank_jump_pair_band_touches",
        "byte",
        (band_small.touches, band_small.bytes),
        (band_large.touches, band_large.bytes),
    );
}

/// One fused three-stream comparison run over the mask-drift triple
/// at `scale` teeth: per-delta touches and input bytes, with
/// the one-touch-per-delta liveness floor enforced before returning.
fn masked_cmp_run(scale: usize) -> Run {
    let (comb, mask, plateau) = Shape::MaskDriftTriple.build_triple(512, scale);
    let v = comb.version();
    let p = before::Party::decode(&mask.bytes[..]).expect("the mask is strict normal form");
    let w = plateau.version();
    let bytes = (v.encode().len() + mask.bytes.len() + w.encode().len()) as u64;
    touch_meter::reset();
    let verdict = (&v / &p).partial_cmp(&w);
    assert_eq!(
        verdict,
        Some(std::cmp::Ordering::Less),
        "the projected comb sits strictly under the plateau (no early exit)"
    );
    let run = Run {
        // The comb's 2n + 1 leaves put 2n delta codes behind the
        // first; the plateau adds none.
        deltas: 2 * scale as u64,
        bytes,
        touches: touch_meter::touches(),
    };
    assert!(
        run.touches >= run.deltas,
        "masked_cmp scale {scale}: {} digit touches under the {}-delta floor: \
         the walk's integrators are not running on the metered accumulator",
        run.touches,
        run.deltas,
    );
    run
}

/// The fused three-stream comparison's per-delta touches and
/// per-byte touch work stays flat across a tooth-count doubling of
/// the mask-drift triple.
///
/// Every mask boundary's sign read — the difference mid-cancel
/// inside owned teeth, the zero-check on unowned intervals — stays
/// amortized O(1) however many boundaries the mask plants.
///
/// Each run carries the one-touch-per-delta liveness floor (in
/// [`masked_cmp_run`]), so flatness is asserted over a meter proven
/// live. This is the correlated family's wedge test: an integrator
/// that materialized a read per boundary would grow the per-delta
/// cost with the magnitude and fail the band.
#[test]
fn masked_cmp_drift_cost_is_flat_per_unit() {
    let small = masked_cmp_run(1_024);
    let large = masked_cmp_run(2_048);
    assert_flat(
        "masked_cmp_touches",
        "delta",
        (small.touches, small.deltas),
        (large.touches, large.deltas),
    );
}

/// One fused four-stream comparison run over the mask-drift
/// quadruple at `scale` teeth, as [`masked_cmp_run`].
fn masked_pair_cmp_run(scale: usize) -> Run {
    let ((sparse, even_mask), (comb, odd_mask)) =
        Shape::MaskDriftQuadruple.build_quadruple(512, scale);
    let v1 = sparse.version();
    let p1 = before::Party::decode(&even_mask.bytes[..]).expect("the mask is strict normal form");
    let v2 = comb.version();
    let p2 = before::Party::decode(&odd_mask.bytes[..]).expect("the mask is strict normal form");
    let bytes =
        (v1.encode().len() + even_mask.bytes.len() + v2.encode().len() + odd_mask.bytes.len())
            as u64;
    touch_meter::reset();
    let verdict = (&v1 / &p1).partial_cmp(&(&v2 / &p2));
    assert_eq!(
        verdict,
        Some(std::cmp::Ordering::Less),
        "the semantically-empty view sits strictly under the tooth-keeping view"
    );
    let run = Run {
        // The sparse comb's n + 1 leaves put n delta codes behind its
        // first; the full comb adds 2n.
        deltas: 3 * scale as u64,
        bytes,
        touches: touch_meter::touches(),
    };
    assert!(
        run.touches >= run.deltas,
        "masked_pair_cmp scale {scale}: {} digit touches under the {}-delta floor: \
         the walk's integrators are not running on the metered accumulator",
        run.touches,
        run.deltas,
    );
    run
}

/// The fused four-stream comparison's per-delta touches and
/// per-byte touch work stays flat across a tooth-count doubling of
/// the mask-drift quadruple.
///
/// The zero-check on cancelling wide spellings (even teeth) and the
/// mid-oscillation reads (odd teeth) are both amortized O(1) per
/// boundary.
#[test]
fn masked_pair_cmp_drift_cost_is_flat_per_unit() {
    let small = masked_pair_cmp_run(1_024);
    let large = masked_pair_cmp_run(2_048);
    assert_flat(
        "masked_pair_cmp_touches",
        "delta",
        (small.touches, small.deltas),
        (large.touches, large.deltas),
    );
}
