//! Leaf iterators over the untyped tree: a shared frontier walk and its two
//! shells, [`Iter`] (the unfiltered, exact-size walk) and [`Range`]
//! (the walk filtered to a causal [`RangeBounds<Version>`] range).
//!
//! A child module of [`node`](super) so the walk can match on the parent's
//! private [`Children`] variants and path-compression internals directly.

use std::cmp::Ordering;
use std::collections::VecDeque;
use std::ops::{Bound, RangeBounds, RangeFull};

use tinyvec::ArrayVec;

use crate::{Version, message::Message};

use super::{Children, Node};

/// A causal range's bound pair, resolved for one subtree check.
///
/// On the partially ordered [`Version`]s, a range denotes a *difference of
/// causal down-sets*: keep the leaves contained in the end bound, subtract
/// the leaves contained in the start bound. Per leaf version `v`:
///
/// - start [`Unbounded`](Bound::Unbounded): subtract nothing;
///   [`Excluded(s)`](Bound::Excluded): subtract `v <= s`;
///   [`Included(s)`](Bound::Included): subtract `v < s`, so `s` itself
///   survives.
/// - end [`Unbounded`](Bound::Unbounded): keep everything;
///   [`Included(e)`](Bound::Included): keep `v <= e`;
///   [`Excluded(e)`](Bound::Excluded): keep `v < e`.
///
/// Note the asymmetry inherent to the partial order: a start bound of
/// either kind keeps versions *concurrent* to it (subtraction removes only
/// the bound's causal past), while an end bound of either kind drops them
/// (keeping demands containment).
struct Bounds<'a> {
    start: Bound<&'a Version>,
    end: Bound<&'a Version>,
}

impl Bounds<'_> {
    /// Whether *no* leaf of a subtree with the given memoized version
    /// bounds can pass.
    ///
    /// Holds when every leaf falls inside the subtracted start down-set (each
    /// is at most the node's ceiling), or none falls inside the kept end
    /// down-set (each is at least the node's floor, and containment composes
    /// through `<=`). Conservative in the right direction: `false` merely
    /// means the walk must look deeper.
    fn prunes<T>(&self, node: &Node<T>) -> bool {
        let below_start = match self.start {
            Bound::Unbounded => false,
            Bound::Excluded(start) => node.ceiling() <= start,
            Bound::Included(start) => node.ceiling() < start,
        };
        let beyond_end = || match self.end {
            Bound::Unbounded => false,
            Bound::Included(end) => matches!(
                node.floor().partial_cmp(end),
                None | Some(Ordering::Greater)
            ),
            Bound::Excluded(end) => matches!(
                node.floor().partial_cmp(end),
                None | Some(Ordering::Equal | Ordering::Greater)
            ),
        };
        below_start || beyond_end()
    }

    /// Whether *every* leaf of a subtree with the given memoized version
    /// bounds passes: the node's floor already escapes the subtracted start
    /// down-set, and its ceiling is already contained in the kept end
    /// down-set.
    ///
    /// For a leaf — whose floor and ceiling are both its version —
    /// prune-or-promote is exhaustive: an unpruned leaf always passes.
    fn promotes<T>(&self, node: &Node<T>) -> bool {
        let clears_start = match self.start {
            Bound::Unbounded => true,
            Bound::Excluded(start) => matches!(
                node.floor().partial_cmp(start),
                None | Some(Ordering::Greater)
            ),
            Bound::Included(start) => matches!(
                node.floor().partial_cmp(start),
                None | Some(Ordering::Equal | Ordering::Greater)
            ),
        };
        clears_start
            && match self.end {
                Bound::Unbounded => true,
                Bound::Included(end) => node.ceiling() <= end,
                Bound::Excluded(end) => node.ceiling() < end,
            }
    }
}

/// One pending subtree in a walk's frontier.
struct Frame<'a, T> {
    /// The subtree not yet entered.
    node: &'a Node<T>,
    /// The path bytes accumulated to reach `node` (above its own compressed
    /// prefix), inline: the tree's depth is fixed at 32, so a leaf's full
    /// path always fits and the buffer never spills to the heap.
    path: ArrayVec<[u8; 32]>,
    /// Whether an ancestor was already promoted: every leaf beneath `node`
    /// is known to satisfy the walk's range, so its descent skips the
    /// version comparisons.
    passes: bool,
}

/// The shared frontier engine beneath [`Iter`] and [`Range`]: a lazy
/// depth-first walk over a subtree's live leaves, filtered by a causal
/// [`RangeBounds<Version>`] range.
///
/// See [`Bounds`] for the semantics; [`RangeFull`] is the unfiltered walk and
/// never touches a version. The walk yields each leaf's reconstructed 32-byte
/// path [`Key`], its [`Version`], and a borrowed handle to its [`Message`].
///
/// The walk is lazy: a single step descends only far enough to reach the
/// next leaf, so the first item is produced after walking one root-to-leaf
/// spine rather than the whole tree. Each pending node in the frontier
/// carries the path bytes accumulated to reach it (above its own compressed
/// prefix); since the tree's depth is fixed at 32, those fit an inline
/// [`ArrayVec<[u8; 32]>`](ArrayVec) (the same shape as
/// [`Prefix`](crate::tree::typed::Prefix)), so the only allocation the walk
/// ever makes is the frontier deque itself.
///
/// A popped subtree is resolved against the range by its memoized
/// [`ceiling`](Node::ceiling)/[`floor`](Node::floor) before it is entered:
/// a subtree that cannot contain a passing leaf is pruned whole
/// ([`Bounds::prunes`]), one whose every leaf must pass is promoted
/// ([`Bounds::promotes`]; its descendants skip the version comparisons), and
/// only subtrees genuinely straddling a bound are descended undecided. For a
/// leaf the prune-or-promote dichotomy is exhaustive, so the walk never
/// compares versions leaf-by-leaf.
///
/// [`Key`]: crate::tree::key::Key
struct Walk<'a, T, R> {
    /// Pending [`Frame`]s, held in ascending key order front-to-back.
    ///
    /// Forward steps consume the front, backward steps the back; a branch is
    /// expanded in place into its children (preserving the ordering), so the
    /// frontier always describes exactly the not-yet-yielded leaves. Empty
    /// once exhausted.
    frames: VecDeque<Frame<'a, T>>,
    /// Leaves not yet visited — the leaf count still reachable from the
    /// frontier.
    ///
    /// Seeded from the root's [`Node::len`], decremented once per
    /// yielded leaf and by a pruned subtree's whole count. Exploding a branch
    /// into its children preserves it (a branch's `len` is the sum of its
    /// children's). With [`RangeFull`] nothing is ever pruned, so this is
    /// exact — what lets [`Iter`] be an [`ExactSizeIterator`]; with any other
    /// range it is an upper bound.
    remaining: usize,
    /// The causal range filter; [`RangeFull`] for the unfiltered [`Iter`].
    range: R,
}

impl<'a, T, R: RangeBounds<Version>> Walk<'a, T, R> {
    fn new(node: Option<&'a Node<T>>, path: &[u8], range: R) -> Self {
        match node {
            None => Self {
                frames: VecDeque::new(),
                remaining: 0,
                range,
            },
            Some(node) => {
                let mut buf = ArrayVec::new();
                buf.extend_from_slice(path);
                Self {
                    frames: VecDeque::from([Frame {
                        node,
                        path: buf,
                        passes: false,
                    }]),
                    remaining: node.len(),
                    range,
                }
            }
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
    fn step(&mut self, back: bool) -> Option<(crate::tree::key::Key, &'a Version, &'a Message<T>)> {
        'frontier: while let Some(Frame {
            node,
            mut path,
            passes,
        }) = if back {
            self.frames.pop_back()
        } else {
            self.frames.pop_front()
        } {
            // Resolve this subtree against the range, unless an ancestor was
            // already promoted.
            let passes = passes || {
                let bounds = Bounds {
                    start: self.range.start_bound(),
                    end: self.range.end_bound(),
                };
                if bounds.prunes(node) {
                    self.remaining -= node.len();
                    continue 'frontier;
                }
                bounds.promotes(node)
            };
            // The compressed prefix sits above this node's level and is stored
            // shallowest-last, so replay it shallowest-first to extend the path.
            for &byte in node.inner.prefix.iter().rev() {
                path.push(byte);
            }
            match &node.inner.children {
                Children::Leaf { message, .. } => {
                    // A leaf's floor and ceiling are both its version, so the
                    // prune/promote dichotomy above is exhaustive: reaching
                    // here means it passes.
                    debug_assert!(passes, "an unpruned leaf passes its range");
                    debug_assert_eq!(
                        path.len(),
                        32,
                        "a leaf sits at depth 32, so its path is 32 bytes"
                    );
                    let path = path.into_inner();
                    self.remaining -= 1;
                    return Some((crate::tree::key::Key(path), node.ceiling(), message));
                }
                Children::Branch { children, .. } => {
                    // Re-push the children onto the end we just popped, each
                    // with its own extended copy of the inline path buffer
                    // (the per-frame buffer is what keeps the descent lazy).
                    // Order so the frontier stays ascending front-to-back:
                    // pushing to the front goes largest-radix-first so the
                    // smallest ends up frontmost; pushing to the back goes
                    // smallest-radix-first so the largest ends up backmost.
                    if back {
                        for (radix, child) in children.iter() {
                            let mut child_path = path;
                            child_path.push(*radix);
                            self.frames.push_back(Frame {
                                node: child,
                                path: child_path,
                                passes,
                            });
                        }
                    } else {
                        for (radix, child) in children.iter().rev() {
                            let mut child_path = path;
                            child_path.push(*radix);
                            self.frames.push_front(Frame {
                                node: child,
                                path: child_path,
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
/// each leaf's reconstructed 32-byte path [`Key`], its [`Version`], and a
/// borrowed handle to its [`Message`].
///
/// For the same walk filtered to a causal range, see [`Range`].
///
/// The [`Message`] is the richest leaf payload (it carries the cached
/// serialization alongside the `Arc<T>`); callers that only want the value
/// project it cheaply with [`Message::as_arc`].
///
/// [`next`](Iterator::next) yields leaves in ascending-key order; the iterator
/// is also a [`DoubleEndedIterator`], so [`next_back`](DoubleEndedIterator::next_back)
/// yields them in descending-key order, and the two ends meet in the middle
/// without overlap. Keys are content-derived hashes, so key order bears *no*
/// relation to the causal order on [`Version`]s: a leaf may be yielded
/// before one that causally precedes it. (The public observers on
/// [`Rumors`](crate::Rumors) still promise nothing about order, but
/// [`unknown`](crate::tree::traverse::unknown) and `Tree::join` lean on the
/// ascending forward order for their own deterministic callback delivery.)
///
/// `Iter` is `Send + Sync` whenever `T: Send + Sync`: it holds only `&Node<T>`
/// references and inline path buffers.
///
/// [`Key`]: crate::tree::key::Key
pub struct Iter<'a, T> {
    walk: Walk<'a, T, RangeFull>,
}

impl<'a, T> Iter<'a, T> {
    /// Iterate the subtree rooted at `node` (a height-32 root, so every leaf's
    /// path is a full 32-byte [`Key`](crate::tree::key::Key)).
    pub(crate) fn root(node: &'a Node<T>) -> Self {
        Self::within(node, &[])
    }

    /// Iterate the subtree rooted at `node` when it does *not* sit at the top
    /// of the tree.
    ///
    /// `path` carries the bytes already walked to reach it (the ancestors'
    /// radixes, shallowest-first), which the descent extends so each leaf
    /// still reconstructs a full 32-byte [`Key`](crate::tree::key::Key).
    /// `path.len()` plus the height of `node` must therefore be 32.
    pub(crate) fn within(node: &'a Node<T>, path: &[u8]) -> Self {
        Self {
            walk: Walk::new(Some(node), path, ..),
        }
    }

    /// The empty iterator, for a tree with no root.
    pub(crate) fn empty() -> Self {
        Self {
            walk: Walk::new(None, &[], ..),
        }
    }
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (crate::tree::key::Key, &'a Version, &'a Message<T>);

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

impl<'a, T> DoubleEndedIterator for Iter<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.walk.step(true)
    }
}

impl<'a, T> ExactSizeIterator for Iter<'a, T> {}

/// The leaf walk filtered to a causal [`RangeBounds<Version>`] range.
///
/// A leaf with version `v` is yielded iff it is contained in the range's end
/// bound and *not* contained in its start bound — a difference of causal
/// down-sets. Per bound kind:
///
/// - start [`Unbounded`](Bound::Unbounded): nothing subtracted;
///   [`Excluded(s)`](Bound::Excluded): leaves with `v <= s` are subtracted;
///   [`Included(s)`](Bound::Included): leaves with `v < s` are subtracted
///   (`s` itself survives).
/// - end [`Unbounded`](Bound::Unbounded): everything kept;
///   [`Included(e)`](Bound::Included): leaves with `v <= e` are kept;
///   [`Excluded(e)`](Bound::Excluded): leaves with `v < e` are kept.
///
/// A start bound of either kind keeps versions *concurrent* to it
/// (subtraction removes only the bound's causal past — "everything since"
/// must not drop other parties' concurrent leaves), while an end bound of
/// either kind drops them (keeping demands containment).
///
/// Subtrees wholly outside the range are pruned by their memoized version
/// bounds without being entered, so a walk over a small causal delta against
/// a large tree costs work proportional to the delta (plus the pruning
/// frontier), not the tree.
///
/// Same item shape and ordering guarantees as [`Iter`] — in particular,
/// iteration order is key order, *not* causal order: filtering by versions
/// does not mean yielding in version order — but *not* an
/// [`ExactSizeIterator`]: how many leaves pass is unknown until they are
/// visited, so [`size_hint`](Iterator::size_hint) reports only an upper
/// bound.
pub struct Range<'a, T, R> {
    walk: Walk<'a, T, R>,
}

impl<'a, T, R: RangeBounds<Version>> Range<'a, T, R> {
    /// Iterate the leaves of the (possibly absent) height-32 root `node`
    /// whose versions fall within the causal `range`.
    pub(crate) fn root(node: Option<&'a Node<T>>, range: R) -> Self {
        Self {
            walk: Walk::new(node, &[], range),
        }
    }
}

impl<'a, T, R: RangeBounds<Version>> Iterator for Range<'a, T, R> {
    type Item = (crate::tree::key::Key, &'a Version, &'a Message<T>);

    fn next(&mut self) -> Option<Self::Item> {
        self.walk.step(false)
    }

    /// An upper bound only: pruning subtracts what it can prove out, but a
    /// visited leaf's passing is not known until it is reached.
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.walk.remaining))
    }
}

impl<'a, T, R: RangeBounds<Version>> DoubleEndedIterator for Range<'a, T, R> {
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
/// Same range semantics and prune/promote logic as the borrowing walk (see
/// [`Bounds`]); forward-only, since its consumers are subscription drains.
/// Yields each passing leaf as an owned [`Leaf`] handle alongside its
/// reconstructed [`Key`](crate::tree::key::Key), which is what lets a caller
/// lend `&Version` / `&Arc<T>` out of a leaf it keeps.
pub struct RangeOwned<T, R> {
    /// The not-yet-visited root, consumed by the first advance.
    start: Option<Node<T>>,
    /// The descent spine: index 0 is the root's level, the last entry is the
    /// level currently being walked. Always branch nodes (leaves are yielded,
    /// never pushed).
    spine: Vec<Level<T>>,
    /// The path bytes accumulated along the spine, extended and rolled back
    /// as the walk descends and ascends; a leaf is yielded exactly when it
    /// reaches 32 bytes.
    path: ArrayVec<[u8; 32]>,
    /// The causal range filter (owned, e.g. a `(Bound<Version>,
    /// Bound<Version>)` pair).
    range: R,
}

/// One level of a [`Frozen`] walk's descent spine.
struct Level<T> {
    /// The branch node this level walks.
    node: Node<T>,
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

/// A live leaf popped out of a [`Frozen`] walk: an owned handle on the leaf
/// node, lending its version and value to whoever holds it.
pub struct Leaf<T>(Node<T>);

impl<T> Leaf<T> {
    /// The causal [`Version`] at which this message was observed.
    pub fn version(&self) -> &Version {
        self.0.ceiling()
    }

    /// The message's value.
    pub fn value(&self) -> &std::sync::Arc<T> {
        self.0
            .as_leaf()
            .expect("a Leaf wraps a leaf node, by construction")
            .as_arc()
    }
}

impl<T, R: RangeBounds<Version>> RangeOwned<T, R> {
    /// Walk the leaves of the (possibly absent) height-32 root `node` whose
    /// versions fall within the causal `range`.
    pub(crate) fn root(node: Option<Node<T>>, range: R) -> Self {
        Self {
            start: node,
            // One level per materialized branch along a root-to-leaf path:
            // never more than the depth, so this is the walk's only
            // allocation.
            spine: Vec::with_capacity(32),
            path: ArrayVec::new(),
            range,
        }
    }

    /// Advance to the next passing leaf. The same prune/promote resolution
    /// as the borrowing walk, with the leaf handed out by value.
    pub(crate) fn next(&mut self) -> Option<(crate::tree::key::Key, Leaf<T>)> {
        loop {
            // Obtain the next unvisited node — the initial root, or the next
            // child at the deepest spine level, ascending past exhausted
            // levels — remembering the path length to roll back to if it
            // proves not to descend.
            let (node, inherited, rollback) = match self.start.take() {
                Some(root) => (root, false, 0),
                None => loop {
                    let level = self.spine.last_mut()?;
                    let next_child = match &level.node.inner.children {
                        // Probe for the smallest not-yet-visited radix: an
                        // O(log fan-out) map lookup, so unvisited siblings
                        // are never enumerated or held.
                        Children::Branch { children, .. } if level.next <= u8::MAX as u16 => {
                            children
                                .range(level.next as u8..)
                                .next()
                                .map(|(radix, child)| (*radix, child.clone()))
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

            // Resolve this subtree against the range, unless an ancestor was
            // already promoted.
            let passes = inherited || {
                let bounds = Bounds {
                    start: self.range.start_bound(),
                    end: self.range.end_bound(),
                };
                if bounds.prunes(&node) {
                    self.path.truncate(rollback);
                    continue;
                }
                bounds.promotes(&node)
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

            // A leaf: by the prune/promote exhaustiveness argument on the
            // borrowing walk, an unpruned leaf always passes. Yield it and
            // roll the path back to its parent.
            debug_assert!(passes, "an unpruned leaf passes its range");
            debug_assert_eq!(
                self.path.len(),
                32,
                "a leaf sits at depth 32, so its path is 32 bytes"
            );
            let key = crate::tree::key::Key(self.path.into_inner());
            self.path.truncate(rollback);
            return Some((key, Leaf(node)));
        }
    }
}
