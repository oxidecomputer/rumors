//! Leaf iterators over the untyped tree.
//!
//! [`Iter`] and [`Range`] borrow their leaves and share a bidirectional walk.
//! [`RangeOwned`] holds node handles and reconstructs each leaf's full path.
//! All three visit leaves in path order; the range walks also filter by a
//! causal query. As a child of [`untyped`](super), this module can inspect
//! compressed paths and [`Children`] directly.

use std::collections::VecDeque;

use tinyvec::ArrayVec;

use crate::causally::{Coverage, Polarity, Query};

use crate::{Version, causally, message::Message};

use super::{Children, Node};
use crate::tree::typed::hash::PATH_LEN;

/// One pending subtree in a walk's frontier.
struct Frame<'a> {
    /// The subtree not yet entered.
    node: &'a Node,
    /// Whether an ancestor was already promoted: every leaf beneath `node`
    /// is known to satisfy the walk's range, so its descent skips the
    /// version comparisons.
    passes: bool,
}

/// A bidirectional walk shared by [`Iter`] and [`Range`].
///
/// The frontier holds unvisited subtrees in path order. Each step expands
/// only enough of one end to yield a leaf, borrowing its version and message.
/// [`Iter`] uses [`causally::all`] to visit every leaf.
///
/// [`Query::coverage`] classifies each subtree's version span: `Empty` skips
/// it, `Full` accepts every leaf beneath it without further comparisons, and
/// `Partial` requires descent. At a leaf, this is a membership test.
struct Walk<'a, P: Polarity> {
    /// Pending [`Frame`]s, held in ascending key order front-to-back.
    ///
    /// Forward steps consume the front, backward steps the back; a branch is
    /// expanded in place into its children (preserving the ordering), so the
    /// frontier always describes exactly the not-yet-yielded leaves. Empty
    /// once exhausted.
    frames: VecDeque<Frame<'a>>,
    /// Leaves not yet visited — the leaf count still reachable from the
    /// frontier.
    ///
    /// Seeded from the root's [`Node::len`], decremented once per
    /// yielded leaf and by a pruned subtree's whole count. Exploding a branch
    /// into its children preserves it (a branch's `len` is the sum of its
    /// children's). Under [`causally::all`] nothing is ever pruned, so this
    /// is exact — what lets [`Iter`] be an [`ExactSizeIterator`]; under any
    /// other query it is an upper bound.
    remaining: usize,
    /// The causal query filter; [`causally::all`] for the unfiltered
    /// [`Iter`].
    query: Query<'a, P>,
}

/// Maintain the ordered frontier and its remaining-leaf bound.
impl<'a, P: Polarity> Walk<'a, P> {
    /// Start at the supplied root, or with an empty frontier.
    fn new(node: Option<&'a Node>, query: Query<'a, P>) -> Self {
        match node {
            None => Self {
                frames: VecDeque::new(),
                remaining: 0,
                query,
            },
            Some(node) => Self {
                frames: VecDeque::from([Frame {
                    node,
                    passes: false,
                }]),
                remaining: node.len(),
                query,
            },
        }
    }

    /// Advance from one end of the frontier to the next passing leaf.
    ///
    /// `back` selects the end: `false` pops the smallest pending subtree off
    /// the front (the `next` direction), `true` pops the largest off the back
    /// (`next_back`). A popped branch is expanded back onto the *same* end,
    /// ordered so the frontier stays ascending front-to-back; the two ends
    /// therefore never yield the same leaf and meet cleanly when the frontier
    /// empties.
    fn step(&mut self, back: bool) -> Option<(&'a Version, &'a Message)> {
        'frontier: while let Some(Frame { node, passes }) = if back {
            self.frames.pop_back()
        } else {
            self.frames.pop_front()
        } {
            // Classify this subtree against the query, unless an ancestor
            // was already promoted.
            let passes = passes
                || match self.query.coverage(node.span()) {
                    Coverage::Empty => {
                        self.remaining -= node.len();
                        continue 'frontier;
                    }
                    Coverage::Full => true,
                    Coverage::Partial => false,
                };
            match &node.inner.children {
                Children::Leaf { message, .. } => {
                    // A leaf's span is coincident, so its coverage verdict is
                    // never Partial: reaching here means it passes.
                    debug_assert!(passes, "an unpruned leaf passes its query");
                    self.remaining -= 1;
                    return Some((node.ceiling(), message));
                }
                Children::Branch { children, .. } => {
                    // Re-push the children onto the end we just popped,
                    // ordered so the frontier stays ascending front-to-back:
                    // pushing to the front goes largest-radix-first so the
                    // smallest ends up frontmost; pushing to the back goes
                    // smallest-radix-first so the largest ends up backmost.
                    if back {
                        for (_, child) in children.iter() {
                            self.frames.push_back(Frame {
                                node: child,
                                passes,
                            });
                        }
                    } else {
                        for (_, child) in children.iter().rev() {
                            self.frames.push_front(Frame {
                                node: child,
                                passes,
                            });
                        }
                    }
                }
            }
        }
        None
    }
}

/// Borrow every leaf's version and message in ascending path order.
///
/// Forward and backward steps share one frontier, so interleaving them
/// visits each leaf once. The remaining length is exact. Path order comes
/// from version hashes and does not imply causal order.
///
/// [`Message`] provides both the payload and its cached encoding. Use
/// [`Message::arc`] for a typed payload handle. [`Range`] adds causal filtering.
pub struct Iter<'a> {
    /// Unfiltered frontier shared by both directions.
    walk: Walk<'a, causally::Neutral>,
}

/// Construct unfiltered borrowing walks.
impl<'a> Iter<'a> {
    /// Iterate the subtree rooted at `node`.
    pub(crate) fn root(node: &'a Node) -> Self {
        Self {
            walk: Walk::new(Some(node), causally::all()),
        }
    }

    /// The empty iterator, for a tree with no root.
    pub(crate) fn empty() -> Self {
        Self {
            walk: Walk::new(None, causally::all()),
        }
    }
}

/// Visit leaves from the smallest remaining path.
impl<'a> Iterator for Iter<'a> {
    /// References into the borrowed tree.
    type Item = (&'a Version, &'a Message);

    /// Yield the next leaf in ascending path order.
    fn next(&mut self) -> Option<Self::Item> {
        self.walk.step(false)
    }

    /// Exact, because the walk's `remaining` tracks the reachable leaf count
    /// precisely when nothing is pruned; the lower and upper bounds always
    /// coincide.
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.walk.remaining, Some(self.walk.remaining))
    }
}

/// Visit leaves from the largest remaining path.
impl<'a> DoubleEndedIterator for Iter<'a> {
    /// Yield the next leaf in descending path order.
    fn next_back(&mut self) -> Option<Self::Item> {
        self.walk.step(true)
    }
}

/// The unfiltered frontier tracks the exact number of remaining leaves.
impl<'a> ExactSizeIterator for Iter<'a> {}

/// Borrow leaves whose versions satisfy a causal [`Query`], in path order.
///
/// Version bounds allow entire subtrees to be accepted or skipped. Both
/// directions share a frontier, as in [`Iter`]. The number of matching
/// leaves is unknown until visited, so the size hint is an upper bound.
pub struct Range<'a, P: Polarity> {
    /// Filtered frontier shared by both directions.
    walk: Walk<'a, P>,
}

/// Construct borrowing walks with a causal filter.
impl<'a, P: Polarity> Range<'a, P> {
    /// Iterate the leaves of the (possibly absent) height-32 root `node`
    /// whose versions the causal `query` admits.
    pub(crate) fn root(node: Option<&'a Node>, query: Query<'a, P>) -> Self {
        Self {
            walk: Walk::new(node, query),
        }
    }
}

/// Visit matching leaves from the smallest remaining path.
impl<'a, P: Polarity> Iterator for Range<'a, P> {
    /// References into the borrowed tree.
    type Item = (&'a Version, &'a Message);

    /// Yield the next matching leaf in ascending path order.
    fn next(&mut self) -> Option<Self::Item> {
        self.walk.step(false)
    }

    /// An upper bound only: pruning subtracts what it can prove out, but a
    /// visited leaf's passing is not known until it is reached.
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.walk.remaining))
    }
}

/// Visit matching leaves from the largest remaining path.
impl<'a, P: Polarity> DoubleEndedIterator for Range<'a, P> {
    /// Yield the next matching leaf in descending path order.
    fn next_back(&mut self) -> Option<Self::Item> {
        self.walk.step(true)
    }
}

/// An owned walk yielding each matching leaf and its full path, in path order.
///
/// Node handles and an owned query let the walk outlive its caller's tree
/// handle. Filtering follows [`Range`]; traversal is forward-only.
///
/// The descent uses a path buffer and one [`Level`] per branch, bounded by
/// the 32-byte path. Each level selects one child at a time and retains its
/// branch's subtree until traversal leaves it.
pub struct RangeOwned<P: Polarity> {
    /// The not-yet-visited root, consumed by the first advance.
    start: Option<Node>,
    /// The descent spine: index 0 is the root's level, the last entry is the
    /// level currently being walked. Always branch nodes (leaves are yielded,
    /// never pushed).
    spine: Vec<Level>,
    /// The path bytes accumulated along the spine, extended and rolled back
    /// as the walk descends and ascends; a leaf is yielded exactly when it
    /// reaches 32 bytes.
    path: ArrayVec<[u8; PATH_LEN]>,
    /// The causal query filter, its bounds settled owned so the walk
    /// carries no lifetime.
    query: Query<'static, P>,
}

/// One level of a [`RangeOwned`] walk's descent spine.
struct Level {
    /// The branch node this level walks.
    node: Node,
    /// The smallest radix still to visit, or `None` when this level is done.
    next: Option<u8>,
    /// Whether an ancestor (or this level itself) was promoted: every leaf
    /// beneath is known to satisfy the range, so descendants skip the
    /// version comparisons.
    passes: bool,
    /// The path length to restore when this level is popped: its length
    /// before this node's radix and compressed prefix were appended.
    rollback: usize,
}

/// An owned handle to a stored leaf, lending its version and payload.
pub struct Leaf(
    /// The stored node, including any compressed path above the leaf.
    Node,
);

/// Read leaf data or obtain a node at height zero.
impl Leaf {
    /// The causal [`Version`] at which this message was observed.
    pub fn version(&self) -> &Version {
        self.0.ceiling()
    }

    /// The message's value as its concrete payload type: an owned handle,
    /// one reference bump on the shared allocation.
    ///
    /// # Panics
    ///
    /// If the payload is not a `T` (see [`Message::arc`]).
    pub fn value<T: Send + Sync + 'static>(&self) -> std::sync::Arc<T> {
        self.0
            .as_leaf()
            .expect("a Leaf wraps a leaf node, by construction")
            .arc::<T>()
    }

    /// Return a leaf node with an empty compressed path.
    ///
    /// Height-zero nodes hash an empty suffix. Reuse an already bare node;
    /// otherwise build one sharing the stored version and payload, leaving
    /// the original compressed node intact for other readers.
    pub(crate) fn into_node(self) -> Node {
        if self.0.inner.prefix.is_empty() {
            return self.0;
        }
        match &self.0.inner.children {
            Children::Leaf { version, message } => Node::leaf(version.clone(), message.clone()),
            Children::Branch { .. } => {
                unreachable!("a Leaf wraps a leaf node, by construction")
            }
        }
    }
}

/// Start an owned walk at the root or at a known path within the tree.
impl<P: Polarity> RangeOwned<P> {
    /// Walk the leaves of the (possibly absent) height-32 root `node`
    /// whose versions the causal `query` admits.
    pub(crate) fn root(node: Option<Node>, query: Query<'static, P>) -> Self {
        Self::within(node, &[], query)
    }

    /// Walk the leaves of a subtree rooted below the top of the tree.
    ///
    /// `path` contains the preceding bytes, shallowest first. Its length plus
    /// the node's height must be 32, so each yielded leaf has a full path.
    pub(crate) fn within(node: Option<Node>, path: &[u8], query: Query<'static, P>) -> Self {
        let mut buf = ArrayVec::new();
        buf.extend_from_slice(path);
        Self {
            start: node,
            // Reserve enough frames for any path so descent never grows
            // this buffer.
            spine: Vec::with_capacity(PATH_LEN),
            path: buf,
            query,
        }
    }
}

/// Yield owned leaves from an ascending walk of the current subtree.
impl<P: Polarity> Iterator for RangeOwned<P> {
    /// A full version-derived path and a handle to its stored leaf.
    type Item = ([u8; PATH_LEN], Leaf);

    /// Advance to the next matching leaf in ascending path order.
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // Obtain the next unvisited node — the initial root, or the next
            // child at the deepest spine level, ascending past exhausted
            // levels — remembering the path length to roll back to if it
            // proves not to descend.
            let (node, inherited, rollback) = match self.start.take() {
                // The starting node rolls back to the seed path it was
                // entered with (empty only for a true root).
                Some(root) => (root, false, self.path.len()),
                None => loop {
                    let level = self.spine.last_mut()?;
                    let next_child = match &level.node.inner.children {
                        // Find the next radix by binary search and clone
                        // only that child. Pending siblings stay in the branch.
                        Children::Branch { children, .. } => level
                            .next
                            .and_then(|at| children.successor(at))
                            .map(|(radix, child)| (radix, child.clone())),
                        Children::Leaf { .. } => {
                            unreachable!("spine levels are branches, by construction")
                        }
                    };
                    match next_child {
                        // Exhausted: ascend, restoring the parent's path.
                        None => {
                            let rollback = level.rollback;
                            self.spine.pop();
                            self.path.truncate(rollback);
                        }
                        Some((radix, child)) => {
                            // Radix 255 is the last child this level can visit.
                            level.next = radix.checked_add(1);
                            let passes = level.passes;
                            let rollback = self.path.len();
                            self.path.push(radix);
                            break (child, passes, rollback);
                        }
                    }
                },
            };

            // Classify this subtree against the query, unless an ancestor
            // was already promoted.
            let passes = inherited
                || match self.query.coverage(node.span()) {
                    Coverage::Empty => {
                        self.path.truncate(rollback);
                        continue;
                    }
                    Coverage::Full => true,
                    Coverage::Partial => false,
                };

            // Replay the compressed prefix, shallowest byte first.
            for &byte in node.inner.prefix.iter().rev() {
                self.path.push(byte);
            }

            if matches!(&node.inner.children, Children::Branch { .. }) {
                // Descend: this node becomes the new deepest level.
                self.spine.push(Level {
                    node,
                    next: Some(0),
                    passes,
                    rollback,
                });
                continue;
            }

            // A leaf: its coincident span makes the coverage verdict
            // membership itself, never Partial, so an unpruned leaf always
            // passes. Yield it and roll the path back to its parent.
            debug_assert!(passes, "an unpruned leaf passes its query");
            debug_assert_eq!(
                self.path.len(),
                PATH_LEN,
                "a leaf's path spans the full address"
            );
            let key = self.path.into_inner();
            self.path.truncate(rollback);
            return Some((key, Leaf(node)));
        }
    }
}
