//! Flatness checks for repeated rank settlement.
//!
//! The multi-arming and pair legs of the settle's bound, held flat per
//! byte. The single-arming wide × dense cases carry their own bands
//! (`deferred_wide_arming`, `answer_embedded_product`); the probes here
//! hold the shapes only arming *count* can reach: trains of wide
//! deferrals settled through the full data-balanced reduction
//! tree, and a pair driving both settle sites through one co-sweep.
//! The tree rewrites a window's digits once per level and the mass
//! balance keeps levels logarithmic in the arming count, so the
//! settle's metered traffic per byte can grow only by the level ratio
//! across an arming-count doubling — ×log₂(2n)/log₂(n), at most ×1.17
//! from the probes' smallest count — and only if the settle dominated
//! the fold's linear work, which it does not: the ×1.25 flatness
//! convention covers the model's whole admissible growth here.

use super::min_ticks_from_big;
use before::testing::meter::registry::Shape;
use num_bigint::BigUint;
use suanpan::touch_meter;

/// One `Version::rank` run: operand bytes and accumulator touches, under
/// the one-touch-per-byte liveness floor.
fn rank_run(v: &before::Version) -> (u64, u64) {
    let bytes = v.encode().len() as u64;
    touch_meter::reset();
    let rank = v.rank();
    std::hint::black_box(rank);
    let touches = touch_meter::touches();
    assert!(
        touches >= bytes,
        "rank at {bytes} operand bytes: {touches} digit touches under \
         the one-per-byte floor: the fold's accumulator work is not \
         metered",
    );
    (bytes, touches)
}

/// Assert one probe's reading against its absolute pinned ceilings
/// and, per currency, flatness (×1.25 per byte) across the
/// doubling, and report the readings.
fn assert_flat_step(name: &str, small: (u64, u64), large: (u64, u64), ceilings: [u64; 2]) {
    let (sb, st) = small;
    let (lb, lt) = large;
    eprintln!(
        "MEASURED settle_flatness_{name}: small={st}/{sb}B \
         large={lt}/{lb}B per_byte={} -> {} (milli-touches)",
        st * 1000 / sb,
        lt * 1000 / lb,
    );
    assert!(
        st <= ceilings[0] && lt <= ceilings[1],
        "{name} exceeds the pinned touch ceilings: {st}/{sb}B -> \
         {lt}/{lb}B against {} / {}",
        ceilings[0],
        ceilings[1],
    );
    assert!(
        u128::from(lt) * u128::from(sb) * 4 <= u128::from(st) * u128::from(lb) * 5,
        "{name} touches grew past the per-byte band across the doubling: \
         {st}/{sb}B -> {lt}/{lb}B"
    );
}

/// Arming width (digits) of the multi-arming probes.
const TRAIN_WIDTH: usize = 50;

/// Window gaps per block of the multi-arming probes.
///
/// Dense enough that the settle's aggregate products dominate the
/// fold's linear work per stored byte: the windows are
/// topology-funded, so `g` buys density the operand barely pays
/// for.
const TRAIN_GAPS: usize = 100;

/// One arming-train rank run with the mirrored `min_ticks` leg.
fn train_run(n: usize, alternate: bool) -> (u64, u64) {
    let v = Shape::ArmingTrain
        .build_train(n, TRAIN_WIDTH, TRAIN_GAPS, alternate)
        .version();
    let band = 32 * TRAIN_WIDTH + (usize::BITS - n.leading_zeros()) as usize + 2;
    let arm = BigUint::ONE << (32 * TRAIN_WIDTH);
    let kicker = BigUint::ONE << 288usize;
    let mut plateau = (BigUint::ONE << band) + (&arm << 1);
    let mut expected = BigUint::ZERO;
    for b in 0..n {
        expected += &plateau * BigUint::from(TRAIN_GAPS as u64);
        if alternate && b % 2 == 1 {
            plateau -= &arm;
        } else {
            plateau += &arm;
        }
        for kick in [BigUint::ZERO, BigUint::ONE, kicker.clone(), BigUint::ONE] {
            plateau += kick;
            expected += &plateau;
        }
    }
    assert_eq!(
        v.min_ticks(),
        min_ticks_from_big(&expected),
        "the family's leaf-value sum disagrees with min_ticks: the \
         generator does not build the tree these probes reason about"
    );
    rank_run(&v)
}

/// One distance-and-lag run over a version pair: combined operand
/// bytes and both counters over the three query bodies together.
///
/// Enforces the one-touch-per-byte liveness floor and the
/// halves-sum value leg (`lag(a, b) + lag(b, a) == distance`,
/// exact `Rank` arithmetic the sweeps share nothing with).
fn pair_run(a: &before::Version, b: &before::Version) -> (u64, u64) {
    let bytes = (a.encode().len() + b.encode().len()) as u64;
    touch_meter::reset();
    let d = a.distance(b);
    let forward = a.lag(b);
    let backward = b.lag(a);
    let touches = touch_meter::touches();
    assert_eq!(
        forward + backward,
        d,
        "the directed halves must sum to the symmetric distance"
    );
    assert!(
        touches >= bytes,
        "pair queries at {bytes} operand bytes: {touches} digit touches \
         under the one-per-byte floor: the co-sweep's difference state is \
         not running on the metered accumulator",
    );
    (bytes, touches)
}

/// Absolute two-scale touch ceilings for the pair probe,
/// measured ×1.25 (the record and every re-pin's movement live in
/// the pin commits).
///
/// Flat per encoded byte across the committed doubling. The
/// plateau side's close-time settle dominates this pair, so repeated
/// width-by-digit work there would exceed these ceilings.
const PAIR_PLATEAU_TRAIN_CEILINGS: [u64; 2] = [507_805, 1_019_693];

/// The plateau-puncture × arming-train pair is flat per byte
/// through the public distance and lag entry points: the
/// shared-integrator argument measured on the pair co-sweep, not
/// inferred from rank alone.
///
/// The pair drives both settle cases in one co-sweep — the
/// plateau side parks one wide drift whose final segment stays
/// dense (the close-time answer-embedded product) while the train
/// side creates deferred-height entries repeatedly (the aggregate
/// products) — so a pair-only regression in either site, or in
/// their interaction through the shared difference integrator,
/// reads here even while every rank-only probe stays green.
#[test]
fn pair_plateau_train_is_flat_per_unit() {
    let small = pair_run(
        &Shape::PlateauPuncture.build2(400, 400).version(),
        &Shape::ArmingTrain
            .build_train(8, TRAIN_WIDTH, TRAIN_GAPS, false)
            .version(),
    );
    let large = pair_run(
        &Shape::PlateauPuncture.build2(800, 800).version(),
        &Shape::ArmingTrain
            .build_train(16, TRAIN_WIDTH, TRAIN_GAPS, false)
            .version(),
    );
    assert_flat_step(
        "pair_plateau_train",
        small,
        large,
        PAIR_PLATEAU_TRAIN_CEILINGS,
    );
}

/// Absolute touch ceilings for the same-sign train at
/// n = 4, 8, 16, measured ×1.25 (the record and every re-pin's
/// movement live in the pin commits).
///
/// The tree's one-rewrite-per-level window traffic under
/// full-width parked sums stays inside the level-ratio model; a
/// schoolbook charge grows past it, rising with the count.
const TRAIN_SAME_SIGN_CEILINGS: [u64; 3] = [29_240, 60_208, 122_261];

/// Absolute touch ceilings for the alternating train at
/// n = 4, 8, 16, measured ×1.25 (the record lives in the pin
/// commit).
///
/// They sit within a few percent of the same-sign train's
/// committed ceilings: under the backend-delegated products the
/// parked sums' sign schedule moves constants only (cancellation
/// narrows a product's factor; the bound never rests on it). The
/// sign schedules' value coverage lives in the promoting
/// differential pool.
const TRAIN_ALTERNATING_CEILINGS: [u64; 3] = [29_927, 61_963, 125_532];

/// Multi-arming trains are flat per byte across two arming-count
/// doublings, in both sign schedules, under absolute pinned
/// ceilings.
///
/// The same-sign train is the tree's hardest committed probe:
/// every level holds the full window density under full-width
/// parked sums, so all of the settle's per-level window traffic
/// rides maximal-width products — and stays inside the ×1.25
/// convention because a doubling adds one level to a logarithmic
/// stack while the byte budget doubles. The alternating twin
/// cancels parked width digit-wise inside the tree's aggregate
/// sums; its committed ceilings sit within a few percent of the
/// same-sign train's — the committed record that the sign
/// schedule is a constants effect under backend-delegated
/// products, not a class effect.
#[test]
fn arming_trains_is_flat_per_unit() {
    let same = [
        train_run(4, false),
        train_run(8, false),
        train_run(16, false),
    ];
    let alt = [train_run(4, true), train_run(8, true), train_run(16, true)];
    for (name, runs, ceilings) in [
        ("train_same_sign", &same, &TRAIN_SAME_SIGN_CEILINGS),
        ("train_alternating", &alt, &TRAIN_ALTERNATING_CEILINGS),
    ] {
        assert_flat_step(name, runs[0], runs[1], [ceilings[0], ceilings[1]]);
        assert_flat_step(name, runs[1], runs[2], [ceilings[1], ceilings[2]]);
    }
}
