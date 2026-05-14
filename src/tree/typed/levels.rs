use imbl::OrdMap;
use itertools::Itertools;

use crate::tree::typed::{
    Node, Prefix,
    height::{Height, Root, S},
};

/// Create a new [`Levels`] from the root of a tree.
pub fn levels<P, T>(root: Option<Node<P, T, Root>>) -> Top<P, T>
where
    P: Clone + Ord + AsRef<[u8]>,
    T: Clone,
{
    Top {
        root: OrdMap::from_iter(root.map(|root| (Prefix::new(), root))),
    }
}

/// An abstract type which represents a multi-zipper into a tree.
///
/// The concrete types which provide the functionality are un-nameable
/// outside this module.
pub trait Levels<P, T>: Clone + sealed::Sealed {
    /// The height of the bottom-most level.
    type Height: Height;

    /// Collapse a [`Levels`] back to a node by folding all the levels together.
    fn collapse(self) -> Option<Node<P, T, Root>>
    where
        P: Clone + Ord + AsRef<[u8]>;

    /// Get an immutable reference to the bottom-most level.
    fn level(&self) -> &OrdMap<Prefix<Self::Height>, Node<P, T, Self::Height>>
    where
        P: Clone + Ord + AsRef<[u8]>;

    /// Get a mutable reference to the bottom-most level.
    fn level_mut(&mut self) -> &mut OrdMap<Prefix<Self::Height>, Node<P, T, Self::Height>>
    where
        P: Clone + Ord + AsRef<[u8]>;

    /// Tack a new level onto the bottom of this [`Levels`], decreasing its height by one.
    ///
    /// This can only be called on a [`Levels`] whose height is more than zero.
    fn down<H>(self, below: OrdMap<Prefix<H>, Node<P, T, H>>) -> Below<P, T, H, Self>
    where
        S<H>: Height,
        H: Height,
        P: Clone + Ord + AsRef<[u8]>,
        T: Clone,
        Self: Levels<P, T, Height = S<H>> + Sized,
    {
        Below {
            above: self,
            here: below,
        }
    }
}

#[derive(Clone)]
pub struct Top<P, T>
where
    P: Clone + Ord + AsRef<[u8]>,
{
    root: OrdMap<Prefix<Root>, Node<P, T, Root>>,
}

impl<P, T> Levels<P, T> for Top<P, T>
where
    T: Clone,
    P: Clone + Ord + AsRef<[u8]>,
{
    type Height = Root;

    fn collapse(mut self) -> Option<Node<P, T, Root>> {
        self.root.remove(&Prefix::new())
    }

    fn level(&self) -> &OrdMap<Prefix<Self::Height>, Node<P, T, Self::Height>> {
        &self.root
    }

    fn level_mut(&mut self) -> &mut OrdMap<Prefix<Self::Height>, Node<P, T, Self::Height>> {
        &mut self.root
    }
}

#[derive(Clone)]
pub struct Below<P, T, H, A>
where
    Self: Levels<P, T, Height = H>,
    H: Height,
    P: Clone + Ord + AsRef<[u8]>,
{
    here: OrdMap<Prefix<H>, Node<P, T, H>>,
    above: A,
}

impl<P, T, H, A> Levels<P, T> for Below<P, T, H, A>
where
    A: Levels<P, T, Height = S<H>>,
    S<H>: Height,
    H: Height,
    T: Clone,
    P: Clone + Ord + AsRef<[u8]>,
{
    type Height = H;

    fn collapse(mut self) -> Option<Node<P, T, Root>> {
        let above = self.above.level_mut();

        // Pop each child's prefix to get its radix and parent prefix. OrdMap
        // iteration is sorted, so siblings are adjacent.
        let siblings = self.here.into_iter().map(|(prefix, node)| {
            let (radix, parent_prefix) = prefix.pop();
            (parent_prefix, radix, node)
        });

        // Group siblings so each parent is deconstructed and reconstructed
        // exactly once.
        for (parent_prefix, group) in &siblings.chunk_by(|(pp, _, _)| pp.clone()) {
            // Disassemble the existing parent (if any) into its children
            let mut children = above
                .remove(&parent_prefix)
                .map(Node::into_children)
                .unwrap_or_default();

            // Merge all siblings in this group into the children map
            for (_, radix, node) in group {
                children.insert(radix, node);
            }

            // Reconstruct the parent and insert it into the level above
            if let Some(parent) = Node::branch(children) {
                above.insert(parent_prefix, parent);
            }
        }

        // Collapse the level above, recursively
        self.above.collapse()
    }

    fn level(&self) -> &OrdMap<Prefix<Self::Height>, Node<P, T, Self::Height>> {
        &self.here
    }

    fn level_mut(&mut self) -> &mut OrdMap<Prefix<Self::Height>, Node<P, T, Self::Height>> {
        &mut self.here
    }
}

mod sealed {
    use super::{Below, Height, Levels, S, Top};

    pub trait Sealed {}
    impl<P: Clone + Ord + AsRef<[u8]>, T> Sealed for Top<P, T> {}
    impl<P: Clone + Ord + AsRef<[u8]>, H: Height, T: Clone, A: Levels<P, T, Height = S<H>> + Sealed>
        Sealed for Below<P, T, H, A>
    where
        S<H>: Height,
    {
    }
}

#[cfg(test)]
mod test;
