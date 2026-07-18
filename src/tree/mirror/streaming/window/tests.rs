use proptest::prelude::*;

use super::{DEFAULT_MAX_IN_FLIGHT_NODES, FAN, SATURABLE_LEVELS, Window};

/// The production default admits two fully fanned levels per edge: a full
/// level of 256 scopes opening full fans yields 256² next-level scopes,
/// and the derived per-edge capacity holds exactly that many.
#[test]
fn default_nodes_admit_a_full_cascade() {
    assert_eq!(
        Window::from_nodes(DEFAULT_MAX_IN_FLIGHT_NODES).scopes(),
        FAN * FAN
    );
}

/// A zero node budget still yields the one-slot liveness floor: capacity
/// zero would be a channel that can never carry an item, not a window.
#[test]
fn zero_budget_is_the_floor() {
    assert_eq!(Window::from_nodes(0), Window::FLOOR);
}

/// Test builds default to the liveness floor, keeping every schedule
/// exercised at the capacity where a bad ordering would deadlock.
#[test]
fn test_default_is_the_floor() {
    assert_eq!(Window::default(), Window::FLOOR);
}

proptest! {
    /// The derivation stays inside its global budget, tightly.
    ///
    /// The admitted scopes, priced at a full fan across every saturable
    /// level, never exceed the requested node budget (once it covers the
    /// one-slot liveness floor), and one more scope per edge would exceed
    /// it.
    #[test]
    fn scopes_stay_inside_the_budget_tightly(nodes in (FAN * SATURABLE_LEVELS)..=1usize << 40) {
        let scopes = Window::from_nodes(nodes).scopes();
        prop_assert!(scopes * FAN * SATURABLE_LEVELS <= nodes);
        prop_assert!((scopes + 1) * FAN * SATURABLE_LEVELS > nodes);
    }
}
