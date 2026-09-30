//! Chooses the cheapest owned region to raise if simplification changes nothing.

use crate::codec::{BitStack, PopStack};
use crate::party::tree::{PartyCursor, PartyNode};

use super::super::frames::Position;
use super::{Cost, Route};

/// The cheapest raise route computed alongside simplification.
pub(in crate::version::skyline::tick) struct RaiseProbe {
    /// The recorded directions; allocated only if a branch is selected.
    route: Option<Route>,
    /// The party stream's bit length, used to size a branchless route.
    party_span: u64,
    /// Whether the tick still needs a raise route.
    live: bool,
}

impl RaiseProbe {
    /// Start route selection over a party of `party_span` bits.
    pub(in crate::version::skyline::tick) fn new(party_span: u64) -> Self {
        Self {
            route: None,
            party_span,
            live: true,
        }
    }

    /// Stop probing once simplification has produced the tick result.
    pub(in crate::version::skyline::tick) fn kill(&mut self) {
        self.live = false;
        self.route = None;
    }

    /// Combine two child costs and record the cheaper child.
    ///
    /// A branch adds one level of depth. Equal costs choose the right child so
    /// route selection is deterministic.
    pub(in crate::version::skyline::tick) fn join(
        &mut self,
        key: u64,
        left: Cost,
        right: Cost,
    ) -> Cost {
        if !self.live {
            return Cost::MAX;
        }
        let chose_left = left < right;
        self.route().record(key, chose_left);
        let cheaper = if chose_left { left } else { right };
        Cost {
            expansions: cheaper.expansions,
            depth: Cost::deepen(cheaper.depth, Cost::CEILING),
        }
    }

    /// Price expanding one version leaf across the party branch at `key`.
    ///
    /// Each party level below the leaf requires one version expansion. This
    /// method scans both present party subtrees, records the path to the nearest
    /// owned terminal, and leaves `party` just past them.
    pub(in crate::version::skyline::tick) fn expand(
        &mut self,
        key: u64,
        party: &mut PartyCursor,
        left: bool,
        right: bool,
    ) -> Cost {
        if !self.live {
            if left {
                party.skip();
            }
            if right {
                party.skip();
            }
            return Cost::MAX;
        }
        let left_cost = if left {
            self.expand_subtree(party, Cost::CEILING)
        } else {
            Cost::MAX
        };
        let right_cost = if right {
            self.expand_subtree(party, Cost::CEILING)
        } else {
            Cost::MAX
        };
        let chose_left = left_cost < right_cost;
        self.route().record(key, chose_left);
        let cheaper = if chose_left { left_cost } else { right_cost };
        Cost {
            expansions: Cost::deepen(cheaper.expansions, Cost::CEILING),
            depth: Cost::deepen(cheaper.depth, Cost::CEILING),
        }
    }

    /// Finish route selection.
    ///
    /// A party containing no branch needs no direction, but returning an empty
    /// route keeps the application path uniform.
    pub(in crate::version::skyline::tick) fn take_route(&mut self) -> Route {
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
    /// recorded and its distance is increased for the parent. Feasible
    /// distances saturate below [`Cost::INFEASIBLE`], so even an extremely deep
    /// path cannot be mistaken for an absent child.
    ///
    /// # Panics
    ///
    /// Panics if `party` is not in normal form. Every branch in a normal party
    /// has at least one present child.
    pub(in crate::version::skyline::tick) fn expand_subtree(
        &mut self,
        party: &mut PartyCursor,
        ceiling: u64,
    ) -> Cost {
        // A false phase awaits the left distance; true awaits the right.
        let mut phase = BitStack::new();
        let mut right_present = BitStack::new();
        let mut values = PopStack::new();
        let mut keys = Position::new();

        loop {
            let key = party.offset();
            let mut distance = match party.read() {
                PartyNode::Owned => 0,
                PartyNode::Branch(branch) => {
                    keys.push(&mut values, key);
                    phase.push(false);
                    right_present.push(branch.has_right());
                    if branch.has_left() {
                        continue;
                    }
                    Cost::INFEASIBLE
                }
            };

            // Return completed distances through ancestors until another right
            // child must be visited or the root is complete.
            loop {
                match phase.last() {
                    None => {
                        assert_ne!(
                            distance,
                            Cost::INFEASIBLE,
                            "an internal node in normal form has a present child"
                        );
                        return Cost {
                            expansions: distance,
                            depth: distance,
                        };
                    }
                    Some(false) => {
                        phase.set_last(true);
                        values.push(Cost::encode_component(distance));
                        if right_present.last().expect("one presence bit per frame") {
                            break;
                        }
                        distance = Cost::INFEASIBLE;
                    }
                    Some(true) => {
                        phase.pop();
                        right_present.pop();
                        let left_distance = Cost::decode_component(values.pop());
                        let right_distance = distance;
                        let key = keys.pop(&mut values);
                        let chose_left = left_distance < right_distance;
                        self.route().record(key, chose_left);
                        let nearer = if chose_left {
                            left_distance
                        } else {
                            right_distance
                        };
                        distance = Cost::deepen(nearer, ceiling);
                    }
                }
            }
        }
    }
}
