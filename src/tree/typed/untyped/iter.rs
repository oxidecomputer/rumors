//! Leaf iterators over the untyped tree: a shared frontier walk and its two
//! shells, [`Iter`] (the unfiltered, exact-size walk) and [`Range`]
//! (the walk filtered to a causal [`causally::Query`]).
//!
//! A child module of [`node`](super) so the walk can match on the parent's
//! private [`Children`] variants and path-compression internals directly.

use std::collections::VecDeque;

use tinyvec::ArrayVec;

use crate::causally::{Coverage, Polarity, Query};

use crate::{Version, causally, message::Message};

use super::{Children, Node};

/// One pending subtree in a walk's frontier.
struct Frame<'a> {
    /// The subtree not yet entered.
    node: &'a Node,
    /// Whether an ancestor was already promoted: every leaf beneath `node`
    /// is known to satisfy the walk's range, so its descent skips the
    /// version comparisons.
    passes: bool,
}

/// The shared frontier engine beneath [`Iter`] and [`Range`]: a lazy
/// depth-first walk over a subtree's live leaves, filtered by a causal
/// [`Query`].
///
/// [`Iter`] passes [`causally::all`], whose one root classification
/// promotes the whole walk. The walk yields each leaf's [`Version`] and a
/// borrowed handle to its [`Message`]; a leaf's location is a pure
/// function of its version, so no path is reconstructed (the owned walk,
/// [`RangeOwned`], is the one that yields paths — its consumers key
/// leaves by them).
///
/// The walk is lazy: a single step descends only far enough to reach the
/// next leaf, so the first item is produced after walking one root-to-leaf
/// spine rather than the whole tree; the only allocation the walk ever
/// makes is the frontier deque itself.
///
/// A popped subtree is classified before it is entered: one
/// [`coverage`](Query::coverage) verdict over its memoized
/// [`span`](Node::span) prunes it whole ([`Empty`](Coverage::Empty)),
/// promotes it ([`Full`](Coverage::Full); its descendants skip the
/// version comparisons), or descends it undecided
/// ([`Partial`](Coverage::Partial)). A leaf's span is coincident, so
/// its verdict degenerates to membership and prune-or-promote is
/// exhaustive: the walk never compares versions leaf-by-leaf.
///
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

impl<'a, P: Polarity> Walk<'a, P> {
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

/// A lazy depth-first iterator over every live leaf in a subtree, yielding
/// each leaf's [`Version`] and a borrowed handle to its [`Message`].
///
/// For the same walk filtered to a causal range, see [`Range`].
///
/// The [`Message`] is the richest leaf payload (it carries the cached
/// serialization alongside the shared payload handle); callers that only
/// want the value project it with [`Message::arc`].
///
/// [`next`](Iterator::next) yields leaves in ascending order of their
/// version-derived paths; the iterator is also a [`DoubleEndedIterator`],
/// so [`next_back`](DoubleEndedIterator::next_back) yields them in
/// descending path order, and the two ends meet in the middle without
/// overlap. Path order bears *no* relation to the causal order on
/// [`Version`]s: a leaf may be yielded before one that causally precedes
/// it. (The public observers on [`Rumors`](crate::Rumors) still promise
/// nothing about order, but [`unknown`](crate::tree::traverse::unknown)
/// and `Tree::join` lean on the ascending forward order for their own
/// deterministic callback delivery.)
///
/// `Iter` is `Send + Sync`: it holds only `&Node` references, and the
/// stored payloads are `Send + Sync` by [`Message`]'s construction bound.
pub struct Iter<'a> {
    walk: Walk<'a, causally::Neutral>,
}

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

impl<'a> Iterator for Iter<'a> {
    type Item = (&'a Version, &'a Message);

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

impl<'a> DoubleEndedIterator for Iter<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.walk.step(true)
    }
}

impl<'a> ExactSizeIterator for Iter<'a> {}

/// The leaf walk filtered to a causal [`Query`].
///
/// A leaf is yielded iff the query [`contains`](Query::contains) its
/// version. Subtrees wholly outside the query are pruned by one
/// [`coverage`](Query::coverage) verdict over their memoized version
/// bounds without being entered, so a walk over a small causal delta
/// against a large tree costs work proportional to the delta (plus
/// the pruning frontier), not the tree.
///
/// Same item shape and ordering guarantees as [`Iter`] — in particular,
/// iteration order is key order, *not* causal order: filtering by versions
/// does not mean yielding in version order — but *not* an
/// [`ExactSizeIterator`]: how many leaves pass is unknown until they are
/// visited, so [`size_hint`](Iterator::size_hint) reports only an upper
/// bound.
pub struct Range<'a, P: Polarity> {
    walk: Walk<'a, P>,
}

impl<'a, P: Polarity> Range<'a, P> {
    /// Iterate the leaves of the (possibly absent) height-32 root `node`
    /// whose versions the causal `query` admits.
    pub(crate) fn root(node: Option<&'a Node>, query: Query<'a, P>) -> Self {
        Self {
            walk: Walk::new(node, query),
        }
    }
}

impl<'a, P: Polarity> Iterator for Range<'a, P> {
    type Item = (&'a Version, &'a Message);

    fn next(&mut self) -> Option<Self::Item> {
        self.walk.step(false)
    }

    /// An upper bound only: pruning subtracts what it can prove out, but a
    /// visited leaf's passing is not known until it is reached.
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.walk.remaining))
    }
}

impl<'a, P: Polarity> DoubleEndedIterator for Range<'a, P> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.walk.step(true)
    }
}

/// The owned, "frozen" counterpart of the borrowing walk: frames hold cheap
/// [`Node`] handles (`Arc` clones) instead of `&Node` borrows.
///
/// The walk carries no lifetime and can be held across awaits and stored in
/// long-lived state.
///
/// Its state is *constant-size*: a descent spine of at most one [`Level`]
/// per materialized branch level along the current path (≤ 32, under two
/// kilobytes all told), plus the shared path buffer. Unvisited siblings are
/// never enumerated — each advance probes the parent's child map for the
/// next radix at or past the level's cursor — and child handles are cloned
/// one at a time, lazily, as they are visited. The spine's node handles pin
/// only the current path's ancestors; everything already walked past is
/// released.
///
/// Same query semantics and prune/promote/descend classification as the
/// borrowing walk (see [`Range`]); forward-only, since its consumers are
/// subscription drains.
/// Yields each passing leaf as an owned [`Leaf`] handle alongside its
/// reconstructed 32-byte path; the version and value read out of the
/// handle as shared, clone-cheap references into the tree's storage.
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
    path: ArrayVec<[u8; 32]>,
    /// The causal query filter, its bounds settled owned so the walk
    /// carries no lifetime.
    query: Query<'static, P>,
}

/// One level of a [`RangeOwned`] walk's descent spine.
struct Level {
    /// The branch node this level walks.
    node: Node,
    /// The smallest child radix not yet visited; `256` means exhausted.
    next: u16,
    /// Whether an ancestor (or this level itself) was promoted: every leaf
    /// beneath is known to satisfy the range, so descendants skip the
    /// version comparisons.
    passes: bool,
    /// The path length to restore when this level is popped: its length
    /// before this node's radix and compressed prefix were appended.
    rollback: usize,
}

/// A live leaf popped out of a [`RangeOwned`] walk: an owned handle on the leaf
/// node, lending its version and value to whoever holds it.
pub struct Leaf(Node);

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
    /// If the payload is not a `T` (see [`Message::message`]).
    pub fn value<T: Send + Sync + 'static>(&self) -> std::sync::Arc<T> {
        self.0
            .as_leaf()
            .expect("a Leaf wraps a leaf node, by construction")
            .arc::<T>()
    }

    /// Unwrap into a bare height-zero leaf node.
    ///
    /// The walk yields the leaf as stored, which usually carries the
    /// compressed spine above it; a height-zero view must shed that prefix
    /// (its hash commits an empty suffix, not the stored spine). The stored handle
    /// is reused when it is already bare; otherwise a fresh prefix-free
    /// leaf is built around the same message handle.
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

impl<P: Polarity> RangeOwned<P> {
    /// Walk the leaves of the (possibly absent) height-32 root `node`
    /// whose versions the causal `query` admits.
    pub(crate) fn root(node: Option<Node>, query: Query<'static, P>) -> Self {
        Self::within(node, &[], query)
    }

    /// Walk the leaves of a subtree rooted below the top of the tree.
    ///
    /// `path` carries the bytes already walked to reach `node` (the
    /// ancestors' radixes, shallowest-first), which the descent extends so
    /// each leaf still reconstructs a full 32-byte
    /// 32-byte path. `path.len()` plus the height of
    /// `node` must therefore be 32.
    pub(crate) fn within(node: Option<Node>, path: &[u8], query: Query<'static, P>) -> Self {
        let mut buf = ArrayVec::new();
        buf.extend_from_slice(path);
        Self {
            start: node,
            // One level per materialized branch along a root-to-leaf path:
            // never more than the depth, so this is the walk's only
            // allocation.
            spine: Vec::with_capacity(32),
            path: buf,
            query,
        }
    }

    /// Advance to the next passing leaf. The same classification as the
    /// borrowing walk, with the leaf handed out by value.
    pub(crate) fn next(&mut self) -> Option<([u8; 32], Leaf)> {
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
                        // Probe for the smallest not-yet-visited radix: one
                        // O(log fan-out) binary search, so unvisited siblings
                        // are never enumerated or held.
                        Children::Branch { children, .. } if level.next <= u8::MAX as u16 => {
                            children
                                .successor(level.next as u8)
                                .map(|(radix, child)| (radix, child.clone()))
                        }
                        Children::Branch { .. } => None,
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
                            level.next = radix as u16 + 1;
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
                    next: 0,
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
                32,
                "a leaf sits at depth 32, so its path is 32 bytes"
            );
            let key = self.path.into_inner();
            self.path.truncate(rollback);
            return Some((key, Leaf(node)));
        }
    }
}
