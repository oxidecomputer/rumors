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

use crate::common::wire::{block_on, bootstrap_fork, wire_gossip};

/// The absorber drops an entry the retiree redacted after their last
/// ordinary gossip: the redaction rides the retire session itself.
#[test]
fn retire_carries_last_minute_redactions() {
    let a = Peer::<String>::seed().into_rumors();
    let b = bootstrap_fork(&a);

    // B originates an entry and A learns it through ordinary gossip.
    b.send("presence: b".to_string());
    let key = b
        .snapshot()
        .iter()
        .map(|(key, _, _)| key)
        .next()
        .expect("the sent entry is live");
    wire_gossip(&a, &b);
    assert!(
        a.snapshot().get(&key).is_some(),
        "precondition: A holds B's entry after gossip"
    );

    // B redacts it *after* that gossip, then retires into A.
    b.redact(key);
    let retiree = block_on(b.try_into_peer()).expect("sole handle");
    let outcome = block_on(async {
        let (b_side, a_side) = tokio::io::duplex(8 * 1024);
        let (mut b_r, mut b_w) = tokio::io::split(b_side);
        let (mut a_r, mut a_w) = tokio::io::split(a_side);
        let (outcome, served) = tokio::join!(
            retiree.retire(&mut b_r, &mut b_w),
            a.gossip(&mut a_r, &mut a_w),
        );
        served.expect("A serves the retire session");
        outcome
    });
    assert!(matches!(outcome, Retire::Retired), "clean retirement");

    // The absorber holds the absence, not the ghost.
    assert!(
        a.snapshot().get(&key).is_none(),
        "A must honor the redaction the retiree carried"
    );
}
