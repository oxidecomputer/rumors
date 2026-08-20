//! Run a [`Schedule<T>`] against a fresh fleet of peers and a
//! spec-shaped oracle.
//!
//! One private core runs the full event alphabet — membership events
//! included — over a slot-per-peer fleet (a retired peer vacates its
//! slot). [`execute_with`], [`execute`], and [`execute_and_quiesce`]
//! are the membership-free entry points: they validate the alphabet and
//! return the fleet as a plain `Vec`. [`execute_membership`] and
//! [`execute_membership_and_quiesce`] expose the slotted result for
//! membership schedules.

use std::collections::BTreeMap;

use rumors::{Retire, Version};

use super::events::{Event, EventIdx, Schedule};
use crate::common::oracle::Oracle;
use crate::common::peer::{Peer, gossip_step, quiesce, quiesce_slots};
use crate::common::window::WindowAssignment;
use crate::common::wire::{LINK_BUF, assert_control_drained, block_on, bootstrap_fork_with_window};

use serde::Serialize;
use serde::de::DeserializeOwned;
pub struct ExecutionResult<T> {
    pub peers: Vec<Peer<T>>,
    pub oracle: Oracle<T>,
    /// For each `Insert` event, the [`Version`] minted at the originating
    /// peer.
    pub resolved_versions: BTreeMap<EventIdx, Version>,
}

/// What executing a membership schedule leaves behind: the fleet as
/// slots (a retired peer's slot is `None`), every retiree's complete
/// observation log, and the same oracle and version map as the
/// membership-free result.
pub struct MembershipExecutionResult<T> {
    /// One slot per peer ever minted — the initial fleet, then every
    /// mid-schedule bootstrap in order. `None` marks a retired peer.
    pub slots: Vec<Option<Peer<T>>>,
    /// Each retired peer's observation log, complete as of the drain
    /// that preceded its retirement.
    pub retired_observations: BTreeMap<usize, Vec<(Version, T)>>,
    pub oracle: Oracle<T>,
    /// For each `Insert` event, the [`Version`] minted at the originating
    /// peer.
    pub resolved_versions: BTreeMap<EventIdx, Version>,
}

impl<T> MembershipExecutionResult<T> {
    /// The live peers, with their fleet indices.
    pub fn live(&self) -> impl Iterator<Item = (usize, &Peer<T>)> {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(i, slot)| slot.as_ref().map(|peer| (i, peer)))
    }
}

/// Run the schedule against a fresh `Vec<Peer<T>>` and a fresh
/// `Oracle<T>`, allowing every gossip event.
///
/// Peer `i`'s reconciliation
/// window is `windows.choice(i)` — the suites sweep this dimension, so
/// the fleet mixes floor, budgeted, and default windows (and sessions
/// between differently-configured endpoints).
pub fn execute<T>(schedule: &Schedule<T>, windows: &WindowAssignment) -> ExecutionResult<T>
where
    T: Clone + Ord + Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
{
    execute_with(schedule, windows, |_, _, _| true)
}

/// Run the schedule and then drive every peer to a full-mesh fixed
/// point. After this returns, every `peers[i].local` should hold the
/// same live content as every other and match the oracle's projection.
pub fn execute_and_quiesce<T>(
    schedule: &Schedule<T>,
    windows: &WindowAssignment,
) -> ExecutionResult<T>
where
    T: Clone + Eq + Ord + Serialize + DeserializeOwned + Send + Sync + 'static,
{
    let mut result = execute(schedule, windows);
    quiesce(&mut result.peers);
    result
}

/// Run a membership-free schedule with a caller-supplied gossip filter.
///
/// `allow_gossip(a, b, event_idx)` returns whether the gossip event
/// at `event_idx` between peers `a` and `b` should actually fire; a
/// `false` return turns it into a no-op for the purposes of this
/// execution.
///
/// When gossip is suppressed, the schedule's *valid-by-construction*
/// guarantee for `Redact` events no longer holds: a `Redact` whose
/// target the peer has not yet observed in this run is silently
/// skipped (and the oracle does not record it), which models real
/// usage — application code can only `redact()` a [`Version`] it has
/// been handed.
///
/// # Panics
///
/// Panics if the schedule carries membership events: this entry point
/// promises a fleet with every peer present at its original index, so
/// membership schedules must run through [`execute_membership`]. The
/// alphabet is validated up front — a bootstrap-only schedule would
/// otherwise return an oversized fleet instead of the promised panic.
pub fn execute_with<T, F>(
    schedule: &Schedule<T>,
    windows: &WindowAssignment,
    allow_gossip: F,
) -> ExecutionResult<T>
where
    T: Clone + Ord + Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
    F: Fn(usize, usize, EventIdx) -> bool,
{
    assert!(
        !schedule
            .events
            .iter()
            .any(|e| matches!(e, Event::Bootstrap { .. } | Event::Retire { .. })),
        "membership schedules run through execute_membership"
    );
    let result = execute_slots(schedule, windows, allow_gossip);
    let peers = result
        .slots
        .into_iter()
        .map(|slot| slot.expect("a membership-free schedule retires nobody"))
        .collect();
    ExecutionResult {
        peers,
        oracle: result.oracle,
        resolved_versions: result.resolved_versions,
    }
}

/// Run a membership schedule, allowing every gossip event.
pub fn execute_membership<T>(
    schedule: &Schedule<T>,
    windows: &WindowAssignment,
) -> MembershipExecutionResult<T>
where
    T: Clone + Ord + Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
{
    execute_slots(schedule, windows, |_, _, _| true)
}

/// Run a membership schedule, then drive the surviving peers to a
/// full-mesh fixed point.
pub fn execute_membership_and_quiesce<T>(
    schedule: &Schedule<T>,
    windows: &WindowAssignment,
) -> MembershipExecutionResult<T>
where
    T: Clone + Eq + Ord + Serialize + DeserializeOwned + Send + Sync + 'static,
{
    let mut result = execute_membership(schedule, windows);
    quiesce_slots(&mut result.slots);
    result
}

/// The execution core: run any schedule over a slotted fleet with a
/// gossip filter.
///
/// The fleet starts along the schedule's fork tree: peer 0 is the
/// universe seed, and every other initial peer is a genuine
/// party-disjoint fork of an already-built peer (`fork_parents[i] < i`),
/// minted by serving it a bootstrap. Mid-schedule `Bootstrap` events
/// append newcomer slots the same way; `Retire` events run a real
/// retirement session over a clean in-memory link (the absorber side is
/// plain gossip) and vacate the retiree's slot, saving its observation
/// log. All peers descend from one seed and live parties stay pairwise
/// disjoint, so every session can always merge.
fn execute_slots<T, F>(
    schedule: &Schedule<T>,
    windows: &WindowAssignment,
    allow_gossip: F,
) -> MembershipExecutionResult<T>
where
    T: Clone + Ord + Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
    F: Fn(usize, usize, EventIdx) -> bool,
{
    let mut slots: Vec<Option<Peer<T>>> = Vec::with_capacity(schedule.n_peers);
    for i in 0..schedule.n_peers {
        let local = if i == 0 {
            windows.choice(0).apply(rumors::Peer::seed()).into_rumors()
        } else {
            let parent = slots[schedule.fork_parents[i]]
                .as_ref()
                .expect("initial peers are all alive during fleet construction");
            bootstrap_fork_with_window(&parent.local, windows.choice(i))
        };
        slots.push(Some(Peer::new(local)));
    }
    let mut retired_observations: BTreeMap<usize, Vec<(Version, T)>> = BTreeMap::new();
    let mut oracle = Oracle::<T>::default();
    let mut resolved_versions: BTreeMap<EventIdx, Version> = BTreeMap::new();

    for (i, event) in schedule.events.iter().enumerate() {
        match event {
            Event::Insert { peer, value } => {
                let peer = slots[*peer].as_mut().expect("insert names an alive peer");
                let version = peer.insert_one(value.clone());
                resolved_versions.insert(i, version);
                oracle.insert(i, value.clone());
            }
            Event::Redact {
                peer,
                target_event_idx,
            } => {
                let version = &resolved_versions[target_event_idx];
                let peer = slots[*peer].as_mut().expect("redact names an alive peer");
                let observed_locally = peer.observations.iter().any(|(v, _)| v == version);
                if observed_locally {
                    peer.redact_one(version);
                    oracle.redact(*target_event_idx);
                }
                // else: under a gossip filter, this peer may not yet
                // have observed the version. Real application code
                // couldn't issue this redact, so skip it.
            }
            Event::Gossip { a, b } => {
                if !allow_gossip(*a, *b, i) {
                    continue;
                }
                let (lo, hi) = if a < b { (*a, *b) } else { (*b, *a) };
                let (left, right) = slots.split_at_mut(hi);
                gossip_step(
                    left[lo].as_mut().expect("gossip names alive peers"),
                    right[0].as_mut().expect("gossip names alive peers"),
                );
            }
            Event::Bootstrap { parent, newcomer } => {
                assert_eq!(
                    *newcomer,
                    slots.len(),
                    "bootstrap events mint indices in order"
                );
                let parent = slots[*parent]
                    .as_ref()
                    .expect("bootstrap names an alive parent");
                let local = bootstrap_fork_with_window(&parent.local, windows.choice(*newcomer));
                slots.push(Some(Peer::new(local)));
            }
            Event::Retire { retiree, absorber } => {
                let mut retiring = slots[*retiree]
                    .take()
                    .expect("retire names an alive retiree");
                // Complete the lifetime log before the peer is consumed:
                // nothing after this point can be observed by it.
                retiring.drain();
                let absorbing = slots[*absorber]
                    .as_mut()
                    .expect("retire names an alive absorber");
                let retiring_peer = block_on(retiring.local.try_into_peer())
                    .expect("the executor holds the only handle");
                let (mut link_r, mut link_a) = rumors::link::memory_with_capacity(LINK_BUF);
                let (outcome, absorbed) = block_on(async {
                    tokio::join!(
                        retiring_peer.retire(&mut link_r),
                        absorbing.local.gossip(&mut link_a),
                    )
                });
                absorbed.expect("clean-wire absorber gossip");
                match outcome {
                    Retire::Retired => {}
                    Retire::Recovered { .. } => panic!("clean-wire retirement recovered"),
                    Retire::Uncertain { .. } => panic!("clean-wire retirement uncertain"),
                    Retire::Declined { .. } => {
                        panic!("the absorber runs plain gossip and never declines")
                    }
                }
                absorbing.drain();
                assert_control_drained(link_r, link_a);
                retired_observations.insert(*retiree, retiring.observations);
            }
        }
    }

    MembershipExecutionResult {
        slots,
        retired_observations,
        oracle,
        resolved_versions,
    }
}
