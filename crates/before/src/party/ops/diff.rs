use crate::codec::Bits;
use crate::idbits::{IdNode, IdReader};
use crate::recurse::descend;

use super::build::{IdBuilder, Slot};

impl IdReader<'_> {
    /// The region *difference* `self \ other` (normal-form ids): the part of
    /// `self`'s region that `other` does not own, as a normalized id.
    ///
    /// Unlike [`sum`](IdReader::sum), `diff` is *total* — overlap is the whole
    /// point, not an error — and its result may be the **empty** `0` leaf,
    /// exactly when `other` covers `self`. The caller ([`Party::without`])
    /// maps that empty result to `None`, since a `Party` is a nonzero share.
    ///
    /// The result is always a subregion of `self` (`self \ other ⊆ self`), so it
    /// introduces no region `self` did not already own. That is what keeps it
    /// linearity-safe where a general id *meet* is not (see the note on the
    /// absent `BitAnd for Clock` in [`oracle`](crate::oracle)): carving a
    /// sub-share out of a region you already hold, and consuming the original,
    /// can never synthesize a region shared with a third live party.
    ///
    /// `O(n + m)`: the both-internal case threads (no skip); `diff(0, b)` and
    /// `diff(a, 1)` skip the dominated side once; `diff(a, 0)` copies `a` and
    /// `diff(1, b)` complements `b`, each bounded by the output size.
    ///
    /// The recursive form of `oracle::Party::without`, guarded by
    /// [`crate::recurse`] so deep ids grow the stack onto the heap rather than
    /// overflowing.
    pub(crate) fn diff(mut self, mut other: IdReader) -> Bits {
        let mut walk = DiffWalk {
            // `self \ other` is a subregion of `self`, but `diff(1, b)` emits
            // `complement(b)`, which can be as large as `other`. Both inputs
            // combined is a safe bound; normalization only shrinks it.
            out: IdBuilder::with_capacity(self.bits().len() + other.bits().len()),
        };
        descend!(0, walk.rec(&mut self, &mut other, 0));
        walk.out.finish()
    }
}

/// The single output builder of a [`diff`](IdReader::diff) walk; the `&mut`
/// readers carry the traversal state, exactly as in [`sum`](IdReader::sum).
struct DiffWalk {
    out: IdBuilder,
}

impl DiffWalk {
    /// Difference the subtrees at the two `&mut` readers, emitting into `out`
    /// and advancing both readers past their subtrees. Reads as a match on the
    /// two id nodes: `diff(0, b) = 0` and `diff(a, 1) = 0` keep nothing (skip
    /// both sides), `diff(a, 0) = a` copies the survivor verbatim, `diff(1, b) =
    /// complement(b)` keeps what `b` lacks, and two nodes recurse and normalize
    /// on close.
    ///
    /// The kept side is [`peek`](IdReader::peek)ed, not read, so `copy_reader`
    /// can splice its whole subtree.
    fn rec(&mut self, a: &mut IdReader, b: &mut IdReader, depth: usize) -> Slot {
        match (a.peek(), b.peek()) {
            // diff(0, b) = 0: `self` owns nothing here. Skip both to resync.
            (IdNode::Empty, _) => {
                a.skip();
                b.skip();
                self.out.leaf(false).into()
            }
            // diff(a, 0) = a: `other` owns nothing here, so keep `a` verbatim.
            (_, IdNode::Empty) => {
                let out_root = self.out.copy_reader(a);
                b.skip();
                out_root
            }
            // diff(a, 1) = 0: `other` owns the whole region, nothing survives.
            (_, IdNode::Full) => {
                a.skip();
                b.skip();
                self.out.leaf(false).into()
            }
            // diff(1, b) = complement(b): `self` owns everything here, so the
            // survivors are exactly the region `b` does *not* own.
            (IdNode::Full, _) => {
                a.skip(); // consume the full `1` leaf
                self.complement(b, depth)
            }
            // Both internal: difference each half (the cursors thread left then
            // right), then close the node, which normalizes.
            (IdNode::Internal, IdNode::Internal) => {
                a.read();
                b.read();
                let node = self.out.open();
                descend!(depth + 1, self.rec(a, b, depth + 1));
                let right = descend!(depth + 1, self.rec(a, b, depth + 1));
                self.out.close_node(node, right)
            }
        }
    }

    /// Emit `complement(b)` — the region `b` does *not* own — advancing `b` past
    /// its subtree. `complement(0) = 1`, `complement(1) = 0`, and an internal
    /// node complements each child. A complemented normal id is already normal
    /// (flipping the leaves of a non-collapsible node cannot make it
    /// collapsible), so `close_node` never actually collapses here; it is used
    /// for uniformity with the rest of the builder.
    fn complement(&mut self, b: &mut IdReader, depth: usize) -> Slot {
        match b.read() {
            IdNode::Empty => self.out.leaf(true).into(),
            IdNode::Full => self.out.leaf(false).into(),
            IdNode::Internal => {
                let node = self.out.open();
                descend!(depth + 1, self.complement(b, depth + 1));
                let right = descend!(depth + 1, self.complement(b, depth + 1));
                self.out.close_node(node, right)
            }
        }
    }
}
