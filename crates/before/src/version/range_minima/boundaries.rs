//! Compact LIFO storage for the differences between armed range minima.
//!
//! Equal adjacent minima form counted zero runs. Separate bit tags, packed
//! integers, and a sparse vector keep each boundary's storage proportional to
//! its width. All columns are LIFO, so they need no per-record indices.
//!
//! # Wide boundaries: live, copied, or whole
//!
//! A boundary wider than a word is stored in one of three ways.
//!
//! - The innermost positive boundary, when it is wide, stays *live*: its
//!   accumulator waits outside the columns until another positive boundary
//!   covers it.
//! - A covered boundary whose working width is at most
//!   [`INLINE_BOUNDARY_BITS`] is *copied*: its limbs enter the packed column,
//!   and a pop rebuilds an accumulator from them.
//! - A wider covered boundary moves *whole*: its accumulator goes onto a
//!   vector, unread.
//!
//! Each form spares one cost.
//!
//! - The live form spares a shrinking boundary from being reread. A drop that
//!   leaves a smaller remainder in place pushes it straight back as the live
//!   boundary, so the tracker never copies a surviving boundary on behalf of
//!   the decrease that shrank it.
//! - The copied form spares a covered boundary the fixed-size structure an
//!   accumulator keeps beside its digits, so the boundary costs space in
//!   proportion to its width. Copying reads the boundary, so it happens only
//!   when a push covers it: each push copies at most one boundary, of at most
//!   [`INLINE_BOUNDARY_BITS`].
//! - The whole form spares a boundary too wide to copy at constant cost from
//!   being read at all. Such a boundary is wide enough that its accumulator's
//!   fixed size is at most a constant per bit it holds.

use suanpan::Accumulator;

use crate::bits::stack::{BitStack, PackedU64Stack};

use super::boundary::Boundary;

/// The widest working width, in bits, at which a covered wide boundary is
/// copied into packed storage instead of moving whole.
///
/// Any fixed bound keeps a copy's cost constant; this one sets how many bits
/// a whole boundary holds at least, against its accumulator's fixed size.
///
/// The bound compares working width, and a value is below `2^(w + 2)` for a
/// working width of `w` bits, so a copied boundary can be up to two bits
/// wider than the bound and take five limbs.
pub const INLINE_BOUNDARY_BITS: u64 = 256;

/// One logical record, starting with the innermost boundary when popped.
pub(super) enum Entry {
    /// This many adjacent pairs of ranges share their minimum.
    Equal(u64),
    /// The outer minimum is lower by this difference.
    Positive(Boundary),
}

/// Synchronized storage columns for zero runs and positive boundaries.
///
/// The records form one stack: the columns hold the older records, then the
/// live wide boundary if there is one, then the top zero run if it is
/// nonzero.
pub(super) struct Boundaries {
    /// Whether each stored record is positive rather than an equal-minimum run.
    positive: BitStack,
    /// Whether each stored positive boundary needs arbitrary precision.
    wide: BitStack,
    /// Whether each stored wide boundary moved whole rather than being copied.
    whole: BitStack,
    /// Zero-run counts, word-sized positive differences, and each copied wide
    /// boundary as its limbs, least significant first, below their count.
    compact: PackedU64Stack,
    /// The innermost positive boundary, when it is wide.
    live_wide: Option<Accumulator>,
    /// Top zero run; extending it must cost O(1), regardless of its count.
    top_zeros: u64,
    /// One accumulator per stored wide boundary that moved whole.
    whole_values: Vec<Accumulator>,
}

/// Keep the storage columns in lockstep as complete records enter and leave.
impl Boundaries {
    /// Construct an empty boundary stack without allocating.
    pub(super) fn new() -> Self {
        Self {
            positive: BitStack::new(),
            wide: BitStack::new(),
            whole: BitStack::new(),
            compact: PackedU64Stack::new(),
            live_wide: None,
            top_zeros: 0,
            whole_values: Vec::new(),
        }
    }

    /// Whether no pair of armed ranges remains separated by a boundary.
    pub(super) fn is_empty(&self) -> bool {
        self.top_zeros == 0 && self.live_wide.is_none() && self.positive.len() == 0
    }

    /// Store the live wide boundary and the top zero run above it, in that
    /// order, before a positive boundary covers them.
    fn store_top(&mut self) {
        if let Some(wide) = self.live_wide.take() {
            self.positive.push(true);
            self.wide.push(true);
            if wide.stored_bits() <= INLINE_BOUNDARY_BITS {
                self.whole.push(false);
                self.push_copied(wide);
            } else {
                self.whole.push(true);
                self.whole_values.push(wide);
            }
        }
        if self.top_zeros != 0 {
            self.positive.push(false);
            self.compact.push(self.top_zeros);
            self.top_zeros = 0;
        }
    }

    /// Push one positive boundary above the current zero run.
    pub(super) fn push_positive(&mut self, boundary: Boundary) {
        self.store_top();
        match boundary {
            Boundary::Word(word) => {
                self.positive.push(true);
                self.wide.push(false);
                self.compact.push(word);
            }
            Boundary::Wide(wide) => self.live_wide = Some(wide),
        }
    }

    /// Copy a positive value's limbs, least significant first, then their count.
    fn push_copied(&mut self, value: Accumulator) {
        let (_, magnitude) = value.signed_magnitude();
        for &limb in magnitude.as_ref() {
            self.compact.push(limb);
        }
        self.compact.push(
            u64::try_from(magnitude.len()).expect("a value below 2^258 has at most five limbs"),
        );
    }

    /// Rebuild a value [`Self::push_copied`] stored, popping its most
    /// significant limb first.
    fn pop_copied(&mut self) -> Accumulator {
        let count = self.compact.pop();
        match count {
            // Through `u128`, a value up to 2^96 stays inline: no digits to read.
            1 | 2 => {
                let high = if count == 2 { self.compact.pop() } else { 0 };
                let low = self.compact.pop();
                Accumulator::from(u128::from(high) << u64::BITS | u128::from(low))
            }
            _ => {
                let mut value = Accumulator::new();
                for index in (0..count).rev() {
                    value.add_shifted_limbs(u64::from(u64::BITS) * index, [self.compact.pop()]);
                }
                value
            }
        }
    }

    /// Extend the equal-minimum run, merging adjacent runs in constant time.
    pub(super) fn push_equal(&mut self, count: u64) {
        self.top_zeros += count;
    }

    /// Pop a complete record, consuming only the columns selected by its tags.
    pub(super) fn pop(&mut self) -> Option<Entry> {
        if self.top_zeros != 0 {
            return Some(Entry::Equal(core::mem::take(&mut self.top_zeros)));
        }
        if let Some(wide) = self.live_wide.take() {
            return Some(Entry::Positive(Boundary::Wide(wide)));
        }
        if !self.positive.pop()? {
            return Some(Entry::Equal(self.compact.pop()));
        }
        let boundary = if self
            .wide
            .pop()
            .expect("each positive boundary has a width tag")
        {
            let whole = self.whole.pop().expect("each wide boundary has a form tag");
            Boundary::Wide(if whole {
                self.whole_values.pop().expect("each whole tag has a value")
            } else {
                self.pop_copied()
            })
        } else {
            Boundary::Word(self.compact.pop())
        };
        Some(Entry::Positive(boundary))
    }
}
