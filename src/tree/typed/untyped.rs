use std::fmt::Debug;
use std::mem;
use std::sync::{Arc, OnceLock};

use borsh::BorshSerialize;
use tinyvec::ArrayVec;

use before::{Dominance, Span};

use crate::{Version, message::Message, tree::typed::Hash};

pub mod fan;
mod iter;
use fan::Fan;
pub use iter::{Iter, Leaf, Range, RangeOwned};

/// One storage node — a leaf or a branch behind a shared `Arc`, carrying
/// its compressed prefix and memoized hash.
///
/// The single representation beneath the height-typed veneer (see
/// [`typed`](super)); cloning is an `Arc` bump, and mutation is
/// copy-on-write.
pub struct Node<T> {
    inner: Arc<NodeInner<T>>,
}

impl<T> Clone for Node<T> {
    fn clone(&self) -> Self {
        Self::from_inner(self.inner.clone())
    }
}

/// Handles are counted, so a dropped one must check out; see [`census`].
#[cfg(any(test, feature = "test-internals"))]
impl<T> Drop for Node<T> {
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
/// since the last reset. The session window's memory bound is stated in
/// in-flight references; this is the instrument that lets tests check the
/// bound against reality. Read through
/// [`testing::node_census`](crate::testing::node_census).
#[cfg(any(test, feature = "test-internals"))]
pub(crate) mod census {
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Handles alive right now.
    static LIVE: AtomicUsize = AtomicUsize::new(0);
    /// The most handles ever concurrently alive since the last reset.
    static PEAK: AtomicUsize = AtomicUsize::new(0);

    pub(crate) fn created() {
        let live = LIVE.fetch_add(1, Ordering::Relaxed) + 1;
        PEAK.fetch_max(live, Ordering::Relaxed);
    }

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

struct NodeInner<T> {
    /// Compressed path above this node's own branching level, stored with the
    /// deepest byte at index 0 and the shallowest byte at the last index. An
    /// empty prefix means the node is not path-compressed above its level.
    ///
    /// Only the path bytes are stored: the node's hash commits them as one
    /// length-tagged field of its single preimage (see [`Node::hash`]), and
    /// any virtual level's hash is recoverable by re-hashing with a
    /// shortened prefix — one fresh preimage, not a per-byte refold.
    prefix: Vec<u8>,
    /// The node's observable hash (the hash of the subtree as seen from the top
    /// of its compressed prefix), computed lazily on first read and memoized.
    ///
    /// Unlike the bounds memo, this lives on `NodeInner` rather than
    /// inside [`Children::Branch`] so a path-compressed leaf memoizes its hash
    /// too: a deep single-leaf spine costs its preimage only once. The memo is
    /// a pure function of the subtree, so it is safe to share across the
    /// structurally-shared (copy-on-write) clones a forked tree produces. The
    /// preimage commits the compressed prefix, so any mutation of `prefix`
    /// *or* `children` invalidates it and must reset this cell.
    hash: OnceLock<Hash>,
    /// The children of this node: either a leaf, or a branch point.
    children: Children<T>,
}

impl<T> Clone for NodeInner<T> {
    fn clone(&self) -> Self {
        Self {
            prefix: self.prefix.clone(),
            hash: self.hash.clone(),
            children: self.children.clone(),
        }
    }
}

impl<T: Debug> Debug for Node<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
            .field("prefix", &hex::encode(&self.inner.prefix))
            .field("children", &self.inner.children)
            .finish()
    }
}

/// The children of a node.
#[derive(Debug)]
enum Children<T> {
    /// A direct leaf, at the true bottom of the tree.
    Leaf {
        /// The version of this leaf.
        version: Version,
        /// The payload of this leaf.
        message: Message<T>,
    },
    /// A materialized branch point, with the invariant that there are always >=
    /// 2 branches (or else they should be path-compressed away).
    Branch {
        /// The tightest causal span containing every leaf version under
        /// this branch — its floor (the meet) and ceiling (the join) as
        /// one [`Span`] — computed lazily on first read of
        /// either bound and memoized.
        ///
        /// Storing the pair as a span makes `floor <= ceiling` a
        /// property of the stored type: every consumer reads bounds
        /// that are ordered by construction, and the classifiers place
        /// versions against the span with no per-read validation.
        ///
        /// This must be reset whenever the branch's children change, but
        /// not when its prefix does.
        bounds: OnceLock<Span<'static>>,
        /// The number of total leaves under this branch.
        leaves: usize,
        /// The largest canonical [`Version`] encoding among every bound
        /// this branch holds — its leaf versions and every descendant
        /// branch's ceiling and floor, its own included — in bytes,
        /// computed lazily on first read and memoized.
        ///
        /// Like the bounds span (which it forces), this must be reset
        /// whenever the branch's children change, but not when its
        /// prefix does.
        version_bytes: OnceLock<usize>,
        /// The children of this branch.
        children: Fan<T>,
    },
}

impl<T> Clone for Children<T> {
    fn clone(&self) -> Self {
        match self {
            Self::Leaf { version, message } => Self::Leaf {
                version: version.clone(),
                message: message.clone(),
            },
            // The lazy memos are pure functions of the (shared) subtree, so
            // cloning the `OnceLock`s carries any already-computed value over
            // to the copy-on-write clone rather than discarding it.
            Self::Branch {
                bounds,
                leaves,
                version_bytes,
                children,
            } => Self::Branch {
                bounds: bounds.clone(),
                leaves: *leaves,
                version_bytes: version_bytes.clone(),
                children: children.clone(),
            },
        }
    }
}

impl<T> Node<T> {
    /// Wrap built node state as a handle.
    ///
    /// Every handle in the crate passes through here or [`Clone`] — the
    /// funnel that makes the test-only [`census`] an exact residency
    /// count.
    fn from_inner(inner: Arc<NodeInner<T>>) -> Self {
        #[cfg(any(test, feature = "test-internals"))]
        census::created();
        Node { inner }
    }

    /// Construct a new branch node from a list of children with distinct
    /// indices (inverse to [`Node::into_children`]).
    pub fn branch(children: Fan<T>) -> Option<Self> {
        match children.len() {
            0 => None,
            1 => {
                let Some((index, node)) = children.into_iter().next() else {
                    unreachable!("a fan with 1 element cannot fail to iterate");
                };
                Some(node.beneath(index))
            }
            _ => Some(Node::from_inner(Arc::new(NodeInner {
                prefix: Vec::new(),
                hash: OnceLock::new(),
                children: Children::Branch {
                    bounds: OnceLock::new(),
                    leaves: children.values().map(Node::len).sum(),
                    version_bytes: OnceLock::new(),
                    children,
                },
            }))),
        }
    }

    /// Convert a node into its radix fan of children (inverse to
    /// [`Node::branch`]).
    ///
    /// If `self` is a leaf node, returns `Err(self)`.
    pub fn into_children(mut self) -> Result<Fan<T>, Node<T>> {
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
            match &self.inner.children {
                Children::Leaf { .. } => Err(self),
                Children::Branch { .. } => {
                    // Extract the children fan; self is dropped, so leaving
                    // its precomputed metadata referencing the now-vacated
                    // branch is harmless.
                    let inner = Arc::make_mut(&mut self.inner);
                    let Children::Branch {
                        children: branch, ..
                    } = &mut inner.children
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
    pub(crate) fn from_sorted_leaves(
        depth: usize,
        leaves: &mut [([u8; 32], Option<Self>)],
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
                inner.prefix.extend(path[depth..].iter().rev());
                inner.hash = OnceLock::new();
            }
            return node;
        }

        // Two or more distinct sorted paths diverge at the first byte where
        // the least and greatest differ; everything from `depth` up to that
        // byte is common to the whole run and compresses into the prefix.
        let first = leaves.first().expect("a leaf run is non-empty").0;
        let last = leaves.last().expect("a leaf run is non-empty").0;
        let branch_at = (depth..32)
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

        Node::from_inner(Arc::new(NodeInner {
            prefix: first[depth..branch_at].iter().rev().copied().collect(),
            hash: OnceLock::new(),
            children: Children::Branch {
                bounds: OnceLock::new(),
                leaves: count,
                version_bytes: OnceLock::new(),
                children,
            },
        }))
    }

    /// Construct a new leaf node.
    pub fn leaf(version: Version, value: Message<T>) -> Self {
        Node::from_inner(Arc::new(NodeInner {
            prefix: Vec::new(),
            hash: OnceLock::new(),
            children: Children::Leaf {
                message: value,
                version,
            },
        }))
    }

    /// Get a reference to the leaf at this node, if it is a leaf.
    pub fn as_leaf(&self) -> Option<&Message<T>> {
        match &self.inner.children {
            Children::Leaf { message, .. } => Some(message),
            _ => None,
        }
    }

    /// Look up the leaf at `path` beneath this node: a single root-to-leaf
    /// descent costing `O(depth)`, never a scan. `None` when no live leaf
    /// sits at that path.
    pub fn get(&self, mut path: &[u8]) -> Option<(&Version, &Message<T>)> {
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
            match &node.inner.children {
                // A full 32-byte path lands exactly at a leaf; a leftover
                // tail means the path was deeper than the tree.
                Children::Leaf { version, message } => {
                    return path.is_empty().then_some((version, message));
                }
                Children::Branch { children, .. } => {
                    let (radix, rest) = path.split_first()?;
                    node = children.get(*radix)?;
                    path = rest;
                }
            }
        }
    }

    /// Get the number of leaves under a node.
    pub fn len(&self) -> usize {
        match self.inner.children {
            Children::Leaf { .. } => 1,
            Children::Branch { leaves, .. } => leaves,
        }
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
        match &self.inner.children {
            Children::Leaf { version, .. } => version.as_bytes().len(),
            Children::Branch {
                version_bytes,
                children,
                ..
            } => *version_bytes.get_or_init(|| {
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
    /// BLAKE3 pass per node — and every read thereafter is an `O(1)` field
    /// load. The convention (see [`Hash::leaf`] and [`Hash::branch`]): a
    /// single preimage commits the node's kind, its compressed prefix in
    /// path order, and, for a branch, its children as ascending
    /// `radix ‖ hash` records. Hash agreement between differently-built
    /// trees rests on the tree's canonical shape; see [`Hash::branch`]'s
    /// canonicity section.
    pub fn hash(&self) -> Hash {
        *self.inner.hash.get_or_init(|| {
            // The preimage takes the prefix in path order — shallowest byte
            // first — while storage is shallowest-last, so reverse into a
            // stack buffer (a compressed span never exceeds the 32-byte
            // path).
            let prefix: ArrayVec<[u8; 32]> = self.inner.prefix.iter().rev().copied().collect();
            match &self.inner.children {
                Children::Leaf { version, .. } => Hash::leaf(&prefix, version),
                Children::Branch { children, .. } => Hash::branch(
                    &prefix,
                    children.iter().map(|(radix, child)| (radix, child.hash())),
                ),
            }
        })
    }

    /// Get the ceiling version of this node (the maximal version of all
    /// children).
    ///
    /// Like [`hash`](Self::hash), the ceiling is computed lazily on first
    /// call and memoized: a leaf's is set at construction, and a branch's
    /// is the join endpoint of its memoized bounds span, so reading
    /// either bound computes both. The memo is a pure function of the
    /// subtree, so it is safe to share across the structurally-shared
    /// clones a forked tree produces.
    pub fn ceiling(&self) -> &Version {
        match &self.inner.children {
            Children::Leaf { version, .. } => version,
            Children::Branch {
                bounds, children, ..
            } => Self::bounds(bounds, children).hi(),
        }
    }

    /// Get the floor version of this node (the minimal version of all
    /// children).
    ///
    /// Like [`hash`](Self::hash), the floor is computed lazily on first
    /// call and memoized: a leaf's is set at construction, and a branch's
    /// is the meet endpoint of its memoized bounds span, so reading
    /// either bound computes both. The memo is a pure function of the
    /// subtree, so it is safe to share across the structurally-shared
    /// clones a forked tree produces.
    pub fn floor(&self) -> &Version {
        match &self.inner.children {
            Children::Leaf { version, .. } => version,
            Children::Branch {
                bounds, children, ..
            } => Self::bounds(bounds, children).lo(),
        }
    }

    /// This subtree's version bounds as one causal span: the memoized
    /// `[floor, ceiling]` pair, borrowed.
    ///
    /// A branch answers by reborrowing its stored bounds span —
    /// ordered by construction, so handing it out revalidates nothing —
    /// and a leaf's bounds coincide at its version, the coincident span
    /// through the trusted door (`version <= version` holds
    /// reflexively). Reading either forces the same memo
    /// [`ceiling`](Self::ceiling) and [`floor`](Self::floor) share.
    pub fn span(&self) -> Span<'_> {
        match &self.inner.children {
            Children::Leaf { version, .. } => Span::at(version),
            Children::Branch {
                bounds, children, ..
            } => Self::bounds(bounds, children).reborrow(),
        }
    }

    /// How much of this subtree's version bounds `probe` dominates: the
    /// deletion-honoring classifiers' verdict, answered from the memos
    /// without descending.
    ///
    /// [`After`](Dominance::After) means the whole subtree is
    /// within `probe`'s causal past; [`Before`](Dominance::Before)
    /// means `probe` dominates not even the floor;
    /// [`Between`](Dominance::Between) means mixed. A branch
    /// answers through its stored bounds span — ordered by construction,
    /// so no validating comparison is paid at any classification — in one
    /// fused walk that decodes `probe` once and keeps the dominance
    /// face's early exit at the first interval refuting `floor <= probe`.
    ///
    /// A leaf's bounds coincide at its version, where the span door
    /// itself collapses the dominance question to one containment
    /// check ([`Span::dominance`]'s coincident rung — a
    /// leaf's span stores its one version twice, and clone identity
    /// certifies the coincidence in `O(1)`), so routing wholly through
    /// [`span`](Self::span) pays a leaf one decode of each stream,
    /// never two.
    pub fn dominance(&self, probe: &Version) -> Dominance {
        self.span().dominance(probe)
    }

    /// Force one branch's bounds memo: the tightest span containing every
    /// leaf version beneath it, stored as a single [`Span`] so
    /// the interval ordering `floor <= ceiling` rides the stored type.
    ///
    /// Two fold regimes, split by what the children hand up:
    ///
    /// - **Fringe** (every child a leaf): one fused balanced hull
    ///   ([`Version::span_all`]) over the leaf versions. Each leaf
    ///   combine derives its pair hull in one fused walk, the meet and
    ///   join legs sharing every operand decode — where the split folds
    ///   this replaces decoded each version once per lattice direction.
    /// - **Interior** (any child a branch): the children's spans fold
    ///   through one balanced containment join
    ///   ([`Span::union_all`]) — a branch child hands up its
    ///   memoized span, a leaf child its coincident one — with the meet
    ///   and join legs folded per endpoint, because different children's
    ///   floors and ceilings share no decode to fuse. The union is
    ///   total by construction, so the owned memo assembles with no
    ///   separate hull walk and no validating comparison.
    ///
    /// Either fold makes every child pass through `O(log k)` combines of
    /// similarly sized operands instead of one combine against the whole
    /// running result, with the children borrowed in, so no child's
    /// version is cloned. Path compression doesn't change which leaves
    /// the subtree contains, so the prefix plays no part; and neither
    /// fold is ever empty, because a branch always has >= 2 children by
    /// the path-compression invariant.
    fn bounds<'a>(bounds: &'a OnceLock<Span<'static>>, children: &Fan<T>) -> &'a Span<'static> {
        bounds.get_or_init(|| {
            if children.values().all(Node::is_leaf) {
                let mut versions = children.values().map(|child| match &child.inner.children {
                    Children::Leaf { version, .. } => version,
                    Children::Branch { .. } => {
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
    #[cfg(any(test, feature = "test-internals"))]
    pub fn max_bound_bytes(&self) -> usize {
        match &self.inner.children {
            Children::Leaf { version, .. } => version.as_bytes().len(),
            Children::Branch { children, .. } => self
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

    /// Whether this node's content is a single leaf (regardless of any
    /// path-compressed prefix above it).
    ///
    /// A leaf carries exactly one version, so its [`floor`](Self::floor) and
    /// [`ceiling`](Self::ceiling) coincide — which lets callers decide "keep
    /// or drop this whole subtree" from the version check alone, without
    /// exploding the compressed prefix.
    pub fn is_leaf(&self) -> bool {
        matches!(self.inner.children, Children::Leaf { .. })
    }

    /// Number of path-compressed prefix bytes carried on this node — i.e.,
    /// the count of virtual-branch levels collapsed above the node's actual
    /// content. Zero for a leaf or a non-compressed branch.
    #[cfg(test)]
    pub fn compressed_prefix_len(&self) -> usize {
        self.inner.prefix.len()
    }

    /// Borsh-serialize the node in its in-memory layout.
    ///
    /// This is the canonical encoder: the typed `BorshSerialize` impl is a
    /// thin delegate over it, and on the decode side the same shape is
    /// reconstructed via the chain-reader trick that synthesizes per-level
    /// `prefix_len` bytes.
    ///
    /// The encoded shape, in order, is:
    ///
    /// 1. `prefix_len: u8` — the path-compressed prefix's byte count;
    /// 2. `prefix_len` head bytes, shallowest first (decoders peel from the
    ///    outermost compressed level inward);
    /// 3. the body, dispatched on `children`:
    ///    - [`Children::Leaf`]: `version: Version`, then `message: Message<T>`;
    ///    - [`Children::Branch`]: `count_minus_two: u8`, then for each
    ///      child (in ascending radix order, structural in the fan):
    ///      `radix: u8`, `serialize_to(child)`.
    ///
    /// Leaf-vs-branch is **not** tagged on the wire: at the receiver, the
    /// typed height and the running `prefix_len` together name the body's
    /// shape. Multi-child branches always carry at least two children, by
    /// the path-compression invariant.
    pub fn serialize_to<W: borsh::io::Write>(&self, writer: &mut W) -> borsh::io::Result<()> {
        let prefix_len = u8::try_from(self.inner.prefix.len()).map_err(|_| {
            borsh::io::Error::new(
                borsh::io::ErrorKind::InvalidData,
                "node prefix length does not fit in a u8",
            )
        })?;
        prefix_len.serialize(writer)?;
        // Wire order is shallowest-first; the in-memory `prefix` stores the
        // shallowest byte at the last index, so iterate in reverse.
        for byte in self.inner.prefix.iter().rev() {
            byte.serialize(writer)?;
        }
        match &self.inner.children {
            Children::Leaf { message, version } => {
                version.serialize(writer)?;
                message.serialize(writer)?;
            }
            Children::Branch { children, .. } => {
                debug_assert!(
                    (2..=256).contains(&children.len()),
                    "multi-child branch must have 2..=256 children",
                );
                let count_minus_two = u8::try_from(children.len() - 2).map_err(|_| {
                    borsh::io::Error::new(
                        borsh::io::ErrorKind::InvalidData,
                        "branch children count does not fit in count_minus_two: u8",
                    )
                })?;
                count_minus_two.serialize(writer)?;
                for (radix, child) in children.iter() {
                    radix.serialize(writer)?;
                    child.serialize_to(writer)?;
                }
            }
        }
        Ok(())
    }

    /// Place a node beneath the given child index, increasing its height by
    /// one.
    ///
    /// Pushing onto the prefix raises the observable hash by one virtual
    /// level, so the memoized hash is invalidated and recomputed lazily on the
    /// next read.
    pub fn beneath(mut self, index: u8) -> Node<T> {
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
        match &self.inner.children {
            Children::Leaf { .. } => true,
            Children::Branch { children, .. } => {
                children.len() >= 2 && children.values().all(Self::is_max_compressed)
            }
        }
    }
}

impl<T> Eq for Node<T> {}

impl<T> PartialEq for Node<T> {
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

#[cfg(test)]
mod tests;
