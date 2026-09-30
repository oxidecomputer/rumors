//! Joins two parties and forks the result for [`Clock::sync`](crate::Clock::sync).
//!
//! Both results retain any initial one-child chain. At the first region whose
//! union has both children, the left ownership becomes one result and the right
//! ownership the other. Building those children separately avoids constructing
//! the whole union when their source ranges can be located without rescanning.

use core::ops::Range;

use crate::codec::{built_view, extend_from_view, BitsBuf, BitsView};
use crate::party::tree::{PartyBranch, PartyCursor, PartyNode};
use crate::party::Party;

impl Party {
    /// Join two parties and fork their union, fusing the walks where possible.
    ///
    /// The result is byte-for-byte identical to [`Party::join`] followed by
    /// [`Party::fork`]. Overlap is rejected exactly as it is by `join`.
    pub(crate) fn sync(&self, other: &Party) -> Option<(Party, Party)> {
        let (keep, give) = Sync::new(self.as_bits(), other.as_bits()).finish()?;
        Some((Party::from_bits(keep), Party::from_bits(give)))
    }
}

/// One fused join-then-fork walk.
///
/// The walk emits the one-child prefix shared by both results. At the first
/// joined branch, it sends one child to each result. A child present in only
/// one input is copied; a child present in both is joined. If finding the branch
/// boundary would rescan a shared subtree, the untouched suffix is joined and
/// forked instead. This keeps input traversal linear; the fallback also makes
/// one linear pass over the joined suffix.
struct Sync<'a> {
    /// First party, positioned after the shared one-child prefix.
    a: PartyCursor<'a>,
    /// Second party at the same spatial position.
    b: PartyCursor<'a>,
    /// One-child ancestors that both results retain.
    prefix: BitsBuf,
}

impl<'a> Sync<'a> {
    /// Start at two complete party trees.
    fn new(a: BitsView<'a>, b: BitsView<'a>) -> Self {
        Self {
            a: PartyCursor::root(a),
            b: PartyCursor::root(b),
            prefix: BitsBuf::new(),
        }
    }

    /// Produce the synchronized parties, or reject overlapping ownership.
    fn finish(mut self) -> Option<(BitsBuf, BitsBuf)> {
        loop {
            let a = Self::branch(self.a.peek())?;
            let b = Self::branch(self.b.peek())?;
            let left = a.has_left() || b.has_left();
            let right = a.has_right() || b.has_right();
            if left && right {
                return self.fork_joined_branch(a, b);
            }

            self.a.read();
            self.b.read();
            self.prefix.push(left);
            self.prefix.push(right);
        }
    }

    /// Return a branch's child-presence bits, rejecting an owned overlap.
    fn branch(node: PartyNode) -> Option<PartyBranch> {
        match node {
            PartyNode::Owned => None,
            PartyNode::Branch(branch) => Some(branch),
        }
    }

    /// Fork the first joined branch whose two children are present.
    fn fork_joined_branch(
        mut self,
        a_branch: PartyBranch,
        b_branch: PartyBranch,
    ) -> Option<(BitsBuf, BitsBuf)> {
        if a_branch.has_left() && b_branch.has_left() {
            // Locating both right children would scan the shared left child
            // before joining it. Joining the untouched suffix avoids that
            // second traversal.
            let prefix = self.prefix;
            let joined = self.a.join(self.b)?;
            let (keep, give) = Party::fork_tree(built_view(&joined));
            return Some((Self::prepend(&prefix, &keep), Self::prepend(&prefix, &give)));
        }

        self.a.read();
        self.b.read();
        let a = ChildRanges::read(&self.a, a_branch);
        let b = ChildRanges::read(&self.b, b_branch);
        let keep = JoinedChild::new(self.a.bits(), self.b.bits(), a.left, b.left)?;
        let give = JoinedChild::new(self.a.bits(), self.b.bits(), a.right, b.right)?;
        Some((
            keep.into_half(&self.prefix, false),
            give.into_half(&self.prefix, true),
        ))
    }

    /// Prepend the already-emitted common prefix to a completed suffix.
    fn prepend(prefix: &BitsBuf, suffix: &BitsBuf) -> BitsBuf {
        let mut out = BitsBuf::with_capacity(prefix.len() + suffix.len());
        out.extend_from_buf(prefix);
        out.extend_from_buf(suffix);
        out
    }
}

/// The bit ranges of a branch's children.
struct ChildRanges {
    /// Stored range of the left child, if present.
    left: Option<Range<u64>>,
    /// Stored range of the right child, if present.
    right: Option<Range<u64>>,
}

impl ChildRanges {
    /// Locate the children after their branch tag has been consumed.
    ///
    /// The branch is a suffix of its operand, so only a two-child branch needs
    /// a scan: it skips the left child to find the right child's start.
    fn read(cursor: &PartyCursor<'_>, branch: PartyBranch) -> Self {
        let bits = cursor.bits();
        let start = cursor.offset();
        match branch {
            PartyBranch::Both => {
                let mut probe = PartyCursor::at(bits, start);
                probe.skip();
                let middle = probe.offset();
                Self {
                    left: Some(start..middle),
                    right: Some(middle..bits.len()),
                }
            }
            PartyBranch::Left => Self {
                left: Some(start..bits.len()),
                right: None,
            },
            PartyBranch::Right => Self {
                left: None,
                right: Some(start..bits.len()),
            },
        }
    }
}

/// One child of the joined branch.
enum JoinedChild<'a> {
    /// Exactly one operand owns the child, so its subtree can be copied.
    Borrowed(BitsView<'a>, Range<u64>),
    /// Both operands contribute disjoint ownership, so the child was joined.
    Built(BitsBuf),
}

impl<'a> JoinedChild<'a> {
    /// Join the two optional source children.
    fn new(
        a_bits: BitsView<'a>,
        b_bits: BitsView<'a>,
        a: Option<Range<u64>>,
        b: Option<Range<u64>>,
    ) -> Option<Self> {
        match (a, b) {
            (Some(range), None) => Some(Self::Borrowed(a_bits, range)),
            (None, Some(range)) => Some(Self::Borrowed(b_bits, range)),
            (Some(a), Some(b)) => PartyCursor::at(a_bits, a.start)
                .join(PartyCursor::at(b_bits, b.start))
                .map(Self::Built),
            (None, None) => unreachable!("a joined branch child is present in some operand"),
        }
    }

    /// Append this child after the common prefix and its one-child branch tag.
    fn into_half(self, prefix: &BitsBuf, right: bool) -> BitsBuf {
        let mut out = BitsBuf::with_capacity(prefix.len() + 2 + self.len());
        out.extend_from_buf(prefix);
        out.push(!right);
        out.push(right);
        match self {
            JoinedChild::Borrowed(bits, range) => {
                extend_from_view(&mut out, bits, range.start, range.end)
            }
            JoinedChild::Built(bits) => out.extend_from_buf(&bits),
        }
        out
    }

    /// This child's bit length.
    fn len(&self) -> u64 {
        match self {
            JoinedChild::Borrowed(_, range) => range.end - range.start,
            JoinedChild::Built(bits) => bits.len(),
        }
    }
}
