//! Subtree minima for [`crate::Version::min_ticks`].
//!
//! A version's minimum tick count is
//!
//! `Σ leaf heights − Σ internal-node subtree minima`.
//!
//! The version stores the first leaf height followed by signed changes between
//! adjacent leaves. A direct implementation would reconstruct every absolute
//! leaf height and retain one absolute minimum per open subtree. A wide early
//! height could then be copied or scanned again for each later one-byte change.
//! These structures keep the same calculation proportional to the input that
//! introduced each value.
//!
//! # Frozen height references
//!
//! [`HeightReferences`] writes the current height as `h_0 + F + L`: `h_0` is
//! the first leaf's height, which the fold counts once, `F` the newest frozen
//! reference above it, and `L` the recent change. Leaves and minima keep only
//! their offset from the reference current when they were observed, and the
//! reference's coefficient defers the `F` part. Its module documentation
//! explains when references freeze and merge, and why `h_0` counts once.
//!
//! # Tracking subtree minima
//!
//! Open subtrees are properly nested. [`SubtreeMinima`] uses [`RangeMinima`] to
//! maintain their minima as differences between adjacent nesting levels, and
//! keeps one record per distinct minimum in a [`Records`] stack: the supplying
//! leaf's offset and the number of subtrees closed at that value. The range
//! tracker reports when each distinct minimum begins and ends, in stack order,
//! so a beginning pushes a record and an ending settles the top one. Most
//! closes merely increment the top record's count; one multiplication settles
//! a whole record.
//!
//! # Why the cost stays linear
//!
//! Every unit of work below is charged to input the event consumes or to a
//! value it destroys, so no surviving wide value is read again for free. Each
//! line names the item whose documentation carries the local argument.
//!
//! - **A height change** is folded once into the live change `L` and once
//!   into the range tracker's running height, each reading only the change.
//! - **A leaf** reads `L` only when `L`'s working width is within
//!   [`UNREAD_FREEZE_BITS`](heights::UNREAD_FREEZE_BITS) of the leaf's
//!   change, and keeps the value only when it is within
//!   [`RECORD_SLACK_BITS`](heights::RECORD_SLACK_BITS); otherwise it freezes.
//!   The read and the addition of the offset into the answer therefore cost
//!   the leaf's own change plus a constant, apart from cancellation the read
//!   compacts once ([`HeightReferences::add_leaf`]).
//! - **A record** is written once when its minimum begins and read once when
//!   it ends; no suspension or resumption moves it ([`Records`]). It holds an
//!   offset no wider than its leaf's change plus the slack, at a cost
//!   proportional to that width ([`IntegerStack`](integers::IntegerStack)).
//! - **A subtree close** increments the top record's count, or ends a minimum
//!   and settles its record by one product with a word-sized count.
//! - **The range tracker** moves boundaries and consumes each crossed one
//!   once ([`RangeMinima`]).
//! - **A freeze** moves `L` into a new reference without reading it again.
//!   Burying the previous top copies a chunk of at most
//!   [`INLINE_CHUNK_BITS`](heights::INLINE_CHUNK_BITS) or moves a wider one
//!   whole, a constant either way.
//! - **A merge** reads only the narrower of two chunks and destroys it, so
//!   the total stored width of live chunks pays for every merge
//!   ([`HeightReferences`]).
//! - **The final settlement** reads the last reference's chunk once.

use suanpan::Accumulator;

use num_bigint::BigInt;

use crate::version::range_minima::{Close, RangeMinima};

mod heights;
mod integers;
mod records;
#[cfg(test)]
mod tests;

#[cfg(test)]
pub use heights::FREEZE_HITS;
pub use heights::{HeightReferences, LeafHeight};
use records::Records;

/// The state the range tracker's callbacks settle records against.
struct Events<'a> {
    /// Live minimum records.
    records: &'a mut Records,
    /// Frozen-height accounting by reference.
    references: &'a mut HeightReferences,
    /// Running min-ticks total.
    total: &'a mut Accumulator,
}

impl Events<'_> {
    /// Settle and remove the innermost distinct minimum's record.
    fn settle_top(&mut self) {
        self.references
            .settle_record(self.records.pop(), self.total);
    }
}

/// Tracks the minimum of every open subtree and the record that settles it.
pub struct SubtreeMinima {
    /// Nested minima, reporting when each distinct minimum begins and ends.
    ranges: RangeMinima,
    /// One record per distinct live minimum, the current minimum on top.
    records: Records,
}

impl SubtreeMinima {
    /// Construct an empty tracker with no open ranges or current minimum.
    pub fn new() -> SubtreeMinima {
        SubtreeMinima {
            ranges: RangeMinima::new(),
            records: Records::new(),
        }
    }

    /// Open `count` ranges: the internal nodes a descent just entered.
    pub fn open_subtrees(&mut self, count: u64) {
        self.ranges.open(count);
    }

    /// Move the running height by one consumed delta.
    pub fn advance_height(&mut self, delta: &BigInt) {
        self.ranges.fold_height(delta);
    }

    /// Close the innermost subtree and subtract its minimum from `total`.
    ///
    /// If the parent has the same minimum, the current record continues. If
    /// the parent has a lower minimum, the current record is settled and the
    /// record below it, the parent's, becomes current. Closing the final
    /// subtree settles the final record.
    pub fn close_subtree(&mut self, total: &mut Accumulator, references: &mut HeightReferences) {
        // Charge this subtree to the leaf that supplied its minimum before
        // that record continues or is settled.
        self.records.increment();
        match self.ranges.close() {
            Close::Equal => {}
            Close::Retired | Close::Lower => {
                references.settle_record(self.records.pop(), total);
            }
        }
    }

    /// Let one leaf update the minima of every subtree containing it.
    ///
    /// The range tracker settles the record of every distinct minimum the leaf
    /// ends, innermost first, and the leaf's own record goes on top if it
    /// begins a new innermost minimum. Only a leaf that arms ranges or truly
    /// undercuts the innermost minimum, which an amortized sign read decides,
    /// does more than O(1) work.
    pub fn observe_leaf(
        &mut self,
        leaf: &LeafHeight,
        total: &mut Accumulator,
        references: &mut HeightReferences,
    ) {
        let mut events = Events {
            records: &mut self.records,
            references: &mut *references,
            total,
        };
        let begins = if self.ranges.has_pending() {
            self.ranges.arm_at_height(&mut events, Events::settle_top)
        } else if self.ranges.armed() && self.ranges.undercuts_here() {
            self.ranges.undercut(&mut events, Events::settle_top);
            true
        } else {
            // The leaf lies at or above the innermost minimum, or the stream
            // is a single leaf with no subtree.
            false
        };
        if begins {
            let first = references.reference_top();
            self.records.push(leaf.sign(), leaf.limbs(), first);
        }
    }

    /// Close every remaining range at the stream's end.
    pub fn close_all(&mut self, total: &mut Accumulator, references: &mut HeightReferences) {
        debug_assert!(
            !self.ranges.has_pending(),
            "the final leaf armed every open range"
        );
        while self.ranges.armed() {
            self.close_subtree(total, references);
        }
    }
}
