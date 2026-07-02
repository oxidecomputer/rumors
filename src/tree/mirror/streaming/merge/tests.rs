//! The streaming [`merge`](super::merge) must produce exactly the sequence
//! [`itertools::Itertools::merge_join_by`] produces on the same two ascending
//! inputs.

use std::convert::Infallible;

use futures::stream::{self, StreamExt, TryStreamExt};
use itertools::Itertools;
use proptest::collection::btree_map;
use proptest::prelude::*;

use super::merge;

/// A sorted, duplicate-free key/value list, the shape `merge` requires of
/// each side. `BTreeMap` gives ascending, unique keys for free.
fn ascending() -> impl Strategy<Value = Vec<(u8, i32)>> {
    btree_map(any::<u8>(), any::<i32>(), 0..=16).prop_map(|m| m.into_iter().collect())
}

proptest! {
    /// Streamed and iterator merge-joins agree item for item, keyed on `u8`.
    #[test]
    fn agrees_with_itertools(left in ascending(), right in ascending()) {
        let streamed = pollster::block_on(
            merge(
                stream::iter(left.clone()).map(Ok::<_, Infallible>),
                stream::iter(right.clone()).map(Ok::<_, Infallible>),
            )
            .try_collect::<Vec<_>>(),
        )
        .unwrap_or_else(|e| match e {});

        let expected: Vec<_> = left
            .into_iter()
            .merge_join_by(right, |&(a, _), &(b, _)| a.cmp(&b))
            .collect();

        prop_assert_eq!(streamed, expected);
    }
}
