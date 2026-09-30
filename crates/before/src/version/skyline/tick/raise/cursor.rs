//! Forward cursors used while applying a raise route.

use core::ops::Range;

use crate::codec::{self, BitCursor, BitsView};

use super::super::super::build::SkylineBuilder;
use super::super::super::walk::LeafWalk;
use super::{Repair, Step};

/// A forward cursor over version topology and payloads.
pub(super) struct VersionCursor<'a> {
    /// Complete source version for range splicing.
    bits: BitsView<'a>,
    /// Next node flag or payload.
    cursor: codec::DsiCursor<'a>,
}

impl<'a> VersionCursor<'a> {
    /// Start at the version root.
    pub(super) fn new(bits: BitsView<'a>) -> Self {
        Self {
            bits,
            cursor: codec::DsiCursor::new(bits),
        }
    }

    /// Bit position of the next node flag.
    pub(super) fn pos(&self) -> u64 {
        self.cursor.position()
    }

    /// Move directly to a node flag located by a side scan.
    fn seek(&mut self, pos: u64) {
        self.cursor = codec::DsiCursor::new_at(self.bits, pos);
    }

    /// Read one node, returning a leaf's payload range or `None` for a branch.
    pub(super) fn read(&mut self) -> Option<Range<u64>> {
        if !self.cursor.read_bit().expect("canonical skyline bits") {
            return None;
        }
        let start = self.cursor.position();
        self.cursor.skip_int().expect("canonical skyline bits");
        Some(start..self.cursor.position())
    }

    /// Advance past one complete subtree.
    #[cfg(test)]
    pub(super) fn skip(&mut self) {
        // Each branch adds one pending child overall; each leaf consumes one.
        let mut pending = 1u64;
        while pending > 0 {
            let branches = self.cursor.read_unary().expect("canonical skyline bits");
            self.cursor.skip_int().expect("canonical skyline bits");
            pending = pending + branches - 1;
        }
    }

    /// Copy the subtree at the cursor, repairing its first delta if needed.
    pub(super) fn feed_subtree(
        &mut self,
        output: &mut SkylineBuilder,
        depth: u64,
        repair: Repair<'_>,
    ) {
        let subtree = Subtree::scan(self.bits, self.pos());
        output.leaf(depth + subtree.first_depth, |payload| match repair {
            Repair::None => {
                payload.splice(self.bits, subtree.first_code.start, subtree.first_code.end)
            }
            Repair::Minus(ticks) => {
                Step::DownDelta.write(payload, self.bits, subtree.first_code.clone(), ticks)
            }
        });
        if subtree.first_depth > 0 {
            output.continue_verbatim(
                self.bits,
                subtree.first_code.end,
                subtree.end,
                depth,
                subtree.first_depth,
                subtree.last_depth,
                subtree.last_code.end - subtree.last_code.start,
            );
        }
        self.seek(subtree.end);
    }
}

/// A forward cursor over party-tree tags.
pub(super) struct PartyTags<'a> {
    /// Complete party tree.
    bits: BitsView<'a>,
    /// Next tag position.
    pos: u64,
}

impl<'a> PartyTags<'a> {
    /// Start at the party root.
    pub(super) fn new(bits: BitsView<'a>) -> Self {
        Self { bits, pos: 0 }
    }

    /// Read the child-presence tags at `pos` without advancing a cursor.
    #[cfg(test)]
    pub(super) fn tag_at(bits: BitsView<'_>, pos: u64) -> (bool, bool) {
        (bits.bit(pos), bits.bit(pos + 1))
    }

    /// Read `(position, left present, right present)` for the next node.
    pub(super) fn read(&mut self) -> (u64, bool, bool) {
        let key = self.pos;
        codec::scan::record_bits(2);
        let children = (self.bits.bit(key), self.bits.bit(key + 1));
        self.pos += 2;
        (key, children.0, children.1)
    }

    /// Skip the subtree at the current position.
    pub(super) fn skip_subtree(&mut self) {
        self.pos = crate::codec::skip_subtree(self.pos, |at| {
            codec::scan::record_bits(2);
            let children = u64::from(self.bits.bit(at)) + u64::from(self.bits.bit(at + 1));
            (children, at + 2)
        });
    }
}

/// Coordinates needed to splice one complete version subtree.
struct Subtree {
    /// Bit position after the subtree.
    end: u64,
    /// First leaf's payload range.
    first_code: Range<u64>,
    /// First leaf's depth below the subtree root.
    first_depth: u64,
    /// Last leaf's payload range.
    last_code: Range<u64>,
    /// Last leaf's depth below the subtree root.
    last_depth: u64,
}

impl Subtree {
    /// Locate the subtree and the boundary leaves needed for splicing.
    fn scan(bits: BitsView<'_>, start: u64) -> Self {
        let mut cursor = codec::DsiCursor::new_at(bits, start);
        let mut first = None;
        let mut last_code = 0..0;
        let mut last_depth = 0;
        let mut walk = LeafWalk::new();

        while let Some(depth) = walk.descend(&mut cursor) {
            let code_start = cursor.position();
            cursor.skip_int().expect("canonical skyline bits");
            last_code = code_start..cursor.position();
            last_depth = depth;
            first.get_or_insert_with(|| (last_code.clone(), depth));
        }

        let (first_code, first_depth) = first.expect("a subtree has at least one leaf");
        Self {
            end: cursor.position(),
            first_code,
            first_depth,
            last_code,
            last_depth,
        }
    }
}
