//! The marked [`merge_disjoint`] union must interleave real items and
//! watermarks by key, hold a consumed-but-unemitted watermark as floor
//! memory, and resolve equal-key meetings by the marked-stream contract.

use std::convert::Infallible;
use std::pin::pin;

use futures::FutureExt;
use futures::stream::{self, Stream, StreamExt, TryStreamExt};
use proptest::prelude::*;

use super::merge_disjoint;

type Item = Result<(u8, Option<char>), Infallible>;

/// An in-memory marked stream: real items and watermarks, pre-sorted by key.
fn marked(items: Vec<(u8, Option<char>)>) -> impl Stream<Item = Item> + Send {
    stream::iter(items.into_iter().map(Ok))
}

/// Drive the union of two in-memory marked streams to completion.
fn union(left: Vec<(u8, Option<char>)>, right: Vec<(u8, Option<char>)>) -> Vec<(u8, Option<char>)> {
    pollster::block_on(merge_disjoint(marked(left), marked(right)).try_collect::<Vec<_>>())
        .unwrap_or_else(|e| match e {})
}

proptest! {
    /// Stripping watermarks from the marked union yields exactly the sorted
    /// union of the two sides' real items (whose key sets are disjoint).
    ///
    /// The full output — watermarks included — is strictly ascending, so no
    /// key is ever emitted twice and no watermark echoes a real item.
    #[test]
    fn equivalent_to_real_union_and_strictly_ascending(
        spec in proptest::collection::btree_map(any::<u8>(), 0u8..7, 0..=64),
    ) {
        // Per key, one of the seven legal cross-side meetings: a real item
        // on one side, a watermark on either or both, or a real item meeting
        // the other side's watermark. Two reals at one key are the routing
        // bug pinned separately below.
        let mut left = Vec::new();
        let mut right = Vec::new();
        let mut reals = Vec::new();
        for (&key, &combo) in &spec {
            match combo {
                0 => {
                    left.push((key, Some('l')));
                    reals.push((key, 'l'));
                }
                1 => {
                    right.push((key, Some('r')));
                    reals.push((key, 'r'));
                }
                2 => left.push((key, None)),
                3 => right.push((key, None)),
                4 => {
                    left.push((key, None));
                    right.push((key, None));
                }
                5 => {
                    left.push((key, Some('l')));
                    right.push((key, None));
                    reals.push((key, 'l'));
                }
                _ => {
                    left.push((key, None));
                    right.push((key, Some('r')));
                    reals.push((key, 'r'));
                }
            }
        }

        let output = union(left, right);
        for pair in output.windows(2) {
            prop_assert!(pair[0].0 < pair[1].0, "output must ascend strictly");
        }
        let stripped: Vec<(u8, char)> = output
            .iter()
            .filter_map(|&(key, value)| value.map(|value| (key, value)))
            .collect();
        prop_assert_eq!(stripped, reals);
    }
}

/// Watermarks ride the ordinary comparison arms: they interleave with the
/// other side's real items by key, unaltered.
#[test]
fn watermarks_interleave_by_key() {
    assert_eq!(
        union(vec![(3, None), (9, Some('x'))], vec![(5, Some('b'))]),
        vec![(3, None), (5, Some('b')), (9, Some('x'))],
    );
}

/// A watermark meeting the other side's real item at one key is legal — the
/// floor promised only its own side's silence at-or-below it — and the real
/// item takes the slot, its key's passage subsuming the floor.
#[test]
fn real_wins_the_equal_key_meeting() {
    assert_eq!(
        union(vec![(5, Some('a'))], vec![(5, None)]),
        vec![(5, Some('a'))],
    );
    assert_eq!(
        union(vec![(5, None)], vec![(5, Some('b'))]),
        vec![(5, Some('b'))],
    );
}

/// Two watermarks at one key collapse to a single watermark.
#[test]
fn equal_watermarks_collapse() {
    assert_eq!(union(vec![(5, None)], vec![(5, None)]), vec![(5, None)]);
}

/// The same key arriving *real* from both inputs is a routing bug: the
/// reconciled contributions are disjoint by construction.
#[test]
#[should_panic(expected = "arrived real from both inputs")]
fn equal_real_keys_are_a_routing_bug() {
    union(vec![(5, Some('a'))], vec![(5, Some('b'))]);
}

/// A held watermark head is floor memory: a real item below it on the other
/// side releases immediately, without the watermark's side speaking again.
///
/// Past the floor the merge parks awaiting new information rather than
/// inventing any. This is the release the deadlock fix leans on, pinned
/// here against a permanently silent counterparty.
#[test]
fn held_watermark_releases_lower_real_items() {
    let left = marked(vec![(5, Some('a'))]).chain(stream::pending());
    let right = marked(vec![(7, None)]).chain(stream::pending());
    let mut merged = pin!(merge_disjoint(left, right));

    assert_eq!(
        merged.next().now_or_never(),
        Some(Some(Ok((5, Some('a'))))),
        "the real item must release against the held floor, not await the silent side",
    );
    assert_eq!(
        merged.next().now_or_never(),
        None,
        "past the floor the merge must park, not emit",
    );
}
