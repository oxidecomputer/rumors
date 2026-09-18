use crate::codec::{extend_from_view, BitBuilder, BitsBuf, BitsView};
use crate::idbits::{IdNode, IdReader};

impl IdReader<'_> {
    /// Split this id (`self`) into two non-overlapping ids that sum to it.
    ///
    /// `O(|self|)`: descend the *spine* — the chain of unary nodes, each owning
    /// exactly one present child — to the *branch* (the first node with both
    /// children present) or the spine's terminal `1` leaf, then build both
    /// halves by copying the spine prefix with one side of the branch kept.
    ///
    /// In the pruned encoding a unary node names its one present child directly
    /// (the child sits at `pos + 2`, right past the tag), so the spine walk is
    /// a simple loop — no recursion, no emptiness scan. At the branch each half
    /// keeps one child and drops the other: a `Both` tag becomes `Left-only`
    /// (a) or `Right-only` (b), and the dropped child is simply omitted (a `0`
    /// is absence, not a leaf). At a terminal the split is `(1,0)`/`(0,1)`.
    ///
    /// The recursive form of `oracle::Party::split` (the paper's `split`).
    pub(crate) fn split(self) -> (BitsBuf, BitsBuf) {
        // split(0) = (0, 0): the empty id splits into two empties.
        if let IdNode::Empty = self.peek() {
            return (BitsBuf::new(), BitsBuf::new());
        }
        let start = self.pos();
        build_split(self.bits(), start)
    }

    /// Select one descendant by a sequence of binary splits.
    ///
    /// Each `false` keeps the left half of the current region and each `true`
    /// keeps the right. This produces the same bits as repeatedly calling
    /// [`split`](Self::split) and retaining the selected half, but descends
    /// through the source only once and builds only the final descendant.
    pub(crate) fn split_path(self, path: impl IntoIterator<Item = bool>) -> BitsBuf {
        let IdReader::At { bits, pos } = self else {
            return BitsBuf::new();
        };
        build_split_path(bits, pos, path)
    }
}

/// Where the spine descent of [`split`](IdReader::split) ended.
enum SpineEnd {
    /// A both-present node `(i1, i2)`: split keeps one child per half.
    Branch,
    /// A terminal `1` leaf: split is `(1,0)`/`(0,1)`.
    Terminal,
}

/// Build the two split halves of the id rooted at `start` in `bits`.
///
/// Walks the unary spine to the branch (or terminal), then splices: each half
/// is the spine prefix, a retagged node, and the kept child — a bulk verbatim
/// copy of already-normal bit ranges, normal by construction (the kept child is
/// nonempty, so no collapse can arise). Iterative: the spine walk is a loop, so
/// deep ids cannot overflow.
fn build_split(bits: BitsView<'_>, start: u64) -> (BitsBuf, BitsBuf) {
    let mut pos = start;
    let (prefix_end, kind) = loop {
        match (bits.bit(pos), bits.bit(pos + 1)) {
            (false, false) => break (pos, SpineEnd::Terminal), // the `1` leaf
            (true, true) => break (pos, SpineEnd::Branch),     // both-present branch
            _ => pos += 2, // unary: descend the single present child (at pos + 2)
        }
    };
    let prefix = start..prefix_end;
    let prefix_len = prefix_end - start;

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

            // Each half keeps one child and drops the other, so its length is
            // exact: prefix + the 2-bit retagged branch + the kept child.
            let mut a = BitsBuf::with_capacity(prefix_len + 2 + (right_child - left_child));
            extend_from_view(&mut a, bits, prefix.start, prefix.end);
            a.push(true); // branch → Left-only: keep i1 ...
            a.push(false); // ... drop i2
            extend_from_view(&mut a, bits, left_child, right_child);

            let mut b = BitsBuf::with_capacity(prefix_len + 2 + (branch_end - right_child));
            extend_from_view(&mut b, bits, prefix.start, prefix.end);
            b.push(false); // branch → Right-only: drop i1 ...
            b.push(true); // ... keep i2
            extend_from_view(&mut b, bits, right_child, branch_end);

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
            let mut a = BitsBuf::with_capacity(prefix_len + 4);
            extend_from_view(&mut a, bits, prefix.start, prefix.end);
            a.push(true); // (1, 0): Left-only ...
            a.push(false);
            a.push(false); // ... over a terminal
            a.push(false);

            let mut b = BitsBuf::with_capacity(prefix_len + 4);
            extend_from_view(&mut b, bits, prefix.start, prefix.end);
            b.push(false); // (0, 1): Right-only ...
            b.push(true);
            b.push(false); // ... over a terminal
            b.push(false);

            (a, b)
        }
    }
}

/// Apply a sequence of binary splits while building only the selected result.
///
/// One ordinary [`split`](IdReader::split) first follows the region's unary
/// prefix: both halves inherit that prefix, so it cannot separate them. At the
/// first branch it keeps one child and changes the branch to the corresponding
/// unary tag. At a terminal it creates that unary node and puts a terminal in
/// the selected child. Repeating `split` would copy the prefix accumulated so
/// far on every step.
///
/// This builder composes those steps in one forward descent. Its loop
/// maintains:
///
/// - `out` is exactly the final encoding before the current selected region;
/// - `pos` is the root of that region in the original `bits`.
///
/// For each direction in `path`, it walks to the next place that can split:
///
/// 1. A unary source node belongs to both possible halves. Copy its tag and
///    advance to its only child, which immediately follows the tag.
/// 2. At a branch, append the unary tag for the chosen half and move `pos` to
///    that child. The left child begins after the branch tag; choosing right
///    scans past the discarded left subtree once.
/// 3. At a terminal, the source has no deeper structure. Every remaining
///    choice therefore becomes a new unary node, followed by one terminal, and
///    the result is complete.
///
/// If the path ends inside the source, the whole selected subtree is unchanged
/// and is appended verbatim. Discarded sibling subtrees are disjoint, so their
/// skip scans do not overlap; selected source structure is never revisited from
/// the root. Once the path is exhausted, one final scan locates the end of the
/// selected subtree and that range is copied into `out`. The work is
/// `O(|bits| + |path| + |output|)` and the only allocated tree is the output.
fn build_split_path(
    bits: BitsView<'_>,
    start: u64,
    path: impl IntoIterator<Item = bool>,
) -> BitsBuf {
    let mut out = BitBuilder::with_capacity(0);
    let mut pos = start;
    let mut path = path.into_iter();

    while let Some(right) = path.next() {
        loop {
            // This walk reads tags directly rather than through `IdReader`.
            crate::codec::scan::record_bits(2);
            match (bits.bit(pos), bits.bit(pos + 1)) {
                (false, false) => {
                    push_unary(&mut out, right);
                    for right in path {
                        push_unary(&mut out, right);
                    }
                    out.push_bit(false);
                    out.push_bit(false);
                    return out.finish();
                }
                (true, true) => {
                    let left = pos + 2;
                    push_unary(&mut out, right);
                    pos = if right {
                        // Preorder stores the complete left subtree before the
                        // right subtree. Its end is therefore the right root.
                        subtree_end(bits, left)
                    } else {
                        left
                    };
                    break;
                }
                (left_present, right_present) => {
                    // A unary node's only child starts immediately after its
                    // tag; both split halves inherit this path unchanged.
                    out.push_bit(left_present);
                    out.push_bit(right_present);
                    pos += 2;
                }
            }
        }
    }

    // No requested split reaches inside this region, so it survives exactly as
    // encoded in the source.
    let end = subtree_end(bits, pos);
    out.splice(bits, pos, end);
    out.finish()
}

/// Append a unary node retaining the selected child.
fn push_unary(out: &mut BitBuilder, right: bool) {
    out.push_bit(!right);
    out.push_bit(right);
}

/// The bit position just past the subtree at `pos` (the shared
/// [`skip`](IdReader::skip) scan), for slicing a branch child's verbatim range.
fn subtree_end(bits: BitsView<'_>, pos: u64) -> u64 {
    let mut r = IdReader::at(bits, pos);
    r.skip();
    r.pos()
}
