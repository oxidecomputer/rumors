//! Traversal of a party as consecutive owned and unowned regions.

use crate::bits::stack::BitStack;
use crate::party::io::{PartyBranch, PartyNode, PartyReader, PartySubtree};
use crate::version::io::regions::RegionReader as OverlayCursor;

/// The region currently covered by a reader.
pub enum Item<'a> {
    /// One owned or unowned region.
    Region { owned: bool },
    /// A whole subtree of the first party that survives unchanged.
    Retained(Option<PartySubtree<'a>>),
}

/// What reading one subtree tag exposed.
pub enum Entered {
    /// The subtree is one owned region.
    Owned,
    /// A present left child must be examined.
    Child,
    /// An absent left child is one unowned region.
    Unowned,
}

/// Walks a party from the left edge of the unit interval to the right.
///
/// An absent child occupies no source bits but appears as an unowned region.
/// The compact path records enough state to find the next region without
/// recursion. After entering a subtree, the difference walk may descend into
/// it, skip it, or retain the complete subtree for the output.
pub struct DifferenceReader<'a> {
    /// The next unread source node.
    cursor: PartyReader<'a>,
    /// Root-to-current-region directions.
    path: BitStack,
    /// Whether each open left branch has a right child.
    pending_right: BitStack,
    /// Open left branches; zero exactly at the final region.
    open_lefts: u64,
    /// The current region or retained subtree.
    item: Item<'a>,
}

impl<'a> DifferenceReader<'a> {
    /// Open a party at its current subtree.
    pub fn open(src: PartyReader<'a>) -> Self {
        Self {
            cursor: src,
            path: BitStack::new(),
            pending_right: BitStack::new(),
            open_lefts: 0,
            item: Item::Region { owned: false },
        }
    }

    /// Take a retained subtree, if this item is one.
    ///
    /// A retained subtree is handed to the writer exactly once. The cursor
    /// advances immediately afterward, so an empty slot is never observed by
    /// another traversal step.
    pub fn take_retained_subtree(&mut self) -> Option<PartySubtree<'a>> {
        match &mut self.item {
            Item::Region { .. } => None,
            Item::Retained(subtree) => Some(
                subtree
                    .take()
                    .expect("a retained subtree is emitted exactly once"),
            ),
        }
    }

    /// Whether the current region is owned.
    ///
    /// The retained-subtree state is never used as the side covering another
    /// cursor, so asking whether it is owned is a logic error.
    pub fn owned(&self) -> bool {
        match &self.item {
            Item::Region { owned } => *owned,
            Item::Retained(_) => unreachable!("a retained subtree never covers another region"),
        }
    }

    /// Read one tag while descending toward the next region.
    pub fn enter(&mut self) -> Entered {
        match self.cursor.read() {
            PartyNode::Owned => {
                self.item = Item::Region { owned: true };
                Entered::Owned
            }
            PartyNode::Branch(branch) => {
                self.path.push(false);
                self.pending_right.push(branch.has_right_child());
                self.open_lefts += 1;
                match branch {
                    PartyBranch::Left | PartyBranch::Both => Entered::Child,
                    PartyBranch::Right => {
                        self.item = Item::Region { owned: false };
                        Entered::Unowned
                    }
                }
            }
        }
    }

    /// Descend to the subtree's first owned or unowned region.
    pub fn descend(&mut self) {
        while matches!(self.enter(), Entered::Child) {}
    }

    /// Skip the current subtree without visiting each region it contains.
    ///
    /// An owned node becomes the current owned region. A branch is scanned
    /// once. When `retain` is true, its complete subtree is saved for the
    /// output; otherwise it becomes one unowned region.
    pub fn skip_subtree(&mut self, disposition: SubtreeDisposition) {
        let (node, subtree) = self.cursor.take_subtree();
        self.item = match node {
            PartyNode::Owned => Item::Region { owned: true },
            PartyNode::Branch(_) if disposition == SubtreeDisposition::Retain => {
                Item::Retained(Some(subtree))
            }
            PartyNode::Branch(_) => Item::Region { owned: false },
        };
    }
}

/// What to do with a branch skipped as one semantic region.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SubtreeDisposition {
    /// Copy the complete subtree to the difference output.
    Retain,
    /// Replace the subtree with one unowned region.
    Discard,
}

/// What advancing a difference cursor reached.
pub enum Boundary {
    /// An absent child, already resolved as an unowned region.
    Region,
    /// A stored subtree whose effect still needs resolving.
    Subtree,
}

/// Supplies interval depth and boundary movement to the shared overlay walk.
impl OverlayCursor for DifferenceReader<'_> {
    type Crossing = Boundary;

    /// The current region's depth.
    fn depth(&self) -> u64 {
        self.path.len()
    }

    /// Whether this is the final region.
    fn done(&self) -> bool {
        self.open_lefts == 0
    }

    /// Advance to the next child position.
    fn step(&mut self) -> (u64, Boundary) {
        loop {
            match self.path.pop() {
                Some(true) => continue,
                Some(false) => break,
                None => unreachable!("the final region is never advanced"),
            }
        }
        self.open_lefts -= 1;
        self.path.push(true);
        let depth = self.depth();
        let right_present = self
            .pending_right
            .pop()
            .expect("every open left branch records its right child");
        if right_present {
            (depth, Boundary::Subtree)
        } else {
            self.item = Item::Region { owned: false };
            (depth, Boundary::Region)
        }
    }
}
