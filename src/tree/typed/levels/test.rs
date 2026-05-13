use imbl::OrdMap;
use proptest::collection::vec;
use proptest::prelude::*;

use crate::tree::traverse::{Action, act};
use crate::tree::typed::height::{Height, Root, S, Z};
use crate::tree::typed::{Node, Path, Prefix};
use crate::{Message, Version};

use super::Levels;

/// Build a typed root tree by inserting random leaves via `act`.
fn arb_root_tree() -> BoxedStrategy<Option<Node<String, (), Root>>> {
    vec(any::<[u8; 32]>(), 0..=16)
        .prop_map(|paths| {
            let actions: Vec<_> = paths
                .into_iter()
                .enumerate()
                .map(|(i, bytes)| {
                    let path = Path::from(bytes);
                    let version = Version::from(("a".to_string(), i as u64 + 1));
                    (path, version, Action::Insert(Message::new(())))
                })
                .collect();
            act(None, actions)
        })
        .boxed()
}

/// Polymorphic-recursive random descent through the height levels.
///
/// At each non-leaf height, each node is either kept in place or
/// exploded into its children at the next lower level, driven by
/// the boolean iterator. When the iterator is exhausted, all
/// remaining nodes stay (biasing toward shallow descent on shrink).
trait Descend: Height {
    fn descend_and_collapse<P, T>(
        levels: impl Levels<P, T, Height = Self>,
        decisions: &mut impl Iterator<Item = bool>,
    ) -> Option<Node<P, T, Root>>
    where
        P: Clone + Ord + AsRef<[u8]> + Send + Sync,
        T: Clone + Send + Sync;
}

impl Descend for Z {
    fn descend_and_collapse<P, T>(
        levels: impl Levels<P, T, Height = Z>,
        _decisions: &mut impl Iterator<Item = bool>,
    ) -> Option<Node<P, T, Root>>
    where
        P: Clone + Ord + AsRef<[u8]> + Send + Sync,
        T: Clone + Send + Sync,
    {
        levels.collapse()
    }
}

impl<H: Descend> Descend for S<H>
where
    S<H>: Height,
    H: Height,
{
    fn descend_and_collapse<P, T>(
        mut levels: impl Levels<P, T, Height = S<H>>,
        decisions: &mut impl Iterator<Item = bool>,
    ) -> Option<Node<P, T, Root>>
    where
        P: Clone + Ord + AsRef<[u8]> + Send + Sync,
        T: Clone + Send + Sync,
    {
        let current = std::mem::take(levels.level_mut());

        let mut stay = OrdMap::new();
        let mut below: OrdMap<Prefix<H>, Node<P, T, H>> = OrdMap::new();

        for (prefix, node) in current {
            if decisions.next().unwrap_or(false) {
                for (byte, child) in node.into_children() {
                    below.insert(prefix.clone().push(byte), child);
                }
            } else {
                stay.insert(prefix, node);
            }
        }

        *levels.level_mut() = stay;
        let deeper = levels.down(below);
        H::descend_and_collapse(deeper, decisions)
    }
}

proptest! {
    /// Randomly partitioning a tree's nodes across levels via `down`,
    /// then folding back via `collapse`, recovers the original tree.
    #[test]
    fn collapse_inverts_down(
        tree in arb_root_tree(),
        decisions in vec(any::<bool>(), 0..=512),
    ) {
        let before = tree.clone();
        if let Some(tree) = tree {
            let after = Descend::descend_and_collapse(tree.levels(), &mut decisions.into_iter());
            prop_assert_eq!(before, after);
        }
    }
}
