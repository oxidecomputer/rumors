use proptest::collection::vec;
use proptest::prelude::*;

use crate::tree::arb::arb_root_node;
use crate::tree::typed::height::{Height, Root, S, Z};
use crate::tree::typed::{Level, Node};

use super::Levels;

/// Polymorphic-recursive random descent through the height levels.
///
/// At each non-leaf height, each node is either kept in place or
/// exploded into its children at the next lower level, driven by
/// the boolean iterator. When the iterator is exhausted, all
/// remaining nodes stay (biasing toward shallow descent on shrink).
trait Descend: Height {
    fn descend_and_collapse<L>(
        levels: L,
        decisions: &mut impl Iterator<Item = bool>,
    ) -> Option<Node<L::Message, Root>>
    where
        L: Levels<Height = Self>,
        L::Message: Clone + Send + Sync;
}

impl Descend for Z {
    fn descend_and_collapse<L>(
        levels: L,
        _decisions: &mut impl Iterator<Item = bool>,
    ) -> Option<Node<L::Message, Root>>
    where
        L: Levels<Height = Self>,
        L::Message: Clone + Send + Sync,
    {
        levels.collapse()
    }
}

impl<H: Descend> Descend for S<H>
where
    S<H>: Height,
    H: Height,
{
    fn descend_and_collapse<L>(
        mut levels: L,
        decisions: &mut impl Iterator<Item = bool>,
    ) -> Option<Node<L::Message, Root>>
    where
        L: Levels<Height = Self>,
        L::Message: Clone + Send + Sync,
    {
        let current = std::mem::take(levels.level_mut());

        let mut stay = Level::default();
        let mut below: Level<L::Message, H> = Level::default();

        // `current` iterates in ascending prefix order, and each node's children
        // ascend by radix, so both partitions are built strictly ascending —
        // `push` keeps them sorted.
        for (prefix, node) in current {
            if decisions.next().unwrap_or(false) {
                for (byte, child) in node.into_children() {
                    below.push(prefix.push(byte), child);
                }
            } else {
                stay.push(prefix, node);
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
        tree in arb_root_node(0, 0..=16),
        decisions in vec(any::<bool>(), 0..=512),
    ) {
        let before = tree.clone();
        let after = Descend::descend_and_collapse(Node::levels(tree), &mut decisions.into_iter());
        prop_assert_eq!(before, after);
    }
}
