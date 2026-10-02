//! Chooses the cheapest owned region to raise if simplification changes nothing.

use crate::bits::stack::{BitStack, PackedU64Stack};
use crate::party::io::{PartyBranch, PartyNode, PartyReader};

use super::frames::Position;
use super::route::{Cost, Distance, Route};

/// The cheapest raise route computed alongside simplification.
pub struct RaiseProbe {
    /// The recorded directions; allocated only if a branch is selected.
    route: Option<Route>,
    /// The party stream's bit length, used to size a branchless route.
    party_span: u64,
    /// Whether the tick still needs a raise route.
    live: bool,
}

impl RaiseProbe {
    /// Start route selection over a party of `party_span` bits.
    pub fn new(party_span: u64) -> Self {
        Self {
            route: None,
            party_span,
            live: true,
        }
    }

    /// Stop probing once simplification has produced the tick result.
    pub fn kill(&mut self) {
        self.live = false;
        self.route = None;
    }

    /// Combine two child costs and record the cheaper child.
    ///
    /// A branch adds one level of depth. Equal costs choose the right child so
    /// route selection is deterministic.
    pub fn join(&mut self, key: u64, left: Cost, right: Cost) -> Cost {
        if !self.live {
            return Cost::INFEASIBLE;
        }
        let (choice, cheaper) = Cost::prefer(left, right);
        self.route().record(key, choice);
        cheaper.descend(Cost::CEILING)
    }

    /// Price expanding one version leaf across the party branch at `key`.
    ///
    /// Each party level below the leaf requires one version expansion. This
    /// method scans both present party subtrees, records the path to the nearest
    /// owned terminal, and leaves `party` just past them.
    pub fn expand(&mut self, key: u64, party: &mut PartyReader, branch: PartyBranch) -> Cost {
        if !self.live {
            if branch.has_left_child() {
                party.skip();
            }
            if branch.has_right_child() {
                party.skip();
            }
            return Cost::INFEASIBLE;
        }
        let left_cost = if branch.has_left_child() {
            self.expand_subtree(party, Cost::CEILING)
        } else {
            Cost::INFEASIBLE
        };
        let right_cost = if branch.has_right_child() {
            self.expand_subtree(party, Cost::CEILING)
        } else {
            Cost::INFEASIBLE
        };
        let (choice, cheaper) = Cost::prefer(left_cost, right_cost);
        self.route().record(key, choice);
        cheaper.expand(Cost::CEILING)
    }

    /// Finish route selection.
    ///
    /// A party containing no branch needs no direction, but returning an empty
    /// route keeps the application path uniform.
    pub fn take_route(&mut self) -> Route {
        debug_assert!(self.live, "only an unchanged walk needs a raise route");
        self.route
            .take()
            .unwrap_or_else(|| Route::new(self.party_span))
    }

    /// Allocate the route on its first recorded direction.
    fn route(&mut self) -> &mut Route {
        self.route
            .get_or_insert_with(|| Route::new(self.party_span))
    }

    /// Find the nearest owned terminal in one party subtree.
    ///
    /// The traversal is an iterative post-order fold. Each frame retains
    /// whether its right child exists and, after the left child completes, the
    /// left distance. When both distances are known, the nearer child is
    /// recorded and its distance is increased for the parent. [`None`] denotes
    /// an absent child; feasible distances retain a nonzero packed value, so
    /// the two cases cannot collide.
    ///
    /// # Panics
    ///
    /// Panics if `party` is not in normal form. Every branch in a normal party
    /// has at least one present child.
    pub fn expand_subtree(&mut self, party: &mut PartyReader, ceiling: u64) -> Cost {
        // A false phase awaits the left distance; true awaits the right.
        let mut phase = BitStack::new();
        let mut right_present = BitStack::new();
        let mut values = PackedU64Stack::new();
        let mut keys = Position::new();

        loop {
            let key = party.offset();
            let mut distance = match party.read() {
                PartyNode::Owned => Some(Distance::ZERO),
                PartyNode::Branch(branch) => {
                    keys.push(&mut values, key);
                    phase.push(false);
                    right_present.push(branch.has_right_child());
                    if branch.has_left_child() {
                        continue;
                    }
                    None
                }
            };

            // Return completed distances through ancestors until another right
            // child must be visited or the root is complete.
            loop {
                match phase.last() {
                    None => {
                        let distance =
                            distance.expect("an internal node in normal form has a present child");
                        return Cost::expansion_path(distance);
                    }
                    Some(false) => {
                        phase.set_last(true);
                        values.push(distance.map_or(0, Distance::stack_word));
                        if right_present.last().expect("one presence bit per frame") {
                            break;
                        }
                        distance = None;
                    }
                    Some(true) => {
                        phase.pop();
                        right_present.pop();
                        let left_distance = Distance::from_stack_word(values.pop());
                        let right_distance = distance;
                        let key = keys.pop(&mut values);
                        let (choice, nearer) = Distance::prefer(left_distance, right_distance);
                        self.route().record(key, choice);
                        distance = nearer.map(|distance| distance.deepen(ceiling));
                    }
                }
            }
        }
    }
}
