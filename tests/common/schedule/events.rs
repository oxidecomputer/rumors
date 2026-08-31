//! The event model: what a schedule is a sequence of.

/// Index of an event in a `Schedule`'s flat `events` vector. Used as
/// a stable cross-reference between the oracle, the schedule
/// executor, and the shadow simulator.
pub type EventIdx = usize;

#[derive(Debug, Clone)]
pub enum Event<T> {
    Insert {
        peer: usize,
        value: T,
    },
    /// Redact the message (by its created `Version`) sent by the
    /// `Insert` event at this index in the schedule's emitted event
    /// sequence.
    ///
    /// The strategy guarantees the redacting peer has observed that
    /// message by the time this event runs.
    Redact {
        peer: usize,
        target_event_idx: EventIdx,
    },
    Gossip {
        a: usize,
        b: usize,
    },
    /// Create a new peer mid-schedule by serving it a bootstrap from
    /// `parent`.
    ///
    /// The newcomer takes index `newcomer` — always the next
    /// unused index, carried explicitly so counterexamples read without
    /// replaying the schedule. Emitted only by the membership strategy;
    /// the strategy guarantees `parent` is alive at this point.
    Bootstrap {
        parent: usize,
        newcomer: usize,
    },
    /// Retire `retiree` into `absorber` over a clean wire: the
    /// absorber ends the session holding the union of both contents
    /// (redactions included), and `retiree` leaves the fleet for good.
    ///
    /// Emitted only by the membership strategy; the strategy guarantees
    /// both peers are alive and distinct.
    Retire {
        retiree: usize,
        absorber: usize,
    },
}

#[derive(Debug, Clone)]
pub struct Schedule<T> {
    pub n_peers: usize,
    /// Fork topology of the peer fleet.
    ///
    /// `fork_parents[i]` is the peer that peer `i` was forked from;
    /// `fork_parents[0]` is unused (peer 0 is the universe seed). The
    /// invariant `fork_parents[i] < i` makes the peers one fork tree
    /// descending from a single seed, hence pairwise *disjoint*: the
    /// precondition for `join`/`gossip` under the `before` crate's Law of
    /// Disjointness. A star (every entry 0) is the shrink target.
    pub fork_parents: Vec<usize>,
    pub events: Vec<Event<T>>,
}
