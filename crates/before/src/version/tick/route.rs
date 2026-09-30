//! Selects the owned region raised by a tick.

use num_bigint::BigUint;

use crate::bits::BitsWriter;
use crate::party::io::BranchChoice;
use crate::Version;

use super::raise::Raise;

/// The cost of raising a region: fewer expansions, then a shallower location.
///
/// Field order gives the derived ordering its lexicographic meaning.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Cost {
    /// Leaf-to-node expansions along the raise path.
    pub expansions: u64,
    /// The selected terminal's depth below the walked root.
    pub depth: u64,
}

impl Cost {
    /// The component value for a region that cannot be raised.
    pub const INFEASIBLE: u64 = u64::MAX;

    /// The largest feasible component value.
    ///
    /// Keeping this below [`INFEASIBLE`](Self::INFEASIBLE) ensures that a deep
    /// feasible path cannot become indistinguishable from an unowned region.
    pub const CEILING: u64 = Self::INFEASIBLE - 1;

    /// Add one path level, saturating feasible values at `ceiling`.
    ///
    /// Supplying the ceiling makes the saturation boundary reachable at small
    /// values while production uses [`CEILING`](Self::CEILING).
    pub fn deepen(component: u64, ceiling: u64) -> u64 {
        if component >= ceiling {
            component
        } else {
            component + 1
        }
    }

    /// Encode one component for compact storage on a [`crate::bits::stack::PackedU64Stack`].
    ///
    /// Zero denotes infeasibility; feasible values are shifted by one. The
    /// shift cannot overflow because feasible components saturate below the
    /// sentinel.
    pub fn encode_component(component: u64) -> u64 {
        if component == Self::INFEASIBLE {
            0
        } else {
            component + 1
        }
    }

    /// Decode a component stored by [`encode_component`](Self::encode_component).
    pub fn decode_component(encoded: u64) -> u64 {
        if encoded == 0 {
            Self::INFEASIBLE
        } else {
            encoded - 1
        }
    }

    /// The cost of an unowned region.
    pub const MAX: Cost = Cost {
        expansions: Self::INFEASIBLE,
        depth: Self::INFEASIBLE,
    };

    /// The zero cost of raising an owned terminal in place.
    pub const FREE: Cost = Cost {
        expansions: 0,
        depth: 0,
    };

    /// Select the cheaper child, choosing right on a tie.
    pub fn prefer(left: Cost, right: Cost) -> (BranchChoice, Cost) {
        if left < right {
            (BranchChoice::Left, left)
        } else {
            (BranchChoice::Right, right)
        }
    }

    /// Select the smaller path component, choosing right on a tie.
    pub fn prefer_component(left: u64, right: u64) -> (BranchChoice, u64) {
        if left < right {
            (BranchChoice::Left, left)
        } else {
            (BranchChoice::Right, right)
        }
    }
}

static_assertions::const_assert!(Cost::CEILING < Cost::INFEASIBLE);

/// The cheapest raise route, keyed by party branch position.
///
/// The deciding walk visits branches outside the final route, so a linear
/// direction list would not align with replay. One bit at each branch's source
/// position permits constant-time lookup while the raise descends.
pub struct Route {
    /// Selected child at each relevant party position.
    dirs: BranchDirections,
}

/// Compact branch choices used while planning and replaying a tick.
///
/// A set bit selects the left child; a clear bit selects the right child.
/// Route planning addresses choices by Party position, while expansion paths
/// append them in traversal order. This wrapper keeps that representation out
/// of the tree algorithms.
pub struct BranchDirections(BitsWriter);

impl BranchDirections {
    /// An empty sequential path.
    pub fn new() -> Self {
        Self(BitsWriter::new())
    }

    /// One default-right entry for every possible Party position.
    fn for_party_span(len: u64) -> Self {
        Self(BitsWriter::repeat(false, len))
    }

    /// Append one traversal choice.
    pub fn push(&mut self, choice: BranchChoice) {
        self.0.push(choice == BranchChoice::Left);
    }

    /// Number of stored choices.
    pub fn len(&self) -> u64 {
        self.0.len()
    }

    /// Read the choice at `position`.
    pub fn choice(&self, position: u64) -> BranchChoice {
        if self.0.bit(position) {
            BranchChoice::Left
        } else {
            BranchChoice::Right
        }
    }

    /// Replace the choice at `position`.
    fn set(&mut self, position: u64, choice: BranchChoice) {
        self.0.patch_bit(position, choice == BranchChoice::Left);
    }
}

impl Route {
    /// Create an empty route covering every party bit position.
    pub fn new(party_span: u64) -> Self {
        Self {
            dirs: BranchDirections::for_party_span(party_span),
        }
    }

    /// Record the chosen child at the party branch beginning at `key`.
    pub fn record(&mut self, key: u64, choice: BranchChoice) {
        self.dirs.set(key, choice);
    }

    /// The child selected at `key`.
    pub fn choice(&self, key: u64) -> BranchChoice {
        self.dirs.choice(key)
    }

    /// Apply this route and register `ticks` events at its selected region.
    pub fn apply(&self, version: &Version, party: &crate::Party, ticks: &BigUint) -> Version {
        debug_assert!(ticks.bits() != 0, "raising registers at least one event");
        Raise::apply(version, party, self, ticks)
    }

    /// Borrow the recorded direction bits.
    #[cfg(test)]
    pub fn dirs(&self) -> &BranchDirections {
        &self.dirs
    }
}
