//! Join transfers content and identity, and returns usable state on
//! failure.

use rumors_testkit::common;
use rumors_testkit::common::wire::ROOMY_CAPACITY;

use std::sync::{Arc, Mutex};

use proptest::prelude::*;
use rumors::{Connection, Join, MaybeBookmark, Peer};

use crate::common::action::{arb_local_actions, arb_string_actions, build_local};
use crate::common::flaky::{DurableStore, FaultFeed, FlakyInMemoryBookmark, persisted_record};
use crate::common::oracle::readout;
use crate::common::wire::{assert_connection_drained, block_on, join_fork, wire_gossip};

use rumors_testkit::common::property::SESSION_DIVISOR;
use rumors_testkit::common::seed::{NetworkSeed, network_seed};
use rumors_testkit::divided_proptest;
use serde::Serialize;
use serde::de::DeserializeOwned;

/// A join outcome with its connection halves released, so it can outlive the
/// connection the session ran on.
type Outcome<B> = Join<u64, (), (), B>;

/// Release a join outcome's connection halves, keeping the outcome.
fn released<B: MaybeBookmark, R, W>(joined: Join<u64, R, W, B>) -> Outcome<B> {
    match joined {
        Join::Connected { peer, connection } => Join::Connected {
            peer,
            connection: released_connection(connection),
        },
        Join::Bounced { builder, .. } => Join::Bounced {
            builder,
            read: (),
            write: (),
        },
        Join::Unbookmarked {
            unbookmarked,
            connection,
        } => Join::Unbookmarked {
            unbookmarked,
            connection: released_connection(connection),
        },
        Join::BookmarkFailed { builder, error, .. } => Join::BookmarkFailed {
            builder,
            error,
            read: (),
            write: (),
        },
        Join::Failed { error, builder } => Join::Failed { error, builder },
    }
}

/// Release a connection's halves, keeping whether it was lost and why.
fn released_connection<R, W>(connection: Connection<R, W>) -> Connection<(), ()> {
    match connection {
        Connection::Reusable { .. } => Connection::Reusable {
            read: (),
            write: (),
        },
        Connection::Lost { error } => Connection::Lost { error },
    }
}

/// Join from an established provider and return the joined replica.
fn wire_joiner<T>(provider: &Peer<T>) -> Peer<T>
where
    T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
{
    block_on(async move {
        let (mut a_end, mut b_end) = rumors::testing::memory_connection(ROOMY_CAPACITY);

        let (provider_out, join_out) = tokio::join!(
            provider.gossip_once(&mut a_end.read, &mut a_end.write),
            Peer::<T>::builder()
                .sync_memory_budget(0)
                .join(&mut b_end.read, &mut b_end.write),
        );
        provider_out.expect("provider gossip");
        let Join::Connected {
            peer: joined,
            connection: Connection::Reusable { .. },
        } = join_out
        else {
            panic!("an established provider must serve the join");
        };
        assert_connection_drained(a_end, b_end);
        joined
    })
}

proptest! {
    /// Join copies the provider's exact live set without changing it.
    /// A first message from the newcomer then survives reconciliation,
    /// showing that its inherited floor covers the provider's history.
    #[test]
    fn join_reproduces_a_fork(actions in arb_local_actions(), network in network_seed()) {
        let seed = network.builder::<u64>().sync_memory_budget(0).seed();
        let provider = build_local(join_fork(&seed), &actions);
        let control = readout(&provider.snapshot());

        let joiner = wire_joiner(&provider);

        prop_assert_eq!(
            readout(&joiner.snapshot()), control.clone(),
            "joined content must match the provider's live set",
        );
        prop_assert_eq!(
            readout(&provider.snapshot()), control,
            "serving a join must not change provider content",
        );

        // A stale inherited floor could make this version appear already
        // observed and redacted when the peers reconcile.
        joiner.send(u64::MAX).unwrap();
        wire_gossip(&provider, &joiner);
        prop_assert!(
            provider.snapshot().iter().any(|(_, message)| *message == u64::MAX),
            "the newcomer's origination must survive gossip into the provider",
        );
    }

    /// Join preserves the same content and floor invariants for strings.
    #[test]
    fn join_reproduces_a_fork_string(
        actions in arb_string_actions(),
        network in network_seed(),
    ) {
        let seed = network.builder::<String>().sync_memory_budget(0).seed();
        let provider = build_local(join_fork(&seed), &actions);
        let control = readout(&provider.snapshot());

        let joiner = wire_joiner(&provider);

        prop_assert_eq!(
            readout(&joiner.snapshot()), control.clone(),
            "joined content must match the provider's live set",
        );
        prop_assert_eq!(
            readout(&provider.snapshot()), control,
            "serving a join must not change provider content",
        );

        joiner.send("newcomer's own".to_owned()).unwrap();
        wire_gossip(&provider, &joiner);
        prop_assert!(
            provider
                .snapshot()
                .iter()
                .any(|(_, message)| *message == "newcomer's own"),
            "the newcomer's origination must survive gossip into the provider",
        );
    }
}

divided_proptest! {
    #![divisor(SESSION_DIVISOR)]

    /// A newcomer does not relearn content redacted before it joined.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn join_floor_rejects_stale_redacted_content(network in network_seed()) {
        let provider = network.builder::<u64>().sync_memory_budget(0).seed();
        let version = provider.send(1).unwrap();
        let stale = join_fork(&provider);
        provider.redact(&version);
        let redacted_frontier = provider.snapshot().latest().clone();
        let newcomer = join_fork(&provider);

        assert_eq!(
            newcomer.snapshot().latest(),
            &redacted_frontier,
            "join must inherit the provider's complete frontier",
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
}

/// Two joiners return their builders without stalling or leaving unread
/// bytes.
#[test]
fn both_joining_return_their_builders() {
    let (a_out, b_out) = block_on(async {
        let (mut a_end, mut b_end) =
            rumors::testing::memory_connection(rumors::testing::MEMORY_CAPACITY);

        let (a_out, b_out) = tokio::join!(
            Peer::<u64>::join(&mut a_end.read, &mut a_end.write),
            Peer::<u64>::join(&mut b_end.read, &mut b_end.write),
        );
        let outcome = (released(a_out), released(b_out));
        assert_connection_drained(a_end, b_end);
        outcome
    });

    assert!(
        matches!(a_out, Join::Bounced { .. }),
        "a mutually-joining peer must return its builder",
    );
    assert!(
        matches!(b_out, Join::Bounced { .. }),
        "a mutually-joining peer must return its builder",
    );
}

divided_proptest! {
    #![divisor(SESSION_DIVISOR)]

    /// A zero sync memory budget selected at join can add latency, never
    /// break the session.
    ///
    /// The join completes, delivers the provider's whole set, and the joined
    /// peer, retaining the zero budget, which its sessions raise to
    /// [`MIN_SYNC_MEMORY_BUDGET`](rumors::testing::MIN_SYNC_MEMORY_BUDGET), still
    /// reconciles a fresh origination back into the provider.
    ///
    /// The budget's any-value safety therefore holds at the one entry point
    /// that runs before the peer exists, and the retained setting survives
    /// into the first session where it can bind.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn zero_budget_join_converges(network in network_seed()) {
        let provider = network.builder::<u64>().sync_memory_budget(0).seed();
        provider.send_all([1, 2, 3]).unwrap();

        let joiner = block_on(async {
            let (mut provider_end, mut newcomer_end) =
                rumors::testing::memory_connection(ROOMY_CAPACITY);
            let (served, joined) = tokio::join!(
                provider.gossip_once(&mut provider_end.read, &mut provider_end.write),
                Peer::<u64>::builder()
                    .sync_memory_budget(0)
                    .join(&mut newcomer_end.read, &mut newcomer_end.write),
            );
            served.expect("the provider serves the zero-budget join");
            let newcomer = match joined {
                rumors::Join::Connected { peer, connection: rumors::Connection::Reusable { .. } } => peer,
                _ => panic!("the provider is established"),
            };
            assert_connection_drained(provider_end, newcomer_end);
            newcomer
        });

        assert_eq!(
            readout(&joiner.snapshot()),
            readout(&provider.snapshot()),
            "the zero-budget join must still deliver the provider's whole set",
        );

        assert_eq!(
            joiner.settings().sync_memory_budget,
            rumors::testing::MIN_SYNC_MEMORY_BUDGET,
            "the joined peer retains the zero budget, effective at the minimum",
        );
        // The joined peer gossips under its retained minimum budget, and the
        // session still converges.
        joiner.send(u64::MAX).unwrap();
        wire_gossip(&provider, &joiner);
        assert!(
            provider.snapshot().iter().any(|(_, m)| *m == u64::MAX),
            "the newcomer's origination must survive its zero-budget gossip",
        );
    }
}

/// A fresh durable store and a bookmark over it whose writes fail on `writes`'
/// schedule (an empty schedule never fails).
fn durable_bookmark(writes: Vec<bool>) -> (DurableStore, FlakyInMemoryBookmark) {
    let store: DurableStore = Arc::new(Mutex::new(None));
    let faults = Arc::new(Mutex::new(FaultFeed::new(Vec::new(), writes)));
    let bookmark = FlakyInMemoryBookmark::new(store.clone(), faults, 0);
    (store, bookmark)
}

/// Drive a provider's `gossip` against a *bookmarked* join over an in-memory
/// connection, returning the newcomer's [`Join`] outcome.
fn wire_join(
    provider: &Peer<u64>,
    joining: rumors::PeerBuilder<u64, FlakyInMemoryBookmark>,
) -> Outcome<FlakyInMemoryBookmark> {
    block_on(async move {
        let (mut provider_end, mut newcomer_end) =
            rumors::testing::memory_connection(ROOMY_CAPACITY);
        let (served, joined) = tokio::join!(
            provider.gossip_once(&mut provider_end.read, &mut provider_end.write),
            joining.join(&mut newcomer_end.read, &mut newcomer_end.write),
        );
        served.expect("the provider serves the bookmarked join");
        let joined = released(joined);
        if !matches!(&joined, Join::Failed { .. }) {
            assert_connection_drained(provider_end, newcomer_end);
        }
        joined
    })
}

/// The first peer of the network `network` names, holding three messages: the
/// corpus the `Connected`-arm tests join from.
fn populated_provider(network: NetworkSeed) -> Peer<u64> {
    let provider = network.builder::<u64>().sync_memory_budget(0).seed();
    provider.send_all([1, 2, 3]).unwrap();
    provider
}

divided_proptest! {
    #![divisor(SESSION_DIVISOR)]

    /// The `Connected` arm: a bookmarked join returns only after the received
    /// identity is durably recorded.
    ///
    /// The store holds a record for the joined
    /// network before the caller ever sees the peer, and the peer carries the
    /// provider's whole set.
    ///
    /// The empty-store precondition is the negative
    /// control: the record demonstrably came from this join.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn bookmarked_join_persists_the_arriving_identity(network in network_seed()) {
        let provider = populated_provider(network);
        let (store, bookmark) = durable_bookmark(Vec::new());
        assert!(
            persisted_record(&store).is_empty(),
            "the store must start empty for the persist to be attributable",
        );

        let Join::Connected { peer, connection: Connection::Reusable { .. } } = wire_join(&provider, Peer::builder().bookmark(bookmark))
        else {
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
            readout(&peer.snapshot()),
            readout(&provider.snapshot()),
            "the bookmarked join must still deliver the provider's whole set",
        );
    }

    /// Mutual join leaves storage untouched; retrying the returned builder persists there.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn mutual_bookmarked_bail_returns_the_builder(network in network_seed()) {
        let (store, bookmark) = durable_bookmark(Vec::new());
        let (a_store, a_bookmark) = durable_bookmark(Vec::new());

        let (a_out, b_out) = block_on(async {
            let (mut a_end, mut b_end) =
                rumors::testing::memory_connection(rumors::testing::MEMORY_CAPACITY);
            let (a_out, b_out) = tokio::join!(
                Peer::<u64>::builder()
                    .bookmark(a_bookmark)
                    .join(&mut a_end.read, &mut a_end.write),
                Peer::<u64>::builder()
                    .bookmark(bookmark)
                    .join(&mut b_end.read, &mut b_end.write),
            );
            let outcome = (released(a_out), released(b_out));
            assert_connection_drained(a_end, b_end);
            outcome
        });

        let Join::Bounced { builder: joining, .. } = b_out else {
            panic!("a mutually-joining bookmarked peer must bail");
        };
        assert!(
            matches!(a_out, Join::Bounced { .. }),
            "both sides of a mutual join must bail",
        );
        assert!(
            persisted_record(&store).is_empty() && persisted_record(&a_store).is_empty(),
            "a bail must leave storage untouched",
        );

        // The retry the bail recommends, with the builder it handed back.
        let provider = populated_provider(network);
        let Join::Connected { peer, connection: Connection::Reusable { .. } } = wire_join(&provider, joining) else {
            panic!("the returned builder must serve the retry against a provider");
        };
        assert!(
            persisted_record(&store).contains_key(&peer.network()),
            "the retry must persist into the very storage the bail returned",
        );
    }

    /// A failed session leaves storage untouched and returns a builder that can join another provider.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn failed_bookmarked_join_returns_the_builder(network in network_seed()) {
        let (store, bookmark) = durable_bookmark(Vec::new());

        let outcome = block_on(async {
            let (near, far) = rumors::testing::memory_connection(rumors::testing::MEMORY_CAPACITY);
            // The counterparty hangs up before the session begins: the join's
            // preamble exchange dies on the closed transport.
            drop(far);
            Peer::<u64>::builder()
                .bookmark(bookmark)
                .join(near.read, near.write)
                .await
        });

        let Join::Failed { builder: joining, .. } = outcome else {
            panic!("a dead counterparty must fail the session before a peer exists");
        };
        assert!(
            persisted_record(&store).is_empty(),
            "a failed session must leave storage untouched",
        );

        // The retry the failure permits, with the builder it handed back.
        let provider = populated_provider(network);
        let Join::Connected { peer, connection: Connection::Reusable { .. } } = wire_join(&provider, joining) else {
            panic!("the returned builder must serve the retry against a provider");
        };
        assert!(
            persisted_record(&store).contains_key(&peer.network()),
            "the retry must persist into the very storage the failure returned",
        );
    }

    /// A failed persist returns the received peer with its content and identity intact for retry.
    ///
    /// The claim holds for every network seed.
    #[test]
    fn persist_failure_hands_back_the_live_peer(network in network_seed()) {
        let provider = populated_provider(network);

        // Negative control first: identical join, no injected fault.
        let (_control_store, control_bookmark) = durable_bookmark(Vec::new());
        assert!(
            matches!(
                wire_join(&provider, Peer::builder().bookmark(control_bookmark)),
                Join::Connected { connection: Connection::Reusable { .. }, .. }
            ),
            "with healthy storage the identical join must take the Joined arm",
        );

        // The first (and only) write fails: the eager attach-time persist.
        let (store, bookmark) = durable_bookmark(vec![true]);
        let Join::Unbookmarked {
            unbookmarked,
            connection: Connection::Reusable { .. },
        } =
            wire_join(&provider, Peer::builder().bookmark(bookmark))
        else {
            panic!("a failed persist must surface the live peer as Unbookmarked");
        };
        assert!(
            persisted_record(&store).is_empty(),
            "a failed write must leave the durable bytes untouched",
        );

        // The documented recovery: retry the attach against healthy storage.
        let (retry_store, retry_bookmark) = durable_bookmark(Vec::new());
        let peer = block_on(unbookmarked.retry_bookmark(retry_bookmark))
            .expect("re-attaching against healthy storage must succeed");
        assert!(
            persisted_record(&retry_store).contains_key(&peer.network()),
            "the recovered attach must persist the received identity",
        );

        // The peer is alive and complete: it holds everything the session
        // delivered, so the failure cost a persist attempt and nothing else.
        assert_eq!(
            readout(&peer.snapshot()),
            readout(&provider.snapshot()),
            "the unbookmarked peer must hold the provider's whole set",
        );
    }
}
