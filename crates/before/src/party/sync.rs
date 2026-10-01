//! Joins two parties and forks their union in one traversal.
//!
//! Both results retain any initial one-child chain. At the first region whose
//! union has both children, the left ownership becomes one result and the right
//! ownership the other. Building those children separately avoids constructing
//! the whole union when their child subtrees can be separated with one scan.

use crate::party::io::writer::PartyWriter;
use crate::party::io::{PartyBranch, PartyReader, PartySubtree};
use crate::party::Party;

impl Party {
    /// Join two parties and fork their union, fusing the walks where possible.
    ///
    /// The result is identical to [`Party::join`] followed by [`Party::fork`].
    /// Overlap is rejected exactly as it is by `join`.
    pub(crate) fn sync(&self, other: &Party) -> Option<(Party, Party)> {
        let mut sync = Sync {
            a: self.reader(),
            b: other.reader(),
            prefix: PartyWriter::new(),
        };
        // Before ownership can divide, the union may follow a chain of
        // one-child branches. Both results need that same prefix, so retain it
        // once and duplicate it only when the union first reaches two children.
        loop {
            let a_start = sync.a.offset();
            let b_start = sync.b.offset();
            let a = sync.a.read().branch()?;
            let b = sync.b.read().branch()?;
            let joined = a.union(b);
            if joined == PartyBranch::Both {
                // This is the first place the union can be split into two
                // nonempty shares. The helper finishes each child without
                // rebuilding the prefix or materializing the whole union.
                return sync.fork_joined_branch(a, b, a_start, b_start);
            }

            sync.prefix.branch(joined);
        }
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
    a: PartyReader<'a>,
    /// Second party at the same spatial position.
    b: PartyReader<'a>,
    /// One-child ancestors shared by both eventual outputs.
    prefix: PartyWriter,
}

impl<'a> Sync<'a> {
    /// Fork the first joined branch whose two children are present.
    fn fork_joined_branch(
        self,
        a_branch: PartyBranch,
        b_branch: PartyBranch,
        a_start: u64,
        b_start: u64,
    ) -> Option<(Party, Party)> {
        let (mut keep_out, mut give_out) = self.prefix.duplicate();
        if a_branch.has_left_child() && b_branch.has_left_child() {
            // Splitting both inputs into child handles would scan each left
            // child, after which joining those children would scan them again.
            // Joining the untouched suffix first and forking that result keeps
            // every source subtree to one traversal.
            let joined = self
                .a
                .restart_at(a_start)
                .join(self.b.restart_at(b_start))?;
            let (keep, give) = Party::fork_tree(joined.reader());
            keep_out.copy_remaining_subtree(keep.reader());
            give_out.copy_remaining_subtree(give.reader());
            return Some((keep_out.finish(), give_out.finish()));
        }

        let a = ChildSubtrees::read(self.a, a_branch);
        let b = ChildSubtrees::read(self.b, b_branch);
        let keep = JoinedChild::new(a.left, b.left)?;
        let give = JoinedChild::new(a.right, b.right)?;
        keep_out.branch(PartyBranch::Left);
        keep.append_to(&mut keep_out);
        give_out.branch(PartyBranch::Right);
        give.append_to(&mut give_out);
        Some((keep_out.finish(), give_out.finish()))
    }
}

/// The stored children of one consumed branch.
struct ChildSubtrees<'a> {
    /// Left child, if present.
    left: Option<PartySubtree<'a>>,
    /// Right child, if present.
    right: Option<PartySubtree<'a>>,
}

impl<'a> ChildSubtrees<'a> {
    /// Locate the children after their branch tag has been consumed.
    ///
    /// The branch is a suffix of its operand, so only a two-child branch needs
    /// a scan: it skips the left child to find the right child's start.
    fn read(mut cursor: PartyReader<'a>, branch: PartyBranch) -> Self {
        match branch {
            PartyBranch::Both => {
                let (_, left) = cursor.take_subtree();
                Self {
                    left: Some(left),
                    right: Some(cursor.remainder()),
                }
            }
            PartyBranch::Left => Self {
                left: Some(cursor.remainder()),
                right: None,
            },
            PartyBranch::Right => Self {
                left: None,
                right: Some(cursor.remainder()),
            },
        }
    }
}

/// One child of the joined branch.
enum JoinedChild<'a> {
    /// Exactly one operand owns the child, so its subtree can be copied.
    Borrowed(PartySubtree<'a>),
    /// Both operands contribute disjoint ownership, so the child was joined.
    Joined(Party),
}

impl<'a> JoinedChild<'a> {
    /// Join the two optional source children.
    fn new(a: Option<PartySubtree<'a>>, b: Option<PartySubtree<'a>>) -> Option<Self> {
        match (a, b) {
            (Some(subtree), None) | (None, Some(subtree)) => Some(Self::Borrowed(subtree)),
            (Some(a), Some(b)) => a.reader().join(b.reader()).map(Self::Joined),
            (None, None) => unreachable!("a joined branch child is present in some operand"),
        }
    }

    /// Append this complete child subtree to a party under construction.
    fn append_to(self, out: &mut PartyWriter) {
        match self {
            JoinedChild::Borrowed(subtree) => out.copy_subtree(subtree),
            JoinedChild::Joined(party) => out.copy_remaining_subtree(party.reader()),
        }
    }
}
