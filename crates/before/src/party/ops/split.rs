use crate::codec::{Bits, BitsSlice};
use crate::idbits::{IdNode, IdReader};
use crate::party::ops::build::IdBuilder;
use crate::recurse::descend;
use crate::step;

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
        match self.peek() {
            // split(0) = (0, 0)
            IdNode::Empty => {
                let empty = {
                    let mut id = IdBuilder::with_capacity(2);
                    id.leaf(false);
                    id.finish()
                };
                return (empty.clone(), empty);
            }
            // split(1) = ((1, 0), (0, 1))
            IdNode::Full => {
                let left = {
                    let mut id = IdBuilder::with_capacity(5);
                    let node = id.open();
                    id.leaf(true);
                    let right = id.leaf(false);
                    id.close_node(node, right);
                    id.finish()
                };
                let right = {
                    let mut id = IdBuilder::with_capacity(5);
                    let node = id.open();
                    id.leaf(false);
                    let right = id.leaf(true);
                    id.close_node(node, right);
                    id.finish()
                };
                return (left, right);
            }
            IdNode::Internal => {}
        }

        // Pass 1: locate the branch by a single recursive preorder scan from the
        // root header at bit 0.
        let mut scan = SplitScan {
            branch: None,
            one_leaf: None,
        };
        descend!(0, scan.scan(bits, 0, 0));
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
    /// Scan the subtree whose root header is at bit `pos` in `bits`, recording
    /// the shallowest both-nonempty node as the branch and the first `1` leaf,
    /// and routing through the amortized stack-growth guard. Returns
    /// `(empty, end)`: whether the subtree owns nothing, and the bit position
    /// just past it (so a parent's right child resumes there).
    ///
    /// Unlike every other walk in the crate, this one is **positional** — it
    /// reads the packed `enc_id` bits at explicit offsets rather than threading
    /// an [`IdReader`](crate::idbits) cursor — because its entire output *is* bit
    /// positions: [`build_split`] reconstructs the two halves by splicing the
    /// input on the recorded `(node, left, right)` offsets. A consuming cursor
    /// would hide exactly the positions this scan exists to capture and force it
    /// to re-derive them at every node; threaded as `usize` here they fall out
    /// of the recursion for free (see below). The `enc_id` layout it decodes:
    ///
    /// - **Leaf** — two bits: a `0` flag, then a value bit. So `bits[pos]` is
    ///   `false`, `bits[pos + 1]` is the value (`true` = the full `1` leaf,
    ///   owned; `false` = the empty `0` leaf), and the leaf spans `[pos, pos+2)`.
    /// - **Node** — a `1` flag, then its two child subtrees in preorder. So
    ///   `bits[pos]` is `true`; the **left** child's header begins at `pos + 1`
    ///   (immediately past the single flag bit — this is the "for free" part),
    ///   and the **right** child's begins where the left subtree ends, which the
    ///   left recursion returns. The node spans `[pos, end)`.
    fn scan(&mut self, bits: &BitsSlice, pos: usize, depth: usize) -> (bool, usize) {
        step!(); // one node-header read, counted for the complexity proptests
        if !bits[pos] {
            // Leaf, `[pos, pos + 2)`. The `1` leaf is nonempty (and the branch
            // fallback for a pure spine); the `0` leaf is empty.
            let full = bits[pos + 1];
            if full {
                self.one_leaf.get_or_insert(pos);
            }
            return (!full, pos + 2);
        }
        // Node: left child just past the flag bit; right child where the left
        // subtree ends (the position the left recursion hands back).
        let left_pos = pos + 1;
        let (left_empty, right_pos) = descend!(depth + 1, self.scan(bits, left_pos, depth + 1));
        let (right_empty, end) = descend!(depth + 1, self.scan(bits, right_pos, depth + 1));
        // The shallowest both-nonempty node wins (smallest start): a parent's
        // position is always less than its descendants', and postorder visits
        // children first, so the parent overwrites any descendant branch.
        if !left_empty && !right_empty && self.branch.is_none_or(|(p, ..)| pos < p) {
            self.branch = Some((pos, left_pos, right_pos));
        }
        (false, end) // a normal-form node is never empty
    }
}

/// Build the two split halves once the branch is located (see
/// [`split`](IdReader::split)). `a` keeps the branch's left side (its right
/// zeroed); `b` keeps the right side (its left zeroed).
///
/// Unlike `sum`, this mostly doesn't use [`IdBuilder`]:
/// it is a bulk verbatim splice of already-normal bit ranges with one branch
/// child replaced by a `0` leaf, which *preserves* normal form by construction
/// (the kept child is nonempty, so no `(0,0)`/`(1,1)` collapse can arise, and
/// the spine/suffix were already normal). `IdBuilder` exists to normalize a
/// recursive node-by-node assembly, where equal-leaf children do arise; a splice
/// has nothing to collapse, so routing through it would only re-emit node by
/// node for no benefit.
fn build_split(
    bits: &BitsSlice,
    branch: Option<(usize, usize, usize)>,
    one_leaf: Option<usize>,
) -> (Bits, Bits) {
    let zero = {
        let mut id = IdBuilder::with_capacity(2);
        id.leaf(false);
        id.finish()
    };
    if let Some((p, left_start, right_start)) = branch {
        // Branch is a node `(i1, i2)`: i1 = bits[left_start..right_start], i2 =
        // bits[right_start..branch_end], with the wrapper spine in the prefix
        // bits[0..p] and the trailing wrapper closings in bits[branch_end..].
        let branch_end = {
            let mut r = IdReader::at(bits, right_start);
            r.skip();
            r.pos()
        };
        let prefix = &bits[0..p];
        let i1 = &bits[left_start..right_start];
        let i2 = &bits[right_start..branch_end];
        let suffix = &bits[branch_end..];

        // Each half keeps one child and zeroes the other, so its length is
        // exact: prefix + branch flag + kept child + `0` leaf + suffix. (Sizing
        // to `bits.len()` would over-allocate by the discarded child, up to
        // half the input.)
        let mut a = Bits::with_capacity(prefix.len() + 1 + i1.len() + zero.len() + suffix.len());
        a.extend_from_bitslice(prefix);
        a.push(true); // the branch node, right child zeroed
        a.extend_from_bitslice(i1);
        a.extend_from_bitslice(&zero);
        a.extend_from_bitslice(suffix);

        let mut b = Bits::with_capacity(prefix.len() + 1 + zero.len() + i2.len() + suffix.len());
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
        let left = {
            let mut id = IdBuilder::with_capacity(5);
            let node = id.open();
            id.leaf(true);
            let right = id.leaf(false);
            id.close_node(node, right);
            id.finish()
        };
        let right = {
            let mut id = IdBuilder::with_capacity(5);
            let node = id.open();
            id.leaf(false);
            let right = id.leaf(true);
            id.close_node(node, right);
            id.finish()
        };

        let mut a = Bits::with_capacity(bits.len() + 3);
        a.extend_from_bitslice(prefix);
        a.extend_from_bitslice(&left);
        a.extend_from_bitslice(suffix);

        let mut b = Bits::with_capacity(bits.len() + 3);
        b.extend_from_bitslice(prefix);
        b.extend_from_bitslice(&right);
        b.extend_from_bitslice(suffix);

        (a, b)
    }
}
