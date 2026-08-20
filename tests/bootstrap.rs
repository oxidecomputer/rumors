//! Integration tests for remote bootstrap (`rumors::Peer::bootstrap`): a
//! stateless peer obtaining a fully-formed `Peer` from a peer that drives
//! `gossip` concurrently.
//!
//! Also covers every arm of the bookmarked builder's [`Joined`] outcome.
//! Mirrors `async_wire.rs`'s setup — building peers
//! from the shared `Insert`/`Redact` action shape and driving both ends over
//! an in-memory [`rumors::link`] pair with `tokio::join!`.

mod common;

use std::sync::{Arc, Mutex};

use proptest::prelude::*;
use rumors::{Joined, Peer, Rumors};
#[cfg(feature = "protocol-v1")]
use rumors::{Protocol, Retire};

use crate::common::action::{arb_local_actions, arb_string_actions, build_local};
use crate::common::flaky::{DurableStore, FaultFeed, FlakyInMemoryBookmark, persisted_record};
use crate::common::oracle::readout;
use crate::common::wire::{
    assert_control_drained, batch_send, block_on, bootstrap_fork, wire_gossip,
};

use serde::Serialize;
use serde::de::DeserializeOwned;
/// Capacity for each in-memory link stream. Roomy enough that the bootstrap
/// descent's largest frames fit without the test depending on backpressure
/// subtleties.
const LINK_BUF: usize = 64 * 1024;

/// Drive a provider's `gossip` against a peer's `bootstrap` over an in-memory
/// link, returning whatever the bootstrapper produced.
fn wire_bootstrap<T>(provider: &Rumors<T>) -> Option<Rumors<T>>
where
    T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
{
    block_on(async move {
        let (mut a_link, mut b_link) = rumors::link::memory_with_capacity(LINK_BUF);

        let (provider_out, bootstrap_out) = tokio::join!(
            provider.gossip(&mut a_link),
            Peer::<T>::bootstrap().join(&mut b_link),
        );
        provider_out.expect("provider gossip");
        let minted = bootstrap_out
            .expect("bootstrap handshake")
            .map(|peer| peer.sync_window_floor().into_rumors());
        assert_control_drained(a_link, b_link);
        minted
    })
}

proptest! {
    /// Bootstrapping from a provider yields exactly the provider's live
    /// content, message identities included (versions are stable across
    /// peers), leaves the provider's own content untouched, and mints a
    /// *disjoint* party.
    ///
    /// Disjointness is proven behaviorally: a message the newcomer originates
    /// survives a gossip round back into the provider, which a non-disjoint or
    /// stale-floored party would silently destroy.
    #[test]
    fn bootstrap_reproduces_a_fork(actions in arb_local_actions()) {
        let seed = Peer::<u64>::seed().sync_window_floor().into_rumors();
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
        bootstrapped.send(u64::MAX).unwrap();
        wire_gossip(&provider, &bootstrapped);
        prop_assert!(
            provider.snapshot().iter().any(|(_, m)| *m == u64::MAX),
            "the newcomer's origination must survive gossip into the provider",
        );
    }

    /// `String`-`T` variant of [`bootstrap_reproduces_a_fork`]: the same
    /// invariant for a non-primitive value type, exercising the wire
    /// round-trip of the whole-tree frame for `T = String`.
    #[test]
    fn bootstrap_reproduces_a_fork_string(actions in arb_string_actions()) {
        let seed = Peer::<String>::seed().sync_window_floor().into_rumors();
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

        bootstrapped.send("newcomer's own".to_string()).unwrap();
        wire_gossip(&provider, &bootstrapped);
        prop_assert!(
            provider.snapshot().iter().any(|(_, m)| *m == "newcomer's own"),
            "the newcomer's origination must survive gossip into the provider",
        );
    }
}

/// When *both* peers declare bootstrapping, neither has state to give: both
/// sides bail with `Ok(None)` after the handshake, and neither deadlocks
/// (the watchdog-free `block_on` returning at all is the liveness proof).
///
/// The mutual bail is a successful session, so it too must leave the
/// control stream drained at the boundary.
#[test]
fn both_bootstrapping_bail_with_none() {
    let (a_out, b_out) = block_on(async {
        let (mut a_link, mut b_link) = rumors::link::memory();

        let outcome = tokio::join!(
            Peer::<u64>::bootstrap().join(&mut a_link),
            Peer::<u64>::bootstrap().join(&mut b_link),
        );
        assert_control_drained(a_link, b_link);
        outcome
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

/// A zero sync memory budget selected at bootstrap can add latency, never
/// break the session.
///
/// The join completes, delivers the provider's whole
/// set, and the minted peer — retaining the zero budget for its own
/// sessions — still reconciles a fresh origination back into the provider.
///
/// The budget's any-value safety therefore holds at the one entry point
/// that runs before the peer exists, and the retained setting survives
/// into the first session where it can bind.
#[test]
fn zero_budget_bootstrap_converges() {
    let provider = Peer::<u64>::seed().sync_window_floor().into_rumors();
    batch_send(&provider, [1, 2, 3]);

    let bootstrapped = block_on(async {
        let (mut provider_link, mut newcomer_link) = rumors::link::memory_with_capacity(LINK_BUF);
        let (served, joined) = tokio::join!(
            provider.gossip(&mut provider_link),
            Peer::<u64>::bootstrap()
                .sync_memory_budget(0)
                .join(&mut newcomer_link),
        );
        served.expect("the provider serves the zero-budget bootstrap");
        let minted = joined
            .expect("a zero budget must not fail the bootstrap handshake")
            .expect("the provider is established")
            .into_rumors();
        assert_control_drained(provider_link, newcomer_link);
        minted
    });

    assert_eq!(
        readout(&bootstrapped.snapshot()),
        readout(&provider.snapshot()),
        "the zero-budget join must still deliver the provider's whole set",
    );

    // The minted peer gossips under its retained zero budget: every
    // window edge at the liveness floor, and the session still converges.
    bootstrapped.send(u64::MAX).unwrap();
    wire_gossip(&provider, &bootstrapped);
    assert!(
        provider.snapshot().iter().any(|(_, m)| *m == u64::MAX),
        "the newcomer's origination must survive its zero-budget gossip",
    );
}

/// A fresh durable store and a bookmark over it whose writes fail on
/// `writes`' schedule (an empty schedule never fails).
fn durable_bookmark(writes: Vec<bool>) -> (DurableStore, FlakyInMemoryBookmark) {
    let store: DurableStore = Arc::new(Mutex::new(None));
    let faults = Arc::new(Mutex::new(FaultFeed::new(Vec::new(), writes)));
    let bookmark = FlakyInMemoryBookmark::new(store.clone(), faults, 0);
    (store, bookmark)
}

/// Drive a provider's `gossip` against a *bookmarked* join over an
/// in-memory link, returning the newcomer's [`Joined`] outcome.
fn wire_bookmarked_join(
    provider: &Rumors<u64>,
    bookmark: FlakyInMemoryBookmark,
) -> Joined<u64, FlakyInMemoryBookmark> {
    block_on(async move {
        let (mut provider_link, mut newcomer_link) = rumors::link::memory_with_capacity(LINK_BUF);
        let (served, joined) = tokio::join!(
            provider.gossip(&mut provider_link),
            Peer::<u64>::bootstrap()
                .bookmark(bookmark)
                .join(&mut newcomer_link),
        );
        served.expect("the provider serves the bookmarked bootstrap");
        joined
    })
}

/// A provider holding three messages, the corpus the `Joined`-arm tests
/// bootstrap from.
fn populated_provider() -> Rumors<u64> {
    let provider = Peer::<u64>::seed().sync_window_floor().into_rumors();
    batch_send(&provider, [1, 2, 3]);
    provider
}

/// The `Joined` arm: a bookmarked join returns only after the received
/// identity is durably recorded.
///
/// The store holds a record for the joined
/// network before the caller ever sees the peer, and the peer carries the
/// provider's whole set.
///
/// The empty-store precondition is the negative
/// control: the record demonstrably came from this join.
#[test]
fn bookmarked_join_persists_the_arriving_identity() {
    let provider = populated_provider();
    let (store, bookmark) = durable_bookmark(Vec::new());
    assert!(
        persisted_record(&store).is_empty(),
        "the store must start empty for the persist to be attributable",
    );

    let Joined::Joined { peer } = wire_bookmarked_join(&provider, bookmark) else {
        panic!("an established provider and healthy storage must mint a joined peer");
    };

    let record = persisted_record(&store);
    let clocks = record
        .get(&peer.network())
        .expect("the record must hold the joined network's identity");
    assert!(
        !clocks.is_empty(),
        "the joined network's entry must record the received identity",
    );
    assert_eq!(
        readout(&peer.into_rumors().snapshot()),
        readout(&provider.snapshot()),
        "the bookmarked join must still deliver the provider's whole set",
    );
}

/// The `Bailed` arm: a mutual bootstrap moves nothing, leaves storage
/// untouched, and hands the bookmark back.
///
/// The returned bookmark is
/// the live storage handle, proven by retrying it against an established
/// provider and finding the record it then writes.
///
/// The retry succeeding is
/// the negative control: a consumed or poisoned bookmark could not take it.
#[test]
fn mutual_bookmarked_bail_returns_the_bookmark() {
    let (store, bookmark) = durable_bookmark(Vec::new());
    let (a_store, a_bookmark) = durable_bookmark(Vec::new());

    let (a_out, b_out) = block_on(async {
        let (mut a_link, mut b_link) = rumors::link::memory();
        tokio::join!(
            Peer::<u64>::bootstrap()
                .bookmark(a_bookmark)
                .join(&mut a_link),
            Peer::<u64>::bootstrap()
                .bookmark(bookmark)
                .join(&mut b_link),
        )
    });

    let Joined::Bailed { bookmark } = b_out else {
        panic!("a mutually-bootstrapping bookmarked peer must bail");
    };
    assert!(
        matches!(a_out, Joined::Bailed { .. }),
        "both sides of a mutual bootstrap must bail",
    );
    assert!(
        persisted_record(&store).is_empty() && persisted_record(&a_store).is_empty(),
        "a bail must leave storage untouched",
    );

    // The retry the bail recommends, with the bookmark it handed back.
    let provider = populated_provider();
    let Joined::Joined { peer } = wire_bookmarked_join(&provider, bookmark) else {
        panic!("the returned bookmark must serve the retry against a provider");
    };
    assert!(
        persisted_record(&store).contains_key(&peer.network()),
        "the retry must persist into the very storage the bail returned",
    );
}

/// The `Failed` arm: a session that dies before any peer is minted leaves
/// storage untouched and hands the bookmark back for the retry.
///
/// The retry
/// succeeding against a live provider is the negative control: the failure
/// consumed nothing but the link.
#[test]
fn failed_bookmarked_join_returns_the_bookmark() {
    let (store, bookmark) = durable_bookmark(Vec::new());

    let outcome = block_on(async {
        let (mut near, far) = rumors::link::memory();
        // The counterparty hangs up before the session begins: the join's
        // preamble exchange dies on the closed transport.
        drop(far);
        Peer::<u64>::bootstrap()
            .bookmark(bookmark)
            .join(&mut near)
            .await
    });

    let Joined::Failed { error: _, bookmark } = outcome else {
        panic!("a dead counterparty must fail the session before a peer exists");
    };
    assert!(
        persisted_record(&store).is_empty(),
        "a failed session must leave storage untouched",
    );

    // The retry the failure permits, with the bookmark it handed back.
    let provider = populated_provider();
    let Joined::Joined { peer } = wire_bookmarked_join(&provider, bookmark) else {
        panic!("the returned bookmark must serve the retry against a provider");
    };
    assert!(
        persisted_record(&store).contains_key(&peer.network()),
        "the retry must persist into the very storage the failure returned",
    );
}

/// The `Unbookmarked` arm: when the session commits but the persist fails,
/// the live peer — holding the received identity and the provider's whole
/// set — comes back inside the outcome rather than being lost.
///
/// Storage is
/// left untouched, and the documented recovery (re-attaching against
/// healthy storage) succeeds.
///
/// The same join under an empty fault schedule
/// is the negative control: the injected write failure is the only thing
/// separating this arm from `Joined`.
#[test]
fn persist_failure_hands_back_the_live_peer() {
    let provider = populated_provider();

    // Negative control first: identical join, no injected fault.
    let (_control_store, control_bookmark) = durable_bookmark(Vec::new());
    assert!(
        matches!(
            wire_bookmarked_join(&provider, control_bookmark),
            Joined::Joined { .. }
        ),
        "with healthy storage the identical join must take the Joined arm",
    );

    // The first (and only) write fails: the eager attach-time persist.
    let (store, bookmark) = durable_bookmark(vec![true]);
    let Joined::Unbookmarked(unbookmarked) = wire_bookmarked_join(&provider, bookmark) else {
        panic!("a failed persist must surface the live peer as Unbookmarked");
    };
    assert!(
        persisted_record(&store).is_empty(),
        "a failed write must leave the durable bytes untouched",
    );

    // The documented recovery: retry the attach against healthy storage.
    let (retry_store, retry_bookmark) = durable_bookmark(Vec::new());
    let peer = block_on(unbookmarked.peer.bookmark(retry_bookmark))
        .expect("re-attaching against healthy storage must succeed");
    assert!(
        persisted_record(&retry_store).contains_key(&peer.network()),
        "the recovered attach must persist the received identity",
    );

    // The peer is alive and complete: it holds everything the session
    // delivered, so the failure cost a persist attempt and nothing else.
    assert_eq!(
        readout(&peer.into_rumors().snapshot()),
        readout(&provider.snapshot()),
        "the unbookmarked peer must hold the provider's whole set",
    );
}

/// Explicit V1 selection applies to both bootstrap and every later session:
/// the original alternating wire remains a usable compatibility path rather
/// than merely a protocol-level test oracle.
#[cfg(feature = "protocol-v1")]
#[test]
fn v1_bootstrap_selection_persists_into_gossip() {
    let provider = Peer::<u64>::seed()
        .sync_window_floor()
        .protocol(Protocol::V1)
        .into_rumors();
    provider.send(1).unwrap();

    let newcomer = block_on(async {
        let (mut provider_link, mut newcomer_link) = rumors::link::memory_with_capacity(LINK_BUF);
        let (served, joined) = tokio::join!(
            provider.gossip(&mut provider_link),
            Peer::<u64>::bootstrap()
                .protocol(Protocol::V1)
                .join(&mut newcomer_link),
        );
        served.expect("V1 provider serves bootstrap");
        let minted = joined
            .expect("V1 bootstrap succeeds")
            .expect("provider is established")
            .into_rumors();
        assert_control_drained(provider_link, newcomer_link);
        minted
    });

    newcomer.send(2).unwrap();
    wire_gossip(&provider, &newcomer);
    assert_eq!(readout(&provider.snapshot()), readout(&newcomer.snapshot()));
    assert_eq!(provider.snapshot().len(), 2);

    let retired = block_on(async {
        let newcomer = newcomer
            .try_into_peer()
            .await
            .expect("sole V1 handle reclaims its peer");
        let (mut provider_link, mut newcomer_link) = rumors::link::memory_with_capacity(LINK_BUF);
        let (served, retired) = tokio::join!(
            provider.gossip(&mut provider_link),
            newcomer.retire(&mut newcomer_link),
        );
        served.expect("V1 provider absorbs retiree");
        assert_control_drained(provider_link, newcomer_link);
        retired
    });
    assert!(matches!(retired, Retire::Retired));
}
