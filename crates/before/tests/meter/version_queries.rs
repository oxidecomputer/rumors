//! Resource envelopes for pairwise version queries and masked comparisons.

use super::*;

// ─── version-pair query scenarios ───────────────────────────────────────────
//
// The public two-operand queries on the pair families (the corpus pairing
// `w = v + one seed tick` collapses the second operand onto a dominating
// plateau, so the co-sweep's orientation switches and its freeze paths
// would go unpriced without these rows). All four rows are linear
// records of the fused co-sweep: the jump-pair rows price wide drift
// crossing the other operand's cheap boundaries over a dense-position
// spine (the shape whose absolute-position accounting read superlinear
// before the anchored-segment discipline; the `version_scaling` band
// test holds it flat across a scale doubling), and the concurrent rows
// price orientation-switch density on word-scale heights.

/// `Version::distance` on the two-operand jump comb stays within its
/// envelope.
///
/// The shape: wide per-level crests of the height difference, parked
/// and settled against segment masses (the `version_scaling` band
/// test carries the cross-scale flatness bound).
///
/// The result is anchored by rank modularity: the distance must equal
/// the two lags' sum.
#[test]
fn version_distance_jump_pair_envelope() {
    let (pa, pb) =
        Shape::JumpPair.build_pair3(JUMP_PAIR_MAGNITUDE_BITS, JUMP_PAIR_TEETH, JUMP_PAIR_DIGITS);
    let a = pa.version();
    let b = pb.version();
    let input_bytes = a.encode().len() + b.encode().len();
    let (r, a, b) = metered(
        "version_distance_jump_pair",
        input_bytes,
        &query_env::DISTANCE_JUMP_PAIR,
        move || {
            let r = a.distance(&b);
            (r, a, b)
        },
    );
    assert_eq!(
        r,
        &a.lag(&b) + &b.lag(&a),
        "the distance must equal the two lags' sum (rank modularity)"
    );
}

/// `Version::lag` on the two-operand jump comb stays within its
/// envelope.
///
/// Lag integrates the directed functional `(h_b − h_a)⁺` over the same
/// overlay walk as distance, so this row prices the one-sided
/// orientation (long zero-orientation stretches where the band
/// dominates) against the symmetric row above.
#[test]
fn version_lag_jump_pair_envelope() {
    let (pa, pb) =
        Shape::JumpPair.build_pair3(JUMP_PAIR_MAGNITUDE_BITS, JUMP_PAIR_TEETH, JUMP_PAIR_DIGITS);
    let a = pa.version();
    let b = pb.version();
    let input_bytes = a.encode().len() + b.encode().len();
    metered(
        "version_lag_jump_pair",
        input_bytes,
        &query_env::LAG_JUMP_PAIR,
        move || {
            let r = a.lag(&b);
            (r, a, b)
        },
    );
}

/// `Version::rank` on one concurrent-pair operand stays within its
/// envelope — the practical-regime constant gauge.
///
/// Word-scale heights over organically forked parties: the regime the
/// overwhelming share of real inputs lives in (every event count fits a
/// machine word, nothing freezes, the promotion ledger never arms). The
/// worst-case rank rows above price the machinery's expensive shapes; this
/// row pins what a benign input pays for that machinery's existence, so
/// a change that cheapens the worst-case path by charging the common
/// one cannot read as an improvement.
#[test]
fn version_rank_concurrent_envelope() {
    let (v, _) = Shape::ConcurrentPair.version_pair(CONCURRENT_PAIR_LEAVES);
    let input_bytes = v.encode().len();
    metered(
        "version_rank_concurrent",
        input_bytes,
        &query_env::RANK_CONCURRENT,
        move || {
            let r = v.rank();
            (r, v)
        },
    );
}

/// `Version::distance` on the concurrent pair stays within its
/// envelope.
///
/// The co-sweep's orientation flips at every one of the `n − 1` overlay
/// boundaries, on word-scale heights, so this row prices switch density
/// with no width in play.
///
/// The semantic anchor: the schedule realizes a distance of exactly the
/// integer rank 2 at every `n` (the generator's construction), so a
/// misrouted orientation switch cannot pass as a cheap reading.
#[test]
fn version_distance_concurrent_envelope() {
    let (v, w) = Shape::ConcurrentPair.version_pair(CONCURRENT_PAIR_LEAVES);
    let input_bytes = v.encode().len() + w.encode().len();
    let (r, _, _) = metered(
        "version_distance_concurrent",
        input_bytes,
        &query_env::DISTANCE_CONCURRENT,
        move || {
            let r = v.distance(&w);
            (r, v, w)
        },
    );
    assert_eq!(
        r,
        uniform_version(2u8).rank(),
        "the schedule's heights must be realized end to end"
    );
}

/// `Version::lag` on the concurrent pair stays within its envelope —
/// the same switch-dense overlay as the distance row, under the
/// one-sided functional (every other plateau reads orientation zero).
#[test]
fn version_lag_concurrent_envelope() {
    let (v, w) = Shape::ConcurrentPair.version_pair(CONCURRENT_PAIR_LEAVES);
    let input_bytes = v.encode().len() + w.encode().len();
    metered(
        "version_lag_concurrent",
        input_bytes,
        &query_env::LAG_CONCURRENT,
        move || {
            let r = v.lag(&w);
            (r, v, w)
        },
    );
}

// ─── masked-comparison scenarios ────────────────────────────────────────────
//
// The fused projected comparisons (`OwnVersion`'s three- and four-stream
// co-walks) on the correlated mask-drift families: ownership toggles at
// every tooth boundary while the other operand's height drift sits on the
// `2^k` carry boundary, so every boundary's sign read lands mid-cancel or
// mid-oscillation. Both scenarios' verdicts are `Less` (pinned by the
// generator tests), so no early exit shortens the measured walk, and both
// anchor the fused verdict against the materialized comparison in the
// same run.

/// The fused three-stream comparison `(comb / mask) ⋚ plateau` stays
/// within its envelope on the correlated triple.
///
/// The mask gates the comb to every other tooth against a flat wide
/// plateau: owned intervals read the near-zero difference spelled by
/// cancelling wide digits, unowned intervals read the zero-check on the
/// plateau's height, and every read is amortized O(1) on the balanced
/// signed-digit accumulator (the flatness band below holds it across a
/// doubling).
#[test]
fn own_version_cmp_mask_drift_envelope() {
    let (comb, mask, plateau) =
        Shape::MaskDriftTriple.build_triple(MASK_DRIFT_MAGNITUDE_BITS, MASK_DRIFT_TEETH);
    let v = comb.version();
    let p = Party::decode(&mask.bytes[..]).expect("the mask is strict normal form");
    let w = plateau.version();
    let input_bytes = v.encode().len() + mask.bytes.len() + w.encode().len();
    let (ord, v, p, w) = metered(
        "own_version_cmp_mask_drift",
        input_bytes,
        &query_env::MASKED_CMP_DRIFT_TRIPLE,
        move || {
            let ord = (&v / &p).partial_cmp(&w);
            (ord, v, p, w)
        },
    );
    assert_eq!(
        ord,
        Some(Ordering::Less),
        "the projected comb sits strictly under the plateau (the full-walk verdict)"
    );
    assert_eq!(
        ord,
        (&v / &p).to_version().partial_cmp(&w),
        "the fused verdict is the materialized verdict"
    );
}

/// The fused four-stream comparison `(v₁/p₁) ⋚ (v₂/p₂)` stays within its
/// envelope on the correlated quadruple.
///
/// The two masks' parities interleave tooth for tooth: even-level teeth
/// read the trichotomy's zero-check on a semantically-zero height spelled
/// by cancelling `2^k`-wide digits, odd-level teeth read the other side's
/// height mid-oscillation across the carry boundary.
#[test]
fn own_version_pair_cmp_mask_drift_envelope() {
    let ((sparse, even_mask), (comb, odd_mask)) =
        Shape::MaskDriftQuadruple.build_quadruple(MASK_DRIFT_MAGNITUDE_BITS, MASK_DRIFT_TEETH);
    let v1 = sparse.version();
    let p1 = Party::decode(&even_mask.bytes[..]).expect("the mask is strict normal form");
    let v2 = comb.version();
    let p2 = Party::decode(&odd_mask.bytes[..]).expect("the mask is strict normal form");
    let input_bytes =
        v1.encode().len() + even_mask.bytes.len() + v2.encode().len() + odd_mask.bytes.len();
    let (ord, v1, p1, v2, p2) = metered(
        "own_version_pair_cmp_mask_drift",
        input_bytes,
        &query_env::MASKED_CMP_DRIFT_QUAD,
        move || {
            let ord = (&v1 / &p1).partial_cmp(&(&v2 / &p2));
            (ord, v1, p1, v2, p2)
        },
    );
    assert_eq!(
        ord,
        Some(Ordering::Less),
        "the semantically-empty view sits strictly under the tooth-keeping view"
    );
    assert_eq!(
        ord,
        (&v1 / &p1)
            .to_version()
            .partial_cmp(&(&v2 / &p2).to_version()),
        "the fused verdict is the materialized verdict"
    );
}

/// The fused three-stream comparison `(spine / mask) ⋚ plateau` stays
/// within its envelope on the masked-hole triple.
///
/// The mask owns one leaf at depth [`MASK_HOLE_MASK_DEPTH`] and leaves the
/// dense spine's whole continuation below it as one unowned run: the
/// walk's block skip must consume that run whole, so the touch ceiling is
/// a function of the mask depth, not the spine depth — the depth band
/// below holds the same reading across a spine-depth doubling — while the
/// scan column holds every skipped bit still read.
#[test]
fn masked_cmp_hole_envelope() {
    let (spine, mask, plateau) =
        Shape::MaskedHoleTriple.build_triple(MASK_HOLE_DEPTH_HI, MASK_HOLE_MASK_DEPTH);
    let v = spine.version();
    let p = Party::decode(&mask.bytes[..]).expect("the mask is strict normal form");
    let w = plateau.version();
    let input_bytes = v.encode().len() + mask.bytes.len() + w.encode().len();
    let (ord, v, p, w) = metered(
        "masked_cmp_hole",
        input_bytes,
        &query_env::MASKED_CMP_HOLE,
        move || {
            let ord = (&v / &p).partial_cmp(&w);
            (ord, v, p, w)
        },
    );
    assert_eq!(
        ord,
        Some(Ordering::Less),
        "the projected spine sits strictly under the plateau (the full-walk verdict)"
    );
    assert_eq!(
        ord,
        (&v / &p).to_version().partial_cmp(&w),
        "the fused verdict is the materialized verdict"
    );
}

/// One masked-hole fused comparison at spine depth `d`: the accumulator
/// touches over the comparison body alone, with the full-walk `Less`
/// verdict enforced (no early exit shortens the measured walk).
#[cfg(feature = "touch-meter")]
fn masked_hole_touches(d: usize) -> u64 {
    let (spine, mask, plateau) = Shape::MaskedHoleTriple.build_triple(d, MASK_HOLE_MASK_DEPTH);
    let v = spine.version();
    let p = Party::decode(&mask.bytes[..]).expect("the mask is strict normal form");
    let w = plateau.version();
    suanpan::touch_meter::reset();
    let verdict = (&v / &p).partial_cmp(&w);
    assert_eq!(
        verdict,
        Some(Ordering::Less),
        "the projected spine sits strictly under the plateau (no early exit)"
    );
    suanpan::touch_meter::touches()
}

/// The flat touch ceiling both depth points must sit under: the measured
/// reading ×1.25 (the reading lives in the pin commit).
///
/// The reading is identical at both spine depths — the block skip makes
/// it a function of the mask depth alone. A per-boundary walk reads ~one
/// touch per spine boundary here (thousands at these depths), so the
/// shared ceiling is what a linear mechanism fails at both points.
#[cfg(feature = "touch-meter")]
const MASK_HOLE_TOUCH_CEILING: u64 = 18;

/// The improvement tripwire under both depth points: the measured reading
/// ×0.75, the envelope columns' tripwire case.
#[cfg(feature = "touch-meter")]
const MASK_HOLE_TOUCH_FLOOR: u64 = 10;

/// The masked walk's block skip is depth-independent: the fused
/// comparison's touches read the same under one flat ceiling at both
/// masked-hole spine depths.
///
/// Shape over point: one ceiling shared across two depths is a claim no
/// per-boundary mechanism can satisfy — a walk that steps the unowned run
/// boundary by boundary scales its touches with the spine depth and fails
/// at both points — while the committed floor (measured ×0.75, the
/// improvement-tripwire case) keeps the column live. The two readings are
/// deterministic and equal: the block skip makes the walk's accumulator
/// work a function of the mask depth alone, so the band also pins the
/// readings' difference at zero.
#[cfg(feature = "touch-meter")]
#[test]
fn masked_cmp_hole_depth_band() {
    let lo = masked_hole_touches(MASK_HOLE_DEPTH_LO);
    let hi = masked_hole_touches(MASK_HOLE_DEPTH_HI);
    eprintln!("MEASURED masked_cmp_hole_depth_band: lo={lo} hi={hi}");
    for (name, reading) in [("lo", lo), ("hi", hi)] {
        assert!(
            reading <= MASK_HOLE_TOUCH_CEILING,
            "masked_cmp_hole depth {name}: {reading} touches exceed the flat ceiling \
             {MASK_HOLE_TOUCH_CEILING}: the unowned run is being consumed per boundary, \
             not as a block"
        );
        assert!(
            reading >= MASK_HOLE_TOUCH_FLOOR,
            "masked_cmp_hole depth {name}: touch counter reads {reading}, below the \
             {MASK_HOLE_TOUCH_FLOOR} improvement tripwire (measured x0.75): attribute \
             the drop — a genuine improvement re-pins the band; a dead meter is the \
             bypass this column exists to catch"
        );
    }
    assert_eq!(
        lo, hi,
        "the block skip makes the accumulator work a function of the mask depth alone: \
         a spine-depth doubling may not move the touch reading"
    );
}
