//! A retiree's last-moment redactions must survive into the absorber.
//!
//! This is the chatroom goodbye path: a departing peer redacts its own
//! presence entry, then retires its party into a live peer. The retire
//! session's built-in reconciliation must carry the *absence* (deletion
//! honoring rides version bounds), not just the retiree's unsent content —
//! otherwise every clean departure leaves a ghost entry behind that only
//! application-level staleness sweeps can clear.

mod common;

use rumors::{Peer, Retire};

use crate::common::wire::{assert_control_drained, block_on, bootstrap_fork, wire_gossip};
use rumors::testing::SnapshotCollect as _;

/// The absorber drops an entry the retiree redacted after their last
/// ordinary gossip: the redaction rides the retire session itself.
#[test]
fn retire_carries_last_minute_redactions() {
    let a = Peer::<String>::seed().sync_window_floor().into_rumors();
    let b = bootstrap_fork(&a);

    // B originates an entry and A learns it through ordinary gossip.
    pollster::block_on(b.send("presence: b".to_string()))
        .expect("the in-memory backend is infallible");
    let key = b
        .snapshot()
        .collected()
        .map(|(key, _, _)| key)
        .next()
        .expect("the sent entry is live");
    wire_gossip(&a, &b);
    assert!(
        pollster::block_on(a.snapshot().get(&key))
            .expect("the in-memory backend is infallible")
            .is_some(),
        "precondition: A holds B's entry after gossip"
    );

    // B redacts it *after* that gossip, then retires into A.
    pollster::block_on(b.redact(key)).expect("the in-memory backend is infallible");
    let retiree = block_on(b.try_into_peer()).expect("sole handle");
    let outcome = block_on(async {
        let (mut b_link, mut a_link) = rumors::link::memory();
        let (outcome, served) = tokio::join!(retiree.retire(&mut b_link), a.gossip(&mut a_link),);
        served.expect("A serves the retire session");
        assert_control_drained(b_link, a_link);
        outcome
    });
    assert!(matches!(outcome, Retire::Retired), "clean retirement");

    // The absorber holds the absence, not the ghost.
    assert!(
        pollster::block_on(a.snapshot().get(&key))
            .expect("the in-memory backend is infallible")
            .is_none(),
        "A must honor the redaction the retiree carried"
    );
}
