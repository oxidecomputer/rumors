//! Builds canonical parties without an intermediate tree.
//!
//! A party is a preorder stream of two-bit node tags. The first tag bit
//! says whether the left child is present and the second says the same for the
//! right child. `00` is an owned terminal; an unowned region emits no bits at
//! all. Canonical form also replaces two empty children with an empty region
//! and two terminal children with one terminal.
//!
//! Operations in this crate often discover an output from left to right. They
//! cannot know a node's final tag until both children have been processed, and
//! they must apply the two collapses above as the node closes. [`PartyWriter`]
//! provides that local open/close operation. [`PartyRegionWriter`] adds the
//! traversal state needed when the input describes consecutive regions by
//! depth rather than by explicit tree nodes.
//!
//! Both builders retain only a few bits per open ancestor. In particular, they
//! do not allocate a machine-word frame at every level: each input ancestor
//! may cost only two bits, so full frames would greatly amplify memory use.

mod positions;

use crate::bits::stack::BitStack;
use crate::bits::BitsWriter;
use crate::party::io::{PartyBranch, PartyNode, PartyPath, PartyReader, PartySubtree};
use crate::party::Party;

pub use positions::Positions;

#[cfg(test)]
mod tests;

/// Constructs one canonical party from ownership-tree operations.
///
/// [`branch`](Self::branch) begins a branch whose shape is already known.
/// [`open_branch`](Self::open_branch) instead reserves a branch whose children
/// may collapse while they are built; [`close_branch`](Self::close_branch)
/// normalizes it once both children are known. A branch and its descendants
/// are always the newest output, so either collapse only truncates that suffix.
pub struct PartyWriter {
    /// The output prefix, including reserved tags for unfinished ancestors.
    out: BitsWriter,
}

/// The canonical result of building a region.
///
/// This summary tells a parent whether its child is present and whether two
/// children can collapse. The actual bits already occupy their final preorder
/// positions, so the summary needs no offset or subtree data.
#[derive(Clone, Copy)]
pub enum RegionKind {
    /// An unowned region, for which no bits were emitted.
    Unowned,
    /// One owned terminal.
    Owned,
    /// A branch subtree.
    Branch,
}

/// A branch reserved until its two children are known.
///
/// The non-cloneable token makes one close consume one open. Builders keep
/// these tokens in preorder nesting order and close the innermost one first.
/// `#[must_use]` also warns if an open is accidentally ignored.
#[must_use = "an opened branch must be closed with close_branch"]
pub struct OpenBranch(u64);

/// The width of a party node's presence tag: one bit per child.
const TAG_BITS: usize = 2;

/// The output width of a node whose two children are both terminals: its own
/// tag followed by the two terminal tags.
const TERMINAL_PAIR_BITS: u64 = 3 * TAG_BITS as u64;

impl PartyWriter {
    /// Create an empty party builder without reserving storage.
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    /// Create an output large enough for the disjoint union's upper bound.
    pub fn for_join(a: &PartyReader<'_>, b: &PartyReader<'_>) -> Self {
        Self::with_capacity(a.stored_len() + b.stored_len())
    }

    /// Create an output expected to resemble `example` in size.
    pub fn sized_like(example: &Party) -> Self {
        Self::with_capacity(example.reader().stored_len())
    }

    /// Turn one unfinished common prefix into two independent outputs.
    ///
    /// Sync delays this copy until it knows the join succeeds. A rejected sync
    /// therefore builds the common prefix only once.
    pub fn duplicate(self) -> (Self, Self) {
        let copy = PartyWriter {
            out: self.out.clone(),
        };
        (self, copy)
    }

    /// Create an empty builder with room for `capacity` output bits.
    ///
    /// The capacity is only an allocation hint; normalization may make the
    /// final party shorter.
    pub fn with_capacity(capacity: u64) -> Self {
        PartyWriter {
            out: BitsWriter::with_capacity(capacity),
        }
    }

    /// Append an owned terminal: the tag `00`, with no children.
    pub fn owned(&mut self) -> RegionKind {
        self.out.push(false);
        self.out.push(false);
        RegionKind::Owned
    }

    /// Begin a branch whose children are already known.
    ///
    /// The caller writes those children next, in left-to-right order. Use
    /// [`open_branch`](Self::open_branch) instead when construction may leave a
    /// child unowned or collapse two owned children.
    pub fn branch(&mut self, branch: PartyBranch) -> RegionKind {
        match branch {
            PartyBranch::Left => self.out.push_bits(0b10, 2),
            PartyBranch::Right => self.out.push_bits(0b01, 2),
            PartyBranch::Both => self.out.push_bits(0b11, 2),
        }
        RegionKind::Branch
    }

    /// Reserve a branch before writing its left and right children.
    ///
    /// Pass the returned token to [`close_branch`](Self::close_branch) after both
    /// children have been written. Opens and closes must be properly nested.
    pub fn open_branch(&mut self) -> OpenBranch {
        OpenBranch(self.out.reserve(TAG_BITS))
    }

    /// Copy one canonical source subtree into the output, advancing `src`
    /// past it and reporting what it was.
    ///
    /// A scan locates the subtree's end, then its bits are copied unchanged.
    pub fn copy_next_subtree(&mut self, src: &mut PartyReader) -> RegionKind {
        let (node, subtree) = src.take_subtree();
        self.copy_subtree(subtree);
        match node {
            PartyNode::Owned => RegionKind::Owned,
            PartyNode::Branch(_) => RegionKind::Branch,
        }
    }

    /// Copy a canonical branch already delimited in `src`.
    ///
    /// The caller has located the complete subtree and therefore need not scan
    /// it again merely to copy it.
    pub fn copy_branch(&mut self, subtree: PartySubtree<'_>) -> RegionKind {
        debug_assert!(subtree.is_branch());
        self.copy_subtree(subtree);
        RegionKind::Branch
    }

    /// Copy one complete canonical subtree whose bounds are already known.
    pub fn copy_subtree(&mut self, subtree: PartySubtree<'_>) {
        let (src, start, end) = subtree.storage();
        self.out.splice(src, start, end);
    }

    /// Copy a chain of one-child branches unchanged.
    pub fn copy_unary_path(&mut self, path: PartyPath<'_>) {
        let (src, start, end) = path.storage();
        self.out.splice(src, start, end);
    }

    /// Copy the same one-child path into both results of a fork.
    pub fn copy_shared_unary_path(
        path: PartyPath<'_>,
        first: &mut PartyWriter,
        second: &mut PartyWriter,
    ) {
        let (src, start, end) = path.storage();
        first.out.splice(src, start, end);
        second.out.splice(src, start, end);
    }

    /// Copy the complete subtree occupying the unread suffix of `src`.
    ///
    /// Some walks know structurally that no later node follows this subtree.
    /// They can copy it without scanning solely to rediscover the stream's end.
    pub fn copy_remaining_subtree(&mut self, src: PartyReader<'_>) {
        self.copy_subtree(src.remainder());
    }

    /// Normalize a reserved branch once both child results are known.
    ///
    /// - two empty children collapse to an empty region;
    /// - two owned children collapse to one owned region;
    /// - otherwise patch the tag to record which children are present.
    pub fn close_branch(
        &mut self,
        node: OpenBranch,
        left: RegionKind,
        right: RegionKind,
    ) -> RegionKind {
        let node = node.0;
        match (left, right) {
            (RegionKind::Unowned, RegionKind::Unowned) => {
                self.out.truncate(node);
                RegionKind::Unowned
            }
            (RegionKind::Owned, RegionKind::Owned) => {
                self.out.truncate(node);
                self.owned()
            }
            (left, right) => {
                // The first reserved bit describes the left child; the second
                // describes the right child.
                self.out
                    .patch_bit(node, !matches!(left, RegionKind::Unowned));
                self.out
                    .patch_bit(node + 1, !matches!(right, RegionKind::Unowned));
                RegionKind::Branch
            }
        }
    }

    /// Collapse the just-written branch with two owned children.
    ///
    /// The branch and both children are the output suffix, so they can be
    /// replaced by one owned region without retaining the branch's position.
    pub fn collapse_owned_children(&mut self) -> RegionKind {
        self.out.truncate(self.out.len() - TERMINAL_PAIR_BITS);
        self.owned()
    }

    /// Finish one nonempty canonical party.
    pub fn finish(self) -> Party {
        let bits = self.out;
        assert!(!bits.is_empty(), "a finished Party owns a region");
        super::finish(bits)
    }
}

/// Builds a party from consecutive dyadic regions.
///
/// Each input gives a depth and says whether that region is owned. A region at
/// depth `d` has width `2^-d`; the inputs proceed from left to right and exactly
/// cover the root. Those depths determine when the traversal descends into a
/// left child, crosses to its right sibling, and finishes an ancestor.
///
/// The builder opens nodes while descending to the next depth. After emitting
/// the region, [`close_up`](Self::close_up) either crosses from a completed left
/// child to its right sibling or closes completed ancestors until another
/// sibling remains. Unowned regions emit no bits. Already-canonical branch
/// subtrees can be spliced as one region with [`subtree`](Self::subtree).
/// [`PartyWriter`] applies canonical collapses as ancestors close.
///
/// The traversal path and child summaries use bit stacks. Reserved output
/// positions use [`Positions`], which compresses the adjacent positions created
/// by an uninterrupted descent. Thus even a very deep description retains
/// only bit-proportional state rather than one machine-word frame per level.
pub struct PartyRegionWriter {
    /// Output tags and canonical normalization.
    out: PartyWriter,
    /// The open traversal frontier, root first: `false` awaits or occupies a
    /// left child, while `true` occupies a right child.
    path: BitStack,
    /// The result of each completed left child whose right sibling is next.
    left_kinds: BitStack,
    /// The open ancestors' reserved tag positions, innermost last.
    tags: Positions,
    /// The root result, set when the final input closes the traversal.
    root: Option<RegionKind>,
}

impl PartyRegionWriter {
    /// Create the output for a difference of two parties.
    pub fn for_difference(a: &PartyReader<'_>, b: &PartyReader<'_>) -> Self {
        Self::with_capacity(a.stored_len() + b.stored_len())
    }

    /// Create a builder with room for `capacity` output bits.
    pub fn with_capacity(capacity: u64) -> Self {
        PartyRegionWriter {
            out: PartyWriter::with_capacity(capacity),
            path: BitStack::new(),
            left_kinds: BitStack::new(),
            tags: Positions::new(),
            root: None,
        }
    }

    /// Append the next region, at `depth`, as owned or unowned.
    ///
    /// Inputs must proceed left to right and cover the root without overlap or
    /// gaps. After prior inputs have closed as far as possible, `depth` may be
    /// no shallower than the remaining traversal frontier.
    pub fn leaf(&mut self, depth: u64, owned: bool) {
        debug_assert!(
            self.root.is_none(),
            "a region arrived after the root was complete"
        );
        debug_assert!(
            depth >= self.path.len(),
            "a region starts above the remaining traversal frontier"
        );
        // New levels begin in their left child. Their tags remain open until
        // both children have been summarized.
        for _ in self.path.len()..depth {
            self.tags.push(self.out.open_branch());
            self.path.push(false);
        }
        let kind = if owned {
            self.out.owned()
        } else {
            RegionKind::Unowned
        };
        self.close_up(kind);
    }

    /// Append one already-canonical branch subtree rooted at `depth`.
    ///
    /// Its interior is already canonical, so only its new ancestors can
    /// collapse. Use [`leaf`](Self::leaf) for an owned or unowned uniform
    /// region.
    pub fn subtree(&mut self, depth: u64, subtree: PartySubtree<'_>) {
        debug_assert!(
            self.root.is_none(),
            "a subtree arrived after the root was complete"
        );
        debug_assert!(
            depth >= self.path.len(),
            "a subtree starts above the remaining traversal frontier"
        );
        debug_assert!(subtree.is_branch());
        // Enter the subtree's position exactly as a leaf at this depth would.
        for _ in self.path.len()..depth {
            self.tags.push(self.out.open_branch());
            self.path.push(false);
        }
        let kind = self.out.copy_branch(subtree);
        self.close_up(kind);
    }

    /// Take the finished canonical party, or `None` when no region is owned.
    pub fn finish(self) -> Option<Party> {
        debug_assert!(
            self.root.is_some(),
            "the input regions close the root exactly once"
        );
        match self.root {
            Some(RegionKind::Unowned) => None,
            Some(_) => Some(self.out.finish()),
            None => unreachable!("the region traversal completes the root"),
        }
    }

    /// Advance the traversal after completing one child with result `kind`.
    ///
    /// Completing a left child records its result and leaves the traversal at
    /// the right sibling, where the next input begins. Completing a right child
    /// closes and normalizes its parent; that parent is now the completed child
    /// of the next ancestor, so the same step repeats. Reaching above the root
    /// means the input is complete.
    fn close_up(&mut self, mut kind: RegionKind) {
        loop {
            match self.path.pop() {
                None => {
                    self.root = Some(kind);
                    return;
                }
                Some(false) => {
                    // The next input begins in this node's right child.
                    self.path.push(true);
                    self.push_kind(kind);
                    return;
                }
                Some(true) => {
                    // Both children are now known. The normalized parent is
                    // the child result propagated to the next level.
                    let left = self.pop_kind();
                    kind = self.out.close_branch(self.tags.pop(), left, kind);
                }
            }
        }
    }

    /// Record a completed left sibling's kind: two bits, is-branch then
    /// is-terminal (`Unowned` is neither).
    fn push_kind(&mut self, kind: RegionKind) {
        self.left_kinds.push(matches!(kind, RegionKind::Branch));
        self.left_kinds.push(matches!(kind, RegionKind::Owned));
    }

    /// Pop the innermost recorded kind.
    fn pop_kind(&mut self) -> RegionKind {
        let terminal = self.left_kinds.pop().expect("kind entries are two bits");
        let node = self.left_kinds.pop().expect("kind entries are two bits");
        match (node, terminal) {
            (false, false) => RegionKind::Unowned,
            (false, true) => RegionKind::Owned,
            (true, false) => RegionKind::Branch,
            (true, true) => unreachable!("a kind is one of three values"),
        }
    }
}
