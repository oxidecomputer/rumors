//! Closed-form answers of the committed adversarial version families.
//!
//! The amplification board and the `focused-fuel` claims judge each
//! operation's cost on these families, and a judgment is only as good as the
//! shape it measures. These checks hold each generator to the tree its cost
//! claims reason about: a family's `min_ticks` against its closed form, and a
//! paired family's queries against the relations they must satisfy, each at
//! two sizes.

use std::cmp::Ordering;

use before::testing::meter::registry::Shape;
use before::testing::meter::{FREEZE_DRIFT_BITS, PROMOTION_REARM_ARM_BITS};
use before::{Count, Party, Rank, Version};
use num_bigint::BigUint;

/// Converts an exact expected value into a tick count, by doubling and
/// adding its 64-bit digits from the most significant down.
///
/// `Count` exposes no big-integer backend, so the conversion goes through
/// its public arithmetic.
fn count_of(value: &BigUint) -> Count {
    let mut digits = value.iter_u64_digits().rev();
    let Some(first) = digits.next() else {
        return Count::ZERO;
    };
    let mut count = Count::from(first);
    for digit in digits {
        for _ in 0..64 {
            count += count.clone();
        }
        count += Count::from(digit);
    }
    count
}

/// Asserts that `version`, the family `name` at one size, has the minimum
/// tick count `expected`.
fn assert_min_ticks(name: &str, version: &Version, expected: &BigUint) {
    assert_eq!(
        version.min_ticks(),
        count_of(expected),
        "{name}: min_ticks disagrees with the family's closed form, so the generator does \
         not build the tree its cost claims reason about"
    );
}

/// The bit length of `value`.
fn bit_length(value: usize) -> usize {
    (usize::BITS - value.leading_zeros()) as usize
}

/// The freeze-position family `FP(k)` holds `2k` wide leaves at ever-deeper
/// freeze positions: its minimum tick count is
/// `2k · 2^band + k(k − 1)(2^P + 1) + k`, where `P` is the freeze drift and
/// `band` is `P + 1 + bits(k)`.
#[test]
fn freeze_position_matches_its_closed_form() {
    for k in [1_000, 2_000] {
        let band = FREEZE_DRIFT_BITS + 1 + bit_length(k);
        let expected = (BigUint::from(2 * k as u64) << band)
            + BigUint::from((k * (k - 1)) as u64)
                * ((BigUint::from(1u8) << FREEZE_DRIFT_BITS) + 1u8)
            + BigUint::from(k as u64);
        assert_min_ticks(
            &format!("FP({k})"),
            &Shape::FreezePosition.build1(k).version(),
            &expected,
        );
    }
}

/// The promotion re-arm spine `PR(p)` creates one deferral per block: its
/// minimum tick count is `16p + p(2^A + 2^P + 2) + 1`, where `A` is the arm
/// width and `P` the freeze drift.
#[test]
fn promotion_rearm_matches_its_closed_form() {
    for p in [1_000, 2_000] {
        let expected = BigUint::from(16 * p as u64)
            + BigUint::from(p as u64)
                * ((BigUint::from(1u8) << PROMOTION_REARM_ARM_BITS)
                    + (BigUint::from(1u8) << FREEZE_DRIFT_BITS)
                    + 2u8)
            + 1u8;
        assert_min_ticks(
            &format!("PR({p})"),
            &Shape::PromotionRearm.build1(p).version(),
            &expected,
        );
    }
}

/// The lone-freeze spine `LF(pre, post)` runs `pre` oscillation pairs before
/// its one freeze and `post` after: its minimum tick count is
/// `pre(2^P + 2) + pre/2 + 3post/2 + 3`, along both axes.
#[test]
fn lone_freeze_matches_its_closed_form() {
    for (pre, post) in [(2_000, 2), (4_000, 2), (2, 2_000), (2, 4_000)] {
        let expected = BigUint::from(pre as u64)
            * ((BigUint::from(1u8) << FREEZE_DRIFT_BITS) + 2u8)
            + BigUint::from((pre / 2) as u64)
            + BigUint::from((3 * post / 2) as u64)
            + 3u8;
        assert_min_ticks(
            &format!("LF({pre}, {post})"),
            &Shape::LoneFreeze.build2(pre, post).version(),
            &expected,
        );
    }
}

/// The pure comb and the reveal comb `(k, k)` hold `k` sites over one
/// `2^k`-wide minimum above a zero floor: their minimum tick count is
/// `k · 2^k`.
#[test]
fn combs_match_their_closed_form() {
    for k in [1_000, 2_000] {
        let expected = (BigUint::from(1u8) << k) * BigUint::from(k as u64);
        for (name, shape) in [
            ("pure comb", Shape::PureComb),
            ("reveal comb", Shape::RevealComb),
        ] {
            assert_min_ticks(
                &format!("{name}({k}, {k})"),
                &shape.build2(k, k).version(),
                &expected,
            );
        }
    }
}

/// The arming train of `n` blocks, each arming 50 digits wide over 100
/// window gaps, in the alternating sign schedule the board's arming-train
/// family uses, has the closed-form minimum tick count of its leaf heights.
///
/// Each block adds its plateau once per window gap, moves the plateau by one
/// arming (down on every odd block, up otherwise), then adds four kicks of
/// zero, one, `2^P`, and one.
#[test]
fn alternating_arming_train_matches_its_closed_form() {
    const WIDTH: usize = 50;
    const GAPS: usize = 100;
    for blocks in [4, 8, 16] {
        let band = 32 * WIDTH + bit_length(blocks) + 2;
        let arm = BigUint::from(1u8) << (32 * WIDTH);
        let kicker = BigUint::from(1u8) << FREEZE_DRIFT_BITS;
        let mut plateau = (BigUint::from(1u8) << band) + (&arm << 1);
        let mut expected = BigUint::ZERO;
        for block in 0..blocks {
            expected += &plateau * BigUint::from(GAPS as u64);
            if block % 2 == 1 {
                plateau -= &arm;
            } else {
                plateau += &arm;
            }
            for kick in [
                BigUint::ZERO,
                BigUint::from(1u8),
                kicker.clone(),
                BigUint::from(1u8),
            ] {
                plateau += kick;
                expected += &plateau;
            }
        }
        assert_min_ticks(
            &format!("alternating arming train of {blocks} blocks"),
            &Shape::ArmingTrain
                .build_train(blocks, WIDTH, GAPS, true)
                .version(),
            &expected,
        );
    }
}

/// A one-leaf version of a `b`-bit height has that height as its minimum
/// tick count, `2^b − 1`.
#[test]
fn hugeleaf_matches_its_closed_form() {
    for bits in [32_000, 64_000] {
        assert_min_ticks(
            &format!("hugeleaf({bits})"),
            &Shape::Hugeleaf.build1(bits).version(),
            &((BigUint::from(1u8) << bits) - 1u8),
        );
    }
}

/// The weight comb `WC(n)` stores the spine's `32n − 1` unit leaves and the
/// block's `n` twos: its minimum tick count is `34n − 1`.
#[test]
fn weight_comb_matches_its_closed_form() {
    for n in [512, 1_024] {
        assert_min_ticks(
            &format!("WC({n})"),
            &Shape::WeightComb.build1(n).version(),
            &BigUint::from((34 * n - 1) as u64),
        );
    }
}

/// The freeze parade `FZ(k)` sums the spine's `64k − 1` unit leaves, the
/// block's `k` wide drops, its internal half-minima differences, and its
/// root's absolute minimum, in closed form.
#[test]
fn freeze_parade_matches_its_closed_form() {
    for k in [512, 1_024] {
        let levels = bit_length(k) - 1;
        let band = FREEZE_DRIFT_BITS + 2 + bit_length(k);
        let drift = BigUint::from(1u8) << FREEZE_DRIFT_BITS;
        let stride = &drift + 1u8;
        let expected = BigUint::from((64 * k - 1) as u64)
            + (BigUint::from(1u8) << band)
            + BigUint::from(k as u64) * &drift
            + BigUint::from((k / 2 * levels) as u64) * &stride
            - BigUint::from((k - 1) as u64) * &stride
            - &drift;
        assert_min_ticks(
            &format!("FZ({k})"),
            &Shape::FreezeParade.build1(k).version(),
            &expected,
        );
    }
}

/// The dense-suffix family `DS(p, p)` creates one deferral per block against
/// a trailing dense mass: its minimum tick count is `p + p(2^A + 2^P + 2) +
/// 1`.
///
/// It dominates its unit-block mate pointwise, so the pair's distance is
/// their rank gap, the dominating side lags by nothing, and the dominated
/// side lags by the whole distance.
#[test]
fn dense_suffix_matches_its_closed_form_and_dominates_its_mate() {
    for p in [500, 1_000] {
        let version = Shape::DenseSuffix.build2(p, p).version();
        let expected = BigUint::from(p as u64)
            + BigUint::from(p as u64)
                * ((BigUint::from(1u8) << PROMOTION_REARM_ARM_BITS)
                    + (BigUint::from(1u8) << FREEZE_DRIFT_BITS)
                    + 2u8)
            + 1u8;
        assert_min_ticks(&format!("DS({p}, {p})"), &version, &expected);
        let mate = Shape::DenseSuffixMate.build2(p, p).version();
        let gap = version
            .rank()
            .checked_sub(&mate.rank())
            .expect("the dense-suffix operand dominates its unit mate");
        let distance = version.distance(&mate);
        assert_eq!(distance, gap, "DS({p}, {p}): distance is the rank gap");
        assert_eq!(
            version.lag(&mate),
            Rank::ZERO,
            "DS({p}, {p}): the dominating side lags by nothing"
        );
        assert_eq!(
            mate.lag(&version),
            distance,
            "DS({p}, {p}): the dominated side lags by the distance"
        );
    }
}

/// The jump pair `JP(512, m, d)`'s distance is the sum of its two lags (rank
/// modularity), at two sizes, the second doubling the first's teeth and
/// digits.
#[test]
fn jump_pair_distance_is_the_sum_of_its_lags() {
    for (teeth, digits) in [(512, 64), (1_024, 128)] {
        let (a, b) = Shape::JumpPair.build_pair3(512, teeth, digits);
        let (a, b) = (a.version(), b.version());
        assert_eq!(
            a.distance(&b),
            &a.lag(&b) + &b.lag(&a),
            "JP(512, {teeth}, {digits}): distance is the sum of the lags"
        );
    }
}

/// The tooth-tail pair `TT(g, m)`'s second operand runs one tick above the
/// first everywhere but the shared terminal, so the first lies strictly
/// below it.
#[test]
fn tooth_tail_orders_strictly() {
    for (spike, boundaries) in [(64, 4_096), (128, 8_192)] {
        let (a, b) = Shape::ToothTail.build_pair(spike, boundaries);
        assert_eq!(
            a.version().partial_cmp(&b.version()),
            Some(Ordering::Less),
            "TT({spike}, {boundaries}): the first operand lies strictly below the second"
        );
    }
}

/// The mask-drift triple's comb, viewed through its mask, lies strictly
/// under the plateau, and the quadruple's sparse view lies strictly under
/// its tooth-keeping view, so neither comparison exits early.
#[test]
fn mask_drift_views_order_strictly() {
    for teeth in [1_024, 2_048] {
        let (comb, mask, plateau) = Shape::MaskDriftTriple.build_triple(512, teeth);
        let mask = Party::decode(&mask.bytes[..]).expect("the mask is canonical");
        assert_eq!(
            (&comb.version() / &mask).partial_cmp(&plateau.version()),
            Some(Ordering::Less),
            "the projected comb lies under the plateau at {teeth} teeth"
        );
        let ((sparse, even), (full, odd)) = Shape::MaskDriftQuadruple.build_quadruple(512, teeth);
        let even = Party::decode(&even.bytes[..]).expect("the mask is canonical");
        let odd = Party::decode(&odd.bytes[..]).expect("the mask is canonical");
        assert_eq!(
            (&sparse.version() / &even).partial_cmp(&(&full.version() / &odd)),
            Some(Ordering::Less),
            "the sparse view lies under the tooth-keeping view at {teeth} teeth"
        );
    }
}
