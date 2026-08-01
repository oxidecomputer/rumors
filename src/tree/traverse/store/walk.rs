//! The backend-generic causal range walk: a lazy depth-first stream over a
//! subtree's live leaves, pruned by the resident version bounds.
//!
//! The generic twin of the in-memory range walk
//! ([`untyped::Range`](crate::tree::typed::untyped::Range)), resolving the
//! same difference-of-down-sets semantics against every handle's resident
//! ceiling and floor: subtrees wholly outside the range are pruned without
//! being fetched, and a promoted subtree's descent skips the version
//! comparisons.

use std::cmp::Ordering;
use std::ops::{Bound, RangeBounds};

use async_stream::try_stream;
use futures::{StreamExt as _, stream};

use crate::{
    Version, causally,
    tree::{
        backend::{BoxNodeStream, Leaf, Node, Store, children_of},
        typed::{
            Prefix,
            height::{Height, S, Z},
        },
    },
};

/// An owned causal range: the [`RangeBounds<Version>`] pair a walk filters
/// by, held by value so the stream is lifetime-free above the backend.
#[derive(Clone, Debug)]
pub struct VersionBounds {
    pub start: Bound<Version>,
    pub end: Bound<Version>,
}

impl VersionBounds {
    /// Capture any [`RangeBounds<Version>`] by cloning its two bounds.
    pub fn from_range<R: RangeBounds<Version>>(range: R) -> Self {
        Self {
            start: range.start_bound().cloned(),
            end: range.end_bound().cloned(),
        }
    }

    /// The subtracted down-set, as a start-only causal range.
    ///
    /// The bounds resolve one side at a time because the raw
    /// [`RangeBounds<Version>`] surface admits *crossed* pairs (which
    /// `causally` validates away at composition): a single-bound range is
    /// always well-formed, so each bound keeps its independent per-bound
    /// meaning and its independent cost — one causal comparison per check.
    fn subtracted(&self) -> causally::Range<'_> {
        match &self.start {
            Bound::Unbounded => causally::all(),
            Bound::Excluded(start) => causally::since(start),
            Bound::Included(start) => causally::not_before(start),
        }
    }

    /// The kept down-set, as an end-only causal range.
    fn kept(&self) -> causally::Range<'_> {
        match &self.end {
            Bound::Unbounded => causally::all(),
            Bound::Included(end) => causally::known_at(end),
            Bound::Excluded(end) => causally::before(end),
        }
    }

    /// Whether *no* leaf of a subtree with the given resident version
    /// bounds can pass.
    ///
    /// Holds when every leaf falls inside the subtracted start down-set
    /// (each is at most the node's ceiling, and subtraction composes down
    /// `<=`), or none falls inside the kept end down-set (each is at least
    /// the node's floor, and escaping containment composes up).
    /// Conservative in the right direction: `false` merely means the walk
    /// must look deeper.
    fn prunes<T, N>(&self, node: &N) -> bool
    where
        T: Send + Sync + 'static,
        N: Node<T>,
    {
        let span = node.span();
        self.subtracted().placement_of(span.join()) == Ordering::Less
            || self.kept().placement_of(span.meet()) == Ordering::Greater
    }

    /// Whether *every* leaf of a subtree with the given resident version
    /// bounds passes: the node's floor already escapes the subtracted start
    /// down-set, and its ceiling is already contained in the kept end
    /// down-set.
    ///
    /// For a leaf — whose floor and ceiling are both its version —
    /// prune-or-promote is exhaustive: an unpruned leaf always passes.
    fn promotes<T, N>(&self, node: &N) -> bool
    where
        T: Send + Sync + 'static,
        N: Node<T>,
    {
        let span = node.span();
        self.subtracted().contains(span.meet()) && self.kept().contains(span.join())
    }
}

/// The inductive step of the generic range walk, implemented per
/// [`Height`]: each level prunes or promotes by the resident bounds before
/// fetching children, yielding passing leaves in ascending path order.
pub trait Walk: Height {
    fn walk<'a, B, T>(
        backend: &'a B,
        prefix: Prefix<Self>,
        node: B::Node<Self>,
        bounds: &'a VersionBounds,
        passes: bool,
    ) -> BoxNodeStream<'a, B, T, Z>
    where
        B: Store<T, Node<Z>: Leaf<T>> + Sync,
        T: Send + Sync + 'static;
}

impl Walk for Z {
    fn walk<'a, B, T>(
        _backend: &'a B,
        prefix: Prefix<Z>,
        node: B::Node<Z>,
        bounds: &'a VersionBounds,
        passes: bool,
    ) -> BoxNodeStream<'a, B, T, Z>
    where
        B: Store<T, Node<Z>: Leaf<T>> + Sync,
        T: Send + Sync + 'static,
    {
        // Prune-or-promote is exhaustive at a leaf: an unpruned leaf passes.
        let verdict = (passes || !bounds.prunes(&node)).then_some(node);
        Box::pin(stream::iter(verdict.map(|node| Ok((prefix, node)))))
    }
}

impl<H: Walk> Walk for S<H>
where
    S<H>: Height,
{
    fn walk<'a, B, T>(
        backend: &'a B,
        prefix: Prefix<S<H>>,
        node: B::Node<S<H>>,
        bounds: &'a VersionBounds,
        passes: bool,
    ) -> BoxNodeStream<'a, B, T, Z>
    where
        B: Store<T, Node<Z>: Leaf<T>> + Sync,
        T: Send + Sync + 'static,
    {
        Box::pin(try_stream! {
            if !passes && bounds.prunes(&node) {
                return;
            }
            let passes = passes || bounds.promotes(&node);
            let children = children_of(backend, prefix, node).await?;
            for (radix, child) in children {
                let mut inner = H::walk(backend, prefix.push(radix), child, bounds, passes);
                while let Some(item) = inner.next().await {
                    yield item?;
                }
            }
        })
    }
}
