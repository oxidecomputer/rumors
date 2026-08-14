//! The window-budget sweep dimension: a generated per-peer choice of
//! reconciliation-window configuration, so behavioral suites exercise the
//! production-default wide-window path alongside the pinned floor.
//!
//! The floor stays in the population deliberately: capacity one is the
//! configuration the deadlock-freedom argument certifies, and suites must
//! keep those orderings exercised. The sweep adds the two budgeted
//! regimes on top — a tight budget whose solve binds (granting widths
//! between the floor and the population clamp, including budgets small
//! enough to resolve back to one slot through the budget path), and the
//! production default. Peers draw choices independently, so sessions
//! between differently-configured endpoints (asymmetric windows) arise
//! throughout the population.
//!
//! `tests/window_sweep.rs` holds the liveness pins for this dimension:
//! proof that the non-floor choices actually widen the granted window at
//! suite-scale set sizes, so the sweep cannot silently degenerate to
//! floor-everywhere.

use proptest::prelude::*;

/// One peer's window configuration in the sweep population.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowChoice {
    /// The one-slot serialization floor, the deadlock-freedom argument's
    /// certified configuration. First variant, so failures shrink toward
    /// the floor-everywhere baseline.
    Floor,
    /// An explicit memory budget in bytes, resolved against the exchanged
    /// set sizes at session start.
    Budget(usize),
    /// The production default: no configuration call at all.
    Default,
}

impl WindowChoice {
    /// Apply this choice to a peer under construction.
    pub fn apply<T>(self, peer: rumors::Peer<T>) -> rumors::Peer<T> {
        match self {
            Self::Floor => peer.sync_window_floor(),
            Self::Budget(bytes) => peer.sync_memory_budget(bytes),
            Self::Default => peer,
        }
    }
}

/// A generated window assignment for a whole fleet: peer `i` takes
/// [`choice(i)`](WindowAssignment::choice).
///
/// Indexing wraps, so one
/// assignment covers founders and any peers minted later (mid-run
/// bootstraps) without knowing the final fleet size up front.
#[derive(Debug, Clone)]
pub struct WindowAssignment(Vec<WindowChoice>);

impl WindowAssignment {
    /// The all-floor assignment: the pre-sweep baseline configuration.
    pub fn floor() -> Self {
        Self(vec![WindowChoice::Floor])
    }

    /// The window choice for peer `i`.
    pub fn choice(&self, peer: usize) -> WindowChoice {
        self.0[peer % self.0.len()]
    }
}

/// Strategy for one peer's window choice.
///
/// The budget arm spans 4 KiB to beyond 16 MiB: at suite-scale corpora
/// that range covers budgets the solve resolves back to one slot, budgets
/// that bind between the floor and the population clamp, and budgets wide
/// enough to be population-clamped like the default
/// (`tests/window_sweep.rs` pins those regimes). Shrinking heads toward
/// [`WindowChoice::Floor`], the historically-tested baseline.
pub fn arb_window_choice() -> impl Strategy<Value = WindowChoice> {
    prop_oneof![
        2 => Just(WindowChoice::Floor),
        3 => (12u32..=24, 0usize..4096)
            .prop_map(|(pow, jitter)| WindowChoice::Budget((1usize << pow) + jitter)),
        2 => Just(WindowChoice::Default),
    ]
}

/// Strategy for a fleet-wide window assignment: a non-empty pool of
/// choices, indexed modulo by peer.
///
/// Independent per-slot draws put
/// asymmetric sessions (differently-configured endpoints) in the
/// population; a length-1 assignment (the shrink direction) is a uniform
/// fleet.
pub fn arb_window_assignment() -> impl Strategy<Value = WindowAssignment> {
    proptest::collection::vec(arb_window_choice(), 1..=8).prop_map(WindowAssignment)
}
