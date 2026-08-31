//! The window-budget sweep dimension: a generated per-peer choice of
//! reconciliation-window configuration, so behavioral suites exercise the
//! production-default wide-window path alongside the pinned floor.
//!
//! The floor stays in the population deliberately: capacity one is the
//! configuration the deadlock-freedom argument certifies, and suites must
//! keep those orderings exercised. The sweep adds the two budgeted
//! regimes on top — explicit byte budgets resolved by the session's
//! solve, and the production default. Peers draw choices independently,
//! so sessions between differently-configured endpoints (asymmetric
//! windows) arise throughout the population.
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
/// assignment covers founders and any peers created later (mid-run
/// bootstraps) without knowing the final fleet size up front.
#[derive(Debug, Clone)]
pub struct WindowAssignment(Vec<WindowChoice>);

impl WindowAssignment {
    /// Wrap a non-empty pool of choices as an assignment.
    ///
    /// # Panics
    ///
    /// Panics on an empty pool: an assignment must answer
    /// [`choice`](Self::choice) for every peer.
    pub fn new(choices: Vec<WindowChoice>) -> Self {
        assert!(
            !choices.is_empty(),
            "a window assignment needs at least one choice"
        );
        Self(choices)
    }

    /// The all-floor assignment: every peer at the serialization floor.
    ///
    /// The constructor for the suites' deterministic baseline legs: each
    /// swept engine keeps one leg pinned here so the capacity-one
    /// orderings the deadlock-freedom argument certifies are exercised on
    /// every iteration, not merely with generated probability.
    pub fn floor() -> Self {
        Self(vec![WindowChoice::Floor])
    }

    /// The window choice for peer `i`.
    pub fn choice(&self, peer: usize) -> WindowChoice {
        self.0[peer % self.0.len()]
    }
}

/// Smallest power-of-two exponent the budget arm draws: budgets from
/// `2^MIN_BUDGET_EXPONENT` bytes upward.
///
/// `tests/window_sweep.rs` pins that a session at this endpoint's budget
/// grants exactly one slot at suite-scale content, so the
/// budget-resolution path's floor-equivalent regime stays in the
/// population.
pub const MIN_BUDGET_EXPONENT: u32 = 12;

/// Largest power-of-two exponent the budget arm draws.
///
/// `tests/window_sweep.rs` pins that a session at this endpoint's budget
/// grants a window wider than one slot at suite-scale content, so the
/// budget arm provably straddles the width-granting threshold.
pub const MAX_BUDGET_EXPONENT: u32 = 24;

/// Strategy for one peer's window choice.
///
/// The budget arm draws `2^e + jitter` for `e` across
/// [`MIN_BUDGET_EXPONENT`]`..=`[`MAX_BUDGET_EXPONENT`]; the two endpoint
/// regimes (one-slot at the minimum, wider-than-one at the maximum) are
/// pinned by `tests/window_sweep.rs`, so the arm provably spans both
/// sides of the width-granting threshold. Shrinking heads toward
/// [`WindowChoice::Floor`], the deadlock-certified baseline.
pub fn arb_window_choice() -> impl Strategy<Value = WindowChoice> {
    prop_oneof![
        2 => Just(WindowChoice::Floor),
        3 => (MIN_BUDGET_EXPONENT..=MAX_BUDGET_EXPONENT, 0usize..4096)
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
    proptest::collection::vec(arb_window_choice(), 1..=8).prop_map(WindowAssignment::new)
}
