//! Chooses the owned region raised by a tick.
//!
//! A tick may be able to raise several owned regions. The selection walk
//! prefers the route requiring fewer leaf expansions, then the shallower
//! route, and finally the right child when both costs are equal.
//!
//! An unowned subtree has no feasible route. [`Cost`] represents that absence
//! directly rather than assigning it a numeric value. Its two feasible
//! counters are stored one greater than their logical values so that zero can
//! retain absence when a suspended cost is written to the traversal stack.

use core::cmp::Ordering;
use core::num::NonZeroU64;

use num_bigint::BigUint;

use crate::bits::BitsWriter;
use crate::party::io::BranchChoice;
use crate::Version;

use super::raise::Raise;

/// The two nonzero counters of a feasible route.
///
/// Storing `n + 1` leaves zero available for an absent route when the value is
/// packed. It also gives [`Option<FeasibleCost>`] a niche, so making
/// infeasibility explicit adds no space to a live [`Cost`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct FeasibleCost {
    /// Leaf-to-node expansions along the raise path, plus one.
    expansions: Distance,
    /// The selected terminal's depth below the walked root, plus one.
    depth: Distance,
}

/// One feasible path length in the stack's nonzero representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Distance(NonZeroU64);

impl Distance {
    /// A zero-length path, encoded as one.
    pub const ZERO: Distance = Distance(NonZeroU64::MIN);

    /// Shift an unencoded feasible component into its nonzero packed form.
    #[cfg(test)]
    fn from_unshifted(component: u64) -> Distance {
        Distance(
            NonZeroU64::new(
                component
                    .checked_add(1)
                    .expect("a feasible cost fits below `u64::MAX`"),
            )
            .expect("adding one makes a feasible component nonzero"),
        )
    }

    /// Read one stack word; zero means that no path exists.
    pub fn from_stack_word(word: u64) -> Option<Distance> {
        NonZeroU64::new(word).map(Distance)
    }

    /// Return the nonzero word retained by the traversal stack.
    pub fn stack_word(self) -> u64 {
        self.0.get()
    }

    /// Add one level, saturating the shifted value for a feasible path.
    pub fn deepen(self, ceiling: u64) -> Distance {
        let encoded_ceiling = ceiling
            .checked_add(1)
            .expect("the shifted distance ceiling must fit `u64`");
        if self.0.get() >= encoded_ceiling {
            self
        } else {
            Distance(
                NonZeroU64::new(self.0.get() + 1)
                    .expect("deepening a nonzero distance remains nonzero"),
            )
        }
    }

    /// Select the shorter feasible path, choosing right on a tie.
    pub fn prefer(
        left: Option<Distance>,
        right: Option<Distance>,
    ) -> (BranchChoice, Option<Distance>) {
        match (left, right) {
            (Some(left), Some(right)) if left < right => (BranchChoice::Left, Some(left)),
            (Some(_), Some(right)) => (BranchChoice::Right, Some(right)),
            (Some(left), None) => (BranchChoice::Left, Some(left)),
            (None, right) => (BranchChoice::Right, right),
        }
    }
}

/// The cost of raising a region, or no route when the region is unowned.
///
/// Feasible costs compare lexicographically by expansions and then depth. An
/// infeasible route compares greater than every feasible one, so taking the
/// minimum naturally ignores absent children.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cost(Option<FeasibleCost>);

impl Cost {
    /// The largest feasible component value.
    ///
    /// Feasible costs saturate here because their packed form stores `n + 1`.
    pub const CEILING: u64 = u64::MAX - 1;

    /// The cost of an unowned region.
    pub const INFEASIBLE: Cost = Cost(None);

    /// The zero cost of raising an owned terminal in place.
    pub const FREE: Cost = Cost(Some(FeasibleCost {
        expansions: Distance::ZERO,
        depth: Distance::ZERO,
    }));

    /// Construct a feasible cost from its unshifted components.
    #[cfg(test)]
    pub fn feasible(expansions: u64, depth: u64) -> Cost {
        Cost(Some(FeasibleCost {
            expansions: Distance::from_unshifted(expansions),
            depth: Distance::from_unshifted(depth),
        }))
    }

    /// Whether this cost describes an owned route.
    #[cfg(test)]
    pub fn is_feasible(self) -> bool {
        self.0.is_some()
    }

    /// Account for descending through one existing branch.
    pub fn descend(self, ceiling: u64) -> Cost {
        Cost(self.0.map(|cost| FeasibleCost {
            expansions: cost.expansions,
            depth: cost.depth.deepen(ceiling),
        }))
    }

    /// Account for expanding one leaf into a branch and descending through it.
    pub fn expand(self, ceiling: u64) -> Cost {
        Cost(self.0.map(|cost| FeasibleCost {
            expansions: cost.expansions.deepen(ceiling),
            depth: cost.depth.deepen(ceiling),
        }))
    }

    /// Build the equal-component cost of a path made entirely of expansions.
    pub(super) fn expansion_path(distance: Distance) -> Cost {
        Cost(Some(FeasibleCost {
            expansions: distance,
            depth: distance,
        }))
    }

    /// Return the two words retained by the traversal stack.
    pub fn stack_words(self) -> (u64, u64) {
        self.0.map_or((0, 0), |cost| {
            (cost.expansions.stack_word(), cost.depth.stack_word())
        })
    }

    /// Restore a cost from the words returned by [`stack_words`](Self::stack_words).
    pub fn from_stack_words(expansions: u64, depth: u64) -> Cost {
        match (
            Distance::from_stack_word(expansions),
            Distance::from_stack_word(depth),
        ) {
            (None, None) => Self::INFEASIBLE,
            (Some(expansions), Some(depth)) => Cost(Some(FeasibleCost { expansions, depth })),
            _ => panic!("a packed route cost is either wholly feasible or wholly absent"),
        }
    }

    /// Select the cheaper child, choosing right on a tie.
    pub fn prefer(left: Cost, right: Cost) -> (BranchChoice, Cost) {
        if left < right {
            (BranchChoice::Left, left)
        } else {
            (BranchChoice::Right, right)
        }
    }
}

impl Ord for Cost {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.0, other.0) {
            (Some(left), Some(right)) => left.cmp(&right),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        }
    }
}

impl PartialOrd for Cost {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

static_assertions::const_assert_eq!(
    core::mem::size_of::<Cost>(),
    2 * core::mem::size_of::<u64>()
);

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
