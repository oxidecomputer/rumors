//! The causal order on event trees: one recursive, offset-threaded pass that
//! reads either representation in place (no transcode) and yields the
//! comparison directly.
//!
//! [`EvReader::causal_cmp`] decides the order of `a` and `b` by a pointwise
//! comparison of their event functions, tracking `a <= b` and `b <= a` in a
//! single `O(n + m)` traversal; running the paper's `leq` twice would do
//! double the work. At each aligned node pair the path sums settle the local
//! direction (`an > bn` rules out `a <= b`, and `bn > an` rules out
//! `b <= a`); the walk then descends into whichever side is internal, a leaf
//! side broadcasting a fresh [`Zero`](EvReader::Zero) to both of the other's
//! children, until both bottom out. Every node of either tree is visited
//! once, and once both directions are excluded the result is concurrent
//! (`None`) and the walk stops early. Recursion is guarded by
//! [`crate::recurse`].
//!
//! The two `&mut` cursors advance in place as they are read, so the right
//! child resumes where the left one left off without re-scanning (see the
//! [traversal overview](super::event)).
//!
//! Path sums (the running total of stored bases along a root-to-node path)
//! are threaded as arbitrary-precision [`Base`] offsets,
//! the same value type as the stored bases and as `join`/`fill`/`grow`.
//! `decode` admits any normal-form tree, including one whose path sums
//! exceed `u64::MAX`, so a bounded accumulator could wrap and invert the
//! causal order; the unbounded type removes that failure class.

use core::cmp::Ordering;

use crate::codec::{decode_int, skip_int, Base, BitsSlice};
use crate::recurse::descend;
use crate::{idbits, step};

use super::working::WorkingVersion;

/// A decoded event node: a leaf value, or an internal node's stored base. The
/// base is the node's *relative* value; a path sum (offset) is threaded
/// alongside to recover absolute values.
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

/// The sole cursor into an event tree, in either storage form, or a synthetic
/// zero. It reads both representations in place (no transcode) and *consumes* —
/// [`read`](EvReader::read) decodes the node at the cursor and advances it in
/// place, so operations thread `&mut` readers rather than bare positions.
///
/// - `Packed`: a bit offset into the canonical `enc_ev` stream (flag bit +
///   gamma-coded base, children following).
/// - `Working`: a node index into the fixed-width working form (`topo`/`base`).
/// - `Zero`: a synthetic `Leaf(0)` that consumes nothing — the paper's `0` in
///   "a leaf `n` behaves as `(n, 0, 0)`". A leaf side of a two-tree walk hands
///   both children a freshly-constructed `Zero`; `grow` uses it as its *virtual
///   leaf*, the `(0,0)` it expands an event leaf into to follow the id deeper.
///
/// Not `Copy` or `Clone`: a cursor is single-use. Advancing it consumes the
/// stream, so a stale or duplicated cursor (a re-scan, which would break the
/// `O(n + m)` bound) cannot be formed by accident. A broadcast hands
/// children a fresh synthetic (`Zero`), not a copy of the cursor. The one
/// operation that reads a tree twice (`grow`, a cost probe then an emit)
/// builds a fresh cursor from the source working form for each pass, as
/// `tick` does. Visibility is `pub(super)`: used throughout `version/`,
/// nowhere outside.
pub(super) enum EvReader<'a> {
    Packed {
        bits: &'a BitsSlice,
        pos: usize,
    },
    Working {
        work: &'a WorkingVersion,
        pos: usize,
    },
    /// A synthetic `Leaf(0)` (see the type doc); reads as `Leaf(0)` and never
    /// advances.
    Zero,
}

impl<'a> EvReader<'a> {
    /// A reader at the root of a packed `enc_ev` stream.
    pub(super) fn packed(bits: &'a BitsSlice) -> Self {
        EvReader::Packed { bits, pos: 0 }
    }

    /// A reader at the root of a working-form event tree.
    pub(super) fn working(work: &'a WorkingVersion) -> Self {
        EvReader::Working { work, pos: 0 }
    }

    /// Decode the node at this cursor, advancing it just past the header — to
    /// the left child, for an internal node. `Zero` reads as `Leaf(0)` and never
    /// advances (a synthetic leaf has no children).
    pub(super) fn read(&mut self) -> EvNode {
        match self {
            EvReader::Zero => EvNode::Leaf(Base::ZERO),
            EvReader::Packed { bits, pos } => {
                step!();
                let bits = *bits;
                let internal = bits[*pos];
                let (base, next) = decode_int(bits, *pos + 1).expect("canonical event bits");
                *pos = next;
                if internal {
                    EvNode::Internal(base)
                } else {
                    EvNode::Leaf(base)
                }
            }
            EvReader::Working { work, pos } => {
                step!();
                let base = work.base[*pos].clone();
                let internal = work.topo[*pos];
                *pos += 1;
                if internal {
                    EvNode::Internal(base)
                } else {
                    EvNode::Leaf(base)
                }
            }
        }
    }

    /// Advance this cursor just past the whole subtree at it. Iterative: a
    /// pending-children counter (the shared
    /// [`skip_subtree`](crate::idbits::skip_subtree) scan), never the call
    /// stack. Packed skips the gamma-coded base without decoding it. `Zero` is a
    /// leaf: nothing to skip.
    pub(super) fn skip(&mut self) {
        match self {
            EvReader::Zero => {}
            EvReader::Packed { bits, pos } => {
                let bits = *bits;
                *pos = idbits::skip_subtree(*pos, |p| {
                    step!();
                    let internal = bits[p];
                    let next = skip_int(bits, p + 1).expect("canonical event bits");
                    // Event nodes are full binary: a node has two children, a
                    // leaf none.
                    (if internal { 2 } else { 0 }, next)
                });
            }
            EvReader::Working { work, pos } => {
                let work = *work;
                *pos = idbits::skip_subtree(*pos, |p| {
                    step!();
                    // Event nodes are full binary: an internal node has two
                    // children, a leaf none.
                    (if work.topo[p] { 2 } else { 0 }, p + 1)
                });
            }
        }
    }

    /// This reader's position if it addresses a real tree, or `None` for the
    /// synthetic `Zero`. `grow` captures it (before a read advances the cursor)
    /// to key its position-indexed `Route`; the synthetic side of a branch is
    /// never the keying side, so its `None` is never unwrapped.
    pub(super) fn pos_opt(&self) -> Option<usize> {
        match self {
            EvReader::Packed { pos, .. } | EvReader::Working { pos, .. } => Some(*pos),
            EvReader::Zero => None,
        }
    }

    /// Whether two readers are *trivially* equal: the same storage form with
    /// byte-for-byte identical contents (a whole-representation check,
    /// independent of position). Both forms are always canonical normal form, so
    /// identical contents is exactly semantic equality — which lets
    /// [`causal_cmp`](EvReader::causal_cmp) settle `Equal` with one
    /// length-checked memcmp instead of the full `O(n + m)` walk. A
    /// representation mismatch declines to `false` and falls through: proving
    /// equality across forms would mean transcoding, no cheaper than the walk.
    pub(super) fn trivially_eq(&self, other: &EvReader) -> bool {
        match (self, other) {
            (EvReader::Packed { bits: a, .. }, EvReader::Packed { bits: b, .. }) => a == b,
            (EvReader::Working { work: a, .. }, EvReader::Working { work: b, .. }) => {
                a.topo == b.topo && a.base == b.base
            }
            _ => false,
        }
    }

    /// A conservative node-count capacity for output builders. Packed event
    /// nodes occupy at least two bits (flag + gamma(0)), so this avoids a full
    /// counting pass while keeping over-allocation bounded for normal
    /// small-base trees.
    pub(super) fn node_capacity_bound(&self) -> usize {
        match self {
            EvReader::Packed { bits, .. } => bits.len().div_ceil(2),
            EvReader::Working { work, .. } => work.base.len(),
            EvReader::Zero => 0,
        }
    }
}

impl EvReader<'_> {
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
    pub(crate) fn causal_cmp(self, other: EvReader) -> Option<Ordering> {
        // Both storage forms are canonical normal form, so identical contents is
        // exactly semantic equality: settle `Equal` with one memcmp before
        // recursing. Covers every entry point — Version vs Version, Batch vs
        // Batch, and a not-yet-materialized Batch (still packed) against either.
        // (Mixed packed/working forms decline and fall through; see
        // `EvReader::trivially_eq`.)
        if self.trivially_eq(&other) {
            return Some(Ordering::Equal);
        }
        let mut walk = CmpWalk { le: true, ge: true };
        let zero = Base::ZERO;
        let (mut a, mut b) = (self, other);
        match descend!(0, walk.rec(&mut a, &zero, &mut b, &zero, 0)) {
            None => None, // concurrent
            Some(()) => match (walk.le, walk.ge) {
                (true, true) => Some(Ordering::Equal),
                (true, false) => Some(Ordering::Less),
                (false, true) => Some(Ordering::Greater),
                (false, false) => unreachable!("both-false returns `None` inside `rec`"),
            },
        }
    }
}

/// The two still-possible directions of a [`causal_cmp`](EvReader::causal_cmp)
/// walk. The `&mut` readers carry the traversal state; only the verdict lives
/// here.
struct CmpWalk {
    /// `a <= b` still possible.
    le: bool,
    /// `b <= a` still possible.
    ge: bool,
}

impl CmpWalk {
    /// Compare the aligned subtrees at the two `&mut` readers and path-sum
    /// offsets, advancing each reader past its subtree (so a right sibling
    /// resumes from it), routing through the amortized stack-growth guard.
    /// `None` signals a decided `concurrent` that unwinds the whole walk.
    ///
    /// Reads as the paper's `leq`: settle the local direction from the path
    /// sums, then descend each side — an internal side into its two real
    /// children (the `&mut` advances through left then right), a leaf side
    /// broadcasting a fresh `Zero` to both — handing both children the node sum.
    fn rec(
        &mut self,
        a: &mut EvReader,
        a_off: &Base,
        b: &mut EvReader,
        b_off: &Base,
        depth: usize,
    ) -> Option<()> {
        let a_node = a.read();
        let b_node = b.read();
        let a_internal = a_node.is_internal();
        let b_internal = b_node.is_internal();
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
        if !a_internal && !b_internal {
            return Some(()); // both leaves: this branch is decided
        }
        // Descend. An internal side hands its `&mut` cursor down (it advances
        // through the left subtree, then the right resumes from it); a leaf side
        // broadcasts a fresh `Zero`, leaving its own cursor at the leaf's end.
        let mut a_zero = EvReader::Zero;
        let mut b_zero = EvReader::Zero;
        descend!(
            depth + 1,
            self.rec(
                if a_internal { &mut *a } else { &mut a_zero },
                &a_sum,
                if b_internal { &mut *b } else { &mut b_zero },
                &b_sum,
                depth + 1,
            )
        )?;
        descend!(
            depth + 1,
            self.rec(
                if a_internal { &mut *a } else { &mut a_zero },
                &a_sum,
                if b_internal { &mut *b } else { &mut b_zero },
                &b_sum,
                depth + 1,
            )
        )
    }
}
