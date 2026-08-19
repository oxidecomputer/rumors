//! Deterministic session *overlap*: open a wire gossip session, hold it
//! parked at any poll prefix while other events run, then finish it.
//!
//! The [`schedule`](super::schedule) executor runs gossip sessions one at
//! a time, and [`sim`](super::sim) overlaps them nondeterministically on a
//! multi-thread runtime. Neither samples a *chosen* interleaving: a
//! session that forks its working state at one point in the schedule and
//! installs at a later one, with arbitrary events (including whole other
//! sessions) in between. That gap is where a real defect lived — its
//! downstream symptom was an innocent leaf silently lost under exactly
//! such an overlap — so overlap is a first-class, deterministically
//! schedulable
//! event: [`Session`] is one in-flight wire session driven by hand, and
//! [`execute_overlap`] runs a generated [`OverlapSchedule`] whose
//! sessions open, park, and close at generated points.
//!
//! The alphabet extends [`schedule::events::Event`] with three session
//! events over a small set of *slots*: [`OverlapEvent::Open`] captures
//! both endpoints' fork-time state and parks, [`OverlapEvent::Step`]
//! polls the parked session a bounded number of times, and
//! [`OverlapEvent::Close`] drives it to completion and installs. The
//! generator keeps the schedule valid by construction the same way
//! [`schedule::arb`] does — a shadow simulator tracks what every peer has
//! observed, with open sessions modeled by their fork-time snapshots so a
//! `Redact` is only ever emitted against a message its peer really holds.

use std::collections::BTreeMap;
use std::fmt::Debug;
use std::future::Future;
use std::ops::RangeInclusive;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use proptest::collection::vec;
use proptest::prelude::*;
use rumors::link::memory_with_capacity;
use rumors::{Rumors, Version};

use crate::common::oracle::Oracle;
use crate::common::peer::{Peer, gossip_step, quiesce};
use crate::common::schedule::EventIdx;
use crate::common::wire::bootstrap_fork;

/// Capacity of an overlapped session's link streams, in bytes.
///
/// Deliberately tiny, unlike [`wire::LINK_BUF`](crate::common::wire::LINK_BUF):
/// granularity is the whole point of this harness. With a roomy buffer a
/// peer's counterparty pre-buffers its entire greeting in one poll, and the
/// peer's next poll then runs its session fork, the exchange, and its
/// install back-to-back with no intervening park — the window between fork
/// and install closes before any interleaving can enter it. A tiny buffer
/// backpressures every message into fragments, making each write a parking
/// point, so the poll sweep genuinely samples interleavings *between* one
/// side's fork and its install.
const OVERLAP_LINK_BUF: usize = 48;

/// Alternation rounds after which an unfinished session is declared
/// deadlocked.
///
/// Every genuine session completes in far fewer rounds, even through the
/// deliberately tiny [`OVERLAP_LINK_BUF`]; the bound only converts a
/// protocol hang into a test failure at its source.
const SESSION_POLL_BOUND: usize = 1 << 20;

/// One in-flight wire gossip session between two peers, each side driven
/// by hand as its own future.
///
/// The sides are polled *separately*, alternating one poll each per
/// [`step`](Session::step) round. Separate side futures are what give the
/// harness its granularity: a jointly-polled pair completes a trivial
/// session inside a handful of polls (each poll of a joined future lets
/// both sides run each other to quiescence through the synchronous
/// in-memory link), which parks the session only at points too coarse to
/// fall between one side's fork and its install. Side-at-a-time
/// alternation makes every wire round trip a distinct parking point.
///
/// Each side owns a clone of its peer handle and its link end, so a
/// parked session tolerates arbitrary mutation of either peer through
/// other handles; dropping an unfinished session models a cancelled one.
/// Each side asserts its own success when it completes. (The completed
/// pair cannot assert a drained control stream the way
/// [`wire_gossip`](crate::common::wire::wire_gossip) does — the link
/// ends live inside the side futures — and a parked or cancelled session
/// legitimately leaves bytes in flight.)
pub struct Session {
    sides: [Pin<Box<dyn Future<Output = ()>>>; 2],
    done: [bool; 2],
}

/// Open a wire gossip session between `a` and `b` without polling it.
pub fn open<T>(a: &Rumors<T>, b: &Rumors<T>) -> Session
where
    T: serde::Serialize + serde::de::DeserializeOwned + Send + Sync + 'static,
{
    let a = a.clone();
    let b = b.clone();
    let (a_link, b_link) = memory_with_capacity(OVERLAP_LINK_BUF);
    Session {
        sides: [
            Box::pin(async move {
                let mut a_link = a_link;
                a.gossip(&mut a_link)
                    .await
                    .expect("overlapped session side A");
            }),
            Box::pin(async move {
                let mut b_link = b_link;
                b.gossip(&mut b_link)
                    .await
                    .expect("overlapped session side B");
            }),
        ],
        done: [false, false],
    }
}

impl Session {
    /// Drive at most `n` alternation rounds — one poll of each unfinished
    /// side per round — returning `true` once both sides have completed.
    /// Stepping a completed session is a no-op.
    ///
    /// The in-memory link delivers bytes synchronously, so alternating
    /// side polls makes real progress without a runtime; a no-op waker
    /// suffices.
    pub fn step(&mut self, n: usize) -> bool {
        if self.done == [true, true] {
            return true;
        }
        let mut cx = Context::from_waker(Waker::noop());
        for _ in 0..n {
            for side in 0..2 {
                if !self.done[side]
                    && let Poll::Ready(()) = self.sides[side].as_mut().poll(&mut cx)
                {
                    self.done[side] = true;
                }
            }
            if self.done == [true, true] {
                return true;
            }
        }
        false
    }

    /// Drive the session to completion.
    ///
    /// # Panics
    ///
    /// If the session makes no progress within [`SESSION_POLL_BOUND`]
    /// polls — a deadlock, which is itself a finding.
    pub fn finish(mut self) {
        assert!(
            self.step(SESSION_POLL_BOUND),
            "overlapped session did not complete within {SESSION_POLL_BOUND} polls: \
             a protocol deadlock"
        );
    }
}

/// The overlap alphabet: [`schedule`](super::schedule)'s events plus
/// hand-driven sessions in a small slot space.
#[derive(Debug, Clone)]
pub enum OverlapEvent<T> {
    /// Insert `value` at `peer`.
    Insert { peer: usize, value: T },
    /// Redact the message minted by the `Insert` at `target_event_idx`.
    /// Valid by construction: the generator's shadow guarantees `peer`
    /// has observed that message when this event runs.
    Redact {
        peer: usize,
        target_event_idx: EventIdx,
    },
    /// One whole (non-overlapped) session between `a` and `b`, as the
    /// serial executor runs them.
    Gossip { a: usize, b: usize },
    /// Open a session between `a` and `b` in `slot`, forking both sides'
    /// working state here; it installs only at its `Close`.
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
    T: Clone + Eq + Ord + serde::Serialize + serde::de::DeserializeOwned + Send + Sync + 'static,
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
                // The generator's shadow makes this always-observed; the
                // guard mirrors the serial executor's, so a shadow
                // imprecision degrades to a skipped event on both sides
                // of the comparison rather than an invalid `redact`.
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
    quiesce(&mut peers);
    (peers, oracle)
}

/// How many session slots a generated schedule may hold open at once.
/// Two suffices to overlap a session with a whole other session; a third
/// lets overlaps themselves overlap.
const SLOTS: usize = 3;

/// Strategy: overlap schedules that are valid by construction.
///
/// Biased toward the shape that discovers install-time interleaving
/// defects — a converged fleet whose base content spans several radix-fan
/// chunks, redactions of that base, and sessions parked across other
/// sessions' installs (the [`Pincer`] motif, spliced into the general
/// soup).
///
/// The preamble (inserts at the seed peer, then one full-mesh round)
/// guarantees every schedule starts from a *converged, populated* fleet:
/// the regime where an overlapped session's fork-time state and its
/// counterparty's are one and the same object, which no unbiased soup of
/// events reliably reaches.
pub fn arb_overlap_schedule<T, S>(
    value_strategy: S,
    n_peers_range: RangeInclusive<usize>,
    max_events: usize,
) -> impl Strategy<Value = OverlapSchedule<T>>
where
    T: Clone + Debug + 'static,
    S: Strategy<Value = T> + Clone + 'static,
{
    n_peers_range.prop_flat_map(move |n_peers| {
        (
            vec(any::<usize>(), n_peers.saturating_sub(1)),
            // Preamble size: enough base content to span multiple
            // radix-fan chunks (the discovering defect needed the root
            // fan to cross one 16-entry chunk), sometimes much more.
            (12usize..=48),
            vec(value_strategy.clone(), 48),
            vec(arb_choice(value_strategy.clone(), n_peers), 0..=max_events),
            vec(arb_pincer(value_strategy.clone(), n_peers), 1..=2),
        )
            .prop_map(
                move |(raw_parents, preamble_len, preamble_values, mut choices, pincers)| {
                    let fork_parents = fork_tree(n_peers, &raw_parents);
                    // Splice each pincer into the soup at its drawn
                    // offset; later offsets are relative to the already-
                    // spliced stream, which keeps splices independent.
                    for pincer in &pincers {
                        let at = pincer.offset % (choices.len() + 1);
                        choices.splice(at..at, pincer.choices(n_peers));
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

/// One deliberate overlap pincer, spliced into the generated soup: the
/// motif distilled from the discovering incident, as choices the shadow
/// processes like any others.
///
/// Expansion (skipped when the fleet has fewer than three peers):
/// re-converge `x` and `y`; mutate `w` (an insert, or a redaction of
/// something it observed); open a session `x <-> y` — now trivially
/// converged, the regime whose install re-joins the fork-time state
/// itself; park it a few rounds; run a whole `x <-> w` session so the
/// mutation installs at `x` mid-window; close the parked session. The
/// generated soup around it supplies every other interleaving; the
/// pincer guarantees the family samples the one that found a real bug,
/// densely enough that the known defect reproduces within a default
/// run's case budget.
#[derive(Debug, Clone)]
struct Pincer<T> {
    offset: usize,
    x: usize,
    y: usize,
    w: usize,
    slot: usize,
    park: usize,
    /// `Ok(value)`: insert at `w`; `Err(idx)`: redact `w`'s `idx`-th
    /// observation.
    mutation: Result<T, usize>,
}

fn arb_pincer<T, S>(value_strategy: S, n_peers: usize) -> impl Strategy<Value = Pincer<T>>
where
    T: Clone + Debug + 'static,
    S: Strategy<Value = T> + Clone + 'static,
{
    (
        any::<usize>(),
        0..n_peers,
        0..n_peers,
        0..n_peers,
        any::<usize>(),
        1usize..=12,
        prop_oneof![value_strategy.prop_map(Ok), any::<usize>().prop_map(Err),],
    )
        .prop_map(|(offset, x, y, w, slot, park, mutation)| Pincer {
            offset,
            x,
            y,
            w,
            slot,
            park,
            mutation,
        })
}

impl<T: Clone> Pincer<T> {
    /// The pincer as ordinary choices, or none when the fleet cannot
    /// host three distinct roles.
    fn choices(&self, n_peers: usize) -> Vec<Choice<T>> {
        if n_peers < 3 {
            return Vec::new();
        }
        // Fold the drawn roles into three distinct peers.
        let x = self.x % n_peers;
        let y = (x + 1 + (self.y % (n_peers - 1))) % n_peers;
        let mut w = self.w % n_peers;
        while w == x || w == y {
            w = (w + 1) % n_peers;
        }
        vec![
            Choice::Gossip { a: x, b: y },
            match &self.mutation {
                Ok(value) => Choice::Insert {
                    peer: w,
                    value: value.clone(),
                },
                Err(idx) => Choice::RedactObservation { peer: w, idx: *idx },
            },
            Choice::Open {
                slot: self.slot,
                a: x,
                b: y,
            },
            Choice::Step {
                slot: self.slot,
                polls: self.park,
            },
            Choice::Gossip { a: x, b: w },
            Choice::Close { slot: self.slot },
        ]
    }
}

/// Per-peer knowledge sets, as in `schedule::arb`'s shadow: everything
/// the peer has ever held, the subset currently live, and the exact
/// observation order.
#[derive(Clone)]
struct Knowledge {
    ever_known: Vec<std::collections::BTreeSet<EventIdx>>,
    live: Vec<std::collections::BTreeSet<EventIdx>>,
    observed_log: Vec<Vec<EventIdx>>,
}

impl Knowledge {
    fn new(n_peers: usize) -> Self {
        Self {
            ever_known: vec![Default::default(); n_peers],
            live: vec![Default::default(); n_peers],
            observed_log: vec![Vec::new(); n_peers],
        }
    }

    /// Merge what a session forked at `snapshot` delivers between `a`
    /// and `b` into the *current* state.
    ///
    /// The session carries each side's fork-time content only: messages
    /// one fork-time side held live propagate to a counterparty that has
    /// never known them; messages either fork-time side had redacted die
    /// on both current sides (deletion honoring, tombstone-free). A
    /// message redacted *after* the fork stays dead locally —
    /// `ever_known` guards resurrection — and its counterparty learns
    /// that deletion only from a later session, exactly as the wire
    /// behaves.
    fn merge_session(&mut self, snapshot: &Knowledge, a: usize, b: usize) {
        let combined: std::collections::BTreeSet<EventIdx> = snapshot.ever_known[a]
            .union(&snapshot.ever_known[b])
            .copied()
            .collect();
        for k in combined {
            let a_had = snapshot.ever_known[a].contains(&k);
            let b_had = snapshot.ever_known[b].contains(&k);
            let redacted_at_fork = (a_had && !snapshot.live[a].contains(&k))
                || (b_had && !snapshot.live[b].contains(&k));
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

/// Build a valid overlap schedule by driving the shadow in lockstep with
/// the emitted events, with each open session modeled by the fork-time
/// snapshot its `Open` captured.
fn build_overlap_schedule<T: Clone>(
    n_peers: usize,
    fork_parents: Vec<usize>,
    preamble: &[T],
    choices: Vec<Choice<T>>,
) -> OverlapSchedule<T> {
    let mut sim = Knowledge::new(n_peers);
    let mut open: BTreeMap<usize, (usize, usize, Knowledge)> = BTreeMap::new();
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
            sim.merge_session(&frozen, a, b);
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
                sim.merge_session(&frozen, a, b);
                events.push(OverlapEvent::Gossip { a, b });
            }
            Choice::Open { slot, a, b } => {
                let slot = slot % SLOTS;
                if a == b || open.contains_key(&slot) {
                    continue;
                }
                open.insert(slot, (a, b, sim.clone()));
                events.push(OverlapEvent::Open { slot, a, b });
            }
            Choice::Step { slot, polls } => {
                let slot = slot % SLOTS;
                if !open.contains_key(&slot) {
                    continue;
                }
                events.push(OverlapEvent::Step { slot, polls });
            }
            Choice::Close { slot } => {
                let slot = slot % SLOTS;
                let Some((a, b, snapshot)) = open.remove(&slot) else {
                    continue;
                };
                sim.merge_session(&snapshot, a, b);
                events.push(OverlapEvent::Close { slot });
            }
        }
    }

    // The executor closes leftover sessions in ascending slot order;
    // mirror that so redact validity extends through the implicit tail.
    for (_, (a, b, snapshot)) in open.into_iter() {
        sim.merge_session(&snapshot, a, b);
    }

    OverlapSchedule {
        n_peers,
        fork_parents,
        events,
    }
}
