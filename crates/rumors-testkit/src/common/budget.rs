//! The budget sweep dimension: a generated per-peer memory budget, so
//! behavioral suites exercise the production default and explicit budgets
//! alongside a ledger budget of zero.
//!
//! A ledger budget of zero stays in the population deliberately: each walk
//! then asks one question at a time, every unit taking its depth's floor
//! slot, the configuration the liveness argument rests on, and suites must
//! keep it exercised. The public setter raises every budget to
//! [`MIN_SYNC_MEMORY_BUDGET`](rumors::MIN_SYNC_MEMORY_BUDGET), so this
//! choice sets the ledger budget through the test path
//! ([`ledger_budget`](rumors::testing::ledger_budget)). The sweep adds
//! explicit byte budgets and the production default on top. Every budget
//! sets aside the floor allowance
//! ([`floor_allowance`](rumors::testing::floor_allowance)) for the floor
//! slots and admits work beyond them against the rest, so the explicit
//! budgets are drawn beyond it. Peers draw choices independently, so
//! sessions between differently-configured endpoints arise throughout the
//! population; each side of such a session admits its work against its
//! own budget.

use crate::bench::session::admitting;
use proptest::prelude::*;

/// One peer's memory budget in the sweep population.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetChoice {
    /// A ledger budget of zero, set through the test path: each walk asks
    /// one question at a time. First variant, so failures shrink toward
    /// the zero-everywhere baseline.
    Zero,
    /// An explicit memory budget in bytes, through the public setter: the
    /// floor allowance and the bytes the ledgers admit work against beside
    /// it, raised to
    /// [`MIN_SYNC_MEMORY_BUDGET`](rumors::MIN_SYNC_MEMORY_BUDGET).
    Bytes(usize),
    /// An explicit ledger budget in bytes, set through the test path
    /// ([`ledger_budget`](rumors::testing::ledger_budget)), below the
    /// public minimum included.
    Ledger(usize),
    /// The production default: no configuration call at all.
    Default,
}

impl BudgetChoice {
    /// Apply this choice to a peer under construction.
    pub fn apply<T: Send + Sync + 'static>(self, peer: rumors::Peer<T>) -> rumors::Peer<T> {
        match self {
            Self::Zero => rumors::testing::ledger_budget(peer, 0),
            Self::Bytes(bytes) => peer.sync_memory_budget(bytes),
            Self::Ledger(bytes) => rumors::testing::ledger_budget(peer, bytes),
            Self::Default => peer,
        }
    }
}

/// A generated budget assignment for a whole fleet: peer `i` takes
/// [`choice(i)`](BudgetAssignment::choice).
///
/// Indexing wraps, so one
/// assignment covers founders and any peers created later (mid-run
/// bootstraps) without knowing the final fleet size up front.
#[derive(Debug, Clone)]
pub struct BudgetAssignment(Vec<BudgetChoice>);

impl BudgetAssignment {
    /// Wrap a non-empty pool of choices as an assignment.
    ///
    /// # Panics
    ///
    /// Panics on an empty pool: an assignment must answer
    /// [`choice`](Self::choice) for every peer.
    pub fn new(choices: Vec<BudgetChoice>) -> Self {
        assert!(
            !choices.is_empty(),
            "a budget assignment needs at least one choice"
        );
        Self(choices)
    }

    /// The all-zero assignment: every peer at a ledger budget of zero.
    ///
    /// The constructor for the suites' deterministic baseline legs: each
    /// swept engine keeps one leg pinned here so the one-question-at-a-time
    /// orderings the liveness argument rests on are exercised on every
    /// iteration, not merely with generated probability.
    pub fn zero() -> Self {
        Self(vec![BudgetChoice::Zero])
    }

    /// The budget choice for peer `i`.
    pub fn choice(&self, peer: usize) -> BudgetChoice {
        self.0[peer % self.0.len()]
    }
}

/// Smallest power-of-two exponent the budget arm draws: budgets admitting
/// from `2^MIN_BUDGET_EXPONENT` bytes, 4 KiB, upward beside the floor
/// allowance.
pub const MIN_BUDGET_EXPONENT: u32 = 12;

/// Largest power-of-two exponent the budget arm draws: budgets admitting
/// up to `2^MAX_BUDGET_EXPONENT` bytes, 16 MiB, beside the floor allowance.
pub const MAX_BUDGET_EXPONENT: u32 = 24;

/// Strategy for one peer's budget choice.
///
/// The budget arm draws the floor allowance and `2^e + jitter` beside it,
/// for `e` across [`MIN_BUDGET_EXPONENT`]`..=`[`MAX_BUDGET_EXPONENT`]
/// ([`admitting`]), through the public setter: the draws below
/// [`MIN_SYNC_MEMORY_BUDGET`](rumors::MIN_SYNC_MEMORY_BUDGET) exercise its
/// raise, and the rest reach budgets that suite-scale sessions do not
/// fill. Shrinking heads toward [`BudgetChoice::Zero`], the baseline the
/// liveness argument rests on.
pub fn arb_budget_choice() -> impl Strategy<Value = BudgetChoice> {
    prop_oneof![
        2 => Just(BudgetChoice::Zero),
        3 => (MIN_BUDGET_EXPONENT..=MAX_BUDGET_EXPONENT, 0usize..4096)
            .prop_map(|(pow, jitter)| BudgetChoice::Bytes(admitting((1usize << pow) + jitter))),
        2 => Just(BudgetChoice::Default),
    ]
}

/// Strategy for a fleet-wide budget assignment: a non-empty pool of
/// choices, indexed modulo by peer.
///
/// Independent per-slot draws put
/// asymmetric sessions (differently-configured endpoints) in the
/// population; a length-1 assignment (the shrink direction) is a uniform
/// fleet.
pub fn arb_budget_assignment() -> impl Strategy<Value = BudgetAssignment> {
    proptest::collection::vec(arb_budget_choice(), 1..=8).prop_map(BudgetAssignment::new)
}
