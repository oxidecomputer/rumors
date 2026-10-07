//! Minima computed ahead of the main tick cursor.
//!
//! Simplifying an owned left child may depend on the simplified minimum of its
//! later right sibling. A pre-scan computes that value, along with nested
//! lookaheads in the same range, before the main walk continues.
//!
//! [`Memo`] passes those results from the pre-scan to the walk in the order the
//! walk encounters the branches. Each result is a difference from a minimum
//! the walk already holds. Equal minima produce a zero difference and occupy
//! no value storage; nonzero differences use [`StoredAccumulator`], which
//! keeps a machine-sized value inline and boxes only a wide one.
//!
//! The pre-scan discovers branches in preorder but finishes them when their
//! ranges close, so writes may arrive out of order. Small fixed-size blocks
//! keep values in eventual read order while a bitset represents zero
//! differences without storage. An insertion moves values within one block at
//! most, avoiding one machine-word index per memo entry.

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

/// Results from one pre-scan, consumed by the main tick walk.
pub struct Memo {
    /// Consumption-order slots, grouped to keep zero differences compact.
    blocks: Vec<Block>,
    /// Number of slots in the current scan.
    len: usize,
    /// Next slot the main walk will consume.
    pub cursor: usize,
    /// End of the current pre-scan's version range. A lookahead before this
    /// position already has a memo slot; one at or after it starts a new scan.
    pub covered_until: u64,
    /// Order-sensitive checksum of the positions recorded by the pre-scan.
    #[cfg(debug_assertions)]
    pub recorded_check: u64,
    /// Order-sensitive checksum of the positions consumed by the walk.
    #[cfg(debug_assertions)]
    pub consumed_check: u64,
}

impl Memo {
    /// Construct an empty memo.
    pub fn new() -> Self {
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

    /// Fold one position into the debug-only order checksum.
    #[cfg(debug_assertions)]
    pub fn check_position(check: u64, pos: u64) -> u64 {
        (check ^ pos).wrapping_mul(0x0100_0000_01b3)
    }

    /// Number of lookahead minima reserved by the current scan.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Reset for a new pre-scan, retaining allocations for reuse.
    pub fn begin_scan(&mut self) {
        debug_assert_eq!(self.cursor, self.len, "the prior scan drained");
        #[cfg(debug_assertions)]
        debug_assert_eq!(
            self.recorded_check, self.consumed_check,
            "the walk consumed the recorded lookaheads in order"
        );
        for block in &mut self.blocks[..self.len.div_ceil(BLOCK_SLOTS)] {
            block.clear();
        }
        self.len = 0;
        self.cursor = 0;
    }

    /// Reserve and return the next consumption-order slot.
    pub fn reserve(&mut self) -> usize {
        let slot = self.len;
        if slot / BLOCK_SLOTS == self.blocks.len() {
            self.blocks.push(Block::new());
        }
        self.len += 1;
        slot
    }

    /// Store a nonzero difference in a reserved slot.
    pub fn set_link(&mut self, slot: usize, link: StoredAccumulator) {
        debug_assert!(slot < self.len, "a memo value has a reserved slot");
        self.blocks[slot / BLOCK_SLOTS].set(slot % BLOCK_SLOTS, link);
    }

    /// Take a slot's difference for its single consuming read.
    pub fn take_link(&mut self, slot: usize) -> Option<StoredAccumulator> {
        debug_assert!(slot < self.len, "the walk consumes a recorded slot");
        self.blocks[slot / BLOCK_SLOTS].take(slot % BLOCK_SLOTS)
    }
}

#[cfg(test)]
mod tests;
