//! The [`rumors::Changes`] observer: the content-free change signal.
//!
//! Pins the contract stated on the type: an immediate first yield, exactly
//! one coalesced tick per observed frontier advance (however many commits
//! that was), ticks for every kind of commit — send, redact, and a join
//! learned by gossip — and a clean end once the set closes.

mod common;

use futures::{FutureExt, StreamExt};
use rumors::{Peer, Rumors};

use crate::common::wire::{bootstrap_fork_async, wire_gossip_async};

/// A fresh observer yields immediately — even on an empty set — because a
/// new subscriber has seen nothing, so whatever the set holds is news.
#[tokio::test(flavor = "current_thread")]
async fn first_poll_yields_immediately() {
    let rumors: Rumors<u64> = Peer::seed().into_rumors();
    let mut changes = rumors.changes();
    assert_eq!(changes.next().now_or_never(), Some(Some(())));
    // And with nothing further committed, the stream is quiet.
    assert_eq!(changes.next().now_or_never(), None);
}

/// Each commit observed in isolation produces exactly one tick: a send, a
/// redact, and a multi-change batch are one frontier advance apiece.
#[tokio::test(flavor = "current_thread")]
async fn one_tick_per_observed_commit() {
    let rumors: Rumors<u64> = Peer::seed().into_rumors();
    let mut changes = rumors.changes();
    assert_eq!(changes.next().now_or_never(), Some(Some(())));

    // One send: one tick.
    rumors.send(1);
    assert_eq!(changes.next().now_or_never(), Some(Some(())));
    assert_eq!(changes.next().now_or_never(), None);

    // One batch of several changes: still one commit, one tick.
    rumors.batch().send(2).send(3);
    assert_eq!(changes.next().now_or_never(), Some(Some(())));
    assert_eq!(changes.next().now_or_never(), None);

    // One redact: one tick.
    let key = rumors
        .snapshot()
        .iter()
        .find_map(|(k, _, m)| (**m == 1).then_some(k))
        .expect("message 1 is live");
    rumors.redact(key);
    assert_eq!(changes.next().now_or_never(), Some(Some(())));
    assert_eq!(changes.next().now_or_never(), None);
}

/// Ticks coalesce: any number of commits between polls is one tick — the
/// stream is a signal, not a ledger.
#[tokio::test(flavor = "current_thread")]
async fn unpolled_commits_coalesce_to_one_tick() {
    let rumors: Rumors<u64> = Peer::seed().into_rumors();
    let mut changes = rumors.changes();
    assert_eq!(changes.next().now_or_never(), Some(Some(())));

    rumors.send(1);
    rumors.send(2);
    rumors.send(3);
    assert_eq!(changes.next().now_or_never(), Some(Some(())));
    assert_eq!(changes.next().now_or_never(), None);
}

/// A join learned by gossip is a commit like any other: an observer on the
/// receiving side ticks when the session lands content from the peer.
#[tokio::test(flavor = "current_thread")]
async fn gossip_join_ticks_the_observer() {
    let a: Rumors<u64> = Peer::seed().into_rumors();
    let b = bootstrap_fork_async(&a).await;

    let mut b_changes = b.changes();
    assert_eq!(b_changes.next().now_or_never(), Some(Some(())));
    assert_eq!(b_changes.next().now_or_never(), None);

    a.send(7);
    wire_gossip_async(&a, &b).await;
    assert_eq!(b_changes.next().now_or_never(), Some(Some(())));
}

/// The stream ends once the set closes: with the `Peer` and every `Rumors`
/// gone no further change is possible, and a tick still owed (committed
/// after the last poll) is delivered before the end.
#[tokio::test(flavor = "current_thread")]
async fn set_closure_ends_the_stream() {
    let rumors: Rumors<u64> = Peer::seed().into_rumors();
    let mut changes = rumors.changes();
    assert_eq!(changes.next().now_or_never(), Some(Some(())));

    rumors.send(1);
    drop(rumors);

    // The final commit is still reported, then the stream ends.
    assert_eq!(changes.next().now_or_never(), Some(Some(())));
    assert_eq!(changes.next().now_or_never(), Some(None));
}

/// Holding a `Changes` does not count against the quiescence that lets
/// [`Rumors::try_into_peer`] reclaim the `Peer`.
#[tokio::test(flavor = "current_thread")]
async fn observer_does_not_block_peer_reclaim() {
    let rumors: Rumors<u64> = Peer::seed().into_rumors();
    let _changes = rumors.changes();
    assert!(rumors.try_into_peer().await.is_some());
}
