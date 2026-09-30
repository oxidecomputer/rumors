//! Bands that prove accumulator skip summaries remain sufficient.

use super::*;

// ── the accumulator skip mechanisms' before-level adequacy bands ──
//
// Three families, one per skip/extent mechanism inside the
// accumulator (`suanpan`), each constructed so that the mechanism's
// *absence* — scans stepping digit by digit instead of consuming a
// zero-run certificate; scaled reads starting at digit 0 instead of
// the write watermark; loop bounds and fold starts reading the
// buffer's high water instead of the settled top — turns one public
// `before` operation superlinear while the family's input stays
// linear (demonstrated by disabling exactly one mechanism in a
// local probe build, value-identical by the full differential
// suite; the probe readings live in the pin commits). On the
// shipped accumulator all three read flat; each band is the
// before-level witness that its mechanism is load-bearing, priced
// through the public API rather than through `suanpan`'s own entry
// points (whose row witnesses,
// `alternating_shifted_writes_cost_the_operand_not_the_gap`,
// `scaled_read_costs_the_written_span`, and
// `held_width_rows_cost_the_held_digits`, pin the same three
// mechanisms crate-locally).

/// One `Version::rank` run over the weight-comb family `WC(n)`
/// (`meter::weight_comb`), both counters over the rank body alone,
/// with the tick total as the semantic leg and a
/// one-touch-per-topology-byte liveness floor.
fn rank_weight_comb_run(n: usize) -> QueryRun {
    let v = Shape::WeightComb.build1(n).version();
    let bytes = v.encode().len() as u64;
    // Σ stored bases: the spine's 32n − 1 unit leaves plus the
    // block's n twos.
    let expected = BigUint::from((34 * n - 1) as u64);
    assert_eq!(
        v.min_ticks(),
        ticks_from_big(&expected),
        "the family's base sum disagrees with min_ticks: the generator \
         does not build the tree this band reasons about"
    );
    touch_meter::reset();
    let rank = v.rank();
    std::hint::black_box(rank);
    let run = QueryRun {
        bytes,
        touches: touch_meter::touches(),
    };
    // The liveness floor is the mechanism's irreducible work, not
    // the family's typical work: every nonzero stored delta folds
    // into the integral's accumulator at least once, and the
    // block's `2n` leaves alternate heights 0 and 2, so all `2n`
    // of its deltas are nonzero.
    assert!(
        run.touches >= 2 * n as u64,
        "rank on WC({n}): {} digit touches under the one-per-nonzero-delta \
         floor of {}: the fold's accumulator work is not metered",
        run.touches,
        2 * n,
    );
    run
}

/// Absolute touch ceilings at two scales for rank on the
/// weight comb, measured ×1.25 (the record and every re-pin's
/// movement live in the pin commits).
///
/// Flat per encoded byte across the doubling: this family never
/// freezes, so no segment feed deposits, and its wide cycling pays
/// one spill each time the quick register is re-entered. With certificate
/// consumption disabled (a local probe build whose scans step
/// digit by digit), the reading goes quadratic — `n² + O(n)`
/// touches — and fails the band, so this band is the before-level
/// adequacy witness for the zero-run ledger.
const RANK_WEIGHT_COMB_CEILINGS: [u64; 2] = [6_414, 12_814];

/// Block pairs of the weight-comb band's small run.
const RANK_WEIGHT_COMB_SMALL: usize = 512;

/// rank is linear on the weight comb: per-byte touch work stays
/// flat (×1.25) across a block doubling, under absolute two-scale
/// ceilings.
///
/// `WC(n)` re-raises and cancels one digit `Θ(n)` digits above a
/// parked unit, `Θ(n)` times, for O(1) stored bits per event — the
/// position weight is topology, so no code funds the gap between.
/// Each cancellation forces the accumulator's top to settle back
/// across the never-written gap: a settlement that walks the gap
/// pays `Θ(n)` unfunded touches per event (`Θ(n²)` on linear
/// input), and the parked digit-0 unit forecloses value-emptiness
/// and write-watermark shortcuts — one certificate per jumped run,
/// consumed whole, is what holds this band flat. This is the
/// public-API lift of the accumulator's own row witness
/// (`alternating_shifted_writes_cost_the_operand_not_the_gap`):
/// there the shift is a free parameter; here the stream buys the
/// position with `Θ(n)` one-time topology bits and then oscillates
/// at O(1) bits per event.
#[test]
fn skyline_rank_weight_comb_is_flat_per_unit() {
    let small = rank_weight_comb_run(RANK_WEIGHT_COMB_SMALL);
    let large = rank_weight_comb_run(2 * RANK_WEIGHT_COMB_SMALL);
    assert_ceilings(
        "skyline_rank_weight_comb",
        &small,
        &large,
        RANK_WEIGHT_COMB_CEILINGS,
    );
    assert_flat(
        "rank_weight_comb_touches",
        "byte",
        (small.touches, small.bytes),
        (large.touches, large.bytes),
    );
}

/// One `Version::rank` run over the freeze-parade family `FZ(k)`
/// (`meter::freeze_parade`): the same harness as the weight
/// comb's.
fn rank_freeze_parade_run(k: usize) -> QueryRun {
    let v = Shape::FreezeParade.build1(k).version();
    let bytes = v.encode().len() as u64;
    // Σ printed bases in closed form: the spine's 64k − 1 unit
    // leaves, the block's k left-leaf wide drops, its internal
    // left children's half-minima differences (k/2 per level,
    // each level's difference doubling from the pair stride
    // 2^288 + 1), and its root's absolute minimum
    // 2^band − (k − 1)(2^288 + 1) − 2^288.
    let j = (usize::BITS - k.leading_zeros()) as usize - 1;
    let band = 290 + (usize::BITS - k.leading_zeros()) as usize;
    let w = BigUint::ONE << 288usize;
    let stride = &w + BigUint::ONE;
    let expected = BigUint::from((64 * k - 1) as u64)
        + (BigUint::ONE << band)
        + BigUint::from(k as u64) * &w
        + BigUint::from((k / 2 * j) as u64) * &stride
        - BigUint::from((k - 1) as u64) * &stride
        - &w;
    assert_eq!(
        v.min_ticks(),
        ticks_from_big(&expected),
        "the family's base sum disagrees with min_ticks: the generator \
         does not build the tree this band reasons about"
    );
    touch_meter::reset();
    let rank = v.rank();
    std::hint::black_box(rank);
    let run = QueryRun {
        bytes,
        touches: touch_meter::touches(),
    };
    // The liveness floor is the mechanism's irreducible work, not
    // the family's typical work: every nonzero stored delta folds
    // into the integral's accumulator at least once, and each of
    // the `k` freeze blocks stores two nonzero drops (the wide
    // in-pair `2^288` and the unit cross-pair code).
    assert!(
        run.touches >= 2 * k as u64,
        "rank on FZ({k}): {} digit touches under the one-per-nonzero-delta \
         floor of {}: the fold's accumulator work is not metered",
        run.touches,
        2 * k,
    );
    run
}

/// Absolute touch ceilings at two scales for rank on the
/// freeze parade, measured ×1.25 (the record and every re-pin's
/// movement live in the pin commits).
///
/// Flat per encoded byte across the doubling: the segment feed
/// opens at the first freeze, so the deep pre-freeze spine
/// deposits nothing while the freeze blocks' banked segments and
/// settles are priced whole. With the write watermark disabled (a
/// local probe build whose scaled reads start at digit 0), the
/// touch reading goes quadratic — every settle
/// re-walks the `Θ(k)`-digit never-written prefix — and fails the
/// band, so this band is the before-level adequacy witness for the
/// watermark read.
const RANK_FREEZE_PARADE_CEILINGS: [u64; 2] = [58_468, 116_913];

/// Freeze blocks of the parade band's small run.
const RANK_FREEZE_PARADE_SMALL: usize = 512;

/// rank is linear on the freeze parade: per-byte touch work stays
/// flat (×1.25) across a block doubling, under absolute two-scale
/// ceilings.
///
/// `FZ(k)` fires `Θ(k)` freezes whose segments all sit `Θ(k)`
/// digits above digit 0 (the blocks are shallow; the deep spine
/// only sets the scale), so every settle's segment read crosses a
/// `Θ(k)`-digit never-written prefix. The watermark read prices
/// each at the segment's written span; a read that starts at digit
/// 0 pays the prefix per freeze — `Θ(k²)` touches on linear input.
/// The freeze-position family pins the
/// query-layer half of this case (no absolute position is read
/// per freeze); this band pins the accumulator half — the
/// public-API lift of `scaled_read_costs_the_written_span`.
#[test]
fn skyline_rank_freeze_parade_is_flat_per_unit() {
    let small = rank_freeze_parade_run(RANK_FREEZE_PARADE_SMALL);
    let large = rank_freeze_parade_run(2 * RANK_FREEZE_PARADE_SMALL);
    assert_ceilings(
        "skyline_rank_freeze_parade",
        &small,
        &large,
        RANK_FREEZE_PARADE_CEILINGS,
    );
    assert_flat(
        "rank_freeze_parade_touches",
        "byte",
        (small.touches, small.bytes),
        (large.touches, large.bytes),
    );
}

/// One comparison-sweep run over the tooth-tail pair `TT(g, m)`
/// (`meter::tooth_tail`): touches over the `causal_cmp` body
/// alone, with the verdict as the semantic leg and a
/// one-touch-per-boundary liveness floor.
fn cmp_tooth_tail_run(g: usize, m: usize) -> QueryRun {
    let (a, b) = Shape::ToothTail.build_pair(g, m);
    let (a, b) = (a.version(), b.version());
    let bytes = (a.as_bytes().len() + b.as_bytes().len()) as u64;
    touch_meter::reset();
    let verdict = meter::version::causal_cmp(&a, &b);
    let run = QueryRun {
        bytes,
        touches: touch_meter::touches(),
    };
    assert_eq!(
        verdict,
        Some(std::cmp::Ordering::Less),
        "b runs one tick above a everywhere except the shared terminal"
    );
    assert!(
        run.touches >= m as u64,
        "cmp at {m} boundaries: {} digit touches under the \
         one-per-boundary floor: the sweep's difference state is not \
         running on the metered accumulator",
        run.touches,
    );
    run
}

/// Absolute touch ceilings at two scales for the comparison
/// sweep on the tooth-tail pair, measured ×1.25 (the record lives
/// in the pin commit).
///
/// Flat per encoded byte across the doubling. With the settled top
/// replaced by the buffer's high water (a local probe build), the
/// reading goes quadratic — `2(g + 1)` touches per boundary, the
/// spike's dead digits re-walked per sign read — and fails the
/// band, so this band is the before-level adequacy witness for
/// exact-top maintenance.
const CMP_TOOTH_TAIL_CEILINGS: [u64; 2] = [5_298, 10_578];

/// Boundaries of the tooth-tail band's small run.
const CMP_TOOTH_TAIL_SMALL: usize = 4_096;

/// The comparison sweep is linear on the tooth-tail pair: per-byte
/// touch work stays flat (×1.25) across a joint `(g, m)` doubling,
/// under absolute two-scale ceilings.
///
/// `TT(g, m)`'s cancelled spike leaves the difference accumulator
/// holding −1 in one digit under a buffer `g` digits tall, and the
/// sweep then reads `sign(D)` once per boundary, `m` times, with
/// no intervening write. The settled top prices each read at the
/// value's width; any high-water bound re-walks the spike's `g`
/// dead digits per read — `Θ(m·g)` on `Θ(m + g)` input, the cost
/// the spike's own code paid once and would otherwise be re-paid
/// per boundary forever. The public-API lift of
/// `held_width_rows_cost_the_held_digits`: reads price the settled
/// width, and the settlement (with its certificate skip) is what
/// keeps the settled width accurate after a cancellation.
#[test]
fn skyline_cmp_tooth_tail_is_flat_per_unit() {
    let m = CMP_TOOTH_TAIL_SMALL;
    let small = cmp_tooth_tail_run(m / 64, m);
    let large = cmp_tooth_tail_run(m / 32, 2 * m);
    assert_ceilings(
        "skyline_cmp_tooth_tail",
        &small,
        &large,
        CMP_TOOTH_TAIL_CEILINGS,
    );
    assert_flat(
        "cmp_tooth_tail_touches",
        "byte",
        (small.touches, small.bytes),
        (large.touches, large.bytes),
    );
}

/// One public `Version::rank` run over the dense-suffix family
/// `DS(p, p)` (`meter::dense_suffix`), both counters over the rank
/// body alone.
///
/// Carries `min_ticks`' closed form as the cross-fold semantic leg
/// (proving the generator builds the gap spine and the block
/// schedule this band reasons about) and the
/// one-touch-per-operand-byte liveness floor.
fn rank_dense_suffix_run(p: usize) -> QueryRun {
    let v = Shape::DenseSuffix.build2(p, p).version();
    let bytes = v.encode().len() as u64;
    let expected = BigUint::from(p as u64)
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

/// Blocks (and suffix digits) of the dense-suffix bands' small runs
/// (the large runs double both).
const DENSE_SUFFIX_SMALL: usize = 500;

/// Absolute touch ceilings at two scales for rank on the
/// dense-suffix family, measured ×1.25 (the record and every
/// re-pin's movement and attribution live in the pin commits).
///
/// The mass-balanced product-tree settle reads flat per encoded
/// byte across the doubling — each aggregate charge is one backend
/// product per dense cluster, never a factor-wide product per
/// window digit — where a per-arming suffix walk re-walks the
/// suffix's Θ(d) balanced digits per arming and reads quadratic.
const RANK_DENSE_SUFFIX_CEILINGS: [u64; 2] = [224_705, 448_907];

/// rank is flat per byte on the dense-suffix family under the
/// declared log model: per-byte touch work stays within
/// ×1.25 across a block-count doubling, under absolute two-scale
/// ceilings.
///
/// `DS(p, p)` fires one promotion per block against a trailing
/// interval mass the gap spine holds at Θ(p) balanced digits — the
/// shape on which any settle that walks the suffix once per arming
/// (or re-reads a promoted prefix once per window) goes quadratic.
/// The mass-balanced product tree charges every arming-window
/// cross term inside exactly one aggregate product and rewrites
/// any window's digits once per tree level, so the declared model
/// admits per-byte growth up to the log ratio — at this family's
/// shape a doubling could read at most ×(log₂ 2p / log₂ p) ≈ ×1.11
/// even if the settle dominated the fold, inside the band's ×1.25
/// slack — and the settle's log term is a small share of the
/// fold's linear work, so the reading sits well inside the band.
#[test]
fn skyline_rank_dense_suffix_is_flat_per_unit() {
    let small = rank_dense_suffix_run(DENSE_SUFFIX_SMALL);
    let large = rank_dense_suffix_run(2 * DENSE_SUFFIX_SMALL);
    assert_ceilings(
        "skyline_rank_dense_suffix",
        &small,
        &large,
        RANK_DENSE_SUFFIX_CEILINGS,
    );
    assert_flat(
        "rank_dense_suffix_touches",
        "byte",
        (small.touches, small.bytes),
        (large.touches, large.bytes),
    );
}

/// One public distance-and-lag run over `(DS(p, p), DSM(p, p))`:
/// both counters over the three query bodies together, with the
/// pair's input bytes as the per-byte denominator.
///
/// The mate is `DS(p, p)`'s unit-block twin, so the co-sweep's
/// freezes and promotions fire at boundaries where the mate's
/// cheap codes set the funded width while the drift being parked
/// and promoted was deposited by the wide operand — and every
/// arming owes its debt across the same dense trailing mass.
/// Value legs anchor all three measures before the counters
/// return: `DS` dominates its mate pointwise, so
/// `distance = rank(a) − rank(b)`, `lag(a, b) = 0`, and
/// `lag(b, a) = distance`.
fn distance_dense_suffix_run(p: usize) -> QueryRun {
    let a = Shape::DenseSuffix.build2(p, p).version();
    let b = Shape::DenseSuffixMate.build2(p, p).version();
    let bytes = (a.encode().len() + b.encode().len()) as u64;
    let gap = a
        .rank()
        .checked_sub(&b.rank())
        .expect("the dense-suffix operand dominates its unit mate");
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
/// triple on the dense-suffix pair, measured ×1.25 (the record and
/// every re-pin's movement and attribution live in the pin
/// commits).
///
/// The ceilings price the three query bodies together — three
/// sweeps' worth — flat per encoded byte across the doubling, on
/// the cluster-delegated settle; a per-arming suffix walk reads
/// quadratic here.
const DISTANCE_DENSE_SUFFIX_CEILINGS: [u64; 2] = [686_817, 1_371_800];

/// Distance and lag are flat per byte on the dense-suffix pair
/// under the declared log model, within ×1.25 across a doubling
/// and under absolute two-scale ceilings.
///
/// `pair_integral` drives the same integrator as `rank` (one shared
/// product-tree settle), so the two-operand form holds the same bound.
#[test]
fn skyline_distance_dense_suffix_is_flat_per_unit() {
    let small = distance_dense_suffix_run(DENSE_SUFFIX_SMALL);
    let large = distance_dense_suffix_run(2 * DENSE_SUFFIX_SMALL);
    assert_ceilings(
        "skyline_distance_dense_suffix",
        &small,
        &large,
        DISTANCE_DENSE_SUFFIX_CEILINGS,
    );
    assert_flat(
        "distance_dense_suffix_touches",
        "byte",
        (small.touches, small.bytes),
        (large.touches, large.bytes),
    );
}
