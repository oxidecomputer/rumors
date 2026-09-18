//! Bootstrap transfers content and identity, and returns usable state on failure.

mod common;

use std::sync::{Arc, Mutex};

use proptest::prelude::*;
use rumors::{Joined, Peer, Rumors};

use crate::common::action::{arb_local_actions, arb_string_actions, build_local};
use crate::common::flaky::{DurableStore, FaultFeed, FlakyInMemoryBookmark, persisted_record};
use crate::common::oracle::readout;
use crate::common::wire::{assert_control_drained, block_on, bootstrap_fork, wire_gossip};

use serde::Serialize;
use serde::de::DeserializeOwned;
/// Capacity for each in-memory link stream. Roomy enough that the bootstrap
/// descent's largest frames fit without the test depending on backpressure
/// subtleties.
const LINK_BUF: usize = 64 * 1024;

/// Bootstrap from an established provider and return the joined replica.
fn wire_bootstrap<T>(provider: &Rumors<T>) -> Rumors<T>
where
    T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
{
    block_on(async move {
        let (mut a_link, mut b_link) = rumors::link::memory_with_capacity(LINK_BUF);

        let (provider_out, bootstrap_out) = tokio::join!(
            provider.gossip_once(&mut a_link),
            Peer::<T>::bootstrap().join(&mut b_link),
        );
        provider_out.expect("provider gossip");
        let Joined::Joined { peer } = bootstrap_out else {
            panic!("an established provider must serve the bootstrap");
        };
        let joined = peer.sync_window_floor().into_rumors();
        assert_control_drained(a_link, b_link);
        joined
    })
}

proptest! {
    /// Bootstrap copies the provider's exact live set without changing it.
    /// A first message from the newcomer then survives reconciliation,
    /// showing that its inherited floor covers the provider's history.
    #[test]
    fn bootstrap_reproduces_a_fork(actions in arb_local_actions()) {
        let seed = Peer::<u64>::seed().sync_window_floor().into_rumors();
        let provider = build_local(bootstrap_fork(&seed), &actions);
        let control = readout(&provider.snapshot());

        let bootstrapped = wire_bootstrap(&provider);

        prop_assert_eq!(
            readout(&bootstrapped.snapshot()), control.clone(),
            "bootstrapped content must match the provider's live set",
        );
        prop_assert_eq!(
            readout(&provider.snapshot()), control,
            "serving a bootstrap must not change provider content",
        );

        // A stale inherited floor could make this version appear already
        // observed and redacted when the peers reconcile.
        bootstrapped.send(u64::MAX).unwrap();
        wire_gossip(&provider, &bootstrapped);
        prop_assert!(
            provider.snapshot().iter().any(|(_, message)| *message == u64::MAX),
            "the newcomer's origination must survive gossip into the provider",
        );
    }

    /// Bootstrap preserves the same content and floor invariants for strings.
    #[test]
    fn bootstrap_reproduces_a_fork_string(actions in arb_string_actions()) {
        let seed = Peer::<String>::seed().sync_window_floor().into_rumors();
        let provider = build_local(bootstrap_fork(&seed), &actions);
        let control = readout(&provider.snapshot());

        let bootstrapped = wire_bootstrap(&provider);

        prop_assert_eq!(
            readout(&bootstrapped.snapshot()), control.clone(),
            "bootstrapped content must match the provider's live set",
        );
        prop_assert_eq!(
            readout(&provider.snapshot()), control,
            "serving a bootstrap must not change provider content",
        );

        bootstrapped.send("newcomer's own".to_owned()).unwrap();
        wire_gossip(&provider, &bootstrapped);
        prop_assert!(
            provider
                .snapshot()
                .iter()
                .any(|(_, message)| *message == "newcomer's own"),
            "the newcomer's origination must survive gossip into the provider",
        );
    }
}

/// A newcomer does not relearn content redacted before it joined.
#[test]
fn bootstrap_floor_rejects_stale_redacted_content() {
    let provider = Peer::<u64>::seed().sync_window_floor().into_rumors();
    let version = provider.send(1).unwrap();
    let stale = bootstrap_fork(&provider);
    provider.redact(&version);
    let redacted_frontier = provider.snapshot().latest().clone();
    let newcomer = bootstrap_fork(&provider);

    assert_eq!(
        newcomer.snapshot().latest(),
        &redacted_frontier,
        "bootstrap must inherit the provider's complete frontier",
    );

    wire_gossip(&newcomer, &stale);

    for peer in [&newcomer, &stale] {
        assert!(
            !peer.snapshot().contains(&version),
            "the stale copy must remain redacted after gossip",
        );
    }
    assert!(
        newcomer.snapshot().latest() >= version,
        "the newcomer must inherit a frontier that covers the redacted version",
    );
}

/// Two bootstrappers return their builders without stalling or leaving unread control bytes.
#[test]
fn both_bootstrapping_return_their_builders() {
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
        matches!(a_out, Joined::Bailed { .. }),
        "a mutually-bootstrapping peer must return its builder",
    );
    assert!(
        matches!(b_out, Joined::Bailed { .. }),
        "a mutually-bootstrapping peer must return its builder",
    );
}

/// A zero sync memory budget selected at bootstrap can add latency, never
/// break the session.
///
/// The join completes, delivers the provider's whole
/// set, and the joined peer — retaining the zero budget for its own
/// sessions — still reconciles a fresh origination back into the provider.
///
/// The budget's any-value safety therefore holds at the one entry point
/// that runs before the peer exists, and the retained setting survives
/// into the first session where it can bind.
#[test]
fn zero_budget_bootstrap_converges() {
    let provider = Peer::<u64>::seed().sync_window_floor().into_rumors();
    provider.send_all([1, 2, 3]).unwrap();

    let bootstrapped = block_on(async {
        let (mut provider_link, mut newcomer_link) = rumors::link::memory_with_capacity(LINK_BUF);
        let (served, joined) = tokio::join!(
            provider.gossip_once(&mut provider_link),
            Peer::<u64>::bootstrap()
                .sync_memory_budget(0)
                .join(&mut newcomer_link),
        );
        served.expect("the provider serves the zero-budget bootstrap");
        let newcomer = (match joined {
            rumors::Joined::Joined { peer } => peer,
            _ => panic!("the provider is established"),
        })
        .into_rumors();
        assert_control_drained(provider_link, newcomer_link);
        newcomer
    });

    assert_eq!(
        readout(&bootstrapped.snapshot()),
        readout(&provider.snapshot()),
        "the zero-budget join must still deliver the provider's whole set",
    );

    // The joined peer gossips under its retained zero budget: every
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
fn wire_join(
    provider: &Rumors<u64>,
    bootstrap: rumors::Bootstrap<u64, FlakyInMemoryBookmark>,
) -> Joined<u64, FlakyInMemoryBookmark> {
    block_on(async move {
        let (mut provider_link, mut newcomer_link) = rumors::link::memory_with_capacity(LINK_BUF);
        let (served, joined) = tokio::join!(
            provider.gossip_once(&mut provider_link),
            bootstrap.join(&mut newcomer_link),
        );
        served.expect("the provider serves the bookmarked bootstrap");
        if !matches!(&joined, Joined::Failed { .. }) {
            assert_control_drained(provider_link, newcomer_link);
        }
        joined
    })
}

/// A provider holding three messages, the corpus the `Joined`-arm tests
/// bootstrap from.
fn populated_provider() -> Rumors<u64> {
    let provider = Peer::<u64>::seed().sync_window_floor().into_rumors();
    provider.send_all([1, 2, 3]).unwrap();
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

    let Joined::Joined { peer } = wire_join(&provider, Peer::bootstrap().bookmark(bookmark)) else {
        panic!("an established provider and healthy storage must produce a joined peer");
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

/// Mutual bootstrap leaves storage untouched; retrying the returned builder persists there.
#[test]
fn mutual_bookmarked_bail_returns_the_builder() {
    let (store, bookmark) = durable_bookmark(Vec::new());
    let (a_store, a_bookmark) = durable_bookmark(Vec::new());

    let (a_out, b_out) = block_on(async {
        let (mut a_link, mut b_link) = rumors::link::memory();
        let outcome = tokio::join!(
            Peer::<u64>::bootstrap()
                .bookmark(a_bookmark)
                .join(&mut a_link),
            Peer::<u64>::bootstrap()
                .bookmark(bookmark)
                .join(&mut b_link),
        );
        assert_control_drained(a_link, b_link);
        outcome
    });

    let Joined::Bailed { bootstrap } = b_out else {
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

    // The retry the bail recommends, with the builder it handed back.
    let provider = populated_provider();
    let Joined::Joined { peer } = wire_join(&provider, bootstrap) else {
        panic!("the returned builder must serve the retry against a provider");
    };
    assert!(
        persisted_record(&store).contains_key(&peer.network()),
        "the retry must persist into the very storage the bail returned",
    );
}

/// A failed session leaves storage untouched and returns a builder that can join another provider.
#[test]
fn failed_bookmarked_join_returns_the_builder() {
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

    let Joined::Failed { bootstrap, .. } = outcome else {
        panic!("a dead counterparty must fail the session before a peer exists");
    };
    assert!(
        persisted_record(&store).is_empty(),
        "a failed session must leave storage untouched",
    );

    // The retry the failure permits, with the builder it handed back.
    let provider = populated_provider();
    let Joined::Joined { peer } = wire_join(&provider, bootstrap) else {
        panic!("the returned builder must serve the retry against a provider");
    };
    assert!(
        persisted_record(&store).contains_key(&peer.network()),
        "the retry must persist into the very storage the failure returned",
    );
}

/// A failed persist returns the received peer with its content and identity intact for retry.
#[test]
fn persist_failure_hands_back_the_live_peer() {
    let provider = populated_provider();

    // Negative control first: identical join, no injected fault.
    let (_control_store, control_bookmark) = durable_bookmark(Vec::new());
    assert!(
        matches!(
            wire_join(&provider, Peer::bootstrap().bookmark(control_bookmark)),
            Joined::Joined { .. }
        ),
        "with healthy storage the identical join must take the Joined arm",
    );

    // The first (and only) write fails: the eager attach-time persist.
    let (store, bookmark) = durable_bookmark(vec![true]);
    let Joined::Unbookmarked(unbookmarked) =
        wire_join(&provider, Peer::bootstrap().bookmark(bookmark))
    else {
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
