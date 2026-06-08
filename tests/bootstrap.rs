//! Integration tests for remote bootstrap (`rumors::Known::bootstrap`): a
//! stateless peer obtaining a fully-formed `Known` from a peer that drives
//! `gossip` concurrently. Mirrors `async_wire.rs`'s setup — building peers from
//! the shared `Insert`/`Redact` action shape and driving both ends over a
//! `tokio::io::duplex` pipe with `tokio::join!`.

mod common;

use std::sync::Arc;

use borsh::{BorshDeserialize, BorshSerialize};
use proptest::prelude::*;
use rumors::{Key, Known};

use crate::common::action::{arb_local_actions, arb_string_actions, build_local_async};
use crate::common::wire::block_on;

/// Capacity for the in-memory duplex pipe. Roomy enough that the provider's
/// whole-tree frame (the bootstrap transfer ships it in one frame) fits without
/// the test depending on backpressure subtleties.
const DUPLEX_BUF: usize = 64 * 1024;

/// Drive a provider's `gossip` against a peer's `bootstrap` over a duplex pipe,
/// returning the (post-serve) provider and whatever the bootstrapper produced.
fn wire_bootstrap<T>(provider: Known<T>) -> (Known<T>, Option<Known<T>>)
where
    T: BorshSerialize + BorshDeserialize + Send + Sync + 'static,
{
    block_on(async move {
        let (a_side, b_side) = tokio::io::duplex(DUPLEX_BUF);
        let (mut a_r, mut a_w) = tokio::io::split(a_side);
        let (mut b_r, mut b_w) = tokio::io::split(b_side);

        let (provider_out, bootstrap_out) = tokio::join!(
            provider.gossip(&mut a_r, &mut a_w),
            Known::<T>::bootstrap(&mut b_r, &mut b_w),
        );
        (
            provider_out.expect("provider gossip"),
            bootstrap_out.expect("bootstrap handshake"),
        )
    })
}

proptest! {
    /// Bootstrapping from a provider yields the same live content as a local
    /// [`fork`](Known::fork) of that provider, leaves the provider's own
    /// content untouched, and mints a *disjoint* party — proven behaviorally
    /// by the two parties successfully [`join`](Known::join)ing afterward
    /// (`join` is `Ok` exactly when the parties are disjoint).
    #[test]
    fn bootstrap_reproduces_a_fork(actions in arb_local_actions()) {
        let mut seed = Known::<u64>::seed();
        let mut provider = block_on(build_local_async(seed.fork(), &actions));

        // A local fork is the oracle: identical content, disjoint party.
        let control = provider.fork();

        let (mut provider_after, bootstrapped) = wire_bootstrap(provider);
        let bootstrapped = bootstrapped.expect("provider served the bootstrap");

        prop_assert_eq!(&control, &bootstrapped, "bootstrapped content must match a local fork");
        prop_assert_eq!(&control, &provider_after, "serving a bootstrap must not change provider content");

        // The minted party is disjoint from the provider's retained half, so
        // the two can be rejoined without error.
        prop_assert!(
            provider_after.join(bootstrapped).is_ok(),
            "bootstrapped party must be disjoint from the provider's",
        );
    }

    /// `String`-`T` variant of [`bootstrap_reproduces_a_fork`]: the same
    /// invariant for a non-primitive value type, exercising the borsh
    /// round-trip of the whole-tree frame for `T = String`.
    #[test]
    fn bootstrap_reproduces_a_fork_string(actions in arb_string_actions()) {
        let mut seed = Known::<String>::seed();
        let mut provider = block_on(build_local_async(seed.fork(), &actions));

        let control = provider.fork();

        let (mut provider_after, bootstrapped) = wire_bootstrap(provider);
        let bootstrapped = bootstrapped.expect("provider served the bootstrap");

        prop_assert_eq!(&control, &bootstrapped, "bootstrapped content must match a local fork");
        prop_assert_eq!(&control, &provider_after, "serving a bootstrap must not change provider content");
        prop_assert!(
            provider_after.join(bootstrapped).is_ok(),
            "bootstrapped party must be disjoint from the provider's",
        );
    }

    /// `bootstrap_then` invokes its callback exactly once per live message in
    /// the received tree, with the correct `(Key, value)` — and, because the
    /// `Key`s are stable across peers, the observed set equals the provider's
    /// own live set. Empty action sequences exercise the zero-message case.
    #[test]
    fn bootstrap_then_observes_every_live_message(actions in arb_local_actions()) {
        let mut seed = Known::<u64>::seed();
        let provider = block_on(build_local_async(seed.fork(), &actions));

        // The provider's live (Key, value) set, captured before `gossip` moves it.
        let mut expected: Vec<(Key, u64)> =
            provider.iter().map(|(k, _, v)| (k, *v.as_ref())).collect();
        expected.sort();

        let mut learned: Vec<(Key, u64)> = block_on(async move {
            let (a_side, b_side) = tokio::io::duplex(DUPLEX_BUF);
            let (mut a_r, mut a_w) = tokio::io::split(a_side);
            let (mut b_r, mut b_w) = tokio::io::split(b_side);

            let mut learned: Vec<(Key, u64)> = Vec::new();
            let (provider_out, bootstrap_out) = tokio::join!(
                provider.gossip(&mut a_r, &mut a_w),
                Known::<u64>::bootstrap_then(&mut b_r, &mut b_w, |k, _, v: &Arc<u64>| {
                    learned.push((k, *v.as_ref()));
                    async {}
                }),
            );
            provider_out.expect("provider gossip");
            bootstrap_out.expect("bootstrap handshake").expect("provider served the bootstrap");
            learned
        });
        learned.sort();

        prop_assert_eq!(learned, expected);
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

/// The synchronous wrapper ([`rumors::sync::Known::bootstrap`]) bootstraps over
/// blocking [`std::io::pipe`]s with the provider on another thread, reproducing
/// the async happy path: matching content and a disjoint, rejoinable party.
#[test]
fn sync_bootstrap_reproduces_a_fork() {
    use rumors::sync::Known as SyncKnown;
    use std::io::pipe;
    use std::thread;

    let mut provider = SyncKnown::<u64>::seed();
    provider.message([10u64, 20, 30]);
    let control = provider.fork();

    // provider → bootstrapper, and bootstrapper → provider.
    let (mut p2b_r, mut p2b_w) = pipe().expect("pipe provider→bootstrapper");
    let (mut b2p_r, mut b2p_w) = pipe().expect("pipe bootstrapper→provider");

    let boot_thread = thread::spawn(move || {
        SyncKnown::<u64>::bootstrap(&mut p2b_r, &mut b2p_w).expect("bootstrap handshake")
    });

    let mut provider_after = provider
        .gossip(&mut b2p_r, &mut p2b_w)
        .expect("provider gossip");
    let bootstrapped = boot_thread
        .join()
        .expect("join bootstrap thread")
        .expect("provider served the bootstrap");

    assert_eq!(
        control, bootstrapped,
        "bootstrapped content must match a local fork"
    );
    assert_eq!(
        control, provider_after,
        "serving must not change provider content"
    );
    assert!(
        provider_after.join(bootstrapped).is_ok(),
        "bootstrapped party must be disjoint from the provider's",
    );
}
