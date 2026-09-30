//! Flatness checks for rank and minimum-tick freeze schedules.

use super::*;

/// One `Version::rank` run over the freeze-position family
/// `FP(k)`, both counters over the rank body alone.
///
/// Carries `min_ticks`' closed form as the cross-fold semantic leg
/// (proving the generator builds the tree this band reasons about)
/// and the one-touch-per-operand-byte liveness floor.
fn rank_freeze_position_run(k: usize) -> QueryRun {
    let encoded = Shape::FreezePosition.build1(k);
    let v = encoded.version();
    let bytes = v.encode().len() as u64;
    let band = 289 + (usize::BITS - k.leading_zeros()) as usize;
    let expected = (BigUint::from(2 * k as u64) << band)
        + BigUint::from((k * (k - 1)) as u64) * ((BigUint::ONE << 288usize) + BigUint::ONE)
        + BigUint::from(k as u64);
    assert_eq!(
        v.min_ticks(),
        ticks_from_big(&expected),
        "the family's leaf sum disagrees with min_ticks: the generator \
         does not build the tree this band reasons about"
    );
    touch_meter::reset();
    let rank = v.rank();
    std::hint::black_box(rank);
    let run = QueryRun {
        bytes,
        touches: touch_meter::touches(),
    };
    assert!(
        run.touches >= run.bytes,
        "rank at {bytes} operand bytes: {} digit touches under the \
         one-per-byte floor: the fold's accumulator work is not metered",
        run.touches,
    );
    run
}

/// Blocks of the freeze-position band's small run (the large run
/// doubles the block count, doubling the operand's size).
const RANK_FREEZE_POSITION_SMALL: usize = 1_000;

/// Absolute touch ceilings at two scales for rank on the
/// freeze-position family, measured ×1.25 (the record and every
/// re-pin's movement live in the pin commits).
///
/// The anchored-segment integral reads flat per encoded byte across
/// the doubling; an accounting that reads the position
/// accumulator's whole written span per freeze reads superlinear
/// and exceeds these ceilings.
const RANK_FREEZE_POSITION_CEILINGS: [u64; 2] = [95_040, 190_342];

/// rank is linear on the freeze-position family: per-byte touch work stays
/// flat (×1.25) across a block-count doubling, under
/// absolute two-scale ceilings.
///
/// `FP(k)` fires one freeze per block — `Θ(k)` freezes at
/// ever-deeper stream positions, every committed comb's count being
/// O(1) — so any freeze accounting that reads an absolute position
/// (or any whole-history state) per freeze goes quadratic here
/// while the family's positions compact to O(1) digits. The
/// anchored-segment discipline settles each freeze against its own
/// segment's mass instead (read through the write watermark, whose span
/// never scales), so the flatness bound holds.
#[test]
fn skyline_rank_freeze_position_is_flat_per_unit() {
    let small = rank_freeze_position_run(RANK_FREEZE_POSITION_SMALL);
    let large = rank_freeze_position_run(2 * RANK_FREEZE_POSITION_SMALL);
    assert_ceilings(
        "skyline_rank_freeze_position",
        &small,
        &large,
        RANK_FREEZE_POSITION_CEILINGS,
    );
    assert_flat(
        "rank_freeze_position_touches",
        "byte",
        (small.touches, small.bytes),
        (large.touches, large.bytes),
    );
}

/// Build the freeze-position family's descending unit spine through public
/// party and version operations.
fn freeze_position_mate(k: usize) -> before::Version {
    let mut tail = before::Party::seed();
    let mut leaves = Vec::with_capacity(2 * k + 1);
    for _ in 0..2 * k {
        let right = tail.fork();
        leaves.push(tail);
        tail = right;
    }
    leaves.push(tail);

    before::Version::new().join_all(leaves.into_iter().zip((0..=2 * k).rev()).filter_map(
        |(party, ticks)| {
            if ticks == 0 {
                return None;
            }
            let mut version = before::Version::new();
            version.ticks(&party, ticks as u64);
            Some(version)
        },
    ))
}

/// Measure distance and lag between the freeze-position family and its
/// descending unit spine.
fn distance_freeze_position_run(k: usize) -> QueryRun {
    let a = Shape::FreezePosition.build1(k).version();
    let b = freeze_position_mate(k);
    let bytes = (a.encode().len() + b.encode().len()) as u64;
    let gap = a
        .rank()
        .checked_sub(&b.rank())
        .expect("the freeze-position version dominates its mate");

    touch_meter::reset();
    let distance = a.distance(&b);
    let forward = a.lag(&b);
    let backward = b.lag(&a);
    let run = QueryRun {
        bytes,
        touches: touch_meter::touches(),
    };

    assert_eq!(distance, gap, "distance is the rank difference");
    assert_eq!(
        forward,
        before::Rank::ZERO,
        "the greater version has no lag"
    );
    assert_eq!(
        backward, distance,
        "the lesser version lags by the distance"
    );
    assert!(
        run.touches >= run.bytes,
        "pair queries at {bytes} encoded bytes recorded too few touches: {}",
        run.touches,
    );
    run
}

/// Absolute two-scale ceilings for distance and lag on the
/// freeze-position family and its mate.
const DISTANCE_FREEZE_POSITION_CEILINGS: [u64; 2] = [268_742, 537_839];

/// Distance and lag remain linear as the number of freeze positions
/// doubles.
#[test]
fn skyline_distance_freeze_position_is_flat_per_unit() {
    let small = distance_freeze_position_run(RANK_FREEZE_POSITION_SMALL);
    let large = distance_freeze_position_run(2 * RANK_FREEZE_POSITION_SMALL);
    assert_ceilings(
        "skyline_distance_freeze_position",
        &small,
        &large,
        DISTANCE_FREEZE_POSITION_CEILINGS,
    );
    assert_flat(
        "distance_freeze_position_touches",
        "byte",
        (small.touches, small.bytes),
        (large.touches, large.bytes),
    );
}

/// One public `Version::rank` run over the promotion re-arm spine
/// `PR(p)` (`meter::promotion_rearm`), both counters over the rank
/// body alone.
///
/// Carries `min_ticks`' closed form as the cross-fold semantic leg
/// (proving the generator builds the re-arm spine this band
/// reasons about) and the one-touch-per-operand-byte liveness
/// floor.
fn rank_promotion_rearm_run(p: usize) -> QueryRun {
    let v = Shape::PromotionRearm.build1(p).version();
    let bytes = v.encode().len() as u64;
    let expected = BigUint::from(16 * p as u64)
        + BigUint::from(p as u64) * ((BigUint::ONE << 608usize) + (BigUint::ONE << 288usize) + 2u8)
        + 1u8;
    assert_eq!(
        v.min_ticks(),
        ticks_from_big(&expected),
        "the family's stored-code sum disagrees with min_ticks: the \
         generator does not build the tree this band reasons about"
    );
    touch_meter::reset();
    let rank = v.rank();
    std::hint::black_box(rank);
    let run = QueryRun {
        bytes,
        touches: touch_meter::touches(),
    };
    assert!(
        run.touches >= run.bytes,
        "rank at {bytes} operand bytes: {} digit touches under the \
         one-per-byte floor: the fold's accumulator work is not metered",
        run.touches,
    );
    run
}

/// Blocks of the promotion re-arm bands' small runs (the large runs
/// double the count).
const PROMOTION_REARM_SMALL: usize = 1_000;

/// Absolute two-scale touch ceilings for rank on the
/// promotion re-arm spine, measured ×1.25 (the record and every
/// re-pin's movement and attribution live in the pin commits).
///
/// The cluster-delegated settle reads flat per encoded byte across
/// the doubling, with the settle's window-digit traffic metered;
/// rereading the position accumulator's whole written span at every
/// promotion would exceed these ceilings.
const RANK_PROMOTION_REARM_CEILINGS: [u64; 2] = [443_187, 886_544];

/// rank is linear on the promotion re-arm spine: per-byte touch work stays
/// flat (×1.25) across a block-count doubling, under
/// absolute two-scale ceilings.
///
/// `PR(p)` fires one promotion per block at O(1) stored codes,
/// after a `32p`-level climb keeps the consumed mass's written span
/// growing — every committed comb promotes never, and the
/// freeze-position spine's parked drift is monotone — so any
/// promotion accounting that re-reads whole-history state per
/// arming goes quadratic here while the family's suffix masses
/// compact to O(1) balanced terms. The promotion ledger records
/// each arming at funded widths and settles once at the sweep's close, so
/// the flatness bound holds.
#[test]
fn skyline_rank_promotion_rearm_is_flat_per_unit() {
    let small = rank_promotion_rearm_run(PROMOTION_REARM_SMALL);
    let large = rank_promotion_rearm_run(2 * PROMOTION_REARM_SMALL);
    assert_ceilings(
        "skyline_rank_promotion_rearm",
        &small,
        &large,
        RANK_PROMOTION_REARM_CEILINGS,
    );
    assert_flat(
        "rank_promotion_rearm_touches",
        "byte",
        (small.touches, small.bytes),
        (large.touches, large.bytes),
    );
}

/// One public `Version::rank` run over the lone-freeze spine
/// `LF(pre, post)` (`meter::lone_freeze`), both counters over the
/// rank body alone.
///
/// Carries `min_ticks`' closed form as the cross-fold semantic leg
/// (proving the generator builds the spine this band reasons
/// about) and the one-touch-per-operand-byte liveness floor.
fn rank_lone_freeze_run(pre: usize, post: usize) -> QueryRun {
    let v = Shape::LoneFreeze.build2(pre, post).version();
    let bytes = v.encode().len() as u64;
    let expected = BigUint::from(pre as u64) * ((BigUint::ONE << 288usize) + BigUint::from(2u8))
        + BigUint::from((pre / 2) as u64)
        + BigUint::from((3 * post / 2) as u64)
        + BigUint::from(3u8);
    assert_eq!(
        v.min_ticks(),
        ticks_from_big(&expected),
        "the family's leaf sum disagrees with min_ticks: the generator \
         does not build the tree this band reasons about"
    );
    touch_meter::reset();
    let rank = v.rank();
    std::hint::black_box(rank);
    let run = QueryRun {
        bytes,
        touches: touch_meter::touches(),
    };
    assert!(
        run.touches >= run.bytes,
        "rank at {bytes} operand bytes: {} digit touches under the \
         one-per-byte floor: the fold's accumulator work is not metered",
        run.touches,
    );
    run
}

/// Oscillation pairs of the lone-freeze bands' doubled axis (the
/// large runs double it; the off-axis dial stays at the generator
/// minimum so the doubled axis dominates the stored bytes).
const LONE_FREEZE_SMALL: usize = 2_000;

/// Absolute touch ceilings at two scales for rank on the
/// lone-freeze late axis, measured ×1.25 (the record lives in the
/// pin commit).
///
/// Flat per encoded byte across the doubling: the gate holds the
/// segment feed shut across the whole never-freezing prefix.
const RANK_LONE_FREEZE_LATE_CEILINGS: [u64; 2] = [5_233, 10_312];

/// rank is linear on the lone-freeze spine's late axis: per-byte
/// touch work stays flat (×1.25) across a doubling of the
/// never-freezing plateau prefix, under absolute two-scale
/// ceilings.
///
/// `LF(pre, 2)`'s whole prefix runs strictly before the sweep's
/// one freeze — the first-freeze gate holds the segment feed shut
/// for `pre` oscillation pairs, and the one settle that eventually
/// runs never reads mass from that span — so any per-interval
/// deposit toward the settle machinery made before drift exists to
/// settle scales with the prefix here while the family's funded
/// wide codes stay O(1). The unit oscillation itself must ride the
/// live component without freezing (the trigger is relative to
/// each boundary's own code).
#[test]
fn skyline_rank_lone_freeze_late_is_flat_per_unit() {
    let small = rank_lone_freeze_run(LONE_FREEZE_SMALL, 2);
    let large = rank_lone_freeze_run(2 * LONE_FREEZE_SMALL, 2);
    assert_ceilings(
        "skyline_rank_lone_freeze_late",
        &small,
        &large,
        RANK_LONE_FREEZE_LATE_CEILINGS,
    );
    assert_flat(
        "rank_lone_freeze_late_touches",
        "byte",
        (small.touches, small.bytes),
        (large.touches, large.bytes),
    );
}

/// Absolute touch ceilings at two scales for rank on the
/// lone-freeze frozen-tail axis, measured ×1.25 (the record lives
/// in the pin commit).
///
/// Flat per encoded byte across the doubling. The tail axis adds
/// the open-gate segment feed over the late axis — amortized O(1)
/// touches per interval — and the close's one settle reads the
/// whole tail's banked mass without moving the per-byte cost.
const RANK_LONE_FREEZE_TAIL_CEILINGS: [u64; 2] = [7_808, 15_465];

/// rank is linear on the lone-freeze spine's frozen-tail axis:
/// per-byte touch work stays flat (×1.25) across a
/// doubling of the tail behind the sweep's one freeze, under
/// absolute two-scale ceilings.
///
/// `LF(2, post)`'s whole tail runs with the first-freeze gate open
/// and a ten-digit drift parked: every tail interval feeds the
/// segment mass, and the close's one `P · segment` settle reads
/// that mass at its watermark across the tail's whole depth
/// variation — so a segment feed that is not amortized O(1) per
/// interval, or a close read priced by anything but the written
/// span and the mass's compacted density, scales with the tail
/// against O(1) funded wide codes. This is the frozen-path cost
/// the gate must not regress: the segment machinery a
/// never-freezing sweep skips runs here over the whole stream.
#[test]
fn skyline_rank_lone_freeze_tail_is_flat_per_unit() {
    let small = rank_lone_freeze_run(2, LONE_FREEZE_SMALL);
    let large = rank_lone_freeze_run(2, 2 * LONE_FREEZE_SMALL);
    assert_ceilings(
        "skyline_rank_lone_freeze_tail",
        &small,
        &large,
        RANK_LONE_FREEZE_TAIL_CEILINGS,
    );
    assert_flat(
        "rank_lone_freeze_tail_touches",
        "byte",
        (small.touches, small.bytes),
        (large.touches, large.bytes),
    );
}

/// Absolute touch ceilings at two scales for min_ticks on the
/// freeze-position spine, measured ×1.25 (the record lives in the
/// pin commit).
///
/// Flat per encoded byte across the doubling: `Θ(k)` frozen components settle
/// at one funded-width product each.
const MIN_TICKS_FREEZE_POSITION_CEILINGS: [u64; 2] = [129_988, 259_988];

/// min_ticks is linear on the freeze-position family: per-byte
/// touch work stays flat (×1.25) across a block-count
/// doubling, under absolute two-scale ceilings.
///
/// `FP(k)` freezes one height component per block. Prefix-height accounting
/// settles each component once using the suffix of its recorded coefficients.
/// Re-reading all prior components at every freeze, or rebasing each recorded
/// offset across a freeze, would go quadratic. Each component instead pays one
/// product at its own funded width with a word-sized suffix count. The rank-side
/// band prices the same schedule through the anchored-segment
/// integral; this one prices the prefix-height accounting, min_ticks' own
/// frozen-component accounting.
#[test]
fn skyline_min_ticks_freeze_position_is_flat_per_unit() {
    let expected = |k: usize| {
        let band = 289 + (usize::BITS - k.leading_zeros()) as usize;
        (BigUint::from(2 * k as u64) << band)
            + BigUint::from((k * (k - 1)) as u64) * ((BigUint::ONE << 288usize) + BigUint::ONE)
            + BigUint::from(k as u64)
    };
    let k = RANK_FREEZE_POSITION_SMALL;
    let small = min_ticks_family_run(Shape::FreezePosition.build1(k), &expected(k));
    let large = min_ticks_family_run(Shape::FreezePosition.build1(2 * k), &expected(2 * k));
    assert_ceilings(
        "skyline_min_ticks_freeze_position",
        &small,
        &large,
        MIN_TICKS_FREEZE_POSITION_CEILINGS,
    );
    assert_flat(
        "min_ticks_freeze_position_touches",
        "byte",
        (small.touches, small.bytes),
        (large.touches, large.bytes),
    );
}

/// Absolute touch ceilings at two scales for min_ticks on the
/// promotion re-arm spine, measured ×1.25 (the record lives in the
/// pin commit).
///
/// Flat per encoded byte across the doubling: `Θ(p)` wide-drift
/// components in both directions settle at one funded-width product
/// each.
const MIN_TICKS_PROMOTION_REARM_CEILINGS: [u64; 2] = [645_075, 1_290_075];

/// min_ticks is linear on the promotion re-arm spine: per-byte
/// touch work stays flat (×1.25) across a block-count
/// doubling, under absolute two-scale ceilings.
///
/// `PR(p)` alternates 20-digit and 10-digit climbs through its
/// blocks — `Θ(p)` freezes whose evicted drifts are wide in both
/// directions. Leaves and subtree minima retain the frozen prefix current when
/// they were observed, so settlement charges every wide component once at its
/// funded width. Re-reading the whole prefix history for each component would
/// go quadratic.
/// The rank-side band prices this schedule through the promotion
/// ledger; min_ticks has no promotion ledger — the prefix-height accounting is
/// its entire frozen-component accounting, and this band covers many freezes.
#[test]
fn skyline_min_ticks_promotion_rearm_is_flat_per_unit() {
    let expected = |p: usize| {
        BigUint::from(16 * p as u64)
            + BigUint::from(p as u64)
                * ((BigUint::ONE << 608usize) + (BigUint::ONE << 288usize) + 2u8)
            + 1u8
    };
    let p = PROMOTION_REARM_SMALL;
    let small = min_ticks_family_run(Shape::PromotionRearm.build1(p), &expected(p));
    let large = min_ticks_family_run(Shape::PromotionRearm.build1(2 * p), &expected(2 * p));
    assert_ceilings(
        "skyline_min_ticks_promotion_rearm",
        &small,
        &large,
        MIN_TICKS_PROMOTION_REARM_CEILINGS,
    );
    assert_flat(
        "min_ticks_promotion_rearm_touches",
        "byte",
        (small.touches, small.bytes),
        (large.touches, large.bytes),
    );
}

/// One public distance-and-lag run over the two-operand promotion
/// re-arm analogue `(PR(p), PRM(p))`: both counters over the three
/// query bodies together, with the pair's input bytes as the
/// per-byte denominator.
///
/// The mate is `PR(p)`'s unit-climb twin (same topology, every base
/// 1), so the co-sweep's freezes and promotions fire at boundaries
/// where the mate's cheap codes set the funded width while the
/// drift being parked and promoted was deposited by the re-arm
/// operand's wide codes — the two-operand arming case the
/// freeze-position analogue's monotone mate cannot reach (its own
/// doc records that promotion never fires there; the committed
/// span-promotion pair tripwire proves it fires here). Value legs
/// anchor all three measures before the counters return: `PR(p)`
/// dominates its mate pointwise, so `distance = rank(a) − rank(b)`,
/// `lag(a, b) = 0`, and `lag(b, a) = distance`.
fn distance_promotion_rearm_run(p: usize) -> QueryRun {
    let a = Shape::PromotionRearm.build1(p).version();
    let b = Shape::PromotionRearmMate.build1(p).version();
    let bytes = (a.encode().len() + b.encode().len()) as u64;
    let gap = a
        .rank()
        .checked_sub(&b.rank())
        .expect("the re-arm operand dominates its unit mate");
    touch_meter::reset();
    let d = a.distance(&b);
    let forward = a.lag(&b);
    let backward = b.lag(&a);
    let run = QueryRun {
        bytes,
        touches: touch_meter::touches(),
    };
    assert_eq!(d, gap, "distance must be the dominating rank gap");
    assert_eq!(
        forward,
        before::Rank::ZERO,
        "the dominating side lags by nothing"
    );
    assert_eq!(backward, d, "the dominated side lags by the whole gap");
    assert!(
        run.touches >= run.bytes,
        "pair queries at {bytes} operand bytes: {} digit touches under \
         the one-per-byte floor: the co-sweep's difference state is not \
         running on the metered accumulator",
        run.touches,
    );
    run
}

/// Absolute touch ceilings at two scales for the distance/lag
/// triple on the promotion re-arm analogue, measured ×1.25 (the
/// record and every re-pin's movement and attribution live in the
/// pin commits).
///
/// The ceilings price the three query bodies together — three
/// sweeps' worth — flat per encoded byte across the doubling, including
/// the settle's window-digit traffic.
const DISTANCE_PROMOTION_REARM_CEILINGS: [u64; 2] = [1_165_230, 2_330_635];

/// Distance and lag are linear on the promotion re-arm analogue:
/// the two-operand arming case reads flat (×1.25) per encoded byte
/// across a block-count doubling, under absolute two-scale
/// ceilings.
///
/// One operand's cheap codes fire freezes and promotions of drift
/// only the other operand's wide codes deposited — the promotion
/// ledger records each arming at funded widths and settles once,
/// so no charge reads an absolute position and the flatness bound
/// holds.
#[test]
fn skyline_distance_promotion_rearm_is_flat_per_unit() {
    let small = distance_promotion_rearm_run(PROMOTION_REARM_SMALL);
    let large = distance_promotion_rearm_run(2 * PROMOTION_REARM_SMALL);
    assert_ceilings(
        "skyline_distance_promotion_rearm",
        &small,
        &large,
        DISTANCE_PROMOTION_REARM_CEILINGS,
    );
    assert_flat(
        "distance_promotion_rearm_touches",
        "byte",
        (small.touches, small.bytes),
        (large.touches, large.bytes),
    );
}
