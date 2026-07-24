//! The [`Bootstrap`] builder's configuration plumbing: what the builder
//! holds, and what the minted [`Peer`] retains.
//!
//! The knobs' *behavioral* contracts are pinned end to end elsewhere —
//! run sizing under the exchanged minimum in `tests/target_message_size.rs`,
//! protocol persistence in `tests/bootstrap.rs`, the session bytes in
//! `tests/bootstrap_snapshot.rs`. This suite pins the plumbing those tests
//! rest on: every builder knob reaches the builder's state, and every
//! stored choice reaches the minted peer unchanged.

use super::Bootstrap;
use crate::tree::mirror::streaming::remote::RunBudget;
use crate::tree::mirror::streaming::window::{DEFAULT_SYNC_MEMORY_BUDGET, WindowConfig};
use crate::{Peer, Protocol};

/// A non-default budget distinguishable from every constant the defaults
/// could alias.
const CUSTOM_BUDGET: usize = 7 * 1024 * 1024;

/// A non-default run target, small enough that no default could equal it.
const CUSTOM_TARGET: usize = 4 * 1024;

/// Unwrap the builder's window choice as a byte budget.
fn budget_bytes(window: WindowConfig) -> usize {
    match window {
        WindowConfig::Budget(bytes) => bytes,
        WindowConfig::Fixed(_) => panic!("the builder only ever selects budget windows"),
    }
}

/// Serve one bootstrap from `provider` and hand back the minted peer.
fn join_from_seed(config: Bootstrap<u64>) -> Peer<u64> {
    pollster::block_on(async {
        let provider = Peer::<u64>::seed().into_rumors();
        let (mut near, mut far) = crate::link::memory();
        let (served, joined) = tokio::join!(provider.gossip(&mut far), config.join(&mut near));
        served.expect("the provider serves the bootstrap");
        joined
            .expect("the bootstrap session completes")
            .expect("the provider is established, not itself bootstrapping")
    })
}

/// A zero-configuration builder is exactly the crate's default peer
/// configuration: the same protocol, budget, and run target
/// [`Peer::seed`] starts with, so joining and seeding cannot diverge on
/// defaults.
#[test]
fn defaults_match_the_seed_configuration() {
    let config = Peer::<u64>::bootstrap();
    assert_eq!(config.protocol, Protocol::default());
    assert_eq!(budget_bytes(config.window), DEFAULT_SYNC_MEMORY_BUDGET);
    assert_eq!(config.run_budget, RunBudget::default());
}

/// Each knob stores exactly the selected value, through the same
/// constructors as the matching [`Peer`] methods.
///
/// The budget lands as a budget-window choice, and the run target
/// saturates at the framing ceiling exactly as
/// [`RunBudget::from_bytes`] does.
#[test]
fn knobs_store_the_selected_values() {
    let config = Peer::<u64>::bootstrap()
        .protocol(Protocol::V1)
        .sync_memory_budget(CUSTOM_BUDGET)
        .target_message_size(CUSTOM_TARGET);
    assert_eq!(config.protocol, Protocol::V1);
    assert_eq!(budget_bytes(config.window), CUSTOM_BUDGET);
    assert_eq!(config.run_budget, RunBudget::from_bytes(CUSTOM_TARGET));

    // The saturating constructor is shared, not reimplemented: an
    // over-ceiling target stores what `RunBudget::from_bytes` saturates
    // it to (the framing ceiling, pinned in the budget module's tests),
    // and in particular not the raw value.
    let saturated = Peer::<u64>::bootstrap().target_message_size(usize::MAX);
    assert_eq!(saturated.run_budget, RunBudget::from_bytes(usize::MAX));
    assert_ne!(saturated.run_budget.bytes(), usize::MAX);
}

/// The minted peer retains every builder choice for its later sessions.
///
/// The configured budget and run target arrive on the [`Peer`] exactly
/// as if selected through [`Peer::sync_memory_budget`] and
/// [`Peer::target_message_size`].
#[test]
fn minted_peer_retains_the_configuration() {
    let peer = join_from_seed(
        Peer::<u64>::bootstrap()
            .sync_memory_budget(CUSTOM_BUDGET)
            .target_message_size(CUSTOM_TARGET),
    );
    assert_eq!(peer.protocol, Protocol::V2);
    assert_eq!(budget_bytes(peer.window), CUSTOM_BUDGET);
    assert_eq!(peer.run_budget, RunBudget::from_bytes(CUSTOM_TARGET));
}

/// Negative control for [`minted_peer_retains_the_configuration`]: an
/// unconfigured join mints a peer at the crate defaults, so the retention
/// test above cannot pass by the defaults happening to equal the custom
/// values.
#[test]
fn unconfigured_join_mints_the_defaults() {
    let peer = join_from_seed(Peer::<u64>::bootstrap());
    assert_eq!(peer.protocol, Protocol::default());
    assert_eq!(budget_bytes(peer.window), DEFAULT_SYNC_MEMORY_BUDGET);
    assert_eq!(peer.run_budget, RunBudget::default());
    assert_ne!(budget_bytes(peer.window), CUSTOM_BUDGET);
    assert_ne!(peer.run_budget, RunBudget::from_bytes(CUSTOM_TARGET));
}
