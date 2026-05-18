use proptest::collection::vec;
use proptest::prelude::*;

use crate::tree::traverse::{Action, act};
use crate::tree::typed::height::Root;
use crate::tree::typed::{Node, Path};
use crate::{message::Message, version::Version};

/// Build a typed root tree by inserting random leaves via `act`.
///
/// The `party` parameter controls which party's version vector the inserts
/// are attributed to, making it possible to generate two trees with
/// independent version histories.
pub fn arb_root_node(
    party: impl Into<String>,
    leaves: impl Into<proptest::collection::SizeRange>,
) -> BoxedStrategy<Option<Node<String, (), Root>>> {
    let party = party.into();
    vec(any::<[u8; 32]>(), leaves)
        .prop_map(move |paths| {
            let actions: Vec<_> = paths
                .into_iter()
                .enumerate()
                .map(|(i, bytes)| {
                    let path = Path::from(bytes);
                    let version = Version::from((party.clone(), i as u64 + 1));
                    (path, version, Action::Insert(Message::new(())))
                })
                .collect();
            act(None, actions)
        })
        .boxed()
}

/// Build a [`crate::tree::Root`] by lifting [`arb_root_node`]: a populated
/// node maps to [`Root::Populated`], and the empty case maps to
/// [`Root::Empty`] at an arbitrary version vector (so the empty branch
/// still exercises non-default versions).
pub fn arb_tree_root(
    party: impl Into<String>,
    leaves: impl Into<proptest::collection::SizeRange>,
) -> BoxedStrategy<crate::tree::Root<String, ()>> {
    (arb_root_node(party, leaves), arb_version())
        .prop_map(|(node, extra)| {
            // The wrapper version must be a causal upper bound on every
            // version inside the contained tree; the mirror protocol reads
            // it as authoritative for "what we have seen." Fold the root
            // node's own version in so a generated `Root` always satisfies
            // that invariant, regardless of `extra`.
            let inner = node.as_ref().map(Node::version).cloned().unwrap_or_default();
            crate::tree::Root {
                version: extra | inner,
                root: node,
            }
        })
        .boxed()
}

/// Build an arbitrary [`Version<String>`] from a small collection of
/// `(party, count)` pairs. Duplicate parties are merged by `Version::new`'s
/// pointwise max.
fn arb_version() -> BoxedStrategy<Version<String>> {
    vec(("[a-z]{1,4}", any::<u64>()), 0..4)
        .prop_map(|entries| {
            Version::new(
                entries
                    .into_iter()
                    .map(|(party, count)| Version::from((party, count))),
            )
        })
        .boxed()
}
