use crate::codec::Bits;
use crate::idbits::{IdNode, IdReader};

impl IdReader<'_> {
    /// Whether `self` and `other` (normal-form ids) share no owned region. `O(n + m)`: both
    /// cursors are threaded, and a side is skipped only where the other's leaf dominates it.
    ///
    /// The cursor form of the paper's region-disjointness test, walked
    /// iteratively: the two consuming cursors carry the traversal, and the
    /// per-ancestor control state is two bits on a bit stack (see
    /// [`Lockstep`]), so a deep operand costs bits, not stack frames or
    /// grown segments. `false` the moment an overlap is found ends the
    /// whole walk.
    // Takes the cursors by value: a reader is single-use, and the walk consumes
    // both. (`is_*`-by-value is unusual, hence the allow.)
    #[allow(clippy::wrong_self_convention)]
    pub(crate) fn is_disjoint(mut self, mut other: IdReader) -> bool {
        let mut walk = Lockstep::new();
        loop {
            // One child pair, as a match on the two id nodes: an empty side
            // is disjoint from anything (skip the other to resync); a full
            // side overlaps any nonempty other; two nodes descend.
            let a_node = walk.read_a(&mut self);
            if let IdNode::Empty = a_node {
                // a owns nothing here: disjoint; skip b's subtree to resync.
                if walk.b_on {
                    other.skip();
                }
                if walk.complete() {
                    return true;
                }
                continue;
            }
            let b_node = walk.read_b(&mut other);
            if let IdNode::Empty = b_node {
                // b owns nothing: disjoint. Skip the rest of a's subtree.
                self.skip_present_children(a_node);
                if walk.complete() {
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
                // One side full, the other nonempty (neither is empty): overlap.
                _ => return false,
            }
        }
    }

    /// Whether `self` (a normal-form id) *covers* `other` — every region `other`
    /// owns is also owned by `self` (`self ⊇ other`).
    ///
    /// `O(n + m)`: both cursors
    /// are threaded, and a side is skipped only where the other's leaf dominates
    /// it, exactly as in [`is_disjoint`](IdReader::is_disjoint).
    ///
    /// The asymmetric counterpart of [`is_disjoint`](IdReader::is_disjoint),
    /// on the same iterative [`Lockstep`] walk: a full `self` dominates
    /// anything (skip the other to resync); an empty `other` is covered by
    /// anything (skip the rest of `self`); an empty `self` against a
    /// nonempty `other`, or a node `self` against a full `other`, is a
    /// region `self` lacks — `false`, ending the whole walk; two nodes
    /// descend.
    // Single-use by-value readers, as with `is_disjoint`.
    #[allow(clippy::wrong_self_convention)]
    pub(crate) fn covers(mut self, mut other: IdReader) -> bool {
        let mut walk = Lockstep::new();
        loop {
            let a_node = walk.read_a(&mut self);
            if let IdNode::Full = a_node {
                // a owns everything here: it covers whatever b is; resync
                // past b.
                if walk.b_on {
                    other.skip();
                }
                if walk.complete() {
                    return true;
                }
                continue;
            }
            let b_node = walk.read_b(&mut other);
            if let IdNode::Empty = b_node {
                // b owns nothing here: trivially covered. Skip the rest of
                // a's subtree.
                self.skip_present_children(a_node);
                if walk.complete() {
                    return true;
                }
                continue;
            }
            match (a_node, b_node) {
                // Both internal: a covers b iff it covers b on both child
                // pairs.
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
                // A region b owns that a does not: a empty under a nonempty
                // b, or a node under a full b (a owns only part of what b
                // owns in full).
                _ => return false,
            }
        }
    }
}

/// The explicit control state of a lockstep predicate walk
/// ([`covers`](IdReader::covers), [`is_disjoint`](IdReader::is_disjoint)).
///
/// A walk visits a both-internal node pair's child pairs left to right,
/// threading the two consuming cursors; the verdict either passes (the
/// walk moves on) or fails (the caller returns at once), so no value is
/// ever carried. The only per-ancestor state is the innermost right child
/// pairs still to walk — two presence bits each, on one bit stack. An
/// ancestor whose right pair is absent on both sides queues nothing (its
/// completion is its left pair's), so a unary lockstep chain of any depth
/// keeps the stack empty.
struct Lockstep {
    /// Two presence bits per queued right child pair, innermost on top.
    pending: Bits,
    /// Whether the current pair's `a` side is a present child (read the
    /// real cursor) or an absent `0` (stand in a synthetic empty).
    a_on: bool,
    /// The `b` side of [`a_on`](Lockstep::a_on).
    b_on: bool,
}

impl Lockstep {
    /// A walk at its root pair: both sides are the real cursors.
    fn new() -> Lockstep {
        Lockstep {
            pending: Bits::new(),
            a_on: true,
            b_on: true,
        }
    }

    /// Decode the current pair's `a`-side node: the real cursor's node
    /// where the side is a present child, the absent child's `Empty`
    /// otherwise (a stored stream never encodes one, so the two cannot be
    /// confused).
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

    /// Enter the child pairs of a both-internal node pair with presence
    /// bits `(al, ar)` / `(bl, br)`: queue the right pair if either side
    /// has a right child, and step into the leftmost pair either side has
    /// at all.
    ///
    /// A pair absent on both sides is never walked — both predicates hold
    /// trivially on it, and neither cursor moves for it — which is what
    /// keeps the pending stack empty on unary chains. Both pairs absent
    /// cannot happen: an internal node has at least one present child.
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
    /// innermost queued right pair, or report the whole walk done (`true`)
    /// when no ancestor is waiting.
    ///
    /// Ancestors between the queued pair and the completed one queued
    /// nothing, so their completion needs no bookkeeping: it *is* this
    /// completion.
    fn complete(&mut self) -> bool {
        let Some(br) = self.pending.pop() else {
            return true;
        };
        let ar = self
            .pending
            .pop()
            .expect("pending right pairs are two bits");
        (self.a_on, self.b_on) = (ar, br);
        false
    }
}
