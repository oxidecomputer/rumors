//! The pipeline window: how many disputed scopes a session keeps in flight.
//!
//! Every recursive edge in the streaming session — the walk's query and
//! resolution queues, the proxy's flushed-question and next-scope queues —
//! is a bounded channel. One slot per edge is the *liveness floor*: the
//! ordering invariants in [`materialized`](super::materialized)'s module
//! docs prove a session at capacity one never deadlocks. But one slot also
//! admits only ~2 disputed scopes in flight per level, which serializes the
//! descent into one wire round trip per disputed scope
//! (`design/streaming-latency-serialization.md`). The window widens those
//! edges so sibling scopes pipeline; capacity only relaxes the wait graph,
//! so every schedule live at the floor stays live at any width.
//!
//! The public knob ([`Peer::max_in_flight_nodes`](crate::Peer::max_in_flight_nodes))
//! is denominated in *node references* — the unit whose worst-case memory a
//! deployment can price — and bounds the **session-global** in-flight
//! total: the derivation charges each in-flight scope a full possible fan
//! and admits [`SATURABLE_LEVELS`] level boundaries running at full
//! occupancy at once, the realistic maximum against terabyte-scale sets
//! (design doc §6.5).
//!
//! Two boundaries the window deliberately does not reach:
//!
//! - Capacity is a bound, not an allocation: the channels are
//!   semaphore-bounded and allocate per queued item, so an idle wide window
//!   costs nothing.
//! - The assembly fan queues are **not** window edges. Their capacity of
//!   one full fan is a *correctness* floor, not a tunable: below it, a
//!   maximally disputed reply's child completions cannot all enqueue while
//!   the walk finishes the reaction loop, and the session deadlocks —
//!   demonstrated by `underbuffered_mirror_stalls` in the capacity tests.
//!   No configuration, however memory-starved, may shrink them.
//!
//! # Sizing the flushed-question edge
//!
//! The proxy's flushed-question queue
//! ([`ProxyLocalQuestions`](super::channel::QueueKind::ProxyLocalQuestions))
//! holds questions that are on the wire but unanswered: the encoder
//! publishes a question only after the reply carrying it has completely
//! flushed, and the decoder retires one per decoded wire reply — a full
//! round trip later. Its capacity is window-wide, and that is not
//! defensive headroom. The claim — derived, not measured, from the
//! premises below; it closes the bound the eager-absorption assessment
//! carried open (`design/eager-absorption.md` §7.2) — is:
//!
//! > At a level whose descent ultimately asks `S` questions, the queue's
//! > supremum occupancy over schedules is exactly `min(capacity, S)`. Its
//! > own capacity is the *only* structural bound short of the frontier.
//!
//! The `≤` half is immediate: the channel is bounded, and a question
//! enters the queue at most once. Reachability is the substantive half.
//! Questions aggregate *across* parent replies — every reply flushed at
//! the level above deposits up to a full fan of them — and a bounded
//! channel upstream limits how many items sit on that edge at once, never
//! how many pass through it: slots recycle. Per-edge independence (the
//! [link contract](crate::link)) therefore admits schedules in which
//! retirement stalls while production continues — a live counterparty
//! that serves every level above promptly but lags on this one, or a
//! local walk that defers consuming this level's responses so retirement
//! parks behind the proxy's one-slot response relay. Under such a
//! schedule, replies decoded above keep refilling the next-scope edge,
//! the encoder keeps pairing recycled scopes with the walk's replies, and
//! each flushed pair deposits up to a fan more questions with none
//! retired: occupancy climbs until the queue's own capacity clamps it or
//! the frontier runs out.
//!
//! Premises, each checked against the code it names:
//!
//! - the encoder flushes one complete wire reply, then publishes that
//!   reply's entire question batch, before dequeuing its next scope
//!   (`remote/proxy/work/encode.rs`; file paths, not intra-doc links,
//!   because `proxy` is private to `remote` and unresolvable from here);
//! - the decoder dequeues question-first and retires exactly one entry
//!   per decoded reply, in wire order (`remote/proxy/work/pump.rs`);
//! - one reply asks at most one fan of questions (its disputed children);
//! - edges are independently flow-controlled, so a full edge stalls only
//!   its own producer — the premise the session's whole liveness argument
//!   already rests on.
//!
//! Two corollaries:
//!
//! - **Window-wide is necessary.** A question holds its slot from
//!   publication until the decoder takes it up, one wire round trip
//!   later, so capacity `C` admits at most `C` questions in flight per
//!   round trip at that level *no matter how wide the walk's own channels
//!   are*: this edge is the level's wire window, and shrinking it below
//!   the window re-serializes the descent at exactly that level.
//!   Shrinking is always *safe* — capacity only relaxes the wait graph,
//!   so any capacity ≥ 1 preserves liveness — but it forfeits latency the
//!   rest of the window was sized to buy.
//! - **The queue undercounts wire in-flight by a bounded slack.** During
//!   a flush the batch rides the wire before publication (up to one fan,
//!   in the encoder's hand), and the decoder holds one dequeued question
//!   while its reply decodes; true wire in-flight at a level is therefore
//!   at most `capacity + fan + 1`. At the liveness floor the slack *is*
//!   the story: capacity one still puts a full fan on the wire, because
//!   the whole reply flushes before any publication. At production widths
//!   it is a sub-percent correction per level, dwarfed by the full-fan
//!   cascade the node-budget derivation above already charges each
//!   saturable boundary.

/// The tree's maximum branching factor: one child per radix byte.
///
/// Also the hard capacity floor of the assembly fan queues (see the
/// [module docs](self)): those channels must admit one *full* fan
/// regardless of any window tuning.
pub(crate) const FAN: usize = 256;

/// Level boundaries a session can hold at full occupancy simultaneously.
///
/// Saturating `L` consecutive boundaries with `K` full-fan scopes each
/// requires a disputed forest of `K × 256^L` leaves; against sets up to a
/// terabyte this caps `L` at three (design doc §6.5). The node budget is
/// divided by this, so the global charge stays inside the budget rather
/// than multiplying per level.
const SATURABLE_LEVELS: usize = 3;

/// Node references a session may hold in flight by default, globally.
///
/// Derives a per-edge window of 65 536 scopes — a fully fanned level's
/// entire cascade (256 scopes × 256 children) never blocks the walk — for
/// a worst-case envelope near 10 GB at ~215 B per placeholder reference
/// (`design/streaming-latency-serialization.md` §6.4–6.5), reached only
/// against multi-gigabyte divergence; typical sessions hold kilobytes.
/// Tune down via [`Peer::max_in_flight_nodes`](crate::Peer::max_in_flight_nodes)
/// under hard memory budgets.
pub const DEFAULT_MAX_IN_FLIGHT_NODES: usize = SATURABLE_LEVELS * FAN * FAN * FAN;

/// Per-edge channel capacity for one session, in disputed scopes.
///
/// Constructed from a global node budget by [`from_nodes`](Self::from_nodes);
/// consumed by the channel constructors of the materialized walk and the
/// remote proxy. `Default` differs by build: production sessions get
/// [`DEFAULT_MAX_IN_FLIGHT_NODES`], while test builds (`cfg(test)` and the
/// `test-internals` feature) get the one-slot liveness floor so every
/// schedule keeps being exercised at the capacity where a bad ordering
/// *would* deadlock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Window {
    /// Scopes admitted per recursive channel edge.
    scopes: usize,
}

impl Window {
    /// The liveness floor: one scope per edge, the deadlock-proof minimum.
    pub(crate) const FLOOR: Self = Self { scopes: 1 };

    /// Derive a window from a global budget of in-flight node references.
    ///
    /// Each in-flight scope may pin up to a full fan of references and up
    /// to [`SATURABLE_LEVELS`] boundaries can run at full occupancy at
    /// once, so the per-edge capacity is `nodes / (256 × 3)`, rounded
    /// down to stay inside the budget and floored at one slot (any budget
    /// below one charged scope, including zero, yields the liveness
    /// floor).
    pub(crate) fn from_nodes(nodes: usize) -> Self {
        Self {
            scopes: (nodes / (FAN * SATURABLE_LEVELS)).max(Self::FLOOR.scopes),
        }
    }

    /// The per-edge channel capacity this window grants, in scopes.
    pub(crate) fn scopes(self) -> usize {
        self.scopes
    }
}

impl Default for Window {
    fn default() -> Self {
        // Tests run at the floor so the capacity-one orderings the
        // deadlock-freedom argument certifies stay exercised; production
        // sessions pipeline by default.
        #[cfg(any(test, feature = "test-internals"))]
        {
            Self::FLOOR
        }
        #[cfg(not(any(test, feature = "test-internals")))]
        {
            Self::from_nodes(DEFAULT_MAX_IN_FLIGHT_NODES)
        }
    }
}

#[cfg(test)]
mod tests;
