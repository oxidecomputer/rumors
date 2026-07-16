//! Integration tests for remote bootstrap (`rumors::Peer::bootstrap`): a
//! stateless peer obtaining a fully-formed `Peer` from a peer that drives
//! `gossip` concurrently. Mirrors `async_wire.rs`'s setup — building peers
//! from the shared `Insert`/`Redact` action shape and driving both ends over
//! a `tokio::io::duplex` pipe with `tokio::join!`.

mod common;

use proptest::prelude::*;
use rumors::{Peer, Rumors};
#[cfg(feature = "protocol-v1")]
use rumors::{Protocol, Retire};

use crate::common::action::{arb_local_actions, arb_string_actions, build_local};
use crate::common::oracle::readout;
use crate::common::wire::{block_on, bootstrap_fork, wire_gossip};

/// Capacity for the in-memory duplex pipe. Roomy enough that the bootstrap
/// descent's largest frames fit without the test depending on backpressure
/// subtleties.
const DUPLEX_BUF: usize = 64 * 1024;

/// Drive a provider's `gossip` against a peer's `bootstrap` over a duplex
/// pipe, returning whatever the bootstrapper produced.
fn wire_bootstrap<T>(provider: &Rumors<T>) -> Option<Rumors<T>>
where
    T: borsh::BorshSerialize + borsh::BorshDeserialize + Send + Sync + 'static,
{
    block_on(async move {
        let (a_side, b_side) = tokio::io::duplex(DUPLEX_BUF);
        let (mut a_r, mut a_w) = tokio::io::split(a_side);
        let (mut b_r, mut b_w) = tokio::io::split(b_side);

        let (provider_out, bootstrap_out) = tokio::join!(
            provider.gossip(&mut a_r, &mut a_w),
            Peer::<T>::bootstrap(&mut b_r, &mut b_w),
        );
        provider_out.expect("provider gossip");
        bootstrap_out
            .expect("bootstrap handshake")
            .map(Peer::into_rumors)
    })
}

proptest! {
    /// Bootstrapping from a provider yields exactly the provider's live
    /// `(Key, value)` content (keys are stable across peers), leaves the
    /// provider's own content untouched, and mints a *disjoint* party —
    /// proven behaviorally: a message the newcomer originates survives a
    /// gossip round back into the provider, which a non-disjoint or
    /// stale-floored party would silently destroy.
    #[test]
    fn bootstrap_reproduces_a_fork(actions in arb_local_actions()) {
        let seed = Peer::<u64>::seed().into_rumors();
        let provider = build_local(bootstrap_fork(&seed), &actions);

        let control = readout(&provider.snapshot());

        let bootstrapped =
            wire_bootstrap(&provider).expect("provider served the bootstrap");

        prop_assert_eq!(
            readout(&bootstrapped.snapshot()), control.clone(),
            "bootstrapped content must match the provider's live set",
        );
        prop_assert_eq!(
            readout(&provider.snapshot()), control,
            "serving a bootstrap must not change provider content",
        );

        // The minted party is disjoint from the provider's retained half
        // and floored at the served tree's version, so a fresh origination
        // survives reconciliation on both sides.
        bootstrapped.send(u64::MAX);
        wire_gossip(&provider, &bootstrapped);
        prop_assert!(
            provider.snapshot().iter().any(|(_, _, m)| **m == u64::MAX),
            "the newcomer's origination must survive gossip into the provider",
        );
    }

    /// `String`-`T` variant of [`bootstrap_reproduces_a_fork`]: the same
    /// invariant for a non-primitive value type, exercising the borsh
    /// round-trip of the whole-tree frame for `T = String`.
    #[test]
    fn bootstrap_reproduces_a_fork_string(actions in arb_string_actions()) {
        let seed = Peer::<String>::seed().into_rumors();
        let provider = build_local(bootstrap_fork(&seed), &actions);

        let control = readout(&provider.snapshot());

        let bootstrapped =
            wire_bootstrap(&provider).expect("provider served the bootstrap");

        prop_assert_eq!(
            readout(&bootstrapped.snapshot()), control.clone(),
            "bootstrapped content must match the provider's live set",
        );
        prop_assert_eq!(
            readout(&provider.snapshot()), control,
            "serving a bootstrap must not change provider content",
        );

        bootstrapped.send("newcomer's own".to_string());
        wire_gossip(&provider, &bootstrapped);
        prop_assert!(
            provider.snapshot().iter().any(|(_, _, m)| **m == "newcomer's own"),
            "the newcomer's origination must survive gossip into the provider",
        );
    }
}

/// When *both* peers declare bootstrapping, neither has state to give: both
/// sides bail with `Ok(None)` after the handshake, and neither deadlocks
/// (the watchdog-free `block_on` returning at all is the liveness proof).
#[test]
fn both_bootstrapping_bail_with_none() {
    let (a_out, b_out) = block_on(async {
        let (a_side, b_side) = tokio::io::duplex(1024);
        let (mut a_r, mut a_w) = tokio::io::split(a_side);
        let (mut b_r, mut b_w) = tokio::io::split(b_side);

        tokio::join!(
            Peer::<u64>::bootstrap(&mut a_r, &mut a_w),
            Peer::<u64>::bootstrap(&mut b_r, &mut b_w),
        )
    });

    assert!(
        a_out.expect("handshake ok").is_none(),
        "a mutually-bootstrapping peer must bail with None",
    );
    assert!(
        b_out.expect("handshake ok").is_none(),
        "a mutually-bootstrapping peer must bail with None",
    );
}

/// Explicit V1 selection applies to both bootstrap and every later session:
/// the original alternating wire remains a usable compatibility path rather
/// than merely a protocol-level test oracle.
#[cfg(feature = "protocol-v1")]
#[test]
fn v1_bootstrap_selection_persists_into_gossip() {
    let provider = Peer::<u64>::seed().protocol(Protocol::V1).into_rumors();
    provider.send(1);

    let newcomer = block_on(async {
        let (provider_side, newcomer_side) = tokio::io::duplex(DUPLEX_BUF);
        let (mut provider_read, mut provider_write) = tokio::io::split(provider_side);
        let (mut newcomer_read, mut newcomer_write) = tokio::io::split(newcomer_side);
        let (served, joined) = tokio::join!(
            provider.gossip(&mut provider_read, &mut provider_write),
            Peer::<u64>::bootstrap_with_protocol(
                Protocol::V1,
                &mut newcomer_read,
                &mut newcomer_write,
            ),
        );
        served.expect("V1 provider serves bootstrap");
        joined
            .expect("V1 bootstrap succeeds")
            .expect("provider is established")
            .into_rumors()
    });

    newcomer.send(2);
    wire_gossip(&provider, &newcomer);
    assert_eq!(readout(&provider.snapshot()), readout(&newcomer.snapshot()));
    assert_eq!(provider.snapshot().len(), 2);

    let retired = block_on(async {
        let newcomer = newcomer
            .try_into_peer()
            .await
            .expect("sole V1 handle reclaims its peer");
        let (provider_side, newcomer_side) = tokio::io::duplex(DUPLEX_BUF);
        let (mut provider_read, mut provider_write) = tokio::io::split(provider_side);
        let (mut newcomer_read, mut newcomer_write) = tokio::io::split(newcomer_side);
        let (served, retired) = tokio::join!(
            provider.gossip(&mut provider_read, &mut provider_write),
            newcomer.retire(&mut newcomer_read, &mut newcomer_write),
        );
        served.expect("V1 provider absorbs retiree");
        retired
    });
    assert!(matches!(retired, Retire::Retired));
}
