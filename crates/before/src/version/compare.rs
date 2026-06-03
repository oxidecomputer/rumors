//! The causal order on event trees: a single recursive, offset-threaded pass
//! that reads either representation in place (no transcode) and yields the
//! comparison directly.
//!
//! [`EvView::causal_cmp`] decides the causal order of `a` and `b` by a
//! pointwise comparison of their event functions, tracking both `a <= b` and `b
//! <= a` in **one** traversal — running the paper's `leq` twice would do double
//! the work. It is the paper's recursive `leq` made symmetric *and* `O(n + m)`,
//! guarded by [`crate::recurse`] against deep trees: at each aligned node pair
//! the path sums settle the local direction (`an > bn` rules out `a <= b`;
//! `bn > an` rules out `b <= a`), then the walk descends into whichever side is
//! internal, broadcasting the leaf side unchanged to both of the other's
//! children, until both bottom out — so every node of either tree is visited
//! once. Right-child positions are **threaded**: each subtree reports where it
//! ended, so a sibling resumes there instead of re-scanning the left subtree.
//! As soon as both directions are excluded the result is concurrent (`None`)
//! and the walk stops early. Path sums are threaded as arbitrary-precision
//! [`Base`](crate::codec::Base) offsets — the same value type as the stored
//! bases and as `join`/`fill`/`grow`. A path sum is the running total of stored
//! bases along a root-to-node path; an unbounded integer type removes the `u64`
//! overflow class entirely (`decode` admits any normal-form tree, including one
//! whose path sums exceed `u64::MAX`, so a bounded accumulator could wrap and
//! invert the causal order).

use core::cmp::Ordering;

use crate::codec::{decode_int, skip_int, Base, BitsSlice};
use crate::recurse::descend;
use crate::{idbits, step};

use super::working::WorkingVersion;

/// The decoded event-node header at a position: whether the node is internal,
/// its stored base, and where the next node begins. Returned by
/// [`EvView::header`].
pub(super) struct EvHeader {
    /// Whether this node is internal (has two children) rather than a leaf.
    pub(super) internal: bool,
    /// This node's stored base, an arbitrary-precision [`Base`].
    pub(super) base: Base,
    /// Position just past this node's header (where its left child, if any,
    /// begins).
    pub(super) next: usize,
}

/// A read-only view of an event tree in either storage form, addressed by a
/// position (a bit offset for packed, a node index for working). Visibility is
/// uniform `pub(super)` across the trio
/// `EvView`/[`header`](EvView::header)/[`skip`](EvView::skip) — used throughout
/// `version/` (compare, event, grow) and nowhere outside it. `Copy`: it holds
/// only shared borrows, so passing it by value is free.
#[derive(Clone, Copy)]
pub(super) enum EvView<'a> {
    Packed(&'a BitsSlice),
    Working(&'a WorkingVersion),
}

impl EvView<'_> {
    /// Decode the event-node header at `at`. For packed, the header is the flag
    /// bit plus the gamma-coded base; the left child (if any) begins at
    /// [`EvHeader::next`]. For working, a node is one slot. The base is an
    /// arbitrary-precision [`Base`], returned by value (cloned from the working
    /// store).
    pub(super) fn header(&self, at: usize) -> EvHeader {
        // `grow` uses `super::event::VIRTUAL` (`usize::MAX`) as a sentinel
        // "virtual leaf" position and always guards `ev == VIRTUAL` before any
        // real header read. This turns a slipped guard into a loud panic
        // instead of a silent out-of-bounds / wrong-answer. Defense-in-depth
        // only; debug builds.
        debug_assert!(
            at != super::event::VIRTUAL,
            "EvView::header called on the VIRTUAL sentinel position",
        );
        step!();
        match self {
            EvView::Packed(bits) => {
                let internal = bits[at];
                let (base, next) = decode_int(bits, at + 1).expect("canonical event bits");
                EvHeader {
                    internal,
                    base,
                    next,
                }
            }
            EvView::Working(work) => EvHeader {
                internal: work.topo[at],
                base: work.base[at].clone(),
                next: at + 1,
            },
        }
    }

    /// Whether the two views are *trivially* equal: the same storage form with
    /// byte-for-byte identical contents. Both forms are always in canonical
    /// normal form (a stored `Version` is canonical; the working form is kept
    /// normal by `event::Builder`), so identical contents is exactly semantic
    /// equality — which lets [`causal_cmp`](EvView::causal_cmp) settle `Equal`
    /// with one length-checked memcmp instead of the full `O(n + m)` recursive
    /// walk. A representation mismatch (one packed, one working) declines to
    /// `false` and falls through: proving equality across forms would mean
    /// transcoding one side, no cheaper than the walk itself.
    pub(super) fn trivially_eq(&self, other: &EvView) -> bool {
        match (self, other) {
            (EvView::Packed(a), EvView::Packed(b)) => a == b,
            (EvView::Working(a), EvView::Working(b)) => a.topo == b.topo && a.base == b.base,
            _ => false,
        }
    }

    /// An exclusive upper bound on the positions this view addresses: the bit
    /// length for packed, the node count for working. Used to size a dense
    /// position-indexed array (see `grow`'s `Choices`).
    pub(super) fn span(&self) -> usize {
        match self {
            EvView::Packed(bits) => bits.len(),
            EvView::Working(work) => work.base.len(),
        }
    }

    /// A conservative node-count capacity for output builders. Packed event
    /// nodes occupy at least two bits (flag + gamma(0)), so this avoids a full
    /// counting pass while keeping over-allocation bounded for normal
    /// small-base trees.
    pub(super) fn node_capacity_bound(&self) -> usize {
        match self {
            EvView::Packed(bits) => bits.len().div_ceil(2),
            EvView::Working(work) => work.base.len(),
        }
    }

    /// Advance past the whole subtree starting at `at`, returning the position
    /// after it. Iterative: a pending-children counter, never the call stack.
    /// Packed views skip the gamma-coded base without decoding it, because only
    /// topology and end positions matter here.
    pub(super) fn skip(&self, at: usize) -> usize {
        match self {
            EvView::Packed(bits) => idbits::skip_subtree(at, |pos| {
                step!();
                let internal = bits[pos];
                let next = skip_int(bits, pos + 1).expect("canonical event bits");
                (internal, next)
            }),
            EvView::Working(_) => idbits::skip_subtree(at, |pos| {
                let h = self.header(pos);
                (h.internal, h.next)
            }),
        }
    }
}

/// A decoded event node: a leaf value, or an internal node's stored base. The
/// base is the node's *relative* value; a path sum (offset) is threaded
/// alongside to recover absolute values (see [`Side`]).
pub(super) enum EvNode {
    Leaf(Base),
    Internal(Base),
}

impl EvNode {
    /// This node's stored (relative) base, leaf or internal.
    pub(super) fn base(&self) -> &Base {
        match self {
            EvNode::Leaf(n) | EvNode::Internal(n) => n,
        }
    }

    pub(super) fn is_internal(&self) -> bool {
        matches!(self, EvNode::Internal(_))
    }
}

/// A cursor into an event tree: a position in some [`EvView`], or `Zero` — a
/// synthetic `Leaf(0)`. `Zero` is the paper's `0` in "a leaf `n` behaves as
/// `(n, 0, 0)`": when a two-tree walk descends a node whose other side is a
/// leaf, the leaf hands both children a `Zero`, so its single value is
/// broadcast unchanged across the other side's subtree. A thin `Copy` handle
/// (a view is shared borrows; `Zero` carries nothing).
#[derive(Clone, Copy)]
pub(super) enum EvReader<'a> {
    At(EvView<'a>, usize),
    Zero,
}

impl<'a> EvReader<'a> {
    /// A reader at the root of `view`.
    pub(super) fn root(view: EvView<'a>) -> Self {
        EvReader::At(view, 0)
    }

    /// Decode this node, returning it together with a reader positioned just
    /// past the header — at the left child, for an internal node. (`Zero` reads
    /// as `Leaf(0)` and never advances: a synthetic leaf has no children.)
    pub(super) fn read(self) -> (EvNode, EvReader<'a>) {
        match self {
            EvReader::Zero => (EvNode::Leaf(Base::ZERO), EvReader::Zero),
            EvReader::At(view, pos) => {
                let EvHeader {
                    internal,
                    base,
                    next,
                } = view.header(pos);
                let node = if internal {
                    EvNode::Internal(base)
                } else {
                    EvNode::Leaf(base)
                };
                (node, EvReader::At(view, next))
            }
        }
    }
}

/// One side of an aligned node pair, resolved for descent — the single place
/// the paper's leaf broadcast lives. An *internal* side descends into its two
/// real children; a *leaf* side hands both children a [`Zero`](EvReader::Zero)
/// subtree at the same offset, so a leaf `n` behaves as `(n, 0, 0)`. Both
/// children of either side receive the same path-sum offset, [`sum`](Side::sum)
/// — borrowed, never cloned.
pub(super) struct Side<'a> {
    internal: bool,
    /// The path sum at this node (incoming offset + base), handed to both
    /// children.
    pub(super) sum: Base,
    /// The reader just past this node's header: its left child if internal, or
    /// the next-sibling position if a leaf.
    after: EvReader<'a>,
}

impl<'a> Side<'a> {
    /// Resolve a side from its decoded node, the reader past its header, and the
    /// incoming path-sum offset.
    pub(super) fn new(node: &EvNode, after: EvReader<'a>, off: &Base) -> Side<'a> {
        Side {
            internal: node.is_internal(),
            sum: off + node.base(),
            after,
        }
    }

    /// The left child's reader: the real left child if internal, else `Zero`.
    pub(super) fn left(&self) -> EvReader<'a> {
        if self.internal {
            self.after
        } else {
            EvReader::Zero
        }
    }

    /// The right child's reader, given where the left child's traversal ended:
    /// the threaded end if internal, else `Zero`.
    pub(super) fn right(&self, left_end: EvReader<'a>) -> EvReader<'a> {
        if self.internal {
            left_end
        } else {
            EvReader::Zero
        }
    }

    /// Where this subtree ends, given where its right child ended: the threaded
    /// end if internal, else just past this leaf's header.
    pub(super) fn end(&self, right_end: EvReader<'a>) -> EvReader<'a> {
        if self.internal {
            right_end
        } else {
            self.after
        }
    }
}

impl EvView<'_> {
    /// The causal order of `self` and `other`, computed in one `O(n + m)` pass;
    /// `None` means concurrent.
    ///
    /// The recursive, offset-threaded form of the paper's `leq`
    /// (`oracle::Version::leq`), run in both directions at once: it tracks
    /// `self <= other` (`le`) and `other <= self` (`ge`) together so the two
    /// pointwise comparisons share a single traversal instead of running `leq`
    /// twice, and stops early the moment both are excluded. The walk descends
    /// into whichever side is internal — both in lockstep, or the internal one
    /// while the leaf side broadcasts a [`Zero`](EvReader::Zero) to both its
    /// children. Recursion is guarded by [`crate::recurse`] so deep, unbalanced
    /// trees grow the stack onto the heap instead of overflowing.
    pub(crate) fn causal_cmp(&self, other: &EvView) -> Option<Ordering> {
        // Both storage forms are canonical normal form, so identical contents is
        // exactly semantic equality: settle `Equal` with one memcmp before
        // recursing. Covers every entry point — Version vs Version, Batch vs
        // Batch, and a not-yet-materialized Batch (still packed) against either.
        // (Mixed packed/working forms decline and fall through; see
        // `EvView::trivially_eq`.)
        if self.trivially_eq(other) {
            return Some(Ordering::Equal);
        }
        let mut walk = CmpWalk { le: true, ge: true };
        let zero = Base::ZERO;
        match descend!(
            0,
            walk.rec(
                EvReader::root(*self),
                &zero,
                EvReader::root(*other),
                &zero,
                0
            )
        ) {
            None => None, // concurrent
            Some(_) => match (walk.le, walk.ge) {
                (true, true) => Some(Ordering::Equal),
                (true, false) => Some(Ordering::Less),
                (false, true) => Some(Ordering::Greater),
                (false, false) => unreachable!("both-false returns `None` inside `rec`"),
            },
        }
    }
}

/// The two still-possible directions of a [`causal_cmp`](EvView::causal_cmp)
/// walk. The readers carry the traversal state; only the verdict lives here.
struct CmpWalk {
    /// `a <= b` still possible.
    le: bool,
    /// `b <= a` still possible.
    ge: bool,
}

impl CmpWalk {
    /// Compare the aligned subtrees at the two readers and path-sum offsets,
    /// routing through the amortized stack-growth guard. Returns the readers
    /// past each subtree (to thread the right sibling), or `None` to signal a
    /// decided `concurrent` that unwinds the whole walk.
    ///
    /// Reads as the paper's `leq`: settle the local direction from the path
    /// sums, then descend each side (an internal side into its children, a leaf
    /// side broadcasting `Zero`), handing both children the node sum.
    fn rec<'a>(
        &mut self,
        a: EvReader<'a>,
        a_off: &Base,
        b: EvReader<'a>,
        b_off: &Base,
        depth: usize,
    ) -> Option<(EvReader<'a>, EvReader<'a>)> {
        let (a_node, a_after) = a.read();
        let (b_node, b_after) = b.read();
        let a_sum = a_off + a_node.base();
        let b_sum = b_off + b_node.base();
        if a_sum > b_sum {
            self.le = false;
        }
        if b_sum > a_sum {
            self.ge = false;
        }
        if !self.le && !self.ge {
            return None; // concurrent: neither direction can recover
        }
        if !a_node.is_internal() && !b_node.is_internal() {
            // Both leaves: this branch is decided. Report both ends.
            return Some((a_after, b_after));
        }
        let sa = Side::new(&a_node, a_after, a_off);
        let sb = Side::new(&b_node, b_after, b_off);
        let (a_mid, b_mid) = descend!(
            depth + 1,
            self.rec(sa.left(), &sa.sum, sb.left(), &sb.sum, depth + 1)
        )?;
        let (a_end, b_end) = descend!(
            depth + 1,
            self.rec(
                sa.right(a_mid),
                &sa.sum,
                sb.right(b_mid),
                &sb.sum,
                depth + 1
            )
        )?;
        Some((sa.end(a_end), sb.end(b_end)))
    }
}
