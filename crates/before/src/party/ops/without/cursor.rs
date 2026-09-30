//! Traversal of a party as consecutive owned and unowned regions.

use crate::codec::{BitStack, BitsView};
use crate::party::tree::PartyCursor;
use crate::version::skyline::overlay::PlateauCursor;

/// The region currently covered by a cursor.
#[derive(Clone, Copy)]
pub(super) enum Item {
    /// One owned or unowned region.
    Region { owned: bool },
    /// A whole subtree of the first party that survives unchanged.
    Copy { start: u64 },
}

/// What reading one subtree tag exposed.
pub(super) enum Entered {
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
/// it, skip it, or retain its complete bit range.
pub(super) struct RegionCursor<'a> {
    /// Canonical source tree.
    bits: BitsView<'a>,
    /// The next unread tag.
    pos: u64,
    /// Root-to-current-region directions.
    path: BitStack,
    /// Whether each open left branch has a right child.
    pending_right: BitStack,
    /// Open left branches; zero exactly at the final region.
    open_lefts: u64,
    /// The current region or retained subtree.
    item: Item,
}

impl<'a> RegionCursor<'a> {
    /// Open a party at its current subtree.
    pub(super) fn open(src: PartyCursor<'a>) -> Self {
        Self {
            bits: src.bits(),
            pos: src.offset(),
            path: BitStack::new(),
            pending_right: BitStack::new(),
            open_lefts: 0,
            item: Item::Region { owned: false },
        }
    }

    /// The current item.
    pub(super) fn item(&self) -> Item {
        self.item
    }

    /// The source stream.
    pub(super) fn bits(&self) -> BitsView<'a> {
        self.bits
    }

    /// The first bit after the consumed item.
    pub(super) fn position(&self) -> u64 {
        self.pos
    }

    /// Whether the current region is owned.
    ///
    /// The retained-subtree state is never used as the side covering another
    /// cursor, so asking whether it is owned is a logic error.
    pub(super) fn owned(&self) -> bool {
        match self.item {
            Item::Region { owned } => owned,
            Item::Copy { .. } => unreachable!("a retained subtree never covers another region"),
        }
    }

    /// Read one tag while descending toward the next region.
    pub(super) fn enter(&mut self) -> Entered {
        crate::codec::scan::record_bits(2);
        let (left, right) = (self.bits.bit(self.pos), self.bits.bit(self.pos + 1));
        self.pos += 2;
        if !left && !right {
            self.item = Item::Region { owned: true };
            return Entered::Owned;
        }

        self.path.push(false);
        self.pending_right.push(right);
        self.open_lefts += 1;
        if left {
            Entered::Child
        } else {
            self.item = Item::Region { owned: false };
            Entered::Unowned
        }
    }

    /// Descend to the subtree's first owned or unowned region.
    pub(super) fn descend(&mut self) {
        while matches!(self.enter(), Entered::Child) {}
    }

    /// Skip the current subtree without visiting each region it contains.
    ///
    /// A terminal becomes the current owned region. A branch is scanned once.
    /// If `copy` is true, its unchanged bit range is retained for the output;
    /// otherwise it becomes one unowned region.
    pub(super) fn skip_subtree(&mut self, copy: bool) {
        let start = self.pos;
        crate::codec::scan::record_bits(2);
        let (left, right) = (self.bits.bit(self.pos), self.bits.bit(self.pos + 1));
        self.pos += 2;
        if !left && !right {
            self.item = Item::Region { owned: true };
            return;
        }

        let bits = self.bits;
        let scan = |at: u64| {
            crate::codec::scan::record_bits(2);
            let children = u64::from(bits.bit(at)) + u64::from(bits.bit(at + 1));
            (children, at + 2)
        };
        if left {
            self.pos = crate::codec::skip_subtree(self.pos, scan);
        }
        if right {
            self.pos = crate::codec::skip_subtree(self.pos, scan);
        }
        self.item = if copy {
            Item::Copy { start }
        } else {
            Item::Region { owned: false }
        };
    }
}

/// Supplies interval depth and boundary movement to the shared overlay walk.
impl PlateauCursor for RegionCursor<'_> {
    /// Whether advancing entered a stored subtree that still needs resolving.
    type Crossing = bool;

    /// The current region's depth.
    fn depth(&self) -> u64 {
        self.path.len()
    }

    /// Whether this is the final region.
    fn done(&self) -> bool {
        self.open_lefts == 0
    }

    /// Advance to the next child position.
    fn step(&mut self) -> (u64, bool) {
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
            (depth, true)
        } else {
            self.item = Item::Region { owned: false };
            (depth, false)
        }
    }
}
