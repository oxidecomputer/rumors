//! The marked [`fold_parents`] must group exactly the parents of its real
//! children, translate watermarks one level up, and flush a covered open
//! group without awaiting the next real child.

use std::collections::BTreeSet;
use std::convert::Infallible;
use std::pin::pin;

use futures::FutureExt;
use futures::stream::{self, StreamExt, TryStreamExt};
use proptest::prelude::*;

use crate::Version;
use crate::message::Message;
use crate::tree::arb::nth_party;
use crate::tree::typed::{
    self, Hash, Path, Prefix,
    height::{S, Z},
};

use super::{Backend, Local, fold_parents};

/// A distinct leaf per call: content is irrelevant to grouping; the versions
/// only need to differ so the resulting parents' hashes do.
fn leaf(version: &mut Version) -> typed::Node<(), Z> {
    version.tick(&nth_party(0));
    typed::Node::leaf(version.clone(), Message::new(()))
}

/// The height-`Z` key with `parent` as its first byte and `radix` as its
/// last, zeros between: children share a parent iff their first bytes agree.
fn key(parent: u8, radix: u8) -> Prefix<Z> {
    let mut bytes = [0u8; 32];
    bytes[0] = parent;
    bytes[31] = radix;
    Path::from(bytes).into()
}

/// The parent-height prefix of [`key`]`(parent, _)`.
fn parent_prefix(parent: u8) -> Prefix<S<Z>> {
    key(parent, 0).pop().0
}

/// Drive the marked fold of an in-memory child stream to completion.
#[allow(clippy::type_complexity)]
fn fold(
    children: Vec<(Prefix<Z>, Option<typed::Node<(), Z>>)>,
) -> Vec<(Prefix<S<Z>>, Option<typed::Node<(), S<Z>>>)> {
    pollster::block_on(
        fold_parents::<Local, (), Z>(
            Local,
            stream::iter(children.into_iter().map(Ok::<_, Infallible>)),
        )
        .try_collect::<Vec<_>>(),
    )
    .unwrap_or_else(|e| match e {})
}

/// Build the expected parent of an all-real radix group directly through the
/// backend, bypassing the fold under test.
fn parent_of(
    prefix: Prefix<S<Z>>,
    children: Vec<(u8, typed::Node<(), Z>)>,
) -> typed::Node<(), S<Z>> {
    pollster::block_on(
        Local.parent(
            prefix,
            children
                .into_iter()
                .map(|(radix, child)| (radix, Some(child)))
                .collect(),
        ),
    )
    .unwrap_or_else(|e| match e {})
    .expect("a non-empty all-real group always constructs its parent")
}

/// Reduce an output level to comparable shape: prefixes with node hashes.
#[allow(clippy::type_complexity)]
fn shape(
    items: Vec<(Prefix<S<Z>>, Option<typed::Node<(), S<Z>>>)>,
) -> Vec<(Prefix<S<Z>>, Option<Hash>)> {
    items
        .into_iter()
        .map(|(prefix, node)| (prefix, node.map(|node| node.hash())))
        .collect()
}

proptest! {
    /// Stripping watermarks from the marked fold's output yields exactly
    /// the parents of the real children.
    ///
    /// Watermark interleaving changes nothing about grouping, and the full
    /// output — watermarks included — is strictly ascending by prefix.
    #[test]
    fn equivalent_to_unmarked_grouping(
        real_keys in proptest::collection::btree_set((0u8..6, any::<u8>()), 0..=40),
        watermark_keys in proptest::collection::btree_set((0u8..6, any::<u8>()), 0..=16),
    ) {
        // Real and watermark keys must be distinct (a real item implies its
        // own key's passage), and one merged ascending input carries both.
        let watermark_keys: BTreeSet<_> =
            watermark_keys.difference(&real_keys).copied().collect();

        let mut version = Version::new();
        let mut input = Vec::new();
        let mut expected_groups: Vec<(u8, Vec<(u8, typed::Node<(), Z>)>)> = Vec::new();
        let mut items: Vec<((u8, u8), bool)> = real_keys.iter().map(|&k| (k, true)).collect();
        items.extend(watermark_keys.iter().map(|&k| (k, false)));
        items.sort();
        for ((parent, radix), real) in items {
            if real {
                let node = leaf(&mut version);
                input.push((key(parent, radix), Some(node.clone())));
                match expected_groups.last_mut() {
                    Some((current, group)) if *current == parent => group.push((radix, node)),
                    _ => expected_groups.push((parent, vec![(radix, node)])),
                }
            } else {
                input.push((key(parent, radix), None));
            }
        }

        let output = fold(input);
        for pair in output.windows(2) {
            prop_assert!(pair[0].0 < pair[1].0, "output must ascend strictly");
        }

        let stripped: Vec<(Prefix<S<Z>>, Hash)> = output
            .into_iter()
            .filter_map(|(prefix, node)| node.map(|node| (prefix, node.hash())))
            .collect();
        let expected: Vec<(Prefix<S<Z>>, Hash)> = expected_groups
            .into_iter()
            .map(|(parent, group)| {
                let prefix = parent_prefix(parent);
                (prefix, parent_of(prefix, group).hash())
            })
            .collect();
        prop_assert_eq!(stripped, expected);
    }
}

/// A covering watermark flushes the open group immediately: the parent is
/// yielded without awaiting the next real child.
///
/// This is the third flush trigger — the one a one-sided region depends
/// on, where no next real child is coming — pinned here against a
/// permanently silent tail.
#[test]
fn covering_watermark_flushes_without_next_child() {
    let mut version = Version::new();
    let (a, b) = (leaf(&mut version), leaf(&mut version));
    let input = stream::iter(vec![
        Ok::<_, Infallible>((key(3, 1), Some(a.clone()))),
        Ok((key(3, 2), Some(b.clone()))),
        Ok((key(3, 0xff), None)),
    ])
    .chain(stream::pending());

    let mut folded = pin!(fold_parents::<Local, (), Z>(Local, input));
    let (prefix, node) = folded
        .next()
        .now_or_never()
        .expect("the covered group must flush without further input")
        .expect("the fold must not end")
        .unwrap_or_else(|e| match e {});
    assert_eq!(prefix, parent_prefix(3));
    assert_eq!(
        node.map(|node| node.hash()),
        Some(parent_of(parent_prefix(3), vec![(1, a), (2, b)]).hash()),
    );
    // The maximal-child watermark's guarantee is the parent itself, and the
    // flush just carried that key: no echo follows, the fold parks.
    assert!(folded.next().now_or_never().is_none());
}

/// A maximal-child watermark covers its whole parent: no sibling can follow
/// `q·0xff`, so the guarantee is `q` itself.
#[test]
fn max_child_watermark_covers_its_parent() {
    let output = fold(vec![(key(3, 0xff), None)]);
    assert_eq!(shape(output), vec![(parent_prefix(3), None)]);
}

/// A non-maximal child watermark steps back to the parent's predecessor:
/// later siblings may still arrive, so the parent itself is not yet covered.
#[test]
fn low_child_watermark_guarantees_the_predecessor() {
    let output = fold(vec![(key(3, 7), None)]);
    assert_eq!(
        shape(output),
        vec![(parent_prefix(3).pred().unwrap(), None)],
    );
}

/// At the all-zeros child there is nothing below to guarantee: the watermark
/// translates to silence, not a panic.
#[test]
fn all_zeros_watermark_translates_to_silence() {
    assert!(fold(vec![(key(0, 0), None)]).is_empty());
}

/// A watermark that does not cover the open parent leaves its group open:
/// the group still flushes complete — including children arriving after the
/// watermark — when its real completion trigger comes.
#[test]
fn uncovering_watermark_leaves_the_group_open() {
    let mut version = Version::new();
    let (a, b) = (leaf(&mut version), leaf(&mut version));
    let output = fold(vec![
        (key(3, 1), Some(a.clone())),
        // Guarantees only the parent's predecessor: below the open group.
        (key(3, 5), None),
        (key(3, 7), Some(b.clone())),
    ]);
    assert_eq!(
        shape(output),
        vec![
            (parent_prefix(3).pred().unwrap(), None),
            (
                parent_prefix(3),
                Some(parent_of(parent_prefix(3), vec![(1, a), (7, b)]).hash()),
            ),
        ],
    );
}

/// Successive watermarks translating to one parent-level guarantee coalesce:
/// the duplicate is suppressed, keeping the output strictly ascending.
#[test]
fn duplicate_guarantees_coalesce() {
    let output = fold(vec![(key(3, 4), None), (key(3, 7), None)]);
    assert_eq!(
        shape(output),
        vec![(parent_prefix(3).pred().unwrap(), None)],
    );
}
