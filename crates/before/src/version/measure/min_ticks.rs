//! The shortest tick history consistent with a [`Version`].
//!
//! Every leaf height counts events reaching that leaf. Events shared by both
//! children of an internal node are thereby counted twice, so the node's
//! subtree minimum must be subtracted once. Repeating this correction through
//! the tree gives
//!
//! `minimum ticks = Σ leaf heights − Σ internal-subtree minima`.
//!
//! The fold below evaluates that identity in leaf order. [`minima`] keeps wide
//! heights and nested minima from being copied or scanned again for every later
//! leaf.

use num_bigint::{BigInt, BigUint};
use suanpan::Accumulator;

use crate::version::io::regions::{RegionReader, VersionRegionReader};
use crate::{Count, Version};

use super::HEIGHT_FREEZE_ALLOWANCE_BITS;

mod minima;

#[cfg(test)]
thread_local! {
    /// Freezes the stored-width trigger below fires on this test thread; the
    /// record trigger in [`minima`] counts its own.
    /// Tests use this to establish which inputs reach the freeze path.
    pub static FREEZE_HITS: core::cell::Cell<u64> = const { core::cell::Cell::new(0) };
}

impl Count {
    /// Compute the shortest tick history consistent with `version`.
    pub(crate) fn min_ticks_for(version: &Version) -> Count {
        let (mut leaves, first_height) = VersionRegionReader::open(version);

        // Recent changes stay in one accumulator. Wider, older changes are
        // frozen as shared references, so a narrow next delta never has to
        // copy the absolute height built before it.
        let mut recent_height_change = Accumulator::new();
        let mut answer = Accumulator::new();
        let mut height_references = minima::HeightReferences::new(first_height);
        let mut subtree_minima = minima::SubtreeMinima::new();

        // The first leaf lies inside one open subtree for each ancestor on its
        // path. Its live offset is zero because its absolute height opened the
        // first reference.
        subtree_minima.open_subtrees(leaves.depth());
        let first_leaf =
            height_references.add_leaf(&mut recent_height_change, &BigInt::ZERO, &mut answer);
        subtree_minima.observe_leaf(&first_leaf, &mut answer, &mut height_references);

        while !leaves.done() {
            let previous_depth = leaves.depth();
            let (turn_depth, height_change) = leaves.step();

            // Both trackers advance to the next leaf height. The first holds
            // the leaf contribution; the second compares that height with the
            // minima of the subtrees that remain open.
            recent_height_change += &height_change;
            subtree_minima.advance_height(&height_change);

            // The turn keeps the ancestor at `turn_depth` open. It closes the
            // deeper ancestors left behind, then the descent opens any new
            // internal subtrees above the next leaf.
            for _ in 0..previous_depth - turn_depth {
                subtree_minima.close_subtree(&mut answer, &mut height_references);
            }
            subtree_minima.open_subtrees(leaves.depth() - turn_depth);

            // Freeze a much wider accumulated change before the narrow delta
            // that exposed the imbalance can cause later work to revisit it.
            // The allowance absorbs small width fluctuations without changing
            // the asymptotic rule. This bounds the stored width that reading
            // the change costs; the record trigger in `add_leaf` then bounds
            // the value's width relative to the delta, far more tightly.
            if recent_height_change.stored_bits()
                > height_change.magnitude().bits() + HEIGHT_FREEZE_ALLOWANCE_BITS
            {
                #[cfg(test)]
                FREEZE_HITS.with(|hits| hits.set(hits.get() + 1));
                height_references.freeze(&mut recent_height_change);
            }

            let leaf =
                height_references.add_leaf(&mut recent_height_change, &height_change, &mut answer);
            subtree_minima.observe_leaf(&leaf, &mut answer, &mut height_references);
        }

        // The final leaf closes every remaining subtree. Their minimum records
        // and the references' deferred heights complete the identity stated in
        // the module documentation.
        subtree_minima.close_all(&mut answer, &mut height_references);
        height_references.settle(&mut answer);
        Count(
            BigUint::try_from(&answer)
                .expect("a subtree minimum never exceeds the sum of its leaves"),
        )
    }
}
