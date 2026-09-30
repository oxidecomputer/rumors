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
//! # Frozen height prefixes
//!
//! [`HeightPrefixes`] writes the current height as `F + L`. `L` is the recent
//! change held in one accumulator. When it grows much wider than the next
//! stored change, it becomes another component of `F`. Leaves and minima keep
//! only their small offset from the prefix of components current when they
//! were observed.
//!
//! Each leaf adds its offset immediately and adds `+1` to its prefix's
//! coefficient. Each closed subtree adds the negative coefficient of its
//! minimum. Final settlement reverses the sums:
//!
//! `Σ_p coefficient_p · prefix_p`
//! `= Σ_c component_c · Σ_{p ≥ c} coefficient_p`.
//!
//! Each frozen component is therefore read for one product, regardless of how
//! many later leaves refer to a prefix containing it.
//!
//! # Tracking subtree minima
//!
//! Open subtrees are properly nested. [`SubtreeMinima`] uses [`RangeMinima`] to
//! maintain their minima as differences between adjacent nesting levels. It
//! attaches a contribution to each distinct minimum: the supplying leaf's
//! offset, its frozen prefix, and the number of subtrees closed at that value.
//! Most closes merely increment that count. When the minimum changes or leaves
//! the stack, one multiplication settles the whole contribution.
//!
//! The common contribution fits in one word. Larger offsets, prefix indices,
//! and close counts spill to exact storage without changing the input domain.
//!
//! # Why the cost remains bounded
//!
//! Each stored height change is folded into one live accumulator. A subtree
//! close increments a compact count or moves one minimum boundary. A new low
//! value permanently consumes each boundary it crosses. Freezing visits a live
//! value once, and final settlement visits each frozen component once. No
//! later shallow subtree or narrow height change repeatedly traverses an older
//! wide value.

use suanpan::Accumulator;

use num_bigint::BigInt;

use crate::version::range_minima::{Close, RangeMinima};

mod contributions;
mod heights;

use contributions::{ContributionStore, StoredContribution};
pub use heights::{HeightPrefixes, LeafHeight};

/// Shared state for the callbacks that arm pending subtree ranges.
struct Arming<'a> {
    /// Contribution for the current innermost minimum.
    current: &'a mut Option<StoredContribution>,
    /// Compact and spilled contributions.
    contributions: &'a mut ContributionStore,
    /// Leaf that may become the new minimum.
    leaf: &'a LeafHeight,
    /// Running min-ticks total.
    total: &'a mut Accumulator,
    /// Frozen-height accounting by prefix.
    prefixes: &'a mut HeightPrefixes,
}

impl Arming<'_> {
    /// Make the pending leaf current and return the contribution it displaces.
    fn replace_current(&mut self) -> StoredContribution {
        self.current
            .replace(self.contributions.store(self.leaf))
            .expect("an armed subtree has a current contribution")
    }

    /// Settle a contribution retired while the pending leaf is armed.
    fn settle(&mut self, contribution: StoredContribution) {
        self.contributions
            .settle(contribution, self.total, self.prefixes);
    }
}

/// Shared state for retiring contributions displaced by a lower leaf.
struct Settlement<'a> {
    /// Compact and spilled contributions.
    contributions: &'a mut ContributionStore,
    /// Running min-ticks total.
    total: &'a mut Accumulator,
    /// Frozen-height accounting by prefix.
    prefixes: &'a mut HeightPrefixes,
}

impl Settlement<'_> {
    /// Settle one outer contribution displaced by the new minimum.
    fn settle(&mut self, contribution: StoredContribution) {
        self.contributions
            .settle(contribution, self.total, self.prefixes);
    }
}

/// Tracks the minimum and deferred contribution of every open subtree.
pub struct SubtreeMinima {
    /// Nested minima; a positive boundary carries the outer contribution.
    ranges: RangeMinima<StoredContribution>,
    /// Contribution for the innermost minimum, while a range is armed.
    current_contribution: Option<StoredContribution>,
    /// Compact storage, with exact spill slots for uncommon values.
    contributions: ContributionStore,
}

impl SubtreeMinima {
    /// Construct an empty tracker with no open ranges or current minimum.
    pub fn new() -> SubtreeMinima {
        SubtreeMinima {
            ranges: RangeMinima::new(),
            current_contribution: None,
            contributions: ContributionStore::new(),
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
    /// If the parent has the same minimum, its current contribution continues. If
    /// the parent has a lower minimum, its suspended contribution resumes and
    /// the inner contribution is settled. Closing the final subtree settles
    /// the final contribution.
    pub fn close_subtree(&mut self, total: &mut Accumulator, prefixes: &mut HeightPrefixes) {
        // Charge this subtree to the leaf that supplied its minimum before
        // that contribution continues, settles, or is replaced.
        match self.ranges.close() {
            Close::Equal => {
                self.contributions.increment(
                    self.current_contribution
                        .as_mut()
                        .expect("an armed subtree has a current contribution"),
                );
            }
            Close::Retired => {
                let mut contribution = self
                    .current_contribution
                    .take()
                    .expect("an armed subtree has a current contribution");
                self.contributions.increment(&mut contribution);
                self.contributions.settle(contribution, total, prefixes);
            }
            Close::Lower(parent) => {
                let mut completed = self
                    .current_contribution
                    .replace(parent)
                    .expect("an armed subtree has a current contribution");
                self.contributions.increment(&mut completed);
                self.contributions.settle(completed, total, prefixes);
            }
        }
    }

    /// Let one leaf update the minima of every subtree containing it.
    ///
    /// Arms any pending ranges at the leaf; otherwise an amortized sign read
    /// decides whether the leaf undercuts the innermost minimum, and only a
    /// true undercut does more than O(1) work. It consumes each crossed
    /// boundary and settles that boundary's contribution once.
    pub fn observe_leaf(
        &mut self,
        leaf: &LeafHeight,
        total: &mut Accumulator,
        prefixes: &mut HeightPrefixes,
    ) {
        if self.ranges.has_pending() {
            if !self.ranges.armed() {
                // The first leaf establishes both the minimum and its
                // accounting contribution.
                self.current_contribution = Some(self.contributions.store(leaf));
            }
            // A higher inner minimum suspends the outer contribution on their
            // boundary. An equal minimum keeps it current. A lower minimum
            // settles every contribution that it displaces.
            let mut context = Arming {
                current: &mut self.current_contribution,
                contributions: &mut self.contributions,
                leaf,
                total,
                prefixes,
            };
            self.ranges.arm_at_height(
                &mut context,
                Arming::replace_current,
                |contribution, context| context.settle(contribution),
            );
            return;
        }
        if !self.ranges.armed() {
            // A single-leaf stream: no node will ever fold a minimum.
            return;
        }
        if !self.ranges.undercuts_here() {
            return;
        }
        // The new leaf supplies the minimum. Settle the displaced contribution,
        // then every outer contribution whose boundary the drop crosses.
        let displaced = self
            .current_contribution
            .take()
            .expect("an armed subtree has a current contribution");
        self.contributions.settle(displaced, total, prefixes);
        self.current_contribution = Some(self.contributions.store(leaf));
        let mut context = Settlement {
            contributions: &mut self.contributions,
            total,
            prefixes,
        };
        self.ranges.undercut(&mut context, |contribution, context| {
            context.settle(contribution);
        });
    }

    /// Close every remaining range at the stream's end.
    pub fn close_all(&mut self, total: &mut Accumulator, prefixes: &mut HeightPrefixes) {
        debug_assert!(
            !self.ranges.has_pending(),
            "the final leaf armed every open range"
        );
        while self.ranges.armed() {
            self.close_subtree(total, prefixes);
        }
    }
}
