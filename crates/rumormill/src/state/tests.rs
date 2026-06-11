//! Lifecycle tests for [`AppState`], driven with real keys and versions
//! minted by a seeded [`rumors::Known`] (both types are opaque outside the
//! rumors crate, and real versions carry real causality).

use std::collections::HashSet;

use rumors::Known;

use crate::timers;

use super::*;

const ALICE: PeerId = [0xaa; 32];
const BOB: PeerId = [0xbb; 32];

/// Insert `entries` into `known` as one batch and return each one's
/// `(key, version)` in insertion order. Insertion order is causal order
/// here: every insert ticks the same party, so the minted versions come
/// back totally ordered and sorting by them recovers the batch order.
fn mint(known: &Known<Entry>, entries: Vec<Entry>) -> Vec<(Key, Version)> {
    let pre = known.latest();
    {
        let mut batch = known.batch();
        for entry in entries {
            batch.send(entry);
        }
    }
    let snapshot = known.snapshot();
    let mut minted: Vec<(Key, Version)> = snapshot
        .range(rumors::causally::since(&pre))
        .map(|(key, version, _)| (key, version.clone()))
        .collect();
    minted.sort_by(|(_, a), (_, b)| {
        a.partial_cmp(b)
            .expect("one party's versions are totally ordered")
    });
    minted
}

/// A party-disjoint fork of `known`, minted by serving a bootstrap over an
/// in-memory duplex: the only honest source of genuinely concurrent
/// versions.
async fn bootstrap_empty_fork(known: &mut Known<Entry>) -> Known<Entry> {
    let (sa, sb) = tokio::io::duplex(64 * 1024);
    let (mut ar, mut aw) = tokio::io::split(sa);
    let (mut br, mut bw) = tokio::io::split(sb);
    let (served, fork) = tokio::join!(
        known.gossip(&mut ar, &mut aw),
        Known::<Entry>::bootstrap(&mut br, &mut bw),
    );
    served.expect("serve the fork's bootstrap");
    fork.expect("bootstrap handshake")
        .expect("the parent served the bootstrap")
}

fn chat(body: &str, sent_at: Millis) -> Entry {
    Entry::Chat {
        channel: "general".into(),
        author: ALICE,
        body: body.into(),
        sent_at,
        ttl_ms: 60_000,
    }
}

fn beat(peer: PeerId, name: &str, at: Millis) -> Entry {
    Entry::Presence {
        peer,
        name: name.into(),
        at,
    }
}

/// A live message is displayed and gets an expiry scheduled at
/// `sent_at + ttl`; an already-expired one is never displayed, only
/// redacted.
#[tokio::test(flavor = "current_thread")]
async fn expiry_policy_on_arrival() {
    let known: Known<Entry> = Known::seed();
    let entries = vec![chat("live", 1_000), chat("dead", 2_000)];
    let minted = mint(&known, entries.clone());
    let mut state = AppState::new();

    let now = 1_000 + 60_000; // "live" has 1ms left; "dead" expired exactly now
    let (live_key, live_version) = &minted[0];
    let effects = state.observe(*live_key, live_version, &entries[0], now - 1);
    assert_eq!(
        effects,
        vec![Effect::Schedule {
            key: *live_key,
            deadline: 61_000
        }]
    );
    assert!(state.messages.contains_key(live_key));

    let (dead_key, dead_version) = &minted[1];
    let effects = state.observe(*dead_key, dead_version, &entries[1], 62_000);
    assert_eq!(effects, vec![Effect::Redact(*dead_key)]);
    assert!(!state.messages.contains_key(dead_key));
}

/// A message that causally follows the channel tail extends the
/// conversation unflagged; one delivered from a *concurrent* line of
/// history is flagged as a [`Effect::ConcurrentArrival`] for the UI to
/// highlight. Display order is plain arrival order — sound because the
/// owner feeds `observe` from a `CausalMessages` observer.
#[tokio::test(flavor = "current_thread")]
async fn concurrent_arrival_is_flagged() {
    // Two causal lines: alice's chain, and a concurrent message minted by a
    // disjoint fork that never saw it. The fork is minted while both sides
    // are empty, so the two lines share no history.
    let mut known: Known<Entry> = Known::seed();
    let fork = bootstrap_empty_fork(&mut known).await;

    let entries = vec![chat("first", 1_000), chat("second", 2_000)];
    let minted = mint(&known, entries.clone());
    let elsewhere = vec![chat("elsewhere", 3_000)];
    let minted_elsewhere = mint(&fork, elsewhere.clone());

    let mut state = AppState::new();

    // The chain, in causal delivery order: the successor is not flagged.
    let effects = state.observe(minted[0].0, &minted[0].1, &entries[0], 0);
    assert_eq!(
        effects,
        vec![Effect::Schedule {
            key: minted[0].0,
            deadline: 61_000
        }]
    );
    let effects = state.observe(minted[1].0, &minted[1].1, &entries[1], 0);
    assert_eq!(
        effects,
        vec![Effect::Schedule {
            key: minted[1].0,
            deadline: 62_000
        }]
    );

    // The concurrent message arrives last (a later pass): flagged.
    let effects = state.observe(
        minted_elsewhere[0].0,
        &minted_elsewhere[0].1,
        &elsewhere[0],
        0,
    );
    assert_eq!(
        effects,
        vec![
            Effect::Schedule {
                key: minted_elsewhere[0].0,
                deadline: 63_000
            },
            Effect::ConcurrentArrival {
                channel: "general".into(),
                key: minted_elsewhere[0].0
            },
        ]
    );
    // Display order is arrival order.
    let order: Vec<Key> = state.channels["general"].list.clone();
    assert_eq!(order, vec![minted[0].0, minted[1].0, minted_elsewhere[0].0]);
}

/// Presence supersession: the causally newer beat wins regardless of arrival
/// order, and the loser is redacted so stale beats never accumulate.
#[tokio::test(flavor = "current_thread")]
async fn presence_supersession_is_arrival_order_independent() {
    let known: Known<Entry> = Known::seed();
    let entries = vec![beat(ALICE, "alice", 1_000), beat(ALICE, "alice", 2_000)];
    let minted = mint(&known, entries.clone());

    // Old then new: the new beat evicts the old.
    let mut state = AppState::new();
    assert_eq!(
        state.observe(minted[0].0, &minted[0].1, &entries[0], 0),
        vec![]
    );
    let effects = state.observe(minted[1].0, &minted[1].1, &entries[1], 0);
    assert_eq!(effects, vec![Effect::Redact(minted[0].0)]);
    assert_eq!(state.presence[&ALICE].at, 2_000);

    // New then old: the stale arrival is redacted on sight.
    let mut state = AppState::new();
    assert_eq!(
        state.observe(minted[1].0, &minted[1].1, &entries[1], 0),
        vec![]
    );
    let effects = state.observe(minted[0].0, &minted[0].1, &entries[0], 0);
    assert_eq!(effects, vec![Effect::Redact(minted[0].0)]);
    assert_eq!(state.presence[&ALICE].at, 2_000);
}

/// The removal diff: any tracked key absent from the live set is dropped
/// from every display structure and returned for timer cancellation. This is
/// the only path by which a peer's redaction reaches the screen.
#[tokio::test(flavor = "current_thread")]
async fn retain_live_drops_peer_redactions() {
    let known: Known<Entry> = Known::seed();
    let entries = vec![
        chat("kept", 1_000),
        chat("redacted-elsewhere", 2_000),
        beat(BOB, "bob", 3_000),
    ];
    let minted = mint(&known, entries.clone());
    let mut state = AppState::new();
    for ((key, version), entry) in minted.iter().zip(&entries) {
        state.observe(*key, version, entry, 0);
    }

    // A peer redacted the second message and bob's presence.
    let live: HashSet<Key> = [minted[0].0].into();
    let mut dead = state.retain_live(&live);
    dead.sort();
    let mut expected = vec![minted[1].0, minted[2].0];
    expected.sort();
    assert_eq!(dead, expected);
    assert!(state.messages.contains_key(&minted[0].0));
    assert!(!state.messages.contains_key(&minted[1].0));
    assert!(!state.presence.contains_key(&BOB));
    assert_eq!(state.channels["general"].list, vec![minted[0].0]);
}

/// The staleness sweep evicts exactly the peers whose newest beat is at
/// least [`timers::PRESENCE_STALE`] old, redacting their presence keys.
#[tokio::test(flavor = "current_thread")]
async fn sweep_stale_boundary() {
    let stale_ms = timers::PRESENCE_STALE.as_millis() as Millis;
    let known: Known<Entry> = Known::seed();
    let entries = vec![beat(ALICE, "alice", 1_000), beat(BOB, "bob", 2_000)];
    let minted = mint(&known, entries.clone());
    let mut state = AppState::new();
    for ((key, version), entry) in minted.iter().zip(&entries) {
        state.observe(*key, version, entry, 0);
    }

    // One tick before alice crosses the threshold: nobody is evicted.
    assert_eq!(state.sweep_stale(1_000 + stale_ms - 1), vec![]);
    assert_eq!(state.presence.len(), 2);

    // At the threshold: alice is evicted, bob (1s younger) survives.
    let effects = state.sweep_stale(1_000 + stale_ms);
    assert_eq!(effects, vec![Effect::Redact(minted[0].0)]);
    assert!(!state.presence.contains_key(&ALICE));
    assert!(state.presence.contains_key(&BOB));
}

/// Channels exist as soon as either a creation entry or a message naming
/// them arrives (delivery is unordered), and creation metadata fills in
/// whenever the creation entry shows up.
#[tokio::test(flavor = "current_thread")]
async fn channel_creation_is_order_independent() {
    let known: Known<Entry> = Known::seed();
    let entries = vec![
        Entry::Chat {
            channel: "dogs".into(),
            author: ALICE,
            body: "early".into(),
            sent_at: 1_000,
            ttl_ms: 60_000,
        },
        Entry::Channel {
            name: "dogs".into(),
            created_by: BOB,
            at: 2_000,
        },
    ];
    let minted = mint(&known, entries.clone());
    let mut state = AppState::new();

    // The message arrives before the channel's creation entry.
    state.observe(minted[0].0, &minted[0].1, &entries[0], 0);
    assert!(state.channels.contains_key("dogs"));
    assert_eq!(state.channels["dogs"].created_by, None);

    state.observe(minted[1].0, &minted[1].1, &entries[1], 0);
    assert_eq!(state.channels["dogs"].created_by, Some(BOB));
    assert_eq!(state.channels["dogs"].list.len(), 1);
}

/// `peer_name` resolves through presence and falls back to a short hex id.
#[tokio::test(flavor = "current_thread")]
async fn peer_name_resolution() {
    let known: Known<Entry> = Known::seed();
    let entries = vec![beat(ALICE, "alice", 1_000)];
    let minted = mint(&known, entries.clone());
    let mut state = AppState::new();
    state.observe(minted[0].0, &minted[0].1, &entries[0], 0);

    assert_eq!(state.peer_name(&ALICE), "alice");
    assert_eq!(state.peer_name(&BOB), "bbbbbbbb");
}
