//! The disjoint union behind [`Party::join`](crate::Party::join).
//!
//! Two cursors consume the operands together. An unowned child copies the
//! other subtree; two branches descend together; an owned region meeting any
//! owned region rejects the join. The output builder normalizes each branch as
//! it closes.

mod frames;

use crate::codec::BitsBuf;
use crate::party::tree::{PartyCursor, PartyNode};

use super::build::{Builder, Built};
use frames::{Frame, Frames};

impl PartyCursor<'_> {
    /// Join two canonical party subtrees.
    ///
    /// Returns their canonical union, or `None` when they overlap. Each input
    /// subtree is consumed at most once. Unowned sides let the builder copy the
    /// other subtree verbatim; paired branches retain compact state until they
    /// close.
    pub(crate) fn join(mut self, mut other: PartyCursor) -> Option<BitsBuf> {
        // A disjoint union has no more tags than its inputs combined; merging
        // shared ancestors and collapsing terminal pairs can only reduce it.
        let mut out = Builder::with_capacity(self.bits().len() + other.bits().len());
        let mut frames = Frames::new();
        // A present child uses its stored cursor; an absent one is synthesized
        // without advancing that cursor.
        let (mut a_on, mut b_on) = (true, true);
        loop {
            let a_node = if a_on { Some(self.peek()) } else { None };
            let b_node = if b_on { Some(other.peek()) } else { None };
            let mut built = match (a_node, b_node) {
                // No ownership on the first side: retain the second unchanged.
                (None, _) => {
                    if b_on {
                        out.copy_cursor(&mut other)
                    } else {
                        Built::Unowned
                    }
                }
                // No ownership on the second side: retain the first unchanged.
                (_, None) => out.copy_cursor(&mut self),
                // An owned terminal meets a nonempty subtree: the parties share
                // a region, so there is no disjoint union.
                (Some(PartyNode::Owned), _) | (_, Some(PartyNode::Owned)) => return None,
                // Both branches: consume the node headers, emit the node's tag,
                // and descend into its child pairs.
                (Some(PartyNode::Branch(a)), Some(PartyNode::Branch(b))) => {
                    self.read();
                    other.read();
                    // The tag is final at first sight: an output child is
                    // nonempty exactly when either input child is present (a
                    // disjoint union of nonempty regions is nonempty), so no
                    // slot is reserved or patched. Only the collapse of two
                    // terminal children must wait until both children finish.
                    let (al, ar) = a.presence();
                    let (bl, br) = b.presence();
                    let left = a.has_left() || b.has_left();
                    let right = a.has_right() || b.has_right();
                    out.push_tag(left, right);
                    if left && right {
                        frames.push_pending_right(ar, br);
                        (a_on, b_on) = (al, bl);
                    } else {
                        // One child pair is absent on both sides: its join is
                        // empty, so the node cannot collapse and closes when
                        // its single nonempty pair completes. (Both absent
                        // cannot happen: each branch has a present
                        // child.)
                        frames.push_pending_close(false);
                        (a_on, b_on) = if left { (al, bl) } else { (ar, br) };
                    }
                    continue;
                }
            };
            // The current pair is joined: unwind closes until a queued right
            // pair resumes the walk, or the root pair is done.
            loop {
                match frames.pop() {
                    None => return Some(out.finish()),
                    Some(Frame::PendingRight { ar, br }) => {
                        frames.push_pending_close(matches!(built, Built::Owned));
                        (a_on, b_on) = (ar, br);
                        break;
                    }
                    Some(Frame::PendingClose { left_terminal }) => {
                        // Normalize on close: two terminal children collapse to
                        // a single terminal; any other combination is already
                        // final under its tag. A pair the walk entered never
                        // joins to empty, so terminal-or-not is the whole
                        // question.
                        built = if left_terminal && matches!(built, Built::Owned) {
                            out.collapse_terminal_pair()
                        } else {
                            Built::Branch
                        };
                    }
                }
            }
        }
    }
}
