//! The disjoint union behind [`Party::join`](crate::Party::join).
//!
//! Two cursors consume the operands together. An unowned child copies the
//! other subtree; two branches descend together; an owned region meeting any
//! owned region rejects the join. The output builder normalizes each branch as
//! it closes.

mod frames;

use crate::party::io::writer::{PartyWriter, RegionKind};
use crate::party::io::{PartyBranch, PartyNode, PartyReader};
use crate::party::Party;
use frames::{Frame, Frames};

impl PartyReader<'_> {
    /// Join two canonical party subtrees.
    ///
    /// Returns their canonical union, or `None` when they overlap. Each input
    /// subtree is consumed at most once. Unowned sides let the builder copy the
    /// other subtree verbatim; paired branches retain compact state until they
    /// close.
    pub fn join(mut self, mut other: PartyReader) -> Option<Party> {
        // A disjoint union has no more tags than its inputs combined; merging
        // shared ancestors and collapsing owned pairs can only reduce it.
        let mut out = PartyWriter::for_join(&self, &other);
        let mut frames = Frames::new();
        // A present child uses its stored cursor; an absent one is synthesized
        // without advancing that cursor.
        let (mut a_present, mut b_present) = (true, true);
        loop {
            let a_node = a_present.then(|| self.peek());
            let b_node = b_present.then(|| other.peek());
            let mut built = match (a_node, b_node) {
                // No ownership on the first side: retain the second unchanged.
                (None, _) => {
                    if b_present {
                        out.copy_next_subtree(&mut other)
                    } else {
                        RegionKind::Unowned
                    }
                }
                // No ownership on the second side: retain the first unchanged.
                (_, None) => out.copy_next_subtree(&mut self),
                // An owned region meets a nonempty subtree: the parties share
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
                    // owned children must wait until both children finish.
                    let a_left = a.has_left_child();
                    let a_right = a.has_right_child();
                    let b_left = b.has_left_child();
                    let b_right = b.has_right_child();
                    let joined = a.union(b);
                    out.branch(joined);
                    if joined == PartyBranch::Both {
                        frames.push_pending_right(a_right, b_right);
                        (a_present, b_present) = (a_left, b_left);
                    } else {
                        // One child pair is absent on both sides: its join is
                        // empty, so the node cannot collapse and closes when
                        // its single nonempty pair completes. (Both absent
                        // cannot happen: each branch has a present
                        // child.)
                        frames.push_pending_close(false);
                        (a_present, b_present) = if joined.has_left_child() {
                            (a_left, b_left)
                        } else {
                            (a_right, b_right)
                        };
                    }
                    continue;
                }
            };
            // The current pair is joined: unwind closes until a queued right
            // pair resumes the walk, or the root pair is done.
            loop {
                match frames.pop() {
                    None => return Some(out.finish()),
                    Some(Frame::PendingRight {
                        a_present: next_a,
                        b_present: next_b,
                    }) => {
                        frames.push_pending_close(matches!(built, RegionKind::Owned));
                        (a_present, b_present) = (next_a, next_b);
                        break;
                    }
                    Some(Frame::PendingClose { left_owned }) => {
                        // Normalize on close: two owned children collapse to
                        // one owned region; any other combination is already
                        // final under its tag. A pair the walk entered never
                        // joins to empty, so owned-or-not is the whole
                        // question.
                        built = if left_owned && matches!(built, RegionKind::Owned) {
                            out.collapse_owned_children()
                        } else {
                            RegionKind::Branch
                        };
                    }
                }
            }
        }
    }
}
