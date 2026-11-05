use std::sync::{Arc, OnceLock};
use std::{fmt, mem};

use itertools::{EitherOrBoth, Itertools};
use smallvec::SmallVec;
use tinyvec::ArrayVec;

use before::Span;

use super::hash::PATH_LEN;
use crate::{
    Version,
    message::Message,
    tree::typed::{Hash, HashKey},
};

pub mod fan;
mod iter;
mod view;
use fan::Fan;
pub use iter::{Iter, Leaf, Range, RangeOwned};
pub use view::View;

/// A shared leaf or branch, with its compressed path and cached hash.
///
/// [`typed`](super) adds compile-time heights to this storage. Cloning a
/// handle shares the node; mutation copies its state when it is shared.
pub struct Node {
    /// State shared by handles to the same node.
    inner: Arc<NodeInner>,
}

/// The radices a branch's children can sit at: one per byte value.
const RADICES: usize = u8::MAX as usize + 1;

/// The bytes of one heap slot of a branch's fan: a radix beside a child
/// handle.
#[cfg(any(test, feature = "test-internals"))]
pub(crate) const FAN_ENTRY_BYTES: usize = mem::size_of::<(u8, Node)>();

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
        /// A view returned whole by [`View::rebuild`](super::View::rebuild)
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

/// The height of a bare branch over `children` ([`Body::Branch`]'s
/// `height`): one more than theirs.
///
/// Every child of a branch sits one level below it, so every child's height
/// is the same; debug builds check that they agree.
fn height_over(children: &Fan) -> u8 {
    let mut heights = children.values().map(Node::height);
    let child = heights.next().expect("a branch has at least two children");
    debug_assert!(
        heights.all(|other| other == child),
        "every child of a branch sits at the same height",
    );
    u8::try_from(child + 1).expect("a branch sits at most at the root's height")
}

/// The height of a branch whose children diverge at the path byte
/// `branch_at` ([`Body::Branch`]'s `height`): the levels from that byte to
/// the leaves.
fn height_at(branch_at: usize) -> u8 {
    u8::try_from(PATH_LEN - branch_at).expect("a path byte lies within the path")
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
    /// Subtree digest as seen from the top of `prefix`, computed on first use
    /// under the hash key of the network whose tree holds the node
    /// ([`Node::hash`]).
    ///
    /// Both leaves and branches cache it. Cloning preserves the cached value;
    /// changing either `prefix` or `body` must clear it.
    ///
    /// # One key per cache
    ///
    /// The memo records no key, so every read must pass the key it was
    /// filled under. Every digest accessor takes the key from the tree that
    /// holds the node, and a node is reachable only from trees of one
    /// network: a tree's clones share its keys, [`Tree::join`] merges only
    /// trees with equal keys, a session lists and compares digests under the
    /// keys of the tree it reconciles and returns its nodes into that tree,
    /// and a bootstrapper's tree holds nothing until the session that fills
    /// it under the keys it received. Independently seeded universes never
    /// interact, so no node is shared between two networks' trees.
    ///
    /// [`Tree::join`]: crate::tree::Tree::join
    hash: OnceLock<Hash>,
    /// The leaf or branch stored beneath the prefix.
    body: Body,
}

/// [`NodeInner`] as it would be without the fields that ride in its
/// padding: the same fields, with [`BodyBaseline`] for its body.
///
/// Nothing builds one; it exists so the compiler lays out the node without
/// those fields, for the assertion below to compare against.
#[expect(dead_code, reason = "a layout reference, measured and never built")]
struct NodeInnerBaseline {
    /// As [`NodeInner`]'s `prefix`.
    prefix: ArrayVec<[u8; PATH_LEN]>,
    /// As [`NodeInner`]'s `hash`.
    hash: OnceLock<Hash>,
    /// As [`NodeInner`]'s `body`, without the fields that ride in padding.
    body: BodyBaseline,
}

/// [`Body`] as it would be without the fields that ride in the node's
/// padding: every variant and field but the branch's grandchild count and
/// its height.
#[expect(dead_code, reason = "a layout reference, measured and never built")]
enum BodyBaseline {
    /// As [`Body::Leaf`].
    Leaf {
        /// As [`Body::Leaf`]'s `version`.
        version: Version,
        /// As [`Body::Leaf`]'s `message`.
        message: Message,
    },
    /// As [`Body::Branch`], without `spread` and `height`.
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

// Every node pays for the branch's grandchild count and height, leaves
// included, unless they fit in padding the node already has. They do on the
// platforms the crate is measured on, and this keeps it so: a platform where
// either grows the node fails here, so the cost is a decision rather than an
// unnoticed change. A field added to `NodeInner` or `Body` belongs in the
// twins above as well, or this fails for that reason instead.
const _: () = assert!(
    mem::size_of::<NodeInner>() == mem::size_of::<NodeInnerBaseline>(),
    "the branch's grandchild count and height must fit in the node's padding",
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
        /// [`View::rebuild`] can derive it exactly from the node it
        /// replaces. [`Span`] keeps the pair ordered by construction.
        bounds: OnceLock<Span<'static>>,
        /// The number of leaves below this branch.
        leaves: usize,
        /// This branch's grandchild count as a bare branch, `S`, stored as
        /// `S − 1`: over its children, each one's own child count, a child
        /// at the leaves counting one ([`View::grandchild_count`]).
        ///
        /// A branch has two to 256 children, each counting one to 256, so
        /// `S` lies in 2..=65,536 and `S − 1` fits exactly. It depends only
        /// on the fan, which never changes after construction; peeling or
        /// extending the prefix leaves it valid.
        spread: u16,
        /// The height of the branch itself above the leaves, one more than
        /// each child's ([`Node::height`]): 1..=32.
        ///
        /// A node's height is its body's plus one level per path byte, so
        /// a stored node knows the height of its top, and a handle tagged
        /// with the height it sits at derives how many path bytes it hides
        /// ([`View::at_height`]). Like `spread`, it depends only on the fan.
        height: u8,
        /// The largest encoded version size in this subtree: leaf versions
        /// and branch bounds, including this branch's own bounds. Computing
        /// it also fills `bounds`; later reads use the cached size.
        version_bytes: OnceLock<usize>,
        /// The children of this branch.
        children: Fan,
    },
}

impl Body {
    /// The height of the body above the leaves: zero for a leaf, and a
    /// branch's stored height.
    fn height(&self) -> usize {
        match self {
            Body::Leaf { .. } => 0,
            Body::Branch { height, .. } => usize::from(*height),
        }
    }
}

/// A body under a compressed path, as one level of the tree sees it: a
/// stored node's whole path, or the part of it a [`View`] leaves visible.
///
/// Everything listing depends on is a function of these two: the children
/// listed, their count and radices, their own children's count, and the
/// hash, which commits the path in one preimage. A view and the node a
/// copy would build for it have the same shape, so they answer alike.
#[derive(Clone, Copy)]
struct Shape<'a> {
    /// The visible path, deepest byte first: its last byte is the top.
    prefix: &'a [u8],
    /// The leaf or branch beneath it.
    body: &'a Body,
}

impl Shape<'_> {
    /// The number of children listing yields: a bare branch's fan, one for
    /// a non-empty path, and none for a bare leaf.
    fn child_count(self) -> usize {
        match (self.prefix.is_empty(), self.body) {
            (true, Body::Leaf { .. }) => 0,
            (true, Body::Branch { children, .. }) => children.len(),
            (false, _) => 1,
        }
    }

    /// The radices listing yields, ascending: the top path byte, or a bare
    /// branch's fan, or none for a bare leaf.
    fn child_radices(self) -> impl Iterator<Item = u8> {
        let fan = match (self.prefix.last(), self.body) {
            (None, Body::Branch { children, .. }) => Some(children.iter().map(|(radix, _)| radix)),
            _ => None,
        };
        self.prefix
            .last()
            .copied()
            .into_iter()
            .chain(fan.into_iter().flatten())
    }

    /// The number of children the listed children have, a child at the
    /// leaves counting one ([`View::grandchild_count`]).
    fn grandchild_count(self) -> usize {
        match (self.prefix.len(), self.body) {
            (0, Body::Leaf { .. }) => 0,
            (0, Body::Branch { spread, .. }) => usize::from(*spread) + 1,
            (1, Body::Leaf { .. }) => 1,
            (1, Body::Branch { children, .. }) => children.len(),
            _ => 1,
        }
    }

    /// The height of the trie position this shape describes, the top of
    /// its visible path: its body's height plus one level per visible path
    /// byte.
    ///
    /// A typed handle at height `H` sees exactly the shape whose top is `H`
    /// ([`View::at_height`]), so this is the handle's typed height.
    fn height(self) -> usize {
        self.body.height() + self.prefix.len()
    }

    /// The subtree's digest under `key`, computed rather than read from any
    /// memo ([`Node::hash`] gives the convention).
    ///
    /// The preimage commits the shape's own height ([`height`](Self::height)),
    /// and each child contributes its memoized digest, which commits the
    /// child's height, one level below the body.
    fn hash(self, key: &HashKey) -> Hash {
        memo_census::record(Work::Hash);
        // The preimage takes the prefix in path order, shallowest byte
        // first, while storage is shallowest-last, so reverse into a stack
        // buffer (a compressed span never exceeds the 32-byte path).
        let prefix: ArrayVec<[u8; PATH_LEN]> = self.prefix.iter().rev().copied().collect();
        match self.body {
            Body::Leaf { .. } => key.leaf(self.height(), &prefix),
            Body::Branch { children, .. } => key.branch(
                self.height(),
                &prefix,
                children
                    .iter()
                    .map(|(radix, child)| (radix, child.hash(key))),
            ),
        }
    }
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
        let height = height_over(&children);
        Node::from_inner(Arc::new(NodeInner {
            prefix: ArrayVec::new(),
            hash: OnceLock::new(),
            body: Body::Branch {
                bounds,
                leaves: children.values().map(Node::len).sum(),
                spread: spread(&children),
                height,
                // Never seeded: once the bounds are known, the full
                // computation reads one stored size per child and the
                // bounds' own two encodings, at most 256 cached loads,
                // exact by construction.
                version_bytes: OnceLock::new(),
                children,
            },
        }))
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
                height: height_at(branch_at),
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

    /// The height of this node's top above the leaves: its body's height,
    /// zero for a leaf, plus one level per byte of its compressed path.
    ///
    /// A typed handle at the same height views the node whole; one at a
    /// lower height, down to the body's, views it from inside its path
    /// ([`View::at_height`]).
    pub fn height(&self) -> usize {
        self.shape().height()
    }

    /// Check [`View::grandchild_count`] against a direct count
    /// at this node and at every level beneath it, returning the first that
    /// disagrees.
    ///
    /// Each level of a compressed path counts, as the view listing yields
    /// for it. A disagreement comes back with the stored and the counted
    /// values.
    ///
    /// The count lists each level's children with [`View::children`] and
    /// each child's own children the same way. The traversal keeps an
    /// explicit stack, bounded by the key depth times the fan, so it never
    /// recurses on the tree's shape.
    #[cfg(test)]
    pub(crate) fn miscounted_grandchildren(&self) -> Option<(View, usize, usize)> {
        let mut stack = vec![View::from(self.clone())];
        while let Some(view) = stack.pop() {
            let Ok(children) = view.clone().children() else {
                if view.grandchild_count() != 0 {
                    return Some((view.clone(), view.grandchild_count(), 0));
                }
                continue;
            };
            let mut counted = 0;
            for (_, child) in children {
                counted += child.clone().children().map_or(1, Iterator::count);
                stack.push(child);
            }
            if view.grandchild_count() != counted {
                return Some((view.clone(), view.grandchild_count(), counted));
            }
        }
        None
    }

    /// The number of children listing a whole view of this node yields
    /// ([`View::children`]).
    ///
    /// Exact, and O(1): a bare branch has its fan, a node with a compressed
    /// path peels to a single child, and a bare leaf has none.
    pub fn child_count(&self) -> usize {
        self.shape().child_count()
    }

    /// How many children listing a whole view of this node yields
    /// ([`View::children`]), counting a bare leaf as one: what it
    /// contributes to its parent's grandchild count.
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

    /// The digest of the subtree rooted at this node under `key`, as
    /// observed from the top of its compressed prefix.
    ///
    /// The digest is computed lazily on first call and memoized, so the first
    /// read of a freshly-built subtree is `O(nodes)` — one preimage and one
    /// HMAC-SHA-256 pass per node — and every read thereafter is an `O(1)`
    /// field load. The convention (see [`HashKey::leaf`] and
    /// [`HashKey::branch`]): a single preimage commits the node's kind, the
    /// height of its top, its compressed prefix in path order, and, for a
    /// branch, its children as ascending `radix ‖ hash` records. Digest
    /// agreement between differently-built trees rests on the tree's
    /// canonical shape; see [`HashKey::branch`]'s canonicity section.
    ///
    /// `key` must be the hash key of the network whose tree holds this node:
    /// the memo is filled under the first key passed and records none
    /// ([`NodeInner`]'s `hash` gives the argument that every caller passes
    /// the same one).
    pub fn hash(&self, key: &HashKey) -> Hash {
        *self.inner.hash.get_or_init(|| self.shape().hash(key))
    }

    /// This node's whole compressed path over its body.
    fn shape(&self) -> Shape<'_> {
        Shape {
            prefix: &self.inner.prefix,
            body: &self.inner.body,
        }
    }

    /// Get the ceiling version of this node (the maximal version of all
    /// children).
    ///
    /// Like [`hash`](Self::hash), the ceiling is computed on first call and
    /// memoized: a leaf's is set at construction, and a branch's is the
    /// join endpoint of its memoized bounds span, so reading either bound
    /// computes both, unless reassembly derived them when it built the
    /// branch ([`View::rebuild`]). The memo is a pure function of the
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
    /// branch ([`View::rebuild`]). The memo is a pure function of the
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
/// `original`, the view the children were listed from, wherever that is
/// exact.
///
/// A view's bounds are its stored node's, since bounds depend on the body
/// alone; which children are unchanged is decided on the views, as
/// listing yields them: the same stored node, hiding the same bytes.
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
fn seeded_bounds(original: &View, children: &[(u8, View)]) -> OnceLock<Span<'static>> {
    let declined = || {
        memo_census::record(Work::BoundsDeclined);
        OnceLock::new()
    };
    // A bare leaf lists no children, so its span is no union of children's.
    if original.child_count() == 0 {
        return declined();
    }
    let Some(old) = original.stored().computed_span() else {
        return declined();
    };
    // `D`, `R`, and which of `N`'s radices hold a child of `D`.
    let mut differ: SmallVec<[&Node; 4]> = SmallVec::new();
    let mut gone: SmallVec<[&Node; 4]> = SmallVec::new();
    let mut differs = [false; RADICES];
    for pair in original.listed().merge_join_by(
        children.iter().map(|(radix, child)| (*radix, child)),
        |(ours, _), (theirs, _)| ours.cmp(theirs),
    ) {
        match pair {
            EitherOrBoth::Both((_, listed), (_, child)) if listed.is(child) => {}
            EitherOrBoth::Both((_, listed), (radix, child)) => {
                differ.push(child.stored());
                gone.push(listed.memos());
                differs[usize::from(radix)] = true;
            }
            EitherOrBoth::Left((_, listed)) => gone.push(listed.memos()),
            EitherOrBoth::Right((radix, child)) => {
                differ.push(child.stored());
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
            .map(|(_, child)| child.stored())
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
            let mut floors = children.iter().map(|(_, child)| child.stored().floor());
            let first = floors.next().expect("a branch has at least two children");
            first.meet_all(floors)
        } else {
            joined
                .as_ref()
                .map_or_else(|| old.lo().clone(), |joined| old.lo().meet(joined.lo()))
        };
        let hi = if ceiling.failed {
            let mut ceilings = children.iter().map(|(_, child)| child.stored().ceiling());
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

/// Compare replicated version sets.
impl Node {
    /// Whether `self` and `other`, at the same trie position in trees of
    /// the network whose hash key is `key`, hold the same version set,
    /// without walking equal shared subtrees.
    ///
    /// Shared backing settles equality with no hashing (and even cold): the
    /// common case for forked or cloned trees and the subtrees they share.
    /// Distinct allocations fall back to the Merkle digest, which commits
    /// shape and version set, never message bytes. So this is version-set
    /// equality, and content equality, because no two messages ever share a
    /// version. (Same-version leaves with different payloads would compare
    /// equal; producing such a pair takes an already-fatal linearity
    /// violation.)
    pub fn same(&self, other: &Self, key: &HashKey) -> bool {
        self.ptr_eq(other) || self.hash(key) == other.hash(key)
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
                        height: height_at(branch_at),
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
