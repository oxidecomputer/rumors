use std::mem;

use itertools::Itertools;

use crate::tree::typed::{
    Children, Node, Prefix,
    height::{Height, Pred, Root, S},
};

mod level;

pub use level::Level;

/// Create a new [`Levels`] from the root of a tree.
pub fn levels<T>(root: Option<Node<T, Root>>) -> Top<T> {
    Top {
        root: Level::from_iter(root.map(|root| (Prefix::new(), root))),
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
    fn level(&self) -> &Level<Self::Message, Self::Height>;

    /// Get a mutable reference to the bottom-most level.
    fn level_mut(&mut self) -> &mut Level<Self::Message, Self::Height>;

    /// Tack a new level onto the bottom of this [`Levels`], decreasing its height by one.
    ///
    /// This can only be called on a [`Levels`] whose height is more than zero.
    fn down<H>(self, below: Level<Self::Message, H>) -> Below<H, Self>
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
    root: Level<T, Root>,
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

    fn level(&self) -> &Level<T, Self::Height> {
        &self.root
    }

    fn level_mut(&mut self) -> &mut Level<T, Self::Height> {
        &mut self.root
    }
}

pub struct Below<H, A>
where
    A: Levels<Height = S<H>>,
    H: Height,
{
    here: Level<A::Message, H>,
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
        // Pop each child's prefix to get its radix and parent prefix. `Level`
        // iteration is sorted, so siblings are adjacent.
        let siblings = self.here.into_iter().map(|(prefix, node)| {
            let (parent_prefix, radix) = prefix.pop();
            (parent_prefix, radix, node)
        });

        // Rebuild the level above by merging its existing parents (sorted) with
        // the parents reconstructed from this level's siblings (also sorted,
        // grouped by `chunk_by`). Co-iterating the two sorted runs rebuilds each
        // parent exactly once in a single linear pass, rather than a
        // binary-search `remove` + `insert` per parent.
        let mut existing = mem::take(self.above.level_mut()).into_iter().peekable();
        let mut rebuilt = Level::default();
        for (parent_prefix, group) in &siblings.chunk_by(|(pp, _, _)| *pp) {
            // Carry over existing parents that precede this group untouched.
            while existing.peek().is_some_and(|(ep, _)| *ep < parent_prefix) {
                let (ep, enode) = existing.next().unwrap();
                rebuilt.push(ep, enode);
            }

            // Start from the existing parent's children when it shares this
            // prefix (deconstructing it once), otherwise from an empty set.
            let mut children = if existing.peek().is_some_and(|(ep, _)| *ep == parent_prefix) {
                existing.next().unwrap().1.into_children()
            } else {
                Children::default()
            };

            // Merge all siblings in this group into the children map.
            for (_, radix, node) in group {
                children.insert(radix, node);
            }

            // Reconstruct the parent and append it to the rebuilt level.
            if let Some(parent) = Node::branch(children) {
                rebuilt.push(parent_prefix, parent);
            }
        }

        // Existing parents past the last group carry over untouched.
        for (ep, enode) in existing {
            rebuilt.push(ep, enode);
        }
        *self.above.level_mut() = rebuilt;

        // Collapse the level above, recursively
        self.above.collapse()
    }

    fn level(&self) -> &Level<A::Message, Self::Height> {
        &self.here
    }

    fn level_mut(&mut self) -> &mut Level<A::Message, Self::Height> {
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
