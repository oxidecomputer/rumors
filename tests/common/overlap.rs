//! Controlled interleavings of gossip sessions with local and remote changes.
//!
//! [`Session::step`] polls each endpoint once per round. A test can pause after
//! any round, make changes through other handles, then finish the session.
//! This exposes the gap between taking a snapshot and publishing its result.
//!
//! [`execute_overlap`] runs these operations as a schedule. Its generator uses
//! [`Knowledge`] to track which messages each peer can redact and when sessions
//! take their snapshots after the preamble exchange ([`FORK_ROUNDS`]). The
//! executor still checks the live observation log before redacting; the
//! shadow-validity suite checks the model against the actual observations.

use std::collections::BTreeMap;
use std::fmt::Debug;
use std::future::{Future, poll_fn};
use std::ops::RangeInclusive;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use proptest::collection::vec;
use proptest::prelude::*;
use rumors::link::memory_with_capacity;
use rumors::{Rumors, Version};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::common::oracle::Oracle;
use crate::common::peer::{Peer, gossip_step, quiesce};
use crate::common::schedule::EventIdx;
use crate::common::wire::{block_on, bootstrap_fork};

/// Small stream buffers make greeting and data writes yield between fragments.
const OVERLAP_LINK_BUF: usize = 48;

/// One session whose endpoints can be polled separately to choose an interleaving.
///
/// Each endpoint owns its link and a cloned replica handle. Other handles can
/// mutate either replica while it is parked; dropping it cancels the session.
/// Each endpoint checks its own result. The closed links are not retained for
/// a final drain check.
pub struct Session {
    /// Endpoint futures, polled in A-then-B order.
    sides: [Pin<Box<dyn Future<Output = ()>>>; 2],
    /// Completed endpoints must not be polled again.
    done: [bool; 2],
}

/// Open a wire gossip session between `a` and `b` without polling it.
pub fn open<T>(a: &Rumors<T>, b: &Rumors<T>) -> Session
where
    T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
{
    let a = a.clone();
    let b = b.clone();
    let (a_link, b_link) = memory_with_capacity(OVERLAP_LINK_BUF);
    Session {
        sides: [
            Box::pin(async move {
                let mut a_link = a_link;
                a.gossip_once(&mut a_link)
                    .await
                    .expect("overlapped session side A");
            }),
            Box::pin(async move {
                let mut b_link = b_link;
                b.gossip_once(&mut b_link)
                    .await
                    .expect("overlapped session side B");
            }),
        ],
        done: [false, false],
    }
}

/// Pause at a chosen round or complete the remaining exchange.
impl Session {
    /// Poll each unfinished endpoint once, preserving A-then-B order.
    fn poll_round(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        for side in 0..2 {
            if !self.done[side] && self.sides[side].as_mut().poll(cx).is_ready() {
                self.done[side] = true;
            }
        }
        if self.done == [true, true] {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }

    /// Run at most `n` rounds, returning whether both endpoints completed.
    ///
    /// Stepping deliberately polls with a no-op waker: the caller chooses the
    /// next round even if no endpoint wakes. [`Self::finish`] restores normal
    /// wakeup-driven progress and detects a stalled session.
    pub fn step(&mut self, n: usize) -> bool {
        let mut cx = Context::from_waker(Waker::noop());
        for _ in 0..n {
            if self.poll_round(&mut cx).is_ready() {
                return true;
            }
        }
        self.done == [true, true]
    }

    /// Complete the session, counting additional rounds for the parking sweep.
    /// Panics if unfinished endpoints are pending with no further wakeup.
    pub fn finish(mut self) -> usize {
        if self.done == [true, true] {
            return 0;
        }
        let mut rounds = 0;
        block_on(poll_fn(|cx| {
            rounds += 1;
            self.poll_round(cx)
        }));
        rounds
    }
}

/// The overlap alphabet: [`schedule`](super::schedule)'s events plus
/// hand-driven sessions in a small slot space.
#[derive(Debug, Clone)]
pub enum OverlapEvent<T> {
    /// Insert `value` at `peer`.
    Insert { peer: usize, value: T },
    /// Redact the message created by the `Insert` at `target_event_idx`.
    /// Valid by construction: the generator's shadow guarantees `peer`
    /// has observed that message when this event runs.
    Redact {
        peer: usize,
        target_event_idx: EventIdx,
    },
    /// One whole (non-overlapped) session between `a` and `b`, as the
    /// serial executor runs them.
    Gossip { a: usize, b: usize },
    /// Open a session between `a` and `b` in `slot` without polling it;
    /// it installs at its `Close`, or at a `Step` that completes it.
    ///
    /// Nothing forks here: each side forks its view under the polls that
    /// complete its preamble exchange ([`FORK_ROUNDS`]), and the shadow
    /// models exactly that.
    Open { slot: usize, a: usize, b: usize },
    /// Poll the session in `slot` at most `polls` times.
    Step { slot: usize, polls: usize },
    /// Drive the session in `slot` to completion and install.
    Close { slot: usize },
}

#[derive(Debug, Clone)]
pub struct OverlapSchedule<T> {
    pub n_peers: usize,
    /// Fork topology, as in [`schedule::events::Schedule`]: peer 0 seeds,
    /// `fork_parents[i] < i`, so the fleet is pairwise disjoint.
    pub fork_parents: Vec<usize>,
    pub events: Vec<OverlapEvent<T>>,
}

/// Run an overlap schedule against a fresh fleet, close every session
/// still open (in ascending slot order), and quiesce the fleet to a
/// full-mesh fixed point.
///
/// Returns the fleet and the spec-shaped oracle; the caller asserts the
/// two agree.
pub fn execute_overlap_and_quiesce<T>(schedule: &OverlapSchedule<T>) -> (Vec<Peer<T>>, Oracle<T>)
where
    T: Clone + Eq + Ord + Serialize + DeserializeOwned + Send + Sync + 'static,
{
    let OverlapRun {
        mut peers, oracle, ..
    } = execute_overlap(schedule);
    quiesce(&mut peers);
    (peers, oracle)
}

/// The fleet after an overlap schedule has run, before any quiescence.
pub struct OverlapRun<T: Send + Sync + 'static> {
    /// The peers in stable fleet order.
    pub peers: Vec<Peer<T>>,
    /// The specification state after the same schedule.
    pub oracle: Oracle<T>,
    /// The [`Version`] each `Insert` event created, by event index.
    pub resolved_versions: BTreeMap<EventIdx, Version>,
}

/// Run an overlap schedule against a fresh fleet and close every session
/// still open (in ascending slot order), leaving the fleet as the schedule
/// left it.
///
/// No quiescence: the observation logs and live sets are those the
/// schedule alone produced.
pub fn execute_overlap<T>(schedule: &OverlapSchedule<T>) -> OverlapRun<T>
where
    T: Clone + Eq + Ord + Serialize + DeserializeOwned + Send + Sync + 'static,
{
    let mut peers: Vec<Peer<T>> = Vec::with_capacity(schedule.n_peers);
    for i in 0..schedule.n_peers {
        let local = if i == 0 {
            rumors::Peer::seed().sync_window_floor().into_rumors()
        } else {
            bootstrap_fork(&peers[schedule.fork_parents[i]].local)
        };
        peers.push(Peer::new(local));
    }
    let mut oracle = Oracle::<T>::default();
    let mut resolved_versions: BTreeMap<EventIdx, Version> = BTreeMap::new();
    // Open sessions, keyed by slot, with their endpoints retained so both
    // observation logs can drain when the session closes.
    let mut open_sessions: BTreeMap<usize, (Session, usize, usize)> = BTreeMap::new();

    for (i, event) in schedule.events.iter().enumerate() {
        match event {
            OverlapEvent::Insert { peer, value } => {
                let version = peers[*peer].insert_one(value.clone());
                resolved_versions.insert(i, version);
                oracle.insert(i, value.clone());
            }
            OverlapEvent::Redact {
                peer,
                target_event_idx,
            } => {
                let version = &resolved_versions[target_event_idx];
                // The shadow's premise is that each side forks where the
                // live session does, so the target is observed here; if
                // the model has drifted from the protocol, skip the event
                // on both sides of the comparison rather than issue a
                // `redact` the live peer could never have made. The
                // shadow-validity meta-test is what makes such drift fail.
                let observed = peers[*peer].observations.iter().any(|(v, _)| v == version);
                if observed {
                    peers[*peer].redact_one(version);
                    oracle.redact(*target_event_idx);
                }
            }
            OverlapEvent::Gossip { a, b } => {
                let (lo, hi) = if a < b { (*a, *b) } else { (*b, *a) };
                let (left, right) = peers.split_at_mut(hi);
                gossip_step(&mut left[lo], &mut right[0]);
            }
            OverlapEvent::Open { slot, a, b } => {
                assert!(
                    !open_sessions.contains_key(slot),
                    "generator invariant: slot {slot} is vacant at Open"
                );
                let session = open(&peers[*a].local, &peers[*b].local);
                open_sessions.insert(*slot, (session, *a, *b));
            }
            OverlapEvent::Step { slot, polls } => {
                // A step can complete a short session outright; its
                // install has then already landed, so the endpoints
                // drain now and the slot frees (the matching `Close`
                // becomes a no-op). Waiting for the `Close` would let an
                // intervening `insert_one` see the session's content in
                // its own drain.
                let completed = open_sessions
                    .get_mut(slot)
                    .is_some_and(|(session, _, _)| session.step(*polls));
                if completed {
                    let (_, a, b) = open_sessions.remove(slot).expect("slot occupied");
                    peers[a].drain();
                    peers[b].drain();
                }
            }
            OverlapEvent::Close { slot } => {
                if let Some((session, a, b)) = open_sessions.remove(slot) {
                    session.finish();
                    peers[a].drain();
                    peers[b].drain();
                }
            }
        }
    }

    for (_, (session, a, b)) in open_sessions.into_iter() {
        session.finish();
        peers[a].drain();
        peers[b].drain();
    }
    OverlapRun {
        peers,
        oracle,
        resolved_versions,
    }
}

/// How many session slots a generated schedule may hold open at once.
/// Two suffices to overlap a session with a whole other session; a third
/// lets overlaps themselves overlap.
const SLOTS: usize = 3;

/// Polling rounds after which each side of an open session has forked
/// its working state, indexed like [`Session`]'s sides (`a`, then `b`).
///
/// A side forks once its preamble exchange completes. [`Session::step`]
/// polls `a` then `b` each round: in the first round `a` writes its
/// preamble and parks on the read, then `b` writes its own, reads `a`'s,
/// and forks; `a` reads `b`'s preamble and forks in the second round. A
/// `Close` polls to completion, so it forks whichever side has not.
const FORK_ROUNDS: [usize; 2] = [2, 1];

/// Generate local actions and overlapping sessions from a populated, converged fleet.
///
/// Insert [`SessionPair`]s into arbitrary actions so both session orderings
/// receive regular coverage, including cases where one session starts with
/// equal snapshots and another changes the replica before it publishes.
pub fn arb_overlap_schedule<T, S>(
    value_strategy: S,
    n_peers_range: RangeInclusive<usize>,
    max_events: usize,
) -> impl Strategy<Value = OverlapSchedule<T>>
where
    T: Clone + Debug + 'static,
    S: Strategy<Value = T> + Clone + 'static,
{
    arb_overlap_schedule_with_shadow(value_strategy, n_peers_range, max_events)
        .prop_map(|(schedule, _shadow)| schedule)
}

/// Also return the model's final state for comparison with the live executor.
pub fn arb_overlap_schedule_with_shadow<T, S>(
    value_strategy: S,
    n_peers_range: RangeInclusive<usize>,
    max_events: usize,
) -> impl Strategy<Value = (OverlapSchedule<T>, Knowledge)>
where
    T: Clone + Debug + 'static,
    S: Strategy<Value = T> + Clone + 'static,
{
    n_peers_range.prop_flat_map(move |n_peers| {
        (
            vec(any::<usize>(), n_peers.saturating_sub(1)),
            // Vary the shared base across small and multi-chunk root fans.
            (12usize..=48),
            vec(value_strategy.clone(), 48),
            vec(arb_choice(value_strategy.clone(), n_peers), 0..=max_events),
            vec(
                SessionPair::arbitrary(value_strategy.clone(), n_peers),
                1..=2,
            ),
        )
            .prop_map(
                move |(raw_parents, preamble_len, preamble_values, mut choices, pairs)| {
                    let fork_parents = fork_tree(n_peers, &raw_parents);
                    // Insert each pair relative to the sequence built so far.
                    for pair in &pairs {
                        let at = pair.offset % (choices.len() + 1);
                        choices.splice(at..at, pair.choices(n_peers));
                    }
                    build_overlap_schedule(
                        n_peers,
                        fork_parents,
                        &preamble_values[..preamble_len],
                        choices,
                    )
                },
            )
    })
}

/// Fold raw entropy into a valid fork tree (as
/// [`schedule::arb`](super::schedule::arb) does).
fn fork_tree(n_peers: usize, raw: &[usize]) -> Vec<usize> {
    (0..n_peers)
        .map(|i| if i == 0 { 0 } else { raw[i - 1] % i })
        .collect()
}

/// Abstract action the strategy emits; concrete events are derived by
/// the shadow in [`build_overlap_schedule`].
#[derive(Debug, Clone)]
enum Choice<T> {
    Insert {
        peer: usize,
        value: T,
    },
    /// Redact the `idx % len`-th entry of the peer's shadow observation
    /// log; dropped if the log is empty.
    RedactObservation {
        peer: usize,
        idx: usize,
    },
    Gossip {
        a: usize,
        b: usize,
    },
    /// Open a session in `slot % SLOTS`; dropped if that slot is
    /// occupied or `a == b`.
    Open {
        slot: usize,
        a: usize,
        b: usize,
    },
    /// Poll the session in `slot % SLOTS`; dropped if vacant.
    Step {
        slot: usize,
        polls: usize,
    },
    /// Close the session in `slot % SLOTS`; dropped if vacant.
    Close {
        slot: usize,
    },
}

fn arb_choice<T, S>(value_strategy: S, n_peers: usize) -> impl Strategy<Value = Choice<T>>
where
    T: Clone + Debug + 'static,
    S: Strategy<Value = T> + Clone + 'static,
{
    prop_oneof![
        3 => (0..n_peers, value_strategy)
            .prop_map(|(peer, value)| Choice::Insert { peer, value }),
        3 => (0..n_peers, any::<usize>())
            .prop_map(|(peer, idx)| Choice::RedactObservation { peer, idx }),
        2 => (0..n_peers, 0..n_peers)
            .prop_map(|(a, b)| Choice::Gossip { a, b }),
        3 => (any::<usize>(), 0..n_peers, 0..n_peers)
            .prop_map(|(slot, a, b)| Choice::Open { slot, a, b }),
        2 => (any::<usize>(), 0usize..=12)
            .prop_map(|(slot, polls)| Choice::Step { slot, polls }),
        3 => any::<usize>().prop_map(|slot| Choice::Close { slot }),
    ]
}

/// Two sessions sharing a replica, with a change at one of the remote peers.
///
/// One session pauses while the other completes. Vary which one is paused
/// to exercise a redaction or send arriving before or after the other result.
#[derive(Debug, Clone)]
struct SessionPair<T> {
    /// Where to insert this pair into the surrounding action sequence.
    offset: usize,
    /// Raw choice of the replica shared by both sessions.
    shared: usize,
    /// Raw choice of the peer synchronized before the mutation.
    unchanged: usize,
    /// Raw choice of the peer that sends or redacts a message.
    changed: usize,
    /// Slot used for the paused session.
    slot: usize,
    /// Polling rounds to run before pausing.
    park: usize,
    /// Whether to pause the session with the changed peer.
    changed_first: bool,
    /// A value to send, or an index into the changed peer's observations to redact.
    mutation: Result<T, usize>,
}

/// Generate and expand a pair of sessions sharing one replica.
impl<T: Clone + Debug + 'static> SessionPair<T> {
    /// Draw both orderings, a mutation, and positions within the surrounding schedule.
    fn arbitrary<S>(value_strategy: S, n_peers: usize) -> impl Strategy<Value = Self>
    where
        S: Strategy<Value = T> + Clone + 'static,
    {
        (
            any::<usize>(),
            0..n_peers,
            0..n_peers,
            0..n_peers,
            any::<usize>(),
            1usize..=12,
            any::<bool>(),
            prop_oneof![value_strategy.prop_map(Ok), any::<usize>().prop_map(Err)],
        )
            .prop_map(
                |(offset, shared, unchanged, changed, slot, park, changed_first, mutation)| Self {
                    offset,
                    shared,
                    unchanged,
                    changed,
                    slot,
                    park,
                    changed_first,
                    mutation,
                },
            )
    }

    /// Expand into actions on distinct peers; smaller fleets cannot host the pair.
    fn choices(&self, n_peers: usize) -> Vec<Choice<T>> {
        if n_peers < 3 {
            return Vec::new();
        }
        let shared = self.shared % n_peers;
        let unchanged = (shared + 1 + (self.unchanged % (n_peers - 1))) % n_peers;
        let mut changed = self.changed % n_peers;
        while changed == shared || changed == unchanged {
            changed = (changed + 1) % n_peers;
        }
        let (first, second) = if self.changed_first {
            (changed, unchanged)
        } else {
            (unchanged, changed)
        };
        vec![
            Choice::Gossip {
                a: shared,
                b: unchanged,
            },
            match &self.mutation {
                Ok(value) => Choice::Insert {
                    peer: changed,
                    value: value.clone(),
                },
                Err(idx) => Choice::RedactObservation {
                    peer: changed,
                    idx: *idx,
                },
            },
            Choice::Open {
                slot: self.slot,
                a: shared,
                b: first,
            },
            Choice::Step {
                slot: self.slot,
                polls: self.park,
            },
            Choice::Gossip {
                a: shared,
                b: second,
            },
            Choice::Close { slot: self.slot },
        ]
    }
}

/// An open session in the model: its endpoints, the polling rounds it
/// has received, and each side's view at the round it forked
/// ([`FORK_ROUNDS`]), taken once that round is reached.
struct OpenSession {
    a: usize,
    b: usize,
    rounds: usize,
    forks: [Option<Knowledge>; 2],
}

impl OpenSession {
    /// Advance the session by `rounds` polling rounds, forking any side
    /// whose round is reached at the current `sim`.
    fn poll(&mut self, rounds: usize, sim: &Knowledge) {
        self.rounds += rounds;
        for (side, fork) in self.forks.iter_mut().enumerate() {
            if fork.is_none() && self.rounds >= FORK_ROUNDS[side] {
                *fork = Some(sim.clone());
            }
        }
    }

    /// Drive the session to completion (as a `Close` does), forking any
    /// side that has not, and deliver it into `sim`.
    fn close(mut self, sim: &mut Knowledge) {
        self.poll(FORK_ROUNDS[0].max(FORK_ROUNDS[1]), sim);
        let [fork_a, fork_b] = self.forks;
        sim.merge_session(
            &fork_a.expect("closed sessions have forked"),
            &fork_b.expect("closed sessions have forked"),
            self.a,
            self.b,
        );
    }
}

/// Per-peer knowledge sets, as in `schedule::arb`'s shadow: everything
/// the peer has ever held, the subset currently live, and the exact
/// observation order.
#[derive(Clone, Debug)]
pub struct Knowledge {
    /// Per-peer set of `EventIdx`s whose message the peer has ever held.
    pub ever_known: Vec<std::collections::BTreeSet<EventIdx>>,
    /// Per-peer set of `EventIdx`s the model predicts the peer holds live.
    pub live: Vec<std::collections::BTreeSet<EventIdx>>,
    /// Per-peer sequence of `EventIdx`s the model predicts the peer's
    /// observation log would have appended.
    pub observed_log: Vec<Vec<EventIdx>>,
}

impl Knowledge {
    fn new(n_peers: usize) -> Self {
        Self {
            ever_known: vec![Default::default(); n_peers],
            live: vec![Default::default(); n_peers],
            observed_log: vec![Vec::new(); n_peers],
        }
    }

    /// Merge what a session delivers between `a` and `b` into the
    /// *current* state, given each side's view at its fork (`fork_a` for
    /// `a`, `fork_b` for `b`).
    ///
    /// The session carries each side's fork-time content only: messages
    /// one fork-time side held live propagate to a counterparty that has
    /// never known them; messages either fork-time side had redacted die
    /// on both current sides (deletion honoring, tombstone-free). A
    /// message redacted *after* the fork stays dead locally --
    /// `ever_known` guards resurrection -- and its counterparty learns
    /// that deletion only from a later session, exactly as the wire
    /// behaves.
    fn merge_session(&mut self, fork_a: &Knowledge, fork_b: &Knowledge, a: usize, b: usize) {
        let combined: std::collections::BTreeSet<EventIdx> = fork_a.ever_known[a]
            .union(&fork_b.ever_known[b])
            .copied()
            .collect();
        for k in combined {
            let a_had = fork_a.ever_known[a].contains(&k);
            let b_had = fork_b.ever_known[b].contains(&k);
            let redacted_at_fork =
                (a_had && !fork_a.live[a].contains(&k)) || (b_had && !fork_b.live[b].contains(&k));
            if redacted_at_fork {
                for p in [a, b] {
                    self.ever_known[p].insert(k);
                    self.live[p].remove(&k);
                }
            } else {
                for p in [a, b] {
                    if !self.ever_known[p].contains(&k) {
                        self.ever_known[p].insert(k);
                        self.live[p].insert(k);
                        self.observed_log[p].push(k);
                    }
                }
            }
        }
    }
}

/// Build an overlap schedule by driving the shadow in lockstep with the
/// emitted events, each open session modeled by its sides' views at the
/// rounds they fork, returning the schedule with the shadow's final
/// state.
///
/// Every emitted `Redact` names a message its peer holds in the model,
/// and the model forks where the live session does.
fn build_overlap_schedule<T: Clone>(
    n_peers: usize,
    fork_parents: Vec<usize>,
    preamble: &[T],
    choices: Vec<Choice<T>>,
) -> (OverlapSchedule<T>, Knowledge) {
    let mut sim = Knowledge::new(n_peers);
    let mut open: BTreeMap<usize, OpenSession> = BTreeMap::new();
    let mut events: Vec<OverlapEvent<T>> = Vec::new();

    // Converged preamble: populate the seed peer, then one sequential
    // full-mesh round (each pair in order shares everything learned so
    // far, so a single round converges static content).
    for value in preamble {
        let idx = events.len();
        sim.ever_known[0].insert(idx);
        sim.live[0].insert(idx);
        sim.observed_log[0].push(idx);
        events.push(OverlapEvent::Insert {
            peer: 0,
            value: value.clone(),
        });
    }
    for a in 0..n_peers {
        for b in (a + 1)..n_peers {
            let frozen = sim.clone();
            sim.merge_session(&frozen, &frozen, a, b);
            events.push(OverlapEvent::Gossip { a, b });
        }
    }

    for choice in choices {
        let next_event_idx = events.len();
        match choice {
            Choice::Insert { peer, value } => {
                sim.ever_known[peer].insert(next_event_idx);
                sim.live[peer].insert(next_event_idx);
                sim.observed_log[peer].push(next_event_idx);
                events.push(OverlapEvent::Insert { peer, value });
            }
            Choice::RedactObservation { peer, idx } => {
                let log = &sim.observed_log[peer];
                if log.is_empty() {
                    continue;
                }
                let target_event_idx = log[idx % log.len()];
                // Only messages still live locally are sensible targets;
                // a second redact of the same message is a no-op the
                // executor would skip asymmetrically.
                if !sim.live[peer].contains(&target_event_idx) {
                    continue;
                }
                sim.live[peer].remove(&target_event_idx);
                events.push(OverlapEvent::Redact {
                    peer,
                    target_event_idx,
                });
            }
            Choice::Gossip { a, b } => {
                if a == b {
                    continue;
                }
                let frozen = sim.clone();
                sim.merge_session(&frozen, &frozen, a, b);
                events.push(OverlapEvent::Gossip { a, b });
            }
            Choice::Open { slot, a, b } => {
                let slot = slot % SLOTS;
                if a == b || open.contains_key(&slot) {
                    continue;
                }
                open.insert(
                    slot,
                    OpenSession {
                        a,
                        b,
                        rounds: 0,
                        forks: [None, None],
                    },
                );
                events.push(OverlapEvent::Open { slot, a, b });
            }
            Choice::Step { slot, polls } => {
                let slot = slot % SLOTS;
                let Some(session) = open.get_mut(&slot) else {
                    continue;
                };
                session.poll(polls, &sim);
                events.push(OverlapEvent::Step { slot, polls });
            }
            Choice::Close { slot } => {
                let slot = slot % SLOTS;
                let Some(session) = open.remove(&slot) else {
                    continue;
                };
                session.close(&mut sim);
                events.push(OverlapEvent::Close { slot });
            }
        }
    }

    // The executor closes leftover sessions in ascending slot order;
    // mirror that so redact validity extends through the implicit tail.
    for (_, session) in open.into_iter() {
        session.close(&mut sim);
    }

    (
        OverlapSchedule {
            n_peers,
            fork_parents,
            events,
        },
        sim,
    )
}
