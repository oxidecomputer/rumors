//! Forward traversal of a party's canonical tree.
//!
//! Each stored node begins with two bits saying whether its left and right
//! children are present. Present children follow in preorder. A `00` tag is an
//! owned terminal; an unowned region is omitted entirely. Thus an absent child
//! is represented by no bits, and an entirely unowned tree is empty.
//!
//! Canonical trees contain no branch with two owned terminals, because that
//! branch denotes one owned terminal. Consequently every stored subtree is
//! nonempty and canonical. Ownership of a whole region can be decided from its
//! root tag without scanning its descendants.
//!
//! [`PartyCursor`] consumes one such tree from left to right. It cannot be
//! cloned, so retaining an old traversal position requires an explicit new
//! cursor rather than an accidental copy. Algorithms that align two parties
//! represent an absent child as `None`; it is not a stored node.

use crate::codec::BitsView;

/// One stored node in a canonical party.
#[derive(Clone, Copy)]
pub(crate) enum PartyNode {
    /// A terminal region the party owns completely.
    Owned,
    /// A subdivided region, with at least one present child.
    Branch(PartyBranch),
}

/// The children present beneath a stored branch.
///
/// The three variants are exactly the valid nonempty presence tags. The `00`
/// tag is [`PartyNode::Owned`], not a branch.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum PartyBranch {
    /// Only the left child is present.
    Left,
    /// Only the right child is present.
    Right,
    /// Both children are present.
    Both,
}

impl PartyBranch {
    /// Whether the branch has a stored left child.
    pub(crate) fn has_left(self) -> bool {
        matches!(self, PartyBranch::Left | PartyBranch::Both)
    }

    /// Whether the branch has a stored right child.
    pub(crate) fn has_right(self) -> bool {
        matches!(self, PartyBranch::Right | PartyBranch::Both)
    }

    /// Return the branch's left and right presence bits.
    pub(crate) fn presence(self) -> (bool, bool) {
        (self.has_left(), self.has_right())
    }
}

/// A cursor into a nonempty canonical party tree.
///
/// [`read`](Self::read) consumes the current two-bit tag. The cursor is not
/// cloneable; an additional traversal must explicitly construct another
/// cursor.
pub(crate) struct PartyCursor<'a> {
    /// Canonical source tree.
    bits: BitsView<'a>,
    /// Start of the next unread tag.
    pos: u64,
}

impl<'a> PartyCursor<'a> {
    /// Start at a complete party tree.
    pub(crate) fn root(bits: BitsView<'a>) -> Self {
        assert!(!bits.is_empty(), "a Party always owns a region");
        PartyCursor { bits, pos: 0 }
    }

    /// Start at the subtree whose tag begins at `pos`.
    pub(crate) fn at(bits: BitsView<'a>, pos: u64) -> Self {
        debug_assert!(
            pos + 2 <= bits.len(),
            "a subtree begins with a complete tag"
        );
        PartyCursor { bits, pos }
    }

    /// Decode the child-presence tag at `pos`.
    #[inline]
    fn tag(bits: BitsView<'_>, pos: u64) -> PartyNode {
        let left = bits.bit(pos);
        let right = bits.bit(pos + 1);
        match (left, right) {
            (false, false) => PartyNode::Owned,
            (true, false) => PartyNode::Branch(PartyBranch::Left),
            (false, true) => PartyNode::Branch(PartyBranch::Right),
            (true, true) => PartyNode::Branch(PartyBranch::Both),
        }
    }

    /// Decode the current node and advance past its tag.
    pub(crate) fn read(&mut self) -> PartyNode {
        crate::codec::scan::record_bits(2);
        let node = Self::tag(self.bits, self.pos);
        self.pos += 2;
        node
    }

    /// Decode the current node without advancing.
    pub(crate) fn peek(&self) -> PartyNode {
        crate::codec::scan::record_bits(2);
        Self::tag(self.bits, self.pos)
    }

    /// Advance past the complete current subtree.
    pub(crate) fn skip(&mut self) {
        let bits = self.bits;
        self.pos = crate::codec::skip_subtree(self.pos, |at| {
            // Presence bits count the children that must still be skipped.
            crate::codec::scan::record_bits(2);
            let children = u64::from(bits.bit(at)) + u64::from(bits.bit(at + 1));
            (children, at + 2)
        });
    }

    /// Advance past every present child of an already-read node.
    pub(crate) fn skip_present_children(&mut self, node: PartyNode) {
        if let PartyNode::Branch(branch) = node {
            if branch.has_left() {
                self.skip();
            }
            if branch.has_right() {
                self.skip();
            }
        }
    }

    /// The underlying bit stream.
    pub(crate) fn bits(&self) -> BitsView<'a> {
        self.bits
    }

    /// The current bit offset.
    pub(crate) fn offset(&self) -> u64 {
        self.pos
    }
}
