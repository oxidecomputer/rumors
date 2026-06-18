use crate::codec::{Bits, BitsSlice};
use crate::idbits::{IdNode, IdReader};
use crate::step;

impl IdReader<'_> {
    /// Split this id (`self`) into two non-overlapping ids that sum to it.
    /// `O(n)`: descend the *spine* — the chain of unary nodes, each owning
    /// exactly one present child — to the *branch* (the first node with both
    /// children present) or the spine's terminal `1` leaf, then build both
    /// halves by copying the spine prefix with one side of the branch kept.
    ///
    /// In the pruned encoding a unary node names its one present child directly
    /// (the child sits at `pos + 2`, right past the tag), so the spine walk is a
    /// simple loop — no recursion, no emptiness scan. At the branch each half
    /// keeps one child and drops the other: a `Both` tag becomes `Left-only` (a)
    /// or `Right-only` (b), and the dropped child is simply omitted (a `0` is
    /// absence, not a leaf). At a terminal the split is `(1,0)`/`(0,1)`.
    ///
    /// The recursive form of `oracle::Party::split` (the paper's `split`).
    pub(crate) fn split(self) -> (Bits, Bits) {
        // split(0) = (0, 0): the empty id splits into two empties.
        if let IdNode::Empty = self.peek() {
            return (Bits::new(), Bits::new());
        }
        let start = self.pos();
        build_split(self.bits(), start)
    }
}

/// Where the spine descent of [`split`](IdReader::split) ended.
enum SpineEnd {
    /// A both-present node `(i1, i2)`: split keeps one child per half.
    Branch,
    /// A terminal `1` leaf: split is `(1,0)`/`(0,1)`.
    Terminal,
}

/// Build the two split halves of the id rooted at `start` in `bits`. Walks the
/// unary spine to the branch (or terminal), then splices: each half is the
/// spine prefix, a retagged node, and the kept child — a bulk verbatim copy of
/// already-normal bit ranges, normal by construction (the kept child is
/// nonempty, so no collapse can arise). Iterative: the spine walk is a loop, so
/// deep ids cannot overflow.
fn build_split(bits: &BitsSlice, start: usize) -> (Bits, Bits) {
    let mut pos = start;
    let (prefix_end, kind) = loop {
        step!(); // one node-header read, counted for the complexity proptests
        match (bits[pos], bits[pos + 1]) {
            (false, false) => break (pos, SpineEnd::Terminal), // the `1` leaf
            (true, true) => break (pos, SpineEnd::Branch),     // both-present branch
            _ => pos += 2, // unary: descend the single present child (at pos + 2)
        }
    };
    let prefix = &bits[start..prefix_end];

    match kind {
        SpineEnd::Branch => {
            // Branch `(i1, i2)`: i1 = bits[left_child..right_child], i2 =
            // bits[right_child..branch_end], with the spine in the prefix.
            let left_child = prefix_end + 2;
            let right_child = subtree_end(bits, left_child);
            let branch_end = subtree_end(bits, right_child);
            debug_assert_eq!(
                branch_end,
                bits.len(),
                "the branch subtree is the spine's tail",
            );
            let i1 = &bits[left_child..right_child];
            let i2 = &bits[right_child..branch_end];

            // Each half keeps one child and drops the other, so its length is
            // exact: prefix + the 2-bit retagged branch + the kept child.
            let mut a = Bits::with_capacity(prefix.len() + 2 + i1.len());
            a.extend_from_bitslice(prefix);
            a.push(true); // branch → Left-only: keep i1 ...
            a.push(false); // ... drop i2
            a.extend_from_bitslice(i1);

            let mut b = Bits::with_capacity(prefix.len() + 2 + i2.len());
            b.extend_from_bitslice(prefix);
            b.push(false); // branch → Right-only: drop i1 ...
            b.push(true); // ... keep i2
            b.extend_from_bitslice(i2);

            (a, b)
        }
        SpineEnd::Terminal => {
            // split(1) = ((1, 0), (0, 1)): the terminal becomes a unary node
            // over a terminal on each side.
            debug_assert_eq!(
                prefix_end + 2,
                bits.len(),
                "the terminal is the spine's tail",
            );
            let mut a = Bits::with_capacity(prefix.len() + 4);
            a.extend_from_bitslice(prefix);
            a.push(true); // (1, 0): Left-only ...
            a.push(false);
            a.push(false); // ... over a terminal
            a.push(false);

            let mut b = Bits::with_capacity(prefix.len() + 4);
            b.extend_from_bitslice(prefix);
            b.push(false); // (0, 1): Right-only ...
            b.push(true);
            b.push(false); // ... over a terminal
            b.push(false);

            (a, b)
        }
    }
}

/// The bit position just past the subtree at `pos` (the shared
/// [`skip`](IdReader::skip) scan), for slicing a branch child's verbatim range.
fn subtree_end(bits: &BitsSlice, pos: usize) -> usize {
    let mut r = IdReader::at(bits, pos);
    r.skip();
    r.pos()
}
