//! Compact accounting for leaves that supply subtree minima.
//!
//! One leaf may be the minimum of many nested subtrees. A contribution records
//! that leaf's height relative to a frozen prefix and counts the subtrees that
//! use it. Settlement then subtracts the offset once, multiplied by the count,
//! and attaches the same count to the frozen prefix.

use num_bigint::{BigInt, Sign};
use suanpan::Accumulator;

use super::heights::{HeightPrefixes, LeafHeight};

#[cfg(test)]
mod tests;

/// The exact contribution of one leaf used by one or more subtree minima.
struct Contribution {
    /// Signed leaf height relative to `prefix`.
    offset: BigInt,
    /// Frozen height prefix that anchors `offset`.
    prefix: usize,
    /// Number of subtree closes whose minimum this leaf supplied.
    closes: u64,
}

impl Contribution {
    /// Construct a contribution that has not yet supplied a closed subtree.
    fn new(offset: BigInt, prefix: usize) -> Self {
        Self {
            offset,
            prefix,
            closes: 0,
        }
    }

    /// Subtract this contribution from the minimum-tick total.
    fn settle(self, total: &mut Accumulator, prefixes: &mut HeightPrefixes) {
        if self.closes == 0 {
            return;
        }
        prefixes.subtract_minimum(self.prefix, self.closes);
        HeightPrefixes::fold_repeated(
            total,
            self.offset.magnitude(),
            u128::from(self.closes),
            self.offset.sign() != Sign::Minus,
        );
    }
}

/// One contribution stored inline or as an index into [`ContributionStore`].
///
/// The inline form holds a signed 32-bit offset, a 23-bit prefix index, and an
/// 8-bit close count. These widths keep the common record to one machine word.
/// Larger values spill to exact storage; they are representation thresholds,
/// not input limits.
#[derive(Clone, Copy)]
pub struct StoredContribution(u64);

/// Bits reserved for the signed offset.
const OFFSET_BITS: u32 = 32;
/// Bits reserved for the frozen-prefix index.
const PREFIX_BITS: u32 = 23;
/// Bits reserved for the close count.
const CLOSE_BITS: u32 = 8;
/// Marks a word as an index into exact storage.
const SPILLED: u64 = 1 << 63;
/// Mask selecting the inline offset.
const OFFSET_MASK: u64 = (1 << OFFSET_BITS) - 1;
/// Mask selecting the inline prefix index.
const PREFIX_MASK: u64 = (1 << PREFIX_BITS) - 1;
/// Mask selecting the inline close count.
const CLOSE_MASK: u64 = (1 << CLOSE_BITS) - 1;
/// Bit position of the inline prefix index.
const PREFIX_SHIFT: u32 = OFFSET_BITS;
/// Bit position of the inline close count.
const CLOSE_SHIFT: u32 = OFFSET_BITS + PREFIX_BITS;

const _: () = assert!(OFFSET_BITS + PREFIX_BITS + CLOSE_BITS == 63);

impl StoredContribution {
    /// Encode a contribution inline when both initial fields fit.
    fn inline(offset: &BigInt, prefix: usize) -> Option<Self> {
        let offset = i32::try_from(offset).ok()?;
        if prefix > PREFIX_MASK as usize {
            return None;
        }
        Some(Self(
            u64::from(offset as u32) | (prefix as u64) << PREFIX_SHIFT,
        ))
    }

    /// Whether this word names an exact out-of-line contribution.
    fn is_spilled(self) -> bool {
        self.0 & SPILLED != 0
    }

    /// Decode an inline contribution exactly.
    fn decode(self) -> Contribution {
        debug_assert!(!self.is_spilled());
        let offset = (self.0 & OFFSET_MASK) as u32 as i32;
        let prefix = ((self.0 >> PREFIX_SHIFT) & PREFIX_MASK) as usize;
        let closes = (self.0 >> CLOSE_SHIFT) & CLOSE_MASK;
        Contribution {
            offset: BigInt::from(offset),
            prefix,
            closes,
        }
    }

    /// Index of the exact contribution in the spill store.
    fn spill_index(self) -> usize {
        debug_assert!(self.is_spilled());
        (self.0 & !SPILLED) as usize
    }
}

/// Exact storage for contributions that exceed the inline representation.
///
/// A spill allocates one stable slot and clones the offset once. Later close
/// counts remain constant-time, and settlement releases the slot for reuse.
pub struct ContributionStore {
    /// Stable slots addressed by spilled contributions.
    slots: Vec<Option<Contribution>>,
    /// Vacant slots available for reuse.
    free: Vec<usize>,
}

impl ContributionStore {
    /// Construct an empty store.
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            free: Vec::new(),
        }
    }

    /// Store a new contribution inline when possible.
    pub fn store(&mut self, leaf: &LeafHeight) -> StoredContribution {
        StoredContribution::inline(leaf.offset(), leaf.prefix())
            .unwrap_or_else(|| self.spill(Contribution::new(leaf.offset().clone(), leaf.prefix())))
    }

    /// Store arbitrary fields for representation-boundary tests.
    #[cfg(test)]
    fn store_parts(&mut self, offset: &BigInt, prefix: usize) -> StoredContribution {
        StoredContribution::inline(offset, prefix)
            .unwrap_or_else(|| self.spill(Contribution::new(offset.clone(), prefix)))
    }

    /// Put an exact contribution in a stable spill slot.
    fn spill(&mut self, contribution: Contribution) -> StoredContribution {
        let index = if let Some(index) = self.free.pop() {
            debug_assert!(self.slots[index].is_none());
            self.slots[index] = Some(contribution);
            index
        } else {
            let index = self.slots.len();
            self.slots.push(Some(contribution));
            index
        };
        let index = u64::try_from(index).expect("contribution spill index fits u64");
        assert!(index < SPILLED, "contribution spill index fits 63 bits");
        StoredContribution(SPILLED | index)
    }

    /// Count one subtree close, spilling if the inline count is full.
    pub fn increment(&mut self, contribution: &mut StoredContribution) {
        if contribution.is_spilled() {
            self.slots[contribution.spill_index()]
                .as_mut()
                .expect("a spilled contribution owns its slot")
                .closes += 1;
            return;
        }
        let closes = (contribution.0 >> CLOSE_SHIFT) & CLOSE_MASK;
        if closes < CLOSE_MASK {
            contribution.0 += 1 << CLOSE_SHIFT;
            return;
        }
        let mut exact = contribution.decode();
        exact.closes += 1;
        *contribution = self.spill(exact);
    }

    /// Remove a contribution, decoding or releasing its exact storage.
    fn take(&mut self, contribution: StoredContribution) -> Contribution {
        if !contribution.is_spilled() {
            return contribution.decode();
        }
        let index = contribution.spill_index();
        let exact = self.slots[index]
            .take()
            .expect("a spilled contribution owns its slot");
        self.free.push(index);
        exact
    }

    /// Settle a contribution and release any spill slot that it occupied.
    pub fn settle(
        &mut self,
        contribution: StoredContribution,
        total: &mut Accumulator,
        prefixes: &mut HeightPrefixes,
    ) {
        self.take(contribution).settle(total, prefixes);
    }
}
