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
//! [`HeightReferences`] writes the current height as `F + L`: `F` is the newest
//! frozen reference and `L` the recent change. Leaves and minima keep only
//! their offset from the reference current when they were observed, and the
//! reference's coefficient defers the `F` part. Its module documentation
//! explains when references freeze and merge.
//!
//! # Tracking subtree minima
//!
//! Open subtrees are properly nested. [`SubtreeMinima`] uses [`RangeMinima`] to
//! maintain their minima as differences between adjacent nesting levels, and
//! keeps one record per distinct minimum in a [`Records`] stack: the supplying
//! leaf's offset and the number of subtrees closed at that value. The
//! range tracker only reports when a minimum is suspended, resumed, or
//! retired, and those events arrive in stack order: a suspension pushes the
//! new minimum above the current one, and every retirement or resumption
//! settles the top record. Most closes merely increment the top record's
//! count; one multiplication settles a whole record.
//!
//! # Why the cost remains bounded
//!
//! Each stored height change is folded into one live accumulator. A record
//! is written once and read once, when it is settled, and no suspension or
//! resumption moves it. A subtree close increments a count or moves one
//! minimum boundary. A new low value permanently consumes each boundary it
//! crosses. Reference merges read only the narrower operand. No later
//! shallow subtree or narrow height change repeatedly traverses an older wide
//! value.

use suanpan::Accumulator;

use num_bigint::BigInt;

use crate::version::range_minima::{Close, RangeMinima};

mod heights;
mod records;
#[cfg(test)]
mod tests;

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
    /// Whether arming suspended or replaced the current minimum, so the
    /// arming leaf becomes the new top record.
    new_minimum: bool,
}

impl Events<'_> {
    /// Settle and remove the current minimum's record.
    fn settle_top(&mut self) {
        let popped = self.records.pop();
        self.references
            .settle_record(&popped.offset, popped.closes, popped.first, self.total);
    }
}

/// Tracks the minimum of every open subtree and the record that settles it.
pub struct SubtreeMinima {
    /// Nested minima; the range tracker carries no client payload.
    ranges: RangeMinima<()>,
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
            Close::Retired | Close::Lower(()) => {
                let popped = self.records.pop();
                references.settle_record(&popped.offset, popped.closes, popped.first, total);
            }
        }
    }

    /// Let one leaf update the minima of every subtree containing it.
    ///
    /// Arms any pending ranges at the leaf; otherwise an amortized sign read
    /// decides whether the leaf undercuts the innermost minimum, and only a
    /// true undercut does more than O(1) work. It consumes each crossed
    /// boundary and settles that boundary's record once.
    pub fn observe_leaf(
        &mut self,
        leaf: &LeafHeight,
        total: &mut Accumulator,
        references: &mut HeightReferences,
    ) {
        if self.ranges.has_pending() {
            if !self.ranges.armed() {
                // The first leaf establishes both the minimum and its record;
                // first arming invokes no callback.
                self.push(leaf, references);
                self.ranges.arm_at_height(&mut (), |_| (), |(), _| {});
                return;
            }
            // A higher inner minimum suspends the current record beneath a
            // new one. An equal minimum keeps it current. A lower minimum
            // settles every record it displaces, current first, before the
            // leaf's own record is pushed.
            let mut events = Events {
                records: &mut self.records,
                references: &mut *references,
                total,
                new_minimum: false,
            };
            self.ranges.arm_at_height(
                &mut events,
                |events| events.new_minimum = true,
                |(), events| events.settle_top(),
            );
            let new_minimum = events.new_minimum;
            if new_minimum {
                self.push(leaf, references);
            }
            return;
        }
        if !self.ranges.armed() {
            // A single-leaf stream: no node will ever fold a minimum.
            return;
        }
        if !self.ranges.undercuts_here() {
            return;
        }
        // The new leaf supplies the minimum. Settle the displaced record,
        // then every outer record whose boundary the drop crosses, then
        // push the leaf's record.
        let mut events = Events {
            records: &mut self.records,
            references: &mut *references,
            total,
            new_minimum: true,
        };
        events.settle_top();
        self.ranges
            .undercut(&mut events, |(), events| events.settle_top());
        self.push(leaf, references);
    }

    /// Push the leaf's record as the current minimum.
    fn push(&mut self, leaf: &LeafHeight, references: &mut HeightReferences) {
        let first = references.reference_top();
        self.records.push(leaf.offset(), first);
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
