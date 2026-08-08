use core::ops::ControlFlow;

use crate::codec::BitsMut;
use crate::idbits::{IdNode, IdReader};

impl IdReader<'_> {
    /// Whether `self` and `other` (normal-form ids) share no owned region.
    /// `O(n + m)`: both cursors are threaded, and a side is skipped only where
    /// the other's leaf dominates it.
    ///
    /// The cursor form of the paper's region-disjointness test, on the shared
    /// lockstep predicate walk ([`lockstep_holds`]).
    // Takes the cursors by value: a reader is single-use, and the walk consumes
    // both. (`is_*`-by-value is unusual, hence the allow.)
    #[allow(clippy::wrong_self_convention)]
    pub(crate) fn is_disjoint(self, other: IdReader) -> bool {
        // An empty `a` settles a pair: `a` owns nothing there, so nothing `b`
        // holds can overlap it. The refuting mixes are then a full side against
        // any nonempty other — an overlap.
        lockstep_holds(self, other, |a_node| matches!(a_node, IdNode::Empty))
    }

    /// Whether `self` (a normal-form id) *covers* `other` — every region
    /// `other` owns is also owned by `self` (`self ⊇ other`).
    ///
    /// `O(|self| + |other|)`: both cursors are threaded, and a side is skipped
    /// only where the other's leaf dominates it, exactly as in
    /// [`is_disjoint`](IdReader::is_disjoint).
    ///
    /// The asymmetric counterpart of [`is_disjoint`](IdReader::is_disjoint), on
    /// the same lockstep predicate walk ([`lockstep_holds`]).
    // Single-use by-value readers, as with `is_disjoint`.
    #[allow(clippy::wrong_self_convention)]
    pub(crate) fn covers(self, other: IdReader) -> bool {
        // A full `a` settles a pair: it covers whatever `b` is there.
        // The refuting mixes are then a region `b` owns that `a` does
        // not — an empty `a` against a nonempty `b`, or an `a` node
        // against a full `b` (`a` owns only part of what `b` owns in
        // full).
        lockstep_holds(self, other, |a_node| matches!(a_node, IdNode::Full))
    }
}

/// Whether a region-pair predicate holds everywhere across two normal-form ids,
/// walked in lockstep — the shared spelling of
/// [`is_disjoint`](IdReader::is_disjoint) and [`covers`](IdReader::covers).
///
/// Iterative: the two consuming cursors carry the traversal, and the
/// per-ancestor control state is two bits on a bit stack (see [`Lockstep`]), so
/// a deep operand costs bits, not stack frames or grown segments. A refuted
/// pair (`false`) ends the whole walk.
///
/// `a_settles` is the predicate's whole identity — the `a` node whose region
/// satisfies it against anything `b` holds there. The rest of the algebra is
/// fixed and shared: an empty `b` holds trivially (both predicates are vacuous
/// over a region `b` does not own, and `a`'s remaining subtree is skipped to
/// resync); two internal nodes descend, the predicate holding iff it holds on
/// every child pair; any other pairing refutes.
fn lockstep_holds(mut a: IdReader, mut b: IdReader, a_settles: impl Fn(IdNode) -> bool) -> bool {
    let mut walk = Lockstep::new();
    loop {
        // One child pair, as a match on the two id nodes, `a`'s first.
        let a_node = walk.read_a(&mut a);
        if a_settles(a_node) {
            // `a` alone decides the pair: skip `b`'s subtree to resync.
            if walk.b_on {
                b.skip();
            }
            if walk.complete().is_break() {
                return true;
            }
            continue;
        }
        let b_node = walk.read_b(&mut b);
        if let IdNode::Empty = b_node {
            // `b` owns nothing here: the predicate holds trivially.
            // Skip the rest of `a`'s subtree.
            a.skip_present_children(a_node);
            if walk.complete().is_break() {
                return true;
            }
            continue;
        }
        match (a_node, b_node) {
            // Both internal: descend in lockstep over each child pair.
            (
                IdNode::Internal {
                    left: al,
                    right: ar,
                },
                IdNode::Internal {
                    left: bl,
                    right: br,
                },
            ) => walk.descend(al, ar, bl, br),
            // Every remaining pairing refutes the predicate (the call
            // sites name their mixes), ending the whole walk.
            _ => return false,
        }
    }
}

/// The explicit control state of a lockstep predicate walk
/// ([`lockstep_holds`]).
///
/// A walk visits a both-internal node pair's child pairs left to right,
/// threading the two consuming cursors; the verdict either passes (the walk
/// moves on) or fails (the caller returns at once), so no value is ever
/// carried. The only per-ancestor state is the innermost right child pairs
/// still to walk — two presence bits each, on one bit stack. An ancestor whose
/// right pair is absent on both sides queues nothing (its completion is its
/// left pair's), so a unary lockstep chain of any depth keeps the stack empty.
struct Lockstep {
    /// Two presence bits per queued right child pair, innermost on top.
    pending: BitsMut,
    /// Whether the current pair's `a` side is a present child (read the real
    /// cursor) or an absent `0` (stand in a synthetic empty).
    a_on: bool,
    /// The `b` side of [`a_on`](Lockstep::a_on).
    b_on: bool,
}

impl Lockstep {
    /// A walk at its root pair: both sides are the real cursors.
    fn new() -> Lockstep {
        Lockstep {
            pending: BitsMut::new(),
            a_on: true,
            b_on: true,
        }
    }

    /// Decode the current pair's `a`-side node: the real cursor's node where
    /// the side is a present child, the absent child's `Empty` otherwise (a
    /// stored stream never encodes one, so the two cannot be confused).
    fn read_a(&self, a: &mut IdReader) -> IdNode {
        if self.a_on {
            a.read()
        } else {
            IdNode::Empty
        }
    }

    /// The `b` side of [`read_a`](Lockstep::read_a).
    fn read_b(&self, b: &mut IdReader) -> IdNode {
        if self.b_on {
            b.read()
        } else {
            IdNode::Empty
        }
    }

    /// Enter the child pairs of a both-internal node pair with presence bits
    /// `(al, ar)` / `(bl, br)`: queue the right pair if either side has a right
    /// child, and step into the leftmost pair either side has at all.
    ///
    /// A pair absent on both sides is never walked — both predicates hold
    /// trivially on it, and neither cursor moves for it — which is what keeps
    /// the pending stack empty on unary chains. Both pairs absent cannot
    /// happen: an internal node has at least one present child.
    fn descend(&mut self, al: bool, ar: bool, bl: bool, br: bool) {
        if (al || bl) && (ar || br) {
            self.pending.push(ar);
            self.pending.push(br);
            (self.a_on, self.b_on) = (al, bl);
        } else if al || bl {
            (self.a_on, self.b_on) = (al, bl);
        } else {
            (self.a_on, self.b_on) = (ar, br);
        }
    }

    /// Complete the current pair with a passing verdict: step into the
    /// innermost queued right pair (`Continue`), or report the whole walk done
    /// (`Break`) when no ancestor is waiting.
    ///
    /// Ancestors between the queued pair and the completed one queued nothing,
    /// so their completion needs no bookkeeping: it *is* this completion.
    fn complete(&mut self) -> ControlFlow<()> {
        let Some(br) = self.pending.pop() else {
            return ControlFlow::Break(());
        };
        let ar = self
            .pending
            .pop()
            .expect("pending right pairs are two bits");
        (self.a_on, self.b_on) = (ar, br);
        ControlFlow::Continue(())
    }
}
