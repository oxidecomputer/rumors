use core::ops::ControlFlow;

use crate::codec::BitStack;
use crate::party::tree::{PartyBranch, PartyCursor, PartyNode};

impl PartyCursor<'_> {
    /// Whether two canonical party subtrees share no owned region.
    ///
    /// Runs in `O(|self| + |other|)`: both cursors advance without backtracking,
    /// and an unowned region lets the other subtree be skipped.
    ///
    // Takes the cursors by value: a cursor is single-use, and the walk consumes
    // both. (`is_*`-by-value is unusual, hence the allow.)
    #[allow(clippy::wrong_self_convention)]
    pub(crate) fn is_disjoint(self, other: PartyCursor) -> bool {
        // An unowned first operand cannot overlap the second. Otherwise an
        // owned terminal against any nonempty subtree proves overlap.
        Comparison::new(self, other).holds(|a_node| a_node.is_none())
    }

    /// Whether `self` (a normal-form party) *covers* `other` — every region
    /// `other` owns is also owned by `self` (`self ⊇ other`).
    ///
    /// `O(|self| + |other|)`: both cursors advance without backtracking, and a
    /// subtree is skipped when the other operand already decides the result, as in
    /// [`is_disjoint`](PartyCursor::is_disjoint).
    ///
    // Single-use by-value cursors, as with `is_disjoint`.
    #[allow(clippy::wrong_self_convention)]
    pub(crate) fn covers(self, other: PartyCursor) -> bool {
        // An owned first operand covers anything in the second. The remaining
        // failures expose ownership missing from the first: an unowned first
        // operand against a nonempty second, or a partially owned first operand
        // against a fully owned second.
        Comparison::new(self, other).holds(|a_node| matches!(a_node, Some(PartyNode::Owned)))
    }
}

/// A shared structural comparison for disjointness and coverage.
///
/// The operation supplies the kind of `a` region that decides a pair
/// immediately: unowned for disjointness, owned for coverage. Unowned `b`
/// regions always pass. Two branches descend together; any other undecided pair
/// fails. Pending right children cost two bits per open branch.
struct Comparison<'a, 'b> {
    /// First operand.
    a: PartyCursor<'a>,
    /// Second operand.
    b: PartyCursor<'b>,
    /// Two presence bits per queued right child pair, innermost on top.
    pending: BitStack,
    /// Whether the current first operand is stored at `a` or is unowned.
    a_on: bool,
    /// Whether the current second operand is stored at `b` or is unowned.
    b_on: bool,
}

impl<'a, 'b> Comparison<'a, 'b> {
    /// Start at the two root regions.
    fn new(a: PartyCursor<'a>, b: PartyCursor<'b>) -> Self {
        Self {
            a,
            b,
            pending: BitStack::new(),
            a_on: true,
            b_on: true,
        }
    }

    /// Test whether the predicate holds over every paired region.
    fn holds(mut self, a_decides: impl Fn(Option<PartyNode>) -> bool) -> bool {
        loop {
            let a_node = self.read_a();
            if a_decides(a_node) {
                if self.b_on {
                    self.b.skip();
                }
                if self.complete().is_break() {
                    return true;
                }
                continue;
            }

            let b_node = self.read_b();
            if b_node.is_none() {
                self.a
                    .skip_present_children(a_node.expect("the first region did not decide"));
                if self.complete().is_break() {
                    return true;
                }
                continue;
            }

            match (a_node, b_node) {
                (Some(PartyNode::Branch(a)), Some(PartyNode::Branch(b))) => self.descend(a, b),
                _ => return false,
            }
        }
    }

    /// Decode the current pair's `a`-side node, or return `None` for an absent
    /// child.
    fn read_a(&mut self) -> Option<PartyNode> {
        if self.a_on {
            Some(self.a.read())
        } else {
            None
        }
    }

    /// Read the current region from the second operand.
    fn read_b(&mut self) -> Option<PartyNode> {
        if self.b_on {
            Some(self.b.read())
        } else {
            None
        }
    }

    /// Enter the child pairs of two branches with presence bits
    /// `(al, ar)` / `(bl, br)`: queue the right pair if either side has a right
    /// child, and step into the leftmost pair either side has at all.
    ///
    /// A pair absent on both sides is never walked — both predicates hold
    /// trivially on it, and neither cursor moves for it — which is what keeps
    /// the pending stack empty on unary chains. Both pairs absent cannot
    /// happen: a branch has at least one present child.
    fn descend(&mut self, a: PartyBranch, b: PartyBranch) {
        let (al, ar) = a.presence();
        let (bl, br) = b.presence();
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
