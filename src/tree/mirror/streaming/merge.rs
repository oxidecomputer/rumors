//! Prefix-ordered streaming merge-join.
//!
//! The streaming analog of [`itertools::Itertools::merge_join_by`]: given two
//! streams whose items ascend by an extracted key, with no duplicate keys
//! within either, walk them in lockstep and emit one
//! [`EitherOrBoth`] per distinct key in ascending order — `Left` for a key only
//! the left stream carries, `Right` for one only the right does, `Both` where
//! they coincide.
//!
//! This is the join at the heart of the mirror's asymmetry matrix (our frontier
//! nodes against the counterparty's `uncertain` hashes), factored out because
//! `futures` ships no equivalent and `itertools`' operates only on iterators.
//!
//! Memory is a single item of lookahead per side: when the two keys differ, the
//! larger item is held back for the next round rather than dropped, so nothing
//! is buffered beyond the two heads.

use async_stream::try_stream;
use futures::stream::{Stream, StreamExt};
use itertools::EitherOrBoth;

/// Merge-join two ascending-by-key, fallible streams into a stream of
/// [`EitherOrBoth`], propagating the first error from either side.
///
/// `key_left`/`key_right` extract the comparison key from each side's item. The
/// caller guarantees each stream is strictly ascending by its key; the output
/// is then ascending too, one item per distinct key.
pub fn merge_join_by<'a, L, R, A, B, E, K, FL, FR>(
    left: L,
    right: R,
    key_left: FL,
    key_right: FR,
) -> impl Stream<Item = Result<EitherOrBoth<A, B>, E>> + Send + 'a
where
    L: Stream<Item = Result<A, E>> + Send + 'a,
    R: Stream<Item = Result<B, E>> + Send + 'a,
    FL: Fn(&A) -> K + Send + 'a,
    FR: Fn(&B) -> K + Send + 'a,
    K: Ord,
    A: Send + 'a,
    B: Send + 'a,
    E: Send + 'a,
{
    try_stream! {
        // Fused so that an exhausted side answers `None` immediately forever:
        // the loop below keeps polling both sides until each has ended.
        let mut left = Box::pin(left.fuse());
        let mut right = Box::pin(right.fuse());

        // One item of lookahead per side. On a key mismatch the larger head is
        // put back here rather than consumed, so each item is compared once.
        let mut head_left: Option<A> = None;
        let mut head_right: Option<B> = None;

        loop {
            // When both heads are empty, fill them concurrently. The mirror
            // feeds this join from channels whose producers are independently
            // scheduled pumps, so a sequential fill would be correct — but it
            // would serialize the join on whichever side happens to be
            // awaited first, adding a round of producer latency per item.
            // Once one head is held, only the empty side is awaited: the held
            // side's producer needs nothing from us until its head is
            // consumed.
            if head_left.is_none() && head_right.is_none() {
                let (l, r) = futures::future::join(left.next(), right.next()).await;
                if let Some(item) = l {
                    head_left = Some(item?);
                }
                if let Some(item) = r {
                    head_right = Some(item?);
                }
            } else if head_left.is_none() {
                if let Some(item) = left.next().await {
                    head_left = Some(item?);
                }
            } else if head_right.is_none()
                && let Some(item) = right.next().await
            {
                head_right = Some(item?);
            }

            match (head_left.take(), head_right.take()) {
                (None, None) => break,
                (Some(a), None) => yield EitherOrBoth::Left(a),
                (None, Some(b)) => yield EitherOrBoth::Right(b),
                (Some(a), Some(b)) => {
                    // Resolve the ordering into a local so the extracted keys
                    // (type `K`, not necessarily `Send`) don't straddle the
                    // `yield` await.
                    let ordering = key_left(&a).cmp(&key_right(&b));
                    match ordering {
                        // Left key is smaller: emit it, hold the right head back.
                        std::cmp::Ordering::Less => {
                            head_right = Some(b);
                            yield EitherOrBoth::Left(a);
                        }
                        // Right key is smaller: emit it, hold the left head back.
                        std::cmp::Ordering::Greater => {
                            head_left = Some(a);
                            yield EitherOrBoth::Right(b);
                        }
                        // Keys coincide: emit both, consuming both heads.
                        std::cmp::Ordering::Equal => yield EitherOrBoth::Both(a, b),
                    }
                }
            }
        }
    }
}

/// Merge two ascending-by-key, fallible streams of the *same* item type into
/// one ascending stream, requiring the key sets to be disjoint.
///
/// This is the union half of [`merge_join_by`], used by the mirror's upward
/// reassembly: each reconciled level is the union of contributions that route
/// through different verdicts, so the same prefix can never arrive from both
/// inputs. A duplicate key means a routing bug; debug builds panic, release
/// builds keep the left item.
pub fn merge_disjoint<'a, L, R, A, E, K, F>(
    left: L,
    right: R,
    key: F,
) -> impl Stream<Item = Result<A, E>> + Send + 'a
where
    L: Stream<Item = Result<A, E>> + Send + 'a,
    R: Stream<Item = Result<A, E>> + Send + 'a,
    F: Fn(&A) -> K + Clone + Send + 'a,
    K: Ord,
    A: Send + 'a,
    E: Send + 'a,
{
    merge_join_by(left, right, key.clone(), key).map(|item| {
        item.map(|cell| match cell {
            EitherOrBoth::Left(a) | EitherOrBoth::Right(a) => a,
            EitherOrBoth::Both(a, _) => {
                debug_assert!(
                    false,
                    "merge_disjoint: the same key arrived from both inputs"
                );
                a
            }
        })
    })
}

#[cfg(test)]
mod tests;
