use crate::codec::BitsMut;
use crate::idbits::{IdNode, IdReader};

use super::build::{Built, IdBuilder};

impl IdReader<'_> {
    /// Sum `self` and `other` (normal-form ids) — the union of their regions —
    /// producing a normalized id, or `None` if they overlap (share a region, so
    /// no disjoint union exists).
    ///
    /// This is the single point of overlap detection: callers (`Party::join`)
    /// need not pre-check [`is_disjoint`](IdReader::is_disjoint), since a
    /// successful `sum` *is* the disjointness proof. `O(n + m)`: the
    /// both-internal case threads (no skip); a `0` child copies the other
    /// subtree verbatim (work bounded by the output size).
    ///
    /// The cursor form of `oracle::Party::sum` (the paper's `sum`/`norm`),
    /// walked iteratively: the two consuming cursors carry the traversal, and
    /// the per-node control state is two or three bits on a bit stack (see
    /// [`Frames`]), so a deep operand costs bits, not stack frames or grown
    /// segments. Each pair of subtrees reads as a match on the two id nodes:
    /// `sum(0, b) = b`, `sum(a, 0) = a` (copy the nonempty side), two nodes
    /// descend and normalize on close, and a full side over a nonempty other is
    /// an overlap — `None` at once, discarding the partial output.
    ///
    /// The nodes are [`peek`](IdReader::peek)ed, not read: a copied side must
    /// stay unconsumed so `copy_reader` can splice its whole subtree.
    pub(crate) fn sum(mut self, mut other: IdReader) -> Option<BitsMut> {
        // Conservative: the disjoint union has at most as many bits as both
        // inputs combined; normalization (collapsing `(v, v)` leaves) only
        // shrinks it. No tighter bound is cheap without doing the sum.
        let mut out = IdBuilder::with_capacity(self.bits().len() + other.bits().len());
        let mut frames = Frames::new();
        // Whether the current pair's side is a present child (read the real
        // cursor) or an absent `0` (stand in a synthetic empty).
        let (mut a_on, mut b_on) = (true, true);
        loop {
            let a_node = if a_on { self.peek() } else { IdNode::Empty };
            let b_node = if b_on { other.peek() } else { IdNode::Empty };
            let mut built = match (a_node, b_node) {
                // sum(0, b) = b: copy b (nothing, where b is absent too).
                (IdNode::Empty, _) => {
                    if b_on {
                        out.copy_reader(&mut other)
                    } else {
                        Built::Empty
                    }
                }
                // sum(a, 0) = a: copy a (present here, or the first arm would
                // have matched).
                (_, IdNode::Empty) => out.copy_reader(&mut self),
                // A `1` (full) leaf meets a nonempty subtree: the two ids share
                // a region, so there is no disjoint union.
                (IdNode::Full, _) | (_, IdNode::Full) => return None,
                // Both internal: consume the node headers, emit the node's tag,
                // and descend into its child pairs.
                (
                    IdNode::Internal {
                        left: al,
                        right: ar,
                    },
                    IdNode::Internal {
                        left: bl,
                        right: br,
                    },
                ) => {
                    self.read();
                    other.read();
                    // The tag is final at first sight: an output child is
                    // nonempty exactly when either input child is present (a
                    // disjoint union of nonempty regions is nonempty), so no
                    // slot is reserved or patched. Only the `(1, 1) → 1`
                    // collapse is close-time knowledge.
                    let left = al || bl;
                    let right = ar || br;
                    out.push_tag(left, right);
                    if left && right {
                        frames.push_pending_right(ar, br);
                        (a_on, b_on) = (al, bl);
                    } else {
                        // One child pair is absent on both sides: its sum is
                        // empty, so the node cannot collapse and closes when
                        // its single nonempty pair completes. (Both absent
                        // cannot happen: each internal node has a present
                        // child.)
                        frames.push_pending_close(false);
                        (a_on, b_on) = if left { (al, bl) } else { (ar, br) };
                    }
                    continue;
                }
            };
            // The current pair is summed: unwind closes until a queued right
            // pair resumes the walk, or the root pair is done.
            loop {
                match frames.pop() {
                    None => return Some(out.finish()),
                    Some(Frame::PendingRight { ar, br }) => {
                        frames.push_pending_close(matches!(built, Built::Terminal));
                        (a_on, b_on) = (ar, br);
                        break;
                    }
                    Some(Frame::PendingClose { left_terminal }) => {
                        // Normalize on close: two terminal children collapse to
                        // a single terminal; any other combination is already
                        // final under its tag. A pair the walk entered never
                        // sums to empty, so terminal-or-not is the whole
                        // question.
                        built = if left_terminal && matches!(built, Built::Terminal) {
                            out.collapse_terminal_pair()
                        } else {
                            Built::Node
                        };
                    }
                }
            }
        }
    }
}

/// The per-node control stack of a [`sum`](IdReader::sum) walk.
///
/// Each open output node holds exactly one frame — two or three bits on one bit
/// stack — in one of two shapes:
///
/// - *pending right*: the node's right child pair is still to sum; the
///   frame carries the two input presence bits that thread it.
/// - *pending close*: only the node's close remains; the frame carries
///   whether the node's left output child closed as a terminal, which is
///   all the `(1, 1) → 1` collapse test needs.
///
/// No cursor position and no value is saved anywhere: the consuming readers
/// advance in place, the output tag is written final at descent, and a collapse
/// retracts a fixed-width suffix of the output
/// ([`IdBuilder::collapse_terminal_pair`]).
struct Frames {
    bits: BitsMut,
}

/// One popped [`Frames`] entry; see the stack's two shapes.
enum Frame {
    /// The enclosing node's right child pair is next, threaded by these
    /// presence bits.
    PendingRight { ar: bool, br: bool },
    /// The enclosing node closes with the pair now completing; its left output
    /// child closed as a terminal iff `left_terminal`.
    PendingClose { left_terminal: bool },
}

impl Frames {
    fn new() -> Frames {
        Frames {
            bits: BitsMut::new(),
        }
    }

    /// Queue the right child pair (present sides `ar`/`br`) of the node now
    /// descending into its left pair.
    fn push_pending_right(&mut self, ar: bool, br: bool) {
        self.bits.push(ar);
        self.bits.push(br);
        self.bits.push(true);
    }

    /// Mark the innermost open node as awaiting only its close.
    ///
    /// Pushed when its right pair starts (recording what the left pair built),
    /// and directly at descent where one child pair is absent on both sides
    /// (whose sum is empty and can never make the node collapse, so
    /// `left_terminal` is fixed `false`).
    fn push_pending_close(&mut self, left_terminal: bool) {
        self.bits.push(left_terminal);
        self.bits.push(false);
    }

    /// Pop the innermost frame, or `None` when no node is open (the pair just
    /// completed was the root).
    fn pop(&mut self) -> Option<Frame> {
        let pending_right = self.bits.pop()?;
        if pending_right {
            let br = self
                .bits
                .pop()
                .expect("a pending-right frame is three bits");
            let ar = self
                .bits
                .pop()
                .expect("a pending-right frame is three bits");
            Some(Frame::PendingRight { ar, br })
        } else {
            let left_terminal = self.bits.pop().expect("a pending-close frame is two bits");
            Some(Frame::PendingClose { left_terminal })
        }
    }
}
