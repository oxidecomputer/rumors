//! The shortest tick history consistent with a [`Version`].
//!
//! Every leaf height counts ticks reaching that leaf. Ticks shared by both
//! children of an internal node are thereby counted twice, so the node's
//! subtree minimum must be subtracted once. Repeating this correction through
//! the tree gives
//!
//! `minimum ticks = Σ leaf heights − Σ internal-subtree minima`.
//!
//! The fold below evaluates that identity in leaf order. [`minima`] keeps wide
//! heights and nested minima from being copied or scanned again for every later
//! leaf.

use core::cmp::Ordering;

use suanpan::Accumulator;

use crate::accumulator::{self, BigIntAccumulator as _};
use crate::version::io::regions::{RegionReader, VersionRegionReader};
use crate::{Ticks, Version};

use super::HEIGHT_FREEZE_ALLOWANCE_DIGITS;

mod minima;

impl Ticks {
    /// Compute the shortest tick history consistent with `version`.
    pub(crate) fn minimum_for(version: &Version) -> Ticks {
        let (mut leaves, first_height) = VersionRegionReader::open(version);

        // Recent changes stay in one accumulator. Wider, older changes are
        // frozen as shared prefixes, so a narrow next delta never has to copy
        // the absolute height built before it.
        let mut recent_height_change = Accumulator::new();
        let mut answer = Accumulator::new();
        let mut height_prefixes = minima::HeightPrefixes::new(first_height);
        let mut subtree_minima = minima::SubtreeMinima::new();

        // The first leaf lies inside one open subtree for each ancestor on its
        // path. Its live offset is zero because its absolute height opened the
        // frozen-prefix representation.
        subtree_minima.open_subtrees(leaves.depth());
        let first_leaf = height_prefixes.add_leaf(&recent_height_change, &mut answer);
        subtree_minima.observe_leaf(&first_leaf, &mut answer, &mut height_prefixes);

        while !leaves.done() {
            let previous_depth = leaves.depth();
            let (turn_depth, height_change) = leaves.step();

            // Both trackers advance to the next leaf height. The first holds
            // the leaf contribution; the second compares that height with the
            // minima of the subtrees that remain open.
            recent_height_change.add_bigint(&height_change);
            subtree_minima.advance_height(&height_change);

            // The turn keeps the ancestor at `turn_depth` open. It closes the
            // deeper ancestors left behind, then the descent opens any new
            // internal subtrees above the next leaf.
            for _ in 0..previous_depth - turn_depth {
                subtree_minima.close_subtree(&mut answer, &mut height_prefixes);
            }
            subtree_minima.open_subtrees(leaves.depth() - turn_depth);

            // Freeze a much wider accumulated change before the narrow delta
            // that exposed the imbalance can cause later work to revisit it.
            // The allowance absorbs small width fluctuations without changing
            // the asymptotic rule.
            if recent_height_change.stored_digit_count()
                > accumulator::digit_len(height_change.magnitude()) + HEIGHT_FREEZE_ALLOWANCE_DIGITS
            {
                height_prefixes.freeze(&mut recent_height_change);
            }

            let leaf = height_prefixes.add_leaf(&recent_height_change, &mut answer);
            subtree_minima.observe_leaf(&leaf, &mut answer, &mut height_prefixes);
        }

        // The final leaf closes every remaining subtree. Their minimum
        // contributions and the deferred frozen prefixes complete the identity
        // stated in the module documentation.
        subtree_minima.close_all(&mut answer, &mut height_prefixes);
        height_prefixes.settle(&mut answer);
        let (sign, magnitude) = answer.biguint_parts();
        debug_assert_ne!(
            sign,
            Ordering::Less,
            "a subtree minimum never exceeds the sum of its leaves"
        );
        Ticks(magnitude)
    }
}
