use crate::codec::{Bits, BitsSlice};
use crate::idbits::{IdNode, IdReader};
use crate::recurse::descend;

use super::build::{id_leaf, id_node};

impl IdReader<'_> {
    /// Split this id (`self`) into two non-overlapping ids that sum to it.
    /// `O(n)` in two passes: locate the *branch* — the shallowest node along
    /// the (unique) nonempty spine whose two children both own something, or
    /// the spine's terminal `1` leaf — then build both halves by copying the
    /// input with one side of the branch zeroed.
    ///
    /// The branch is the both-nonempty node of minimum start position (all
    /// shallower nodes are spine wrappers, with one empty child), found by a
    /// single forward scan rather than by descending and re-scanning to test
    /// each right child for emptiness.
    ///
    /// The recursive form of `oracle::Party::split` (the paper's `split`). Where
    /// the oracle recurses down the spine, this records the same branch during a
    /// single recursive scan and rebuilds the two halves without re-descending.
    /// The scan threads an [`IdReader`] but records bit *positions* — the branch
    /// node's and its children's — since `build_split` splices the input on
    /// them. Guarded by [`crate::recurse`] so deep ids grow the stack onto the
    /// heap rather than overflowing.
    pub(crate) fn split(self) -> (Bits, Bits) {
        let bits = self.bits();
        // A whole-tree leaf splits directly.
        let (root, _) = self.read();
        match root {
            // split(0) = (0, 0)
            IdNode::Empty => return (id_leaf(false), id_leaf(false)),
            // split(1) = ((1, 0), (0, 1))
            IdNode::Full => {
                return (
                    id_node(&id_leaf(true), &id_leaf(false)),
                    id_node(&id_leaf(false), &id_leaf(true)),
                )
            }
            IdNode::Internal => {}
        }

        // Pass 1: locate the branch by a single recursive preorder scan.
        let mut scan = SplitScan {
            branch: None,
            one_leaf: None,
        };
        descend!(0, scan.scan(self, 0));
        build_split(bits, scan.branch, scan.one_leaf)
    }
}

/// Pass-1 scan state for [`split`](IdReader::split): the branch node found so
/// far (`(start, left_start, right_start)` bit positions, the shallowest
/// both-nonempty node), and any `1` leaf position (the branch when the tree is
/// a pure spine with no both-nonempty node).
struct SplitScan {
    branch: Option<(usize, usize, usize)>,
    one_leaf: Option<usize>,
}

impl SplitScan {
    /// Scan the subtree at `id`, recording the shallowest both-nonempty node as
    /// the branch and the first `1` leaf, routed through the amortized
    /// stack-growth guard. Returns `(empty, end)`: whether the subtree owns
    /// nothing, and a reader just past it.
    fn scan<'a>(&mut self, id: IdReader<'a>, depth: usize) -> (bool, IdReader<'a>) {
        let (node, after) = id.read();
        match node {
            IdNode::Empty => (true, after), // a `0` leaf is empty
            IdNode::Full => {
                self.one_leaf.get_or_insert(id.pos());
                (false, after) // a `1` leaf is not empty
            }
            IdNode::Internal => {
                let left = after;
                let (left_empty, right) = descend!(depth + 1, self.scan(left, depth + 1));
                let (right_empty, end) = descend!(depth + 1, self.scan(right, depth + 1));
                // The shallowest both-nonempty node wins (smallest start): a
                // parent's position is always less than its descendants', and
                // postorder visits children first, so the parent overwrites any
                // descendant branch.
                if !left_empty && !right_empty && self.branch.is_none_or(|(p, ..)| id.pos() < p) {
                    self.branch = Some((id.pos(), left.pos(), right.pos()));
                }
                (false, end) // a normal-form node is never empty
            }
        }
    }
}

/// Build the two split halves once the branch is located (see
/// [`split`](IdReader::split)). `a` keeps the branch's left side (its right
/// zeroed); `b` keeps the right side (its left zeroed).
fn build_split(
    bits: &BitsSlice,
    branch: Option<(usize, usize, usize)>,
    one_leaf: Option<usize>,
) -> (Bits, Bits) {
    let zero = id_leaf(false);
    if let Some((p, left_start, right_start)) = branch {
        // Branch is a node `(i1, i2)`: i1 = bits[left_start..right_start], i2 =
        // bits[right_start..branch_end], with the wrapper spine in the prefix
        // bits[0..p] and the trailing wrapper closings in bits[branch_end..].
        let branch_end = IdReader::at(bits, right_start).skip().pos();
        let prefix = &bits[0..p];
        let i1 = &bits[left_start..right_start];
        let i2 = &bits[right_start..branch_end];
        let suffix = &bits[branch_end..];

        let mut a = Bits::with_capacity(bits.len());
        a.extend_from_bitslice(prefix);
        a.push(true); // the branch node, right child zeroed
        a.extend_from_bitslice(i1);
        a.extend_from_bitslice(&zero);
        a.extend_from_bitslice(suffix);

        let mut b = Bits::with_capacity(bits.len());
        b.extend_from_bitslice(prefix);
        b.push(true); // the branch node, left child zeroed
        b.extend_from_bitslice(&zero);
        b.extend_from_bitslice(i2);
        b.extend_from_bitslice(suffix);

        (a, b)
    } else {
        // No both-nonempty node: the spine ends in a `1` leaf, split as
        // (1,0)/(0,1).
        let p = one_leaf.expect("a nonempty id has a branch node or a 1 leaf");
        let prefix = &bits[0..p];
        let suffix = &bits[p + 2..]; // the `1` leaf occupies 2 bits
        let one = id_leaf(true);

        let mut a = Bits::with_capacity(bits.len() + 3);
        a.extend_from_bitslice(prefix);
        a.extend_from_bitslice(&id_node(&one, &zero));
        a.extend_from_bitslice(suffix);

        let mut b = Bits::with_capacity(bits.len() + 3);
        b.extend_from_bitslice(prefix);
        b.extend_from_bitslice(&id_node(&zero, &one));
        b.extend_from_bitslice(suffix);

        (a, b)
    }
}
