//! Balanced reductions over encoded trees.
//!
//! Each stack entry combines the same number of inputs. A new input repeatedly
//! merges with an equal-sized entry, so every input participates in `O(log k)`
//! combines with similarly sized values. This avoids repeatedly scanning a
//! growing accumulator. Infallible combiners must be associative and
//! commutative; fallible combiners stop at the first incompatible pair.

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
