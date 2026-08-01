//! The balanced binary-counter reduction every n-ary fold runs on: the
//! one home for the counter discipline, so a hardening of the fold
//! shape reaches every fold at once.
//!
//! An incoming operand merges upward while the top stack entry holds as
//! many inputs as it does, so every input passes through `O(log k)`
//! combines against similarly sized partners and no combine's operand
//! is more than a bounded factor larger than its partner. A sequential
//! left fold instead combines every input into the whole accumulated
//! result — quadratic sweep work whenever the accumulator's packed size
//! tracks the population's, which every lattice direction here reaches:
//! a join's union can grow without coalescing (interleaved single-tick
//! versions; scattered party regions), and a meet's result can shrink
//! in value but not in packed size (one deep version among operands
//! that dominate it). Every combiner fed to this module is associative
//! and commutative, so the counter's regrouping is value-identical to
//! the left fold's.
//!
//! The callers: [`Version::join_all`](crate::Version::join_all) and
//! [`Version::meet_all`](crate::Version::meet_all) through
//! [`balanced_reduce`]; [`Party::join_all`](crate::Party::join_all) and
//! [`Clock::join_all`](crate::Clock::join_all) through
//! [`balanced_try_fold`], whose fallible combiner and rejection channel
//! carry their aliased-input hand-back policy. The promotion ledger's
//! product-tree settle (`Integrator::settle_armings`, the skyline query
//! fold) runs the same counter discipline hand-rolled: its combiner
//! charges an accumulator as a side effect and its closing drain folds
//! newest-first (the committed settle readings are pinned against that
//! association), so it names this module instead of routing through it.

/// Reduce `iter` through the balanced binary counter with a fallible
/// combiner, returning the surviving groups oldest-first; inputs the
/// fold cannot place land in `rejected`.
///
/// Each accepted input enters the counter at weight 0 and merges upward
/// through `combine(older, newer)` — the left operand is always the
/// group that arrived earlier — while the top stack entry holds as many
/// inputs as the merging group does. The two rejection paths preserve
/// the caller's feed-order accounting exactly:
///
/// - an input failing `accept` is handed to `rejected` immediately,
///   before it touches the counter (the callers' up-front overlap test
///   against a fixed accumulator);
/// - a failed combine (`Err((older, newer))`) retains the older group
///   on the stack at its weight; a *lone* newer input (weight 0) is
///   handed to `rejected`, while a newer group that already coalesced
///   stays on the stack unmerged at the same weight — dropping nothing,
///   at the cost of one over-full counter slot on inputs only aliasing
///   can produce.
///
/// The returned groups are in stack order, oldest (heaviest) first, for
/// the caller's closing drain: a left-to-right fold over them keeps
/// every combine's left operand the older group.
pub(crate) fn balanced_try_fold<T>(
    iter: impl IntoIterator<Item = T>,
    mut accept: impl FnMut(&T) -> bool,
    mut combine: impl FnMut(T, T) -> Result<T, (T, T)>,
    rejected: &mut Vec<T>,
) -> Vec<T> {
    let mut stack: Vec<(T, u32)> = Vec::new();
    for item in iter {
        if !accept(&item) {
            rejected.push(item);
            continue;
        }
        let mut merged = Some(item);
        let mut weight = 0u32;
        while stack.last().is_some_and(|(_, w)| *w == weight) {
            let (top, _) = stack.pop().expect("the loop condition saw a top entry");
            match combine(
                top,
                merged.take().expect("the operand is held while merging up"),
            ) {
                Ok(group) => {
                    merged = Some(group);
                    weight += 1;
                }
                Err((top, back)) => {
                    stack.push((top, weight));
                    if weight == 0 {
                        rejected.push(back);
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
    stack.into_iter().map(|(group, _)| group).collect()
}

/// Reduce `iter` through the balanced binary counter with an infallible
/// combiner, or [`None`] for an empty iterator.
///
/// [`balanced_try_fold`] with every input accepted and the closing
/// drain folded in: `Version::join_all` restores its identity (the
/// empty version) over the `None`; the meet has none, so
/// `Version::meet_all` returns the `Option` as is.
pub(crate) fn balanced_reduce<T>(
    iter: impl IntoIterator<Item = T>,
    mut combine: impl FnMut(T, T) -> T,
) -> Option<T> {
    let mut rejected = Vec::new();
    let groups = balanced_try_fold(iter, |_| true, |a, b| Ok(combine(a, b)), &mut rejected);
    debug_assert!(
        rejected.is_empty(),
        "an infallible combiner rejects nothing"
    );
    // The closing drain: bottom-up, so the heaviest group seeds the
    // fold and every remaining combine pairs it with the next-lighter
    // one, keeping the left operand the older group.
    groups.into_iter().reduce(combine)
}
