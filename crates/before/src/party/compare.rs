//! Structural comparisons between two parties.
//!
//! The walks align the parties over the same regions of the ownership tree.
//! A region absent from one party needs no stored node on that side. When a
//! uniform region decides the comparison, the other party's complete subtree
//! can be skipped. Otherwise, two branches are descended together. Each stored
//! node is therefore visited at most once.

use core::ops::ControlFlow;

use crate::bits::stack::BitStack;
use crate::party::io::{PartyBranch, PartyNode, PartyReader};

impl PartyReader<'_> {
    /// Whether two canonical party subtrees share no owned region.
    ///
    /// Runs in `O(|self| + |other|)`: both cursors advance without backtracking,
    /// and an unowned region lets the other subtree be skipped.
    ///
    // Takes the cursors by value: a cursor is single-use, and the walk consumes
    // both. (`is_*`-by-value is unusual, hence the allow.)
    #[allow(clippy::wrong_self_convention)]
    pub fn is_disjoint(self, other: PartyReader) -> bool {
        // An unowned first region cannot overlap the second. Any other pair
        // must be examined unless the second region is unowned.
        Comparison::holds(self, other, |first| first.is_none())
    }

    /// Whether `self` (a normal-form party) *covers* `other` — every region
    /// `other` owns is also owned by `self` (`self ⊇ other`).
    ///
    /// `O(|self| + |other|)`: both cursors advance without backtracking, and a
    /// subtree is skipped when the other operand already decides the result, as in
    /// [`is_disjoint`](PartyReader::is_disjoint).
    ///
    // Single-use by-value cursors, as with `is_disjoint`.
    #[allow(clippy::wrong_self_convention)]
    pub fn covers(self, other: PartyReader) -> bool {
        // An owned first operand covers anything in the second. The remaining
        // failures expose ownership missing from the first: an unowned first
        // operand against a nonempty second, or a partially owned first operand
        // against a fully owned second.
        Comparison::holds(self, other, |first| matches!(first, Some(PartyNode::Owned)))
    }
}

/// A shared structural comparison for disjointness and coverage.
///
/// The operation supplies the kind of first region that decides a pair
/// immediately: unowned for disjointness, owned for coverage. Unowned regions
/// in the second operand always pass. Two branches descend together; any other
/// undecided pair fails. Pending right children cost two bits per open branch.
struct Comparison<'a, 'b> {
    /// First operand.
    first: PartyReader<'a>,
    /// Second operand.
    second: PartyReader<'b>,
    /// Two presence bits per queued right child pair, innermost on top.
    pending: BitStack,
    /// Whether the current first operand has a stored node or is unowned.
    first_present: bool,
    /// Whether the current second operand has a stored node or is unowned.
    second_present: bool,
}

impl<'a, 'b> Comparison<'a, 'b> {
    /// Test whether the predicate holds over every paired region.
    fn holds(
        first: PartyReader<'a>,
        second: PartyReader<'b>,
        first_decides: impl Fn(Option<PartyNode>) -> bool,
    ) -> bool {
        let mut comparison = Self {
            first,
            second,
            pending: BitStack::new(),
            first_present: true,
            second_present: true,
        };
        loop {
            let first = comparison.read_first();
            if first_decides(first) {
                if comparison.second_present {
                    comparison.second.skip();
                }
                if comparison.complete().is_break() {
                    return true;
                }
                continue;
            }

            let second = comparison.read_second();
            if second.is_none() {
                comparison
                    .first
                    .skip_present_children(first.expect("the first region did not decide"));
                if comparison.complete().is_break() {
                    return true;
                }
                continue;
            }

            match (first, second) {
                (Some(PartyNode::Branch(first)), Some(PartyNode::Branch(second))) => {
                    comparison.descend(first, second)
                }
                _ => return false,
            }
        }
    }

    /// Read the first region, or return `None` when that child is absent.
    fn read_first(&mut self) -> Option<PartyNode> {
        if self.first_present {
            Some(self.first.read())
        } else {
            None
        }
    }

    /// Read the current region from the second operand.
    fn read_second(&mut self) -> Option<PartyNode> {
        if self.second_present {
            Some(self.second.read())
        } else {
            None
        }
    }

    /// Enter the child pairs of two branches.
    ///
    /// Queue the right pair if either side has a right child, then enter the
    /// left pair when either side has one. If both left children are absent,
    /// enter the right pair directly.
    ///
    /// A pair absent on both sides is never walked — both predicates hold
    /// trivially on it, and neither cursor moves for it — which is what keeps
    /// the pending stack empty on unary chains. Both pairs absent cannot
    /// happen: a branch has at least one present child.
    fn descend(&mut self, first: PartyBranch, second: PartyBranch) {
        let first_left = first.has_left_child();
        let first_right = first.has_right_child();
        let second_left = second.has_left_child();
        let second_right = second.has_right_child();
        if (first_left || second_left) && (first_right || second_right) {
            self.pending.push(first_right);
            self.pending.push(second_right);
            (self.first_present, self.second_present) = (first_left, second_left);
        } else if first_left || second_left {
            (self.first_present, self.second_present) = (first_left, second_left);
        } else {
            (self.first_present, self.second_present) = (first_right, second_right);
        }
    }

    /// Complete the current pair with a passing verdict: step into the
    /// innermost queued right pair (`Continue`), or report the whole walk done
    /// (`Break`) when no ancestor is waiting.
    ///
    /// Ancestors between the queued pair and the completed one queued nothing,
    /// so their completion needs no bookkeeping: it *is* this completion.
    fn complete(&mut self) -> ControlFlow<()> {
        let Some(second_present) = self.pending.pop() else {
            return ControlFlow::Break(());
        };
        let first_present = self
            .pending
            .pop()
            .expect("pending right pairs are two bits");
        (self.first_present, self.second_present) = (first_present, second_present);
        ControlFlow::Continue(())
    }
}
