//! The two-party walk that computes a region difference.
//!
//! Each cursor describes a region containing the current position. When their
//! depths differ, the deeper region lies inside the shallower one and ends no
//! later. The walk crosses that boundary and advances every region that ends
//! there. This lets it emit the first party's ownership outside the second
//! without storing arbitrary-precision interval coordinates.
//!
//! A uniform region can also decide a whole subtree: an unowned second region
//! preserves the first subtree, an owned second region removes it, and an
//! unowned first region makes the second subtree irrelevant. Those subtrees
//! are scanned once as blocks instead of walking each leaf through the merge.

use crate::codec::BitsBuf;
use crate::party::ops::build::RegionBuilder;
use crate::party::tree::PartyCursor;
use crate::version::skyline::overlay::{self, PlateauCursor};

use super::cursor::{Entered, Item, RegionCursor};

/// One forward walk computing the region owned by `a` but not `b`.
pub(super) struct Difference<'a, 'b> {
    /// Ownership to retain except where the second party overlaps it.
    a: RegionCursor<'a>,
    /// Ownership to remove.
    b: RegionCursor<'b>,
    /// Canonical output built from the consecutive surviving regions.
    out: RegionBuilder,
}

impl<'a, 'b> Difference<'a, 'b> {
    /// Open the operands and resolve their roots.
    pub(super) fn new(a: PartyCursor<'a>, b: PartyCursor<'b>) -> Self {
        let capacity = a.bits().len() + b.bits().len();
        let a = RegionCursor::open(a);
        let b = RegionCursor::open(b);
        let mut this = Self {
            a,
            b,
            out: RegionBuilder::with_capacity(capacity),
        };
        this.descend_pair();
        this
    }

    /// Consume both partitions and return their canonical difference.
    pub(super) fn finish(mut self) -> BitsBuf {
        loop {
            match self.a.item() {
                Item::Copy { start } => {
                    debug_assert!(
                        self.b.depth() <= self.a.depth() && !self.b.owned(),
                        "a copied subtree is covered by an unowned region"
                    );
                    self.out
                        .subtree(self.a.depth(), self.a.bits(), start, self.a.position());
                }
                Item::Region { owned } => self
                    .out
                    .leaf(self.a.depth().max(self.b.depth()), owned && !self.b.owned()),
            }
            if self.a.done() && self.b.done() {
                return self.out.finish();
            }
            self.advance();
        }
    }

    /// Cross the next boundary, then resolve any subtree entered there.
    fn advance(&mut self) {
        match overlay::advance(&mut self.a, &mut self.b, |_| {}) {
            (Some(a_entered), Some(b_entered)) => self.resolve_pair(a_entered, b_entered),
            (Some(true), None) => self.resolve_a(),
            (None, Some(true)) => self.resolve_b(),
            (Some(false), None) | (None, Some(false)) => {}
            (None, None) => unreachable!("the overlay advances at least one cursor"),
        }
    }

    /// Resolve an `a` subtree covered by `b`'s current region.
    fn resolve_a(&mut self) {
        self.a.skip_subtree(!self.b.owned());
    }

    /// Resolve a `b` subtree covered by `a`'s current region.
    fn resolve_b(&mut self) {
        if self.a.owned() {
            self.b.descend();
        } else {
            self.b.skip_subtree(false);
        }
    }

    /// Resolve the two positions reached at the same depth.
    fn resolve_pair(&mut self, a_entered: bool, b_entered: bool) {
        match (a_entered, b_entered) {
            (false, false) => {}
            (true, false) => self.resolve_a(),
            (false, true) => self.resolve_b(),
            (true, true) => self.descend_pair(),
        }
    }

    /// Descend matching subtrees until one side reaches a covering region.
    fn descend_pair(&mut self) {
        loop {
            match (self.a.enter(), self.b.enter()) {
                (Entered::Child, Entered::Child) => continue,
                (Entered::Child, Entered::Unowned) => return self.a.skip_subtree(true),
                (Entered::Child, Entered::Owned) => return self.a.skip_subtree(false),
                (Entered::Unowned, Entered::Child) => return self.b.skip_subtree(false),
                (Entered::Owned, Entered::Child) => return self.b.descend(),
                (Entered::Owned, Entered::Owned)
                | (Entered::Owned, Entered::Unowned)
                | (Entered::Unowned, Entered::Owned)
                | (Entered::Unowned, Entered::Unowned) => return,
            }
        }
    }
}
