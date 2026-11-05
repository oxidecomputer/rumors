use std::sync::{Arc, OnceLock};
use std::{fmt, mem};

use itertools::{EitherOrBoth, Itertools};
use smallvec::SmallVec;
use tinyvec::ArrayVec;

use before::Span;

use super::hash::PATH_LEN;
use crate::{Version, message::Message, tree::typed::Hash};

pub mod fan;
mod iter;
use fan::Fan;
pub use iter::{Iter, Leaf, Range, RangeOwned};

/// A shared leaf or branch, with its compressed path and cached hash.
///
/// [`typed`](super) adds compile-time heights to this storage. Cloning a
/// handle shares the node; mutation copies its state when it is shared.
pub struct Node {
    /// State shared by handles to the same node.
    inner: Arc<NodeInner>,
}

/// The bytes of one node's shared allocation: the two reference counts an
/// [`Arc`] keeps beside the node's own storage.
///
/// Copying a shared node, as peeling a byte off a compressed node's path
/// does, makes one such allocation; its fan's heap slots, if it has any,
/// are [`FAN_ENTRY_BYTES`] each.
pub(crate) const NODE_ALLOCATION_BYTES: usize =
    2 * mem::size_of::<usize>() + mem::size_of::<NodeInner>();

// The reference counts are words, and the node's storage is no more aligned
// than a word, so the allocation is their sizes added with no padding.
const _: () = assert!(mem::align_of::<NodeInner>() <= mem::align_of::<usize>());

/// The radices a branch's children can sit at: one per byte value.
const RADICES: usize = u8::MAX as usize + 1;

/// The bytes of one heap slot of a branch's fan: a radix beside a child
/// handle.
pub(crate) const FAN_ENTRY_BYTES: usize = mem::size_of::<(u8, Node)>();

/// The most bytes [`Node::explosion_bytes`] reports: the copy of a shared
/// branch with a child at every radix, one node allocation beside a full
/// fan's heap slots, which grow with the fan's length.
pub(crate) const MAX_EXPLOSION_BYTES: usize =
    NODE_ALLOCATION_BYTES + fan::heap_slots(u8::MAX as usize + 1) * FAN_ENTRY_BYTES;

/// An owned, ascending walk over one node's immediate children.
///
/// An uncompressed branch stays shared while the iterator clones each child
/// handle as it reaches it. A compressed node has one virtual child and must
/// shorten its private view of the compressed prefix before yielding it.
pub(crate) enum ChildIter {
    /// The single child obtained by peeling one byte from a compressed prefix.
    Compressed(Option<(u8, Node)>),
    /// An uncompressed branch and the next radix from which to search its fan.
    Branch {
        /// The parent handle that keeps the borrowed fan alive.
        node: Node,
        /// The next radix at which to resume the walk.
        next: Option<u8>,
    },
}

/// Yield immediate children without copying an uncompressed branch's fan.
impl Iterator for ChildIter {
    /// A radix and a shared handle to its child.
    type Item = (u8, Node);

    /// Clone only the next child handle, retaining the parent for later calls.
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Compressed(child) => child.take(),
            Self::Branch { node, next } => {
                let start = (*next)?;
                let Body::Branch { children, .. } = &node.inner.body else {
                    unreachable!("a child iterator retains only an uncompressed branch")
                };
                let Some((radix, child)) = children.successor(start) else {
                    *next = None;
                    return None;
                };
                *next = radix.checked_add(1);
                Some((radix, child.clone()))
            }
        }
    }
}

/// One child a node lists, as [`Node::child_iter`] would yield it, read
/// in place.
#[derive(Clone, Copy)]
enum Listed<'a> {
    /// A bare branch's child, stored in its fan.
    Stored(&'a Node),
    /// The one child of a node with a compressed path: the node itself with
    /// its top path byte peeled. The view shares the node's body, and so
    /// its bounds and its encoded version size.
    Peeled(&'a Node),
}

impl<'a> Listed<'a> {
    /// The node whose stored memos answer for this child: the child itself,
    /// or for a peeled view, the node it was peeled from, whose body it
    /// shares.
    fn memos(self) -> &'a Node {
        match self {
            Listed::Stored(node) | Listed::Peeled(node) => node,
        }
    }

    /// Whether `candidate` is this child: the same allocation as a stored
    /// child, or for a peeled one, the node's remaining path over the same
    /// body ([`Node::is_peel_of`]).
    fn is(self, candidate: &Node) -> bool {
        match self {
            Listed::Stored(node) => node.ptr_eq(candidate),
            Listed::Peeled(node) => candidate.is_peel_of(node),
        }
    }
}

/// Share the node and count the new handle in instrumented builds.
impl Clone for Node {
    /// Clone the shared handle without copying node state.
    fn clone(&self) -> Self {
        Self::from_inner(self.inner.clone())
    }
}

/// Handles are counted, so a dropped one must check out; see [`census`].
#[cfg(any(test, feature = "test-internals"))]
impl Drop for Node {
    /// Release this handle's census entry.
    fn drop(&mut self) {
        census::dropped();
    }
}

/// Test-only census of live node handles, crate-wide.
///
/// Every [`Node`] any code holds was constructed by
/// [`Node::from_inner`] or [`Clone`] and released by [`Drop`], so the
/// pair of counters here is an exact concurrent-residency measure: `live`
/// handles exist right now, and `peak` is the most that ever existed
/// since the last reset. Tests read it to measure how many node handles a
/// session holds at once. Read through
/// [`testing::node_census`](crate::testing::node_census).
#[cfg(any(test, feature = "test-internals"))]
pub(crate) mod census {
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Handles alive right now.
    static LIVE: AtomicUsize = AtomicUsize::new(0);
    /// The most handles ever concurrently alive since the last reset.
    static PEAK: AtomicUsize = AtomicUsize::new(0);

    /// Count a new handle and update the peak.
    pub(crate) fn created() {
        let live = LIVE.fetch_add(1, Ordering::Relaxed) + 1;
        PEAK.fetch_max(live, Ordering::Relaxed);
    }

    /// Remove a released handle from the live count.
    pub(crate) fn dropped() {
        LIVE.fetch_sub(1, Ordering::Relaxed);
    }

    /// `(live, peak)` at this instant.
    pub(crate) fn read() -> (usize, usize) {
        (LIVE.load(Ordering::Relaxed), PEAK.load(Ordering::Relaxed))
    }

    /// Restart the high-water mark from the current live count.
    pub(crate) fn reset_peak() {
        PEAK.store(LIVE.load(Ordering::Relaxed), Ordering::Relaxed);
    }
}

/// Test-only counts of the work nodes do building branches and computing
/// their memos, crate-wide.
///
/// Each [`Work`] names one kind of step, counted where it happens; outside
/// test builds [`record`](memo_census::record) compiles to nothing. Tests
/// read the counts through
/// [`testing::memo_work`](crate::testing::memo_work) to see how much of
/// that work a session does, a deterministic stand-in for the time it
/// takes.
pub(crate) mod memo_census {
    #[cfg(any(test, feature = "test-internals"))]
    use std::sync::atomic::{AtomicU64, Ordering};

    /// One kind of step the census counts.
    #[derive(Clone, Copy, Debug)]
    pub(crate) enum Work {
        /// A bare branch assembled from a fan of children by
        /// [`Node::branch`](super::Node::branch), as reassembly assembles
        /// them.
        ///
        /// The subtrees a session receives are built leaf by leaf
        /// ([`SortedLeaves`](super::SortedLeaves)) and not counted.
        Branch,
        /// A node returned whole by [`Node::rebuild`](super::Node::rebuild)
        /// because its children came back unchanged.
        Reused,
        /// A hash computed rather than read from its memo.
        Hash,
        /// A branch's bounds computed from all of its children.
        BoundsFull,
        /// A rebuilt branch's bounds seeded whole from its original's.
        BoundsSeeded,
        /// A rebuilt branch's bounds seeded at one endpoint from its
        /// original's, the other folded from all of its children.
        BoundsHalf,
        /// A rebuilt branch's bounds seeded, whole or at one endpoint, with
        /// the help of an unchanged child's bound.
        BoundsBySibling,
        /// A rebuilt branch's bounds left for warming to compute from all
        /// of its children, because seeding them would not be exact.
        BoundsDeclined,
        /// A branch's encoded version size computed from all of its
        /// children.
        VersionBytesFull,
    }

    /// How many kinds of step there are.
    #[cfg(any(test, feature = "test-internals"))]
    const KINDS: usize = Work::VersionBytesFull as usize + 1;

    /// The count of each kind of step since the last reset.
    #[cfg(any(test, feature = "test-internals"))]
    static COUNTS: [AtomicU64; KINDS] = [const { AtomicU64::new(0) }; KINDS];

    /// Count one step of `work`.
    #[inline]
    pub(crate) fn record(work: Work) {
        #[cfg(any(test, feature = "test-internals"))]
        COUNTS[work as usize].fetch_add(1, Ordering::Relaxed);
        #[cfg(not(any(test, feature = "test-internals")))]
        let _ = work;
    }

    /// The steps of `work` counted since the last reset.
    #[cfg(any(test, feature = "test-internals"))]
    pub(crate) fn read(work: Work) -> u64 {
        COUNTS[work as usize].load(Ordering::Relaxed)
    }

    /// Restart every count from zero.
    #[cfg(any(test, feature = "test-internals"))]
    pub(crate) fn reset() {
        for count in &COUNTS {
            count.store(0, Ordering::Relaxed);
        }
    }
}
use memo_census::Work;

/// A branch's grandchild count, less one, from its fan
/// ([`Body::Branch`]'s `spread`).
///
/// A fan of two to 256 children, each weighing one to 256, sums to at most
/// 65,536, so the stored value fits a `u16` exactly.
fn spread(children: &Fan) -> u16 {
    debug_assert!(children.len() >= 2, "a branch has at least two children");
    let grandchildren: usize = children.values().map(Node::weight).sum();
    u16::try_from(grandchildren - 1).expect("a branch has at most 65,536 grandchildren")
}

/// Shared node storage, copied only when a mutation cannot reuse it.
#[derive(Clone)]
struct NodeInner {
    /// Path compressed above this leaf or branch, deepest byte first.
    /// Pushing or popping the last byte adds or removes its topmost level.
    ///
    /// The path is at most 32 bytes, so storing it inline avoids a separate
    /// allocation for compressed nodes, at the cost of a larger node body.
    prefix: ArrayVec<[u8; PATH_LEN]>,
    /// Subtree hash as seen from the top of `prefix`, computed on first use.
    /// Both leaves and branches cache it. Cloning preserves the cached value;
    /// changing either `prefix` or `body` must clear it.
    hash: OnceLock<Hash>,
    /// The leaf or branch stored beneath the prefix.
    body: Body,
}

/// [`NodeInner`] as it would be without the grandchild count: the same
/// fields, with [`BodyWithoutSpread`] for its body.
///
/// Nothing builds one; it exists so the compiler lays out the node without
/// `spread`, for the assertion below to compare against.
#[expect(dead_code, reason = "a layout reference, measured and never built")]
struct NodeInnerWithoutSpread {
    /// As [`NodeInner`]'s `prefix`.
    prefix: ArrayVec<[u8; PATH_LEN]>,
    /// As [`NodeInner`]'s `hash`.
    hash: OnceLock<Hash>,
    /// As [`NodeInner`]'s `body`, without `spread`.
    body: BodyWithoutSpread,
}

/// [`Body`] as it would be without `spread`: every variant and field but
/// the branch's grandchild count.
#[expect(dead_code, reason = "a layout reference, measured and never built")]
enum BodyWithoutSpread {
    /// As [`Body::Leaf`].
    Leaf {
        /// As [`Body::Leaf`]'s `version`.
        version: Version,
        /// As [`Body::Leaf`]'s `message`.
        message: Message,
    },
    /// As [`Body::Branch`], without `spread`.
    Branch {
        /// As [`Body::Branch`]'s `bounds`.
        bounds: OnceLock<Span<'static>>,
        /// As [`Body::Branch`]'s `leaves`.
        leaves: usize,
        /// As [`Body::Branch`]'s `version_bytes`.
        version_bytes: OnceLock<usize>,
        /// As [`Body::Branch`]'s `children`.
        children: Fan,
    },
}

// Every node pays for the grandchild count, leaves included, unless it fits
// in padding the node already has. It does on the platforms the crate is
// measured on, and this keeps it so: a platform where the count grows the
// node fails here, so the cost is a decision rather than an unnoticed
// change. A field added to `NodeInner` or `Body` belongs in the twins above
// as well, or this fails for that reason instead.
const _: () = assert!(
    mem::size_of::<NodeInner>() == mem::size_of::<NodeInnerWithoutSpread>(),
    "the branch's grandchild count must fit in the node's padding",
);

/// Display node content without evaluating its cached summaries.
impl fmt::Debug for Node {
    /// Format the prefix in path order, followed by leaf or branch content.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let prefix: ArrayVec<[u8; PATH_LEN]> = self.inner.prefix.iter().rev().copied().collect();
        f.debug_struct("Node")
            .field("prefix", &hex::encode(prefix))
            .field("body", &self.inner.body)
            .finish()
    }
}

/// Content stored beneath a node's compressed prefix.
///
/// Cloning shares leaf data and child nodes and preserves computed summaries.
/// Those summaries depend only on the subtree, so changing children must
/// clear them; changing the prefix leaves them valid.
#[derive(Debug, Clone)]
enum Body {
    /// A leaf's version and payload.
    Leaf {
        /// The version of this leaf.
        version: Version,
        /// The payload of this leaf.
        message: Message,
    },
    /// At least two children; singleton branches are folded into the prefix.
    Branch {
        /// The tightest span containing all leaf versions below this branch:
        /// its floor is their meet, its ceiling their join.
        ///
        /// It is computed on first read, or at construction when
        /// [`Node::rebuild`] can derive it exactly from the node it
        /// replaces. [`Span`] keeps the pair ordered by construction.
        bounds: OnceLock<Span<'static>>,
        /// The number of leaves below this branch.
        leaves: usize,
        /// This branch's grandchild count as a bare branch, `S`, stored as
        /// `S − 1`: over its children, each one's own child count, a child
        /// at the leaves counting one ([`Node::grandchild_count`]).
        ///
        /// A branch has two to 256 children, each counting one to 256, so
        /// `S` lies in 2..=65,536 and `S − 1` fits exactly. It depends only
        /// on the fan, which never changes after construction; peeling or
        /// extending the prefix leaves it valid.
        spread: u16,
        /// The largest encoded version size in this subtree: leaf versions
        /// and branch bounds, including this branch's own bounds. Computing
        /// it also fills `bounds`; later reads use the cached size.
        version_bytes: OnceLock<usize>,
        /// The children of this branch.
        children: Fan,
    },
}

/// Construct, query, and reshape storage nodes.
impl Node {
    /// Wrap built node state as a handle.
    ///
    /// Every handle in the crate passes through here or [`Clone`] — the
    /// funnel that makes the test-only [`census`] an exact residency
    /// count.
    fn from_inner(inner: Arc<NodeInner>) -> Self {
        #[cfg(any(test, feature = "test-internals"))]
        census::created();
        Node { inner }
    }

    /// Assemble children at distinct radixes into a subtree.
    ///
    /// Empty input returns `None`. A single child absorbs its radix into its
    /// prefix; only two or more children require a branch node. This reverses
    /// [`into_children`](Self::into_children).
    ///
    /// A branch node stores its fan [fitted](Fan::fit), releasing any heap
    /// slots beyond [`heap_slots`](fan::heap_slots) of its length.
    pub fn branch(children: Fan) -> Option<Self> {
        match children.len() {
            0 => None,
            1 => {
                let Some((index, node)) = children.into_iter().next() else {
                    unreachable!("a fan with 1 element cannot fail to iterate");
                };
                Some(node.beneath(index))
            }
            _ => Some(Node::bare_branch(children, OnceLock::new())),
        }
    }

    /// Store children, at least two, as a bare branch whose bounds are
    /// `bounds`, either computed already or left for their first read.
    ///
    /// A branch node stores its fan [fitted](Fan::fit).
    fn bare_branch(mut children: Fan, bounds: OnceLock<Span<'static>>) -> Node {
        debug_assert!(
            children.len() >= 2,
            "a bare branch has at least two children"
        );
        memo_census::record(Work::Branch);
        children.fit();
        Node::from_inner(Arc::new(NodeInner {
            prefix: ArrayVec::new(),
            hash: OnceLock::new(),
            body: Body::Branch {
                bounds,
                leaves: children.values().map(Node::len).sum(),
                spread: spread(&children),
                // Never seeded: once the bounds are known, the full
                // computation reads one stored size per child and the
                // bounds' own two encodings, at most 256 cached loads,
                // exact by construction.
                version_bytes: OnceLock::new(),
                children,
            },
        }))
    }

    /// Assemble children into a subtree as [`branch`](Self::branch) does,
    /// reusing `original`, the node the children were listed from before
    /// any of them changed.
    ///
    /// When the children are exactly those `original` lists, at the same
    /// radices, this returns `original` itself, its hash and every other
    /// memo intact, so reassembling an unchanged scope allocates nothing and
    /// computes no memo. Otherwise it builds the node `branch` builds; when
    /// that is a bare branch, it seeds the new node's bounds from
    /// `original`'s wherever that is exact ([`seeded_bounds`]).
    ///
    /// A stored child counts as unchanged only as the same allocation. A
    /// compressed node lists a peeled view of itself, which the walk's
    /// explosion builds fresh, so that child counts as unchanged when it has
    /// the node's remaining path over the same body ([`Self::is_peel_of`]).
    pub fn rebuild(original: Option<Node>, children: Fan) -> Option<Self> {
        let Some(original) = original else {
            return Node::branch(children);
        };
        // An empty group deletes the scope, whatever stood there.
        if children.len() == 0 {
            return None;
        }
        if original.lists_exactly(&children) {
            memo_census::record(Work::Reused);
            return Some(original);
        }
        if children.len() < 2 {
            // One child: `branch` extends its path, which keeps its memos.
            return Node::branch(children);
        }
        let bounds = seeded_bounds(&original, &children);
        Some(Node::bare_branch(children, bounds))
    }

    /// Whether `children` are exactly the children
    /// [`child_iter`](Self::child_iter) lists for this node: the same
    /// radices, each child the one listed there ([`Listed::is`]).
    fn lists_exactly(&self, children: &Fan) -> bool {
        let mut listed = self.listed();
        children.iter().all(|(radix, child)| {
            listed
                .next()
                .is_some_and(|(at, own)| at == radix && own.is(child))
        }) && listed.next().is_none()
    }

    /// The children [`child_iter`](Self::child_iter) lists for this node,
    /// read in place, in radix order: a bare branch's fan, a compressed
    /// node's one peeled view, and none for a bare leaf.
    fn listed(&self) -> impl Iterator<Item = (u8, Listed<'_>)> {
        let peeled = self
            .inner
            .prefix
            .last()
            .map(|&top| (top, Listed::Peeled(self)));
        let fan = match (peeled, &self.inner.body) {
            (None, Body::Branch { children, .. }) => Some(
                children
                    .iter()
                    .map(|(radix, child)| (radix, Listed::Stored(child))),
            ),
            _ => None,
        };
        peeled.into_iter().chain(fan.into_iter().flatten())
    }

    /// Whether this node is `parent` with its top path byte peeled: the
    /// rest of `parent`'s path over the same body, and so the same subtree
    /// as the child `parent`'s listing yields.
    ///
    /// Peeling copies a shared node, so the view is a separate allocation
    /// whose body copies `parent`'s: the same leaf, or a fan of the same
    /// child handles. Comparing those decides it, in at most a fan's pointer
    /// comparisons. A leaf is identified by its version, as everywhere in
    /// the tree: no two messages share one.
    fn is_peel_of(&self, parent: &Node) -> bool {
        let Some((_, below)) = parent.inner.prefix.split_last() else {
            return false;
        };
        self.inner.prefix.as_slice() == below
            && match (&self.inner.body, &parent.inner.body) {
                (
                    Body::Leaf { version: ours, .. },
                    Body::Leaf {
                        version: theirs, ..
                    },
                ) => ours == theirs,
                (
                    Body::Branch { children: ours, .. },
                    Body::Branch {
                        children: theirs, ..
                    },
                ) => {
                    ours.len() == theirs.len()
                        && ours
                            .iter()
                            .zip(theirs.iter())
                            .all(|((a, x), (b, y))| a == b && x.ptr_eq(y))
                }
                _ => false,
            }
    }

    /// This node's bounds if already computed: a leaf's always are, a
    /// branch's once first read.
    fn computed_span(&self) -> Option<Span<'_>> {
        match &self.inner.body {
            Body::Leaf { version, .. } => Some(Span::at(version)),
            Body::Branch { bounds, .. } => bounds.get().map(Span::reborrow),
        }
    }

    /// Convert a node into its radix fan of children (inverse to
    /// [`Node::branch`]).
    ///
    /// If `self` is a leaf node, returns `Err(self)`.
    pub fn into_children(mut self) -> Result<Fan, Node> {
        if !self.inner.prefix.is_empty() {
            // Path-compressed: pop the top (shallowest) byte and rewrap self
            // under it. Popping shortens the prefix, so the observable hash
            // moves down one virtual level; the memoized hash is now stale and
            // must be cleared so the next read recomputes from the shortened
            // prefix.
            let inner = Arc::make_mut(&mut self.inner);
            let index = inner.prefix.pop().expect("non-empty prefix");
            inner.hash = OnceLock::new();
            Ok(Fan::unit(index, self))
        } else {
            match &self.inner.body {
                Body::Leaf { .. } => Err(self),
                Body::Branch { .. } => {
                    // Extract the children fan; self is dropped, so leaving
                    // its precomputed metadata referencing the now-vacated
                    // branch is harmless.
                    let inner = Arc::make_mut(&mut self.inner);
                    let Body::Branch {
                        children: branch, ..
                    } = &mut inner.body
                    else {
                        unreachable!("just matched Branch")
                    };
                    Ok(mem::take(branch))
                }
            }
        }
    }

    /// Walk immediate children while keeping an uncompressed branch shared.
    ///
    /// Unlike [`into_children`](Self::into_children), this does not copy the
    /// node and its fan when another handle shares the branch. Each yielded
    /// child is instead one cheap shared handle. Peeling a compressed prefix
    /// still needs a distinct node view because its hash and prefix change.
    pub(crate) fn child_iter(mut self) -> Result<ChildIter, Node> {
        if !self.inner.prefix.is_empty() {
            let inner = Arc::make_mut(&mut self.inner);
            let radix = inner.prefix.pop().expect("non-empty prefix");
            inner.hash = OnceLock::new();
            return Ok(ChildIter::Compressed(Some((radix, self))));
        }

        match &self.inner.body {
            Body::Leaf { .. } => Err(self),
            Body::Branch { .. } => Ok(ChildIter::Branch {
                node: self,
                next: Some(0),
            }),
        }
    }

    /// Build the maximally-compressed subtree over one sorted leaf run.
    ///
    /// `leaves` pairs each full 32-byte path with its bare (prefix-free)
    /// leaf node, **strictly ascending by path**, every path sharing its
    /// first `depth` bytes; the run must be non-empty, and each node is
    /// consumed exactly once (the `Option` lets the recursion move nodes
    /// out of a shared slice). The result observes the tree from depth
    /// `depth` — i.e. it sits at height `32 - depth`.
    ///
    /// This is the bulk inverse of a leaf walk: where composing
    /// [`branch`](Self::branch)/[`beneath`](Self::beneath) level by level
    /// costs an allocation per *virtual* level, this jumps straight to
    /// each divergence byte (sorted input makes it the first/last path
    /// comparison) and lays down every compressed span in one step, so
    /// the work is proportional to the *materialized* structure: one node
    /// per real branch point plus one per leaf spine.
    ///
    /// The session builds supplied subtrees one leaf at a time instead
    /// ([`SortedLeaves`]), which builds this same tree; this constructor is
    /// the reference that builder is tested against.
    ///
    /// # Panics
    ///
    /// Panics if the run is empty, contains duplicate paths or a consumed
    /// slot, or supplies a node that is not a bare leaf.
    #[cfg(test)]
    pub(crate) fn from_sorted_leaves(
        depth: usize,
        leaves: &mut [([u8; PATH_LEN], Option<Self>)],
    ) -> Self {
        debug_assert!(
            leaves.windows(2).all(|pair| pair[0].0 < pair[1].0),
            "a leaf run is strictly ascending by path",
        );
        if let [(path, node)] = leaves {
            let mut node = node
                .take()
                .expect("each leaf node is consumed exactly once");
            debug_assert!(
                node.inner.prefix.is_empty() && node.is_leaf(),
                "a leaf run supplies bare leaf nodes",
            );
            if depth < path.len() {
                // A lone leaf compresses its whole remaining spine into the
                // prefix (stored deepest-first), in one extension. The node
                // now observes from a shallower level, so any memoized hash
                // would be stale; freshly-built leaves have none, but a
                // reused bare handle may.
                let inner = Arc::make_mut(&mut node.inner);
                inner.prefix.extend(path[depth..].iter().rev().copied());
                inner.hash = OnceLock::new();
            }
            return node;
        }

        // Two or more distinct sorted paths diverge at the first byte where
        // the least and greatest differ; everything from `depth` up to that
        // byte is common to the whole run and compresses into the prefix.
        let first = leaves.first().expect("a leaf run is non-empty").0;
        let last = leaves.last().expect("a leaf run is non-empty").0;
        let branch_at = (depth..PATH_LEN)
            .find(|&at| first[at] != last[at])
            .expect("distinct 32-byte paths diverge before the bottom");

        let count = leaves.len();
        let mut children = Fan::new();
        let mut rest = leaves;
        while let Some(radix) = rest.first().map(|(path, _)| path[branch_at]) {
            let split = rest
                .iter()
                .position(|(path, _)| path[branch_at] != radix)
                .unwrap_or(rest.len());
            let (group, tail) = mem::take(&mut rest).split_at_mut(split);
            // Groups peel off in ascending radix order, so each child is an
            // O(1) append.
            children.push(radix, Self::from_sorted_leaves(branch_at + 1, group));
            rest = tail;
        }
        debug_assert!(children.len() >= 2, "a branch point separates >= 2 runs");
        // Pushes alone keep the fan within its slot bound, so this never
        // reallocates; fitting anyway keeps the bound local to every branch
        // constructor rather than resting on the fan's growth policy.
        children.fit();

        Node::from_inner(Arc::new(NodeInner {
            prefix: first[depth..branch_at].iter().rev().copied().collect(),
            hash: OnceLock::new(),
            body: Body::Branch {
                bounds: OnceLock::new(),
                leaves: count,
                spread: spread(&children),
                version_bytes: OnceLock::new(),
                children,
            },
        }))
    }

    /// Construct a new leaf node.
    pub fn leaf(version: Version, value: Message) -> Self {
        Node::from_inner(Arc::new(NodeInner {
            prefix: ArrayVec::new(),
            hash: OnceLock::new(),
            body: Body::Leaf {
                message: value,
                version,
            },
        }))
    }

    /// Get a reference to the leaf at this node, if it is a leaf.
    pub fn as_leaf(&self) -> Option<&Message> {
        match &self.inner.body {
            Body::Leaf { message, .. } => Some(message),
            _ => None,
        }
    }

    /// Look up the leaf at `path` beneath this node: a single root-to-leaf
    /// descent costing `O(depth)`, never a scan. `None` when no live leaf
    /// sits at that path.
    pub fn get(&self, mut path: &[u8]) -> Option<(&Version, &Message)> {
        let mut node = self;
        loop {
            // Consume the compressed prefix, shallowest byte first (it is
            // stored shallowest-last); any divergence means the path exits
            // the tree inside the compressed span.
            for &byte in node.inner.prefix.iter().rev() {
                match path.split_first() {
                    Some((&next, rest)) if next == byte => path = rest,
                    _ => return None,
                }
            }
            match &node.inner.body {
                // The supplied suffix must end exactly at the leaf.
                Body::Leaf { version, message } => {
                    return path.is_empty().then_some((version, message));
                }
                Body::Branch { children, .. } => {
                    let (radix, rest) = path.split_first()?;
                    node = children.get(*radix)?;
                    path = rest;
                }
            }
        }
    }

    /// Get the number of leaves under a node.
    pub fn len(&self) -> usize {
        match self.inner.body {
            Body::Leaf { .. } => 1,
            Body::Branch { leaves, .. } => leaves,
        }
    }

    /// The number of children this node's children have, as
    /// [`child_iter`](Self::child_iter) would list them.
    ///
    /// That is, over its children, each one's own child count, a child at
    /// the leaves counting one. A bare leaf, which has no children, has
    /// none.
    ///
    /// Exact, and O(1): a node with a compressed path of two bytes or more
    /// peels to one child that itself peels to one, so it has one; a node
    /// with one byte of path over a branch has that branch's fan as its
    /// grandchildren, and over a leaf has the leaf alone; a bare branch
    /// stores its count.
    pub fn grandchild_count(&self) -> usize {
        match (self.inner.prefix.len(), &self.inner.body) {
            (0, Body::Leaf { .. }) => 0,
            (0, Body::Branch { spread, .. }) => usize::from(*spread) + 1,
            (1, Body::Leaf { .. }) => 1,
            (1, Body::Branch { children, .. }) => children.len(),
            _ => 1,
        }
    }

    /// Check [`grandchild_count`](Self::grandchild_count) against a direct count
    /// at this node and every node beneath it, returning the first that
    /// disagrees.
    ///
    /// Each peeled view of a compressed path counts as a node. A
    /// disagreement comes back with the stored and the counted values.
    ///
    /// The count lists each node's children with
    /// [`child_iter`](Self::child_iter) and each child's own children the
    /// same way. The traversal keeps an explicit stack, bounded by the key
    /// depth times the fan, so it never recurses on the tree's shape.
    #[cfg(test)]
    pub(crate) fn miscounted_grandchildren(&self) -> Option<(Node, usize, usize)> {
        let mut stack = vec![self.clone()];
        while let Some(node) = stack.pop() {
            let Ok(children) = node.clone().child_iter() else {
                if node.grandchild_count() != 0 {
                    return Some((node.clone(), node.grandchild_count(), 0));
                }
                continue;
            };
            let mut counted = 0;
            for (_, child) in children {
                counted += child.clone().child_iter().map_or(1, Iterator::count);
                stack.push(child);
            }
            if node.grandchild_count() != counted {
                return Some((node.clone(), node.grandchild_count(), counted));
            }
        }
        None
    }

    /// The bytes [`child_iter`](Self::child_iter) allocates to list this
    /// node's children, beyond the handles it yields: exact, absent other
    /// threads releasing handles to the node meanwhile.
    ///
    /// Listing a bare branch's children shares their handles and allocates
    /// nothing. Listing a compressed node's one child peels a byte off its
    /// path, which changes the node, so a node another handle shares is
    /// copied first: one node allocation, and for a branch its fan's heap
    /// slots, which the copy's fan holds at [`fan::heap_slots`] of its
    /// length. A node no other handle shares is peeled in place. A handle
    /// released elsewhere between this call and the listing can only turn a
    /// copy into an in-place peel, so the answer never falls short.
    pub fn explosion_bytes(&self) -> usize {
        if self.inner.prefix.is_empty() || Arc::strong_count(&self.inner) == 1 {
            return 0;
        }
        let fan = match &self.inner.body {
            Body::Leaf { .. } => 0,
            Body::Branch { children, .. } => fan::heap_slots(children.len()) * FAN_ENTRY_BYTES,
        };
        NODE_ALLOCATION_BYTES + fan
    }

    /// The number of children [`child_iter`](Self::child_iter) would list.
    ///
    /// Exact, and O(1): a bare branch has its fan, a node with a compressed
    /// path peels to a single child, and a bare leaf has none.
    pub fn child_count(&self) -> usize {
        match (self.inner.prefix.is_empty(), &self.inner.body) {
            (true, Body::Leaf { .. }) => 0,
            (true, Body::Branch { children, .. }) => children.len(),
            (false, _) => 1,
        }
    }

    /// The radices [`child_iter`](Self::child_iter) would list, in ascending
    /// order: a bare branch's fan, a compressed node's one, the last byte
    /// of its path, and none for a bare leaf.
    ///
    /// Reads them in place: no handle is cloned and no node is peeled.
    pub fn child_radices(&self) -> impl Iterator<Item = u8> + '_ {
        let fan = match (self.inner.prefix.last(), &self.inner.body) {
            (None, Body::Branch { children, .. }) => Some(children.iter().map(|(radix, _)| radix)),
            _ => None,
        };
        self.inner
            .prefix
            .last()
            .copied()
            .into_iter()
            .chain(fan.into_iter().flatten())
    }

    /// How many children this node has, as [`child_iter`](Self::child_iter)
    /// would list them, counting a bare leaf as one: what it contributes to
    /// its parent's grandchild count.
    fn weight(&self) -> usize {
        self.child_count().max(1)
    }

    /// The largest canonical [`Version`] encoding among every bound this
    /// subtree holds — its leaf versions and every branch's ceiling and
    /// floor — in bytes.
    ///
    /// Exact, never a high-water mark: a leaf answers with its own
    /// version's packed length, and a branch memoizes the max over its
    /// children's values and its own two bounds, computed lazily on
    /// first read like [`ceiling`](Self::ceiling) (which it forces).
    /// Every mutation rebuilds its copy-on-write spine through the
    /// branch constructors with fresh memos, so deleting the version
    /// that carries the maximum resizes the aggregate down with no
    /// separate invalidation, exactly like `len`. Interior bounds must
    /// be covered because a join over many small concurrent leaf stamps
    /// can encode several times larger than any one of them — the
    /// aggregate answers for what the tree *holds*, not only what its
    /// leaves carry.
    ///
    /// The first read after loading a large corpus materializes the
    /// subtree's bounds once; a session forces the same memos along
    /// every divergent path it walks, and they are shared through the
    /// copy-on-write clones, so subsequent reads cost the freshly
    /// rebuilt spine only.
    pub fn version_bytes(&self) -> usize {
        match &self.inner.body {
            Body::Leaf { version, .. } => version.as_bytes().len(),
            Body::Branch {
                version_bytes,
                children,
                ..
            } => *version_bytes.get_or_init(|| {
                memo_census::record(Work::VersionBytesFull);
                children
                    .values()
                    .map(Node::version_bytes)
                    .max()
                    .expect("a branch always has >= 2 children")
                    .max(self.ceiling().as_bytes().len())
                    .max(self.floor().as_bytes().len())
            }),
        }
    }

    /// Whether two nodes share the same backing allocation: a sufficient
    /// (not necessary) test for structural equality that touches no hash.
    ///
    /// Forked trees share their unchanged subtrees by `Arc`, so an in-memory
    /// merge can short-circuit those in `O(1)`, even with cold memos, before
    /// falling back to the Merkle hash for subtrees that diverged in memory
    /// but hold the same version set.
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }

    /// Hash the subtree rooted at this node, as observed from the top of its
    /// compressed prefix.
    ///
    /// The hash is computed lazily on first call and memoized, so the first
    /// read of a freshly-built subtree is `O(nodes)` — one preimage and one
    /// SHA-256 pass per node — and every read thereafter is an `O(1)` field
    /// load. The convention (see [`Hash::leaf`] and [`Hash::branch`]): a
    /// single preimage commits the node's kind, its compressed prefix in
    /// path order, and, for a branch, its children as ascending
    /// `radix ‖ hash` records. Hash agreement between differently-built
    /// trees rests on the tree's canonical shape; see [`Hash::branch`]'s
    /// canonicity section.
    pub fn hash(&self) -> Hash {
        *self.inner.hash.get_or_init(|| {
            memo_census::record(Work::Hash);
            // The preimage takes the prefix in path order — shallowest byte
            // first — while storage is shallowest-last, so reverse into a
            // stack buffer (a compressed span never exceeds the 32-byte
            // path).
            let prefix: ArrayVec<[u8; PATH_LEN]> =
                self.inner.prefix.iter().rev().copied().collect();
            match &self.inner.body {
                Body::Leaf { .. } => Hash::leaf(&prefix),
                Body::Branch { children, .. } => Hash::branch(
                    &prefix,
                    children.iter().map(|(radix, child)| (radix, child.hash())),
                ),
            }
        })
    }

    /// Get the ceiling version of this node (the maximal version of all
    /// children).
    ///
    /// Like [`hash`](Self::hash), the ceiling is computed on first call and
    /// memoized: a leaf's is set at construction, and a branch's is the
    /// join endpoint of its memoized bounds span, so reading either bound
    /// computes both, unless reassembly derived them when it built the
    /// branch ([`rebuild`](Self::rebuild)). The memo is a pure function of the
    /// subtree, so it is safe to share across the structurally-shared
    /// clones a forked tree produces.
    pub fn ceiling(&self) -> &Version {
        match &self.inner.body {
            Body::Leaf { version, .. } => version,
            Body::Branch {
                bounds, children, ..
            } => Self::bounds(bounds, children).hi(),
        }
    }

    /// Get the floor version of this node (the minimal version of all
    /// children).
    ///
    /// Like [`hash`](Self::hash), the floor is computed on first call and
    /// memoized: a leaf's is set at construction, and a branch's is the
    /// meet endpoint of its memoized bounds span, so reading either bound
    /// computes both, unless reassembly derived them when it built the
    /// branch ([`rebuild`](Self::rebuild)). The memo is a pure function of the
    /// subtree, so it is safe to share across the structurally-shared
    /// clones a forked tree produces.
    pub fn floor(&self) -> &Version {
        match &self.inner.body {
            Body::Leaf { version, .. } => version,
            Body::Branch {
                bounds, children, ..
            } => Self::bounds(bounds, children).lo(),
        }
    }

    /// This subtree's version bounds as one causal span: the memoized
    /// `[floor, ceiling]` pair, borrowed.
    ///
    /// A branch reborrows its stored span. A leaf uses [`Span::at`] because
    /// both bounds equal its version. Reading a branch span fills the same
    /// memo shared by [`ceiling`](Self::ceiling) and [`floor`](Self::floor).
    pub fn span(&self) -> Span<'_> {
        match &self.inner.body {
            Body::Leaf { version, .. } => Span::at(version),
            Body::Branch {
                bounds, children, ..
            } => Self::bounds(bounds, children).reborrow(),
        }
    }

    /// Compute the tightest span containing every leaf version below a branch.
    ///
    /// A fringe branch uses [`Version::span_all`] to derive both bounds in one
    /// pass over its leaf versions. An interior branch uses [`Span::union_all`]
    /// over child spans, reusing their cached bounds. Both balanced folds avoid
    /// repeatedly combining one growing accumulator with the next child.
    /// The fold cannot be empty: canonical branches have at least two children.
    fn bounds<'a>(bounds: &'a OnceLock<Span<'static>>, children: &Fan) -> &'a Span<'static> {
        bounds.get_or_init(|| {
            memo_census::record(Work::BoundsFull);
            if children.values().all(Node::is_leaf) {
                let mut versions = children.values().map(|child| match &child.inner.body {
                    Body::Leaf { version, .. } => version,
                    Body::Branch { .. } => {
                        unreachable!("every child of a fringe branch is a leaf")
                    }
                });
                let first = versions.next().expect("a branch always has >= 2 children");
                first.span_all(versions)
            } else {
                let mut spans = children.values().map(Node::span);
                let first = spans.next().expect("a branch always has >= 2 children");
                first.union_all(spans)
            }
        })
    }

    /// The largest canonical encoding among every version this subtree
    /// holds — leaf versions plus every branch's ceiling and floor —
    /// recomputed by direct walk with no aggregate memo consulted.
    ///
    /// Test instrumentation: the independent oracle the aggregate
    /// proptests and the census pin hold
    /// [`version_bytes`](Self::version_bytes) against — the two must
    /// agree on every tree, and this side derives the answer from the
    /// bounds alone. Materialized depth is at most the 32-byte path, so
    /// the recursion is stack-safe.
    #[cfg(test)]
    pub fn max_bound_bytes(&self) -> usize {
        match &self.inner.body {
            Body::Leaf { version, .. } => version.as_bytes().len(),
            Body::Branch { children, .. } => self
                .ceiling()
                .as_bytes()
                .len()
                .max(self.floor().as_bytes().len())
                .max(
                    children
                        .values()
                        .map(Node::max_bound_bytes)
                        .max()
                        .unwrap_or_default(),
                ),
        }
    }

    /// The tightest span containing every leaf version beneath this node,
    /// computed by direct walk, consulting no memo: each version folded in
    /// turn into a running meet and join.
    ///
    /// Test instrumentation: the independent oracle stored bounds are held
    /// against ([`mismemoized`](Self::mismemoized)). It shares neither the
    /// memos nor the balanced folds the bounds are computed with. The
    /// traversal keeps an explicit stack.
    #[cfg(test)]
    pub(crate) fn walked_span(&self) -> Span<'static> {
        let mut versions = Vec::new();
        let mut stack = vec![self];
        while let Some(node) = stack.pop() {
            match &node.inner.body {
                Body::Leaf { version, .. } => versions.push(version),
                Body::Branch { children, .. } => stack.extend(children.values()),
            }
        }
        let (first, rest) = versions.split_first().expect("every node holds a leaf");
        let (lo, hi) = rest
            .iter()
            .fold(((*first).clone(), (*first).clone()), |(lo, hi), version| {
                (lo.meet(version), hi.join(version))
            });
        Span::new(lo, hi).expect("the meet of versions is at or below their join")
    }

    /// The first node at or beneath this one whose bounds differ from a
    /// direct walk ([`walked_span`](Self::walked_span)) or whose encoded
    /// version size differs from one ([`max_bound_bytes`](Self::max_bound_bytes)).
    #[cfg(test)]
    pub(crate) fn mismemoized(&self) -> Option<Node> {
        let mut stack = vec![self.clone()];
        while let Some(node) = stack.pop() {
            if node.span() != node.walked_span() || node.version_bytes() != node.max_bound_bytes() {
                return Some(node);
            }
            if let Body::Branch { children, .. } = &node.inner.body {
                stack.extend(children.values().cloned());
            }
        }
        None
    }

    /// The fan a branch stores, or `None` for a leaf.
    #[cfg(test)]
    pub(crate) fn stored_fan(&self) -> Option<&Fan> {
        match &self.inner.body {
            Body::Leaf { .. } => None,
            Body::Branch { children, .. } => Some(children),
        }
    }

    /// Inspect stored memos without computing them, including every descendant.
    #[cfg(test)]
    pub(crate) fn memos_are_warm(&self) -> bool {
        self.inner.hash.get().is_some()
            && match &self.inner.body {
                Body::Leaf { .. } => true,
                Body::Branch {
                    bounds,
                    version_bytes,
                    children,
                    ..
                } => {
                    bounds.get().is_some()
                        && version_bytes.get().is_some()
                        && children.values().all(Node::memos_are_warm)
                }
            }
    }

    /// Whether this node's content is a single leaf (regardless of any
    /// path-compressed prefix above it).
    ///
    /// A leaf carries exactly one version, so its [`floor`](Self::floor) and
    /// [`ceiling`](Self::ceiling) coincide — which lets callers decide "keep
    /// or drop this whole subtree" from the version check alone, without
    /// exploding the compressed prefix.
    pub fn is_leaf(&self) -> bool {
        matches!(self.inner.body, Body::Leaf { .. })
    }

    /// Number of levels compressed above this leaf or branch.
    #[cfg(test)]
    pub fn compressed_prefix_len(&self) -> usize {
        self.inner.prefix.len()
    }

    /// Place a node beneath the given child index, increasing its height by
    /// one.
    ///
    /// Pushing onto the prefix raises the observable hash by one virtual
    /// level, so the memoized hash is invalidated and recomputed lazily on the
    /// next read.
    pub fn beneath(mut self, index: u8) -> Node {
        let inner = Arc::make_mut(&mut self.inner);
        inner.prefix.push(index);
        inner.hash = OnceLock::new();
        self
    }

    /// Return `true` if no node in the tree violates path compression: every
    /// branch must have at least two children.
    ///
    /// The empty tree is represented by the absence of a root, so empty and
    /// one-child branches are never valid anywhere in the tree.
    #[cfg(test)]
    fn is_max_compressed(&self) -> bool {
        match &self.inner.body {
            Body::Leaf { .. } => true,
            Body::Branch { children, .. } => {
                children.len() >= 2 && children.values().all(Self::is_max_compressed)
            }
        }
    }
}

/// The bounds of the bare branch over `children`, derived from those of
/// `original`, the node the children were listed from, wherever that is
/// exact.
///
/// Otherwise the memo is left empty, and warming fills it by computing the
/// bounds from every child.
///
/// # Exactness
///
/// A branch's span is the union of its children's spans: its floor is the
/// meet of their floors, and its ceiling the join of their ceilings. Call
/// the new branch `N` and `original` `O`. Split `N`'s children into `U`,
/// those `O` lists unchanged at the same radix, and `D`, those that differ
/// or are new; and call `R` the children of `O` that changed or are gone.
/// Let `T` be the union of `D`'s spans. Union is associative, commutative
/// and idempotent, so
///
/// ```text
/// span(O) ∪ T = span(U) ∪ span(R) ∪ T     span(N) = span(U) ∪ T
/// ```
///
/// and the two agree exactly when every span in `R` lies within span(N).
/// The endpoints answer separately. `N`'s ceiling is `O`'s joined with
/// `T`'s exactly when every `r` in `R` has a ceiling at or below `N`'s;
/// `N`'s floor is `O`'s met with `T`'s exactly when every `r` has a floor
/// at or above `N`'s.
///
/// The ceiling of `r` is *certified* at or below `N`'s by any bound known to
/// be at or below `N`'s ceiling and at or above `r`'s: `T`'s ceiling, or the
/// ceiling of any child in `U`. A floor is certified likewise, by `T`'s
/// floor or any `U` child's. Certificates are sufficient, not necessary: a
/// ceiling above every certificate can still lie below the join of
/// several.
///
/// An endpoint whose every `r` is certified is seeded, at the cost of one
/// join or meet. An endpoint with an uncertified `r` is folded over all of
/// `N`'s children, half the full computation. When neither is certified,
/// the memo is left for the full computation, which folds both endpoints
/// in one pass.
///
/// # What is read
///
/// `O`'s bounds and its children's are read only if already computed:
/// computing them to seed would cost a full pass over the old node, more
/// than computing the new one directly. `D`'s bounds are computed if they
/// are not yet: publication computes the bounds of every node it
/// publishes, so this moves that work rather than adding it, and lets
/// seeding continue up the rebuilt path. `U`'s are read as certificates
/// only if already computed, and computed only where an endpoint is folded,
/// as warming would compute them.
///
/// Each certificate search tries the child that certified last first, then
/// takes the unchanged children in order, resuming where it left off. Per
/// endpoint it compares at most once per child of `R` and once per child of
/// `U`: no more steps than the fold it spares, which joins or meets once per
/// child, and a comparison allocates nothing.
fn seeded_bounds(original: &Node, children: &Fan) -> OnceLock<Span<'static>> {
    let declined = || {
        memo_census::record(Work::BoundsDeclined);
        OnceLock::new()
    };
    // A bare leaf lists no children, so its span is no union of children's.
    if original.child_count() == 0 {
        return declined();
    }
    let Some(old) = original.computed_span() else {
        return declined();
    };
    // `D`, `R`, and which of `N`'s radices hold a child of `D`.
    let mut differ: SmallVec<[&Node; 4]> = SmallVec::new();
    let mut gone: SmallVec<[&Node; 4]> = SmallVec::new();
    let mut differs = [false; RADICES];
    for pair in original
        .listed()
        .merge_join_by(children.iter(), |(ours, _), (theirs, _)| ours.cmp(theirs))
    {
        match pair {
            EitherOrBoth::Both((_, listed), (_, child)) if listed.is(child) => {}
            EitherOrBoth::Both((_, listed), (radix, child)) => {
                differ.push(child);
                gone.push(listed.memos());
                differs[usize::from(radix)] = true;
            }
            EitherOrBoth::Left((_, listed)) => gone.push(listed.memos()),
            EitherOrBoth::Right((radix, child)) => {
                differ.push(child);
                differs[usize::from(radix)] = true;
            }
        }
    }
    // Computing `D`'s bounds here, if they are cold, moves work rather than
    // adding it: publication computes the bounds of every node it
    // publishes. It lets seeding continue up the rebuilt path.
    let joined = differ.split_first().map(|(first, rest)| {
        first
            .span()
            .union_all(rest.iter().map(|child| child.span()))
    });
    // `U`, collected only if `T` leaves a bound uncertified.
    let mut kept: Option<Vec<&Node>> = None;
    let unchanged = || {
        children
            .iter()
            .filter(|(radix, _)| !differs[usize::from(*radix)])
            .map(|(_, child)| child)
            .collect()
    };
    let mut ceiling = Certifier::default();
    let mut floor = Certifier::default();
    for replaced in &gone {
        // The old side is read only where already computed.
        let Some(span) = replaced.computed_span() else {
            return declined();
        };
        if !joined
            .as_ref()
            .is_some_and(|joined| span.hi() <= joined.hi())
        {
            ceiling.certify(kept.get_or_insert_with(unchanged), |child| {
                child
                    .computed_span()
                    .is_some_and(|bound| span.hi() <= bound.hi())
            });
        }
        if !joined
            .as_ref()
            .is_some_and(|joined| joined.lo() <= span.lo())
        {
            floor.certify(kept.get_or_insert_with(unchanged), |child| {
                child
                    .computed_span()
                    .is_some_and(|bound| bound.lo() <= span.lo())
            });
        }
        if ceiling.failed && floor.failed {
            return declined();
        }
    }
    let span = if !ceiling.failed && !floor.failed {
        memo_census::record(Work::BoundsSeeded);
        match &joined {
            Some(joined) => old.union(joined),
            None => old.into_owned(),
        }
    } else {
        memo_census::record(Work::BoundsHalf);
        let lo = if floor.failed {
            let mut floors = children.values().map(Node::floor);
            let first = floors.next().expect("a branch has at least two children");
            first.meet_all(floors)
        } else {
            joined
                .as_ref()
                .map_or_else(|| old.lo().clone(), |joined| old.lo().meet(joined.lo()))
        };
        let hi = if ceiling.failed {
            let mut ceilings = children.values().map(Node::ceiling);
            let first = ceilings.next().expect("a branch has at least two children");
            first.join_all(ceilings)
        } else {
            joined
                .as_ref()
                .map_or_else(|| old.hi().clone(), |joined| old.hi().join(joined.hi()))
        };
        Span::new(lo, hi)
            .expect("the meet of a branch's floors is at or below the join of its ceilings")
    };
    if (ceiling.by_sibling && !ceiling.failed) || (floor.by_sibling && !floor.failed) {
        memo_census::record(Work::BoundsBySibling);
    }
    OnceLock::from(span)
}

/// One endpoint's search of a rebuilt branch's unchanged children for
/// certificates ([`seeded_bounds`]).
#[derive(Default)]
struct Certifier {
    /// The position of the child that certified last, tried first next time.
    last: Option<usize>,
    /// The position the scan resumes from: children before it have been
    /// tried once.
    next: usize,
    /// Whether an unchanged child certified a bound.
    by_sibling: bool,
    /// Whether a bound found no certificate, so the endpoint is folded.
    failed: bool,
}

impl Certifier {
    /// Find a child in `kept` that `certifies`, unless the search already
    /// failed: the child that certified last first, then the children not
    /// yet tried, in order. Records a failure when none does.
    fn certify(&mut self, kept: &[&Node], certifies: impl Fn(&Node) -> bool) {
        if self.failed {
            return;
        }
        if self.last.is_some_and(|at| certifies(kept[at])) {
            return;
        }
        while let Some(&child) = kept.get(self.next) {
            self.next += 1;
            if certifies(child) {
                self.last = Some(self.next - 1);
                self.by_sibling = true;
                return;
            }
        }
        self.failed = true;
    }
}

/// Node equality compares the replicated version set.
impl Eq for Node {}

/// Compare shared storage first, then subtree hashes.
impl PartialEq for Node {
    /// Test version-set equality without walking equal shared subtrees.
    fn eq(&self, other: &Self) -> bool {
        // Shared backing settles equality with no hashing (and even cold): the
        // common case for forked/cloned trees and the subtrees they share.
        // Distinct allocations fall back to the Merkle hash, which commits
        // shape and version set — never message bytes — so this is
        // version-set equality — and content equality, because no two
        // messages ever share a version. (Same-version leaves with
        // different payloads would compare equal; producing such a pair
        // takes an already-fatal linearity violation.)
        self.ptr_eq(other) || self.hash() == other.hash()
    }
}

/// Builds the maximally compressed subtree over a strictly ascending stream
/// of bare leaves, one leaf at a time, holding only the subtree's open right
/// spine.
///
/// The result is the node the tests' whole-run constructor,
/// `Node::from_sorted_leaves`, builds from the same leaves: the same
/// compressed prefixes, branches, and fans, so the same hash. Where that
/// constructor needs the whole run in hand, this one keeps
/// at most one open branch per path byte below its starting depth, each with
/// the children closed beneath it so far, and the last leaf pushed. A
/// subtree of any size therefore assembles in transient memory bounded by
/// [`PATH_LEN`] fans of at most 256 children.
///
/// Each push compares the new leaf's path with the previous one's. Every
/// open branch deeper than the byte where the two diverge can gain no more
/// children, so it closes, and is built once with its whole compressed
/// prefix, exactly as the bulk constructor builds it. The work is the bulk
/// constructor's: one node per branch point and one per leaf.
pub(crate) struct SortedLeaves {
    /// The depth the finished subtree observes from: every leaf's path
    /// shares its first `depth` bytes.
    depth: usize,
    /// The open branches along the right spine, shallowest first, with
    /// strictly increasing divergence bytes.
    spine: Vec<OpenBranch>,
    /// The last leaf pushed, with its path. It stays unattached because its
    /// compressed prefix starts below its parent's divergence byte, which
    /// the next leaf decides.
    last: Option<([u8; PATH_LEN], Node)>,
}

/// One branch of a [`SortedLeaves`] spine that may still gain children.
struct OpenBranch {
    /// The byte at which this branch's children diverge.
    branch_at: usize,
    /// The path of the first leaf beneath it, which spells its compressed
    /// prefix.
    first: [u8; PATH_LEN],
    /// Leaves beneath its closed children.
    leaves: usize,
    /// Its closed children, in ascending radix order.
    children: Fan,
}

/// A finished subtree of a [`SortedLeaves`] build, not yet given the
/// compressed prefix its parent's divergence byte decides.
enum Closed {
    /// A bare leaf at its full path.
    Leaf([u8; PATH_LEN], Node),
    /// A branch whose children are all closed.
    Branch(OpenBranch),
}

impl Closed {
    /// The path of the first leaf beneath it.
    fn first(&self) -> &[u8; PATH_LEN] {
        match self {
            Closed::Leaf(path, _) => path,
            Closed::Branch(branch) => &branch.first,
        }
    }

    /// Build the node observed from depth `from`, returning it with its
    /// leaf count: its compressed prefix spans the path bytes from `from` to
    /// its own divergence byte, or to the end of the path for a leaf.
    fn observed_from(self, from: usize) -> (Node, usize) {
        match self {
            Closed::Leaf(path, mut node) => {
                if from < PATH_LEN {
                    // As in the bulk constructor: the node now observes
                    // from a shallower level, so any memoized hash is
                    // stale.
                    let inner = Arc::make_mut(&mut node.inner);
                    inner.prefix.extend(path[from..].iter().rev().copied());
                    inner.hash = OnceLock::new();
                }
                (node, 1)
            }
            Closed::Branch(OpenBranch {
                branch_at,
                first,
                leaves,
                mut children,
            }) => {
                debug_assert!(children.len() >= 2, "a branch point separates >= 2 runs");
                children.fit();
                let node = Node::from_inner(Arc::new(NodeInner {
                    prefix: first[from..branch_at].iter().rev().copied().collect(),
                    hash: OnceLock::new(),
                    body: Body::Branch {
                        bounds: OnceLock::new(),
                        leaves,
                        spread: spread(&children),
                        version_bytes: OnceLock::new(),
                        children,
                    },
                }));
                (node, leaves)
            }
        }
    }
}

impl OpenBranch {
    /// Open a branch diverging at `branch_at` whose first child is `first`.
    fn new(branch_at: usize, first: Closed) -> Self {
        let mut branch = Self {
            branch_at,
            first: *first.first(),
            leaves: 0,
            children: Fan::new(),
        };
        branch.adopt(first);
        branch
    }

    /// Close `child` beneath this branch, as its next child in radix order.
    fn adopt(&mut self, child: Closed) {
        let radix = child.first()[self.branch_at];
        let (node, leaves) = child.observed_from(self.branch_at + 1);
        self.children.push(radix, node);
        self.leaves += leaves;
    }
}

impl SortedLeaves {
    /// The most transient bytes one build holds at once, whatever the
    /// subtree's size.
    ///
    /// That is [`PATH_LEN`] open branches, each with the heap slots of a
    /// full fan, counted twice for the blocks a growing vector or fan frees
    /// as it reallocates. Closed children are not transient: each becomes
    /// part of the finished subtree.
    #[cfg(any(test, feature = "test-internals"))]
    pub(crate) const MAX_TRANSIENT_BYTES: usize =
        2 * PATH_LEN * (mem::size_of::<OpenBranch>() + fan::heap_slots(256) * FAN_ENTRY_BYTES);

    /// Begin a subtree observed from `depth`.
    pub(crate) fn new(depth: usize) -> Self {
        Self {
            depth,
            spine: Vec::new(),
            last: None,
        }
    }

    /// Append the next bare leaf at `path`.
    ///
    /// # Panics
    ///
    /// Panics if `path` does not ascend strictly from the previous leaf's
    /// path, and in debug builds if it leaves the subtree's shared prefix or
    /// `leaf` is not a bare leaf. Session decoding establishes all three.
    pub(crate) fn push(&mut self, path: [u8; PATH_LEN], leaf: Node) {
        debug_assert!(
            leaf.inner.prefix.is_empty() && leaf.is_leaf(),
            "a leaf run supplies bare leaf nodes",
        );
        let Some((previous, last)) = self.last.replace((path, leaf)) else {
            return;
        };
        debug_assert!(
            previous[..self.depth] == path[..self.depth],
            "every leaf in a run falls under the run's prefix",
        );
        let diverge = (self.depth..PATH_LEN)
            .find(|&at| previous[at] != path[at])
            .filter(|&at| previous[at] < path[at])
            .expect("a leaf run is strictly ascending by path");
        // Every open branch deeper than the divergence byte is complete: no
        // later leaf, sharing fewer bytes with it, can fall beneath it.
        let mut closed = Closed::Leaf(previous, last);
        while let Some(deeper) = self.spine.pop_if(|open| open.branch_at > diverge) {
            let mut deeper = deeper;
            deeper.adopt(closed);
            closed = Closed::Branch(deeper);
        }
        match self.spine.last_mut() {
            Some(open) if open.branch_at == diverge => open.adopt(closed),
            _ => self.spine.push(OpenBranch::new(diverge, closed)),
        }
    }

    /// Close every open branch and return the finished subtree, or `None`
    /// if no leaf was pushed.
    pub(crate) fn finish(mut self) -> Option<Node> {
        let (path, leaf) = self.last.take()?;
        let mut closed = Closed::Leaf(path, leaf);
        while let Some(mut open) = self.spine.pop() {
            open.adopt(closed);
            closed = Closed::Branch(open);
        }
        Some(closed.observed_from(self.depth).0)
    }
}

#[cfg(test)]
mod tests;
