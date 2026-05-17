use imbl::OrdMap;
use itertools::Itertools;

use crate::tree::typed::{
    Node, Prefix,
    height::{Height, Pred, Root, S},
};

/// Create a new [`Levels`] from the root of a tree.
pub fn levels<P, T>(root: Option<Node<P, T, Root>>) -> Top<P, T>
where
    P: Clone + Ord + AsRef<[u8]>,
{
    Top {
        root: OrdMap::from_iter(root.map(|root| (Prefix::new(), root))),
    }
}

/// An abstract type which represents a multi-zipper into a tree.
pub trait Levels: Default + Clone + sealed::Sealed {
    /// The party type of the underlying nodes.
    type Party: Clone + Ord + AsRef<[u8]>;

    /// The message type of the underlying nodes.
    type Message;

    /// The height of the bottom-most level.
    type Height: Height;

    /// Collapse a [`Levels`] back to a node by folding all the levels together.
    fn collapse(self) -> Option<Node<Self::Party, Self::Message, Root>>;

    /// Get an immutable reference to the bottom-most level.
    #[allow(clippy::type_complexity)]
    fn level(
        &self,
    ) -> &OrdMap<Prefix<Self::Height>, Node<Self::Party, Self::Message, Self::Height>>;

    /// Get a mutable reference to the bottom-most level.
    #[allow(clippy::type_complexity)]
    fn level_mut(
        &mut self,
    ) -> &mut OrdMap<Prefix<Self::Height>, Node<Self::Party, Self::Message, Self::Height>>;

    /// Tack a new level onto the bottom of this [`Levels`], decreasing its height by one.
    ///
    /// This can only be called on a [`Levels`] whose height is more than zero.
    fn down<H>(
        self,
        below: OrdMap<Prefix<H>, Node<Self::Party, Self::Message, H>>,
    ) -> Below<H, Self>
    where
        S<H>: Height,
        H: Height,
        Self: Levels<Height = S<H>> + Sized,
    {
        Below {
            above: self,
            here: below,
        }
    }
}

pub struct Top<P, T>
where
    P: Clone + Ord + AsRef<[u8]>,
{
    root: OrdMap<Prefix<Root>, Node<P, T, Root>>,
}

impl<P: Clone + Ord + AsRef<[u8]>, T> Clone for Top<P, T> {
    fn clone(&self) -> Self {
        Self {
            root: self.root.clone(),
        }
    }
}

impl<P, T> Default for Top<P, T>
where
    P: Clone + Ord + AsRef<[u8]>,
{
    fn default() -> Self {
        Self {
            root: Default::default(),
        }
    }
}

impl<P, T> Levels for Top<P, T>
where
    P: Clone + Ord + AsRef<[u8]>,
{
    type Party = P;
    type Message = T;
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

pub struct Below<H, A>
where
    A: Levels<Height = S<H>>,
    H: Height,
{
    here: OrdMap<Prefix<H>, Node<A::Party, A::Message, H>>,
    above: A,
}

impl<H, A> Clone for Below<H, A>
where
    A: Levels<Height = S<H>>,
    H: Height,
{
    fn clone(&self) -> Self {
        Self {
            here: self.here.clone(),
            above: self.above.clone(),
        }
    }
}

impl<H, A> Default for Below<H, A>
where
    A: Levels<Height = S<H>>,
    H: Height,
{
    fn default() -> Self {
        Self {
            here: Default::default(),
            above: Default::default(),
        }
    }
}

impl<H, A> Levels for Below<H, A>
where
    A: Levels<Height = S<H>>,
    S<H>: Height,
    H: Height,
{
    type Party = A::Party;
    type Message = A::Message;
    type Height = <A::Height as Pred>::Pred;

    fn collapse(mut self) -> Option<Node<A::Party, A::Message, Root>> {
        let above = self.above.level_mut();

        // Pop each child's prefix to get its radix and parent prefix. OrdMap
        // iteration is sorted, so siblings are adjacent.
        let siblings = self.here.into_iter().map(|(prefix, node)| {
            let (parent_prefix, radix) = prefix.pop();
            (parent_prefix, radix, node)
        });

        // Group siblings so each parent is deconstructed and reconstructed
        // exactly once.
        for (parent_prefix, group) in &siblings.chunk_by(|(pp, _, _)| *pp) {
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

    fn level(&self) -> &OrdMap<Prefix<Self::Height>, Node<A::Party, A::Message, Self::Height>> {
        &self.here
    }

    fn level_mut(
        &mut self,
    ) -> &mut OrdMap<Prefix<Self::Height>, Node<A::Party, A::Message, Self::Height>> {
        &mut self.here
    }
}

mod sealed {
    use super::{Below, Height, Levels, Pred, S, Top};

    pub trait Sealed {}
    impl<P: Clone + Ord + AsRef<[u8]>, T> Sealed for Top<P, T> {}
    impl<A: Levels<Height = S<H>> + Sealed, H: Height> Sealed for Below<H, A> where A::Height: Pred {}
}

#[cfg(test)]
mod test;
