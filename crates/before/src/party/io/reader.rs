//! Forward traversal of a party's canonical tree.
//!
//! Each stored node begins with two bits saying whether its left and right
//! children are present. Present children follow in preorder. A `00` tag is an
//! owned terminal. An unowned child has no stored node, while a `Party` itself
//! always contains at least one owned terminal.
//!
//! Canonical trees contain no branch with two owned terminals, because that
//! branch denotes one owned terminal. Consequently every stored subtree is
//! nonempty and canonical. Ownership of a whole region can be decided from its
//! root tag without scanning its descendants.
//!
//! [`PartyReader`] consumes one such tree from left to right. It cannot be
//! cloned, so retaining an old traversal position requires an explicit new
//! reader rather than an accidental copy. Algorithms that align two parties
//! represent an absent child as `None`; it is not a stored node.

use crate::bits::Bits;
use crate::party::Party;
use crate::testing::instrument::scan;

/// One complete subtree borrowed from a party reader.
pub struct PartySubtree<'a> {
    /// Tree containing the subtree.
    bits: &'a Bits,
    /// Root tag.
    start: u64,
    /// First bit after the subtree.
    end: u64,
}

/// A possibly empty chain of one-child branches.
///
/// Forking preserves this path in both results before ownership can divide.
/// Keeping it as one delimited unit lets the writer copy it in bulk.
pub struct PartyPath<'a> {
    /// Tree containing the path.
    bits: &'a Bits,
    /// First branch tag.
    start: u64,
    /// First bit after the final branch tag.
    end: u64,
}

impl<'a> PartyPath<'a> {
    /// Number of stored bits in the shared path.
    pub fn stored_len(&self) -> u64 {
        self.end - self.start
    }

    /// Expose the delimited storage only to the Party writer.
    pub fn storage(self) -> (&'a Bits, u64, u64) {
        (self.bits, self.start, self.end)
    }
}

/// The first place a party's ownership can divide.
pub enum ForkPoint<'a> {
    /// The shared path ends at an owned region, which must be divided.
    Owned(PartyPath<'a>),
    /// The shared path ends at a branch whose two children are the division.
    Children(PartyPath<'a>),
}

impl<'a> PartySubtree<'a> {
    /// Open a reader at the subtree root.
    pub fn reader(self) -> PartyReader<'a> {
        PartyReader::from_storage(self.bits, self.start, self.end)
    }

    /// Number of stored bits in the subtree.
    pub fn stored_len(&self) -> u64 {
        self.end - self.start
    }

    /// Expose the delimited storage only to the Party writer.
    pub fn storage(self) -> (&'a Bits, u64, u64) {
        (self.bits, self.start, self.end)
    }

    /// Whether the subtree root is a branch.
    pub fn is_branch(&self) -> bool {
        !matches!(
            PartyReader::node_at(self.bits, self.start),
            PartyNode::Owned
        )
    }
}

/// An owned read-only snapshot of a party tree.
///
/// Fork planning needs to revisit the original shape after the live party has
/// been narrowed. This type makes that retained topology explicit without
/// pretending it is another live, linearly owned [`Party`].
pub struct PartySnapshot(Bits);

impl PartySnapshot {
    /// Retain the canonical storage of `party` for read-only planning.
    pub fn new(party: &Party) -> Self {
        Self(party.0.clone())
    }

    /// Start reading the retained tree.
    pub fn reader(&self) -> PartyReader<'_> {
        let end = PartyReader::source_len_of(&self.0);
        PartyReader::from_storage(&self.0, 0, end)
    }
}

/// One stored node in a canonical party.
#[derive(Clone, Copy)]
pub enum PartyNode {
    /// A terminal region the party owns completely.
    Owned,
    /// A subdivided region, with at least one present child.
    Branch(PartyBranch),
}

impl PartyNode {
    /// Return this node's children, or `None` for a wholly owned region.
    pub fn branch(self) -> Option<PartyBranch> {
        match self {
            PartyNode::Owned => None,
            PartyNode::Branch(branch) => Some(branch),
        }
    }
}

/// The children present beneath a stored branch.
///
/// The three variants are exactly the valid nonempty presence tags. The `00`
/// tag is [`PartyNode::Owned`], not a branch.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PartyBranch {
    /// Only the left child is present.
    Left,
    /// Only the right child is present.
    Right,
    /// Both children are present.
    Both,
}

/// One selected child of a binary tree node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BranchChoice {
    /// Select the left child.
    Left,
    /// Select the right child.
    Right,
}

impl BranchChoice {
    /// Convert a stored direction bit into its semantic choice.
    pub fn from_right(right: bool) -> Self {
        if right {
            Self::Right
        } else {
            Self::Left
        }
    }

    /// The compact representation bit used by paths and route storage.
    pub fn is_right(self) -> bool {
        matches!(self, Self::Right)
    }
}

impl PartyBranch {
    /// A branch containing only the selected child.
    pub fn with_only_child(choice: BranchChoice) -> Self {
        match choice {
            BranchChoice::Left => PartyBranch::Left,
            BranchChoice::Right => PartyBranch::Right,
        }
    }

    /// Whether the branch has a stored left child.
    pub fn has_left_child(self) -> bool {
        matches!(self, PartyBranch::Left | PartyBranch::Both)
    }

    /// Whether the branch has a stored right child.
    pub fn has_right_child(self) -> bool {
        matches!(self, PartyBranch::Right | PartyBranch::Both)
    }

    /// The branch containing every child present in either operand.
    pub fn union(self, other: PartyBranch) -> PartyBranch {
        match (self, other) {
            (PartyBranch::Left, PartyBranch::Left) => PartyBranch::Left,
            (PartyBranch::Right, PartyBranch::Right) => PartyBranch::Right,
            _ => PartyBranch::Both,
        }
    }
}

/// A reader over a nonempty canonical party tree.
///
/// [`read`](Self::read) consumes the current two-bit tag. The reader is not
/// cloneable; an additional traversal must explicitly construct another
/// reader.
pub struct PartyReader<'a> {
    /// Canonical source tree.
    bits: &'a Bits,
    /// Bit offset of the next unread tag.
    position: u64,
    /// First bit after the subtree this reader may consume.
    end: u64,
}

impl<'a> PartyReader<'a> {
    /// Start at a live party's root.
    pub fn for_party(party: &'a Party) -> Self {
        let end = Self::source_len_of(&party.0);
        Self::from_storage(&party.0, 0, end)
    }

    /// The length of the canonical Party stream backing this reader.
    pub fn source_len(&self) -> u64 {
        Self::source_len_of(self.bits)
    }

    /// Start at validated storage. Tests use this to exercise internal walks on
    /// constructed canonical trees without creating a live party.
    #[cfg(test)]
    pub fn from_bits(bits: &'a Bits) -> Self {
        let end = Self::source_len_of(bits);
        Self::from_storage(bits, 0, end)
    }

    /// Start at the subtree occupying `pos..end`.
    fn from_storage(bits: &'a Bits, pos: u64, end: u64) -> Self {
        let source_len = Self::source_len_of(bits);
        debug_assert!(source_len != 0, "a Party always owns a region");
        debug_assert!(
            pos + 2 <= end && end <= source_len,
            "a subtree begins with a complete tag"
        );
        PartyReader {
            bits,
            position: pos,
            end,
        }
    }

    /// Recover the live length from canonical marker padding.
    fn source_len_of(bits: &Bits) -> u64 {
        let bytes = bits.as_raw_slice();
        let last = *bytes
            .last()
            .expect("a Party's nonempty tree has canonical storage");
        debug_assert!(last != 0, "canonical storage ends in a marker bit");
        bytes.len() as u64 * 8 - 1 - u64::from(last.trailing_zeros())
    }

    /// Decode the child-presence tag at `position`.
    ///
    /// Party tags are uniformly two bits wide, so every tag starts at an even
    /// offset and lies wholly within one byte. Decoding that byte directly is
    /// substantially cheaper than invoking the general-purpose buffered bit
    /// reader used for variable-width version fields.
    #[inline]
    fn node_at(bits: &Bits, position: u64) -> PartyNode {
        debug_assert_eq!(position % 2, 0, "Party tags are two-bit aligned");
        let byte = bits.as_raw_slice()[(position / 8) as usize];
        let shift = 6 - (position % 8) as u32;
        scan::record_bits(2);
        match (byte >> shift) & 0b11 {
            0b00 => PartyNode::Owned,
            0b10 => PartyNode::Branch(PartyBranch::Left),
            0b01 => PartyNode::Branch(PartyBranch::Right),
            0b11 => PartyNode::Branch(PartyBranch::Both),
            _ => unreachable!("a two-bit word has four values"),
        }
    }

    /// Decode the current node and advance past its tag.
    pub fn read(&mut self) -> PartyNode {
        debug_assert!(
            self.position + 2 <= self.end,
            "the reader has a complete node"
        );
        let node = Self::node_at(self.bits, self.position);
        self.position += 2;
        node
    }

    /// Decode the current node without advancing.
    pub fn peek(&self) -> PartyNode {
        debug_assert!(
            self.position + 2 <= self.end,
            "the reader has a complete node"
        );
        Self::node_at(self.bits, self.position)
    }

    /// Advance past the complete current subtree.
    pub fn skip(&mut self) {
        let mut pending = 1u64;
        while pending != 0 {
            pending -= 1;
            if let PartyNode::Branch(branch) = self.read() {
                pending += u64::from(branch.has_left_child()) + u64::from(branch.has_right_child());
            }
        }
        debug_assert!(
            self.position <= self.end,
            "a subtree stays within its reader"
        );
    }

    /// Advance past every present child of an already-read node.
    pub fn skip_present_children(&mut self, node: PartyNode) {
        if let PartyNode::Branch(branch) = node {
            if branch.has_left_child() {
                self.skip();
            }
            if branch.has_right_child() {
                self.skip();
            }
        }
    }

    /// Restart at a known subtree in the same party.
    pub fn restart_at(&self, pos: u64) -> PartyReader<'a> {
        Self::from_storage(self.bits, pos, self.end)
    }

    /// The unread subtree's stored bit length, for builder sizing only.
    pub fn stored_len(&self) -> u64 {
        self.end - self.position
    }

    /// The current bit offset.
    pub fn offset(&self) -> u64 {
        self.position
    }

    /// Whether the reader has consumed its containing tree.
    pub fn at_end(&self) -> bool {
        self.position == self.end
    }

    /// Consume and delimit the complete current subtree.
    pub fn take_subtree(&mut self) -> (PartyNode, PartySubtree<'a>) {
        let start = self.position;
        let node = self.read();
        self.skip_present_children(node);
        (
            node,
            PartySubtree {
                bits: self.bits,
                start,
                end: self.position,
            },
        )
    }

    /// The complete subtree known to occupy the unread suffix.
    pub fn remainder(self) -> PartySubtree<'a> {
        debug_assert!(self.position < self.end, "a remainder contains one subtree");
        PartySubtree {
            bits: self.bits,
            start: self.position,
            end: self.end,
        }
    }

    /// Advance through one-child branches to the next ownership division.
    ///
    /// The reader stops just after the terminal or two-child branch tag. Its
    /// next unread node is therefore the branch's left child when the result is
    /// [`ForkPoint::Children`].
    pub fn next_fork(&mut self) -> ForkPoint<'a> {
        let start = self.position;
        loop {
            let node_start = self.position;
            match self.read() {
                PartyNode::Owned => {
                    return ForkPoint::Owned(PartyPath {
                        bits: self.bits,
                        start,
                        end: node_start,
                    });
                }
                PartyNode::Branch(PartyBranch::Both) => {
                    return ForkPoint::Children(PartyPath {
                        bits: self.bits,
                        start,
                        end: node_start,
                    });
                }
                PartyNode::Branch(PartyBranch::Left | PartyBranch::Right) => {}
            }
        }
    }
}
