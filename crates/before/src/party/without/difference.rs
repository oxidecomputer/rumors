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

use crate::party::io::writer::PartyRegionWriter;
use crate::party::io::PartyReader;
use crate::party::Party;
use crate::version::io::regions::RegionReader as _;
use crate::version::overlay;

use super::reader::{Boundary, DifferenceReader, Entered, SubtreeDisposition};

/// One forward walk computing the region owned by `a` but not `b`.
pub struct Difference<'a, 'b> {
    /// Ownership to retain except where the second party overlaps it.
    a: DifferenceReader<'a>,
    /// Ownership to remove.
    b: DifferenceReader<'b>,
    /// Canonical output built from the consecutive surviving regions.
    out: PartyRegionWriter,
}

impl<'a, 'b> Difference<'a, 'b> {
    /// Return the canonical region owned by `a` but not `b`.
    pub fn between(a: PartyReader<'a>, b: PartyReader<'b>) -> Option<Party> {
        let out = PartyRegionWriter::for_difference(&a, &b);
        let a = DifferenceReader::open(a);
        let b = DifferenceReader::open(b);
        let mut difference = Self { a, b, out };
        difference.descend_pair();
        difference.resolve()
    }

    /// Consume both partitions and return their canonical difference.
    fn resolve(mut self) -> Option<Party> {
        loop {
            if let Some(subtree) = self.a.take_retained_subtree() {
                debug_assert!(
                    self.b.depth() <= self.a.depth() && !self.b.owned(),
                    "a copied subtree is covered by an unowned region"
                );
                self.out.subtree(self.a.depth(), subtree);
            } else {
                self.out.leaf(
                    self.a.depth().max(self.b.depth()),
                    self.a.owned() && !self.b.owned(),
                );
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
            (Some(Boundary::Subtree), None) => self.resolve_a(),
            (None, Some(Boundary::Subtree)) => self.resolve_b(),
            (Some(Boundary::Region), None) | (None, Some(Boundary::Region)) => {}
            (None, None) => unreachable!("the overlay advances at least one cursor"),
        }
    }

    /// Resolve an `a` subtree covered by `b`'s current region.
    fn resolve_a(&mut self) {
        let disposition = if self.b.owned() {
            SubtreeDisposition::Discard
        } else {
            SubtreeDisposition::Retain
        };
        self.a.skip_subtree(disposition);
    }

    /// Resolve a `b` subtree covered by `a`'s current region.
    fn resolve_b(&mut self) {
        if self.a.owned() {
            self.b.descend();
        } else {
            self.b.skip_subtree(SubtreeDisposition::Discard);
        }
    }

    /// Resolve the two positions reached at the same depth.
    fn resolve_pair(&mut self, a_entered: Boundary, b_entered: Boundary) {
        match (a_entered, b_entered) {
            (Boundary::Region, Boundary::Region) => {}
            (Boundary::Subtree, Boundary::Region) => self.resolve_a(),
            (Boundary::Region, Boundary::Subtree) => self.resolve_b(),
            (Boundary::Subtree, Boundary::Subtree) => self.descend_pair(),
        }
    }

    /// Descend matching subtrees until one side reaches a covering region.
    fn descend_pair(&mut self) {
        loop {
            match (self.a.enter(), self.b.enter()) {
                (Entered::Child, Entered::Child) => continue,
                (Entered::Child, Entered::Unowned) => {
                    return self.a.skip_subtree(SubtreeDisposition::Retain);
                }
                (Entered::Child, Entered::Owned) => {
                    return self.a.skip_subtree(SubtreeDisposition::Discard);
                }
                (Entered::Unowned, Entered::Child) => {
                    return self.b.skip_subtree(SubtreeDisposition::Discard);
                }
                (Entered::Owned, Entered::Child) => return self.b.descend(),
                (Entered::Owned, Entered::Owned)
                | (Entered::Owned, Entered::Unowned)
                | (Entered::Unowned, Entered::Owned)
                | (Entered::Unowned, Entered::Unowned) => return,
            }
        }
    }
}
