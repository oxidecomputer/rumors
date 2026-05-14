use proptest::collection::vec;
use proptest::prelude::*;

use crate::tree::traverse::{Action, act};
use crate::tree::typed::height::Root;
use crate::tree::typed::{Node, Path};
use crate::{Message, Version};

/// Build a typed root tree by inserting random leaves via `act`.
///
/// The `party` parameter controls which party's version vector the inserts
/// are attributed to, making it possible to generate two trees with
/// independent version histories.
pub fn arb_root_tree(
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

/// Like [`arb_root_tree`], but guarantees the tree is non-empty.
///
/// Pass a range starting at 1 (e.g. `1..=8`) to avoid filter rejections.
pub fn arb_root_node(
    party: impl Into<String>,
    leaves: impl Into<proptest::collection::SizeRange>,
) -> BoxedStrategy<Node<String, (), Root>> {
    arb_root_tree(party, leaves)
        .prop_filter_map("non-empty tree", |t| t)
        .boxed()
}
