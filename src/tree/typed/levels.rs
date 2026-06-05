use std::collections::BTreeMap;

use itertools::Itertools;

use crate::tree::typed::{
    Node, Prefix,
    height::{Height, Pred, Root, S},
};

/// Create a new [`Levels`] from the root of a tree.
pub fn levels<T>(root: Option<Node<T, Root>>) -> Top<T>
where
{
    Top {
        root: BTreeMap::from_iter(root.map(|root| (Prefix::new(), root))),
    }
}

/// An abstract type which represents a multi-zipper into a tree.
pub trait Levels: Default + Clone + sealed::Sealed {
    /// The message type of the underlying nodes.
    ///
    /// `Send + Sync` is required because the traversal futures
    /// ([`Unknown::unknown`], [`Act::act`]) are declared as
    /// `-> impl Future + Send` so that the recursive `Box::pin` inside each
    /// inductive case can coerce to `Pin<Box<dyn Future + Send>>`. That
    /// coercion discharges the inner state machine's auto-trait check at each
    /// recursion site, terminating what would otherwise be a height-deep walk
    /// through the `imbl` btree internals; the captured node values (containing
    /// messages) must therefore be `Send + Sync`.
    type Message: Send + Sync;

    /// The height of the bottom-most level.
    type Height: Height;

    /// Collapse a [`Levels`] back to a node by folding all the levels together.
    fn collapse(self) -> Option<Node<Self::Message, Root>>;

    /// Get an immutable reference to the bottom-most level.
    #[allow(clippy::type_complexity)]
    fn level(&self) -> &BTreeMap<Prefix<Self::Height>, Node<Self::Message, Self::Height>>;

    /// Get a mutable reference to the bottom-most level.
    #[allow(clippy::type_complexity)]
    fn level_mut(
        &mut self,
    ) -> &mut BTreeMap<Prefix<Self::Height>, Node<Self::Message, Self::Height>>;

    /// Tack a new level onto the bottom of this [`Levels`], decreasing its height by one.
    ///
    /// This can only be called on a [`Levels`] whose height is more than zero.
    fn down<H>(self, below: BTreeMap<Prefix<H>, Node<Self::Message, H>>) -> Below<H, Self>
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

pub struct Top<T> {
    root: BTreeMap<Prefix<Root>, Node<T, Root>>,
}

impl<T> Clone for Top<T> {
    fn clone(&self) -> Self {
        Self {
            root: self.root.clone(),
        }
    }
}

impl<T> Default for Top<T> {
    fn default() -> Self {
        Self {
            root: Default::default(),
        }
    }
}

impl<T> Levels for Top<T>
where
    T: Send + Sync,
{
    type Message = T;
    type Height = Root;

    fn collapse(mut self) -> Option<Node<T, Root>> {
        self.root.remove(&Prefix::new())
    }

    fn level(&self) -> &BTreeMap<Prefix<Self::Height>, Node<T, Self::Height>> {
        &self.root
    }

    fn level_mut(&mut self) -> &mut BTreeMap<Prefix<Self::Height>, Node<T, Self::Height>> {
        &mut self.root
    }
}

pub struct Below<H, A>
where
    A: Levels<Height = S<H>>,
    H: Height,
{
    here: BTreeMap<Prefix<H>, Node<A::Message, H>>,
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
    type Message = A::Message;
    type Height = <A::Height as Pred>::Pred;

    fn collapse(mut self) -> Option<Node<A::Message, Root>> {
        let above = self.above.level_mut();

        // Pop each child's prefix to get its radix and parent prefix. BTreeMap
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

    fn level(&self) -> &BTreeMap<Prefix<Self::Height>, Node<A::Message, Self::Height>> {
        &self.here
    }

    fn level_mut(&mut self) -> &mut BTreeMap<Prefix<Self::Height>, Node<A::Message, Self::Height>> {
        &mut self.here
    }
}

mod sealed {
    use super::{Below, Height, Levels, Pred, S, Top};

    pub trait Sealed {}
    impl<T> Sealed for Top<T> {}
    impl<A: Levels<Height = S<H>> + Sealed, H: Height> Sealed for Below<H, A> where A::Height: Pred {}
}

#[cfg(test)]
mod test;
