//! Prefix-ordered streaming merge-join.
//!
//! The streaming analog of [`itertools::Itertools::merge_join_by`], specialized
//! to this module's one shape: streams of key-value pairs, ascending by key
//! (here always a prefix), with no duplicate keys within either stream.
//! [`merge`] walks both in lockstep and emits one [`EitherOrBoth`] per distinct
//! key in ascending order — `Left` for a key only the left stream carries,
//! `Right` for one only the right does, `Both` where they coincide.
//!
//! This is the join at the heart of the mirror's asymmetry matrix (our frontier
//! nodes against the counterparty's `uncertain` hashes), factored out because
//! `futures` ships no equivalent and `itertools`' operates only on iterators.
//!
//! Memory is a single item of lookahead per side: when the two keys differ, the
//! larger item is held back for the next round rather than dropped, so nothing
//! is buffered beyond the two heads.

use std::pin::pin;

use async_stream::try_stream;
use futures::stream::{Stream, StreamExt};
use itertools::EitherOrBoth;

/// One merged cell: the key's pair from the left stream, the right, or both.
type Cell<K, A, B> = EitherOrBoth<(K, A), (K, B)>;

/// Merge-join two ascending-by-key, fallible streams of key-value pairs into a
/// stream of [`EitherOrBoth`], propagating the first error from either side.
///
/// The caller guarantees each stream is strictly ascending by its pairs' keys;
/// the output is then ascending too, one item per distinct key.
pub fn merge<'a, L, R, K, A, B, E>(
    left: L,
    right: R,
) -> impl Stream<Item = Result<Cell<K, A, B>, E>> + Send + 'a
where
    L: Stream<Item = Result<(K, A), E>> + Send + 'a,
    R: Stream<Item = Result<(K, B), E>> + Send + 'a,
    K: Ord + Send + 'a,
    A: Send + 'a,
    B: Send + 'a,
    E: Send + 'a,
{
    try_stream! {
        // Fused so that an exhausted side answers `None` immediately forever:
        // the loop below keeps polling both sides until each has ended.
        let mut left = pin!(left.fuse());
        let mut right = pin!(right.fuse());

        // One item of lookahead per side. On a key mismatch the larger head is
        // put back here rather than consumed, so each item is compared once.
        let mut head_left: Option<(K, A)> = None;
        let mut head_right: Option<(K, B)> = None;

        loop {
            // When both heads are empty, fill them concurrently. The mirror
            // feeds this join from channels whose producers are independently
            // scheduled futures, so a sequential fill would be correct, but it
            // would serialize the join on whichever side happens to be awaited
            // first, adding a round of producer latency per item. Once one head
            // is held, only the empty side is awaited: the held side's producer
            // needs nothing from us until its head is consumed.
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
                    match a.0.cmp(&b.0) {
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

/// Merge two ascending-by-key, fallible streams of the *same* pair type into
/// one ascending stream, requiring the key sets to be disjoint.
///
/// This is the union half of [`merge`], used by the mirror's upward
/// reassembly: each reconciled level is the union of contributions that route
/// through different verdicts, so the same prefix can never arrive from both
/// inputs. A duplicate key means a routing bug; debug builds panic, release
/// builds keep the left item.
pub fn merge_disjoint<'a, L, R, K, A, E>(
    left: L,
    right: R,
) -> impl Stream<Item = Result<(K, A), E>> + Send + 'a
where
    L: Stream<Item = Result<(K, A), E>> + Send + 'a,
    R: Stream<Item = Result<(K, A), E>> + Send + 'a,
    K: Ord + Send + 'a,
    A: Send + 'a,
    E: Send + 'a,
{
    merge(left, right).map(|item| {
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
