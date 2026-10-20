//! Reviewer scratch: exhaustive short histories ending in or passing through
//! `reset`, checking that a reset never leaves a nonzero retained digit.
//!
//! Every history of up to `DEPTH` steps over `Op::ALL` runs from both a fresh
//! scalar start and a forced-digit start. After every step the stored form
//! must satisfy the digit invariants (which, for an idle value after reset,
//! require every retained digit to be zero), and the value must equal an
//! independent `num-bigint` oracle.

use core::cmp::Ordering;

use num_bigint::BigInt as IBig;

use super::{fresh, read_value, Accumulator};

const DEPTH: usize = 5;

#[derive(Clone, Copy, Debug)]
enum Op {
    /// Add 1 at digit `p`.
    AddOne(u32),
    /// Subtract 1 at digit `p`.
    SubOne(u32),
    /// Add `2^64 - 1` at digit `p` (two full digits, carries on repeat).
    AddWide(u32),
    /// Compare with zero in place (sign compaction).
    Sign,
    /// Scaled read in place.
    ScaledRead,
    /// Unscaled read in place.
    FullRead,
    /// Unary negation.
    Negate,
    /// `<<= 32`.
    ShiftDigit,
    /// `clone_from` a source with a high cancelled-then-restored value.
    CloneHigh,
    /// Reset.
    Reset,
}

impl Op {
    const ALL: [Op; 13] = [
        Op::AddOne(0),
        Op::AddOne(3),
        Op::AddOne(9),
        Op::SubOne(3),
        Op::SubOne(9),
        Op::AddWide(5),
        Op::Sign,
        Op::ScaledRead,
        Op::FullRead,
        Op::Negate,
        Op::ShiftDigit,
        Op::CloneHigh,
        Op::Reset,
    ];
}

fn apply(op: Op, acc: &mut Accumulator, oracle: &mut IBig) {
    match op {
        Op::AddOne(p) => {
            acc.add_shifted_limbs(32 * u64::from(p), [1u64]);
            *oracle += IBig::from(1u8) << (32 * p as usize);
        }
        Op::SubOne(p) => {
            acc.sub_shifted_limbs(32 * u64::from(p), [1u64]);
            *oracle -= IBig::from(1u8) << (32 * p as usize);
        }
        Op::AddWide(p) => {
            acc.add_shifted_limbs(32 * u64::from(p), [u64::MAX]);
            *oracle += IBig::from(u64::MAX) << (32 * p as usize);
        }
        Op::Sign => {
            let sign = acc.sign();
            assert_eq!(sign, oracle.sign().into_ordering());
        }
        Op::ScaledRead => {
            assert_eq!(read_value(acc, true), *oracle);
        }
        Op::FullRead => {
            assert_eq!(read_value(acc, false), *oracle);
        }
        Op::Negate => {
            *acc = -core::mem::take(acc);
            *oracle = -core::mem::take(oracle);
        }
        Op::ShiftDigit => {
            *acc <<= 32u32;
            *oracle <<= 32usize;
        }
        Op::CloneHigh => {
            let mut source = Accumulator::new();
            source.add_shifted_limbs(32 * 12, [1u64]);
            source.sub_shifted_limbs(32 * 11, [1u64 << 32]);
            source.add_shifted_limbs(32 * 7, [5u64]);
            acc.clone_from(&source);
            *oracle = IBig::from(5u8) << (32 * 7usize);
        }
        Op::Reset => {
            acc.reset();
            *oracle = IBig::ZERO;
        }
    }
}

trait IntoOrdering {
    fn into_ordering(self) -> Ordering;
}

impl IntoOrdering for num_bigint::Sign {
    fn into_ordering(self) -> Ordering {
        match self {
            num_bigint::Sign::Minus => Ordering::Less,
            num_bigint::Sign::NoSign => Ordering::Equal,
            num_bigint::Sign::Plus => Ordering::Greater,
        }
    }
}

/// The digit form's `(lowest_written, highest_nonzero)`, parsed from `Debug`,
/// or `None` for the scalar form or an unwritten watermark.
fn watermark(acc: &Accumulator) -> Option<(usize, usize)> {
    let text = format!("{acc:?}");
    let field = |name: &str| -> Option<usize> {
        let start = text.find(name)? + name.len();
        let rest = &text[start..];
        let end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
        rest[..end].parse().ok()
    };
    let low = field("lowest_written: ")?;
    let high = field("highest_nonzero: ")?;
    (low != usize::MAX).then_some((low, high))
}

/// Every history of up to five steps keeps every retained digit outside the
/// written span zero, so every reset leaves an all-zero buffer, and the value
/// matches the oracle throughout.
#[test]
fn reviewer_reset_leaves_no_stale_digit_in_short_histories() {
    let mut histories = 0u64;
    let mut resets_above_zero = 0u64;
    let mut resets_above_top = 0u64;
    let mut stack: Vec<Vec<Op>> = vec![Vec::new()];
    while let Some(history) = stack.pop() {
        for digits_start in [false, true] {
            let mut acc = fresh(digits_start);
            let mut oracle = IBig::ZERO;
            let trace: Vec<u8> = history.iter().map(|op| Op::ALL.iter().position(|o| format!("{o:?}") == format!("{op:?}")).unwrap() as u8).collect();
            for (index, &op) in history.iter().enumerate() {
                if matches!(op, Op::Reset) {
                    if let Some((low, high)) = watermark(&acc) {
                        if low > 0 {
                            resets_above_zero += 1;
                        }
                        if low > high {
                            resets_above_top += 1;
                        }
                    }
                }
                apply(op, &mut acc, &mut oracle);
                acc.stored().assert_invariants(&trace[..=index]);
                // Read a stored copy so the check does not change the history.
                let mut copy = acc.stored_copy();
                assert_eq!(read_value(&mut copy, true), oracle, "after {history:?}");
            }
            // Reuse after a final reset must start from zero.
            acc.reset();
            acc.stored().assert_invariants(&trace);
            acc.add_shifted_limbs(32 * 2, [3u64]);
            assert_eq!(
                read_value(&mut acc, false),
                IBig::from(3u8) << 64usize,
                "reuse after {history:?}"
            );
        }
        histories += 1;
        if history.len() < DEPTH {
            for op in Op::ALL {
                let mut next = history.clone();
                next.push(op);
                stack.push(next);
            }
        }
    }
    eprintln!("REVIEW histories={histories} resets_above_zero={resets_above_zero} resets_above_top={resets_above_top}");
    assert!(resets_above_zero > 0 && resets_above_top > 0);
}
