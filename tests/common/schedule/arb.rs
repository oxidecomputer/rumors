//! The proptest strategies for `Schedule<T>` and the shadow simulator
//! backing them.
//!
//! Every schedule emitted by [`arb_schedule`] and
//! [`arb_membership_schedule`] is *valid by construction*: a `Redact`
//! event always references an `Insert` whose message the redacting peer
//! has already observed by that point, and every event names only peers
//! alive when it runs (a `Retire` needs two distinct live peers). To
//! enforce this, the generator drives a [`SimState`] in lockstep with
//! the choices it emits — a shadow simulator that mirrors what each
//! `Peer<T>` would observe under the protocol (including the
//! deletion-honoring propagation of redactions during gossip, the
//! content copy a bootstrap hands a newcomer, and the absorption a
//! retirement performs). A choice that resolves to nothing an
//! application could do (redacting with nothing observed, retiring with
//! one peer alive) is simply dropped, so the executor never has to
//! filter "impossible" events at runtime.

use std::collections::BTreeSet;
use std::fmt::Debug;
use std::ops::RangeInclusive;

use proptest::collection::vec;
use proptest::prelude::*;

use super::events::{Event, EventIdx, Schedule};

/// Strategy: every emitted schedule has only causally-valid events.
///
/// `value_strategy` supplies the value type carried by each `Insert`
/// event; pass `any::<u64>()` for the default suite or e.g.
/// `"[a-z]{0,8}".prop_map(String::from)` for a string-valued variant.
pub fn arb_schedule<T, S>(
    value_strategy: S,
    n_peers_range: RangeInclusive<usize>,
    max_events: usize,
) -> impl Strategy<Value = Schedule<T>>
where
    T: Clone + Debug + 'static,
    S: Strategy<Value = T> + Clone + 'static,
{
    arb_schedule_with_shadow(value_strategy, n_peers_range, max_events)
        .prop_map(|(schedule, _shadow)| schedule)
}

/// Final state of the shadow simulator after a schedule has been
/// built, surfaced by [`arb_schedule_with_shadow`] and
/// [`arb_membership_schedule_with_shadow`] for use by the
/// shadow-validity meta-tests.
///
/// Vectors are indexed by peer over the *total* population — the
/// initial fleet plus every mid-schedule bootstrap. A retired peer's
/// entries freeze at its retirement: `observed_log` is its complete
/// lifetime log, and its `live` set is meaningless once `alive` is
/// false (the content moved into its absorber).
#[derive(Debug, Clone)]
pub struct ShadowFinal {
    /// Per-peer sequence of `EventIdx`s the shadow predicts the live
    /// `Peer<T>` would have appended to its observation vector.
    pub observed_log: Vec<Vec<EventIdx>>,
    /// Per-peer set of `EventIdx`s the shadow predicts the live peer
    /// still has *live* in its rumor set at the end of the schedule.
    pub live: Vec<BTreeSet<EventIdx>>,
    /// Whether each peer is still in the fleet at the end of the
    /// schedule. Always all-true for the membership-free strategy.
    pub alive: Vec<bool>,
}

/// Variant of [`arb_schedule`] that also yields the shadow simulator's
/// final state. Used by the shadow-validity meta-test to confirm the
/// generator's model agrees with what the real executor produces.
pub fn arb_schedule_with_shadow<T, S>(
    value_strategy: S,
    n_peers_range: RangeInclusive<usize>,
    max_events: usize,
) -> impl Strategy<Value = (Schedule<T>, ShadowFinal)>
where
    T: Clone + Debug + 'static,
    S: Strategy<Value = T> + Clone + 'static,
{
    arb_schedule_with_shadow_using(n_peers_range, max_events, move |n_peers| {
        arb_choice(value_strategy.clone(), n_peers)
    })
}

/// [`arb_schedule`] with membership events in the alphabet: mid-schedule
/// bootstraps grow the fleet and retires shrink it, every emitted event
/// valid by construction against the shadow's alive set.
///
/// Peer references
/// draw raw entropy resolved against the population alive at each point,
/// so events reach mid-schedule newcomers too.
pub fn arb_membership_schedule<T, S>(
    value_strategy: S,
    n_peers_range: RangeInclusive<usize>,
    max_events: usize,
) -> impl Strategy<Value = Schedule<T>>
where
    T: Clone + Debug + 'static,
    S: Strategy<Value = T> + Clone + 'static,
{
    arb_membership_schedule_with_shadow(value_strategy, n_peers_range, max_events)
        .prop_map(|(schedule, _shadow)| schedule)
}

/// Variant of [`arb_membership_schedule`] that also yields the shadow
/// simulator's final state, for the membership shadow-validity meta-test.
pub fn arb_membership_schedule_with_shadow<T, S>(
    value_strategy: S,
    n_peers_range: RangeInclusive<usize>,
    max_events: usize,
) -> impl Strategy<Value = (Schedule<T>, ShadowFinal)>
where
    T: Clone + Debug + 'static,
    S: Strategy<Value = T> + Clone + 'static,
{
    arb_schedule_with_shadow_using(n_peers_range, max_events, move |_n_peers| {
        arb_membership_choice(value_strategy.clone())
    })
}

/// The shared schedule-generation core, parameterized by the per-fleet
/// choice strategy (the alphabet): both the membership-free and the
/// membership generators are this function with their own alphabets.
///
/// Alongside the event choices, it draws raw entropy for the fork
/// topology: one value per non-seed peer, folded into a valid parent
/// index by [`fork_tree`]. Drawing it here (rather than fixing a star)
/// exercises imbalanced fork lattices, which stress the ITC party
/// arithmetic.
fn arb_schedule_with_shadow_using<T, C, F>(
    n_peers_range: RangeInclusive<usize>,
    max_events: usize,
    choice_strategy: F,
) -> impl Strategy<Value = (Schedule<T>, ShadowFinal)>
where
    T: Clone + Debug + 'static,
    C: Strategy<Value = Choice<T>>,
    F: Fn(usize) -> C + Clone + 'static,
{
    n_peers_range.prop_flat_map(move |n_peers| {
        (
            vec(any::<usize>(), n_peers.saturating_sub(1)),
            vec(choice_strategy(n_peers), 0..=max_events),
        )
            .prop_map(move |(raw_parents, choices)| {
                let fork_parents = fork_tree(n_peers, &raw_parents);
                build_schedule(n_peers, fork_parents, choices)
            })
    })
}

/// Fold raw entropy into a valid fork tree.
///
/// Peer `i` (for `i >= 1`) forks from `raw[i - 1] % i`, which is always an
/// already-existing peer, so the whole fleet descends from peer 0 (the
/// seed) and every pair of peers is disjoint. `fork_parents[0]` is a
/// placeholder `0` (peer 0 is the seed itself). Because `raw` shrinks
/// toward `0`, the topology shrinks toward a star rooted at peer 0 — the
/// simplest reproduction of any failure.
fn fork_tree(n_peers: usize, raw: &[usize]) -> Vec<usize> {
    (0..n_peers)
        .map(|i| if i == 0 { 0 } else { raw[i - 1] % i })
        .collect()
}

/// Abstract action the strategy emits. Concrete `Event`s are derived
/// from these in [`build_schedule`] by consulting a per-peer
/// observation log that mirrors the protocol's effects.
///
/// Peer fields are raw entropy, resolved against the alive population at
/// build time (`alive[raw % alive.len()]`). The membership-free strategy
/// draws them already in `0..n_peers`, where the resolution is the
/// identity — its emitted schedules are untouched by the mapping.
#[derive(Debug, Clone)]
enum Choice<T> {
    Insert {
        peer: usize,
        value: T,
    },
    /// Pick the `idx % len`-th entry in the redacting peer's current
    /// observation log; if the log is empty, the choice is dropped.
    RedactObservation {
        peer: usize,
        idx: usize,
    },
    Gossip {
        a: usize,
        b: usize,
    },
    /// Bootstrap a new peer from the resolved parent. Membership
    /// strategy only.
    Bootstrap {
        parent: usize,
    },
    /// Retire the resolved retiree into a distinct alive absorber
    /// (`absorber` is an offset among the other alive peers); dropped
    /// when fewer than two peers are alive. Membership strategy only.
    Retire {
        retiree: usize,
        absorber: usize,
    },
}

fn arb_choice<T, S>(value_strategy: S, n_peers: usize) -> impl Strategy<Value = Choice<T>>
where
    T: Clone + Debug + 'static,
    S: Strategy<Value = T> + Clone + 'static,
{
    prop_oneof![
        4 => (0..n_peers, value_strategy)
            .prop_map(|(peer, value)| Choice::Insert { peer, value }),
        2 => (0..n_peers, any::<usize>())
            .prop_map(|(peer, idx)| Choice::RedactObservation { peer, idx }),
        3 => (0..n_peers, 0..n_peers)
            .prop_map(|(a, b)| Choice::Gossip { a, b }),
    ]
}

/// [`arb_choice`] plus the membership alphabet. Peer references draw raw
/// entropy (the population grows mid-schedule, so no closed range fits);
/// weights keep the churn meaningful without drowning the content events.
fn arb_membership_choice<T, S>(value_strategy: S) -> impl Strategy<Value = Choice<T>>
where
    T: Clone + Debug + 'static,
    S: Strategy<Value = T> + Clone + 'static,
{
    prop_oneof![
        4 => (any::<usize>(), value_strategy)
            .prop_map(|(peer, value)| Choice::Insert { peer, value }),
        2 => (any::<usize>(), any::<usize>())
            .prop_map(|(peer, idx)| Choice::RedactObservation { peer, idx }),
        3 => (any::<usize>(), any::<usize>())
            .prop_map(|(a, b)| Choice::Gossip { a, b }),
        1 => any::<usize>().prop_map(|parent| Choice::Bootstrap { parent }),
        1 => (any::<usize>(), any::<usize>())
            .prop_map(|(retiree, absorber)| Choice::Retire { retiree, absorber }),
    ]
}

/// Shadow simulator: per-peer state kept in lockstep with what the
/// live simulation would observe under the actual protocol. For peer
/// `p`:
///
/// * `ever_known[p]` is every `EventIdx` whose message `p` has ever
///   held (whether it currently holds it or has since redacted it).
/// * `live[p]` is the subset currently in `p`'s live rumor set.
/// * `observed_log[p]` is the exact sequence of `EventIdx`s that the
///   live `Peer<T>` would have appended to its observation vector by
///   this point — driven by both local inserts and gossip events.
///
/// `RedactObservation` picks an entry from `observed_log` to redact,
/// so the schedule is guaranteed to issue every `Redact` on a message
/// the peer actually holds at that moment.
struct SimState {
    ever_known: Vec<BTreeSet<EventIdx>>,
    live: Vec<BTreeSet<EventIdx>>,
    observed_log: Vec<Vec<EventIdx>>,
    /// Which peers are in the fleet right now; membership events flip
    /// entries. Every peer reference a choice carries resolves through
    /// [`alive_peers`](Self::alive_peers), so no emitted event ever
    /// names a retired peer.
    alive: Vec<bool>,
}

impl SimState {
    fn new(n_peers: usize) -> Self {
        Self {
            ever_known: vec![BTreeSet::new(); n_peers],
            live: vec![BTreeSet::new(); n_peers],
            observed_log: vec![Vec::new(); n_peers],
            alive: vec![true; n_peers],
        }
    }

    /// Indices of the peers currently in the fleet.
    fn alive_peers(&self) -> Vec<usize> {
        (0..self.alive.len()).filter(|&p| self.alive[p]).collect()
    }

    /// Mint a new peer as a bootstrap copy of `parent`.
    ///
    /// The newcomer holds everything the parent holds (including the
    /// version frontier that carries the parent's redactions), but its
    /// observation log starts empty — content already present at birth
    /// is never observed, matching the executor's `Peer::new`
    /// checkpoint semantics.
    fn record_bootstrap(&mut self, parent: usize) -> usize {
        let newcomer = self.alive.len();
        self.ever_known.push(self.ever_known[parent].clone());
        self.live.push(self.live[parent].clone());
        self.observed_log.push(Vec::new());
        self.alive.push(true);
        newcomer
    }

    /// Retire `retiree` into `absorber`, freezing the retiree's log and
    /// live set.
    ///
    /// One [`absorb`](Self::absorb): the retiree's state is never read
    /// again, and the real executor consumes the peer before anything
    /// could observe what the session taught it.
    fn record_retire(&mut self, retiree: usize, absorber: usize) {
        self.absorb(retiree, absorber);
        self.alive[retiree] = false;
    }

    /// One direction of a reconciliation: `dst` ends holding the union
    /// of both contents — it learns `src`'s novel live messages
    /// (observing them) and either side's redaction prevails in `dst` —
    /// while `src` is untouched.
    ///
    /// A full gossip is an absorb each way; a retirement is one absorb
    /// into the survivor.
    fn absorb(&mut self, src: usize, dst: usize) {
        let combined: BTreeSet<EventIdx> = self.ever_known[src]
            .union(&self.ever_known[dst])
            .copied()
            .collect();
        for k in combined {
            let src_known = self.ever_known[src].contains(&k);
            let src_live = self.live[src].contains(&k);
            let dst_known = self.ever_known[dst].contains(&k);
            let dst_live = self.live[dst].contains(&k);
            let any_redacted = (src_known && !src_live) || (dst_known && !dst_live);

            if any_redacted {
                self.ever_known[dst].insert(k);
                self.live[dst].remove(&k);
            } else if !dst_known {
                self.ever_known[dst].insert(k);
                self.live[dst].insert(k);
                self.observed_log[dst].push(k);
            }
        }
    }

    fn record_insert(&mut self, peer: usize, event_idx: EventIdx) {
        self.ever_known[peer].insert(event_idx);
        self.live[peer].insert(event_idx);
        self.observed_log[peer].push(event_idx);
    }

    fn record_redact(&mut self, peer: usize, target_event_idx: EventIdx) {
        // Removing from live (the peer's act of forgetting).
        // `ever_known` and `observed_log` are unchanged: the peer
        // still remembers that it once held this message.
        self.live[peer].remove(&target_event_idx);
    }

    fn gossip(&mut self, a: usize, b: usize) {
        if a == b {
            return;
        }
        // The hash-tree mirror reconciles *every* tree position that
        // differs; an absorb in each direction reproduces its fixed
        // point (the second pass reads the first's updates, so a
        // redaction on either side prevails in both), and per-peer
        // observation order is unchanged: each side still gains novel
        // messages in sorted combined order.
        self.absorb(a, b);
        self.absorb(b, a);
    }

    fn lookup_observation(&self, peer: usize, idx: usize) -> Option<EventIdx> {
        let log = &self.observed_log[peer];
        if log.is_empty() {
            None
        } else {
            Some(log[idx % log.len()])
        }
    }
}

fn build_schedule<T>(
    n_peers: usize,
    fork_parents: Vec<usize>,
    choices: Vec<Choice<T>>,
) -> (Schedule<T>, ShadowFinal) {
    let mut sim = SimState::new(n_peers);
    let mut events: Vec<Event<T>> = Vec::new();
    for choice in choices {
        let next_event_idx = events.len();
        // Resolve every raw peer reference against the population alive
        // right now; for the membership-free strategy this is the
        // identity (references are drawn in range and nobody retires).
        let alive = sim.alive_peers();
        let resolve = |raw: usize| alive[raw % alive.len()];
        match choice {
            Choice::Insert { peer, value } => {
                let peer = resolve(peer);
                sim.record_insert(peer, next_event_idx);
                events.push(Event::Insert { peer, value });
            }
            Choice::RedactObservation { peer, idx } => {
                let peer = resolve(peer);
                if let Some(target_event_idx) = sim.lookup_observation(peer, idx) {
                    sim.record_redact(peer, target_event_idx);
                    events.push(Event::Redact {
                        peer,
                        target_event_idx,
                    });
                }
                // else: the peer has not yet observed anything, so no
                // application code path could have produced this
                // `redact()` call. Drop the choice.
            }
            Choice::Gossip { a, b } => {
                let (a, b) = (resolve(a), resolve(b));
                if a == b {
                    continue;
                }
                sim.gossip(a, b);
                events.push(Event::Gossip { a, b });
            }
            Choice::Bootstrap { parent } => {
                let parent = resolve(parent);
                let newcomer = sim.record_bootstrap(parent);
                events.push(Event::Bootstrap { parent, newcomer });
            }
            Choice::Retire { retiree, absorber } => {
                // Retirement needs a distinct absorber: with fewer than
                // two peers alive no application could retire anyone, so
                // the choice is dropped. The absorber offset picks among
                // the *other* alive peers, keeping the pair distinct by
                // construction.
                if alive.len() < 2 {
                    continue;
                }
                let retiree_pos = retiree % alive.len();
                let absorber_pos = (retiree_pos + 1 + (absorber % (alive.len() - 1))) % alive.len();
                let retiree = alive[retiree_pos];
                let absorber = alive[absorber_pos];
                sim.record_retire(retiree, absorber);
                events.push(Event::Retire { retiree, absorber });
            }
        }
    }
    let SimState {
        observed_log,
        live,
        alive,
        ..
    } = sim;
    (
        Schedule {
            n_peers,
            fork_parents,
            events,
        },
        ShadowFinal {
            observed_log,
            live,
            alive,
        },
    )
}
