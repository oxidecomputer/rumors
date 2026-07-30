use crate::codec::{Bits, BitsSlice};
use crate::idbits::{IdNode, IdReader};

impl<'a> IdReader<'a> {
    /// Sum `self` and `other` (normal-form ids) and split the union, in
    /// one walk: the two halves of the disjoint union, or `None` if the
    /// ids overlap.
    ///
    /// Byte-identical to [`sum`](IdReader::sum) followed by
    /// [`split`](IdReader::split) of the union, with the union itself
    /// never built.
    ///
    /// The fusion rests on where `split` cuts: it copies the union's unary
    /// *spine* into both halves, then keeps one child of the union's first
    /// both-present node (the *branch*) per half. Both structures are
    /// visible in the operands directly:
    ///
    /// - **The spine is a lockstep descent.** A node's presence in the
    ///   union is the `or` of its presence in the operands, so along a
    ///   unary stretch of the union each operand is itself unary in the
    ///   *same* direction (an operand child on the other side would make
    ///   the union node both-present). Both cursors therefore descend
    ///   their own unary spines together, and the union's spine tags —
    ///   which `split` would copy out of the built union — are emitted to
    ///   both halves as they are read. Neither operand can end (a
    ///   present operand node stays present below, so an empty side is
    ///   settled at the root), so the descent stops at the first
    ///   both-present union node or at an overlap.
    /// - **At the branch, each union child is one operand's subtree or a
    ///   genuine merge.** A child present on one side alone is that
    ///   operand's subtree verbatim — spliced by bit range, exactly the
    ///   bytes `sum` would copy and `split` would slice back out — and a
    ///   child present on both sides is one [`sum`](IdReader::sum) of the
    ///   two subtrees, positioned by cursor. Normal form is
    ///   context-free (a subtree's canonical bits depend only on the
    ///   region set it denotes), so each delegated result equals the
    ///   corresponding range of the built union bit for bit.
    ///
    /// The branch runs in one of two modes, picked so no bit is ever read
    /// twice. When the *left* child pair is a genuine merge, positioning
    /// a both-present operand's right child would mean skipping its left
    /// child and then reading it again inside the merge — paying more
    /// than the composition wherever the union collapses — so the walk
    /// delegates the whole branch subtree pair to `sum` and `split`
    /// (both cursors still sit at the branch tags, and the branch
    /// subtrees are suffixes of their streams): the composition's own
    /// bytes and reads, minus the built union's spine. Everywhere else
    /// the targeted path runs, and every skip it pays feeds a splice.
    ///
    /// The one seam `split` sees that the targeted path does not is the
    /// union collapsing *at* the branch: when both union children are
    /// full, the built union's branch node becomes a terminal and `split`
    /// lands in its terminal arm — which emits per half a one-child node
    /// over a terminal, exactly the tag-plus-full-child bytes the branch
    /// arm here emits. The `sum_split_is_sum_then_split` differential
    /// holds the whole map, `None` arm included, to the composition.
    ///
    /// Overlap is detected exactly as [`sum`](IdReader::sum) detects it —
    /// a full leaf meeting a nonempty region, on the spine or inside a
    /// delegated merge — and nothing is returned partially built.
    ///
    /// `O(n + m)` worst case, never above the composition's own reads,
    /// and sublinear where the operands do not interleave: a subtree
    /// present on one side alone is spliced without reading its nodes,
    /// where the composition pays two scans of it (`sum`'s copy skip,
    /// then `split`'s subtree-end scan) plus its bytes in the built
    /// union.
    pub(crate) fn sum_split(mut self, mut other: IdReader) -> Option<(Bits, Bits)> {
        // An empty operand leaves the union the other operand, whole, so
        // the halves are its plain split. Only the root can be empty:
        // below it, presence in the union keeps both cursors live.
        if matches!(self, IdReader::Empty) {
            return Some(other.split());
        }
        if matches!(other, IdReader::Empty) {
            return Some(self.split());
        }
        // The union's spine tags, shared by both halves (split's prefix).
        let mut spine = Bits::new();
        loop {
            let (a_node, b_node) = (self.peek(), other.peek());
            let (al, ar) = match a_node {
                // A full leaf meets the other id's nonempty region (the
                // other cursor is live, so its subtree here is nonempty):
                // the ids share a region, and there is no disjoint union.
                IdNode::Full => return None,
                IdNode::Internal { left, right } => (left, right),
                IdNode::Empty => unreachable!("both cursors stay live below the root"),
            };
            let (bl, br) = match b_node {
                IdNode::Full => return None,
                IdNode::Internal { left, right } => (left, right),
                IdNode::Empty => unreachable!("both cursors stay live below the root"),
            };
            let (left, right) = (al || bl, ar || br);
            if left && right {
                if al && bl {
                    // The left pair is a genuine merge: delegate the
                    // whole branch subtree pair to the composition (the
                    // method doc's mode argument). The cursors still sit
                    // at the branch tags, and the branch subtrees are
                    // suffixes of their streams, so `sum` merges exactly
                    // them; its root is both-present or full, so `split`
                    // cuts it exactly where it would cut the built
                    // union's branch.
                    let union = self.sum(other)?;
                    let (keep_child, give_child) = IdReader::root(&union).split();
                    return Some((splice(&spine, &keep_child), splice(&spine, &give_child)));
                }
                // The targeted branch: each half keeps the spine, a
                // one-child retag, and its own side's union child.
                self.read();
                other.read();
                let (a_left, a_right) = branch_children(&self, al, ar);
                let (b_left, b_right) = branch_children(&other, bl, br);
                let keep_child = union_child(a_left, b_left)?;
                let give_child = union_child(a_right, b_right)?;
                let keep = half(&spine, true, false, &keep_child);
                let give = half(&spine, false, true, &give_child);
                return Some((keep, give));
            }
            // A unary union node: both operands are unary in the same
            // direction (the module doc's spine argument), and the tag
            // rides into both halves.
            self.read();
            other.read();
            spine.push(left);
            spine.push(right);
        }
    }
}

/// One union child's bits at the branch: an operand's subtree verbatim, or
/// a freshly merged pair.
enum UnionChild<'a> {
    /// The child is one operand's subtree alone: its verbatim bit range.
    Verbatim(&'a BitsSlice),
    /// The child is present on both sides: the merged (summed) subtree.
    Merged(Bits),
}

impl UnionChild<'_> {
    fn len(&self) -> usize {
        match self {
            UnionChild::Verbatim(bits) => bits.len(),
            UnionChild::Merged(bits) => bits.len(),
        }
    }

    fn bits(&self) -> &BitsSlice {
        match self {
            UnionChild::Verbatim(bits) => bits,
            UnionChild::Merged(bits) => bits,
        }
    }
}

/// The bit ranges of one operand's children at the union's branch node,
/// with the cursor just past the node's tag.
///
/// The branch node's subtree is a suffix of its stream (the spine descent
/// above it consumed only unary tags), so the last present child runs to
/// the stream's end and only a both-present operand pays a skip — of its
/// left child, to find the boundary between the two.
fn branch_children<'a>(
    reader: &IdReader<'a>,
    left: bool,
    right: bool,
) -> (Option<&'a BitsSlice>, Option<&'a BitsSlice>) {
    let bits = reader.bits();
    let start = reader.pos();
    match (left, right) {
        (true, true) => {
            let mut probe = IdReader::at(bits, start);
            probe.skip();
            let mid = probe.pos();
            (Some(&bits[start..mid]), Some(&bits[mid..]))
        }
        (true, false) => (Some(&bits[start..]), None),
        (false, true) => (None, Some(&bits[start..])),
        (false, false) => unreachable!("an internal id node has a present child"),
    }
}

/// One union child at the branch: the side present alone, verbatim, or the
/// merge of both — `None` if the merged subtrees overlap.
fn union_child<'a>(a: Option<&'a BitsSlice>, b: Option<&'a BitsSlice>) -> Option<UnionChild<'a>> {
    match (a, b) {
        (Some(a), None) => Some(UnionChild::Verbatim(a)),
        (None, Some(b)) => Some(UnionChild::Verbatim(b)),
        (Some(a), Some(b)) => IdReader::root(a)
            .sum(IdReader::root(b))
            .map(UnionChild::Merged),
        (None, None) => unreachable!("a union branch child is present on some side"),
    }
}

/// Assemble one half: the spine, the branch retagged to its kept side,
/// and the kept child's bits.
fn half(spine: &BitsSlice, left: bool, right: bool, child: &UnionChild) -> Bits {
    let mut out = Bits::with_capacity(spine.len() + 2 + child.len());
    out.extend_from_bitslice(spine);
    out.push(left);
    out.push(right);
    out.extend_from_bitslice(child.bits());
    out
}

/// Assemble one delegated-mode half: the spine, then the composition's
/// own half of the branch subtree's union.
fn splice(spine: &BitsSlice, tail: &BitsSlice) -> Bits {
    let mut out = Bits::with_capacity(spine.len() + tail.len());
    out.extend_from_bitslice(spine);
    out.extend_from_bitslice(tail);
    out
}
