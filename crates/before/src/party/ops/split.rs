use crate::codec::{Bits, BitsSlice};
use crate::idbits::{IdHeader, IdView};

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
    /// The iterative form of the recursive `oracle::Party::split` (the paper's
    /// `split`). Where the oracle recurses down the spine, this locates the
    /// same branch by a forward scan and rebuilds the two halves without
    /// re-descending.
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

        // Pass 1: find the branch by a single forward preorder scan.
        enum Frame {
            /// An opened node whose left child is being parsed; `start` is its bit position.
            NeedLeft { start: usize },
            /// An opened node whose left child is done and right child is being parsed.
            NeedRight {
                /// The node's bit position.
                start: usize,
                /// Whether the (now-parsed) left child owned nothing.
                left_empty: bool,
                /// Bit position where the right child begins.
                right_start: usize,
            },
        }
        // The branch node `(start, left_start, right_start)`, and any `1` leaf
        // (the branch when the tree is a pure spine with no both-nonempty
        // node).
        let mut branch: Option<(usize, usize, usize)> = None;
        let mut one_leaf: Option<usize> = None;
        let mut stack: Vec<Frame> = Vec::new();
        let mut pos = 0;
        // Two interleaved phases per outer iteration: phase A descends left to
        // a leaf (pushing `NeedLeft` frames); phase B (the inner `loop`) pops
        // completed ancestors, recording the shallowest both-nonempty node as
        // the branch, until one still needs its right child (then resume phase
        // A there) or the stack empties (then build).
        loop {
            let IdHeader {
                node: is_node,
                val,
                next,
            } = id.header(pos);
            let start = pos;
            pos = next;
            // What the just-parsed subtree reports to its parent: was it empty?
            let mut child_empty = if is_node {
                stack.push(Frame::NeedLeft { start });
                continue; // descend into the left child
            } else {
                if val {
                    one_leaf.get_or_insert(start);
                }
                !val // a `0` leaf is empty; a `1` leaf is not
            };
            // Bubble the completed subtree up, completing ancestors as their
            // turn comes.
            loop {
                match stack.pop() {
                    None => return build_split(bits, branch, one_leaf),
                    Some(Frame::NeedLeft { start }) => {
                        stack.push(Frame::NeedRight {
                            start,
                            left_empty: child_empty,
                            right_start: pos,
                        });
                        break; // parse the right child next
                    }
                    Some(Frame::NeedRight {
                        start,
                        left_empty,
                        right_start,
                    }) => {
                        let both_nonempty = !left_empty && !child_empty;
                        if both_nonempty && branch.is_none_or(|(p, ..)| start < p) {
                            branch = Some((start, start + 1, right_start));
                        }
                        child_empty = false; // a normal-form node is never empty
                    }
                }
            }
        }
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
