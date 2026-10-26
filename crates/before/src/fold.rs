//! Balanced reductions over stored trees.
//!
//! Each stack entry represents the same number of inputs as any entry at its
//! level; their byte sizes need not match. A new input repeatedly merges with
//! an entry at the same level, so every input participates in `O(log k)`
//! combines. When a combiner's result is no larger than its operands together,
//! every level contains at most the original `D` bytes. A linear combiner then
//! takes `O(D log k)` time and the live groups occupy `O(D)` space. This avoids
//! repeatedly scanning one growing accumulator. Infallible combiners must be
//! associative and commutative; fallible combiners stop at the first
//! incompatible pair.
//!
//! An idempotent combiner can also skip inputs it would absorb unchanged:
//! [`dedup_runs`] drops adjacent inputs that share storage before the counter
//! reads them.
//!
//! [`join_all`] reunites disjoint values, such as parties or clocks, through
//! the fallible counter, in two steps a caller may also run apart:
//! [`fold_groups`] and then [`absorb_groups`].

#[cfg(test)]
mod tests;

/// Reduce `iter` with a fallible combiner.
///
/// On success, returns at most `O(log k)` groups in arrival order. On the first
/// failed combine, it stops. The returned groups and untouched iterator items
/// represent every unconsumed input exactly once.
pub(crate) fn balanced_try_fold<T>(
    iter: impl IntoIterator<Item = T>,
    mut combine: impl FnMut(T, T) -> Result<T, (T, T)>,
) -> Result<Vec<T>, Vec<T>> {
    let mut iter = iter.into_iter();
    let mut stack: Vec<(T, usize)> = Vec::new();
    while let Some(item) = iter.next() {
        let mut merged = item;
        let mut weight = 0usize;
        while stack.last().is_some_and(|(_, w)| *w == weight) {
            let (top, _) = stack.pop().expect("the loop condition saw a top entry");
            match combine(top, merged) {
                Ok(group) => {
                    merged = group;
                    weight += 1;
                }
                Err((top, back)) => {
                    stack.push((top, weight));
                    let mut uncombined: Vec<T> =
                        stack.into_iter().map(|(group, _)| group).collect();
                    uncombined.push(back);
                    uncombined.extend(iter);
                    return Err(uncombined);
                }
            }
        }
        stack.push((merged, weight));
    }
    Ok(stack.into_iter().map(|(group, _)| group).collect())
}

/// Joins every item of `iter` into `receiver`: [`fold_groups`], then
/// [`absorb_groups`].
///
/// `join(target, other)` must either join `other` into `target` and return
/// `Ok(())`, or, when the two overlap, leave `target` unchanged and return
/// `Err(other)`. [`Party::join`](crate::Party::join) has this contract, and
/// [`Clock::join`](crate::Clock::join) has it once its returned version is
/// discarded.
///
/// Success means `receiver` and the inputs were pairwise disjoint: the fold
/// tests every pair of inputs within one group, and the absorb tests each
/// group against `receiver`, which by then holds every earlier group.
///
/// # Errors
///
/// When some pair overlaps, returns what was not joined into `receiver`, as
/// each step's `# Errors` describes. Every input then belongs to exactly one
/// value: `receiver` or one returned element. A returned element may be the
/// join of several inputs.
pub(crate) fn join_all<T>(
    receiver: &mut T,
    iter: impl IntoIterator<Item = T>,
    mut join: impl FnMut(&mut T, T) -> Result<(), T>,
) -> Result<(), Vec<T>> {
    let groups = fold_groups(iter, &mut join)?;
    absorb_groups(receiver, groups, join)
}

/// Folds `iter` into at most `O(log k)` groups, `k` the input count, by
/// joining pairs of groups with `join`, the first step of [`join_all`].
///
/// `join` has [`join_all`]'s contract. The fold runs on
/// [`balanced_try_fold`], so a group forms only when `join` finds two smaller
/// groups disjoint, and each group is therefore the union of
/// pairwise-disjoint inputs. The groups may overlap one another: the counter
/// joins only groups that represent equally many inputs, and the groups it
/// returns each represent a different count, so it never tests any two of
/// them against each other. [`absorb_groups`] tests that.
///
/// The fold reads no receiver. A caller that must join into a copy of its
/// receiver can therefore make the copy after the fold returns, keeping it
/// out of the fold's peak heap.
///
/// # Errors
///
/// On the first join whose operands overlap, returns every group built so far
/// and every input not yet folded, so each input belongs to exactly one
/// returned element.
pub(crate) fn fold_groups<T>(
    iter: impl IntoIterator<Item = T>,
    mut join: impl FnMut(&mut T, T) -> Result<(), T>,
) -> Result<Vec<T>, Vec<T>> {
    balanced_try_fold(iter, |mut group, incoming| {
        match join(&mut group, incoming) {
            Ok(()) => Ok(group),
            Err(incoming) => Err((group, incoming)),
        }
    })
}

/// Joins `groups` into `receiver` in order with `join`, the second step of
/// [`join_all`].
///
/// `join` has [`join_all`]'s contract. Each group meets a `receiver` that
/// already holds every earlier group, so this step rejects a group that
/// overlaps the original receiver or any earlier group.
///
/// # Errors
///
/// On the first group that overlaps `receiver`, returns that group and every
/// later one, untested. The groups before it stay joined into `receiver`, so
/// each input belongs to exactly one of `receiver` and the returned elements.
pub(crate) fn absorb_groups<T>(
    receiver: &mut T,
    groups: Vec<T>,
    mut join: impl FnMut(&mut T, T) -> Result<(), T>,
) -> Result<(), Vec<T>> {
    let mut groups = groups.into_iter();
    while let Some(group) = groups.next() {
        if let Err(group) = join(receiver, group) {
            let mut uncombined = vec![group];
            uncombined.extend(groups);
            return Err(uncombined);
        }
    }
    Ok(())
}

/// Reduce `iter` through the balanced binary counter with an infallible
/// combiner, or [`None`] for an empty iterator.
///
/// The closing reduction preserves arrival order.
pub(crate) fn balanced_reduce<T>(
    iter: impl IntoIterator<Item = T>,
    mut combine: impl FnMut(T, T) -> T,
) -> Option<T> {
    let groups = match balanced_try_fold(iter, |a, b| Ok::<T, (T, T)>(combine(a, b))) {
        Ok(groups) => groups,
        Err(_) => unreachable!("an infallible combiner rejects nothing"),
    };
    groups.into_iter().reduce(combine)
}

/// Yield the first item of each run of adjacent items, where a run continues
/// while `same(first, next)` holds for the run's first item.
///
/// Finding a run's end means pulling the item after it, so the adapter holds
/// one input ahead of its consumer. When inputs are built as they are pulled,
/// peak heap can exceed an on-demand fold's by up to one input.
pub(crate) fn dedup_runs<I: IntoIterator>(
    iter: I,
    mut same: impl FnMut(&I::Item, &I::Item) -> bool,
) -> impl Iterator<Item = I::Item> {
    let mut iter = iter.into_iter().peekable();
    core::iter::from_fn(move || {
        let first = iter.next()?;
        while iter.next_if(|next| same(&first, next)).is_some() {}
        Some(first)
    })
}
