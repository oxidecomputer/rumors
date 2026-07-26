//! The join-fold discipline held as a differential oracle, with its
//! up-front overlap test spelled as a per-input cursor walk.
//!
//! `Party::join_all` and `Clock::join_all` test every input against the
//! *fixed* accumulator up front — the hand-back granularity their
//! contracts document — through a per-call `IdIndex` of the accumulator.
//! The index is a performance mechanism only: which inputs come back, in
//! what order, and what the accumulator holds afterward must be exactly
//! what the same discipline decides when each up-front test is a plain
//! cursor walk of the fixed accumulator ([`Party::is_disjoint`]). These
//! oracles are that spelling: the identical binary-counter fold with the
//! identical hand-back bookkeeping, differing from production in the
//! up-front test's mechanism alone. The fold differentials in
//! `party/tests.rs` and `clock/tests.rs` pin production against them —
//! byte-identical accumulators, identical hand-back vectors — across
//! adversarial input mixes, so the index can never silently change an
//! outcome.
//!
//! Deliberately quadratic on populations of many inputs against a large
//! accumulator (each test re-walks the fixed side): bounded test
//! populations only.

use crate::{Clock, Party};

/// Fold every disjoint [`Party`] in `inputs` into `acc` with the
/// production discipline, testing each input against the fixed `acc` by
/// cursor walk.
///
/// Same contract as [`Party::join_all`]: overlapping inputs come back in
/// the error, everything else coalesces into `acc`.
pub(crate) fn party_join_all(
    acc: &mut Party,
    inputs: impl IntoIterator<Item = Party>,
) -> Result<(), Vec<Party>> {
    let mut overlapping = Vec::new();
    let mut stack: Vec<(Party, u32)> = Vec::new();
    for other in inputs {
        if !acc.is_disjoint(&other) {
            overlapping.push(other);
            continue;
        }
        let mut merged = Some(other);
        let mut weight = 0u32;
        while stack.last().is_some_and(|(_, w)| *w == weight) {
            let (mut top, _) = stack.pop().expect("the loop condition saw a top entry");
            match top.join(merged.take().expect("the operand is held while merging up")) {
                Ok(()) => {
                    merged = Some(top);
                    weight += 1;
                }
                Err(back) => {
                    stack.push((top, weight));
                    if weight == 0 {
                        overlapping.push(back);
                    } else {
                        stack.push((back, weight));
                    }
                    break;
                }
            }
        }
        if let Some(merged) = merged {
            stack.push((merged, weight));
        }
    }
    for (group, _) in stack {
        if let Err(back) = acc.join(group) {
            overlapping.push(back);
        }
    }
    if overlapping.is_empty() {
        Ok(())
    } else {
        Err(overlapping)
    }
}

/// Fold every disjoint [`Clock`] in `inputs` into `acc` with the
/// production discipline, testing each input's party against the fixed
/// `acc`'s by cursor walk.
///
/// Same contract as [`Clock::join_all`]: overlapping inputs come back in
/// the error, everything else has its party reunited and its version
/// merged into `acc`.
pub(crate) fn clock_join_all(
    acc: &mut Clock,
    inputs: impl IntoIterator<Item = Clock>,
) -> Result<(), Vec<Clock>> {
    let mut overlapping = Vec::new();
    let mut stack: Vec<(Clock, u32)> = Vec::new();
    for other in inputs {
        if !acc.party().is_disjoint(other.party()) {
            overlapping.push(other);
            continue;
        }
        let mut merged = Some(other);
        let mut weight = 0u32;
        while stack.last().is_some_and(|(_, w)| *w == weight) {
            let (mut top, _) = stack.pop().expect("the loop condition saw a top entry");
            match top.join(merged.take().expect("the operand is held while merging up")) {
                Ok(_) => {
                    merged = Some(top);
                    weight += 1;
                }
                Err(back) => {
                    stack.push((top, weight));
                    if weight == 0 {
                        overlapping.push(back);
                    } else {
                        stack.push((back, weight));
                    }
                    break;
                }
            }
        }
        if let Some(merged) = merged {
            stack.push((merged, weight));
        }
    }
    for (group, _) in stack {
        if let Err(back) = acc.join(group) {
            overlapping.push(back);
        }
    }
    if overlapping.is_empty() {
        Ok(())
    } else {
        Err(overlapping)
    }
}
