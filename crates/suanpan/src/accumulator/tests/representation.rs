//! Arithmetic schedules preserve the signed digits and recorded zero ranges.
//!
//! At every step, compare the value with a big-integer oracle and inspect the
//! representation itself. A stale range could skip a nonzero digit on a later
//! query even while the current value still reads correctly.

use num_bigint::{BigInt as IBig, BigUint as UBig};
use proptest::prelude::*;

use super::{assert_value, fresh, from_limbs, oracle_sign, Accumulator, TestBig as _};

/// Fixed operands that create carries, cancellations, and separated writes.
struct Operands {
    /// `2^32`, used to deposit a whole signed coefficient in one digit.
    ///
    /// The word path does not split the coefficient across two digits. That
    /// distinction constructs an adjacent representation such as `(+1, −2^32)`,
    /// which can drive a sign fold's partial to zero above a recorded zero range.
    word32: UBig,
    /// A unit contribution at digit three.
    p96: IBig,
    /// A unit contribution at digit seven.
    p224: IBig,
    /// A word-sized contribution large enough to carry across digits.
    max64: IBig,
}

/// Number of update and sign-query operations explored at each step.
const REPRESENTATION_OPS: u8 = 11;

/// Apply the finite operation alphabet using its precomputed operands.
impl Operands {
    /// Apply one operation to the accumulator and oracle in lockstep.
    ///
    /// Unit and maximum-word updates exercise ordinary deposits and carries.
    /// Jumps to digits three and seven create, clear, and split zero runs.
    /// Depositing a whole coefficient of ±2^32 at digit six makes the sign
    /// fold reach a run with either a small nonzero or exactly zero partial.
    fn apply(&self, acc: &mut Accumulator, oracle: &mut IBig, op: u8) {
        match op {
            0 => {
                *acc += 1_i64;
                *oracle += 1;
            }
            1 => {
                *acc -= 1_i64;
                *oracle -= 1;
            }
            2 => {
                *acc += u64::MAX;
                *oracle += &self.max64;
            }
            3 => {
                *acc -= u64::MAX;
                *oracle -= &self.max64;
            }
            4 => {
                acc.add_limb_value_shl(&UBig::ONE, 96);
                *oracle += &self.p96;
            }
            5 => {
                acc.sub_limb_value_shl(&UBig::ONE, 96);
                *oracle -= &self.p96;
            }
            6 => {
                acc.add_limb_value_shl(&UBig::ONE, 224);
                *oracle += &self.p224;
            }
            7 => {
                acc.sub_limb_value_shl(&UBig::ONE, 224);
                *oracle -= &self.p224;
            }
            8 => {
                acc.sub_value_shl(&self.word32, 192);
                *oracle -= &self.p224;
            }
            9 => {
                acc.add_value_shl(&self.word32, 192);
                *oracle += &self.p224;
            }
            _ => {
                assert_eq!(acc.cmp_zero(), oracle_sign(oracle), "comparison with zero");
            }
        }
    }
}

/// Depth of the exhaustive representation sweep.
///
/// Every schedule of at most this many alphabet ops runs, with every
/// invariant checked at every step of every schedule (the search is a
/// prefix tree, so each state is reached and checked exactly once).
const REPRESENTATION_DEPTH: usize = 6;

/// Check every child schedule, then recurse while retaining its operation trace.
fn check_schedules(
    operands: &Operands,
    acc: &Accumulator,
    oracle: &IBig,
    schedule: &mut Vec<u8>,
    depth: usize,
) {
    for op in 0..REPRESENTATION_OPS {
        schedule.push(op);
        let mut next_acc = acc.clone();
        let mut next_oracle = oracle.clone();
        operands.apply(&mut next_acc, &mut next_oracle, op);
        next_acc
            .digits
            .assert_invariants(next_acc.small.is_some(), schedule);
        assert_value(&next_acc, &next_oracle);
        if depth > 1 {
            check_schedules(operands, &next_acc, &next_oracle, schedule, depth - 1);
        }
        schedule.pop();
    }
}

/// Every schedule of up to six operations preserves the digit and range invariants.
///
/// The eleven-operation alphabet exercises gap creation, splitting, and
/// consumption, including carries and sign collapse inside a recorded zero range.
/// Every state must match the oracle and retain disjoint zero ranges wholly
/// within the stored width.
#[test]
fn representation_invariants_hold_exhaustively() {
    let operands = Operands {
        word32: UBig::from(1u64 << 32),
        p96: IBig::from(UBig::ONE << 96usize),
        p224: IBig::from(UBig::ONE << 224usize),
        max64: IBig::from(u64::MAX),
    };
    let acc = fresh(true);
    let oracle = IBig::from(0);
    let mut schedule = Vec::with_capacity(REPRESENTATION_DEPTH);
    check_schedules(
        &operands,
        &acc,
        &oracle,
        &mut schedule,
        REPRESENTATION_DEPTH,
    );
}

proptest! {
    /// Longer streams with wider shifts preserve the value, signed-digit bounds,
    /// stored width, and disjoint zero ranges after every step.
    ///
    /// The exhaustive sweep's long-schedule, deep-shift complement:
    /// shifts to 4,096 bits, schedules to 150 ops, comparisons with zero
    /// interleaved throughout.
    #[test]
    fn representation_invariants_hold_on_run_forming_streams(
        ops in proptest::collection::vec(
            (0u8..5, proptest::collection::vec(any::<u64>(), 1..=2), 0u64..4_096),
            1..150,
        ),
        digits_first: bool,
    ) {
        let mut acc = fresh(digits_first);
        let mut oracle = IBig::from(0);
        let mut schedule: Vec<u8> = Vec::new();
        for (step, (arm, limbs, shift)) in ops.iter().enumerate() {
            schedule.push(*arm);
            let value = from_limbs(limbs);
            let scaled = IBig::from(value.clone()) << usize::try_from(*shift).unwrap();
            match arm {
                0 => {
                    acc.add_limb_value_shl(&value, *shift);
                    oracle += scaled;
                }
                1 => {
                    acc.sub_limb_value_shl(&value, *shift);
                    oracle -= scaled;
                }
                2 => {
                    acc.sub_value_shl(&value, *shift);
                    oracle -= scaled;
                }
                3 => {
                    let delta = limbs[0] as i64;
                    acc += delta;
                    oracle += delta;
                }
                _ => {
                    prop_assert_eq!(
                        acc.cmp_zero(),
                        oracle_sign(&oracle),
                        "sign at step {}",
                        step
                    );
                }
            }
            acc.digits.assert_invariants(acc.small.is_some(), &schedule);
            if step % 32 == 0 {
                assert_value(&acc, &oracle);
            }
        }
        assert_value(&acc, &oracle);
    }
}
