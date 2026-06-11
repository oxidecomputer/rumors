//! Integration tests for remote bootstrap (`rumors::Known::bootstrap`): a
//! stateless peer obtaining a fully-formed `Known` from a peer that drives
//! `gossip` concurrently. Mirrors `async_wire.rs`'s setup — building peers
//! from the shared `Insert`/`Redact` action shape and driving both ends over
//! a `tokio::io::duplex` pipe with `tokio::join!`.

mod common;

use proptest::prelude::*;
use rumors::Known;

use crate::common::action::{arb_local_actions, arb_string_actions, build_local_async};
use crate::common::oracle::readout;
use crate::common::wire::{block_on, bootstrap_fork, wire_gossip};

/// Capacity for the in-memory duplex pipe. Roomy enough that the bootstrap
/// descent's largest frames fit without the test depending on backpressure
/// subtleties.
const DUPLEX_BUF: usize = 64 * 1024;

/// Drive a provider's `gossip` against a peer's `bootstrap` over a duplex
/// pipe, returning whatever the bootstrapper produced.
fn wire_bootstrap<T>(provider: &mut Known<T>) -> Option<Known<T>>
where
    T: borsh::BorshSerialize + borsh::BorshDeserialize + Send + Sync + 'static,
{
    block_on(async move {
        let (a_side, b_side) = tokio::io::duplex(DUPLEX_BUF);
        let (mut a_r, mut a_w) = tokio::io::split(a_side);
        let (mut b_r, mut b_w) = tokio::io::split(b_side);

        let (provider_out, bootstrap_out) = tokio::join!(
            provider.gossip(&mut a_r, &mut a_w),
            Known::<T>::bootstrap(&mut b_r, &mut b_w),
        );
        provider_out.expect("provider gossip");
        bootstrap_out.expect("bootstrap handshake")
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
        let mut seed = Known::<u64>::seed();
        let mut provider = build_local_async(bootstrap_fork(&mut seed), &actions);

        let control = readout(&provider.snapshot());

        let mut bootstrapped =
            wire_bootstrap(&mut provider).expect("provider served the bootstrap");

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
        wire_gossip(&mut provider, &mut bootstrapped);
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
        let mut seed = Known::<String>::seed();
        let mut provider = build_local_async(bootstrap_fork(&mut seed), &actions);

        let control = readout(&provider.snapshot());

        let mut bootstrapped =
            wire_bootstrap(&mut provider).expect("provider served the bootstrap");

        prop_assert_eq!(
            readout(&bootstrapped.snapshot()), control.clone(),
            "bootstrapped content must match the provider's live set",
        );
        prop_assert_eq!(
            readout(&provider.snapshot()), control,
            "serving a bootstrap must not change provider content",
        );

        bootstrapped.send("newcomer's own".to_string());
        wire_gossip(&mut provider, &mut bootstrapped);
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
            Known::<u64>::bootstrap(&mut a_r, &mut a_w),
            Known::<u64>::bootstrap(&mut b_r, &mut b_w),
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

/// The synchronous wrapper ([`rumors::sync::Known::bootstrap`]) bootstraps
/// over blocking [`std::io::pipe`]s with the provider on another thread,
/// reproducing the async happy path: matching content, untouched provider.
#[test]
fn sync_bootstrap_reproduces_a_fork() {
    use rumors::sync::Known as SyncKnown;
    use std::io::pipe;
    use std::thread;

    let mut provider = SyncKnown::<u64>::seed();
    provider.batch().send(10).send(20).send(30);
    let control = readout(&provider.snapshot());

    // provider → bootstrapper, and bootstrapper → provider.
    let (mut p2b_r, mut p2b_w) = pipe().expect("pipe provider→bootstrapper");
    let (mut b2p_r, mut b2p_w) = pipe().expect("pipe bootstrapper→provider");

    let boot_thread = thread::spawn(move || {
        SyncKnown::<u64>::bootstrap(&mut p2b_r, &mut b2p_w).expect("bootstrap handshake")
    });

    provider
        .gossip(&mut b2p_r, &mut p2b_w)
        .expect("provider gossip");
    let bootstrapped = boot_thread
        .join()
        .expect("join bootstrap thread")
        .expect("provider served the bootstrap");

    assert_eq!(
        readout(&bootstrapped.snapshot()),
        control,
        "bootstrapped content must match the provider snapshot"
    );
    assert_eq!(
        readout(&provider.snapshot()),
        control,
        "serving must not change provider content"
    );
}
