//! Minima computed ahead of the fill walk.
//!
//! A left-full party branch replaces its left event range with a leaf whose
//! height depends on the filled minimum of the right range. That minimum lies
//! ahead of the main walk. A pre-scan therefore computes it, along with every
//! other left-full minimum inside the same range, before the walk continues.
//!
//! [`Memo`] passes those results from the pre-scan to the walk in the order the
//! walk encounters the branches. Each result is a difference from a minimum
//! the walk already holds. Equal minima produce a zero difference and occupy
//! no value storage; nonzero differences use [`StoredAccumulator`], which
//! keeps a machine-sized value inline and boxes only a wide one.
//!
//! The pre-scan discovers branches in preorder but finishes their minima as
//! ranges close, so results are not written in consumption order. The memo
//! divides the preorder slots into small blocks. Within a block, its presence
//! bits locate nonzero values and its value vector stays in slot order. An
//! out-of-order insertion can move at most one block, giving constant work per
//! result without a machine-word index for every zero entry. This is important
//! for nested inputs where thousands of branches share the same minimum.

use super::StoredAccumulator;

/// Slots per memo block.
///
/// One `u64` records exactly one block's occupied slots. The fixed bound also
/// caps the work of an out-of-order insertion at 63 value moves.
const BLOCK_SLOTS: usize = u64::BITS as usize;

/// One block of consumption-order memo slots.
struct Block {
    /// A set bit means the corresponding slot has a nonzero value.
    occupied: u64,
    /// Nonzero values in increasing slot order.
    values: Vec<StoredAccumulator>,
}

impl Block {
    /// Construct an empty block.
    fn new() -> Self {
        Block {
            occupied: 0,
            values: Vec::new(),
        }
    }

    /// Reset the block while retaining its value allocation for a later scan.
    fn clear(&mut self) {
        self.occupied = 0;
        self.values.clear();
    }

    /// Store a nonzero value at `slot` within this block.
    fn set(&mut self, slot: usize, value: StoredAccumulator) {
        let bit = 1u64 << slot;
        debug_assert_eq!(self.occupied & bit, 0, "a memo slot is written once");
        let index = (self.occupied & (bit - 1)).count_ones() as usize;
        self.values.insert(index, value);
        self.occupied |= bit;
    }

    /// Take the value at `slot`, if the slot holds a nonzero difference.
    fn take(&mut self, slot: usize) -> Option<StoredAccumulator> {
        let bit = 1u64 << slot;
        if self.occupied & bit == 0 {
            return None;
        }
        let index = (self.occupied & (bit - 1)).count_ones() as usize;
        Some(core::mem::replace(
            &mut self.values[index],
            StoredAccumulator::Small(0),
        ))
    }
}

/// Results from one memoizing pre-scan, consumed by the fill walk.
pub(super) struct Memo {
    /// Consumption-order slots, grouped to keep zero differences compact.
    blocks: Vec<Block>,
    /// Number of slots in the current scan.
    len: usize,
    /// Next slot the fill walk will consume.
    pub(super) cursor: usize,
    /// End of the current pre-scan's event range. A site before this position
    /// already has a memo slot; a site at or after it starts the next scan.
    pub(super) covered_until: u64,
    /// Order-sensitive checksum of the positions recorded by the pre-scan.
    #[cfg(debug_assertions)]
    pub(super) recorded_check: u64,
    /// Order-sensitive checksum of the positions consumed by the walk.
    #[cfg(debug_assertions)]
    pub(super) consumed_check: u64,
}

/// Fold one position into an order-sensitive checksum (FNV-style).
#[cfg(debug_assertions)]
pub(super) fn position_check(check: u64, pos: u64) -> u64 {
    (check ^ pos).wrapping_mul(0x0100_0000_01b3)
}

impl Memo {
    /// Construct an empty memo.
    pub(super) fn new() -> Self {
        Memo {
            blocks: Vec::new(),
            len: 0,
            cursor: 0,
            covered_until: 0,
            #[cfg(debug_assertions)]
            recorded_check: 0,
            #[cfg(debug_assertions)]
            consumed_check: 0,
        }
    }

    /// Number of sites recorded by the current scan.
    pub(super) fn len(&self) -> usize {
        self.len
    }

    /// Reset for a new pre-scan, retaining allocations for reuse.
    pub(super) fn begin_scan(&mut self) {
        debug_assert_eq!(self.cursor, self.len, "the prior scan drained");
        #[cfg(debug_assertions)]
        debug_assert_eq!(
            self.recorded_check, self.consumed_check,
            "the walk consumed the recorded sites, in order"
        );
        for block in &mut self.blocks[..self.len.div_ceil(BLOCK_SLOTS)] {
            block.clear();
        }
        self.len = 0;
        self.cursor = 0;
    }

    /// Reserve and return the next consumption-order slot.
    pub(super) fn reserve(&mut self) -> usize {
        let slot = self.len;
        if slot / BLOCK_SLOTS == self.blocks.len() {
            self.blocks.push(Block::new());
        }
        self.len += 1;
        slot
    }

    /// Store a nonzero difference in a reserved slot.
    pub(super) fn set_link(&mut self, slot: usize, link: StoredAccumulator) {
        debug_assert!(slot < self.len, "a memo value has a reserved slot");
        self.blocks[slot / BLOCK_SLOTS].set(slot % BLOCK_SLOTS, link);
    }

    /// Take a slot's difference for its single consuming read.
    pub(super) fn take_link(&mut self, slot: usize) -> Option<StoredAccumulator> {
        debug_assert!(slot < self.len, "the walk consumes a recorded slot");
        self.blocks[slot / BLOCK_SLOTS].take(slot % BLOCK_SLOTS)
    }
}

#[cfg(test)]
mod tests {
    use super::{Memo, StoredAccumulator};

    /// Values inserted nonsequentially at both edges of three memo blocks are
    /// read back from those same six slots.
    #[test]
    fn selected_out_of_order_values_cross_block_boundaries() {
        let mut memo = Memo::new();
        let mut expected = [None; 130];
        for _ in &expected {
            memo.reserve();
        }

        for (slot, value) in [(129, -7), (64, 5), (0, 1), (63, -2), (65, 3), (128, 4)] {
            expected[slot] = Some(value);
            memo.set_link(slot, StoredAccumulator::Small(value));
        }

        for (slot, expected) in expected.into_iter().enumerate() {
            let actual = memo.take_link(slot).map(|stored| match stored {
                StoredAccumulator::Small(value) => value,
                StoredAccumulator::Wide(_) => panic!("the fixture stores only machine words"),
            });
            assert_eq!(actual, expected, "slot {slot}");
        }
    }
}
