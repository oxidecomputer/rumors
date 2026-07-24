//! Differential tests for the accumulator against an exact `BigInt` oracle.
//!
//! Every stream compares the sign after every operation — the read the
//! sweeps depend on — and snapshots the full value periodically through
//! [`Accum::sign_magnitude`]. The deterministic streams are the adversarial
//! shapes the representation exists to survive: the boundary-comb ±1
//! oscillation across a high carry cliff, wide teeth across a higher cliff,
//! and cancelling-prefix chains that force the sign fold below the top
//! digit.

use core::cmp::Ordering;

use num_bigint::{BigInt, BigUint, Sign};
use proptest::prelude::*;

use super::Accum;
use crate::codec::Base;

/// One accumulator operation, oracle-applicable.
#[derive(Debug, Clone)]
enum Op {
    /// A signed machine-word delta.
    Small(i64),
    /// A wide delta with an explicit sign.
    Wide { negative: bool, value: BigUint },
}

/// Apply one operation to the accumulator and the oracle in lockstep.
fn apply(acc: &mut Accum, oracle: &mut BigInt, op: &Op) {
    match op {
        Op::Small(delta) => {
            acc.add_small(*delta);
            *oracle += *delta;
        }
        Op::Wide { negative, value } => {
            if *negative {
                acc.sub_wide(value);
                *oracle -= BigInt::from(value.clone());
            } else {
                acc.add_wide(value);
                *oracle += BigInt::from(value.clone());
            }
        }
    }
}

/// The oracle's sign as the accumulator reports it.
fn oracle_sign(oracle: &BigInt) -> Ordering {
    match oracle.sign() {
        Sign::Minus => Ordering::Less,
        Sign::NoSign => Ordering::Equal,
        Sign::Plus => Ordering::Greater,
    }
}

/// Assert the accumulator's full value equals the oracle's, sign and
/// magnitude both.
fn assert_value(acc: &Accum, oracle: &BigInt) {
    let (sign, magnitude) = acc.sign_magnitude();
    assert_eq!(sign, oracle_sign(oracle), "sign_magnitude sign");
    assert_eq!(&magnitude, oracle.magnitude(), "sign_magnitude magnitude");
}

/// A wide magnitude from little-endian 64-bit limbs.
fn from_limbs(limbs: &[u64]) -> BigUint {
    let bytes: Vec<u8> = limbs.iter().flat_map(|l| l.to_le_bytes()).collect();
    BigUint::from_bytes_le(&bytes)
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
    /// exact `BigInt` oracle after every single operation, and the full
    /// value matches at periodic snapshots and at the end.
    #[test]
    fn mixed_streams_match_the_bigint_oracle(
        ops in proptest::collection::vec(arb_op(), 1..300),
    ) {
        let mut acc = Accum::new();
        let mut oracle = BigInt::from(0);
        for (step, op) in ops.iter().enumerate() {
            apply(&mut acc, &mut oracle, op);
            prop_assert_eq!(acc.sign(), oracle_sign(&oracle), "sign at step {}", step);
            if step % 64 == 0 {
                assert_value(&acc, &oracle);
            }
        }
        assert_value(&acc, &oracle);
    }

    /// The `Base` entry points agree with the raw wide/small entry points:
    /// a stream applied through `add_base`/`sub_base` (spilled and inline
    /// magnitudes both) matches the oracle at every sign and at the final
    /// value.
    #[test]
    fn base_entry_points_match_the_oracle(
        ops in proptest::collection::vec(
            (any::<bool>(), proptest::collection::vec(any::<u64>(), 1..=3)),
            1..200,
        ),
    ) {
        let mut acc = Accum::new();
        let mut oracle = BigInt::from(0);
        for (negative, limbs) in &ops {
            let value = from_limbs(limbs);
            // `Base::from` keeps inline-range values in the `Small` arm, so
            // the stream exercises both match arms.
            let base = Base::from(value.clone());
            if *negative {
                acc.sub_base(&base);
                oracle -= BigInt::from(value);
            } else {
                acc.add_base(&base);
                oracle += BigInt::from(value);
            }
            prop_assert_eq!(acc.sign(), oracle_sign(&oracle));
        }
        assert_value(&acc, &oracle);
    }

    /// The shifted entry points hold `x · 2^s` exactly: a stream applied
    /// through `add_base_shl` at arbitrary sub-digit and multi-digit
    /// shifts, mixed with unshifted subtractions, matches the oracle's
    /// explicitly shifted value at every sign and at the final value.
    #[test]
    fn shifted_entry_points_match_the_oracle(
        ops in proptest::collection::vec(
            (any::<bool>(), proptest::collection::vec(any::<u64>(), 1..=3), 0u64..200),
            1..200,
        ),
    ) {
        let mut acc = Accum::new();
        let mut oracle = BigInt::from(0);
        for (negative, limbs, shift) in &ops {
            let value = from_limbs(limbs);
            if *negative {
                acc.sub_wide(&value);
                oracle -= BigInt::from(value);
            } else {
                let base = Base::from(value.clone());
                acc.add_base_shl(&base, *shift);
                oracle += BigInt::from(value << shift);
            }
            prop_assert_eq!(acc.sign(), oracle_sign(&oracle));
        }
        assert_value(&acc, &oracle);
    }

    /// Accumulator-into-accumulator merges preserve the value: building two
    /// operands from independent streams and merging one into the other at
    /// an arbitrary shift equals the oracle's `x + y · 2^s`.
    #[test]
    fn merges_match_the_oracle(
        x_ops in proptest::collection::vec(arb_op(), 1..60),
        y_ops in proptest::collection::vec(arb_op(), 1..60),
        merge_shift in 0u64..200,
    ) {
        let mut x = Accum::new();
        let mut x_oracle = BigInt::from(0);
        for op in &x_ops {
            apply(&mut x, &mut x_oracle, op);
        }
        let mut y = Accum::new();
        let mut y_oracle = BigInt::from(0);
        for op in &y_ops {
            apply(&mut y, &mut y_oracle, op);
        }
        x.add_accum_shl(&y, merge_shift);
        x_oracle += y_oracle << merge_shift;
        assert_value(&x, &x_oracle);
    }

    /// `digit_count` covers the held width: after any stream, the value's
    /// magnitude fits inside the counted digits (each digit spans 32 bits
    /// plus one lazy-zone bit of overhang).
    #[test]
    fn size_probe_covers_the_value(
        ops in proptest::collection::vec(arb_op(), 1..120),
    ) {
        let mut acc = Accum::new();
        let mut oracle = BigInt::from(0);
        for op in &ops {
            apply(&mut acc, &mut oracle, op);
            let (_, magnitude) = acc.sign_magnitude();
            prop_assert!(
                u64::try_from(acc.digit_count()).expect("digit counts fit u64") * 32 + 33
                    >= magnitude.bits(),
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
    let below_cliff = (BigUint::from(1u8) << k) - 1u8;
    let mut acc = Accum::new();
    let mut oracle = BigInt::from(0);
    acc.add_wide(&below_cliff);
    oracle += BigInt::from(below_cliff);
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
    let cliff = BigUint::from(1u8) << k;
    let tooth = BigUint::from(1u8) << w;
    let mut acc = Accum::new();
    let mut oracle = BigInt::from(0);
    acc.add_wide(&cliff);
    oracle += BigInt::from(cliff);
    for _ in 0..500 {
        acc.sub_wide(&tooth);
        oracle -= BigInt::from(tooth.clone());
        assert_eq!(acc.sign(), Ordering::Greater, "below the cliff");
        acc.add_wide(&tooth);
        oracle += BigInt::from(tooth.clone());
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
    let peak = BigUint::from(1u8) << k;
    let drop = (BigUint::from(1u8) << k) - 1u8;
    let mut acc = Accum::new();
    let mut oracle = BigInt::from(0);
    acc.add_wide(&peak);
    oracle += BigInt::from(peak);
    assert_eq!(acc.sign(), Ordering::Greater);
    for cycle in 0..200 {
        acc.sub_wide(&drop);
        oracle -= BigInt::from(drop.clone());
        assert_eq!(acc.sign(), Ordering::Greater, "down at 1");
        acc.add_wide(&drop);
        oracle += BigInt::from(drop.clone());
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
    let wide = BigUint::from(1u8) << 512u32;
    let mut acc = Accum::new();
    let mut oracle = BigInt::from(0);
    acc.add_wide(&wide);
    oracle += BigInt::from(wide.clone());
    acc.sub_wide(&wide);
    oracle -= BigInt::from(wide);
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
    let mut acc = Accum::new();
    let mut oracle = BigInt::from(0);
    let wide = (BigUint::from(1u8) << 192u32) - 5u8;
    acc.sub_wide(&wide);
    oracle -= BigInt::from(wide);
    assert_eq!(acc.sign(), Ordering::Less);
    assert_value(&acc, &oracle);
    // Exact multiple of 2^32: −2^96.
    let mut acc = Accum::new();
    let mut oracle = BigInt::from(0);
    let aligned = BigUint::from(1u8) << 96u32;
    acc.sub_wide(&aligned);
    oracle -= BigInt::from(aligned);
    assert_eq!(acc.sign(), Ordering::Less);
    assert_value(&acc, &oracle);
}

/// The unsigned machine-word entry points cover the full `u64` range,
/// including values past `i64::MAX`, in both directions.
#[test]
fn u64_entry_points_cover_the_full_range() {
    let mut acc = Accum::new();
    let mut oracle = BigInt::from(0);
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

/// A fresh accumulator (and its `Default`) holds exactly zero.
#[test]
fn new_and_default_hold_zero() {
    for mut acc in [Accum::new(), Accum::default()] {
        assert_eq!(acc.sign(), Ordering::Equal);
        let (sign, magnitude) = acc.sign_magnitude();
        assert_eq!(sign, Ordering::Equal);
        assert_eq!(magnitude, BigUint::from(0u8));
    }
}
