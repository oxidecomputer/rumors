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
    pub events: Vec<Event<T>>,
}
