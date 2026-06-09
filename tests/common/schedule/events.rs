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
    /// Redact the `Key` minted by the `Insert` event at this index in
    /// the schedule's emitted event sequence. The strategy guarantees
    /// the redacting peer has observed that `Key` by the time this
    /// event runs.
    Redact {
        peer: usize,
        target_event_idx: EventIdx,
    },
    Gossip {
        a: usize,
        b: usize,
    },
}

#[derive(Debug, Clone)]
pub struct Schedule<T> {
    pub n_peers: usize,
    /// Fork topology of the peer fleet. `fork_parents[i]` is the peer that peer
    /// `i` was forked from; `fork_parents[0]` is unused (peer 0 is the universe
    /// seed). The invariant `fork_parents[i] < i` makes the peers one fork tree
    /// descending from a single seed, hence pairwise *disjoint*: the
    /// precondition for `join`/`gossip` under the `before` crate's Law of
    /// Disjointness. A star (every entry 0) is the shrink target.
    pub fork_parents: Vec<usize>,
    pub events: Vec<Event<T>>,
}
