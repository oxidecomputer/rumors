//! The streaming [`unknown`](super::unknown) prune must agree, node for node,
//! with the materialized [`Unknown`](crate::tree::traverse::unknown::Unknown)
//! oracle it mirrors.

use std::convert::Infallible;

use futures::stream::{self, TryStreamExt};
use proptest::collection::vec;
use proptest::prelude::*;

use crate::Version;
use crate::message::Message;
use crate::tree::arb::nth_party;
use crate::tree::traverse::unknown::Unknown;
use crate::tree::traverse::{Action, act};
use crate::tree::typed::{self, Path, Prefix, height::Root};

use super::super::Local;
use super::unknown;

/// Build a root from `flags_a.len()` party-0 leaves and `flags_b.len()` party-1
/// leaves, inserted with strictly ascending versions, plus a `known` version
/// that is the join of the leaf versions flagged `true`.
///
/// Flagged leaves fall causally at or before `known` (the prune must drop
/// them); the rest are concurrent with or after it (the prune must keep them).
/// Splitting leaves across two parties guarantees cross-party concurrency, so
/// the "floor concurrent, keep whole subtree" fast path is exercised alongside
/// the drop path.
fn tree_and_known(flags_a: &[bool], flags_b: &[bool]) -> (Option<typed::node::Root<()>>, Version) {
    let mut actions: Vec<(Path, Version, Action<()>)> = Vec::new();
    let mut known = Version::new();

    for (party_index, flags) in [(0, flags_a), (1, flags_b)] {
        let party = nth_party(party_index);
        let mut version = Version::new();
        for &flagged in flags {
            version.tick(&party);
            let message = Message::new(());
            let path = Path::for_leaf(&version, message.bytes());
            actions.push((path, version.clone(), Action::Insert(message)));
            if flagged {
                known |= version.clone();
            }
        }
    }

    (act(None, actions, |_| ()), known)
}

/// Collect the streaming prune of a single-rooted stream back into an optional
/// root, driving the in-memory stream to completion with a trivial executor.
fn stream_prune(
    root: Option<typed::node::Root<()>>,
    known: &Version,
) -> Option<typed::node::Root<()>> {
    let input = root.map(|node| Ok::<_, Infallible>((Prefix::new(), node)));
    let collected = pollster::block_on(
        unknown::<Local, (), Root>(&Local, known, stream::iter(input)).try_collect::<Vec<_>>(),
    )
    .unwrap_or_else(|e| match e {});
    collected.into_iter().next().map(|(_prefix, node)| node)
}

proptest! {
    /// The streamed prune and the materialized prune reconcile to the same
    /// tree (equal Merkle roots) for every tree and every `known` version.
    #[test]
    fn agrees_with_materialized_oracle(
        flags_a in vec(any::<bool>(), 0..=8),
        flags_b in vec(any::<bool>(), 0..=8),
    ) {
        let (root, known) = tree_and_known(&flags_a, &flags_b);

        let oracle = Unknown::unknown(root.clone(), &known);
        let streamed = stream_prune(root, &known);

        prop_assert_eq!(
            typed::Node::root_hash(&oracle),
            typed::Node::root_hash(&streamed),
        );
    }
}
