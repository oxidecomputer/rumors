use crate::codec::{Bits, BitsSlice};
use crate::idbits::IdView;

use super::build::{id_leaf, id_node};

impl IdView<'_> {
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
    /// The scan is guarded by [`crate::recurse`] so deep ids grow the stack onto
    /// the heap rather than overflowing.
    pub(crate) fn split(&self) -> (Bits, Bits) {
        let id = *self;
        let bits = id.bits();
        // A whole-tree leaf splits directly.
        let root = id.header(0);
        if !root.node {
            return if root.val {
                // split(1) = ((1, 0), (0, 1))
                (
                    id_node(&id_leaf(true), &id_leaf(false)),
                    id_node(&id_leaf(false), &id_leaf(true)),
                )
            } else {
                (id_leaf(false), id_leaf(false))
            };
        }

        // Pass 1: locate the branch by a single recursive preorder scan.
        let mut scan = SplitScan {
            id,
            branch: None,
            one_leaf: None,
        };
        scan.scan(0, 0);
        build_split(bits, scan.branch, scan.one_leaf)
    }
}

/// Pass-1 scan state for [`split`](IdView::split): the id being scanned, the
/// branch node found so far (`(start, left_start, right_start)`, the
/// shallowest both-nonempty node), and any `1` leaf (the branch when the tree
/// is a pure spine with no both-nonempty node).
struct SplitScan<'a> {
    id: IdView<'a>,
    branch: Option<(usize, usize, usize)>,
    one_leaf: Option<usize>,
}

impl SplitScan<'_> {
    /// Scan the subtree at `pos`, recording the shallowest both-nonempty node as
    /// the branch and the first `1` leaf, routed through the amortized
    /// stack-growth guard. Returns `(empty, end)`: whether the subtree owns
    /// nothing, and the position just past it.
    fn scan(&mut self, pos: usize, depth: usize) -> (bool, usize) {
        crate::recurse::guarded(depth, move || {
            let hdr = self.id.header(pos);
            if !hdr.node {
                if hdr.val {
                    self.one_leaf.get_or_insert(pos);
                }
                return (!hdr.val, hdr.next); // a `0` leaf is empty; a `1` leaf is not
            }
            let left_start = hdr.next;
            let (left_empty, right_start) = self.scan(left_start, depth + 1);
            let (right_empty, end) = self.scan(right_start, depth + 1);
            // The shallowest both-nonempty node wins (smallest start); a parent's
            // position is always less than its descendants', and postorder visits
            // children first, so the parent overwrites any descendant branch.
            if !left_empty && !right_empty && self.branch.is_none_or(|(p, ..)| pos < p) {
                self.branch = Some((pos, left_start, right_start));
            }
            (false, end) // a normal-form node is never empty
        })
    }
}

/// Build the two split halves once the branch is located (see
/// [`split`](IdView::split)). `a` keeps the branch's left side (its right
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
        let branch_end = IdView(bits).skip(right_start);
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
