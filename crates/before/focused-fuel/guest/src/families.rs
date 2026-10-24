//! Probes of the measure folds, comparisons, and ticks over committed
//! adversarial version families, at the knobs the amplification board does
//! not scale.
//!
//! Every probe checks its answer outside the measured window. Most check an
//! independent value: a family's closed-form `min_ticks`, which proves the
//! generator builds the tree the claim reasons about, or the relations its
//! pair queries must satisfy. The wide-tooth rank, the same-sign train rank,
//! and the wide-base and wide-ladder probes check only that the measured call
//! agrees with the same operation's public answer taken before the window.

use std::cmp::Ordering;

use before::testing::meter::registry::Shape;
use before::testing::meter::{version, FREEZE_DRIFT_BITS};
use before::{Count, Party, Rank, Version};
use num_bigint::BigUint;

use crate::{measured, measured_per};

/// Converts an exact expected value into a tick count, by doubling and
/// adding its 64-bit digits from the most significant down.
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

/// Builds `2^exponent` as a tick count, by doubling.
fn power_of_two(exponent: usize) -> Count {
    let mut count = Count::from(1u8);
    for _ in 0..exponent {
        count += count.clone();
    }
    count
}

/// The encoded bytes of `versions`.
fn bytes(versions: &[&Version]) -> u64 {
    versions.iter().map(|v| v.encode().len() as u64).sum()
}

/// Asserts that `version`'s minimum tick count is `expected`.
fn assert_min_ticks(version: &Version, expected: &BigUint) {
    assert_eq!(
        version.min_ticks(),
        count_of(expected),
        "the family's closed form disagrees with min_ticks: the generator does not build \
         the tree the claim reasons about"
    );
}

/// Measures `partial_cmp` of the comb `C(scale, scale)` against the empty
/// version, in units of the comb's delta codes.
pub fn cliff_compare_empty(scale: usize) -> u64 {
    let comb = Shape::CliffComb.build2(scale, scale).version();
    let empty = Version::new();
    measured_per(
        2 * scale as u64,
        || comb.partial_cmp(&empty),
        |order| assert_eq!(order, Some(Ordering::Greater), "the comb dominates empty"),
    )
}

/// Measures the join emission of the comb `C(scale, scale)` with a one-tick
/// version, in units of the comb's delta codes.
pub fn cliff_join_one_tick(scale: usize) -> u64 {
    let comb = Shape::CliffComb.build2(scale, scale).version();
    let mut one = Version::new();
    Party::seed().tick(&mut one);
    let expected = &comb | &one;
    measured_per(
        2 * scale as u64,
        || version::join(&comb, &one),
        |join| assert_eq!(join, expected, "the emitted join is the public join"),
    )
}

/// Measures `rank` of the wide-tooth comb `W(9600 · scale, bits, 128 ·
/// scale)`, in units of its encoded bytes.
pub fn rank_wide_tooth(scale: usize, bits: usize) -> u64 {
    let comb = Shape::WideToothComb
        .build3(9_600 * scale, bits, 128 * scale)
        .version();
    let expected = comb.rank();
    measured_per(
        bytes(&[&comb]),
        || version::rank(&comb),
        |rank| assert_eq!(rank, expected, "the fold's rank is the public rank"),
    )
}

/// Measures `distance` and both `lag`s of `a` against `b`, which `a`
/// dominates pointwise, checking them against the rank gap.
fn dominating_pair(a: Version, b: Version) -> u64 {
    let gap = a
        .rank()
        .checked_sub(&b.rank())
        .expect("the first operand dominates its mate");
    measured_per(
        bytes(&[&a, &b]),
        || (a.distance(&b), a.lag(&b), b.lag(&a)),
        |(distance, forward, backward)| {
            assert_eq!(distance, gap, "distance is the dominating rank gap");
            assert_eq!(forward, Rank::ZERO, "the dominating side lags by nothing");
            assert_eq!(
                backward, distance,
                "the dominated side lags by the whole gap"
            );
        },
    )
}

/// Builds the freeze-position family's descending unit spine: leaf `i` of
/// `2k + 1` raised by `2k − i`, through public party and version operations.
fn freeze_position_mate(k: usize) -> Version {
    let mut tail = Party::seed();
    let mut leaves = Vec::with_capacity(2 * k + 1);
    for _ in 0..2 * k {
        let right = tail.fork();
        leaves.push(tail);
        tail = right;
    }
    leaves.push(tail);
    Version::new().join_all(leaves.into_iter().zip((0..=2 * k).rev()).filter_map(
        |(party, ticks)| {
            (ticks > 0).then(|| {
                let mut version = Version::new();
                version.ticks(&party, ticks as u64);
                version
            })
        },
    ))
}

/// Measures the pair queries of `FP(k)` against its descending unit spine.
pub fn distance_freeze_position(k: usize) -> u64 {
    dominating_pair(
        Shape::FreezePosition.build1(k).version(),
        freeze_position_mate(k),
    )
}

/// Measures the pair queries of `PR(p)` against its unit-climb mate.
pub fn distance_promotion_rearm(p: usize) -> u64 {
    dominating_pair(
        Shape::PromotionRearm.build1(p).version(),
        Shape::PromotionRearmMate.build1(p).version(),
    )
}

/// Measures `min_ticks` of `version`, checking it against `expected`, in
/// units of its encoded bytes.
fn min_ticks(version: Version, expected: &BigUint) -> u64 {
    let expected = count_of(expected);
    measured_per(
        bytes(&[&version]),
        || version.min_ticks(),
        |ticks| assert_eq!(ticks, expected, "the family's closed-form min_ticks"),
    )
}

/// `5 · 2^(32·(r − 1))`, the seam families' residue top: a decision-bound
/// top digit 5 at position `r − 1`.
fn residue_top(r: usize) -> BigUint {
    BigUint::from(5u8) << (32 * (r - 1))
}

/// Measures `min_ticks` of the seam plunge `SP(k, r)`, whose closed form is
/// `(k + 1)` ascending leaves over all-zero minima plus the plunge.
pub fn seam_plunge(k: usize, r: usize) -> u64 {
    let expected = residue_top(r) * BigUint::from((k + 1) as u64)
        + (BigUint::from(5u8) << 64usize) * BigUint::from(((k + 1) * (k + 2) / 2) as u64);
    min_ticks(Shape::SeamPlunge.build2(k, r).version(), &expected)
}

/// Measures `min_ticks` of the seam plunge's leveled control `SPC(k, r)`:
/// the ascent on the bases, the terminal one rung up.
pub fn seam_plunge_control(k: usize, r: usize) -> u64 {
    let expected = residue_top(r) + (BigUint::from(5u8) << 64usize) * BigUint::from((k + 2) as u64);
    min_ticks(Shape::SeamPlungeControl.build2(k, r).version(), &expected)
}

/// The seam stop pair's shared closed form; the control differs only in
/// zero-base wrapping.
fn seam_stop_ticks(k: usize) -> BigUint {
    (BigUint::from(5u8) << 128usize)
        + (BigUint::from((k - 1) as u64) << 80usize)
        + (BigUint::from(5u8) << 64usize) * BigUint::from((k * (k - 1) / 2) as u64)
}

/// Measures `min_ticks` of the seam stop `SS(k)`.
pub fn seam_stop(k: usize) -> u64 {
    min_ticks(Shape::SeamStop.build1(k).version(), &seam_stop_ticks(k))
}

/// Measures `min_ticks` of the seam stop's control `SSC(k)`.
pub fn seam_stop_control(k: usize) -> u64 {
    min_ticks(
        Shape::SeamStopControl.build1(k).version(),
        &seam_stop_ticks(k),
    )
}

/// Measures `min_ticks` of the latent ladder `LL(w, k)`: the parked pair
/// over the floor plus `k` ladder leaves one to `k` under the anchor.
pub fn latent_ladder(w: usize, k: usize) -> u64 {
    let expected = residue_top(w) * BigUint::from((k + 1) as u64) + BigUint::from(1u8)
        - BigUint::from((k * (k + 1) / 2) as u64);
    min_ticks(Shape::LatentLadder.build2(w, k).version(), &expected)
}

/// The arming width, in digits, of the arming trains.
const TRAIN_WIDTH: usize = 50;

/// The window gaps per block of the arming trains, dense enough that the
/// settle's aggregate products dominate the fold's linear work per byte.
const TRAIN_GAPS: usize = 100;

/// Builds the arming train of `blocks` blocks, in the same-sign schedule or
/// the alternating one, checking its closed-form `min_ticks`.
fn arming_train(blocks: usize, alternate: bool) -> Version {
    let train = Shape::ArmingTrain
        .build_train(blocks, TRAIN_WIDTH, TRAIN_GAPS, alternate)
        .version();
    let band = 32 * TRAIN_WIDTH + (usize::BITS - blocks.leading_zeros()) as usize + 2;
    let arm = BigUint::from(1u8) << (32 * TRAIN_WIDTH);
    let kicker = BigUint::from(1u8) << FREEZE_DRIFT_BITS;
    let mut plateau = (BigUint::from(1u8) << band) + (&arm << 1);
    let mut expected = BigUint::ZERO;
    for block in 0..blocks {
        expected += &plateau * BigUint::from(TRAIN_GAPS as u64);
        if alternate && block % 2 == 1 {
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
    assert_min_ticks(&train, &expected);
    train
}

/// Measures the pair queries of `PP(400 · scale, 400 · scale)` against the
/// same-sign arming train of `8 · scale` blocks, checking that the directed
/// halves sum to the distance.
pub fn pair_plateau_train(scale: usize) -> u64 {
    let plateau = Shape::PlateauPuncture
        .build2(400 * scale, 400 * scale)
        .version();
    let train = arming_train(8 * scale, false);
    measured_per(
        bytes(&[&plateau, &train]),
        || {
            (
                plateau.distance(&train),
                plateau.lag(&train),
                train.lag(&plateau),
            )
        },
        |(distance, forward, backward)| {
            assert_eq!(
                forward + backward,
                distance,
                "the directed halves sum to the distance"
            );
        },
    )
}

/// Measures `rank` of the same-sign arming train of `blocks` blocks.
pub fn same_sign_train_rank(blocks: usize) -> u64 {
    let train = arming_train(blocks, false);
    let expected = train.rank();
    measured_per(
        bytes(&[&train]),
        || train.rank(),
        |rank| assert_eq!(rank, expected, "rank is deterministic"),
    )
}

/// `10^w` as a tick count.
fn ten_to_the(w: usize) -> Count {
    let mut count = Count::from(1u8);
    for _ in 0..w {
        let twice = &count + &count;
        let four = &twice + &twice;
        let eight = &four + &four;
        count = &eight + &twice;
    }
    count
}

/// Splits `seed` into `n` parties tiling its space, by repeated fork.
fn fork_parties(seed: Party, n: usize) -> Vec<Party> {
    let mut parties = vec![seed];
    while parties.len() < n {
        let mut next = Vec::with_capacity(parties.len() * 2);
        for mut party in parties {
            let other = party.fork();
            next.push(party);
            next.push(other);
        }
        parties = next;
    }
    parties
}

/// Builds the wide-base tiny-tail version `WT(n, w)`: one raise by `10^w`
/// over the whole space while the seed still owns it, then `n` single ticks
/// on alternating forked leaves.
fn wide_base_tiny_tail(n: usize, w: usize) -> Version {
    let mut version = Version::new();
    let seed = Party::seed();
    version.ticks(&seed, ten_to_the(w));
    for party in fork_parties(seed, n).iter().step_by(2) {
        version.tick(party);
    }
    version
}

/// Builds the wide ladder `WL(n, w)`: `n` forked parties, party `i` raised by
/// `10^w + i`.
fn wide_ladder(n: usize, w: usize) -> Version {
    let base = ten_to_the(w);
    let mut version = Version::new();
    let mut bump = Count::ZERO;
    for party in &fork_parties(Party::seed(), n) {
        version.ticks(party, &base + &bump);
        bump = &bump + &Count::from(1u8);
    }
    version
}

/// Measures `min_ticks` of `version`, checking it against the public
/// reading taken before the window, in units of its encoded bytes.
fn measure_min_ticks(version: Version) -> u64 {
    let expected = version.min_ticks();
    measured_per(
        bytes(&[&version]),
        || version.min_ticks(),
        |ticks| assert_eq!(ticks, expected, "min_ticks is deterministic"),
    )
}

/// Measures `rank` of `version`, checking it against the public reading
/// taken before the window, in units of its encoded bytes.
fn measure_rank(version: Version) -> u64 {
    let expected = version.rank();
    measured_per(
        bytes(&[&version]),
        || version.rank(),
        |rank| assert_eq!(rank, expected, "rank is deterministic"),
    )
}

/// Measures `min_ticks` of `WT(n, w)`.
pub fn wide_base_tiny_tail_min_ticks(n: usize, w: usize) -> u64 {
    measure_min_ticks(wide_base_tiny_tail(n, w))
}

/// Measures `rank` of `WT(n, w)`.
pub fn wide_base_tiny_tail_rank(n: usize, w: usize) -> u64 {
    measure_rank(wide_base_tiny_tail(n, w))
}

/// Measures `min_ticks` of `WL(n, w)`.
pub fn wide_ladder_min_ticks(n: usize, w: usize) -> u64 {
    measure_min_ticks(wide_ladder(n, w))
}

/// Measures `rank` of `WL(n, w)`.
pub fn wide_ladder_rank(n: usize, w: usize) -> u64 {
    measure_rank(wide_ladder(n, w))
}

/// The dense spine's depth for the dense tick probe.
const DENSE_DEPTH: usize = 125_000;

/// The magnitude and depth of the wide tick operands.
const TICK_CROSS_SCALE: usize = 4_000;

/// Measures `ticks(2^exponent)` on `version` through `party`; after one
/// public tick first if `grow` is set, checking then that the minimum tick
/// count grows by exactly the count.
fn ticks(version: Version, party: Party, exponent: usize, grow: bool) -> u64 {
    let count = power_of_two(exponent);
    let mut version = version;
    if grow {
        party.tick(&mut version);
    }
    let before = version.min_ticks();
    let after = grow.then(|| before.clone() + count.clone());
    measured(
        || {
            party.ticks(&mut version, count);
            version
        },
        |version| {
            if let Some(after) = after {
                assert_eq!(
                    version.min_ticks(),
                    after,
                    "a grow-branch tick adds exactly n"
                );
            } else {
                assert!(version.min_ticks() > before, "ticks raises the version");
            }
        },
    )
}

/// Decodes a generated party.
fn party_of(encoded: &before::testing::meter::Encoding) -> Party {
    Party::decode(&encoded.bytes[..]).expect("the generated party is canonical")
}

/// Measures `ticks(2^exponent)` on the dense spine through the seed.
pub fn ticks_dense(exponent: usize, grow: bool) -> u64 {
    ticks(
        Shape::Dense.build1(DENSE_DEPTH).version(),
        Party::seed(),
        exponent,
        grow,
    )
}

/// Measures `ticks(2^exponent)` on the big-root version through the
/// nested-full id.
pub fn ticks_nested_wide(exponent: usize, grow: bool) -> u64 {
    ticks(
        Shape::Bigroot
            .build2(TICK_CROSS_SCALE, TICK_CROSS_SCALE)
            .version(),
        party_of(&Shape::NestedFullId.build1(TICK_CROSS_SCALE)),
        exponent,
        grow,
    )
}

/// Measures `ticks(2^exponent)` on the wide-tail version through the
/// mirror nested-full id.
pub fn ticks_mirror_wide(exponent: usize, grow: bool) -> u64 {
    ticks(
        Shape::WideTail
            .build2(TICK_CROSS_SCALE, TICK_CROSS_SCALE)
            .version(),
        party_of(&Shape::NestedLeftFullId.build1(TICK_CROSS_SCALE)),
        exponent,
        grow,
    )
}
