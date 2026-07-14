//! [`fold_parents`] must group an ascending child stream into exactly the
//! parents of its radix groups, flushing on prefix change and at input end.

use std::convert::Infallible;

use futures::stream::{self, TryStreamExt};
use proptest::prelude::*;

use crate::{
    Version,
    message::Message,
    tree::{
        arb::nth_party,
        mirror::streaming::{Backend, Local, convert::fold_parents},
        typed::{
            self, Hash, Path, Prefix,
            height::{S, Z},
        },
    },
};

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

/// Build the expected parent of a radix group directly through the backend,
/// bypassing the fold under test.
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

/// Drive the fold of an in-memory child stream to completion.
fn fold(children: Vec<(Prefix<Z>, typed::Node<(), Z>)>) -> Vec<(Prefix<S<Z>>, Hash)> {
    pollster::block_on(
        fold_parents::<Local, (), Z>(
            Local,
            stream::iter(children.into_iter().map(Ok::<_, Infallible>)),
        )
        .try_collect::<Vec<_>>(),
    )
    .unwrap_or_else(|e| match e {})
    .into_iter()
    .map(|(prefix, node)| (prefix, node.hash()))
    .collect()
}

proptest! {
    /// The fold's output is exactly the parents of the input's radix groups,
    /// in the input's (strictly ascending) prefix order.
    ///
    /// This includes the final group, which no prefix change follows: it
    /// must flush at input end.
    #[test]
    fn folds_to_exactly_the_group_parents(
        keys in proptest::collection::btree_set((0u8..6, any::<u8>()), 0..=40),
    ) {
        let mut version = Version::new();
        let mut input = Vec::new();
        let mut expected_groups: Vec<(u8, Vec<(u8, typed::Node<(), Z>)>)> = Vec::new();
        for (parent, radix) in keys {
            let node = leaf(&mut version);
            input.push((key(parent, radix), node.clone()));
            match expected_groups.last_mut() {
                Some((current, group)) if *current == parent => group.push((radix, node)),
                _ => expected_groups.push((parent, vec![(radix, node)])),
            }
        }

        let expected: Vec<(Prefix<S<Z>>, Hash)> = expected_groups
            .into_iter()
            .map(|(parent, group)| {
                let prefix = parent_prefix(parent);
                (prefix, parent_of(prefix, group).hash())
            })
            .collect();
        prop_assert_eq!(fold(input), expected);
    }
}
