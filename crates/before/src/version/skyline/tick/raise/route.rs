//! Selects the owned region raised by a tick.

use num_bigint::BigUint;

use crate::codec::{BitsBuf, BitsView};

use super::Raise;

/// The cost of raising a region: fewer expansions, then a shallower location.
///
/// Field order gives the derived ordering its lexicographic meaning.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Cost {
    /// Leaf-to-node expansions along the raise path.
    pub(in crate::version::skyline::tick) expansions: u64,
    /// The selected terminal's depth below the walked root.
    pub(in crate::version::skyline::tick) depth: u64,
}

impl Cost {
    /// The component value for a region that cannot be raised.
    pub(crate) const INFEASIBLE: u64 = u64::MAX;

    /// The largest feasible component value.
    ///
    /// Keeping this below [`INFEASIBLE`](Self::INFEASIBLE) ensures that a deep
    /// feasible path cannot become indistinguishable from an unowned region.
    pub(crate) const CEILING: u64 = Self::INFEASIBLE - 1;

    /// Add one path level, saturating feasible values at `ceiling`.
    ///
    /// The explicit ceiling lets tests exercise saturation at constructible
    /// depths; production passes [`CEILING`](Self::CEILING).
    pub(crate) fn deepen(component: u64, ceiling: u64) -> u64 {
        if component >= ceiling {
            component
        } else {
            component + 1
        }
    }

    /// Encode one component for compact storage on a [`crate::codec::PopStack`].
    ///
    /// Zero denotes infeasibility; feasible values are shifted by one. The
    /// shift cannot overflow because feasible components saturate below the
    /// sentinel.
    pub(in crate::version::skyline::tick) fn encode_component(component: u64) -> u64 {
        if component == Self::INFEASIBLE {
            0
        } else {
            component + 1
        }
    }

    /// Decode a component stored by [`encode_component`](Self::encode_component).
    pub(in crate::version::skyline::tick) fn decode_component(encoded: u64) -> u64 {
        if encoded == 0 {
            Self::INFEASIBLE
        } else {
            encoded - 1
        }
    }

    /// The cost of an unowned region.
    pub(in crate::version::skyline::tick) const MAX: Cost = Cost {
        expansions: Self::INFEASIBLE,
        depth: Self::INFEASIBLE,
    };

    /// The zero cost of raising an owned terminal in place.
    pub(in crate::version::skyline::tick) const FREE: Cost = Cost {
        expansions: 0,
        depth: 0,
    };
}

static_assertions::const_assert!(Cost::CEILING < Cost::INFEASIBLE);

/// The cheapest raise route, keyed by party branch position.
///
/// The deciding walk visits branches outside the final route, so a linear
/// direction list would not align with replay. One bit at each branch's source
/// position permits constant-time lookup while the raise descends.
pub(in crate::version::skyline::tick) struct Route {
    /// Whether the route selects the left child at each party bit position.
    dirs: BitsBuf,
}

impl Route {
    /// Create an empty route covering every party bit position.
    pub(in crate::version::skyline::tick) fn new(party_span: u64) -> Self {
        Self {
            dirs: BitsBuf::repeat(false, party_span),
        }
    }

    /// Record the chosen child at the party branch beginning at `key`.
    pub(in crate::version::skyline::tick) fn record(&mut self, key: u64, left: bool) {
        self.dirs.set(key, left);
    }

    /// Whether the route selects the left child at `key`.
    pub(super) fn descends_left(&self, key: u64) -> bool {
        self.dirs.get(key)
    }

    /// Apply this route and register `ticks` events at its selected region.
    pub(in crate::version::skyline::tick) fn apply(
        &self,
        version: BitsView<'_>,
        party: BitsView<'_>,
        ticks: &BigUint,
    ) -> BitsBuf {
        debug_assert!(!party.is_empty(), "raising requires an owning party");
        debug_assert!(ticks.bits() != 0, "raising registers at least one event");
        Raise::new(version, party).run(self, ticks)
    }

    /// The raw direction bits for comparison with the recursive oracle.
    #[cfg(test)]
    pub(in crate::version::skyline::tick) fn dirs(&self) -> &BitsBuf {
        &self.dirs
    }
}
