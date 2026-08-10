//! Differential streams against the exact `IBig` oracle.
//!
//! Randomized mixed small/wide operation streams compare the sign after
//! every operation — the read a caller's interleaved sweeps depend on —
//! and the full value at periodic snapshots; deterministic streams pin
//! the adversarial shapes the representation exists to survive: the
//! boundary-comb ±1 oscillation across a high carry cliff, wide teeth
//! across a higher cliff, and cancelling-prefix chains that force the
//! sign fold below the top digit.

use core::cmp::Ordering;

use dashu_int::ops::BitTest;
use dashu_int::{IBig, UBig};
use proptest::prelude::*;

use super::{
    assert_value, fresh, from_limbs, oracle_sign, park_extreme_negative_digit, Accumulator,
};
use crate::accumulator::QUICK_SHIFT_MAX;

/// One accumulator operation, oracle-applicable.
#[derive(Debug, Clone)]
enum Op {
    /// A signed machine-word delta.
    Small(i64),
    /// A wide delta with an explicit sign.
    Wide { negative: bool, value: UBig },
    /// A wide delta scaled by `2^shift`, entering above digit zero.
    WideShl {
        negative: bool,
        value: UBig,
        shift: u64,
    },
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
        Op::WideShl {
            negative,
            value,
            shift,
        } => {
            let scaled = IBig::from(value.clone()) << usize::try_from(*shift).unwrap();
            if *negative {
                acc.sub_wide_shl(value, *shift);
                *oracle -= scaled;
            } else {
                acc.add_wide_shl(value, *shift);
                *oracle += scaled;
            }
        }
    }
}

/// A mixed operation stream: mostly small deltas of varying width, some
/// dense random wide deltas, and some all-ones/all-zeros "cliffy" wide
/// deltas whose application sits exactly on carry boundaries.
fn arb_op() -> impl Strategy<Value = Op> {
    prop_oneof![
        4 => (any::<i64>(), 0u32..60).prop_map(|(value, narrowing)| Op::Small(value >> narrowing)),
        1 => (proptest::collection::vec(any::<u64>(), 1..=6), any::<bool>()).prop_map(
            |(limbs, negative)| Op::Wide {
                negative,
                value: from_limbs(&limbs),
            }
        ),
        1 => (proptest::collection::vec(any::<bool>(), 1..=6), any::<bool>()).prop_map(
            |(mask, negative)| {
                let mut limbs: Vec<u64> =
                    mask.iter().map(|&saturated| if saturated { u64::MAX } else { 0 }).collect();
                if limbs.iter().all(|&limb| limb == 0) {
                    limbs[0] = 1;
                }
                Op::Wide {
                    negative,
                    value: from_limbs(&limbs),
                }
            }
        ),
        1 => (
            proptest::collection::vec(any::<u64>(), 1..=4),
            any::<bool>(),
            0u64..512,
        )
            .prop_map(|(limbs, negative, shift)| Op::WideShl {
                negative,
                value: from_limbs(&limbs),
                shift,
            }),
    ]
}

/// How the domination family's held value is built.
#[derive(Debug, Clone)]
enum Held {
    /// A random mixed stream: samples the digit engine and the sign
    /// fold's decision index.
    Stream { ops: Vec<Op>, engine_first: bool },
    /// A register-parked `m · 2^(32·(floor + 1) − 31) + delta`, both
    /// signs: samples the quick branch's direct magnitude comparison,
    /// optionally spilled so the same magnitudes also run through the
    /// fold.
    Register {
        m: u64,
        delta: u64,
        negative: bool,
        spill: bool,
    },
}

/// A held value paired with the floor it is probed at.
///
/// Stream-built values draw any floor; register-built values
/// concentrate `m` (the multiplier in units of `2^31`) around the
/// quick branch's certification boundary `3 · 2^(32·(floor + 1))` and
/// the redundant-operand bound just above `2 · 2^(32·(floor + 1))`,
/// spanning multipliers 1.5..4 in between. Register floors stop at 1:
/// a wider floor puts the boundary past the register's ceiling.
fn arb_held() -> impl Strategy<Value = (Held, usize)> {
    prop_oneof![
        1 => (proptest::collection::vec(arb_op(), 1..60), any::<bool>(), 0usize..8).prop_map(
            |(ops, engine_first, floor)| (Held::Stream { ops, engine_first }, floor)
        ),
        1 => (
            prop_oneof![
                2 => Just(1u64 << 32),           // multiplier 2: the operand bound
                1 => Just(3u64 << 31),           // multiplier 3: the certification boundary
                3 => (3u64 << 30)..(1u64 << 33), // multipliers spanning 1.5..4
            ],
            prop_oneof![
                2 => Just(0u64),
                3 => 0u64..(1 << 32),
                1 => 0u64..(1 << 34),
            ],
            any::<bool>(),
            any::<bool>(),
            0usize..=1,
        )
            .prop_map(|(m, delta, negative, spill, floor)| {
                (
                    Held::Register {
                        m,
                        delta,
                        negative,
                        spill,
                    },
                    floor,
                )
            }),
    ]
}

/// Materialize a held-value spec as the accumulator and its exact
/// oracle.
fn build_held(held: &Held, floor: usize) -> (Accumulator, IBig) {
    match held {
        Held::Stream { ops, engine_first } => {
            let mut acc = fresh(*engine_first);
            let mut oracle = IBig::from(0);
            for op in ops {
                apply(&mut acc, &mut oracle, op);
            }
            (acc, oracle)
        }
        Held::Register {
            m,
            delta,
            negative,
            spill,
        } => {
            let mut acc = Accumulator::new();
            acc.add_u64(*m);
            // Shift to the floor's scale in chunks the register accepts,
            // so the value stays register-held throughout.
            let mut remaining = 32 * (floor as u64 + 1) - 31;
            while remaining > 0 {
                let step = remaining.min(QUICK_SHIFT_MAX);
                acc.shl(step);
                remaining -= step;
            }
            acc.add_u64(*delta);
            if *negative {
                acc.negate();
            }
            assert!(
                acc.quick.is_some(),
                "the register arm stays register-held until its own spill"
            );
            let mut oracle = (IBig::from(*m) << (32 * (floor + 1) - 31)) + IBig::from(*delta);
            if *negative {
                oracle = -oracle;
            }
            if *spill {
                acc.spill();
            }
            (acc, oracle)
        }
    }
}

/// One digit of an accumulator-operand probe.
#[derive(Debug, Clone, Copy)]
enum ProbeDigit {
    /// Parked at the lazy zone's edge: `−(2^33 − 1)`.
    Extreme,
    /// A single word deposit `−w`, `w < 2^32`.
    Word(u64),
    /// No deposit at this index.
    Absent,
}

/// A probe digit, biased toward the zone's edge — the spellings whose
/// magnitude exceeds any plain probe under the same floor.
fn arb_probe_digit() -> impl Strategy<Value = ProbeDigit> {
    prop_oneof![
        3 => Just(ProbeDigit::Extreme),
        2 => (1u64..(1 << 32)).prop_map(ProbeDigit::Word),
        1 => Just(ProbeDigit::Absent),
    ]
}

/// Materialize an accumulator-operand probe held in digits
/// `0..=floor`, as the accumulator and its exact oracle.
///
/// `all_extreme` overrides every digit spec to the zone's edge — the
/// extremal operand of the floor, up to `(2^33 − 1)` per digit —
/// so the strategy carries a point mass at the strongest probe.
fn build_probe(
    digits: &[ProbeDigit],
    all_extreme: bool,
    negate: bool,
    floor: usize,
) -> (Accumulator, IBig) {
    let mut probe = Accumulator::new();
    // A digit-engine spelling by construction: the deposits below must
    // land as lazy-zone digits, not fuse in the register.
    probe.spill();
    let mut oracle = IBig::from(0);
    for (index, spec) in digits[..=floor].iter().enumerate() {
        let spec = if all_extreme {
            ProbeDigit::Extreme
        } else {
            *spec
        };
        match spec {
            ProbeDigit::Extreme => {
                park_extreme_negative_digit(&mut probe, index as u64);
                oracle -= IBig::from((1u64 << 33) - 1) << (32 * index);
            }
            ProbeDigit::Word(w) => {
                probe.sub_magnitude_shl(&UBig::from(w), 32 * index as u64);
                oracle -= IBig::from(w) << (32 * index);
            }
            ProbeDigit::Absent => {}
        }
    }
    if negate {
        probe.negate();
        oracle = -oracle;
    }
    (probe, oracle)
}

proptest! {
    /// On random mixed small/wide streams the accumulator's sign matches an
    /// exact `IBig` oracle after every single operation, and the full
    /// value matches at periodic snapshots and at the end.
    #[test]
    fn mixed_streams_match_the_bigint_oracle(
        ops in proptest::collection::vec(arb_op(), 1..300),
        engine_first: bool,
    ) {
        let mut acc = fresh(engine_first);
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
    /// [`Magnitude`](crate::Magnitude) implementation on `UBig`,
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
        receiver_ops in proptest::collection::vec(arb_op(), 1..60),
        operand_ops in proptest::collection::vec(arb_op(), 1..60),
        merge_shift in 0u64..200,
        subtract: bool,
    ) {
        let mut receiver = Accumulator::new();
        let mut receiver_oracle = IBig::from(0);
        for op in &receiver_ops {
            apply(&mut receiver, &mut receiver_oracle, op);
        }
        let mut operand = Accumulator::new();
        let mut operand_oracle = IBig::from(0);
        for op in &operand_ops {
            apply(&mut operand, &mut operand_oracle, op);
        }
        if subtract {
            receiver.sub_accum_shl(&operand, merge_shift);
            receiver_oracle -= operand_oracle << merge_shift as usize;
        } else {
            receiver.add_accum_shl(&operand, merge_shift);
            receiver_oracle += operand_oracle << merge_shift as usize;
        }
        assert_value(&receiver, &receiver_oracle);
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

    /// The fold primitives match the oracle.
    ///
    /// `add_accum` and `sub_accum` fold one accumulator's held value
    /// into another at any interleaving, `negate` flips the value
    /// exactly, and `reset` returns to exact zero — all with the sign
    /// readable afterward.
    #[test]
    fn fold_primitives_match_the_oracle(
        receiver_ops in proptest::collection::vec(arb_op(), 1..60),
        operand_ops in proptest::collection::vec(arb_op(), 1..60),
        subtract: bool,
        flip: bool,
        receiver_engine: bool,
        operand_engine: bool,
    ) {
        let mut receiver = fresh(receiver_engine);
        let mut receiver_oracle = IBig::from(0);
        for op in &receiver_ops {
            apply(&mut receiver, &mut receiver_oracle, op);
        }
        let mut operand = fresh(operand_engine);
        let mut operand_oracle = IBig::from(0);
        for op in &operand_ops {
            apply(&mut operand, &mut operand_oracle, op);
        }
        if flip {
            operand.negate();
            operand_oracle = -operand_oracle;
            assert_value(&operand, &operand_oracle);
        }
        if subtract {
            receiver.sub_accum(&operand);
            receiver_oracle -= &operand_oracle;
        } else {
            receiver.add_accum(&operand);
            receiver_oracle += &operand_oracle;
        }
        assert_value(&receiver, &receiver_oracle);
        assert_value(&operand, &operand_oracle);
        receiver.reset();
        assert_eq!(receiver.sign(), Ordering::Equal);
        assert_value(&receiver, &IBig::from(0));
    }

    /// The pooled-reuse seam is exact: a reset re-arms the register, and
    /// a later stream that spills back into the retained digit buffer
    /// sees no residue of the pre-reset value.
    ///
    /// Two independent streams with a reset between them: after the
    /// reset the accumulator equals fresh zero, and the second stream —
    /// whose wide operands re-arm the digit engine over the buffer the
    /// first stream grew — matches an oracle that starts from zero.
    #[test]
    fn pooled_reuse_after_reset_matches_the_oracle(
        first in proptest::collection::vec(arb_op(), 1..60),
        second in proptest::collection::vec(arb_op(), 1..60),
    ) {
        let mut acc = Accumulator::new();
        let mut oracle = IBig::from(0);
        for op in &first {
            apply(&mut acc, &mut oracle, op);
        }
        acc.reset();
        let mut oracle = IBig::from(0);
        assert_value(&acc, &oracle);
        for (step, op) in second.iter().enumerate() {
            apply(&mut acc, &mut oracle, op);
            prop_assert_eq!(acc.sign(), oracle_sign(&oracle), "sign at step {}", step);
        }
        assert_value(&acc, &oracle);
    }

    /// `is_literally_zero` is one-sided and a sign read canonicalizes.
    ///
    /// After any stream, `is_literally_zero() == true` implies the value
    /// is zero, and whenever the value is zero a `sign` read collapses
    /// the spelling so `is_literally_zero` reads true afterward.
    #[test]
    fn is_literally_zero_is_sound_and_sign_canonicalizes(
        ops in proptest::collection::vec(arb_op(), 1..120),
        engine_first: bool,
    ) {
        let mut acc = fresh(engine_first);
        let mut oracle = IBig::from(0);
        for op in &ops {
            apply(&mut acc, &mut oracle, op);
            if acc.is_literally_zero() {
                prop_assert_eq!(&oracle, &IBig::ZERO);
            }
            if acc.sign() == Ordering::Equal {
                prop_assert_eq!(&oracle, &IBig::ZERO);
                prop_assert!(
                    acc.is_literally_zero(),
                    "a sign read canonicalizes zero"
                );
            }
        }
    }

    /// Run-forming shift streams match the exact oracle at every step.
    ///
    /// Wide-shifted writes at scales that jump far above the held top —
    /// creating, splitting, and consuming zero-run certificates on
    /// every schedule the strategy can draw — mixed with word-scale
    /// deltas and a sign read per step: the ledger never changes the
    /// value the digits denote, and the full value matches at periodic
    /// snapshots and the end.
    #[test]
    fn run_forming_shift_streams_match_the_bigint_oracle(
        ops in proptest::collection::vec(
            (0u8..4, proptest::collection::vec(any::<u64>(), 1..=2), 0u64..4_096),
            1..150,
        ),
        engine_first: bool,
    ) {
        let mut acc = fresh(engine_first);
        let mut oracle = IBig::from(0);
        for (step, (arm, limbs, shift)) in ops.iter().enumerate() {
            let value = from_limbs(limbs);
            let scaled = IBig::from(value.clone()) << usize::try_from(*shift).unwrap();
            match arm {
                0 => {
                    acc.add_wide_shl(&value, *shift);
                    oracle += scaled;
                }
                1 => {
                    acc.sub_wide_shl(&value, *shift);
                    oracle -= scaled;
                }
                2 => {
                    acc.sub_magnitude_shl(&value, *shift);
                    oracle -= scaled;
                }
                _ => {
                    let delta = limbs[0] as i64;
                    acc.add_small(delta);
                    oracle += delta;
                }
            }
            prop_assert_eq!(acc.sign(), oracle_sign(&oracle), "sign at step {}", step);
            if step % 32 == 0 {
                assert_value(&acc, &oracle);
            }
        }
        assert_value(&acc, &oracle);
    }

    /// `shl` scales in place exactly: after any stream, shifting the held
    /// value by an arbitrary amount matches the oracle's `x · 2^s`.
    #[test]
    fn in_place_shift_matches_the_oracle(
        ops in proptest::collection::vec(arb_op(), 1..60),
        shift in 0u64..200,
        engine_first: bool,
    ) {
        let mut acc = fresh(engine_first);
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
        receiver_ops in proptest::collection::vec(arb_op(), 1..60),
        operand_ops in proptest::collection::vec(arb_op(), 1..60),
        receiver_engine: bool,
        operand_engine: bool,
    ) {
        let mut receiver = fresh(receiver_engine);
        let mut receiver_oracle = IBig::from(0);
        for op in &receiver_ops {
            apply(&mut receiver, &mut receiver_oracle, op);
        }
        let mut operand = fresh(operand_engine);
        let mut operand_oracle = IBig::from(0);
        for op in &operand_ops {
            apply(&mut operand, &mut operand_oracle, op);
        }
        let drained = receiver.merge_into_wider(operand);
        receiver_oracle += &operand_oracle;
        assert_value(&receiver, &receiver_oracle);
        // The pool contract: a drained buffer re-arms to a clean zero.
        let mut reused = drained;
        reused.reset();
        assert_value(&reused, &IBig::from(0));
    }

    /// `sign_dominates_at` never lies at any floor: the sign matches
    /// the oracle's, and a `decided` verdict covers every operand in
    /// digits `0..=floor` — plain or redundantly spelled.
    ///
    /// Held values come from random streams (the digit fold's decision
    /// index) and from register-parked values concentrated at the quick
    /// branch's certification boundary (its direct magnitude
    /// comparison). Three legs per decided verdict: a plain-magnitude
    /// probe under `2^(32·(floor + 1))` folded through the oracle; an
    /// accumulator-operand probe built from lazy-zone digit deposits in
    /// digits `0..=floor` (whose redundant spelling can reach
    /// `2.01 · 2^(32·(floor + 1))`), folded in both signs with the held
    /// magnitude asserted strictly larger; and, while register-held, the
    /// documented certification bound pinned exactly —
    /// `decided ⇔ |value| ≥ 3 · 2^(32·(floor + 1))`.
    #[test]
    fn floor_domination_is_sound(
        (held, floor) in arb_held(),
        probe_limbs in proptest::collection::vec(any::<u64>(), 1..=4),
        probe_negative: bool,
        probe_digits in proptest::collection::vec(arb_probe_digit(), 8),
        probe_all_extreme: bool,
        probe_negate: bool,
    ) {
        let (mut acc, oracle) = build_held(&held, floor);
        let register_held = acc.quick.is_some();
        let (sign, decided) = acc.sign_dominates_at(floor);
        prop_assert_eq!(sign, oracle_sign(&oracle));
        assert_value(&acc, &oracle);
        if register_held {
            // The register arm's contract is exact, not merely sound:
            // the certificate is the direct magnitude comparison.
            let (_, magnitude) = acc.sign_magnitude();
            prop_assert_eq!(
                decided,
                magnitude >= UBig::from(3u8) << (32 * (floor + 1)),
                "a register-held verdict is exactly the comparison \
                 against 3 · 2^(32·(floor + 1))"
            );
        }
        if decided {
            // The largest plain operand the verdict covers: top digit
            // index at most `floor`, so at most 32·(floor + 1) bits.
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
            // The accumulator-operand clause: a redundant lazy-zone
            // spelling in digits 0..=floor can exceed every plain
            // magnitude under the floor, and the verdict covers it too.
            let (operand, operand_oracle) =
                build_probe(&probe_digits, probe_all_extreme, probe_negate, floor);
            prop_assert!(
                operand.digit_count() <= floor + 1,
                "the probe operand sits at or below the floor"
            );
            assert_value(&operand, &operand_oracle);
            let (_, held_magnitude) = acc.sign_magnitude();
            let (_, operand_magnitude) = operand.sign_magnitude();
            prop_assert!(
                held_magnitude > operand_magnitude,
                "a decided verdict implies the held magnitude strictly \
                 exceeds any operand held in digits 0..=floor"
            );
            let mut folded = acc.clone();
            folded.sub_accum(&operand);
            prop_assert_eq!(
                folded.sign(), sign,
                "a decided verdict survives subtracting an operand held \
                 in digits 0..=floor"
            );
            let mut folded = acc.clone();
            folded.add_accum(&operand);
            prop_assert_eq!(
                folded.sign(), sign,
                "a decided verdict survives adding an operand held in \
                 digits 0..=floor"
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
        engine_first: bool,
    ) {
        let mut acc = fresh(engine_first);
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

    /// Digit-aligned streams — every delta an exact multiple of 2^32 —
    /// convert exactly at every step: recenter ties leave zero
    /// remainders that the read-out's complement carry must ripple
    /// across.
    ///
    /// The generalized family behind the flush-right carry-tie witness:
    /// with the digit engine forced, deposits biased toward small
    /// multiples drive digits onto exact `±k · 2^33` boundaries (every
    /// recenter of a digit holding a multiple of `2^32` is a tie, since
    /// the recentered remainder is both a multiple of `2^32` and inside
    /// `[−2^31, 2^31)` — so exactly 0), and the zero digits they leave
    /// under nonzero highs route `sign_magnitude` through the
    /// `rem_euclid`/complement seam at random digit offsets, both
    /// signs. The value is oracle-checked after every deposit, before
    /// the sign read collapses the spelling.
    #[test]
    fn carry_tie_streams_match_the_oracle(
        ops in proptest::collection::vec(
            (
                prop_oneof![3 => 1u64..=4, 1 => 1u64..(1 << 32)],
                0u64..6,
                any::<bool>(),
            ),
            1..40,
        ),
    ) {
        let mut acc = Accumulator::new();
        // The seam lives in the digit engine: register-held values read
        // out from the register arm and never recenter.
        acc.spill();
        let mut oracle = IBig::from(0);
        for (multiple, index, negative) in &ops {
            let delta = UBig::from(multiple << 32);
            let scaled =
                IBig::from(delta.clone()) << usize::try_from(32 * index).unwrap();
            if *negative {
                acc.sub_magnitude_shl(&delta, 32 * index);
                oracle -= scaled;
            } else {
                acc.add_magnitude_shl(&delta, 32 * index);
                oracle += scaled;
            }
            // Raw spelling first: the read-out samples the complement
            // seam before the sign read's collapse rewrites the digits.
            assert_value(&acc, &oracle);
            prop_assert_eq!(acc.sign(), oracle_sign(&oracle));
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
    let cliff_bits = 512u32;
    let below_cliff = (UBig::from(1u8) << cliff_bits as usize) - 1u8;
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
    let (cliff_bits, tooth_bits) = (512u32, 192u32);
    let cliff = UBig::from(1u8) << cliff_bits as usize;
    let tooth = UBig::from(1u8) << tooth_bits as usize;
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
    let peak_bits = 512u32;
    let peak = UBig::from(1u8) << peak_bits as usize;
    let descent = (UBig::from(1u8) << peak_bits as usize) - 1u8;
    let mut acc = Accumulator::new();
    let mut oracle = IBig::from(0);
    acc.add_wide(&peak);
    oracle += IBig::from(peak);
    assert_eq!(acc.sign(), Ordering::Greater);
    for cycle in 0..200 {
        acc.sub_wide(&descent);
        oracle -= IBig::from(descent.clone());
        assert_eq!(acc.sign(), Ordering::Greater, "down at 1");
        acc.add_wide(&descent);
        oracle += IBig::from(descent.clone());
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
    for engine in [false, true] {
        let mut acc = fresh(engine);
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
}

/// A redundantly spelled zero reads `is_literally_zero() == false` until
/// a sign read collapses it: the one-sided contract the method's name and
/// rustdoc carry, pinned so any change to it is deliberate.
#[test]
fn redundant_zero_reads_nonzero_until_collapsed() {
    let mut acc = Accumulator::new();
    acc.add_wide(&(UBig::from(1u8) << 32usize));
    acc.sub_small(1 << 32);
    let (sign, magnitude) = acc.sign_magnitude();
    assert_eq!((sign, magnitude), (Ordering::Equal, UBig::ZERO));
    assert!(
        !acc.is_literally_zero(),
        "cancelling digits spell zero redundantly"
    );
    assert_eq!(acc.sign(), Ordering::Equal);
    assert!(
        acc.is_literally_zero(),
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
