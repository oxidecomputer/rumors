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
//! they must apply the two collapses above as the node closes. [`Builder`]
//! provides that local open/close operation. [`RegionBuilder`] adds the
//! traversal state needed when the input describes consecutive regions by
//! depth rather than by explicit tree nodes.
//!
//! Both builders retain only a few bits per open ancestor. In particular, they
//! do not allocate a machine-word frame at every level: each input ancestor
//! may cost only two bits, so full frames would greatly amplify memory use.

mod positions;

use crate::codec::{BitBuilder, BitStack, BitsBuf, BitsView};
use crate::party::tree::{PartyCursor, PartyNode};

pub(super) use positions::Positions;

#[cfg(test)]
mod tests;

/// Writes one canonical party subtree into a single bit buffer.
///
/// Opening a node reserves its tag. Its children are then written in preorder,
/// and [`close_node`](Self::close_node) patches the tag once their results are
/// known. Because the node and all of its descendants form the suffix written
/// since [`open`](Self::open), either canonical collapse is a truncation; no
/// earlier output needs to move.
pub(super) struct Builder {
    /// The output prefix, including reserved tags for unfinished ancestors.
    out: BitBuilder,
}

/// The canonical result of building a region.
///
/// This summary tells a parent whether its child is present and whether two
/// children can collapse. The actual bits already occupy their final preorder
/// positions, so the summary needs no offset or subtree data.
#[derive(Clone, Copy)]
pub(super) enum Built {
    /// An unowned region, for which no bits were emitted.
    Unowned,
    /// One owned terminal.
    Owned,
    /// A branch subtree.
    Branch,
}

/// A reserved node tag awaiting its two children.
///
/// The non-cloneable token makes one close consume one open. Builders keep
/// these tokens in preorder nesting order and close the innermost one first.
/// `#[must_use]` also warns if an open is accidentally ignored.
#[must_use = "an opened node must be closed with close_node"]
pub(super) struct Open(u64);

/// The width of a party node's presence tag: one bit per child.
const TAG_BITS: usize = 2;

/// The output width of a node whose two children are both terminals: its own
/// tag followed by the two terminal tags.
const TERMINAL_PAIR_BITS: u64 = 3 * TAG_BITS as u64;

impl Builder {
    /// Create an empty builder with room for `capacity` output bits.
    ///
    /// The capacity is only an allocation hint; normalization may make the
    /// final party shorter.
    pub(super) fn with_capacity(capacity: u64) -> Self {
        Builder {
            out: BitBuilder::with_capacity(capacity),
        }
    }

    /// Append an owned terminal: the tag `00`, with no children.
    pub(super) fn terminal(&mut self) -> Built {
        self.push_tag(false, false);
        Built::Owned
    }

    /// Append a node's two-bit presence tag when both bits are already known.
    ///
    /// This bypasses the open/close protocol because no later normalization or
    /// patch is needed.
    pub(super) fn push_tag(&mut self, left: bool, right: bool) {
        self.out.push_bit(left);
        self.out.push_bit(right);
    }

    /// Reserve a node's tag before writing its left and right children.
    ///
    /// Pass the returned token to [`close_node`](Self::close_node) after both
    /// children have been written. Opens and closes must be properly nested.
    pub(super) fn open(&mut self) -> Open {
        Open(self.out.reserve(TAG_BITS))
    }

    /// Copy one canonical source subtree into the output, advancing `src`
    /// past it and reporting what it was.
    ///
    /// A scan locates the subtree's end, then its bits are copied unchanged.
    pub(super) fn copy_cursor(&mut self, src: &mut PartyCursor) -> Built {
        let is_terminal = matches!(src.peek(), PartyNode::Owned);
        let start = src.offset();
        src.skip();
        self.out.splice(src.bits(), start, src.offset());
        if is_terminal {
            Built::Owned
        } else {
            Built::Branch
        }
    }

    /// Copy the canonical subtree in `src[start..end]` unchanged. The caller
    /// supplies its kind when closing the parent with [`close_node`](Self::close_node).
    pub(super) fn splice(&mut self, src: BitsView<'_>, start: u64, end: u64) {
        self.out.splice(src, start, end);
    }

    /// Normalize and close the node opened at `node` from what its two children
    /// turned out to be, consuming the open token:
    ///
    /// - two empty children collapse to an empty region;
    /// - two terminal children collapse to one terminal;
    /// - otherwise patch the tag to record which children are present.
    pub(super) fn close_node(&mut self, node: Open, left: Built, right: Built) -> Built {
        let node = node.0;
        match (left, right) {
            (Built::Unowned, Built::Unowned) => {
                self.out.truncate(node);
                Built::Unowned
            }
            (Built::Owned, Built::Owned) => {
                self.out.truncate(node);
                self.terminal()
            }
            (left, right) => {
                // The first reserved bit describes the left child; the second
                // describes the right child.
                self.out.patch_bit(node, !matches!(left, Built::Unowned));
                self.out
                    .patch_bit(node + 1, !matches!(right, Built::Unowned));
                Built::Branch
            }
        }
    }

    /// Collapse the just-written node with two terminal children.
    ///
    /// Its tag and both children are the last three tags in the output. They can
    /// therefore be truncated and replaced by one terminal without retaining
    /// the parent's position.
    pub(super) fn collapse_terminal_pair(&mut self) -> Built {
        self.out.truncate(self.out.len() - TERMINAL_PAIR_BITS);
        self.terminal()
    }

    /// Finish writing and return the canonical bit stream.
    pub(super) fn finish(self) -> BitsBuf {
        self.out.finish()
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
/// [`Builder`] applies canonical collapses as ancestors close.
///
/// The traversal path and child summaries use bit stacks. Reserved output
/// positions use [`Positions`], which compresses the adjacent positions created
/// by an uninterrupted descent. Thus even a very deep description retains
/// only bit-proportional state rather than one machine-word frame per level.
pub(super) struct RegionBuilder {
    /// Output tags and canonical normalization.
    out: Builder,
    /// The open traversal frontier, root first: `false` awaits or occupies a
    /// left child, while `true` occupies a right child.
    path: BitStack,
    /// The result of each completed left child whose right sibling is next.
    left_kinds: BitStack,
    /// The open ancestors' reserved tag positions, innermost last.
    tags: Positions,
    /// The root result, set when the final input closes the traversal.
    root: Option<Built>,
}

impl RegionBuilder {
    /// Create a builder with room for `capacity` output bits.
    pub(super) fn with_capacity(capacity: u64) -> Self {
        RegionBuilder {
            out: Builder::with_capacity(capacity),
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
    pub(super) fn leaf(&mut self, depth: u64, owned: bool) {
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
            self.tags.push(self.out.open());
            self.path.push(false);
        }
        let kind = if owned {
            self.out.terminal()
        } else {
            Built::Unowned
        };
        self.close_up(kind);
    }

    /// Append one already-canonical branch subtree rooted at `depth`.
    ///
    /// `src[start..end]` must be one complete branch subtree in canonical
    /// form. Its interior therefore needs no rebuilding. Only its ancestors in
    /// the output can collapse, so the range is spliced verbatim and reported
    /// upward as [`Built::Branch`]. Use [`leaf`](Self::leaf) for a terminal or an
    /// unowned region.
    pub(super) fn subtree(&mut self, depth: u64, src: BitsView<'_>, start: u64, end: u64) {
        debug_assert!(
            self.root.is_none(),
            "a subtree arrived after the root was complete"
        );
        debug_assert!(
            depth >= self.path.len(),
            "a subtree starts above the remaining traversal frontier"
        );
        debug_assert!(
            src.bit(start) || src.bit(start + 1),
            "a spliced block is a branch subtree, never a lone terminal"
        );
        // Enter the subtree's position exactly as a leaf at this depth would.
        for _ in self.path.len()..depth {
            self.tags.push(self.out.open());
            self.path.push(false);
        }
        self.out.splice(src, start, end);
        self.close_up(Built::Branch);
    }

    /// Take the finished canonical stream.
    ///
    /// A sequence containing no owned region produces an empty stream.
    pub(super) fn finish(self) -> BitsBuf {
        debug_assert!(
            self.root.is_some(),
            "the input regions close the root exactly once"
        );
        self.out.finish()
    }

    /// Advance the traversal after completing one child with result `kind`.
    ///
    /// Completing a left child records its result and leaves the traversal at
    /// the right sibling, where the next input begins. Completing a right child
    /// closes and normalizes its parent; that parent is now the completed child
    /// of the next ancestor, so the same step repeats. Reaching above the root
    /// means the input is complete.
    fn close_up(&mut self, mut kind: Built) {
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
                    kind = self.out.close_node(self.tags.pop(), left, kind);
                }
            }
        }
    }

    /// Record a completed left sibling's kind: two bits, is-branch then
    /// is-terminal (`Unowned` is neither).
    fn push_kind(&mut self, kind: Built) {
        self.left_kinds.push(matches!(kind, Built::Branch));
        self.left_kinds.push(matches!(kind, Built::Owned));
    }

    /// Pop the innermost recorded kind.
    fn pop_kind(&mut self) -> Built {
        let terminal = self.left_kinds.pop().expect("kind entries are two bits");
        let node = self.left_kinds.pop().expect("kind entries are two bits");
        match (node, terminal) {
            (false, false) => Built::Unowned,
            (false, true) => Built::Owned,
            (true, false) => Built::Branch,
            (true, true) => unreachable!("a kind is one of three values"),
        }
    }
}
