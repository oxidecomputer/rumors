//! Differential tests for the accumulator against an exact `IBig` oracle.
//!
//! Every stream compares the sign after every operation — the read a
//! consumer's sweeps depend on — and snapshots the full value periodically
//! through [`Accumulator::sign_magnitude`]. The deterministic streams are
//! the adversarial shapes the representation exists to survive: the
//! boundary-comb ±1 oscillation across a high carry cliff, wide teeth
//! across a higher cliff, and cancelling-prefix chains that force the sign
//! fold below the top digit.

use core::cmp::Ordering;

use dashu_int::ops::BitTest;
use dashu_int::{IBig, Sign, UBig};
use proptest::prelude::*;

use super::Accumulator;

/// One accumulator operation, oracle-applicable.
#[derive(Debug, Clone)]
enum Op {
    /// A signed machine-word delta.
    Small(i64),
    /// A wide delta with an explicit sign.
    Wide { negative: bool, value: UBig },
}

/// Apply one operation to the accumulator and the oracle in lockstep.
fn apply(acc: &mut Accumulator, oracle: &mut IBig, op: &Op) {
    match op {
        Op::Small(delta) => {
            acc.add_small(*delta);
            *oracle += *delta;
        }
        Op::Wide { negative, value } => {
            if *negative {
                acc.sub_wide(value);
                *oracle -= IBig::from(value.clone());
            } else {
                acc.add_wide(value);
                *oracle += IBig::from(value.clone());
            }
        }
    }
}

/// The oracle's sign as the accumulator reports it.
fn oracle_sign(oracle: &IBig) -> Ordering {
    if *oracle == IBig::ZERO {
        Ordering::Equal
    } else {
        match oracle.sign() {
            Sign::Negative => Ordering::Less,
            Sign::Positive => Ordering::Greater,
        }
    }
}

/// Assert the accumulator's full value equals the oracle's, sign and
/// magnitude both.
fn assert_value(acc: &Accumulator, oracle: &IBig) {
    let (sign, magnitude) = acc.sign_magnitude();
    assert_eq!(sign, oracle_sign(oracle), "sign_magnitude sign");
    // The sign was just asserted, so signing the magnitude with it makes
    // the magnitude comparison exact.
    let rebuilt = match sign {
        Ordering::Less => -IBig::from(magnitude),
        _ => IBig::from(magnitude),
    };
    assert_eq!(&rebuilt, oracle, "sign_magnitude magnitude");
}

/// A wide magnitude from little-endian 64-bit limbs.
fn from_limbs(limbs: &[u64]) -> UBig {
    let bytes: Vec<u8> = limbs.iter().flat_map(|l| l.to_le_bytes()).collect();
    UBig::from_le_bytes(&bytes)
}

/// A mixed operation stream: mostly small deltas of varying width, some
/// dense random wide deltas, and some all-ones/all-zeros "cliffy" wide
/// deltas whose application sits exactly on carry boundaries.
fn arb_op() -> impl Strategy<Value = Op> {
    prop_oneof![
        4 => (any::<i64>(), 0u32..60).prop_map(|(v, s)| Op::Small(v >> s)),
        1 => (proptest::collection::vec(any::<u64>(), 1..=6), any::<bool>()).prop_map(
            |(limbs, negative)| Op::Wide {
                negative,
                value: from_limbs(&limbs),
            }
        ),
        1 => (proptest::collection::vec(any::<bool>(), 1..=6), any::<bool>()).prop_map(
            |(mask, negative)| {
                let mut limbs: Vec<u64> =
                    mask.iter().map(|&m| if m { u64::MAX } else { 0 }).collect();
                if limbs.iter().all(|&l| l == 0) {
                    limbs[0] = 1;
                }
                Op::Wide {
                    negative,
                    value: from_limbs(&limbs),
                }
            }
        ),
    ]
}

proptest! {
    /// On random mixed small/wide streams the accumulator's sign matches an
    /// exact `IBig` oracle after every single operation, and the full
    /// value matches at periodic snapshots and at the end.
    #[test]
    fn mixed_streams_match_the_bigint_oracle(
        ops in proptest::collection::vec(arb_op(), 1..300),
    ) {
        let mut acc = Accumulator::new();
        let mut oracle = IBig::from(0);
        for (step, op) in ops.iter().enumerate() {
            apply(&mut acc, &mut oracle, op);
            prop_assert_eq!(acc.sign(), oracle_sign(&oracle), "sign at step {}", step);
            if step % 64 == 0 {
                assert_value(&acc, &oracle);
            }
        }
        assert_value(&acc, &oracle);
    }

    /// The width-dispatching entry points agree with the raw wide/small
    /// entry points against the oracle.
    ///
    /// A stream applied through `add_magnitude`/`sub_magnitude` — via the
    /// [`Magnitude`](super::Magnitude) implementation on `UBig`,
    /// word-scale and wide values both — matches the oracle at every
    /// sign and at the final value.
    #[test]
    fn magnitude_entry_points_match_the_oracle(
        ops in proptest::collection::vec(
            (any::<bool>(), proptest::collection::vec(any::<u64>(), 1..=3)),
            1..200,
        ),
    ) {
        let mut acc = Accumulator::new();
        let mut oracle = IBig::from(0);
        for (negative, limbs) in &ops {
            let value = from_limbs(limbs);
            // One to three limbs per value, so the stream exercises the
            // word-sized dispatch path and the wide one both.
            if *negative {
                acc.sub_magnitude(&value);
                oracle -= IBig::from(value);
            } else {
                acc.add_magnitude(&value);
                oracle += IBig::from(value);
            }
            prop_assert_eq!(acc.sign(), oracle_sign(&oracle));
        }
        assert_value(&acc, &oracle);
    }

    /// The shifted entry points hold `x · 2^s` exactly.
    ///
    /// A stream applied through `add_magnitude_shl` at arbitrary sub-digit and
    /// multi-digit shifts, mixed with unshifted subtractions, matches the
    /// oracle's explicitly shifted value at every sign and at the final
    /// value.
    #[test]
    fn shifted_entry_points_match_the_oracle(
        ops in proptest::collection::vec(
            (any::<bool>(), proptest::collection::vec(any::<u64>(), 1..=3), 0u64..200),
            1..200,
        ),
    ) {
        let mut acc = Accumulator::new();
        let mut oracle = IBig::from(0);
        for (negative, limbs, shift) in &ops {
            let value = from_limbs(limbs);
            if *negative {
                acc.sub_wide(&value);
                oracle -= IBig::from(value);
            } else {
                acc.add_magnitude_shl(&value, *shift);
                oracle += IBig::from(value << *shift as usize);
            }
            prop_assert_eq!(acc.sign(), oracle_sign(&oracle));
        }
        assert_value(&acc, &oracle);
    }

    /// The shifted subtraction entry points hold `−x · 2^s` exactly: a
    /// stream mixing `sub_magnitude_shl` and `sub_wide_shl` at arbitrary shifts
    /// with unshifted additions matches the oracle at every sign and at
    /// the final value.
    #[test]
    fn shifted_subtraction_entry_points_match_the_oracle(
        ops in proptest::collection::vec(
            (0u8..3, proptest::collection::vec(any::<u64>(), 1..=3), 0u64..200),
            1..200,
        ),
    ) {
        let mut acc = Accumulator::new();
        let mut oracle = IBig::from(0);
        for (arm, limbs, shift) in &ops {
            let value = from_limbs(limbs);
            match arm {
                0 => {
                    acc.sub_magnitude_shl(&value, *shift);
                    oracle -= IBig::from(value << *shift as usize);
                }
                1 => {
                    acc.sub_wide_shl(&value, *shift);
                    oracle -= IBig::from(value << *shift as usize);
                }
                _ => {
                    acc.add_wide(&value);
                    oracle += IBig::from(value);
                }
            }
            prop_assert_eq!(acc.sign(), oracle_sign(&oracle));
        }
        assert_value(&acc, &oracle);
    }

    /// Accumulator-into-accumulator merges preserve the value: building two
    /// operands from independent streams and merging one into the other at
    /// an arbitrary shift, added or subtracted, equals the oracle's
    /// `x ± y · 2^s`.
    #[test]
    fn merges_match_the_oracle(
        x_ops in proptest::collection::vec(arb_op(), 1..60),
        y_ops in proptest::collection::vec(arb_op(), 1..60),
        merge_shift in 0u64..200,
        subtract: bool,
    ) {
        let mut x = Accumulator::new();
        let mut x_oracle = IBig::from(0);
        for op in &x_ops {
            apply(&mut x, &mut x_oracle, op);
        }
        let mut y = Accumulator::new();
        let mut y_oracle = IBig::from(0);
        for op in &y_ops {
            apply(&mut y, &mut y_oracle, op);
        }
        if subtract {
            x.sub_accum_shl(&y, merge_shift);
            x_oracle -= y_oracle << merge_shift as usize;
        } else {
            x.add_accum_shl(&y, merge_shift);
            x_oracle += y_oracle << merge_shift as usize;
        }
        assert_value(&x, &x_oracle);
    }

    /// `digit_count` covers the held width: after any stream, the value's
    /// magnitude fits inside the counted digits (each digit spans 32 bits
    /// plus one lazy-zone bit of overhang).
    #[test]
    fn size_probe_covers_the_value(
        ops in proptest::collection::vec(arb_op(), 1..120),
    ) {
        let mut acc = Accumulator::new();
        let mut oracle = IBig::from(0);
        for op in &ops {
            apply(&mut acc, &mut oracle, op);
            let (_, magnitude) = acc.sign_magnitude();
            prop_assert!(
                u64::try_from(acc.digit_count()).expect("digit counts fit u64") * 32 + 33
                    >= magnitude.bit_len() as u64,
                "digit_count misses value width"
            );
        }
    }
}

/// The boundary-comb stream: a ±1 oscillation across the `2^k` carry cliff
/// stays sign-correct at every step and value-correct at the end.
///
/// This is the stream on which a normalized representation pays a full
/// k-bit carry/borrow per delta.
#[test]
fn boundary_comb_oscillation_matches_the_oracle() {
    let k = 512u32;
    let below_cliff = (UBig::from(1u8) << k as usize) - 1u8;
    let mut acc = Accumulator::new();
    let mut oracle = IBig::from(0);
    acc.add_wide(&below_cliff);
    oracle += IBig::from(below_cliff);
    for _ in 0..2_000 {
        acc.add_small(1);
        oracle += 1;
        assert_eq!(acc.sign(), Ordering::Greater, "above the cliff");
        acc.sub_small(1);
        oracle -= 1;
        assert_eq!(acc.sign(), Ordering::Greater, "back below the cliff");
    }
    assert_value(&acc, &oracle);
}

/// The wide-tooth stream: ±2^w teeth oscillating across a `2^k` cliff
/// (w far past any machine-word window) stay sign-correct at every step
/// and value-correct at the end.
#[test]
fn wide_teeth_across_the_cliff_match_the_oracle() {
    let (k, w) = (512u32, 192u32);
    let cliff = UBig::from(1u8) << k as usize;
    let tooth = UBig::from(1u8) << w as usize;
    let mut acc = Accumulator::new();
    let mut oracle = IBig::from(0);
    acc.add_wide(&cliff);
    oracle += IBig::from(cliff);
    for _ in 0..500 {
        acc.sub_wide(&tooth);
        oracle -= IBig::from(tooth.clone());
        assert_eq!(acc.sign(), Ordering::Greater, "below the cliff");
        acc.add_wide(&tooth);
        oracle += IBig::from(tooth.clone());
        assert_eq!(acc.sign(), Ordering::Greater, "back at the cliff");
    }
    assert_value(&acc, &oracle);
}

/// The cancelling-prefix chain: repeatedly dropping from `2^k` to 1 and
/// back stays sign-correct at every step and value-correct at snapshots.
///
/// Each drop builds a wide cancelling prefix that the next sign fold must
/// scan below the top digit and collapse.
#[test]
fn cancelling_prefix_chain_matches_the_oracle() {
    let k = 512u32;
    let peak = UBig::from(1u8) << k as usize;
    let drop = (UBig::from(1u8) << k as usize) - 1u8;
    let mut acc = Accumulator::new();
    let mut oracle = IBig::from(0);
    acc.add_wide(&peak);
    oracle += IBig::from(peak);
    assert_eq!(acc.sign(), Ordering::Greater);
    for cycle in 0..200 {
        acc.sub_wide(&drop);
        oracle -= IBig::from(drop.clone());
        assert_eq!(acc.sign(), Ordering::Greater, "down at 1");
        acc.add_wide(&drop);
        oracle += IBig::from(drop.clone());
        assert_eq!(acc.sign(), Ordering::Greater, "back at the peak");
        if cycle % 16 == 0 {
            assert_value(&acc, &oracle);
        }
    }
    assert_value(&acc, &oracle);
}

/// Exact wide cancellation lands on sign `Equal`, and unit nudges off zero
/// read `Less`/`Greater` — the near-zero discrimination the `|s| ≥ 3` fold
/// threshold must not blur.
#[test]
fn exact_cancellation_and_unit_nudges_read_correctly() {
    let wide = UBig::from(1u8) << 512usize;
    let mut acc = Accumulator::new();
    let mut oracle = IBig::from(0);
    acc.add_wide(&wide);
    oracle += IBig::from(wide.clone());
    acc.sub_wide(&wide);
    oracle -= IBig::from(wide);
    assert_eq!(acc.sign(), Ordering::Equal, "exact cancellation is zero");
    assert_value(&acc, &oracle);
    acc.sub_small(1);
    oracle -= 1;
    assert_eq!(acc.sign(), Ordering::Less, "one below zero");
    assert!(acc.is_negative(), "is_negative agrees with sign");
    assert_value(&acc, &oracle);
    acc.add_small(2);
    oracle += 2;
    assert_eq!(acc.sign(), Ordering::Greater, "one above zero");
    assert!(!acc.is_negative(), "is_negative agrees with sign");
    assert_value(&acc, &oracle);
}

/// Negative values convert correctly through both magnitude arms: a
/// negative with a nonzero low part (the complement path) and a negative
/// that is an exact multiple of the digit base (the untouched-zeros path).
#[test]
fn negative_magnitudes_convert_through_both_arms() {
    // Nonzero low part: −(2^192 − 5).
    let mut acc = Accumulator::new();
    let mut oracle = IBig::from(0);
    let wide = (UBig::from(1u8) << 192usize) - 5u8;
    acc.sub_wide(&wide);
    oracle -= IBig::from(wide);
    assert_eq!(acc.sign(), Ordering::Less);
    assert_value(&acc, &oracle);
    // Exact multiple of 2^32: −2^96.
    let mut acc = Accumulator::new();
    let mut oracle = IBig::from(0);
    let aligned = UBig::from(1u8) << 96usize;
    acc.sub_wide(&aligned);
    oracle -= IBig::from(aligned);
    assert_eq!(acc.sign(), Ordering::Less);
    assert_value(&acc, &oracle);
}

/// The unsigned machine-word entry points cover the full `u64` range,
/// including values past `i64::MAX`, in both directions.
#[test]
fn u64_entry_points_cover_the_full_range() {
    let mut acc = Accumulator::new();
    let mut oracle = IBig::from(0);
    acc.add_u64(u64::MAX);
    oracle += u64::MAX;
    assert_eq!(acc.sign(), Ordering::Greater);
    assert_value(&acc, &oracle);
    acc.sub_u64(u64::MAX);
    oracle -= u64::MAX;
    assert_eq!(acc.sign(), Ordering::Equal);
    assert_value(&acc, &oracle);
    acc.sub_u64(u64::MAX);
    oracle -= u64::MAX;
    assert_eq!(acc.sign(), Ordering::Less);
    assert_value(&acc, &oracle);
}

/// A redundantly spelled zero reads `is_zero() == false` until a sign
/// read collapses it: the canonical-spelling contract `is_zero` documents,
/// pinned so any change to it is deliberate.
#[test]
fn redundant_zero_reads_nonzero_until_collapsed() {
    let mut acc = Accumulator::new();
    acc.add_wide(&(UBig::from(1u8) << 32usize));
    acc.sub_small(1 << 32);
    let (sign, magnitude) = acc.sign_magnitude();
    assert_eq!((sign, magnitude), (Ordering::Equal, UBig::ZERO));
    assert!(!acc.is_zero(), "cancelling digits spell zero redundantly");
    assert_eq!(acc.sign(), Ordering::Equal);
    assert!(
        acc.is_zero(),
        "the sign read collapses to the canonical zero"
    );
}

/// A fresh accumulator (and its `Default`) holds exactly zero.
#[test]
fn new_and_default_hold_zero() {
    for mut acc in [Accumulator::new(), Accumulator::default()] {
        assert_eq!(acc.sign(), Ordering::Equal);
        let (sign, magnitude) = acc.sign_magnitude();
        assert_eq!(sign, Ordering::Equal);
        assert_eq!(magnitude, UBig::from(0u8));
    }
}

proptest! {
    /// The fold primitives match the oracle.
    ///
    /// `add_accum` and `sub_accum` fold one accumulator's held value
    /// into another at any interleaving, `negate` flips the value
    /// exactly, and `reset` returns to exact zero — all with the sign
    /// readable afterward.
    #[test]
    fn fold_primitives_match_the_oracle(
        x_ops in proptest::collection::vec(arb_op(), 1..60),
        y_ops in proptest::collection::vec(arb_op(), 1..60),
        subtract: bool,
        flip: bool,
    ) {
        let mut x = Accumulator::new();
        let mut x_oracle = IBig::from(0);
        for op in &x_ops {
            apply(&mut x, &mut x_oracle, op);
        }
        let mut y = Accumulator::new();
        let mut y_oracle = IBig::from(0);
        for op in &y_ops {
            apply(&mut y, &mut y_oracle, op);
        }
        if flip {
            y.negate();
            y_oracle = -y_oracle;
            assert_value(&y, &y_oracle);
        }
        if subtract {
            x.sub_accum(&y);
            x_oracle -= &y_oracle;
        } else {
            x.add_accum(&y);
            x_oracle += &y_oracle;
        }
        assert_value(&x, &x_oracle);
        assert_value(&y, &y_oracle);
        x.reset();
        assert_eq!(x.sign(), Ordering::Equal);
        assert_value(&x, &IBig::from(0));
    }

    /// `is_zero` is one-sided and a sign read canonicalizes.
    ///
    /// After any stream, `is_zero() == true` implies the value is zero,
    /// and whenever the value is zero a `sign` read collapses the
    /// spelling so `is_zero` reads true afterward.
    #[test]
    fn is_zero_is_sound_and_sign_canonicalizes(
        ops in proptest::collection::vec(arb_op(), 1..120),
    ) {
        let mut acc = Accumulator::new();
        let mut oracle = IBig::from(0);
        for op in &ops {
            apply(&mut acc, &mut oracle, op);
            if acc.is_zero() {
                prop_assert_eq!(&oracle, &IBig::ZERO);
            }
            if acc.sign() == Ordering::Equal {
                prop_assert_eq!(&oracle, &IBig::ZERO);
                prop_assert!(acc.is_zero(), "a sign read canonicalizes zero");
            }
        }
    }

    /// `shl` scales in place exactly: after any stream, shifting the held
    /// value by an arbitrary amount matches the oracle's `x · 2^s`.
    #[test]
    fn in_place_shift_matches_the_oracle(
        ops in proptest::collection::vec(arb_op(), 1..60),
        shift in 0u64..200,
    ) {
        let mut acc = Accumulator::new();
        let mut oracle = IBig::from(0);
        for op in &ops {
            apply(&mut acc, &mut oracle, op);
        }
        acc.shl(shift);
        oracle <<= shift as usize;
        assert_value(&acc, &oracle);
    }

    /// `merge_into_wider` preserves the sum regardless of which operand is
    /// wider, and hands back a drained buffer whose stale digits never
    /// leak into the result.
    #[test]
    fn width_ordered_merges_match_the_oracle(
        x_ops in proptest::collection::vec(arb_op(), 1..60),
        y_ops in proptest::collection::vec(arb_op(), 1..60),
    ) {
        let mut x = Accumulator::new();
        let mut x_oracle = IBig::from(0);
        for op in &x_ops {
            apply(&mut x, &mut x_oracle, op);
        }
        let mut y = Accumulator::new();
        let mut y_oracle = IBig::from(0);
        for op in &y_ops {
            apply(&mut y, &mut y_oracle, op);
        }
        let drained = x.merge_into_wider(y);
        x_oracle += &y_oracle;
        assert_value(&x, &x_oracle);
        // The pool contract: a drained buffer re-arms to a clean zero.
        let mut reused = drained;
        reused.reset();
        assert_value(&reused, &IBig::from(0));
    }

    /// `sign_dominates_at` never lies at any floor: the sign matches the
    /// oracle's, and a `decided` verdict implies no operand fitting in
    /// digits `0..=floor` could flip the sign when folded in.
    #[test]
    fn floor_domination_is_sound(
        ops in proptest::collection::vec(arb_op(), 1..60),
        floor in 0usize..8,
        probe_limbs in proptest::collection::vec(any::<u64>(), 1..=4),
        probe_negative: bool,
    ) {
        let mut acc = Accumulator::new();
        let mut oracle = IBig::from(0);
        for op in &ops {
            apply(&mut acc, &mut oracle, op);
        }
        let (sign, decided) = acc.sign_dominates_at(floor);
        prop_assert_eq!(sign, oracle_sign(&oracle));
        assert_value(&acc, &oracle);
        if decided {
            // The largest operand the verdict covers: top digit index at
            // most `floor`, so at most 32·(floor + 1) bits.
            let mut probe = from_limbs(&probe_limbs);
            let cap = 32 * (floor + 1);
            probe &= (UBig::from(1u8) << cap) - 1u8;
            let mut folded = oracle.clone();
            if probe_negative {
                folded -= IBig::from(probe);
            } else {
                folded += IBig::from(probe);
            }
            prop_assert_eq!(
                oracle_sign(&folded), sign,
                "a decided verdict survives any fold under its floor"
            );
        }
    }

    /// `sign_dominates_word` never lies: the sign always matches the
    /// oracle's, and a `decided` verdict implies the held magnitude
    /// exceeds any machine word — folding any `u64` afterward cannot
    /// flip the sign.
    #[test]
    fn word_domination_is_sound(
        ops in proptest::collection::vec(arb_op(), 1..60),
        probe: u64,
        probe_negative: bool,
    ) {
        let mut acc = Accumulator::new();
        let mut oracle = IBig::from(0);
        for op in &ops {
            apply(&mut acc, &mut oracle, op);
        }
        let (sign, decided) = acc.sign_dominates_word();
        prop_assert_eq!(sign, oracle_sign(&oracle));
        assert_value(&acc, &oracle);
        if decided {
            let mut folded = oracle.clone();
            if probe_negative {
                folded -= probe;
            } else {
                folded += probe;
            }
            prop_assert_eq!(
                oracle_sign(&folded), sign,
                "a decided verdict survives any word-scale fold"
            );
        }
    }
}
