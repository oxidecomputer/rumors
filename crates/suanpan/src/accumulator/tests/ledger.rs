//! The zero-run ledger's structural invariants, exhaustively.
//!
//! The ledger's certificates are consumed by scans and trusted without
//! re-reading the digits they certify, so their soundness (all-zero
//! interiors) and structure (disjointness, containment under the
//! settled top) are load-bearing for both cost and correctness:
//! `crop_runs`' descending early stop is derived from disjointness
//! plus containment, and a stale certificate over a written digit
//! would corrupt values, not just costs. The checker below reads the
//! private state directly and holds the full letter of the invariant
//! after every step of every schedule the drivers explore — stronger
//! than the field doc needs, so a weakening of any clause fails here
//! first, by name.

use dashu_int::{IBig, UBig};
use proptest::prelude::*;

use super::{assert_value, fresh, from_limbs, oracle_sign, Accumulator};
use crate::accumulator::LAZY_LIMIT;

/// Every structural invariant of the digit buffer and the zero-run
/// ledger, checked against the private state.
///
/// Clauses: every digit in the lazy zone; `top` exact (zeros above,
/// nonzero at it unless the buffer is all-zero); the write watermark
/// sound (digits below `bottom` all zero); and per certificate a
/// nonempty interior, an all-zero interior (soundness), containment
/// at or below the settled `top`, and disjointness from every other
/// run (in `lo` order, each run starts at or past the previous run's
/// end).
fn assert_ledger_invariants(acc: &Accumulator, schedule: &[u8]) {
    if acc.quick.is_some() {
        assert!(
            acc.digits.iter().all(|&digit| digit == 0) && acc.zero_runs.is_empty(),
            "a live register leaves the digit engine idle after {schedule:?}"
        );
        return;
    }
    assert!(
        acc.top < acc.digits.len(),
        "top {} outside the digit buffer (len {}) after {schedule:?}",
        acc.top,
        acc.digits.len()
    );
    for (index, &digit) in acc.digits.iter().enumerate() {
        assert!(
            i128::from(digit).abs() < LAZY_LIMIT,
            "digit {index} = {digit} outside the lazy zone after {schedule:?}"
        );
    }
    assert!(
        acc.digits[acc.top + 1..].iter().all(|&digit| digit == 0),
        "nonzero digit above top {} after {schedule:?}",
        acc.top
    );
    assert!(
        acc.top == 0 || acc.digits[acc.top] != 0,
        "top {} rests on a zero digit after {schedule:?}",
        acc.top
    );
    let floor = acc.bottom.min(acc.digits.len());
    assert!(
        acc.digits[..floor].iter().all(|&digit| digit == 0),
        "nonzero digit below the write watermark {} after {schedule:?}",
        acc.bottom
    );
    let mut prev_hi = 0usize;
    for (&lo, &hi) in &acc.zero_runs {
        assert!(
            lo + 1 < hi,
            "certificate ({lo}, {hi}) has an empty interior after {schedule:?}"
        );
        assert!(
            hi <= acc.top,
            "certificate ({lo}, {hi}) stranded above the settled top {} \
             after {schedule:?}",
            acc.top
        );
        assert!(
            lo >= prev_hi,
            "certificate ({lo}, {hi}) overlaps the run ending at {prev_hi} \
             after {schedule:?}"
        );
        assert!(
            acc.digits[lo + 1..hi].iter().all(|&digit| digit == 0),
            "certificate ({lo}, {hi}) covers a nonzero digit after \
             {schedule:?}: a stale interior-zero claim corrupts every scan \
             that consumes it"
        );
        prev_hi = hi;
    }
}

/// Precomputed operands of the exhaustive ledger driver's alphabet.
struct LedgerCtx {
    /// `2^32` as a word-scale magnitude.
    ///
    /// `*_magnitude_shl` deposits it raw at a digit position (no
    /// per-digit canonicalization), the one public route to an
    /// adjacent-digit spelling like `(+1, −2^32)` — the shape that
    /// drives the sign fold's running partial to exact zero above a
    /// certified run.
    word32: UBig,
    /// Oracle values of the shifted ops: `2^96`, `2^224`, `u64::MAX`.
    p96: IBig,
    p224: IBig,
    max64: IBig,
}

/// Ops in the exhaustive ledger driver's alphabet.
const LEDGER_OPS: u8 = 11;

/// Apply one alphabet op to the accumulator and the oracle in
/// lockstep.
///
/// The alphabet reaches every ledger transition within a short
/// schedule: word-scale deltas at digit 0 (including `u64::MAX`,
/// whose deposit recenters across digits 0–1 and, repeated, carries
/// into digit 2 — through a certified run's floor), one-limb jumps to
/// digits 3 and 7 in both signs (above-top certificate inserts at two
/// heights, cancelling rewrites, splits when one lands inside the
/// other's run), the raw `−2^32`/`+2^32` deposit at digit 6 (the
/// cancelling under-digit that walks a sign fold into a certified
/// run with a small nonzero partial, or to an exact-zero partial at
/// its edge), and the collapsing sign read itself.
fn ledger_op(ctx: &LedgerCtx, acc: &mut Accumulator, oracle: &mut IBig, op: u8) {
    match op {
        0 => {
            acc.add_small(1);
            *oracle += 1;
        }
        1 => {
            acc.sub_small(1);
            *oracle -= 1;
        }
        2 => {
            acc.add_u64(u64::MAX);
            *oracle += &ctx.max64;
        }
        3 => {
            acc.sub_u64(u64::MAX);
            *oracle -= &ctx.max64;
        }
        4 => {
            acc.add_wide_shl(&UBig::ONE, 96);
            *oracle += &ctx.p96;
        }
        5 => {
            acc.sub_wide_shl(&UBig::ONE, 96);
            *oracle -= &ctx.p96;
        }
        6 => {
            acc.add_wide_shl(&UBig::ONE, 224);
            *oracle += &ctx.p224;
        }
        7 => {
            acc.sub_wide_shl(&UBig::ONE, 224);
            *oracle -= &ctx.p224;
        }
        8 => {
            acc.sub_magnitude_shl(&ctx.word32, 192);
            *oracle -= &ctx.p224;
        }
        9 => {
            acc.add_magnitude_shl(&ctx.word32, 192);
            *oracle += &ctx.p224;
        }
        _ => {
            assert_eq!(acc.sign(), oracle_sign(oracle), "sign read");
        }
    }
}

/// Depth of the exhaustive ledger sweep.
///
/// Every schedule of at most this many alphabet ops runs, with every
/// invariant checked at every step of every schedule (the search is a
/// prefix tree, so each state is reached and checked exactly once).
const LEDGER_DEPTH: usize = 6;

/// Walk the schedule prefix tree: apply each op to a clone of the
/// parent state, check every invariant and the full oracle value,
/// recurse.
fn ledger_dfs(
    ctx: &LedgerCtx,
    acc: &Accumulator,
    oracle: &IBig,
    schedule: &mut Vec<u8>,
    depth: usize,
) {
    for op in 0..LEDGER_OPS {
        schedule.push(op);
        let mut next_acc = acc.clone();
        let mut next_oracle = oracle.clone();
        ledger_op(ctx, &mut next_acc, &mut next_oracle, op);
        assert_ledger_invariants(&next_acc, schedule);
        assert_value(&next_acc, &next_oracle);
        if depth > 1 {
            ledger_dfs(ctx, &next_acc, &next_oracle, schedule, depth - 1);
        }
        schedule.pop();
    }
}

/// The zero-run ledger's letter invariant holds after every operation
/// of every schedule at exhaustive small scope.
///
/// The letter: disjoint certificates with all-zero interiors, every
/// one contained at or below the settled top — checked alongside
/// exact value agreement with the `IBig` oracle.
///
/// Exhaustive over all 11-op schedules of length ≤ 6 (1,948,716
/// states, each checked once; ~4 s dev — the length-≤ 7 sweep's
/// 21.4M states also passed once, at pin time): word-scale deltas,
/// recentering
/// `u64::MAX` deltas, one-limb jumps to digits 3 and 7 in both
/// signs, raw `±2^32` deposits at digit 6, and collapsing sign
/// reads — the space containing every collapse-over-certificate
/// interaction: a fold breaking one step inside a certified run
/// (small nonzero partial over zeros decides at the run's first
/// interior digit, and the collapse re-deposit's crop keeps only the
/// lower remnant), a zero partial consuming a run mid-fold, above-top
/// jump inserts stacking and splitting certificates, and carry runs
/// writing through a run's floor. In particular: no schedule strands
/// a certificate above the settled top, so the containment clause is
/// a standing invariant here, not merely a creation-time fact.
#[test]
fn ledger_invariants_hold_exhaustively() {
    let ctx = LedgerCtx {
        word32: UBig::from(1u64 << 32),
        p96: IBig::from(UBig::ONE << 96usize),
        p224: IBig::from(UBig::ONE << 224usize),
        max64: IBig::from(u64::MAX),
    };
    let acc = fresh(true);
    let oracle = IBig::from(0);
    let mut schedule = Vec::with_capacity(LEDGER_DEPTH);
    ledger_dfs(&ctx, &acc, &oracle, &mut schedule, LEDGER_DEPTH);
}

proptest! {
    /// The ledger's structural invariants hold after every step of
    /// randomized run-forming streams.
    ///
    /// The exhaustive sweep's long-schedule, deep-shift complement:
    /// shifts to 4,096 bits, schedules to 150 ops, sign reads
    /// interleaved throughout.
    #[test]
    fn ledger_invariants_hold_on_run_forming_streams(
        ops in proptest::collection::vec(
            (0u8..5, proptest::collection::vec(any::<u64>(), 1..=2), 0u64..4_096),
            1..150,
        ),
        engine_first: bool,
    ) {
        let mut acc = fresh(engine_first);
        let mut oracle = IBig::from(0);
        let mut schedule: Vec<u8> = Vec::new();
        for (step, (arm, limbs, shift)) in ops.iter().enumerate() {
            schedule.push(*arm);
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
                3 => {
                    let delta = limbs[0] as i64;
                    acc.add_small(delta);
                    oracle += delta;
                }
                _ => {
                    prop_assert_eq!(
                        acc.sign(),
                        oracle_sign(&oracle),
                        "sign at step {}",
                        step
                    );
                }
            }
            assert_ledger_invariants(&acc, &schedule);
            if step % 32 == 0 {
                assert_value(&acc, &oracle);
            }
        }
        assert_value(&acc, &oracle);
    }
}
